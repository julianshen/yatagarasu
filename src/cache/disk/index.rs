//! Cache index management

use super::backend::DiskBackend;
use super::error::DiskCacheError;
use super::types::EntryMetadata;
use crate::cache::CacheKey;
use bytes::Bytes;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Thread-safe in-memory index of cached entries
pub struct CacheIndex {
    entries: Arc<RwLock<HashMap<CacheKey, EntryMetadata>>>,
    total_size: Arc<AtomicU64>,
}

impl CacheIndex {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            total_size: Arc::new(AtomicU64::new(0)),
        }
    }

    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache Trait Implementation)
    pub fn get(&self, key: &CacheKey) -> Option<EntryMetadata> {
        self.entries.read().get(key).cloned()
    }

    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache Trait Implementation)
    pub fn insert(&self, key: CacheKey, metadata: EntryMetadata) {
        let size = metadata.size_bytes;
        self.entries.write().insert(key, metadata);
        self.total_size.fetch_add(size, Ordering::SeqCst);
    }

    #[allow(dead_code)] // Will be used in Phase 28.7 (LRU Eviction)
    pub fn remove(&self, key: &CacheKey) -> Option<EntryMetadata> {
        let removed = self.entries.write().remove(key);
        if let Some(ref metadata) = removed {
            self.total_size
                .fetch_sub(metadata.size_bytes, Ordering::SeqCst);
        }
        removed
    }

    #[allow(dead_code)] // Will be used in Phase 28.7 (LRU Eviction)
    pub fn total_size(&self) -> u64 {
        self.total_size.load(Ordering::SeqCst)
    }

    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache Trait Implementation)
    pub fn entry_count(&self) -> usize {
        self.entries.read().len()
    }

    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache Trait Implementation)
    pub fn clear(&self) {
        self.entries.write().clear();
        self.total_size.store(0, Ordering::SeqCst);
    }

    /// Find the least recently accessed entry (for LRU eviction)
    #[allow(dead_code)] // Will be used in Phase 28.7 (LRU Eviction)
    pub fn find_lru_entry(&self) -> Option<(CacheKey, EntryMetadata)> {
        let entries = self.entries.read();
        entries
            .iter()
            .min_by_key(|(_, meta)| meta.last_accessed_at)
            .map(|(k, v)| (k.clone(), v.clone()))
    }

    /// Find all keys belonging to a specific bucket
    pub fn keys_for_bucket(&self, bucket: &str) -> Vec<CacheKey> {
        self.entries
            .read()
            .keys()
            .filter(|k| k.bucket == bucket)
            .cloned()
            .collect()
    }

    /// Calculate stats for a specific bucket
    pub fn stats_for_bucket(&self, bucket: &str) -> (u64, u64) {
        let entries = self.entries.read();
        let mut size_bytes: u64 = 0;
        let mut item_count: u64 = 0;

        for (key, metadata) in entries.iter() {
            if key.bucket == bucket {
                size_bytes += metadata.size_bytes;
                item_count += 1;
            }
        }

        (size_bytes, item_count)
    }

    /// Save index to JSON file
    #[allow(dead_code)] // Will be called in Phase 28.8 (Recovery & Startup)
    pub async fn save_to_file<B: DiskBackend>(
        &self,
        path: &Path,
        backend: &B,
    ) -> Result<(), DiskCacheError> {
        // Create a serializable snapshot of the index
        let entries: Vec<_> = self
            .entries
            .read()
            .iter()
            .map(|(k, v)| IndexEntry {
                key: k.clone(),
                metadata: v.clone(),
            })
            .collect();

        let snapshot = IndexSnapshot {
            entries,
            version: 1,
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&snapshot)?;

        // Write to file atomically
        backend.write_file_atomic(path, Bytes::from(json)).await?;

        Ok(())
    }

    /// Load index from JSON file
    #[allow(dead_code)] // Will be called in Phase 28.8 (Recovery & Startup)
    pub async fn load_from_file<B: DiskBackend>(
        path: &Path,
        backend: &B,
    ) -> Result<Self, DiskCacheError> {
        // Try to read the file
        let data = match backend.read_file(path).await {
            Ok(d) => d,
            Err(DiskCacheError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist - return empty index
                return Ok(Self::new());
            }
            Err(e) => return Err(e),
        };

        // Parse JSON
        let json_str = String::from_utf8(data.to_vec()).map_err(|e| {
            DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8: {}", e),
            ))
        })?;

        let snapshot: IndexSnapshot = match serde_json::from_str(&json_str) {
            Ok(s) => s,
            Err(e) => {
                // Log error and return empty index
                eprintln!(
                    "Failed to parse index.json: {}. Starting with empty index.",
                    e
                );
                return Ok(Self::new());
            }
        };

        // Convert Vec back to HashMap and calculate total size
        let mut entries_map = HashMap::new();
        let mut total_size: u64 = 0;

        for entry in snapshot.entries {
            total_size += entry.metadata.size_bytes;
            entries_map.insert(entry.key, entry.metadata);
        }

        Ok(Self {
            entries: Arc::new(RwLock::new(entries_map)),
            total_size: Arc::new(AtomicU64::new(total_size)),
        })
    }

    /// Validate and repair the index by scanning the filesystem
    #[allow(dead_code)] // Will be called in Phase 28.8 (Recovery & Startup)
    pub async fn validate_and_repair<B: DiskBackend>(
        &self,
        entries_dir: &Path,
        backend: &B,
    ) -> Result<(), DiskCacheError> {
        use super::utils::{cache_key_to_file_path, key_to_hash};
        use std::collections::HashSet;
        use std::time::SystemTime;

        // Scan directory for all files
        let files = match backend.read_dir(entries_dir).await {
            Ok(f) => f,
            Err(_) => return Ok(()), // Directory doesn't exist yet
        };

        // Build set of all hashes found in filesystem
        let mut fs_hashes = HashSet::new();
        let mut data_files = HashMap::new();
        let mut meta_files = HashMap::new();
        let mut tmp_files = Vec::new();

        for file_path in files {
            if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                if let Some(hash) = filename.strip_suffix(".data") {
                    fs_hashes.insert(hash.to_string());
                    data_files.insert(hash.to_string(), file_path.clone());
                } else if let Some(hash) = filename.strip_suffix(".meta") {
                    fs_hashes.insert(hash.to_string());
                    meta_files.insert(hash.to_string(), file_path.clone());
                } else if filename.ends_with(".tmp") {
                    // Collect .tmp files for cleanup
                    tmp_files.push(file_path.clone());
                }
            }
        }

        // Delete orphaned .tmp files from failed writes
        for tmp_file in tmp_files {
            let _ = backend.delete_file(&tmp_file).await;
        }

        // Build map of index keys to their hashes
        let mut index_key_to_hash = HashMap::new();
        let entries_snapshot: Vec<_> = self
            .entries
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (key, _) in &entries_snapshot {
            let hash = key_to_hash(key);
            index_key_to_hash.insert(key.clone(), hash);
        }

        let index_is_empty = entries_snapshot.is_empty();

        if index_is_empty {
            // Recovery mode: discover files and add to index
            for hash in &fs_hashes {
                // Only process if we have both data and meta files
                if let (Some(_data_path), Some(meta_path)) =
                    (data_files.get(hash), meta_files.get(hash))
                {
                    // Try to load metadata
                    if let Ok(meta_bytes) = backend.read_file(meta_path).await {
                        if let Ok(meta_str) = String::from_utf8(meta_bytes.to_vec()) {
                            if let Ok(metadata) = serde_json::from_str::<EntryMetadata>(&meta_str) {
                                // Check if expired
                                let now = SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                if !metadata.is_expired(now) {
                                    // Add to index
                                    self.insert(metadata.cache_key.clone(), metadata);
                                }
                            }
                        }
                    }
                }
            }
            // Return early - no need to process existing entries
            return Ok(());
        } else {
            // Cleanup mode: remove orphaned files (files without index entries)
            let index_hashes: HashSet<_> = index_key_to_hash.values().cloned().collect();
            for hash in &fs_hashes {
                if !index_hashes.contains(hash) {
                    // Orphaned file - remove both data and meta
                    if let Some(data_path) = data_files.get(hash) {
                        let _ = backend.delete_file(data_path).await;
                    }
                    if let Some(meta_path) = meta_files.get(hash) {
                        let _ = backend.delete_file(meta_path).await;
                    }
                }
            }
        }

        // Process each index entry
        let mut keys_to_remove = Vec::new();
        let mut new_total_size = 0u64;

        for (key, metadata) in &entries_snapshot {
            let hash = key_to_hash(key);

            // Check if files exist
            let data_path = cache_key_to_file_path(entries_dir, key, false);
            let meta_path = cache_key_to_file_path(entries_dir, key, true);

            let data_exists = data_files.contains_key(&hash);
            let meta_exists = meta_files.contains_key(&hash);

            // If files don't exist, remove from index
            if !data_exists || !meta_exists {
                keys_to_remove.push(key.clone());
                continue;
            }

            // Check if expired
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if metadata.is_expired(now) {
                // Expired - remove from index and delete files
                keys_to_remove.push(key.clone());
                let _ = backend.delete_file(&data_path).await;
                let _ = backend.delete_file(&meta_path).await;
                continue;
            }

            // Recalculate size from actual file
            match backend.file_size(&data_path).await {
                Ok(actual_size) => {
                    new_total_size += actual_size;

                    // Update metadata if size changed
                    if actual_size != metadata.size_bytes {
                        let mut updated_metadata = metadata.clone();
                        updated_metadata.size_bytes = actual_size;
                        self.entries.write().insert(key.clone(), updated_metadata);
                    }
                }
                Err(_) => {
                    // Can't read file size - remove from index
                    keys_to_remove.push(key.clone());
                }
            }
        }

        // Remove invalid entries
        {
            let mut entries = self.entries.write();
            for key in &keys_to_remove {
                entries.remove(key);
            }
        }

        // Update total size
        self.total_size.store(new_total_size, Ordering::SeqCst);

        Ok(())
    }
}

/// Serializable snapshot of the cache index
#[derive(Serialize, Deserialize)]
#[allow(dead_code)] // Used in save_to_file/load_from_file (Phase 28.8)
struct IndexSnapshot {
    entries: Vec<IndexEntry>,
    version: u32,
}

/// A single index entry for serialization
#[derive(Serialize, Deserialize)]
#[allow(dead_code)] // Used in save_to_file/load_from_file (Phase 28.8)
struct IndexEntry {
    key: CacheKey,
    metadata: EntryMetadata,
}
