//! Memory cache implementation
//!
//! This module provides in-memory cache implementations:
//! - `MemoryCache`: High-performance LRU cache backed by moka
//! - `NullCache`: No-op implementation for disabled caching

use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::config::MemoryCacheConfig;
use super::entry::{CacheEntry, CacheKey};
use super::error::CacheError;
use super::stats::CacheStats;
use super::traits::Cache;

/// Statistics tracker using atomics for thread safety
pub(crate) struct CacheStatsTracker {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl CacheStatsTracker {
    /// Create a new stats tracker with all counters at zero
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Increment hit counter
    pub fn increment_hits(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment miss counter
    pub fn increment_misses(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment eviction counter
    pub fn increment_evictions(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current statistics
    pub fn snapshot(
        &self,
        current_size_bytes: u64,
        current_item_count: u64,
        max_size_bytes: u64,
    ) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            current_size_bytes,
            current_item_count,
            max_size_bytes,
        }
    }
}

/// MemoryCache wraps moka for our Cache trait
pub struct MemoryCache {
    cache: moka::future::Cache<CacheKey, CacheEntry>,
    stats: Arc<CacheStatsTracker>,
    max_item_size_bytes: u64,
}

impl MemoryCache {
    /// Create a new MemoryCache from configuration
    pub fn new(config: &MemoryCacheConfig) -> Self {
        use std::time::Duration;

        // Create stats tracker first so we can share it with the eviction listener
        let stats = Arc::new(CacheStatsTracker::new());
        let stats_clone = stats.clone();

        let cache = moka::future::Cache::builder()
            .max_capacity(config.max_cache_size_bytes())
            .time_to_live(Duration::from_secs(config.default_ttl_seconds))
            .weigher(|_key, entry: &CacheEntry| {
                let size = entry.size_bytes();
                if size > u32::MAX as usize {
                    u32::MAX
                } else {
                    size as u32
                }
            })
            .eviction_listener(move |_key, _value, cause| {
                // Increment eviction counter when entry is evicted
                // This includes both size-based evictions and expirations
                use moka::notification::RemovalCause;
                match cause {
                    RemovalCause::Size | RemovalCause::Expired => {
                        stats_clone.increment_evictions();
                    }
                    _ => {
                        // Don't count explicit removals (invalidate) as evictions
                    }
                }
            })
            .build();

        Self {
            cache,
            stats,
            max_item_size_bytes: config.max_item_size_bytes(),
        }
    }

    /// Get an entry from the cache
    /// Returns None if key not found or entry expired
    pub async fn get_entry(&self, key: &CacheKey) -> Option<CacheEntry> {
        match self.cache.get(key).await {
            Some(entry) => {
                self.stats.increment_hits();
                Some(entry)
            }
            None => {
                self.stats.increment_misses();
                None
            }
        }
    }

    /// Insert an entry into the cache
    /// Returns error if entry exceeds max_item_size
    pub async fn set_entry(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
        // Validate entry size
        let entry_size = entry.size_bytes() as u64;
        if entry_size > self.max_item_size_bytes {
            return Err(CacheError::StorageFull);
        }

        // Insert into moka cache
        self.cache.insert(key, entry).await;
        Ok(())
    }

    /// Delete an entry from the cache
    /// Returns true if the entry existed and was deleted
    pub async fn delete_entry(&self, key: &CacheKey) -> bool {
        self.cache.invalidate(key).await;
        // Moka's invalidate returns () not bool, so we can't determine if key existed
        // Return true to indicate operation completed
        true
    }

    /// Clear all entries from the cache
    pub async fn clear_all(&self) {
        self.cache.invalidate_all();
        // Note: This initiates invalidation but may not complete immediately
        // Call run_pending_tasks() to ensure completion
    }

    /// Run pending maintenance tasks
    /// Forces moka to process pending evictions, expirations, and invalidations
    pub async fn run_pending(&self) {
        self.cache.run_pending_tasks().await;
    }

    /// Get current weighted size in bytes
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }

    /// Get current entry count (approximate due to eventual consistency)
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Get cache statistics snapshot
    fn get_stats(&self) -> CacheStats {
        self.stats.snapshot(
            self.cache.weighted_size(),
            self.cache.entry_count(),
            self.max_item_size_bytes,
        )
    }
}

// Implement Cache trait for MemoryCache
#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        Ok(self.get_entry(key).await)
    }

    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
        self.set_entry(key, entry).await
    }

    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError> {
        Ok(self.delete_entry(key).await)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        self.clear_all().await;
        Ok(())
    }

    async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError> {
        // Iterate through all keys and delete those matching the bucket
        // Note: moka's iter() returns Arc<K>, so we need to dereference
        let keys_to_delete: Vec<CacheKey> = self
            .cache
            .iter()
            .filter(|(k, _)| k.bucket == bucket)
            .map(|(k, _)| (*k).clone())
            .collect();

        let count = keys_to_delete.len();
        for key in keys_to_delete {
            self.cache.invalidate(&key).await;
        }

        // Run pending tasks to ensure invalidations complete
        self.cache.run_pending_tasks().await;

        Ok(count)
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        Ok(self.get_stats())
    }

    async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError> {
        // Calculate stats for a specific bucket by iterating entries
        let mut size_bytes: u64 = 0;
        let mut item_count: u64 = 0;

        for (key, entry) in self.cache.iter() {
            if key.bucket == bucket {
                size_bytes += entry.size_bytes() as u64;
                item_count += 1;
            }
        }

        Ok(CacheStats {
            hits: 0,      // Not tracked per-bucket
            misses: 0,    // Not tracked per-bucket
            evictions: 0, // Not tracked per-bucket
            current_size_bytes: size_bytes,
            current_item_count: item_count,
            max_size_bytes: self.max_item_size_bytes, // Overall max
        })
    }

    async fn run_pending_tasks(&self) {
        self.cache.run_pending_tasks().await;
    }
}

/// NullCache is a no-op cache implementation used when caching is disabled
pub struct NullCache;

#[async_trait]
impl Cache for NullCache {
    async fn get(&self, _key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        Ok(None)
    }

    async fn set(&self, _key: CacheKey, _entry: CacheEntry) -> Result<(), CacheError> {
        Ok(())
    }

    async fn delete(&self, _key: &CacheKey) -> Result<bool, CacheError> {
        Ok(false)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        Ok(())
    }

    async fn clear_bucket(&self, _bucket: &str) -> Result<usize, CacheError> {
        // NullCache has no entries, so nothing to clear
        Ok(0)
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        Ok(CacheStats {
            hits: 0,
            misses: 0,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        })
    }

    async fn stats_bucket(&self, _bucket: &str) -> Result<CacheStats, CacheError> {
        // NullCache has no entries
        Ok(CacheStats::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_can_create_cache_stats_tracker_struct() {
        let tracker = CacheStatsTracker::new();
        let stats = tracker.snapshot(0, 0, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
    }

    #[test]
    fn test_cache_stats_tracker_provides_increment_methods() {
        let tracker = CacheStatsTracker::new();
        tracker.increment_hits();
        tracker.increment_misses();
        tracker.increment_evictions();

        let stats = tracker.snapshot(0, 0, 0);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cache_stats_tracker_provides_snapshot_method() {
        let tracker = CacheStatsTracker::new();
        tracker.increment_hits();
        tracker.increment_hits();
        tracker.increment_misses();

        let stats = tracker.snapshot(1024, 5, 10240);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.current_size_bytes, 1024);
        assert_eq!(stats.current_item_count, 5);
        assert_eq!(stats.max_size_bytes, 10240);
    }

    #[tokio::test]
    async fn test_memory_cache_new_creates_moka_cache_with_max_capacity() {
        let config = MemoryCacheConfig {
            max_item_size_mb: 10,
            max_cache_size_mb: 100,
            default_ttl_seconds: 3600,
        };
        let cache = MemoryCache::new(&config);
        assert_eq!(cache.max_item_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_memory_cache_contains_config_parameters() {
        let config = MemoryCacheConfig {
            max_item_size_mb: 5,
            max_cache_size_mb: 50,
            default_ttl_seconds: 1800,
        };
        let cache = MemoryCache::new(&config);
        assert_eq!(cache.max_item_size_bytes, 5 * 1024 * 1024);
    }

    #[test]
    fn test_memory_cache_implements_cache_trait() {
        fn assert_cache<T: Cache>() {}
        assert_cache::<MemoryCache>();
    }

    #[test]
    fn test_memory_cache_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MemoryCache>();
    }

    #[tokio::test]
    async fn test_memory_cache_get_returns_none_for_missing_key() {
        let config = MemoryCacheConfig::default();
        let cache = MemoryCache::new(&config);

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "nonexistent".to_string(),
            etag: None,
        };

        let result = cache.get(&key).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_set_and_get() {
        let config = MemoryCacheConfig::default();
        let cache = MemoryCache::new(&config);

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "myfile.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("hello world"),
            "text/plain".to_string(),
            "etag123".to_string(),
            None,
            None,
        );

        cache.set(key.clone(), entry).await.unwrap();

        let result = cache.get(&key).await.unwrap();
        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.data, Bytes::from("hello world"));
        assert_eq!(retrieved.content_type, "text/plain");
    }

    #[tokio::test]
    async fn test_memory_cache_delete() {
        let config = MemoryCacheConfig::default();
        let cache = MemoryCache::new(&config);

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "deleteme.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );

        cache.set(key.clone(), entry).await.unwrap();
        cache.delete(&key).await.unwrap();
        cache.run_pending_tasks().await;

        let result = cache.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_clear() {
        let config = MemoryCacheConfig::default();
        let cache = MemoryCache::new(&config);

        let key1 = CacheKey {
            bucket: "test".to_string(),
            object_key: "file1.txt".to_string(),
            etag: None,
        };
        let key2 = CacheKey {
            bucket: "test".to_string(),
            object_key: "file2.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );

        cache.set(key1.clone(), entry.clone()).await.unwrap();
        cache.set(key2.clone(), entry).await.unwrap();

        cache.clear().await.unwrap();
        cache.run_pending_tasks().await;

        let result1 = cache.get(&key1).await.unwrap();
        let result2 = cache.get(&key2).await.unwrap();
        assert!(result1.is_none());
        assert!(result2.is_none());
    }

    #[tokio::test]
    async fn test_memory_cache_stats() {
        let config = MemoryCacheConfig::default();
        let cache = MemoryCache::new(&config);

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };

        // Miss
        cache.get(&key).await.unwrap();

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );
        cache.set(key.clone(), entry).await.unwrap();

        // Hit
        cache.get(&key).await.unwrap();

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_memory_cache_rejects_oversized_entries() {
        let config = MemoryCacheConfig {
            max_item_size_mb: 1, // 1MB max
            max_cache_size_mb: 10,
            default_ttl_seconds: 3600,
        };
        let cache = MemoryCache::new(&config);

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "large.bin".to_string(),
            etag: None,
        };

        // Create a 2MB entry
        let large_data = Bytes::from(vec![0u8; 2 * 1024 * 1024]);
        let entry = CacheEntry::new(
            large_data,
            "application/octet-stream".to_string(),
            "etag".to_string(),
            None,
            None,
        );

        let result = cache.set(key, entry).await;
        assert!(result.is_err());
        matches!(result.unwrap_err(), CacheError::StorageFull);
    }

    #[tokio::test]
    async fn test_null_cache_get_returns_none() {
        let cache = NullCache;
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };

        let result = cache.get(&key).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_null_cache_set_succeeds() {
        let cache = NullCache;
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };
        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );

        let result = cache.set(key, entry).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_null_cache_stats_returns_zeros() {
        let cache = NullCache;
        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.current_size_bytes, 0);
        assert_eq!(stats.current_item_count, 0);
    }
}
