use std::fs;
use std::path::Path;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::Command;
use anyhow::Result;
use tracing::{debug, info, warn, error};
use tokio::time::{interval, Duration};

use crate::config::DN42_REGISTRY_PATH;

const DN42_REGISTRY_URL: &str = "https://git.pysio.online/pysio/mirrors-dn42.git";

/// Synchronize DN42 registry from git repository
pub async fn sync_dn42_registry() -> Result<()> {
    info!("Starting DN42 registry synchronization from {}", DN42_REGISTRY_URL);
    
    let registry_path = Path::new(DN42_REGISTRY_PATH);
    
    // Run git operations in a blocking task to avoid blocking the async runtime
    let result = tokio::task::spawn_blocking(move || {
        if registry_path.exists() {
            // If directory exists, check if it's a git repository
            let git_dir = registry_path.join(".git");
            if git_dir.exists() {
                info!("Repository exists, pulling latest changes...");
                pull_latest_changes()
            } else {
                warn!("Directory exists but is not a git repository. Attempting fresh clone...");
                // Remove directory and clone fresh
                if let Err(remove_err) = std::fs::remove_dir_all(registry_path) {
                    error!("Failed to remove directory: {}", remove_err);
                    return Err(anyhow::anyhow!("Failed to remove directory: {}", remove_err));
                }
                clone_repository()
            }
        } else {
            // Directory doesn't exist, clone repository
            info!("Repository doesn't exist, cloning from {}", DN42_REGISTRY_URL);
            clone_repository()
        }
    }).await?;
    
    match result {
        Ok(_) => {
            info!("DN42 registry synchronization completed successfully");
            Ok(())
        },
        Err(e) => {
            error!("DN42 registry synchronization failed: {}", e);
            Err(e)
        }
    }
}

/// Clone the DN42 registry repository using system git command
fn clone_repository() -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(DN42_REGISTRY_PATH).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    info!("Cloning repository from {} to {}", DN42_REGISTRY_URL, DN42_REGISTRY_PATH);
    
    let output = Command::new("git")
        .args(&["clone", "--depth", "1", DN42_REGISTRY_URL, DN42_REGISTRY_PATH])
        .output()?;
    
    if output.status.success() {
        info!("Successfully cloned DN42 registry to {}", DN42_REGISTRY_PATH);
        
        // Log any output from git command
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            debug!("Git clone stdout: {}", stdout);
        }
        
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Failed to clone repository: {}", stderr);
        Err(anyhow::anyhow!("Git clone failed: {}", stderr))
    }
}

/// Pull latest changes from the repository using system git command
fn pull_latest_changes() -> Result<()> {
    info!("Pulling latest changes from repository");
    
    // First, fetch the latest changes
    let fetch_output = Command::new("git")
        .args(&["fetch", "origin"])
        .current_dir(DN42_REGISTRY_PATH)
        .output()?;
    
    if !fetch_output.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_output.stderr);
        error!("Failed to fetch from repository: {}", stderr);
        return Err(anyhow::anyhow!("Git fetch failed: {}", stderr));
    }
    
    // Check if there are any changes to pull
    let status_output = Command::new("git")
        .args(&["status", "-uno", "--porcelain"])
        .current_dir(DN42_REGISTRY_PATH)
        .output()?;
    
    if !status_output.status.success() {
        let stderr = String::from_utf8_lossy(&status_output.stderr);
        error!("Failed to check git status: {}", stderr);
        return Err(anyhow::anyhow!("Git status failed: {}", stderr));
    }
    
    // Reset hard to origin/master (or origin/master)
    let reset_output = Command::new("git")
        .args(&["reset", "--hard", "origin/master"])
        .current_dir(DN42_REGISTRY_PATH)
        .output();
    
    let reset_result = match reset_output {
        Ok(output) if output.status.success() => {
            info!("Successfully reset to origin/master");
            Ok(())
        },
        Ok(output) => {
            // Try origin/master if origin/master failed
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("Reset to origin/master failed: {}, trying origin/master", stderr);
            
            let master_output = Command::new("git")
                .args(&["reset", "--hard", "origin/master"])
                .current_dir(DN42_REGISTRY_PATH)
                .output()?;
            
            if master_output.status.success() {
                info!("Successfully reset to origin/master");
                Ok(())
            } else {
                let master_stderr = String::from_utf8_lossy(&master_output.stderr);
                error!("Failed to reset to origin/master: {}", master_stderr);
                Err(anyhow::anyhow!("Git reset failed: {}", master_stderr))
            }
        },
        Err(e) => {
            error!("Failed to execute git reset: {}", e);
            Err(anyhow::anyhow!("Git reset execution failed: {}", e))
        }
    };
    
    // Log fetch output if available
    if !fetch_output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&fetch_output.stdout);
        debug!("Git fetch stdout: {}", stdout);
    }
    
    reset_result
}

/// Start the periodic DN42 registry sync task
pub async fn start_periodic_sync() {
    info!("Starting periodic DN42 registry sync (every hour)");
    
    // Initial sync at startup
    if let Err(e) = sync_dn42_registry().await {
        error!("Initial DN42 registry sync failed: {}", e);
    }
    
    // Set up hourly sync
    let mut interval = interval(Duration::from_secs(3600)); // 1 hour
    interval.tick().await; // Skip the first tick (we just did initial sync)
    
    loop {
        interval.tick().await;
        
        info!("Starting scheduled DN42 registry sync");
        if let Err(e) = sync_dn42_registry().await {
            error!("Scheduled DN42 registry sync failed: {}", e);
        }
    }
}

/// Check if a file exists in the DN42 registry
fn file_exists(subdir: &str, target: &str) -> bool {
    if target.is_empty() {
        return false;
    }
    
    let sanitized_target = target.replace('/', "_");
    let file_path = format!("{}/data/{}/{}", DN42_REGISTRY_PATH, subdir, sanitized_target);
    Path::new(&file_path).exists()
}

/// Read file content from DN42 registry
fn file_read(subdir: &str, target: &str) -> Option<String> {
    if target.is_empty() {
        return None;
    }
    
    let sanitized_target = target.replace('/', "_");
    let file_path = format!("{}/data/{}/{}", DN42_REGISTRY_PATH, subdir, sanitized_target);
    
    match fs::read_to_string(&file_path) {
        Ok(content) => Some(content),
        Err(e) => {
            debug!("Failed to read file {}: {}", file_path, e);
            None
        }
    }
}

/// Find the best matching IPv4 network in DN42 registry
fn ipv4_find(subdir: &str, ip: Ipv4Addr, query_mask: u8) -> Option<String> {
    let ip_int = u32::from(ip);
    
    // Search from the query mask down to /0
    for mask in (0..=query_mask).rev() {
        let network_int = if mask > 0 {
            ip_int & (0xFFFFFFFF << (32 - mask))
        } else {
            0
        };
        
        let network_ip = Ipv4Addr::from(network_int);
        let network_str = format!("{}/{}", network_ip, mask);
        
        if file_exists(subdir, &network_str) {
            return Some(network_str);
        }
    }
    
    None
}

/// Find the best matching IPv6 network in DN42 registry  
fn ipv6_find(subdir: &str, ip: Ipv6Addr, query_mask: u8) -> Option<String> {
    let ip_int = u128::from(ip);
    
    // Search from the query mask down to /0
    for mask in (0..=query_mask).rev() {
        let network_int = if mask > 0 {
            ip_int & (0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF << (128 - mask))
        } else {
            0
        };
        
        let network_ip = Ipv6Addr::from(network_int);
        let network_str = format!("{}/{}", network_ip, mask);
        
        if file_exists(subdir, &network_str) {
            return Some(network_str);
        }
    }
    
    None
}

/// Process DN42 query and return raw data (for email processing)
pub async fn query_dn42_raw(query: &str) -> Result<String> {
    debug!("Processing DN42 raw query: {}", query);
    
    // Handle different query types and return just the content
    if let Some(result) = handle_query_routes_raw(query).await {
        Ok(result)
    } else if let Some(result) = handle_direct_lookup_raw(query).await {
        Ok(result)
    } else {
        Ok(String::new()) // Return empty string for not found
    }
}

/// Process DN42 query using registry files
pub async fn process_dn42_query(query: &str) -> Result<String> {
    debug!("Processing DN42 query: {}", query);
    
    let mut response = String::new();
    response.push_str(&format!("% Query: {}\n", query));
    
    // Handle different query types
    if let Some(result) = handle_query_routes(query).await {
        response.push_str(&result);
    } else if let Some(result) = handle_direct_lookup(query).await {
        response.push_str(&result);
    } else {
        response.push_str("% 404 Not Found\n");
    }
    
    Ok(response)
}

/// Handle IP address queries (both IPv4 and IPv6)
async fn handle_query_routes(query: &str) -> Option<String> {
    // Parse IPv4 CIDR
    if let Some((ip_str, mask_str)) = query.split_once('/') {
        if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>()) {
            if mask <= 32 {
                return Some(handle_ipv4_query(ipv4, mask).await);
            }
        }
        
        if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>()) {
            if mask <= 128 {
                return Some(handle_ipv6_query(ipv6, mask).await);
            }
        }
    }
    
    // Parse single IP address (assume /32 for IPv4, /128 for IPv6)
    if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
        return Some(handle_ipv4_query(ipv4, 32).await);
    }
    
    if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
        return Some(handle_ipv6_query(ipv6, 128).await);
    }
    
    None
}

/// Handle IPv4 queries (inetnum and route lookups)
async fn handle_ipv4_query(ip: Ipv4Addr, mask: u8) -> String {
    let mut response = String::new();
    
    // Look up inetnum
    if let Some(target) = ipv4_find("inetnum", ip, mask) {
        if let Some(content) = file_read("inetnum", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - inetnum not found\n");
        }
    } else {
        response.push_str("% 404 - inetnum not found\n");
    }
    
    response.push_str("% Relevant route object:\n");
    
    // Look up route
    if let Some(target) = ipv4_find("route", ip, mask) {
        if let Some(content) = file_read("route", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - route not found\n");
        }
    } else {
        response.push_str("% 404 - route not found\n");
    }
    
    response
}

/// Handle IPv6 queries (inet6num and route6 lookups)
async fn handle_ipv6_query(ip: Ipv6Addr, mask: u8) -> String {
    let mut response = String::new();
    
    // Look up inet6num
    if let Some(target) = ipv6_find("inet6num", ip, mask) {
        if let Some(content) = file_read("inet6num", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - inet6num not found\n");
        }
    } else {
        response.push_str("% 404 - inet6num not found\n");
    }
    
    response.push_str("% Relevant route object:\n");
    
    // Look up route6
    if let Some(target) = ipv6_find("route6", ip, mask) {
        if let Some(content) = file_read("route6", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - route6 not found\n");
        }
    } else {
        response.push_str("% 404 - route6 not found\n");
    }
    
    response
}

/// Handle direct object lookups (aut-num, person, mntner, etc.)
async fn handle_direct_lookup(query: &str) -> Option<String> {
    let normalized_query = query.to_uppercase();
    
    // Handle ASN queries
    if let Some(asn) = parse_asn(&normalized_query) {
        if let Some(content) = file_read("aut-num", &asn) {
            return Some(content);
        }
    }
    
    // Handle person objects (-DN42 suffix)
    if normalized_query.ends_with("-DN42") {
        if let Some(content) = file_read("person", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle maintainer objects (-MNT suffix)  
    if normalized_query.ends_with("-MNT") {
        if let Some(content) = file_read("mntner", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle schema objects (-SCHEMA suffix)
    if normalized_query.ends_with("-SCHEMA") {
        if let Some(content) = file_read("schema", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle organisation objects (ORG- prefix)
    if normalized_query.starts_with("ORG-") {
        if let Some(content) = file_read("organisation", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-keyset objects (SET-*-TINC pattern)
    if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
        if let Some(content) = file_read("tinc-keyset", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-key objects (-TINC suffix)
    if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
        if let Some(content) = file_read("tinc-key", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle route-set objects (RS- prefix)
    if normalized_query.starts_with("RS-") {
        if let Some(content) = file_read("route-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-block objects (AS*-AS* pattern)
    if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
        if let Some(content) = file_read("as-block", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-set objects (AS prefix, not an ASN)
    if normalized_query.starts_with("AS") && !normalized_query.chars().skip(2).all(|c| c.is_ascii_digit()) {
        if let Some(content) = file_read("as-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle DNS objects (default fallback)
    if let Some(content) = file_read("dns", &query.to_lowercase()) {
        return Some(content);
    }
    
    None
}

/// Handle IP address queries (raw data, no formatting)
async fn handle_query_routes_raw(query: &str) -> Option<String> {
    // Parse IPv4 CIDR
    if let Some((ip_str, mask_str)) = query.split_once('/') {
        if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>()) {
            if mask <= 32 {
                if let Some(target) = ipv4_find("inetnum", ipv4, mask) {
                    return file_read("inetnum", &target);
                }
            }
        }
        
        if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>()) {
            if mask <= 128 {
                if let Some(target) = ipv6_find("inet6num", ipv6, mask) {
                    return file_read("inet6num", &target);
                }
            }
        }
    }
    
    // Parse single IP address (assume /32 for IPv4, /128 for IPv6)
    if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
        if let Some(target) = ipv4_find("inetnum", ipv4, 32) {
            return file_read("inetnum", &target);
        }
    }
    
    if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
        if let Some(target) = ipv6_find("inet6num", ipv6, 128) {
            return file_read("inet6num", &target);
        }
    }
    
    None
}

/// Handle direct object lookups (raw data, no formatting)
async fn handle_direct_lookup_raw(query: &str) -> Option<String> {
    let normalized_query = query.to_uppercase();
    
    // Handle ASN queries
    if let Some(asn) = parse_asn(&normalized_query) {
        if let Some(content) = file_read("aut-num", &asn) {
            return Some(content);
        }
    }
    
    // Handle person objects (-DN42 suffix)
    if normalized_query.ends_with("-DN42") {
        if let Some(content) = file_read("person", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle maintainer objects (-MNT suffix)  
    if normalized_query.ends_with("-MNT") {
        if let Some(content) = file_read("mntner", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle schema objects (-SCHEMA suffix)
    if normalized_query.ends_with("-SCHEMA") {
        if let Some(content) = file_read("schema", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle organisation objects (ORG- prefix)
    if normalized_query.starts_with("ORG-") {
        if let Some(content) = file_read("organisation", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-keyset objects (SET-*-TINC pattern)
    if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
        if let Some(content) = file_read("tinc-keyset", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-key objects (-TINC suffix)
    if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
        if let Some(content) = file_read("tinc-key", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle route-set objects (RS- prefix)
    if normalized_query.starts_with("RS-") {
        if let Some(content) = file_read("route-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-block objects (AS*-AS* pattern)
    if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
        if let Some(content) = file_read("as-block", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-set objects (AS prefix, not an ASN)
    if normalized_query.starts_with("AS") && !normalized_query.chars().skip(2).all(|c| c.is_ascii_digit()) {
        if let Some(content) = file_read("as-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle DNS objects (default fallback)
    if let Some(content) = file_read("dns", &query.to_lowercase()) {
        return Some(content);
    }
    
    None
}

/// Parse ASN from query, handling various formats
fn parse_asn(query: &str) -> Option<String> {
    let normalized = query.to_uppercase();
    
    // Handle short ASN formats (1-4 digits) - convert to full DN42 format
    if let Ok(num) = normalized.parse::<u32>() {
        return match num.to_string().len() {
            1 => Some(format!("AS424242000{}", num)),
            2 => Some(format!("AS42424200{}", num)), 
            3 => Some(format!("AS4242420{}", num)),
            4 => Some(format!("AS424242{}", num)),
            _ => Some(format!("AS{}", num)),
        };
    }
    
    // Handle AS prefix
    if normalized.starts_with("AS") {
        let asn_part = &normalized[2..];
        if let Ok(num) = asn_part.parse::<u32>() {
            return match asn_part.len() {
                1 => Some(format!("AS424242000{}", num)),
                2 => Some(format!("AS42424200{}", num)),
                3 => Some(format!("AS4242420{}", num)), 
                4 => Some(format!("AS424242{}", num)),
                _ => Some(normalized),
            };
        }
    }
    
    None
}

/// Blocking version of raw query (for email processing)
pub fn query_dn42_raw_blocking(query: &str) -> Result<String> {
    debug!("Processing DN42 raw query (blocking): {}", query);
    
    // Handle different query types and return just the content
    if let Some(result) = handle_query_routes_raw_blocking(query) {
        Ok(result)
    } else if let Some(result) = handle_direct_lookup_raw_blocking(query) {
        Ok(result)
    } else {
        Ok(String::new()) // Return empty string for not found
    }
}

/// Blocking version for compatibility
pub fn process_dn42_query_blocking(query: &str) -> Result<String> {
    // For now, we'll use a simple blocking approach since file I/O is typically fast
    // In a real implementation, you might want to use tokio::task::spawn_blocking
    
    debug!("Processing DN42 query (blocking): {}", query);
    
    let mut response = String::new();
    response.push_str(&format!("% Query: {}\n", query));
    
    // Handle different query types
    if let Some(result) = handle_query_routes_blocking(query) {
        response.push_str(&result);
    } else if let Some(result) = handle_direct_lookup_blocking(query) {
        response.push_str(&result);
    } else {
        response.push_str("% 404 Not Found\n");
    }
    
    Ok(response)
}

/// Blocking version of handle_query_routes
fn handle_query_routes_blocking(query: &str) -> Option<String> {
    // Parse IPv4 CIDR
    if let Some((ip_str, mask_str)) = query.split_once('/') {
        if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>()) {
            if mask <= 32 {
                return Some(handle_ipv4_query_blocking(ipv4, mask));
            }
        }
        
        if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>()) {
            if mask <= 128 {
                return Some(handle_ipv6_query_blocking(ipv6, mask));
            }
        }
    }
    
    // Parse single IP address (assume /32 for IPv4, /128 for IPv6)
    if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
        return Some(handle_ipv4_query_blocking(ipv4, 32));
    }
    
    if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
        return Some(handle_ipv6_query_blocking(ipv6, 128));
    }
    
    None
}

/// Blocking version of handle_ipv4_query
fn handle_ipv4_query_blocking(ip: Ipv4Addr, mask: u8) -> String {
    let mut response = String::new();
    
    // Look up inetnum
    if let Some(target) = ipv4_find("inetnum", ip, mask) {
        if let Some(content) = file_read("inetnum", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - inetnum not found\n");
        }
    } else {
        response.push_str("% 404 - inetnum not found\n");
    }
    
    response.push_str("% Relevant route object:\n");
    
    // Look up route
    if let Some(target) = ipv4_find("route", ip, mask) {
        if let Some(content) = file_read("route", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - route not found\n");
        }
    } else {
        response.push_str("% 404 - route not found\n");
    }
    
    response
}

/// Blocking version of handle_ipv6_query
fn handle_ipv6_query_blocking(ip: Ipv6Addr, mask: u8) -> String {
    let mut response = String::new();
    
    // Look up inet6num
    if let Some(target) = ipv6_find("inet6num", ip, mask) {
        if let Some(content) = file_read("inet6num", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - inet6num not found\n");
        }
    } else {
        response.push_str("% 404 - inet6num not found\n");
    }
    
    response.push_str("% Relevant route object:\n");
    
    // Look up route6
    if let Some(target) = ipv6_find("route6", ip, mask) {
        if let Some(content) = file_read("route6", &target) {
            response.push_str(&content);
        } else {
            response.push_str("% 404 - route6 not found\n");
        }
    } else {
        response.push_str("% 404 - route6 not found\n");
    }
    
    response
}

/// Blocking version of handle_direct_lookup
fn handle_direct_lookup_blocking(query: &str) -> Option<String> {
    let normalized_query = query.to_uppercase();
    
    // Handle ASN queries
    if let Some(asn) = parse_asn(&normalized_query) {
        if let Some(content) = file_read("aut-num", &asn) {
            return Some(content);
        }
    }
    
    // Handle person objects (-DN42 suffix)
    if normalized_query.ends_with("-DN42") {
        if let Some(content) = file_read("person", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle maintainer objects (-MNT suffix)  
    if normalized_query.ends_with("-MNT") {
        if let Some(content) = file_read("mntner", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle schema objects (-SCHEMA suffix)
    if normalized_query.ends_with("-SCHEMA") {
        if let Some(content) = file_read("schema", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle organisation objects (ORG- prefix)
    if normalized_query.starts_with("ORG-") {
        if let Some(content) = file_read("organisation", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-keyset objects (SET-*-TINC pattern)
    if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
        if let Some(content) = file_read("tinc-keyset", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-key objects (-TINC suffix)
    if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
        if let Some(content) = file_read("tinc-key", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle route-set objects (RS- prefix)
    if normalized_query.starts_with("RS-") {
        if let Some(content) = file_read("route-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-block objects (AS*-AS* pattern)
    if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
        if let Some(content) = file_read("as-block", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-set objects (AS prefix, not an ASN)
    if normalized_query.starts_with("AS") && !normalized_query.chars().skip(2).all(|c| c.is_ascii_digit()) {
        if let Some(content) = file_read("as-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle DNS objects (default fallback)
    if let Some(content) = file_read("dns", &query.to_lowercase()) {
        return Some(content);
    }
    
    None
}

/// Handle IP address queries (raw data, no formatting, blocking)
fn handle_query_routes_raw_blocking(query: &str) -> Option<String> {
    // Parse IPv4 CIDR
    if let Some((ip_str, mask_str)) = query.split_once('/') {
        if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>()) {
            if mask <= 32 {
                if let Some(target) = ipv4_find("inetnum", ipv4, mask) {
                    return file_read("inetnum", &target);
                }
            }
        }
        
        if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>()) {
            if mask <= 128 {
                if let Some(target) = ipv6_find("inet6num", ipv6, mask) {
                    return file_read("inet6num", &target);
                }
            }
        }
    }
    
    // Parse single IP address (assume /32 for IPv4, /128 for IPv6)
    if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
        if let Some(target) = ipv4_find("inetnum", ipv4, 32) {
            return file_read("inetnum", &target);
        }
    }
    
    if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
        if let Some(target) = ipv6_find("inet6num", ipv6, 128) {
            return file_read("inet6num", &target);
        }
    }
    
    None
}

/// Handle direct object lookups (raw data, no formatting, blocking)
fn handle_direct_lookup_raw_blocking(query: &str) -> Option<String> {
    let normalized_query = query.to_uppercase();
    
    // Handle ASN queries
    if let Some(asn) = parse_asn(&normalized_query) {
        if let Some(content) = file_read("aut-num", &asn) {
            return Some(content);
        }
    }
    
    // Handle person objects (-DN42 suffix)
    if normalized_query.ends_with("-DN42") {
        if let Some(content) = file_read("person", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle maintainer objects (-MNT suffix)  
    if normalized_query.ends_with("-MNT") {
        if let Some(content) = file_read("mntner", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle schema objects (-SCHEMA suffix)
    if normalized_query.ends_with("-SCHEMA") {
        if let Some(content) = file_read("schema", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle organisation objects (ORG- prefix)
    if normalized_query.starts_with("ORG-") {
        if let Some(content) = file_read("organisation", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-keyset objects (SET-*-TINC pattern)
    if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
        if let Some(content) = file_read("tinc-keyset", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle tinc-key objects (-TINC suffix)
    if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
        if let Some(content) = file_read("tinc-key", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle route-set objects (RS- prefix)
    if normalized_query.starts_with("RS-") {
        if let Some(content) = file_read("route-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-block objects (AS*-AS* pattern)
    if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
        if let Some(content) = file_read("as-block", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle as-set objects (AS prefix, not an ASN)
    if normalized_query.starts_with("AS") && !normalized_query.chars().skip(2).all(|c| c.is_ascii_digit()) {
        if let Some(content) = file_read("as-set", &normalized_query) {
            return Some(content);
        }
    }
    
    // Handle DNS objects (default fallback)
    if let Some(content) = file_read("dns", &query.to_lowercase()) {
        return Some(content);
    }
    
    None
} 