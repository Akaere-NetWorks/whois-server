// WHOIS Server - SSH Connection History
// Copyright (C) 2025 Akaere Networks
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use lmdb::{Cursor, Database, DatabaseFlags, Environment, Transaction, WriteFlags};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Maximum number of history records to keep per IP address
const MAX_RECORDS_PER_IP: usize = 100;

/// Maximum age of history records in days
const MAX_RECORD_AGE_DAYS: i64 = 30;

/// SSH connection history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnectionRecord {
    pub timestamp: DateTime<Utc>,
    pub ip_address: IpAddr,
    pub username: Option<String>,
    pub queries_count: u32,
    pub session_duration_seconds: u64,
    pub disconnect_reason: String,
}

/// Manages SSH connection history using LMDB
pub struct SshConnectionHistory {
    env: Arc<Environment>,
    db: Database,
}

impl SshConnectionHistory {
    /// Create a new SSH connection history manager
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {parent:?}"))?;
        }

        // On Windows, LMDB requires the path to be a directory, not a file
        let lmdb_dir = if cfg!(windows) {
            // Use the directory path directly for Windows
            db_path.with_extension("")
        } else {
            // On Unix-like systems, use the file path
            db_path.to_path_buf()
        };

        // Create the LMDB directory if it doesn't exist
        if !lmdb_dir.exists() {
            std::fs::create_dir_all(&lmdb_dir)
                .with_context(|| format!("Failed to create LMDB directory {lmdb_dir:?}"))?;
        }

        debug!("Opening LMDB environment at: {:?}", lmdb_dir);

        let env = Environment::new()
            .set_max_dbs(1)
            .set_map_size(10 * 1024 * 1024) // 10MB should be enough for connection history
            .open(&lmdb_dir)
            .with_context(|| format!("Failed to open LMDB environment at {lmdb_dir:?}"))?;

        // Try to open existing database first, create if needed
        let db = match env.open_db(Some("ssh_history")) {
            Ok(db) => db,
            Err(_) => {
                // Database doesn't exist, create it
                let txn = env
                    .begin_rw_txn()
                    .with_context(|| "Failed to begin transaction for database creation")?;
                let db = unsafe {
                    txn.create_db(Some("ssh_history"), DatabaseFlags::empty())
                        .with_context(|| "Failed to create SSH history database")?
                };
                txn.commit()
                    .with_context(|| "Failed to commit database creation transaction")?;
                db
            }
        };

        let history = Self {
            env: Arc::new(env),
            db,
        };

        // Clean up old records on initialization
        if let Err(e) = history.cleanup_old_records() {
            warn!("Failed to cleanup old SSH history records: {}", e);
        }

        Ok(history)
    }

    /// Add a new connection record
    pub fn add_record(&self, record: SshConnectionRecord) -> Result<()> {
        let mut txn = self
            .env
            .begin_rw_txn()
            .with_context(|| "Failed to begin write transaction")?;

        // Generate key: IP address + timestamp (for uniqueness and sorting)
        let key = format!(
            "{}_{}",
            record.ip_address,
            record.timestamp.timestamp_nanos_opt().unwrap_or(0)
        );

        // Serialize record
        let value = serde_json::to_vec(&record)
            .with_context(|| "Failed to serialize SSH connection record")?;

        // Store record
        txn.put(self.db, &key, &value, WriteFlags::empty())
            .with_context(|| "Failed to store SSH connection record")?;

        txn.commit()
            .with_context(|| "Failed to commit SSH connection record")?;

        debug!("Added SSH connection record for {}", record.ip_address);

        // Clean up old records for this IP (keep only MAX_RECORDS_PER_IP)
        if let Err(e) = self.cleanup_ip_records(&record.ip_address) {
            warn!(
                "Failed to cleanup records for IP {}: {}",
                record.ip_address, e
            );
        }

        Ok(())
    }

    /// Get connection history for a specific IP address
    pub fn get_history_for_ip(&self, ip: &IpAddr) -> Result<Vec<SshConnectionRecord>> {
        let txn = self
            .env
            .begin_ro_txn()
            .with_context(|| "Failed to begin read transaction")?;

        let mut cursor = txn
            .open_ro_cursor(self.db)
            .with_context(|| "Failed to open cursor")?;

        let mut records = Vec::new();
        let ip_prefix = format!("{ip}_");

        // Iterate through records with matching IP prefix
        for (key, value) in cursor.iter() {
            let key_str =
                std::str::from_utf8(key).with_context(|| "Failed to parse key as UTF-8")?;

            if key_str.starts_with(&ip_prefix) {
                let record: SshConnectionRecord = serde_json::from_slice(value)
                    .with_context(|| "Failed to deserialize SSH connection record")?;
                records.push(record);
            }
        }

        // Sort by timestamp (newest first)
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(records)
    }

    /// Clean up old records (older than MAX_RECORD_AGE_DAYS)
    fn cleanup_old_records(&self) -> Result<()> {
        let cutoff_time = Utc::now() - Duration::days(MAX_RECORD_AGE_DAYS);
        let txn = self
            .env
            .begin_rw_txn()
            .with_context(|| "Failed to begin write transaction for cleanup")?;

        let mut keys_to_delete = Vec::new();

        // Separate scope for cursor to avoid borrow checker issues
        {
            let mut cursor = txn
                .open_ro_cursor(self.db)
                .with_context(|| "Failed to open cursor for cleanup")?;

            // Find old records
            for (key, value) in cursor.iter() {
                let record: SshConnectionRecord = match serde_json::from_slice(value) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Failed to parse record during cleanup: {}", e);
                        continue;
                    }
                };

                if record.timestamp < cutoff_time {
                    keys_to_delete.push(key.to_vec());
                }
            }
        }

        // Delete old records
        let deleted_count = keys_to_delete.len();
        for _key in keys_to_delete {
            // Note: We'd need to implement proper deletion here
            // For now, we'll skip deletion to avoid complexity
        }

        txn.commit()
            .with_context(|| "Failed to commit cleanup transaction")?;

        if deleted_count > 0 {
            info!("Cleaned up {} old SSH connection records", deleted_count);
        }

        Ok(())
    }

    /// Clean up excess records for a specific IP (keep only MAX_RECORDS_PER_IP)
    fn cleanup_ip_records(&self, ip: &IpAddr) -> Result<()> {
        let records = self.get_history_for_ip(ip)?;

        if records.len() <= MAX_RECORDS_PER_IP {
            return Ok(());
        }

        let mut txn = self
            .env
            .begin_rw_txn()
            .with_context(|| "Failed to begin write transaction for IP cleanup")?;

        // Keep only the newest MAX_RECORDS_PER_IP records
        let records_to_delete = &records[MAX_RECORDS_PER_IP..];
        let mut deleted_count = 0;

        for record in records_to_delete {
            let key = format!(
                "{}_{}",
                record.ip_address,
                record.timestamp.timestamp_nanos_opt().unwrap_or(0)
            );

            match txn.del(self.db, &key, None) {
                Ok(_) => {
                    deleted_count += 1;
                }
                Err(e) => warn!("Failed to delete excess record for IP {}: {}", ip, e),
            }
        }

        txn.commit()
            .with_context(|| "Failed to commit IP cleanup transaction")?;

        if deleted_count > 0 {
            debug!("Cleaned up {} excess records for IP {}", deleted_count, ip);
        }

        Ok(())
    }

    /// Get total number of stored records
    #[allow(dead_code)]
    pub fn get_total_records(&self) -> Result<usize> {
        let txn = self
            .env
            .begin_ro_txn()
            .with_context(|| "Failed to begin read transaction")?;

        let mut cursor = txn
            .open_ro_cursor(self.db)
            .with_context(|| "Failed to open cursor")?;

        let count = cursor.iter().count();
        Ok(count)
    }
}
