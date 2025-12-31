//! Main DiskCache implementation

use super::backend::DiskBackend;
use super::index::CacheIndex;
use crate::cache::sendfile::{SendfileConfig, SendfileResponse};
use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Disk-based cache implementation
pub struct DiskCache {
    backend: Arc<dyn DiskBackend>,
    pub(crate) index: Arc<CacheIndex>, // pub(crate) for testing
    cache_dir: PathBuf,
    max_size_bytes: u64,
    eviction_count: Arc<AtomicU64>,
    /// Cache hit counter
    hit_count: Arc<AtomicU64>,
    /// Cache miss counter
    miss_count: Arc<AtomicU64>,
    /// sendfile configuration for zero-copy file serving
    sendfile_config: SendfileConfig,
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

    /// Create a new DiskCache with custom cache directory and size
    pub fn with_config(cache_dir: PathBuf, max_size_bytes: u64) -> Self {
        Self::with_sendfile_config(cache_dir, max_size_bytes, SendfileConfig::default())
    }

    /// Create a new DiskCache with custom cache directory, size, and sendfile config
    pub fn with_sendfile_config(
        cache_dir: PathBuf,
        max_size_bytes: u64,
        sendfile_config: SendfileConfig,
    ) -> Self {
        // Use platform-specific backend (UringBackend on Linux, TokioFsBackend elsewhere)
        // io-uring crate (not tokio-uring) provides Send + Sync types on Linux
        let backend: Arc<dyn DiskBackend> = Arc::new(super::platform_backend::create_backend());

        Self {
            backend,
            index: Arc::new(CacheIndex::new()),
            cache_dir,
            max_size_bytes,
            eviction_count: Arc::new(AtomicU64::new(0)),
            hit_count: Arc::new(AtomicU64::new(0)),
            miss_count: Arc::new(AtomicU64::new(0)),
            sendfile_config,
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
            None => {
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        };

        // Check if expired
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if metadata.is_expired(now) {
            // Remove expired entry
            let _ = self.delete(key).await;
            self.miss_count.fetch_add(1, Ordering::Relaxed);
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
                self.miss_count.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }
        };

        // Track cache hit
        self.hit_count.fetch_add(1, Ordering::Relaxed);

        // Reconstruct CacheEntry with metadata fields
        let entry = CacheEntry {
            data: data.clone(),
            content_type: metadata.content_type.clone(),
            content_length: data.len(),
            etag: metadata.etag.clone(),
            last_modified: metadata.last_modified.clone(),
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
            entry.content_type.clone(),
            entry.etag.clone(),
            entry.last_modified.clone(),
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

        // Delete all files from cache directory
        // This prevents orphaned files from persisting on disk
        if let Ok(mut entries) = tokio::fs::read_dir(&self.cache_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                // Only delete regular files (not directories)
                if path.is_file() {
                    if let Err(e) = tokio::fs::remove_file(&path).await {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to delete cache file during clear"
                        );
                    }
                }
            }
        }

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
            hits: self.hit_count.load(Ordering::Relaxed),
            misses: self.miss_count.load(Ordering::Relaxed),
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

    /// Get sendfile response for zero-copy file serving
    ///
    /// Returns a SendfileResponse with the file path and metadata if:
    /// - sendfile is enabled and supported on this platform
    /// - The cache entry exists and is not expired
    /// - The file size exceeds the sendfile threshold
    ///
    /// This allows the caller to use the Linux sendfile() syscall for
    /// direct kernel-to-kernel data transfer, achieving 2.6x throughput
    /// improvement for large files.
    async fn get_sendfile(&self, key: &CacheKey) -> Result<Option<SendfileResponse>, CacheError> {
        use super::utils::{generate_paths, key_to_hash};
        use std::time::SystemTime;

        // Check if entry exists in index
        let metadata = match self.index.get(key) {
            Some(meta) => meta,
            None => {
                return Ok(None);
            }
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

        // Check if sendfile should be used for this file size
        if !self
            .sendfile_config
            .should_use_sendfile(metadata.size_bytes)
        {
            return Ok(None);
        }

        // Generate file path
        let hash = key_to_hash(key);
        let (data_path, _meta_path) = generate_paths(&self.cache_dir, &hash);

        // Verify file exists before returning sendfile response
        if self.backend.file_size(&data_path).await.is_err() {
            // File doesn't exist - remove from index
            let _ = self.delete(key).await;
            return Ok(None);
        }

        // Create sendfile response with file metadata
        // Note: We don't increment hit count here because the proxy calls
        // cache.get() first, which already tracks the hit. get_sendfile()
        // is only used to check eligibility for zero-copy serving.
        let response = SendfileResponse::new(
            data_path,
            metadata.size_bytes,
            metadata.content_type.clone(),
            Some(metadata.etag.clone()),
            metadata.last_modified.clone(),
        );

        Ok(Some(response))
    }
}
