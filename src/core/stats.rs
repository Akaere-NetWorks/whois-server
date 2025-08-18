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

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{ Utc, Duration as ChronoDuration };
use serde::{ Deserialize, Serialize };
use std::path::Path;
use tokio::fs;
use tracing::{ info, error };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub requests: u64,
    pub bytes_served: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalStats {
    pub total_requests: u64,
    pub total_bytes_served: u64,
    pub daily_stats: HashMap<String, DailyStats>, // Date in YYYY-MM-DD format
    pub hourly_stats: HashMap<String, DailyStats>, // DateTime in YYYY-MM-DD HH format
}

impl Default for TotalStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_bytes_served: 0,
            daily_stats: HashMap::new(),
            hourly_stats: HashMap::new(),
        }
    }
}

pub type StatsState = Arc<RwLock<TotalStats>>;

const STATS_FILE: &str = "stats.json";

pub async fn create_stats_state() -> StatsState {
    let stats = load_stats_from_file().await.unwrap_or_default();
    Arc::new(RwLock::new(stats))
}

async fn load_stats_from_file() -> Result<TotalStats, Box<dyn std::error::Error>> {
    if Path::new(STATS_FILE).exists() {
        let data = fs::read_to_string(STATS_FILE).await?;
        let stats: TotalStats = serde_json::from_str(&data)?;
        info!("Loaded statistics from {}", STATS_FILE);
        Ok(stats)
    } else {
        info!("No existing stats file found, starting with empty statistics");
        Ok(TotalStats::default())
    }
}

async fn save_stats_to_file(stats: &TotalStats) -> Result<(), Box<dyn std::error::Error>> {
    let data = serde_json::to_string_pretty(stats)?;
    fs::write(STATS_FILE, data).await?;
    Ok(())
}

async fn cleanup_old_stats(stats: &mut TotalStats) {
    let now = Utc::now();
    let one_month_ago = (now - ChronoDuration::days(31)).format("%Y-%m-%d").to_string();
    let one_day_ago = (now - ChronoDuration::hours(25)).format("%Y-%m-%d %H").to_string();

    // Clean up old daily stats (older than 31 days)
    let mut daily_to_remove = Vec::new();
    for (date, _daily_stats) in &stats.daily_stats {
        if date < &one_month_ago {
            daily_to_remove.push(date.clone());
        }
    }

    if !daily_to_remove.is_empty() {
        info!("Cleaning up {} old daily stats entries", daily_to_remove.len());
        for date in daily_to_remove {
            stats.daily_stats.remove(&date);
        }
    }

    // Clean up old hourly stats (older than 25 hours)
    let mut hourly_to_remove = Vec::new();
    for (datetime, _hourly_stats) in &stats.hourly_stats {
        if datetime < &one_day_ago {
            hourly_to_remove.push(datetime.clone());
        }
    }

    if !hourly_to_remove.is_empty() {
        info!("Cleaning up {} old hourly stats entries", hourly_to_remove.len());
        for datetime in hourly_to_remove {
            stats.hourly_stats.remove(&datetime);
        }
    }
}

pub async fn record_request(stats: &StatsState, response_size: usize) {
    let mut stats_guard = stats.write().await;
    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_hour = now.format("%Y-%m-%d %H").to_string();

    // Update total stats
    stats_guard.total_requests += 1;
    stats_guard.total_bytes_served += response_size as u64;

    // Update daily stats
    let daily_stats = stats_guard.daily_stats.entry(today).or_insert(DailyStats {
        requests: 0,
        bytes_served: 0,
    });

    daily_stats.requests += 1;
    daily_stats.bytes_served += response_size as u64;

    // Update hourly stats
    let hourly_stats = stats_guard.hourly_stats.entry(current_hour).or_insert(DailyStats {
        requests: 0,
        bytes_served: 0,
    });

    hourly_stats.requests += 1;
    hourly_stats.bytes_served += response_size as u64;

    // Cleanup old stats periodically (every 100 requests)
    if stats_guard.total_requests % 100 == 0 {
        cleanup_old_stats(&mut stats_guard).await;
    }

    // Save to file periodically (every 10 requests)
    if stats_guard.total_requests % 10 == 0 {
        let stats_copy = stats_guard.clone();
        drop(stats_guard); // Release lock before async operation

        if let Err(e) = save_stats_to_file(&stats_copy).await {
            error!("Failed to save statistics: {}", e);
        }
    }
}

pub async fn get_stats(stats: &StatsState) -> TotalStats {
    stats.read().await.clone()
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

pub async fn get_stats_response(stats: &StatsState) -> StatsResponse {
    let stats_data = get_stats(stats).await;
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
        let date = (now - ChronoDuration::days(29 - i)).format("%Y-%m-%d").to_string();
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

pub async fn save_stats_on_shutdown(stats: &StatsState) {
    let stats_data = get_stats(stats).await;
    if let Err(e) = save_stats_to_file(&stats_data).await {
        error!("Failed to save statistics on shutdown: {}", e);
    } else {
        info!("Statistics saved successfully on shutdown");
    }
}
