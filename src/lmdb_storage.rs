use std::path::Path;
use std::fs;
use anyhow::Result;
use lmdb::{Database, Environment, Transaction, WriteFlags, Cursor};
use tracing::{debug, info, warn};

/// LMDB storage manager for DN42 registry data
pub struct LmdbStorage {
    env: Environment,
    db: Database,
}

impl LmdbStorage {
    /// Create a new LMDB storage instance
    pub fn new(db_path: &str) -> Result<Self> {
        // Create the LMDB directory itself (LMDB expects a directory, not a file)
        let db_dir = Path::new(db_path);
        if !db_dir.exists() {
            fs::create_dir_all(db_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create LMDB directory {}: {}", db_path, e))?;
            info!("Created LMDB directory: {}", db_path);
        }

        // Open LMDB environment
        let env = Environment::new()
            .set_map_size(1024 * 1024 * 1024) // 1GB max size
            .set_max_dbs(1)
            .open(db_dir)
            .map_err(|e| anyhow::anyhow!("Failed to open LMDB environment at {}: {}", db_path, e))?;

        // Open database
        let db = env.open_db(None)?;

        info!("LMDB storage initialized at: {}", db_path);
        
        Ok(LmdbStorage { env, db })
    }

    /// Store a key-value pair in the database
    pub fn put(&self, key: &str, value: &str) -> Result<()> {
        let mut txn = self.env.begin_rw_txn()?;
        txn.put(self.db, &key, &value, WriteFlags::empty())?;
        txn.commit()?;
        Ok(())
    }

    /// Retrieve a value by key from the database
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.db, &key) {
            Ok(bytes) => {
                let value = std::str::from_utf8(bytes)?.to_string();
                Ok(Some(value))
            },
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if a key exists in the database
    pub fn exists(&self, key: &str) -> Result<bool> {
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.db, &key) {
            Ok(_) => Ok(true),
            Err(lmdb::Error::NotFound) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a key from the database
    pub fn delete(&self, key: &str) -> Result<()> {
        let mut txn = self.env.begin_rw_txn()?;
        match txn.del(self.db, &key, None) {
            Ok(_) => {
                txn.commit()?;
                Ok(())
            },
            Err(lmdb::Error::NotFound) => Ok(()), // Key already doesn't exist
            Err(e) => Err(e.into()),
        }
    }

    /// Clear all data from the database
    pub fn clear(&self) -> Result<()> {
        let mut txn = self.env.begin_rw_txn()?;
        txn.clear_db(self.db)?;
        txn.commit()?;
        info!("LMDB database cleared");
        Ok(())
    }

    /// Get database statistics (simplified)
    pub fn stats(&self) -> Result<()> {
        let _txn = self.env.begin_ro_txn()?;
        info!("LMDB database connection verified");
        Ok(())
    }

    /// Populate database with DN42 registry data from a directory
    pub fn populate_from_registry(&self, registry_path: &str) -> Result<()> {
        info!("Starting to populate LMDB from registry: {}", registry_path);
        
        let data_path = Path::new(registry_path).join("data");
        if !data_path.exists() {
            return Err(anyhow::anyhow!("Registry data directory not found: {:?}", data_path));
        }

        let mut total_files = 0;
        let mut processed_files = 0;

        // Process each subdirectory in the data folder
        for entry in fs::read_dir(&data_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let subdir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid directory name"))?;
                
                debug!("Processing subdirectory: {}", subdir_name);
                
                // Process files in this subdirectory
                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();
                    
                    if file_path.is_file() {
                        total_files += 1;
                        
                        let filename = file_path.file_name()
                            .and_then(|n| n.to_str())
                            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
                        
                        // Create LMDB key in format "subdir/filename"
                        let key = format!("{}/{}", subdir_name, filename);
                        
                        // Read file content
                        match fs::read_to_string(&file_path) {
                            Ok(content) => {
                                // Store in LMDB
                                if let Err(e) = self.put(&key, &content) {
                                    warn!("Failed to store {}: {}", key, e);
                                } else {
                                    processed_files += 1;
                                    if processed_files % 1000 == 0 {
                                        debug!("Processed {} files...", processed_files);
                                    }
                                }
                            },
                            Err(e) => {
                                warn!("Failed to read file {:?}: {}", file_path, e);
                            }
                        }
                    }
                }
            }
        }

        info!("LMDB population completed: {}/{} files processed", processed_files, total_files);
        Ok(())
    }

    /// Batch update - more efficient for bulk operations
    pub fn batch_update<F>(&self, mut operation: F) -> Result<()>
    where
        F: FnMut(&mut lmdb::RwTransaction) -> Result<()>,
    {
        let mut txn = self.env.begin_rw_txn()?;
        operation(&mut txn)?;
        txn.commit()?;
        Ok(())
    }

    /// Get all keys with a specific prefix
    pub fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.db)?;
        let mut keys = Vec::new();

        for (key, _) in cursor.iter() {
            let key_str = std::str::from_utf8(key)?;
            if key_str.starts_with(prefix) {
                keys.push(key_str.to_string());
            }
        }

        Ok(keys)
    }
}

// Note: Environment doesn't implement Clone, so we'll use Arc for sharing
// impl Clone for LmdbStorage {
//     fn clone(&self) -> Self {
//         LmdbStorage {
//             env: self.env.clone(),
//             db: self.db,
//         }
//     }
// }

/// Thread-safe wrapper for LmdbStorage
use std::sync::Arc;

pub type SharedLmdbStorage = Arc<LmdbStorage>;

pub fn create_shared_storage(db_path: &str) -> Result<SharedLmdbStorage> {
    let storage = LmdbStorage::new(db_path)?;
    Ok(Arc::new(storage))
} 