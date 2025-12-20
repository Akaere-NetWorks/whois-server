/*
 * WHOIS Server with DN42 Support
 * Copyright (C) 2025 Akaere Networks
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use crate::config::STATS_LMDB_PATH;
use crate::storage::lmdb::LmdbStorage;

use crate::{log_error, log_info, log_warn};
// Legacy stats file path for migration
const LEGACY_STATS_FILE: &str = "stats.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub requests: u64,
    pub bytes_served: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TotalStats {
    pub total_requests: u64,
    pub total_bytes_served: u64,
    pub daily_stats: HashMap<String, DailyStats>, // Date in YYYY-MM-DD format
    pub hourly_stats: HashMap<String, DailyStats>, // DateTime in YYYY-MM-DD HH format
}

pub struct StatsManager {
    pub stats: Arc<RwLock<TotalStats>>,
    storage: Arc<LmdbStorage>,
}

pub type StatsState = Arc<StatsManager>;

// LMDB keys for different stats
const STATS_KEY_TOTAL: &str = "stats:total";
const STATS_KEY_DAILY_PREFIX: &str = "stats:daily:";
const STATS_KEY_HOURLY_PREFIX: &str = "stats:hourly:";

pub async fn create_stats_state() -> StatsState {
    use crate::{log_init_ok_with_details, log_init_failed};

    let storage = match LmdbStorage::new(STATS_LMDB_PATH) {
        Ok(s) => {
            log_init_ok_with_details!("Statistics Storage", &format!("LMDB at {}", STATS_LMDB_PATH));
            Arc::new(s)
        },
        Err(e) => {
            log_init_failed!("Statistics Storage", &format!("LMDB creation failed: {}", e));
            // Create a dummy storage that doesn't persist
            Arc::new(LmdbStorage::new("/tmp/stats_dummy").unwrap_or_else(|_| {
                // As a last resort, create in-memory storage
                panic!("Failed to create any storage for statistics")
            }))
        }
    };

    // Try to migrate from legacy stats.json if it exists and LMDB is empty
    if let Err(e) = migrate_from_legacy_json(&storage).await {
        log_warn!("Failed to migrate legacy stats.json: {}", e);
    }

    let stats = load_stats_from_lmdb(&storage).await.unwrap_or_default();

    Arc::new(StatsManager {
        stats: Arc::new(RwLock::new(stats)),
        storage,
    })
}

/// Migrate data from legacy stats.json file to LMDB
async fn migrate_from_legacy_json(
    storage: &Arc<LmdbStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let legacy_path = Path::new(LEGACY_STATS_FILE);

    // Check if legacy file exists
    if !legacy_path.exists() {
        return Ok(()); // No migration needed
    }

    // Check if LMDB already has data (avoid re-migration)
    if storage.exists(STATS_KEY_TOTAL)? {
        log_info!("LMDB already contains statistics data, skipping migration");
        return Ok(());
    }

    log_info!("Found legacy stats.json, migrating to LMDB...");

    // Read and parse legacy JSON file
    let json_data = fs::read_to_string(legacy_path).await?;
    let legacy_stats: TotalStats = serde_json::from_str(&json_data)?;

    log_info!(
        "Migrating {} total requests, {} daily entries, {} hourly entries",
        legacy_stats.total_requests,
        legacy_stats.daily_stats.len(),
        legacy_stats.hourly_stats.len()
    );

    // Save to LMDB
    save_stats_to_lmdb(storage, &legacy_stats).await?;

    log_info!("Successfully migrated statistics from stats.json to LMDB");

    // Rename the old file to .migrated for backup
    let backup_path = format!("{}.migrated", LEGACY_STATS_FILE);
    if let Err(e) = fs::rename(legacy_path, &backup_path).await {
        log_warn!(
            "Failed to rename legacy stats.json to {}: {}",
            backup_path, e
        );
        log_warn!("You may want to manually delete or rename stats.json");
    } else {
        log_info!("Renamed legacy stats.json to {}", backup_path);
    }

    Ok(())
}

async fn load_stats_from_lmdb(
    storage: &Arc<LmdbStorage>,
) -> Result<TotalStats, Box<dyn std::error::Error>> {
    // Load total stats
    let (total_requests, total_bytes_served) =
        match storage.get_json::<(u64, u64)>(STATS_KEY_TOTAL)? {
            Some((req, bytes)) => {
                log_info!(
                    "Loaded total statistics from LMDB: {} requests, {} bytes",
                    req, bytes
                );
                (req, bytes)
            }
            None => {
                log_info!("No existing stats in LMDB, starting with empty statistics");
                (0, 0)
            }
        };

    // Load daily stats
    let mut daily_stats = HashMap::new();
    let daily_keys = storage.get_keys_with_prefix(STATS_KEY_DAILY_PREFIX)?;
    for key in daily_keys {
        if let Some(date) = key.strip_prefix(STATS_KEY_DAILY_PREFIX) {
            if let Some(stats) = storage.get_json::<DailyStats>(&key)? {
                daily_stats.insert(date.to_string(), stats);
            }
        }
    }
    log_info!("Loaded {} daily stats entries from LMDB", daily_stats.len());

    // Load hourly stats
    let mut hourly_stats = HashMap::new();
    let hourly_keys = storage.get_keys_with_prefix(STATS_KEY_HOURLY_PREFIX)?;
    for key in hourly_keys {
        if let Some(datetime) = key.strip_prefix(STATS_KEY_HOURLY_PREFIX) {
            if let Some(stats) = storage.get_json::<DailyStats>(&key)? {
                hourly_stats.insert(datetime.to_string(), stats);
            }
        }
    }
    log_info!(
        "Loaded {} hourly stats entries from LMDB",
        hourly_stats.len()
    );

    Ok(TotalStats {
        total_requests,
        total_bytes_served,
        daily_stats,
        hourly_stats,
    })
}

async fn save_stats_to_lmdb(
    storage: &Arc<LmdbStorage>,
    stats: &TotalStats,
) -> Result<(), Box<dyn std::error::Error>> {
    // Save total stats
    storage.put_json(
        STATS_KEY_TOTAL,
        &(stats.total_requests, stats.total_bytes_served),
    )?;

    // Save daily stats (only updated entries)
    for (date, daily_stat) in &stats.daily_stats {
        let key = format!("{}{}", STATS_KEY_DAILY_PREFIX, date);
        storage.put_json(&key, daily_stat)?;
    }

    // Save hourly stats (only updated entries)
    for (datetime, hourly_stat) in &stats.hourly_stats {
        let key = format!("{}{}", STATS_KEY_HOURLY_PREFIX, datetime);
        storage.put_json(&key, hourly_stat)?;
    }

    Ok(())
}

async fn cleanup_old_stats(storage: &Arc<LmdbStorage>, stats: &mut TotalStats) {
    let now = Utc::now();
    let one_month_ago = (now - ChronoDuration::days(31))
        .format("%Y-%m-%d")
        .to_string();
    let one_day_ago = (now - ChronoDuration::hours(25))
        .format("%Y-%m-%d %H")
        .to_string();

    // Clean up old daily stats (older than 31 days)
    let mut daily_to_remove = Vec::new();
    for date in stats.daily_stats.keys() {
        if date < &one_month_ago {
            daily_to_remove.push(date.clone());
        }
    }

    if !daily_to_remove.is_empty() {
        log_info!(
            "Cleaning up {} old daily stats entries",
            daily_to_remove.len()
        );
        for date in daily_to_remove {
            stats.daily_stats.remove(&date);
            // Remove from LMDB as well
            let key = format!("{}{}", STATS_KEY_DAILY_PREFIX, date);
            if let Err(e) = storage.delete(&key) {
                log_error!("Failed to delete old daily stat {}: {}", key, e);
            }
        }
    }

    // Clean up old hourly stats (older than 25 hours)
    let mut hourly_to_remove = Vec::new();
    for datetime in stats.hourly_stats.keys() {
        if datetime < &one_day_ago {
            hourly_to_remove.push(datetime.clone());
        }
    }

    if !hourly_to_remove.is_empty() {
        log_info!(
            "Cleaning up {} old hourly stats entries",
            hourly_to_remove.len()
        );
        for datetime in hourly_to_remove {
            stats.hourly_stats.remove(&datetime);
            // Remove from LMDB as well
            let key = format!("{}{}", STATS_KEY_HOURLY_PREFIX, datetime);
            if let Err(e) = storage.delete(&key) {
                log_error!("Failed to delete old hourly stat {}: {}", key, e);
            }
        }
    }
}

pub async fn record_request(stats_manager: &StatsState, response_size: usize) {
    let mut stats_guard = stats_manager.stats.write().await;
    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_hour = now.format("%Y-%m-%d %H").to_string();

    // Update total stats
    stats_guard.total_requests += 1;
    stats_guard.total_bytes_served += response_size as u64;

    // Update daily stats
    let daily_stats = stats_guard
        .daily_stats
        .entry(today.clone())
        .or_insert(DailyStats {
            requests: 0,
            bytes_served: 0,
        });

    daily_stats.requests += 1;
    daily_stats.bytes_served += response_size as u64;

    // Update hourly stats
    let hourly_stats = stats_guard
        .hourly_stats
        .entry(current_hour.clone())
        .or_insert(DailyStats {
            requests: 0,
            bytes_served: 0,
        });

    hourly_stats.requests += 1;
    hourly_stats.bytes_served += response_size as u64;

    // Cleanup old stats periodically (every 100 requests)
    if stats_guard.total_requests % 100 == 0 {
        cleanup_old_stats(&stats_manager.storage, &mut stats_guard).await;
    }

    // Save to LMDB periodically (every 10 requests)
    if stats_guard.total_requests % 10 == 0 {
        let stats_copy = stats_guard.clone();
        let storage = stats_manager.storage.clone();
        drop(stats_guard); // Release lock before async operation

        tokio::spawn(async move {
            if let Err(e) = save_stats_to_lmdb(&storage, &stats_copy).await {
                log_error!("Failed to save statistics to LMDB: {}", e);
            }
        });
    }
}

pub async fn get_stats(stats_manager: &StatsState) -> TotalStats {
    stats_manager.stats.read().await.clone()
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_requests: u64,
    pub total_bytes_served: u64,
    pub total_kb_served: f64,
    pub daily_stats_24h: Vec<DailyStatsEntry>,
    pub daily_stats_30d: Vec<DailyStatsEntry>,
}

#[derive(Serialize)]
pub struct DailyStatsEntry {
    pub date: String,
    pub requests: u64,
    pub bytes_served: u64,
    pub kb_served: f64,
}

pub async fn get_stats_response(stats_manager: &StatsState) -> StatsResponse {
    let stats_data = get_stats(stats_manager).await;
    let now = Utc::now();

    // Generate last 24 hours (using hourly data)
    let mut daily_24h = Vec::new();
    for i in 0..24 {
        let hour_time = now - ChronoDuration::hours(23 - i);
        let hour_key = hour_time.format("%Y-%m-%d %H").to_string();
        let hour_display = hour_time.format("%H:00").to_string();
        let hourly_stat = stats_data.hourly_stats.get(&hour_key);
        daily_24h.push(DailyStatsEntry {
            date: hour_display,
            requests: hourly_stat.map(|s| s.requests).unwrap_or(0),
            bytes_served: hourly_stat.map(|s| s.bytes_served).unwrap_or(0),
            kb_served: (hourly_stat.map(|s| s.bytes_served).unwrap_or(0) as f64) / 1024.0,
        });
    }

    // Generate last 30 days (using daily data, ensure today is included)
    let mut daily_30d = Vec::new();
    for i in 0..30 {
        let date = (now - ChronoDuration::days(29 - i))
            .format("%Y-%m-%d")
            .to_string();
        let daily_stat = stats_data.daily_stats.get(&date);
        daily_30d.push(DailyStatsEntry {
            date: date.clone(),
            requests: daily_stat.map(|s| s.requests).unwrap_or(0),
            bytes_served: daily_stat.map(|s| s.bytes_served).unwrap_or(0),
            kb_served: (daily_stat.map(|s| s.bytes_served).unwrap_or(0) as f64) / 1024.0,
        });
    }

    // Ensure today's date is included in the 30-day data even if no data exists yet
    let today = now.format("%Y-%m-%d").to_string();
    if !daily_30d.iter().any(|entry| entry.date == today) {
        let today_stat = stats_data.daily_stats.get(&today);
        daily_30d.push(DailyStatsEntry {
            date: today,
            requests: today_stat.map(|s| s.requests).unwrap_or(0),
            bytes_served: today_stat.map(|s| s.bytes_served).unwrap_or(0),
            kb_served: (today_stat.map(|s| s.bytes_served).unwrap_or(0) as f64) / 1024.0,
        });
        // Sort to maintain chronological order
        daily_30d.sort_by(|a, b| a.date.cmp(&b.date));
    }

    StatsResponse {
        total_requests: stats_data.total_requests,
        total_bytes_served: stats_data.total_bytes_served,
        total_kb_served: (stats_data.total_bytes_served as f64) / 1024.0,
        daily_stats_24h: daily_24h,
        daily_stats_30d: daily_30d,
    }
}

pub async fn save_stats_on_shutdown(stats_manager: &StatsState) {
    let stats_data = get_stats(stats_manager).await;
    if let Err(e) = save_stats_to_lmdb(&stats_manager.storage, &stats_data).await {
        log_error!("Failed to save statistics to LMDB on shutdown: {}", e);
    } else {
        log_info!("Statistics saved successfully to LMDB on shutdown");
    }
}
