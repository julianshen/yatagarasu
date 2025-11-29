//! Main DiskCache implementation

use super::backend::DiskBackend;
use super::index::CacheIndex;
use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Disk-based cache implementation
pub struct DiskCache {
    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache::get/set implementation)
    backend: Arc<dyn DiskBackend>,
    pub(crate) index: Arc<CacheIndex>, // pub(crate) for testing
    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache::set for file paths)
    cache_dir: PathBuf,
    max_size_bytes: u64,
    eviction_count: Arc<AtomicU64>,
}

impl Default for DiskCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DiskCache {
    pub fn new() -> Self {
        // Default configuration with 1GB max size
        Self::with_config(PathBuf::from("/tmp/yatagarasu_cache"), 1024 * 1024 * 1024)
    }

    pub fn with_config(cache_dir: PathBuf, max_size_bytes: u64) -> Self {
        // Use platform-specific backend (UringBackend on Linux, TokioFsBackend elsewhere)
        // io-uring crate (not tokio-uring) provides Send + Sync types on Linux
        let backend: Arc<dyn DiskBackend> = Arc::new(super::platform_backend::create_backend());

        Self {
            backend,
            index: Arc::new(CacheIndex::new()),
            cache_dir,
            max_size_bytes,
            eviction_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Cache for DiskCache {
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        use super::utils::generate_paths;
        use super::utils::key_to_hash;
        use std::time::SystemTime;

        // Check if entry exists in index
        let metadata = match self.index.get(key) {
            Some(meta) => meta,
            None => return Ok(None),
        };

        // Check if expired
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if metadata.is_expired(now) {
            // Remove expired entry
            let _ = self.delete(key).await;
            return Ok(None);
        }

        // Read data file
        let hash = key_to_hash(key);
        let (data_path, _meta_path) = generate_paths(&self.cache_dir, &hash);

        let data = match self.backend.read_file(&data_path).await {
            Ok(d) => d,
            Err(_) => {
                // File doesn't exist - remove from index
                let _ = self.delete(key).await;
                return Ok(None);
            }
        };

        // Reconstruct CacheEntry
        let entry = CacheEntry {
            data: data.clone(),
            content_type: "application/octet-stream".to_string(), // TODO: Store in metadata
            content_length: data.len(),
            etag: "".to_string(), // TODO: Store in metadata
            last_modified: None,  // TODO: Store in metadata
            created_at: SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(metadata.created_at),
            expires_at: SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(metadata.expires_at),
            last_accessed_at: SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(metadata.last_accessed_at),
        };

        Ok(Some(entry))
    }

    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
        use super::types::EntryMetadata;
        use super::utils::{generate_paths, key_to_hash};
        use bytes::Bytes;
        use std::time::SystemTime;

        let new_entry_size = entry.data.len() as u64;

        // Evict entries if necessary to make room for new entry
        while self.index.total_size() + new_entry_size > self.max_size_bytes {
            // Find least recently accessed entry
            if let Some((lru_key, _lru_metadata)) = self.index.find_lru_entry() {
                // Delete the LRU entry
                let _ = self.delete(&lru_key).await;
                // Increment eviction counter
                self.eviction_count.fetch_add(1, Ordering::SeqCst);
            } else {
                // No entries to evict - cache is empty
                break;
            }
        }

        // Generate file paths
        let hash = key_to_hash(&key);
        let (data_path, meta_path) = generate_paths(&self.cache_dir, &hash);

        // Write data file
        self.backend
            .write_file_atomic(&data_path, entry.data.clone())
            .await?;

        // Create metadata
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at_unix = entry
            .expires_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let metadata = EntryMetadata::new(
            key.clone(),
            data_path.clone(),
            entry.data.len() as u64,
            now,
            expires_at_unix,
        );

        // Write metadata file
        let meta_json = serde_json::to_string(&metadata)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        self.backend
            .write_file_atomic(&meta_path, Bytes::from(meta_json))
            .await?;

        // Update index
        self.index.insert(key, metadata);

        Ok(())
    }

    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError> {
        use super::utils::generate_paths;
        use super::utils::key_to_hash;

        // Try to remove from index first
        let _metadata = match self.index.remove(key) {
            Some(meta) => meta,
            None => return Ok(false), // Entry doesn't exist
        };

        // Generate file paths
        let hash = key_to_hash(key);
        let (data_path, meta_path) = generate_paths(&self.cache_dir, &hash);

        // Delete both files (ignore errors - index is already updated)
        let _ = self.backend.delete_file(&data_path).await;
        let _ = self.backend.delete_file(&meta_path).await;

        Ok(true)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        // Clear the index
        self.index.clear();

        // Reset eviction counter
        self.eviction_count.store(0, Ordering::SeqCst);

        // TODO: Optionally delete all files from disk (left for later optimization)
        // For now, orphaned files will be cleaned up during next validate_and_repair()

        Ok(())
    }

    async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError> {
        // Find all keys belonging to this bucket
        let keys_to_delete = self.index.keys_for_bucket(bucket);
        let count = keys_to_delete.len();

        // Delete each key
        for key in keys_to_delete {
            let _ = self.delete(&key).await;
        }

        Ok(count)
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        Ok(CacheStats {
            hits: 0,   // TODO: Track hits
            misses: 0, // TODO: Track misses
            evictions: self.eviction_count.load(Ordering::SeqCst),
            current_size_bytes: self.index.total_size(),
            current_item_count: self.index.entry_count() as u64,
            max_size_bytes: self.max_size_bytes,
        })
    }

    async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError> {
        let (size_bytes, item_count) = self.index.stats_for_bucket(bucket);

        Ok(CacheStats {
            hits: 0,      // Not tracked per-bucket
            misses: 0,    // Not tracked per-bucket
            evictions: 0, // Not tracked per-bucket
            current_size_bytes: size_bytes,
            current_item_count: item_count,
            max_size_bytes: self.max_size_bytes, // Overall max
        })
    }
}
