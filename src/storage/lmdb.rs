use std::path::Path;
use std::fs;
use anyhow::Result;
use lmdb::{ Database, Environment, Transaction, WriteFlags, Cursor };
use tracing::{ debug, info, warn };
use std::time::SystemTime;
use serde::{ Serialize, Deserialize };

/// File metadata for tracking changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct FileMetadata {
    size: u64,
    modified: u64, // Unix timestamp
}

impl FileMetadata {
    /// Create metadata from file path
    fn from_file(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
        Ok(FileMetadata {
            size: metadata.len(),
            modified,
        })
    }
}

/// LMDB storage manager for DN42 registry data
#[derive(Debug)]
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
            fs
                ::create_dir_all(db_dir)
                .map_err(|e|
                    anyhow::anyhow!("Failed to create LMDB directory {}: {}", db_path, e)
                )?;
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

    /// Store file metadata
    fn put_metadata(&self, key: &str, metadata: &FileMetadata) -> Result<()> {
        let metadata_key = format!("__meta__{}", key);
        let metadata_json = serde_json::to_string(metadata)?;
        self.put(&metadata_key, &metadata_json)
    }

    /// Get file metadata
    fn get_metadata(&self, key: &str) -> Result<Option<FileMetadata>> {
        let metadata_key = format!("__meta__{}", key);
        match self.get(&metadata_key)? {
            Some(metadata_json) => {
                let metadata: FileMetadata = serde_json::from_str(&metadata_json)?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    /// Retrieve a value by key from the database
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        debug!("LMDB: Attempting to read key: {}", key);
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.db, &key) {
            Ok(bytes) => {
                let value = std::str::from_utf8(bytes)?.to_string();
                debug!(
                    "LMDB: Successfully read key '{}', content length: {} bytes",
                    key,
                    value.len()
                );
                Ok(Some(value))
            }
            Err(lmdb::Error::NotFound) => {
                debug!("LMDB: Key not found: {}", key);
                Ok(None)
            }
            Err(e) => {
                warn!("LMDB: Error reading key '{}': {}", key, e);
                Err(e.into())
            }
        }
    }

    /// Check if a key exists in the database
    pub fn exists(&self, key: &str) -> Result<bool> {
        debug!("LMDB: Checking if key exists: {}", key);
        let txn = self.env.begin_ro_txn()?;
        match txn.get(self.db, &key) {
            Ok(_) => {
                debug!("LMDB: Key exists: {}", key);
                Ok(true)
            }
            Err(lmdb::Error::NotFound) => {
                debug!("LMDB: Key does not exist: {}", key);
                Ok(false)
            }
            Err(e) => {
                warn!("LMDB: Error checking key '{}': {}", key, e);
                Err(e.into())
            }
        }
    }

    /// Delete a key from the database
    pub fn delete(&self, key: &str) -> Result<()> {
        let mut txn = self.env.begin_rw_txn()?;
        match txn.del(self.db, &key, None) {
            Ok(_) => {
                txn.commit()?;
                Ok(())
            }
            Err(lmdb::Error::NotFound) => Ok(()), // Key already doesn't exist
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a key and its metadata
    fn delete_with_metadata(&self, key: &str) -> Result<()> {
        let metadata_key = format!("__meta__{}", key);
        self.delete(key)?;
        self.delete(&metadata_key)?;
        Ok(())
    }

    /// Clear all data from the database
    #[allow(dead_code)]
    pub fn clear(&self) -> Result<()> {
        let mut txn = self.env.begin_rw_txn()?;
        txn.clear_db(self.db)?;
        txn.commit()?;
        info!("LMDB database cleared");
        Ok(())
    }

    /// Get database statistics (simplified)
    #[allow(dead_code)]
    pub fn stats(&self) -> Result<()> {
        let _txn = self.env.begin_ro_txn()?;
        info!("LMDB database connection verified");
        Ok(())
    }

    /// Populate database with DN42 registry data from a directory (with incremental update)
    pub fn populate_from_registry(&self, registry_path: &str) -> Result<()> {
        info!("Starting incremental LMDB update from registry: {}", registry_path);

        let data_path = Path::new(registry_path).join("data");
        if !data_path.exists() {
            return Err(anyhow::anyhow!("Registry data directory not found: {:?}", data_path));
        }

        let mut total_files = 0;
        let mut updated_files = 0;
        let mut skipped_files = 0;
        let mut current_keys = std::collections::HashSet::new();

        // Process each subdirectory in the data folder
        for entry in fs::read_dir(&data_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let subdir_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow::anyhow!("Invalid directory name"))?;

                debug!("Processing subdirectory: {}", subdir_name);

                // Process files in this subdirectory
                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.is_file() {
                        total_files += 1;

                        let filename = file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

                        // Create LMDB key in format "subdir/filename"
                        let key = format!("{}/{}", subdir_name, filename);
                        current_keys.insert(key.clone());

                        // Get current file metadata
                        let current_metadata = match FileMetadata::from_file(&file_path) {
                            Ok(metadata) => metadata,
                            Err(e) => {
                                warn!("Failed to get metadata for {:?}: {}", file_path, e);
                                continue;
                            }
                        };

                        // Check if file needs update
                        let needs_update = match self.get_metadata(&key) {
                            Ok(Some(stored_metadata)) => {
                                if stored_metadata != current_metadata {
                                    debug!(
                                        "File changed: {} (size: {} -> {}, modified: {} -> {})",
                                        key,
                                        stored_metadata.size,
                                        current_metadata.size,
                                        stored_metadata.modified,
                                        current_metadata.modified
                                    );
                                    true
                                } else {
                                    debug!("File unchanged, skipping: {}", key);
                                    false
                                }
                            }
                            Ok(None) => {
                                debug!("New file detected: {}", key);
                                true
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to get stored metadata for {}: {}, treating as new file",
                                    key,
                                    e
                                );
                                true
                            }
                        };

                        if needs_update {
                            // Read file content and update
                            match fs::read_to_string(&file_path) {
                                Ok(content) => {
                                    // Store content and metadata
                                    if let Err(e) = self.put(&key, &content) {
                                        warn!("Failed to store content for {}: {}", key, e);
                                    } else if
                                        let Err(e) = self.put_metadata(&key, &current_metadata)
                                    {
                                        warn!("Failed to store metadata for {}: {}", key, e);
                                    } else {
                                        updated_files += 1;
                                        if updated_files % 1000 == 0 {
                                            debug!("Updated {} files...", updated_files);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to read file {:?}: {}", file_path, e);
                                }
                            }
                        } else {
                            skipped_files += 1;
                        }
                    }
                }
            }
        }

        // Clean up deleted files from LMDB
        let deleted_count = self.cleanup_deleted_files(&current_keys)?;

        info!(
            "LMDB incremental update completed: {}/{} files processed, {} updated, {} skipped, {} deleted",
            total_files,
            total_files,
            updated_files,
            skipped_files,
            deleted_count
        );
        Ok(())
    }

    /// Remove files from LMDB that no longer exist in the filesystem
    fn cleanup_deleted_files(
        &self,
        current_keys: &std::collections::HashSet<String>
    ) -> Result<usize> {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.db)?;
        let mut keys_to_delete = Vec::new();

        // Find all keys that don't start with __meta__ and are not in current_keys
        for (key_bytes, _) in cursor.iter() {
            let key_str = std::str::from_utf8(key_bytes)?;

            // Skip metadata keys
            if key_str.starts_with("__meta__") {
                continue;
            }

            // If this key is not in current filesystem, mark for deletion
            if !current_keys.contains(key_str) {
                keys_to_delete.push(key_str.to_string());
            }
        }

        drop(cursor);
        drop(txn);

        // Delete the keys
        let deleted_count = keys_to_delete.len();
        for key in keys_to_delete {
            debug!("Deleting removed file from LMDB: {}", key);
            if let Err(e) = self.delete_with_metadata(&key) {
                warn!("Failed to delete key {}: {}", key, e);
            }
        }

        if deleted_count > 0 {
            info!("Cleaned up {} deleted files from LMDB", deleted_count);
        }

        Ok(deleted_count)
    }

    /// Iterate over keys that start with a specific prefix
    pub fn iterate_keys<F>(&self, prefix: &str, mut callback: F) -> Result<()>
        where
            F: FnMut(&str) -> bool // Return false to stop iteration
    {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.db)?;

        for (key_bytes, _) in cursor.iter() {
            let key_str = std::str::from_utf8(key_bytes)?;

            // Skip metadata keys
            if key_str.starts_with("__meta__") {
                continue;
            }

            // Check if key starts with prefix
            if key_str.starts_with(prefix) {
                if !callback(key_str) {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Batch update - more efficient for bulk operations
    #[allow(dead_code)]
    pub fn batch_update<F>(&self, mut operation: F) -> Result<()>
        where F: FnMut(&mut lmdb::RwTransaction) -> Result<()>
    {
        let mut txn = self.env.begin_rw_txn()?;
        operation(&mut txn)?;
        txn.commit()?;
        Ok(())
    }

    /// Get all keys with a specific prefix
    #[allow(dead_code)]
    pub fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.db)?;
        let mut keys = Vec::new();

        for (key, _) in cursor.iter() {
            let key_str = std::str::from_utf8(key)?;
            if key_str.starts_with(prefix) && !key_str.starts_with("__meta__") {
                keys.push(key_str.to_string());
            }
        }

        Ok(keys)
    }

    /// List all keys (excluding metadata keys)
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let txn = self.env.begin_ro_txn()?;
        let mut cursor = txn.open_ro_cursor(self.db)?;
        let mut keys = Vec::new();

        for (key, _) in cursor.iter() {
            let key_str = std::str::from_utf8(key)?;
            if !key_str.starts_with("__meta__") {
                keys.push(key_str.to_string());
            }
        }

        Ok(keys)
    }

    /// Generic put method for serializable types
    pub fn put_json<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json_str = serde_json::to_string(value)?;
        self.put(key, &json_str)
    }

    /// Generic get method for deserializable types
    pub fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get(key)? {
            Some(json_str) => {
                let value: T = serde_json::from_str(&json_str)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Force full refresh (clear and repopulate)
    #[allow(dead_code)]
    pub fn force_full_refresh(&self, registry_path: &str) -> Result<()> {
        info!("Performing full LMDB refresh");
        self.clear()?;
        self.populate_from_registry(registry_path)
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
