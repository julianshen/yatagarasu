//! Main DiskCache implementation

use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use std::sync::Arc;
use super::backend::DiskBackend;
use super::index::CacheIndex;

/// Disk-based cache implementation
pub struct DiskCache {
    _backend: Arc<dyn DiskBackend>,
    _index: Arc<CacheIndex>,
}

impl DiskCache {
    pub fn new() -> Self {
        // Use platform-specific backend
        #[cfg(target_os = "linux")]
        let backend: Arc<dyn DiskBackend> = Arc::new(super::uring_backend::UringBackend::new());

        #[cfg(not(target_os = "linux"))]
        let backend: Arc<dyn DiskBackend> = Arc::new(super::tokio_backend::TokioFsBackend::new());

        Self {
            _backend: backend,
            _index: Arc::new(CacheIndex::new()),
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
        // TODO: Implement
        Ok(CacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        })
    }
}
