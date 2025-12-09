use anyhow::Result;
use tracing::{ debug, info };

use crate::config::DN42_LMDB_PATH;
use crate::dn42::online_backend::{ DN42OnlineFetcher, get_platform_info, is_windows };
use crate::dn42::query::{
    DN42QueryType,
    format_ipv4_network_response,
    format_ipv6_network_response,
    format_query_response,
};
use crate::storage::{ SharedLmdbStorage, create_shared_storage };

/// DN42 platform-aware manager that automatically selects Git or online mode
pub struct DN42Manager {
    mode: DN42Mode,
}

enum DN42Mode {
    Online(DN42OnlineFetcher),
    Git(SharedLmdbStorage), // For non-Windows systems using the existing git-based approach
}

impl DN42Manager {
    /// Create a new DN42 manager with automatic platform detection
    pub async fn new() -> Result<Self> {
        let mode = if is_windows() {
            info!("DN42 Manager: Platform detected: {}", get_platform_info());
            info!("DN42 Manager: Using online file access mode for Windows");
            let fetcher = DN42OnlineFetcher::new()?;
            DN42Mode::Online(fetcher)
        } else {
            info!("DN42 Manager: Platform detected: {}", get_platform_info());
            info!("DN42 Manager: Using git repository mode for Unix-like systems");
            let storage = create_shared_storage(DN42_LMDB_PATH).map_err(|e|
                anyhow::anyhow!("Failed to create LMDB storage: {}", e)
            )?;
            DN42Mode::Git(storage)
        };

        Ok(DN42Manager { mode })
    }

    /// Initialize the DN42 manager
    pub async fn initialize(&mut self) -> Result<()> {
        match &mut self.mode {
            DN42Mode::Online(fetcher) => {
                info!("DN42 Manager: Initializing online mode");
                fetcher.initialize().await?;
                info!("DN42 Manager: Online mode initialization completed");
            }
            DN42Mode::Git(_storage) => {
                info!(
                    "DN42 Manager: Git mode detected - initialization handled by existing DN42 system"
                );
                // Git mode initialization is handled by the existing dn42.rs system
            }
        }
        Ok(())
    }

    /// Process DN42 query and return formatted response
    pub async fn query(&mut self, query: &str) -> Result<String> {
        debug!("DN42 Manager: Processing query: {}", query);

        match &mut self.mode {
            DN42Mode::Online(fetcher) => DN42Manager::query_online_static(fetcher, query).await,
            DN42Mode::Git(_storage) => DN42Manager::query_git_static(query).await,
        }
    }

    /// Process DN42 query and return raw data (for email processing)
    pub async fn query_raw(&mut self, query: &str) -> Result<String> {
        debug!("DN42 Manager: Processing raw query: {}", query);

        match &mut self.mode {
            DN42Mode::Online(fetcher) => DN42Manager::query_raw_online_static(fetcher, query).await,
            DN42Mode::Git(_storage) => DN42Manager::query_raw_git_static(query).await,
        }
    }

    /// Perform maintenance tasks (cleanup cache for online mode)
    pub async fn maintenance(&mut self) -> Result<()> {
        match &mut self.mode {
            DN42Mode::Online(fetcher) => {
                info!("DN42 Manager: Running online mode maintenance (cache cleanup)");
                fetcher.cleanup_cache().await?;
                info!("DN42 Manager: Online mode maintenance completed");
            }
            DN42Mode::Git(_storage) => {
                debug!("DN42 Manager: Git mode maintenance handled by existing DN42 system");
                // Git mode maintenance is handled by the existing dn42.rs system
            }
        }
        Ok(())
    }

    /// Get platform information
    pub fn get_platform_info(&self) -> &'static str {
        get_platform_info()
    }

    /// Check if running in online mode
    pub fn is_online_mode(&self) -> bool {
        matches!(self.mode, DN42Mode::Online(_))
    }

    /// Query using online fetcher
    async fn query_online_static(fetcher: &mut DN42OnlineFetcher, query: &str) -> Result<String> {
        let query_type = DN42QueryType::parse(query);
        debug!("DN42 Online: Parsed query type: {:?}", query_type);

        match query_type {
            DN42QueryType::IPv4Network { ip, mask } => {
                // Fetch inetnum data
                let inetnum_content = fetcher.find_ipv4_network("inetnum", ip, mask).await?;

                // Fetch route data
                let route_content = fetcher.find_ipv4_network("route", ip, mask).await?;

                Ok(format_ipv4_network_response(query, inetnum_content, route_content))
            }
            DN42QueryType::IPv6Network { ip, mask } => {
                // Fetch inet6num data
                let inet6num_content = fetcher.find_ipv6_network("inet6num", ip, mask).await?;

                // Fetch route6 data
                let route6_content = fetcher.find_ipv6_network("route6", ip, mask).await?;

                Ok(format_ipv6_network_response(query, inet6num_content, route6_content))
            }
            _ => {
                // For other query types, fetch the object directly
                let object_type = query_type.get_object_type();
                let file_name = query_type.get_file_name();

                let content = fetcher.fetch_file(object_type, &file_name).await?;
                Ok(format_query_response(query, content))
            }
        }
    }

    /// Query raw data using online fetcher
    async fn query_raw_online_static(
        fetcher: &mut DN42OnlineFetcher,
        query: &str
    ) -> Result<String> {
        let query_type = DN42QueryType::parse(query);

        match query_type {
            DN42QueryType::IPv4Network { ip, mask } => {
                if let Some(content) = fetcher.find_ipv4_network("inetnum", ip, mask).await? {
                    Ok(content)
                } else {
                    Ok(String::new())
                }
            }
            DN42QueryType::IPv6Network { ip, mask } => {
                if let Some(content) = fetcher.find_ipv6_network("inet6num", ip, mask).await? {
                    Ok(content)
                } else {
                    Ok(String::new())
                }
            }
            _ => {
                let object_type = query_type.get_object_type();
                let file_name = query_type.get_file_name();

                if let Some(content) = fetcher.fetch_file(object_type, &file_name).await? {
                    Ok(content)
                } else {
                    Ok(String::new())
                }
            }
        }
    }

    /// Query using git-based LMDB storage
    async fn query_git_static(query: &str) -> Result<String> {
        debug!("DN42 Manager: Processing Git mode query: {}", query);

        // Use git backend's process_dn42_query which already implements LMDB querying
        crate::dn42::git_backend::process_dn42_query(query).await
    }

    /// Query raw data using git-based LMDB storage
    async fn query_raw_git_static(query: &str) -> Result<String> {
        debug!("DN42 Manager: Processing Git mode raw query: {}", query);

        // Use git backend's query_dn42_raw which already implements LMDB querying
        crate::dn42::git_backend::query_dn42_raw(query).await
    }
}

// Global DN42 manager instance
use std::sync::OnceLock;
use tokio::sync::Mutex;
static DN42_MANAGER_INSTANCE: OnceLock<Mutex<DN42Manager>> = OnceLock::new();

/// Get the global DN42 manager instance
async fn get_dn42_manager() -> Result<&'static Mutex<DN42Manager>> {
    if let Some(manager) = DN42_MANAGER_INSTANCE.get() {
        Ok(manager)
    } else {
        let manager = DN42Manager::new().await?;
        let mutex = Mutex::new(manager);
        match DN42_MANAGER_INSTANCE.set(mutex) {
            Ok(_) => Ok(DN42_MANAGER_INSTANCE.get().expect("Manager should be set after successful initialization")),
            Err(_) => DN42_MANAGER_INSTANCE.get().ok_or_else(|| anyhow::anyhow!("Failed to get DN42 manager instance after set")),
        }
    }
}

/// Initialize DN42 manager system
pub async fn initialize_dn42_manager() -> Result<()> {
    let manager_mutex = get_dn42_manager().await?;
    let mut manager = manager_mutex.lock().await;
    manager.initialize().await
}

/// Process DN42 query using the manager
pub async fn process_dn42_query_managed(query: &str) -> Result<String> {
    let manager_mutex = get_dn42_manager().await?;
    let mut manager = manager_mutex.lock().await;
    manager.query(query).await
}

/// Process DN42 raw query using the manager
pub async fn query_dn42_raw_managed(query: &str) -> Result<String> {
    let manager_mutex = get_dn42_manager().await?;
    let mut manager = manager_mutex.lock().await;
    manager.query_raw(query).await
}

/// Run DN42 manager maintenance tasks
pub async fn dn42_manager_maintenance() -> Result<()> {
    let manager_mutex = get_dn42_manager().await?;
    let mut manager = manager_mutex.lock().await;
    manager.maintenance().await
}

/// Get platform information from the manager
pub async fn get_dn42_platform_info() -> Result<&'static str> {
    let manager_mutex = get_dn42_manager().await?;
    let manager = manager_mutex.lock().await;
    Ok(manager.get_platform_info())
}

/// Check if DN42 manager is running in online mode
pub async fn is_dn42_online_mode() -> Result<bool> {
    let manager_mutex = get_dn42_manager().await?;
    let manager = manager_mutex.lock().await;
    Ok(manager.is_online_mode())
}
