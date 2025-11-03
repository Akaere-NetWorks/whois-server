use crate::storage::lmdb::LmdbStorage;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// IANA Private Enterprise Number entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenEntry {
    /// Enterprise number (e.g., 9, 64537)
    pub number: u32,
    /// Organization name
    pub organization: String,
    /// Contact person name
    pub contact: String,
    /// Contact email
    pub email: String,
    /// Full OID (1.3.6.1.4.1.{number})
    pub oid: String,
    /// When this entry was cached
    pub cached_at: u64,
}

impl PenEntry {
    pub fn new(number: u32, organization: String, contact: String, email: String) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let oid = format!("1.3.6.1.4.1.{}", number);

        Self {
            number,
            organization,
            contact,
            email,
            oid,
            cached_at,
        }
    }

    /// Format as WHOIS-style output
    pub fn to_whois_format(&self) -> String {
        format!(
            "% IANA Private Enterprise Number (PEN) Information\n\
             % https://www.iana.org/assignments/enterprise-numbers\n\
             \n\
             Enterprise-Number: {}\n\
             OID: {}\n\
             OID-Prefix: iso.org.dod.internet.private.enterprise (1.3.6.1.4.1)\n\
             Organization: {}\n\
             Contact: {}\n\
             Email: {}\n\
             \n\
             % This information is provided for informational purposes only.\n\
             % Data source: IANA Enterprise Numbers Registry\n\
             % Last updated: {}",
            self.number,
            self.oid,
            self.organization,
            self.contact,
            self.email,
            chrono::DateTime::<chrono::Utc>::from(
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(self.cached_at)
            )
            .format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Check if cache entry is expired (older than 30 days)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 30 days = 30 * 24 * 60 * 60 = 2592000 seconds
        now - self.cached_at > 2592000
    }
}

pub struct PenService {
    storage: LmdbStorage,
    data_url: String,
}

// Global PEN cache update state
static PEN_UPDATE_RUNNING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl PenService {
    pub fn new() -> Result<Self> {
        let storage = LmdbStorage::new("./cache/pen_cache")?;
        let data_url = "https://www.iana.org/assignments/enterprise-numbers.txt".to_string();

        Ok(Self { storage, data_url })
    }

    /// Check if cache needs update (older than 1 day)
    pub fn needs_update(&self) -> Result<bool> {
        let last_update_key = "pen_last_update";

        match self.storage.get_json::<u64>(&last_update_key) {
            Ok(Some(last_update)) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Update if older than 1 day (86400 seconds)
                Ok(now - last_update > 86400)
            }
            _ => Ok(true), // No timestamp found, need to update
        }
    }

    /// Force update cache data
    pub async fn force_update(&self) -> Result<()> {
        info!("Force updating IANA Private Enterprise Numbers data...");

        // Download the file content
        let content = self.download_pen_data().await?;

        // Cache the entire file content
        let file_cache_key = "pen_file_content";
        self.storage.put(file_cache_key, &content)?;

        // Parse and cache individual entries (with batch processing)
        self.parse_pen_data_batched(&content).await?;

        // Update timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_update_key = "pen_last_update";
        self.storage.put_json(last_update_key, &now)?;

        info!("PEN cache updated successfully");
        Ok(())
    }

    /// Query PEN by exact number
    pub async fn query_by_number(&self, number: u32) -> Result<Option<String>> {
        let cache_key = format!("pen_{}", number);

        // Try cache first
        match self.storage.get_json::<PenEntry>(&cache_key) {
            Ok(Some(entry)) if !entry.is_expired() => {
                debug!("PEN cache hit for number {}", number);
                return Ok(Some(entry.to_whois_format()));
            }
            Ok(Some(_)) => {
                debug!("PEN cache entry expired for number {}", number);
                let _ = self.storage.delete(&cache_key);
            }
            Ok(None) => {
                debug!("PEN cache miss for number {}", number);
            }
            Err(e) => {
                warn!("Failed to read PEN cache for number {}: {}", number, e);
            }
        }

        // Check if data exists but entry is missing (may need to re-parse)
        self.ensure_data_available().await?;

        // Try cache again after ensuring data is available
        match self.storage.get_json::<PenEntry>(&cache_key) {
            Ok(Some(entry)) => Ok(Some(entry.to_whois_format())),
            _ => Ok(None),
        }
    }

    /// Search PEN by organization, contact name, or email (fuzzy search)
    pub async fn search_by_name(&self, query: &str) -> Result<Vec<String>> {
        // Ensure data is available before searching
        self.ensure_data_available().await?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let max_results = 20; // Limit results to avoid overwhelming output

        let keys = self.storage.list_keys()?;
        for key in keys {
            if !key.starts_with("pen_") {
                continue;
            }

            if let Ok(Some(entry)) = self.storage.get_json::<PenEntry>(&key) {
                let org_lower = entry.organization.to_lowercase();
                let contact_lower = entry.contact.to_lowercase();
                let email_lower = entry.email.to_lowercase();

                // Fuzzy matching: check if query is contained in org, contact, or email
                if org_lower.contains(&query_lower) 
                    || contact_lower.contains(&query_lower)
                    || email_lower.contains(&query_lower)
                {
                    results.push(entry.to_whois_format());

                    if results.len() >= max_results {
                        results.push(format!(
                            "\n% Search limited to {} results. Please refine your query for more specific results.",
                            max_results
                        ));
                        break;
                    }
                }
            }
        }

        if results.is_empty() {
            Ok(vec![format!(
                "% No IANA Private Enterprise Numbers found matching: {}\n\
                 % Please try a different search term or use exact PEN number query.",
                query
            )])
        } else {
            Ok(results)
        }
    }

    /// Ensure PEN data is available (check if parsed entries exist, re-parse if needed)
    async fn ensure_data_available(&self) -> Result<()> {
        // Quick check: see if any entries exist
        let sample_key = "pen_1";
        if self
            .storage
            .get_json::<PenEntry>(&sample_key)
            .ok()
            .flatten()
            .is_some()
        {
            // Data exists, no need to do anything
            return Ok(());
        }

        debug!("PEN entries not found, checking for cached file");

        // Check if we have cached file content
        let file_cache_key = "pen_file_content";
        if let Ok(Some(content)) = self.storage.get(file_cache_key) {
            debug!("Re-parsing PEN data from cached file");
            self.parse_pen_data_batched(&content).await?;
            return Ok(());
        }

        // No cached data at all, need to download
        // This should only happen if the periodic update task hasn't run yet
        warn!("No PEN cache found, triggering initial download");
        self.force_update().await?;

        Ok(())
    }

    /// Refresh PEN data from IANA if cache is empty or stale
    /// Note: This is now primarily used by the periodic update task
    #[allow(dead_code)]
    async fn refresh_data_if_needed(&self) -> Result<()> {
        // Check if we have cached file content
        let file_cache_key = "pen_file_content";
        let last_update_key = "pen_last_update";

        let should_refresh = match self.storage.get_json::<u64>(&last_update_key) {
            Ok(Some(last_update)) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Refresh if older than 1 day (86400 seconds)
                now - last_update > 86400
            }
            _ => true, // No timestamp found, need to refresh
        };

        if should_refresh {
            // Use force_update to handle the update
            self.force_update().await?;
        } else {
            // Check if we have parsed entries, if not parse from cached file
            let sample_key = "pen_1";
            if self
                .storage
                .get_json::<PenEntry>(&sample_key)
                .ok()
                .flatten()
                .is_none()
            {
                // We have file cache but no parsed entries, re-parse
                if let Ok(Some(content)) = self.storage.get(file_cache_key) {
                    debug!("Re-parsing PEN data from cached file");
                    self.parse_pen_data_batched(&content).await?;
                }
            }
        }

        Ok(())
    }

    /// Download PEN data from IANA with custom User-Agent
    async fn download_pen_data(&self) -> Result<String> {
        info!(
            "Downloading IANA Private Enterprise Numbers from {}",
            self.data_url
        );

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36")
            .build()?;

        let response = client.get(&self.data_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download PEN data: HTTP {}",
                response.status()
            ));
        }

        let content = response.text().await?;
        info!("Downloaded {} bytes of PEN data", content.len());

        Ok(content)
    }

    /// Parse PEN data format with batched processing (10000 entries at a time):
    /// ```
    /// Decimal
    /// | Organization
    /// | | Contact
    /// | | | Email
    /// | | | |
    /// 0
    ///   Reserved
    ///     Internet Assigned Numbers Authority
    ///       iana&iana.org
    /// 1
    ///   NxNetworks
    ///     Michael Kellen
    ///       OID.Admin&NxNetworks.com
    /// ```
    async fn parse_pen_data_batched(&self, content: &str) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        let mut count = 0;
        let batch_size = 10000;
        let mut batch_entries: Vec<(String, PenEntry)> = Vec::with_capacity(batch_size);

        // Skip header until we find the first numeric line
        while i < lines.len() {
            let line = lines[i].trim();
            if line.parse::<u32>().is_ok() {
                break;
            }
            i += 1;
        }

        // Parse entries
        while i < lines.len() {
            let line = lines[i].trim();

            // Try to parse as enterprise number
            if let Ok(number) = line.parse::<u32>() {
                // Next 3 lines should be organization, contact, email (with 2-space indent)
                if i + 3 < lines.len() {
                    let org_line = lines[i + 1];
                    let contact_line = lines[i + 2];
                    let email_line = lines[i + 3];

                    // Check if lines start with spaces (indentation)
                    if org_line.starts_with("  ")
                        && contact_line.starts_with("    ")
                        && email_line.starts_with("      ")
                    {
                        let organization = org_line.trim().to_string();
                        let contact = contact_line.trim().to_string();
                        let email = email_line.trim().replace('&', "@");

                        let entry = PenEntry::new(number, organization, contact, email);
                        let cache_key = format!("pen_{}", number);

                        batch_entries.push((cache_key, entry));
                        count += 1;

                        // Process batch when it reaches batch_size
                        if batch_entries.len() >= batch_size {
                            self.store_batch(&batch_entries).await?;
                            info!("Cached {} PEN entries (batch processed)", count);
                            batch_entries.clear();
                        }

                        i += 4; // Skip to next entry
                        continue;
                    }
                }
            }

            i += 1;
        }

        // Store remaining entries in the last batch
        if !batch_entries.is_empty() {
            self.store_batch(&batch_entries).await?;
        }

        info!("Successfully cached {} PEN entries (total)", count);
        Ok(())
    }

    /// Store a batch of entries to LMDB
    async fn store_batch(&self, entries: &[(String, PenEntry)]) -> Result<()> {
        for (cache_key, entry) in entries {
            if let Err(e) = self.storage.put_json(cache_key, entry) {
                warn!("Failed to cache PEN entry for key {}: {}", cache_key, e);
            }
        }
        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
        Ok(())
    }

    /// Handle -pen query
    pub async fn handle_query(&self, query: &str) -> Result<String> {
        let query = query.trim();

        // Try to parse as number first (exact match)
        if let Ok(number) = query.parse::<u32>() {
            if let Some(result) = self.query_by_number(number).await? {
                return Ok(result);
            } else {
                return Ok(format!(
                    "% IANA Private Enterprise Number {} not found.\n\
                     % The number may not be assigned yet, or the database needs updating.",
                    number
                ));
            }
        }

        // Otherwise, treat as name search (fuzzy)
        let results = self.search_by_name(query).await?;

        if results.is_empty() {
            Ok(format!(
                "% No results found for query: {}\n\
                 % Try searching by enterprise number or organization name.",
                query
            ))
        } else {
            Ok(results.join("\n\n"))
        }
    }
}

/// Process PEN query (public function for use in query_processor)
pub async fn process_pen_query(query: &str) -> Result<String> {
    let service = PenService::new()?;
    service.handle_query(query).await
}

/// Check if PEN cache needs update (for periodic maintenance)
pub async fn pen_needs_update() -> Result<bool> {
    let service = PenService::new()?;
    service.needs_update()
}

/// Perform PEN cache update (for periodic maintenance)
pub async fn pen_update_cache() -> Result<()> {
    // Use atomic flag to prevent concurrent updates
    if PEN_UPDATE_RUNNING
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_err()
    {
        info!("PEN cache update already in progress, skipping");
        return Ok(());
    }

    let result = async {
        let service = PenService::new()?;
        service.force_update().await
    }
    .await;

    // Release the lock
    PEN_UPDATE_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);

    result
}

/// Start periodic PEN cache update task (call this from main.rs)
pub async fn start_pen_periodic_update() {
    use tokio::time::{interval, Duration};

    info!("Starting PEN periodic update task (checking every hour)");
    
    // Immediately check and update on startup
    info!("PEN: Performing initial cache check on startup");
    match pen_needs_update().await {
        Ok(true) => {
            info!("PEN cache needs initial update, starting download...");
            if let Err(e) = pen_update_cache().await {
                warn!("Failed to perform initial PEN cache update: {}", e);
            } else {
                info!("PEN cache initial update completed successfully");
            }
        }
        Ok(false) => {
            info!("PEN cache is up to date on startup");
        }
        Err(e) => {
            warn!("Failed to check PEN update status on startup: {}", e);
        }
    }

    let mut check_interval = interval(Duration::from_secs(3600)); // Check every hour
    check_interval.tick().await; // Skip the first tick

    loop {
        check_interval.tick().await;

        match pen_needs_update().await {
            Ok(true) => {
                info!("PEN cache needs update, starting update...");
                if let Err(e) = pen_update_cache().await {
                    warn!("Failed to update PEN cache: {}", e);
                } else {
                    info!("PEN cache updated successfully");
                }
            }
            Ok(false) => {
                debug!("PEN cache is up to date");
            }
            Err(e) => {
                warn!("Failed to check PEN update status: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pen_entry_creation() {
        let entry = PenEntry::new(
            9,
            "ciscoSystems".to_string(),
            "Dave Jones".to_string(),
            "davej@cisco.com".to_string(),
        );

        assert_eq!(entry.number, 9);
        assert_eq!(entry.oid, "1.3.6.1.4.1.9");
        assert_eq!(entry.organization, "ciscoSystems");
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_whois_format() {
        let entry = PenEntry::new(
            64537,
            "AKAERE NETWORKS TECHNOLOGY LTD".to_string(),
            "Liu HaoRan".to_string(),
            "qq593277393@outlook.com".to_string(),
        );

        let output = entry.to_whois_format();
        assert!(output.contains("Enterprise-Number: 64537"));
        assert!(output.contains("OID: 1.3.6.1.4.1.64537"));
        assert!(output.contains("AKAERE NETWORKS"));
        assert!(output.contains("iso.org.dod.internet.private.enterprise (1.3.6.1.4.1)"));
    }

    #[test]
    fn test_pen_entry_expiration() {
        let mut entry = PenEntry::new(
            1,
            "Test".to_string(),
            "Test Contact".to_string(),
            "test@example.com".to_string(),
        );

        assert!(!entry.is_expired());

        // Set cached_at to 31 days ago
        entry.cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (31 * 24 * 60 * 60);

        assert!(entry.is_expired());
    }
}
