use std::net::IpAddr;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use anyhow::Result;
use tracing::{ debug, error, warn };
use tokio::process::Command;
use tokio::fs;
use tokio::sync::Mutex;
use reqwest;
use regex::Regex;

/// NextTrace binary URLs for different platforms
const NEXTTRACE_WINDOWS_URL: &str =
    "https://github.com/nxtrace/NTrace-core/releases/download/v1.4.0/nexttrace_windows_amd64.exe";
const NEXTTRACE_LINUX_URL: &str =
    "https://github.com/nxtrace/NTrace-core/releases/download/v1.4.0/nexttrace_linux_amd64";

/// Cache directory for NextTrace binaries
const CACHE_DIR: &str = "./cache";

/// Binary filenames
const WINDOWS_BINARY: &str = "nexttrace_windows_amd64.exe";
const LINUX_BINARY: &str = "nexttrace_linux_amd64";

/// Strip ANSI color codes from text
fn strip_ansi_codes(text: &str) -> String {
    // Regex pattern to match ANSI escape sequences
    // Matches sequences like \x1b[...m (color codes) and \x1b[...J (clear screen)
    static ANSI_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let regex = ANSI_REGEX.get_or_init(|| { Regex::new(r"\x1b\[[0-9;]*[mJKH]").unwrap() });

    regex.replace_all(text, "").to_string()
}

/// NextTrace binary manager
#[derive(Default)]
pub struct NextTraceManager {
    binary_path: String,
    initialized: bool,
}

impl NextTraceManager {
    /// Create a new NextTrace manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Setup Linux capabilities for nexttrace binary
    #[cfg(unix)]
    async fn setup_linux_capabilities(&self) -> Result<()> {
        use std::process::Stdio;

        // Check if we're on Linux and if setcap is available
        if !cfg!(target_os = "linux") {
            return Ok(());
        }

        // Try to set CAP_NET_RAW capability using setcap
        debug!("Attempting to set CAP_NET_RAW capability for nexttrace");

        let output = Command::new("setcap")
            .arg("cap_net_raw+ep")
            .arg(&self.binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output().await;

        match output {
            Ok(result) => {
                if result.status.success() {
                    debug!("Successfully set CAP_NET_RAW capability for nexttrace");
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    warn!("Failed to set capabilities (this is normal for non-root users): {}", stderr);
                    debug!(
                        "nexttrace will run without special capabilities - some features may be limited"
                    );
                }
            }
            Err(e) => {
                debug!("setcap command not available or failed: {} - this is normal on many systems", e);
            }
        }

        Ok(())
    }

    /// Check if nexttrace has sufficient privileges
    #[cfg(unix)]
    async fn check_privileges(&self) -> bool {
        use std::process::Stdio;

        // Try a quick capability check
        let output = Command::new("getcap")
            .arg(&self.binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output().await;

        if let Ok(result) = output {
            let stdout = String::from_utf8_lossy(&result.stdout);
            return stdout.contains("cap_net_raw");
        }

        // Check if running as root
        let uid_output = Command::new("id")
            .arg("-u")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output().await;

        if let Ok(result) = uid_output {
            let uid = String::from_utf8_lossy(&result.stdout).trim().parse::<u32>().unwrap_or(1000);
            return uid == 0;
        }

        false
    }

    /// Initialize NextTrace binary (download if needed)
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        debug!("Initializing NextTrace binary");

        // Create cache directory
        fs
            ::create_dir_all(CACHE_DIR).await
            .map_err(|e| { anyhow::anyhow!("Failed to create cache directory: {}", e) })?;

        // Detect platform and set binary info
        let (binary_name, download_url) = if cfg!(target_os = "windows") {
            (WINDOWS_BINARY, NEXTTRACE_WINDOWS_URL)
        } else {
            (LINUX_BINARY, NEXTTRACE_LINUX_URL)
        };

        self.binary_path = format!("{}/{}", CACHE_DIR, binary_name);

        // Check if binary already exists
        if Path::new(&self.binary_path).exists() {
            debug!("NextTrace binary already exists at {}", self.binary_path);
            self.initialized = true;
            return Ok(());
        }

        // Download the binary
        debug!("Downloading NextTrace binary from {}", download_url);
        let client = reqwest::Client::new();
        let response = client
            .get(download_url)
            .send().await
            .map_err(|e| { anyhow::anyhow!("Failed to download NextTrace binary: {}", e) })?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("Failed to download NextTrace binary: HTTP {}", response.status())
            );
        }

        let binary_data = response
            .bytes().await
            .map_err(|e| { anyhow::anyhow!("Failed to read NextTrace binary data: {}", e) })?;

        // Write binary to file
        fs
            ::write(&self.binary_path, binary_data).await
            .map_err(|e| { anyhow::anyhow!("Failed to write NextTrace binary: {}", e) })?;

        // Make binary executable on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.binary_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.binary_path, perms).await?;

            // Try to set CAP_NET_RAW capability for ICMP on Linux
            self.setup_linux_capabilities().await?;
        }

        debug!("NextTrace binary downloaded and installed at {}", self.binary_path);
        self.initialized = true;
        Ok(())
    }

    /// Execute NextTrace for the given target IP
    pub async fn trace_route(&self, target_ip: &str) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("NextTrace not initialized"));
        }

        debug!("Running NextTrace for target: {}", target_ip);

        // Check privileges and provide guidance if needed
        #[cfg(unix)]
        let has_privileges = self.check_privileges().await;
        #[cfg(not(unix))]
        let has_privileges = true;

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg(target_ip);

        // Add fallback options for unprivileged execution
        #[cfg(unix)]
        if !has_privileges {
            // Try UDP mode first as fallback for unprivileged users
            cmd.arg("--udp");
            debug!("Using UDP mode for unprivileged execution");
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output().await
            .map_err(|e| { anyhow::anyhow!("Failed to execute NextTrace: {}", e) })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("NextTrace execution failed: {}", stderr);

            // Provide helpful error message with privilege guidance
            let error_msg = if !has_privileges && stderr.contains("permission") {
                format!(
                    "NextTrace execution failed due to insufficient privileges.\n\n{}\n\nTo resolve this issue on Linux, try one of the following:\n1. Run as root: sudo whois-server\n2. Set capabilities: sudo setcap cap_net_raw+ep {}\n3. Use UDP mode: nexttrace --udp {}\n\nNote: ICMP traceroute requires elevated privileges for raw socket access.",
                    stderr,
                    self.binary_path,
                    target_ip
                )
            } else {
                format!("NextTrace execution failed: {}", stderr)
            };

            return Err(anyhow::anyhow!(error_msg));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("NextTrace completed for {}", target_ip);

        // Strip ANSI color codes from output
        let clean_output = strip_ansi_codes(&stdout);

        // Add privilege status to output for transparency
        let privilege_note = if !has_privileges {
            "\n\nNote: Running in UDP mode without CAP_NET_RAW capability.\nFor ICMP traceroute, consider running with elevated privileges or setting capabilities:\n  sudo setcap cap_net_raw+ep nexttrace\n"
        } else {
            ""
        };

        Ok(format!("{}{}", clean_output, privilege_note))
    }
}

/// Global NextTrace manager instance
static NEXTTRACE_MANAGER: tokio::sync::OnceCell<Arc<Mutex<NextTraceManager>>> = tokio::sync::OnceCell::const_new();

/// Get or initialize the global NextTrace manager
async fn get_nexttrace_manager() -> Result<Arc<Mutex<NextTraceManager>>> {
    let manager = NEXTTRACE_MANAGER.get_or_init(|| async {
        Arc::new(Mutex::new(NextTraceManager::new()))
    }).await;

    Ok(manager.clone())
}

/// Process traceroute query with -TRACE suffix
pub async fn process_traceroute_query(query: &str) -> Result<String> {
    debug!("Processing traceroute query: {}", query);

    // Remove -TRACE suffix if present
    let clean_query = if query.to_uppercase().ends_with("-TRACE") {
        &query[..query.len() - 6]
    } else {
        query
    };

    // Parse IP address or resolve hostname
    let target = if let Ok(ip) = clean_query.parse::<IpAddr>() {
        ip.to_string()
    } else {
        // For hostnames, pass directly to NextTrace which can handle them
        clean_query.to_string()
    };

    debug!("Starting NextTrace traceroute to {}", target);

    // Get NextTrace manager and execute
    match get_nexttrace_manager().await {
        Ok(manager_arc) => {
            let mut manager = manager_arc.lock().await;

            // Initialize if needed
            if !manager.initialized {
                if let Err(e) = manager.initialize().await {
                    error!("Failed to initialize NextTrace: {}", e);
                    return Ok(
                        format!("Traceroute service initialization failed: {}\n\nPlease ensure internet connectivity for initial setup.\n", e)
                    );
                }
            }

            match manager.trace_route(&target).await {
                Ok(output) => {
                    debug!("NextTrace traceroute completed successfully");
                    let final_output = format!(
                        "Traceroute to {} using NextTrace:\n\n{}",
                        target,
                        output
                    );
                    // Strip any remaining ANSI codes from the final output
                    Ok(strip_ansi_codes(&final_output))
                }
                Err(e) => {
                    error!("NextTrace execution failed: {}", e);
                    Ok(
                        format!("Traceroute failed: {}\n\nNote: NextTrace requires network access and may need administrator privileges on some systems.\n", e)
                    )
                }
            }
        }
        Err(e) => {
            error!("Failed to get NextTrace manager: {}", e);
            Ok(format!("Traceroute service error: {}\n", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_selection() {
        let (binary_name, _) = if cfg!(target_os = "windows") {
            (WINDOWS_BINARY, NEXTTRACE_WINDOWS_URL)
        } else {
            (LINUX_BINARY, NEXTTRACE_LINUX_URL)
        };

        assert!(!binary_name.is_empty());
    }

    #[tokio::test]
    #[ignore] // This test requires network access and can hang
    async fn test_traceroute_query_parsing() {
        let result = process_traceroute_query("8.8.8.8-TRACE").await;
        assert!(result.is_ok());

        let result = process_traceroute_query("google.com-TRACE").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_nexttrace_manager_creation() {
        let manager = NextTraceManager::new();
        assert!(!manager.initialized);
        assert!(manager.binary_path.is_empty());
    }
}
