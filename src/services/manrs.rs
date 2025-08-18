/*
 * MANRS (Mutually Agreed Norms for Routing Security) Integration
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

use anyhow::Result;
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::collections::HashSet;
use std::time::{ Duration, SystemTime, UNIX_EPOCH };
use tracing::{ debug, info, warn, error };
use chrono::DateTime;
use crate::storage::{ SharedLmdbStorage, create_shared_storage };

const MANRS_API_URL: &str = "https://api.manrs.org/asns";
const MANRS_LMDB_PATH: &str = "./cache/manrs_lmdb";
const CACHE_KEY: &str = "manrs_asns";
const CACHE_TIMESTAMP_KEY: &str = "manrs_last_updated";
const CACHE_DURATION_DAYS: u64 = 14;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManrsApiResponse {
    asns: Vec<u64>,
}

pub struct ManrsChecker {
    storage: SharedLmdbStorage,
    client: Client,
}

impl ManrsChecker {
    pub fn new(storage: SharedLmdbStorage) -> Self {
        Self {
            storage,
            client: Client::new(),
        }
    }

    fn is_cache_expired(&self) -> Result<bool> {
        match self.storage.get(CACHE_TIMESTAMP_KEY)? {
            Some(timestamp_str) => {
                let last_updated = timestamp_str
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("Invalid timestamp format in cache"))?;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let cache_age_seconds = now.saturating_sub(last_updated);
                let cache_duration_seconds = CACHE_DURATION_DAYS * 24 * 60 * 60;

                Ok(cache_age_seconds > cache_duration_seconds)
            }
            None => Ok(true), // No cache exists
        }
    }

    fn get_cached_asns(&self) -> Result<Option<HashSet<u64>>> {
        match self.storage.get(CACHE_KEY)? {
            Some(asns_json) => {
                let asns: Vec<u64> = serde_json::from_str(&asns_json)?;
                Ok(Some(asns.into_iter().collect()))
            }
            None => Ok(None),
        }
    }

    fn save_asns_to_cache(&self, asns: &[u64]) -> Result<()> {
        let asns_json = serde_json::to_string(asns)?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        self.storage.put(CACHE_KEY, &asns_json)?;
        self.storage.put(CACHE_TIMESTAMP_KEY, &now.to_string())?;

        info!("Saved {} MANRS ASNs to LMDB cache", asns.len());
        Ok(())
    }

    async fn refresh_cache(&self) -> Result<HashSet<u64>> {
        info!("Refreshing MANRS ASN cache from API...");

        let response = self.client
            .get(MANRS_API_URL)
            .timeout(Duration::from_secs(30))
            .header("User-Agent", "whois-server/0.1.0")
            .send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("MANRS API returned status: {}", response.status()));
        }

        let api_response: ManrsApiResponse = response.json().await?;

        info!("Retrieved {} MANRS member ASNs from API", api_response.asns.len());

        self.save_asns_to_cache(&api_response.asns)?;

        Ok(api_response.asns.into_iter().collect())
    }

    pub async fn check_asn(&self, asn: u64) -> Result<ManrsStatus> {
        let asns = if self.is_cache_expired()? {
            debug!("MANRS cache is expired, refreshing...");
            match self.refresh_cache().await {
                Ok(asns) => asns,
                Err(e) => {
                    error!("Failed to refresh MANRS cache: {}", e);
                    // Try to use expired cache if available
                    match self.get_cached_asns()? {
                        Some(cached_asns) => {
                            warn!("Using expired MANRS cache due to API failure");
                            cached_asns
                        }
                        None => {
                            return Ok(ManrsStatus::Unknown);
                        }
                    }
                }
            }
        } else {
            match self.get_cached_asns()? {
                Some(cached_asns) => {
                    debug!("Using cached MANRS data");
                    cached_asns
                }
                None => {
                    debug!("No MANRS cache found, refreshing...");
                    match self.refresh_cache().await {
                        Ok(asns) => asns,
                        Err(e) => {
                            error!("Failed to refresh MANRS cache: {}", e);
                            return Ok(ManrsStatus::Unknown);
                        }
                    }
                }
            }
        };

        let is_member = asns.contains(&asn);
        let total_members = asns.len();

        // Get last updated timestamp
        let last_updated = match self.storage.get(CACHE_TIMESTAMP_KEY)? {
            Some(timestamp_str) => timestamp_str.parse::<u64>().unwrap_or(0),
            None => 0,
        };

        Ok(ManrsStatus::Known {
            asn,
            is_member,
            total_members,
            last_updated,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ManrsStatus {
    Known {
        asn: u64,
        is_member: bool,
        total_members: usize,
        last_updated: u64,
    },
    Unknown,
}

impl ManrsStatus {
    pub fn format_response(&self) -> String {
        match self {
            ManrsStatus::Known { asn, is_member, total_members, last_updated } => {
                let status = if *is_member { "MEMBER" } else { "NON-MEMBER" };
                let updated_time = format_timestamp(*last_updated);

                format!(
                    "% MANRS (Mutually Agreed Norms for Routing Security) Information\n\
                     %\n\
                     aut-num:            AS{}\n\
                     status:             {}\n\
                     asn:                AS{}\n\
                     total-members:      {}\n\
                     updated-time:       {}\n\
                     %\n\
                     % MANRS is a global initiative that provides crucial fixes to reduce\n\
                     % the most common routing threats. The four actions of MANRS are:\n\
                     %   1. Filtering - Implement routing filters to prevent incorrect routing information\n\
                     %   2. Anti-spoofing - Enable anti-spoofing protection to prevent address spoofing\n\
                     %   3. Coordination - Facilitate coordination between network operators\n\
                     %   4. Global Validation - Facilitate global routing information validation\n\
                     %\n\
                     % For more information about MANRS, visit: https://www.manrs.org/\n\
                     %\n\
                     % Cache refresh interval: 14 days\n\
                     % This query was served from: LOCAL CACHE\n\
                     %\n\
                     % Terms and Conditions of Use\n\
                     %\n\
                     % The data in this response is provided for informational purposes.\n\
                     % MANRS membership status is updated periodically from the official\n\
                     % MANRS API at https://api.manrs.org/\n\
                     %\n",
                    asn,
                    status,
                    asn,
                    total_members,
                    updated_time
                )
            }
            ManrsStatus::Unknown => {
                "% MANRS Information: Unable to determine membership status\n\
                 %\n\
                 % This could be due to network connectivity issues or API unavailability.\n\
                 % Please try again later or check https://www.manrs.org/ directly.\n\
                 %\n".to_string()
            }
        }
    }
}

pub fn parse_asn_from_query(query: &str) -> Option<u64> {
    // The query should end with -MANRS
    if !query.to_uppercase().ends_with("-MANRS") {
        return None;
    }

    // Remove -MANRS suffix
    let base_query = &query[..query.len() - 6];

    // Handle ASN formats: AS123, as123, 123
    let asn_str = if base_query.to_uppercase().starts_with("AS") {
        &base_query[2..]
    } else {
        base_query
    };

    asn_str.parse::<u64>().ok()
}

fn format_timestamp(timestamp: u64) -> String {
    match DateTime::from_timestamp(timestamp as i64, 0) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => "Unknown".to_string(),
    }
}

// Global MANRS checker instance
use std::sync::OnceLock;
static MANRS_CHECKER_INSTANCE: OnceLock<ManrsChecker> = OnceLock::new();

/// Get the global MANRS checker instance
async fn get_manrs_checker() -> Result<&'static ManrsChecker> {
    if let Some(checker) = MANRS_CHECKER_INSTANCE.get() {
        Ok(checker)
    } else {
        let storage = create_shared_storage(MANRS_LMDB_PATH).map_err(|e|
            anyhow::anyhow!("Failed to create MANRS LMDB storage: {}", e)
        )?;
        let checker = ManrsChecker::new(storage);
        match MANRS_CHECKER_INSTANCE.set(checker) {
            Ok(_) => Ok(MANRS_CHECKER_INSTANCE.get().unwrap()),
            Err(_) => Ok(MANRS_CHECKER_INSTANCE.get().unwrap()), // Another thread set it
        }
    }
}

/// Process a MANRS query and return formatted response
pub async fn process_manrs_query(query: &str) -> Result<String> {
    // Parse the ASN from the query
    let asn = match parse_asn_from_query(query) {
        Some(asn) => asn,
        None => {
            return Ok(
                format!("% MANRS Query Error: Invalid ASN format in query '{}'\n\
                 % Expected format: AS<number>-MANRS or <number>-MANRS\n\
                 % Example: AS64496-MANRS or 64496-MANRS\n", query)
            );
        }
    };

    // Get the MANRS checker and perform the lookup
    let checker = get_manrs_checker().await?;
    let status = checker.check_asn(asn).await?;

    Ok(status.format_response())
}

/// Blocking version for compatibility with blocking server
pub fn process_manrs_query_blocking(query: &str) -> Result<String> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(process_manrs_query(query))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_asn_from_query() {
        assert_eq!(parse_asn_from_query("AS123-MANRS"), Some(123));
        assert_eq!(parse_asn_from_query("as456-MANRS"), Some(456));
        assert_eq!(parse_asn_from_query("789-MANRS"), Some(789));
        assert_eq!(parse_asn_from_query("AS123"), None); // Missing -MANRS suffix
        assert_eq!(parse_asn_from_query("123"), None); // Missing -MANRS suffix
        assert_eq!(parse_asn_from_query("invalid-MANRS"), None);
        assert_eq!(parse_asn_from_query("AS-MANRS"), None); // No ASN number
    }
}
