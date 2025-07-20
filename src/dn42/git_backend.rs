use std::path::Path;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::Command;
use anyhow::Result;
use tracing::{debug, info, warn, error};
use tokio::time::{interval, Duration};

use crate::config::{DN42_REGISTRY_PATH, DN42_LMDB_PATH};
use crate::storage::{SharedLmdbStorage, create_shared_storage};

const DN42_REGISTRY_URL: &str = "https://git.pysio.online/pysio/mirrors-dn42.git";

/// DN42 registry manager with LMDB storage
pub struct DN42Registry {
    storage: SharedLmdbStorage,
}

impl DN42Registry {
    /// Create a new DN42 registry instance with LMDB storage
    pub async fn new() -> Result<Self> {
        let storage = create_shared_storage(DN42_LMDB_PATH)
            .map_err(|e| anyhow::anyhow!("Failed to create LMDB storage: {}", e))?;
        
        info!("DN42Registry created successfully with LMDB storage");
        Ok(DN42Registry { storage })
    }

    /// Initialize the DN42 registry (sync and populate LMDB)
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing DN42 registry with LMDB storage");
        
        // Sync the registry from git
        self.sync_registry().await?;
        
        // Populate LMDB with registry data
        self.populate_lmdb().await?;
        
        info!("DN42 registry initialization completed");
        Ok(())
    }

    /// Sync DN42 registry from git repository
    async fn sync_registry(&self) -> Result<()> {
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

    /// Populate LMDB with registry data after git sync
    async fn populate_lmdb(&self) -> Result<()> {
        info!("Populating LMDB with DN42 registry data");
        
        // Verify the registry directory exists
        let registry_path = Path::new(DN42_REGISTRY_PATH);
        if !registry_path.exists() {
            return Err(anyhow::anyhow!("DN42 registry directory does not exist: {}", DN42_REGISTRY_PATH));
        }
        
        let data_dir = registry_path.join("data");
        if !data_dir.exists() {
            return Err(anyhow::anyhow!("DN42 registry data directory does not exist: {:?}", data_dir));
        }
        
        let storage = self.storage.clone();
        let registry_path_str = DN42_REGISTRY_PATH.to_string();
        
        tokio::task::spawn_blocking(move || {
            storage.populate_from_registry(&registry_path_str)
        }).await?
            .map_err(|e| anyhow::anyhow!("Failed to populate LMDB from registry: {}", e))
    }

    /// Update the registry and refresh LMDB data (incremental)
    pub async fn update(&self) -> Result<()> {
        info!("Updating DN42 registry and LMDB data (incremental)");
        
        // Sync from git
        self.sync_registry().await?;
        
        // Perform incremental update (no need to clear everything)
        self.populate_lmdb().await?;
        
        info!("DN42 registry incremental update completed");
        Ok(())
    }

    /// Force full refresh of LMDB data (clear and repopulate)
    #[allow(dead_code)]
    pub async fn force_full_refresh(&self) -> Result<()> {
        info!("Forcing full DN42 registry refresh");
        
        // Sync from git
        self.sync_registry().await?;
        
        // Force full refresh
        let storage = self.storage.clone();
        let registry_path_str = DN42_REGISTRY_PATH.to_string();
        
        tokio::task::spawn_blocking(move || {
            storage.force_full_refresh(&registry_path_str)
        }).await?
            .map_err(|e| anyhow::anyhow!("Failed to force full LMDB refresh: {}", e))?;
        
        info!("DN42 registry full refresh completed");
        Ok(())
    }

    /// Query DN42 registry data and return formatted response
    pub async fn query(&self, query: &str) -> Result<String> {
        debug!("DN42: Processing query: {}", query);
        
        let mut response = String::new();
        response.push_str(&format!("% Query: {}\n", query));
        
        // Handle different query types
        if let Some(result) = self.handle_ip_query(query).await? {
            debug!("DN42: Query '{}' matched as IP query, response length: {} bytes", query, result.len());
            response.push_str(&result);
        } else if let Some(result) = self.handle_object_query(query).await? {
            debug!("DN42: Query '{}' matched as object query, response length: {} bytes", query, result.len());
            response.push_str(&result);
        } else {
            debug!("DN42: Query '{}' did not match any data", query);
            response.push_str("% 404 Not Found\n");
        }
        
        Ok(response)
    }

    /// Query DN42 registry and return raw data (for email processing)
    pub async fn query_raw(&self, query: &str) -> Result<String> {
        debug!("Processing DN42 raw query: {}", query);
        
        // Handle different query types and return just the content
        if let Some(result) = self.handle_ip_query_raw(query).await? {
            Ok(result)
        } else if let Some(result) = self.handle_object_query_raw(query).await? {
            Ok(result)
        } else {
            Ok(String::new()) // Return empty string for not found
        }
    }

    /// Handle IP address queries (both IPv4 and IPv6)
    async fn handle_ip_query(&self, query: &str) -> Result<Option<String>> {
        // Parse IPv4 CIDR
        if let Some((ip_str, mask_str)) = query.split_once('/') {
            if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>()) {
                if mask <= 32 {
                    return Ok(Some(self.handle_ipv4_query(ipv4, mask).await?));
                }
            }
            
            if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>()) {
                if mask <= 128 {
                    return Ok(Some(self.handle_ipv6_query(ipv6, mask).await?));
                }
            }
        }
        
        // Parse single IP address (assume /32 for IPv4, /128 for IPv6)
        if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
            return Ok(Some(self.handle_ipv4_query(ipv4, 32).await?));
        }
        
        if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
            return Ok(Some(self.handle_ipv6_query(ipv6, 128).await?));
        }
        
        Ok(None)
    }

    /// Handle IPv4 queries (inetnum and route lookups)
    async fn handle_ipv4_query(&self, ip: Ipv4Addr, mask: u8) -> Result<String> {
        let mut response = String::new();
        
        // Look up inetnum
        if let Some(target) = self.find_ipv4_network("inetnum", ip, mask).await? {
            if let Some(content) = self.get_from_storage(&format!("inetnum/{}", target)).await? {
                response.push_str(&content);
            } else {
                response.push_str("% 404 - inetnum not found\n");
            }
        } else {
            response.push_str("% 404 - inetnum not found\n");
        }
        
        response.push_str("% Relevant route object:\n");
        
        // Look up route
        if let Some(target) = self.find_ipv4_network("route", ip, mask).await? {
            if let Some(content) = self.get_from_storage(&format!("route/{}", target)).await? {
                response.push_str(&content);
            } else {
                response.push_str("% 404 - route not found\n");
            }
        } else {
            response.push_str("% 404 - route not found\n");
        }
        
        Ok(response)
    }

    /// Handle IPv6 queries (inet6num and route6 lookups)
    async fn handle_ipv6_query(&self, ip: Ipv6Addr, mask: u8) -> Result<String> {
        let mut response = String::new();
        
        // Look up inet6num
        if let Some(target) = self.find_ipv6_network("inet6num", ip, mask).await? {
            if let Some(content) = self.get_from_storage(&format!("inet6num/{}", target)).await? {
                response.push_str(&content);
            } else {
                response.push_str("% 404 - inet6num not found\n");
            }
        } else {
            response.push_str("% 404 - inet6num not found\n");
        }
        
        response.push_str("% Relevant route object:\n");
        
        // Look up route6
        if let Some(target) = self.find_ipv6_network("route6", ip, mask).await? {
            if let Some(content) = self.get_from_storage(&format!("route6/{}", target)).await? {
                response.push_str(&content);
            } else {
                response.push_str("% 404 - route6 not found\n");
            }
        } else {
            response.push_str("% 404 - route6 not found\n");
        }
        
        Ok(response)
    }

    /// Handle direct object lookups (aut-num, person, mntner, etc.)
    async fn handle_object_query(&self, query: &str) -> Result<Option<String>> {
        let normalized_query = query.to_uppercase();
        
        // Handle ASN queries
        if let Some(asn) = parse_asn(&normalized_query) {
            if let Some(content) = self.get_from_storage(&format!("aut-num/{}", asn)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle person objects (-DN42 suffix)
        if normalized_query.ends_with("-DN42") {
            if let Some(content) = self.get_from_storage(&format!("person/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle maintainer objects (-MNT suffix)
        if normalized_query.ends_with("-MNT") {
            if let Some(content) = self.get_from_storage(&format!("mntner/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle schema objects (-SCHEMA suffix)
        if normalized_query.ends_with("-SCHEMA") {
            if let Some(content) = self.get_from_storage(&format!("schema/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle organisation objects (ORG- prefix)
        if normalized_query.starts_with("ORG-") {
            if let Some(content) = self.get_from_storage(&format!("organisation/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle tinc-keyset objects (SET-*-TINC pattern)
        if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
            if let Some(content) = self.get_from_storage(&format!("tinc-keyset/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle tinc-key objects (-TINC suffix)
        if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
            if let Some(content) = self.get_from_storage(&format!("tinc-key/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle route-set objects (RS- prefix)
        if normalized_query.starts_with("RS-") {
            if let Some(content) = self.get_from_storage(&format!("route-set/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle as-block objects (AS*-AS* pattern)
        if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
            if let Some(content) = self.get_from_storage(&format!("as-block/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle as-set objects (AS prefix, not an ASN)
        if normalized_query.starts_with("AS") && !normalized_query.chars().skip(2).all(|c| c.is_ascii_digit()) {
            if let Some(content) = self.get_from_storage(&format!("as-set/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle DNS objects (default fallback)
        if let Some(content) = self.get_from_storage(&format!("dns/{}", query.to_lowercase())).await? {
            return Ok(Some(content));
        }
        
        Ok(None)
    }

    /// Handle IP address queries (raw data, no formatting)
    async fn handle_ip_query_raw(&self, query: &str) -> Result<Option<String>> {
        // Parse IPv4 CIDR
        if let Some((ip_str, mask_str)) = query.split_once('/') {
            if let (Ok(ipv4), Ok(mask)) = (ip_str.parse::<Ipv4Addr>(), mask_str.parse::<u8>()) {
                if mask <= 32 {
                    if let Some(target) = self.find_ipv4_network("inetnum", ipv4, mask).await? {
                        return Ok(self.get_from_storage(&format!("inetnum/{}", target)).await?);
                    }
                }
            }
            
            if let (Ok(ipv6), Ok(mask)) = (ip_str.parse::<Ipv6Addr>(), mask_str.parse::<u8>()) {
                if mask <= 128 {
                    if let Some(target) = self.find_ipv6_network("inet6num", ipv6, mask).await? {
                        return Ok(self.get_from_storage(&format!("inet6num/{}", target)).await?);
                    }
                }
            }
        }
        
        // Parse single IP address (assume /32 for IPv4, /128 for IPv6)
        if let Ok(ipv4) = query.parse::<Ipv4Addr>() {
            if let Some(target) = self.find_ipv4_network("inetnum", ipv4, 32).await? {
                return Ok(self.get_from_storage(&format!("inetnum/{}", target)).await?);
            }
        }
        
        if let Ok(ipv6) = query.parse::<Ipv6Addr>() {
            if let Some(target) = self.find_ipv6_network("inet6num", ipv6, 128).await? {
                return Ok(self.get_from_storage(&format!("inet6num/{}", target)).await?);
            }
        }
        
        Ok(None)
    }

    /// Handle direct object lookups (raw data, no formatting)
    async fn handle_object_query_raw(&self, query: &str) -> Result<Option<String>> {
        let normalized_query = query.to_uppercase();
        
        // Handle ASN queries
        if let Some(asn) = parse_asn(&normalized_query) {
            if let Some(content) = self.get_from_storage(&format!("aut-num/{}", asn)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle person objects (-DN42 suffix)
        if normalized_query.ends_with("-DN42") {
            if let Some(content) = self.get_from_storage(&format!("person/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle maintainer objects (-MNT suffix)
        if normalized_query.ends_with("-MNT") {
            if let Some(content) = self.get_from_storage(&format!("mntner/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle schema objects (-SCHEMA suffix)
        if normalized_query.ends_with("-SCHEMA") {
            if let Some(content) = self.get_from_storage(&format!("schema/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle organisation objects (ORG- prefix)
        if normalized_query.starts_with("ORG-") {
            if let Some(content) = self.get_from_storage(&format!("organisation/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle tinc-keyset objects (SET-*-TINC pattern)
        if normalized_query.starts_with("SET-") && normalized_query.ends_with("-TINC") {
            if let Some(content) = self.get_from_storage(&format!("tinc-keyset/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle tinc-key objects (-TINC suffix)
        if normalized_query.ends_with("-TINC") && !normalized_query.starts_with("SET-") {
            if let Some(content) = self.get_from_storage(&format!("tinc-key/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle route-set objects (RS- prefix)
        if normalized_query.starts_with("RS-") {
            if let Some(content) = self.get_from_storage(&format!("route-set/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle as-block objects (AS*-AS* pattern)
        if normalized_query.contains("-AS") && normalized_query.starts_with("AS") {
            if let Some(content) = self.get_from_storage(&format!("as-block/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle as-set objects (AS prefix, not an ASN)
        if normalized_query.starts_with("AS") && !normalized_query.chars().skip(2).all(|c| c.is_ascii_digit()) {
            if let Some(content) = self.get_from_storage(&format!("as-set/{}", normalized_query)).await? {
                return Ok(Some(content));
            }
        }
        
        // Handle DNS objects (default fallback)
        if let Some(content) = self.get_from_storage(&format!("dns/{}", query.to_lowercase())).await? {
            return Ok(Some(content));
        }
        
        Ok(None)
    }

    /// Find the best matching IPv4 network in LMDB storage
    async fn find_ipv4_network(&self, subdir: &str, ip: Ipv4Addr, query_mask: u8) -> Result<Option<String>> {
        debug!("DN42: Searching for IPv4 network in '{}' for IP {} with mask /{}", subdir, ip, query_mask);
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
            let key = format!("{}/{}", subdir, network_str);
            
            debug!("DN42: Checking IPv4 network: {}", network_str);
            if self.key_exists(&key).await? {
                debug!("DN42: Found matching IPv4 network: {}", network_str);
                return Ok(Some(network_str));
            }
        }
        
        debug!("DN42: No matching IPv4 network found in '{}' for IP {}", subdir, ip);
        Ok(None)
    }

    /// Find the best matching IPv6 network in LMDB storage
    async fn find_ipv6_network(&self, subdir: &str, ip: Ipv6Addr, query_mask: u8) -> Result<Option<String>> {
        debug!("DN42: Searching for IPv6 network in '{}' for IP {} with mask /{}", subdir, ip, query_mask);
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
            let key = format!("{}/{}", subdir, network_str);
            
            debug!("DN42: Checking IPv6 network: {}", network_str);
            if self.key_exists(&key).await? {
                debug!("DN42: Found matching IPv6 network: {}", network_str);
                return Ok(Some(network_str));
            }
        }
        
        debug!("DN42: No matching IPv6 network found in '{}' for IP {}", subdir, ip);
        Ok(None)
    }

    /// Get data from LMDB storage
    async fn get_from_storage(&self, key: &str) -> Result<Option<String>> {
        debug!("DN42: Requesting data from LMDB for key: {}", key);
        let storage = self.storage.clone();
        let key_copy = key.to_string();
        let key_for_log = key.to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            storage.get(&key_copy)
        }).await?;
        
        match &result {
            Ok(Some(data)) => debug!("DN42: Retrieved data from LMDB for key '{}', length: {} bytes", key_for_log, data.len()),
            Ok(None) => debug!("DN42: No data found in LMDB for key: {}", key_for_log),
            Err(e) => warn!("DN42: Failed to retrieve data from LMDB for key '{}': {}", key_for_log, e),
        }
        
        result
    }

    /// Check if key exists in LMDB storage
    async fn key_exists(&self, key: &str) -> Result<bool> {
        debug!("DN42: Checking if key exists in LMDB: {}", key);
        let storage = self.storage.clone();
        let key_copy = key.to_string();
        let key_for_log = key.to_string();
        
        let result = tokio::task::spawn_blocking(move || {
            storage.exists(&key_copy)
        }).await?;
        
        match &result {
            Ok(true) => debug!("DN42: Key exists in LMDB: {}", key_for_log),
            Ok(false) => debug!("DN42: Key does not exist in LMDB: {}", key_for_log),
            Err(e) => warn!("DN42: Error checking key existence in LMDB for '{}': {}", key_for_log, e),
        }
        
        result
    }
}

/// Clone the DN42 registry repository using system git command
fn clone_repository() -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(DN42_REGISTRY_PATH).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create git repository parent directory {:?}: {}", parent, e))?;
            info!("Created git repository parent directory: {:?}", parent);
        }
    }
    
    info!("Cloning repository from {} to {}", DN42_REGISTRY_URL, DN42_REGISTRY_PATH);
    
    // Check if git is available
    let git_check = Command::new("git")
        .args(&["--version"])
        .output();
    
    match git_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            debug!("Git version: {}", version.trim());
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git version check failed: {}", stderr));
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Git not found or not executable: {}. Please install git.", e));
        }
    }
    
    let output = Command::new("git")
        .args(&["clone", "--depth", "1", DN42_REGISTRY_URL, DN42_REGISTRY_PATH])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute git clone command: {}", e))?;
    
    if output.status.success() {
        info!("Successfully cloned DN42 registry to {}", DN42_REGISTRY_PATH);
        
        // Log any output from git command
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            debug!("Git clone stdout: {}", stdout);
        }
        
        // Verify the data directory exists
        let data_dir = Path::new(DN42_REGISTRY_PATH).join("data");
        if !data_dir.exists() {
            return Err(anyhow::anyhow!("Cloned repository is missing data directory: {:?}", data_dir));
        }
        
        info!("Verified DN42 registry data directory exists: {:?}", data_dir);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!("Git clone failed - stderr: {}", stderr);
        if !stdout.is_empty() {
            error!("Git clone failed - stdout: {}", stdout);
        }
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
    
    // Reset hard to origin/master (or origin/main)
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
            // Try origin/main if origin/master failed
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("Reset to origin/master failed: {}, trying origin/main", stderr);
            
            let main_output = Command::new("git")
                .args(&["reset", "--hard", "origin/main"])
                .current_dir(DN42_REGISTRY_PATH)
                .output()?;
            
            if main_output.status.success() {
                info!("Successfully reset to origin/main");
                Ok(())
            } else {
                let main_stderr = String::from_utf8_lossy(&main_output.stderr);
                error!("Failed to reset to origin/main: {}", main_stderr);
                Err(anyhow::anyhow!("Git reset failed: {}", main_stderr))
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

// Global DN42 registry instance
use std::sync::OnceLock;
static DN42_REGISTRY_INSTANCE: OnceLock<DN42Registry> = OnceLock::new();

/// Get the global DN42 registry instance
async fn get_dn42_registry() -> Result<&'static DN42Registry> {
    if let Some(registry) = DN42_REGISTRY_INSTANCE.get() {
        Ok(registry)
    } else {
        let registry = DN42Registry::new().await?;
        match DN42_REGISTRY_INSTANCE.set(registry) {
            Ok(_) => Ok(DN42_REGISTRY_INSTANCE.get().unwrap()),
            Err(_) => Ok(DN42_REGISTRY_INSTANCE.get().unwrap()), // Another thread set it
        }
    }
}

/// Initialize DN42 registry system
pub async fn initialize_dn42_system() -> Result<()> {
    let registry = get_dn42_registry().await?;
    registry.initialize().await
}

/// Start the periodic DN42 registry sync task
pub async fn start_periodic_sync() {
    info!("Starting periodic DN42 registry sync (every hour)");
    
    // Initial sync at startup
    if let Err(e) = initialize_dn42_system().await {
        error!("Initial DN42 registry initialization failed: {}", e);
    }
    
    // Set up hourly sync
    let mut interval = interval(Duration::from_secs(3600)); // 1 hour
    interval.tick().await; // Skip the first tick (we just did initial sync)
    
    loop {
        interval.tick().await;
        
        info!("Starting scheduled DN42 registry sync");
        if let Ok(registry) = get_dn42_registry().await {
            if let Err(e) = registry.update().await {
                error!("Scheduled DN42 registry sync failed: {}", e);
            }
        } else {
            error!("Failed to get DN42 registry instance for scheduled sync");
        }
    }
}

/// Process DN42 query using LMDB storage
pub async fn process_dn42_query(query: &str) -> Result<String> {
    let registry = get_dn42_registry().await?;
    registry.query(query).await
}

/// Process DN42 query and return raw data (for email processing)
pub async fn query_dn42_raw(query: &str) -> Result<String> {
    let registry = get_dn42_registry().await?;
    registry.query_raw(query).await
}

/// Blocking version of raw query (for email processing)
#[allow(dead_code)]
pub fn query_dn42_raw_blocking(query: &str) -> Result<String> {
    // For compatibility, we'll use a blocking approach
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(query_dn42_raw(query))
    })
}

/// Blocking version for compatibility
#[allow(dead_code)]
pub fn process_dn42_query_blocking(query: &str) -> Result<String> {
    // For compatibility, we'll use a blocking approach
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(process_dn42_query(query))
    })
}

/// Force full refresh of DN42 registry (clear and repopulate LMDB)
#[allow(dead_code)]
pub async fn force_full_refresh_dn42() -> Result<()> {
    let registry = get_dn42_registry().await?;
    registry.force_full_refresh().await
} 