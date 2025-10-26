use anyhow::Result;
use reqwest::Client;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

use crate::config::DN42_LMDB_PATH;
use crate::storage::{SharedLmdbStorage, create_shared_storage};

const DN42_RAW_BASE_URL: &str = "https://git.pysio.online/pysio/mirrors-dn42/-/raw/master/data";
const CACHE_EXPIRATION_SECONDS: u64 = 86400; // 1 day
const CACHE_PREFIX: &str = "online_cache:";
const TIMESTAMP_PREFIX: &str = "timestamp:";

#[derive(Debug)]
pub struct DN42OnlineFetcher {
    client: Client,
    storage: SharedLmdbStorage,
}

impl DN42OnlineFetcher {
    /// Create a new DN42 online fetcher instance
    pub fn new() -> Result<Self> {
        let cache_db_path = format!("{}/online_cache", DN42_LMDB_PATH);
        let storage = create_shared_storage(&cache_db_path).map_err(|e| {
            anyhow::anyhow!("Failed to create LMDB storage for online cache: {}", e)
        })?;

        Ok(DN42OnlineFetcher {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("whois-server/1.0")
                .build()?,
            storage,
        })
    }

    /// Initialize the online fetcher (create cache database)
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing DN42 online fetcher with LMDB cache");

        // Test LMDB storage access
        let test_key = format!("{}test", CACHE_PREFIX);
        let storage = self.storage.clone();

        tokio::task::spawn_blocking(move || storage.put(&test_key, "test_value"))
            .await?
            .map_err(|e| anyhow::anyhow!("Failed to test LMDB cache storage: {}", e))?;

        info!("DN42 online fetcher initialized successfully with LMDB cache");
        Ok(())
    }

    /// Fetch a file from DN42 registry (with LMDB caching)
    pub async fn fetch_file(
        &mut self,
        object_type: &str,
        file_name: &str,
    ) -> Result<Option<String>> {
        let cache_key = format!("{}{}/{}", CACHE_PREFIX, object_type, file_name);
        let timestamp_key = format!("{}{}", TIMESTAMP_PREFIX, cache_key);
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Check cache first
        let storage = self.storage.clone();
        let cache_key_clone = cache_key.clone();
        let timestamp_key_clone = timestamp_key.clone();

        let cache_result = tokio::task::spawn_blocking(move || -> Result<Option<String>> {
            let content = storage.get(&cache_key_clone)?;
            let timestamp_str = storage.get(&timestamp_key_clone)?;

            if let (Some(content), Some(timestamp_str)) = (content, timestamp_str)
                && let Ok(timestamp) = timestamp_str.parse::<u64>()
                && current_time - timestamp < CACHE_EXPIRATION_SECONDS
            {
                return Ok(Some(content));
            }
            Ok(None)
        })
        .await??;

        if let Some(cached_content) = cache_result {
            debug!("DN42 Online: Cache hit for {}/{}", object_type, file_name);
            return Ok(Some(cached_content));
        }

        // Fetch from online
        debug!(
            "DN42 Online: Fetching {}/{} from remote",
            object_type, file_name
        );
        let url = format!("{}/{}/{}", DN42_RAW_BASE_URL, object_type, file_name);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(content) => {
                            info!(
                                "DN42 Online: Successfully fetched {}/{}, size: {} bytes",
                                object_type,
                                file_name,
                                content.len()
                            );

                            // Store in LMDB cache
                            let storage = self.storage.clone();
                            let content_clone = content.clone();
                            let timestamp_str = current_time.to_string();

                            tokio::task::spawn_blocking(move || {
                                storage.put(&cache_key, &content_clone)?;
                                storage.put(&timestamp_key, &timestamp_str)?;
                                Ok::<(), anyhow::Error>(())
                            })
                            .await?
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to cache content in LMDB: {}", e)
                            })?;

                            Ok(Some(content))
                        }
                        Err(e) => {
                            warn!(
                                "DN42 Online: Failed to read response body for {}/{}: {}",
                                object_type, file_name, e
                            );
                            Ok(None)
                        }
                    }
                } else if response.status().as_u16() == 404 {
                    debug!("DN42 Online: File not found: {}/{}", object_type, file_name);
                    Ok(None)
                } else {
                    warn!(
                        "DN42 Online: HTTP error {} for {}/{}",
                        response.status(),
                        object_type,
                        file_name
                    );
                    Ok(None)
                }
            }
            Err(e) => {
                error!(
                    "DN42 Online: Network error fetching {}/{}: {}",
                    object_type, file_name, e
                );
                Ok(None)
            }
        }
    }

    /// Search for IPv4 network file by trying different CIDR blocks
    pub async fn find_ipv4_network(
        &mut self,
        object_type: &str,
        ip: std::net::Ipv4Addr,
        query_mask: u8,
    ) -> Result<Option<String>> {
        debug!(
            "DN42 Online: Searching for IPv4 network in '{}' for IP {} with mask /{}",
            object_type, ip, query_mask
        );
        let ip_int = u32::from(ip);

        // Search from the query mask down to /0
        for mask in (0..=query_mask).rev() {
            let network_int = if mask > 0 {
                ip_int & (0xffffffff << (32 - mask))
            } else {
                0
            };

            let network_ip = std::net::Ipv4Addr::from(network_int);
            let network_str = format!("{},{}", network_ip, mask);

            debug!("DN42 Online: Checking IPv4 network file: {}", network_str);
            if let Some(content) = self.fetch_file(object_type, &network_str).await? {
                debug!("DN42 Online: Found matching IPv4 network: {}", network_str);
                return Ok(Some(content));
            }
        }

        debug!(
            "DN42 Online: No matching IPv4 network found in '{}' for IP {}",
            object_type, ip
        );
        Ok(None)
    }

    /// Search for IPv6 network file by trying different CIDR blocks
    pub async fn find_ipv6_network(
        &mut self,
        object_type: &str,
        ip: std::net::Ipv6Addr,
        query_mask: u8,
    ) -> Result<Option<String>> {
        debug!(
            "DN42 Online: Searching for IPv6 network in '{}' for IP {} with mask /{}",
            object_type, ip, query_mask
        );
        let ip_int = u128::from(ip);

        // Search from the query mask down to /0
        for mask in (0..=query_mask).rev() {
            let network_int = if mask > 0 {
                ip_int & (0xffffffffffffffffffffffffffffffff << (128 - mask))
            } else {
                0
            };

            let network_ip = std::net::Ipv6Addr::from(network_int);
            let network_str = format!("{},{}", network_ip, mask);

            debug!("DN42 Online: Checking IPv6 network file: {}", network_str);
            if let Some(content) = self.fetch_file(object_type, &network_str).await? {
                debug!("DN42 Online: Found matching IPv6 network: {}", network_str);
                return Ok(Some(content));
            }
        }

        debug!(
            "DN42 Online: No matching IPv6 network found in '{}' for IP {}",
            object_type, ip
        );
        Ok(None)
    }

    /// Cleanup expired cache entries from LMDB
    pub async fn cleanup_cache(&mut self) -> Result<()> {
        info!("DN42 Online: Starting LMDB cache cleanup");
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let storage = self.storage.clone();

        let cleanup_result = tokio::task::spawn_blocking(move || {
            let mut expired_keys = Vec::new();

            // Iterate through all keys to find expired cache entries
            storage.iterate_keys(CACHE_PREFIX, |key| {
                let timestamp_key = format!("{}{}", TIMESTAMP_PREFIX, key);
                if let Ok(Some(timestamp_str)) = storage.get(&timestamp_key)
                    && let Ok(timestamp) = timestamp_str.parse::<u64>()
                    && current_time - timestamp >= CACHE_EXPIRATION_SECONDS
                {
                    expired_keys.push((key.to_string(), timestamp_key));
                }
                true // Continue iteration
            })?;

            // Remove expired entries
            for (cache_key, timestamp_key) in &expired_keys {
                storage.delete(cache_key)?;
                storage.delete(timestamp_key)?;
            }

            Ok::<usize, anyhow::Error>(expired_keys.len())
        })
        .await?;

        match cleanup_result {
            Ok(removed_count) => {
                if removed_count > 0 {
                    info!(
                        "DN42 Online: Cache cleanup completed, removed {} expired entries",
                        removed_count
                    );
                } else {
                    debug!("DN42 Online: Cache cleanup completed, no expired entries found");
                }
            }
            Err(e) => {
                warn!("DN42 Online: Cache cleanup failed: {}", e);
            }
        }

        Ok(())
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub async fn get_cache_stats(&self) -> Result<(usize, usize)> {
        let storage = self.storage.clone();

        tokio::task::spawn_blocking(move || {
            let mut total_entries = 0;
            let mut expired_entries = 0;
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            storage.iterate_keys(CACHE_PREFIX, |key| {
                total_entries += 1;
                let timestamp_key = format!("{}{}", TIMESTAMP_PREFIX, key);
                if let Ok(Some(timestamp_str)) = storage.get(&timestamp_key)
                    && let Ok(timestamp) = timestamp_str.parse::<u64>()
                    && current_time - timestamp >= CACHE_EXPIRATION_SECONDS
                {
                    expired_entries += 1;
                }
                true // Continue iteration
            })?;

            Ok::<(usize, usize), anyhow::Error>((total_entries, expired_entries))
        })
        .await?
    }
}

/// Check if the current platform is Windows
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

/// Get platform-specific message
pub fn get_platform_info() -> &'static str {
    if is_windows() {
        "Windows (using online file access with LMDB cache)"
    } else {
        "Unix-like (using git repository)"
    }
}
