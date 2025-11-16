//! Main DiskCache implementation

use super::backend::DiskBackend;
use super::index::CacheIndex;
use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// Disk-based cache implementation
pub struct DiskCache {
    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache::get/set implementation)
    backend: Arc<dyn DiskBackend>,
    index: Arc<CacheIndex>,
    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache::set for file paths)
    cache_dir: PathBuf,
    max_size_bytes: u64,
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
        // Use platform-specific backend
        #[cfg(target_os = "linux")]
        let backend: Arc<dyn DiskBackend> = Arc::new(super::uring_backend::UringBackend::new());

        #[cfg(not(target_os = "linux"))]
        let backend: Arc<dyn DiskBackend> = Arc::new(super::tokio_backend::TokioFsBackend::new());

        Self {
            backend,
            index: Arc::new(CacheIndex::new()),
            cache_dir,
            max_size_bytes,
        }
    }
}

#[async_trait]
impl Cache for DiskCache {
    async fn get(&self, _key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        // TODO: Implement
        Ok(None)
    }

    async fn set(&self, _key: CacheKey, _entry: CacheEntry) -> Result<(), CacheError> {
        // TODO: Implement
        Ok(())
    }

    async fn delete(&self, _key: &CacheKey) -> Result<bool, CacheError> {
        // TODO: Implement
        Ok(false)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        // TODO: Implement
        Ok(())
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        Ok(CacheStats {
            hits: 0, // TODO: Track hits
            misses: 0, // TODO: Track misses
            evictions: 0, // TODO: Track evictions
            current_size_bytes: self.index.total_size(),
            current_item_count: self.index.entry_count() as u64,
            max_size_bytes: self.max_size_bytes,
        })
    }
}
