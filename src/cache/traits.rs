//! Cache trait definition
//!
//! This module defines the `Cache` trait that all cache implementations must satisfy.
//! The trait provides a common interface for memory, disk, and Redis caches.

use async_trait::async_trait;

use super::entry::{CacheEntry, CacheKey};
use super::error::CacheError;
use super::sendfile::SendfileResponse;
use super::stats::CacheStats;

/// Cache trait for different cache implementations (memory, disk, redis)
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a cache entry by key
    /// Returns None if the key is not found or the entry has expired
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError>;

    /// Set a cache entry
    /// Overwrites existing entry if key already exists
    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError>;

    /// Delete a cache entry by key
    /// Returns true if the entry was deleted, false if it didn't exist
    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError>;

    /// Clear all cache entries
    async fn clear(&self) -> Result<(), CacheError>;

    /// Clear all cache entries for a specific bucket
    /// Returns the number of entries deleted
    async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError>;

    /// Get cache statistics
    async fn stats(&self) -> Result<CacheStats, CacheError>;

    /// Get cache statistics for a specific bucket
    /// Returns partial stats (item count and size) for the bucket
    async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError>;

    /// Run pending async tasks (for caches that use async backends like moka)
    /// Default implementation is a no-op
    async fn run_pending_tasks(&self) {
        // No-op by default
    }

    /// Get sendfile response for zero-copy file serving (disk cache only)
    ///
    /// Returns a SendfileResponse with file path and metadata if:
    /// - The cache entry exists and is not expired
    /// - The file is on disk and larger than the sendfile threshold
    ///
    /// Default implementation returns None (not supported by this cache type).
    /// Only disk-based caches should implement this.
    async fn get_sendfile(&self, _key: &CacheKey) -> Result<Option<SendfileResponse>, CacheError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    // Mock Cache implementation for testing
    struct MockCache;

    #[async_trait]
    impl Cache for MockCache {
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
            Ok(0)
        }

        async fn stats(&self) -> Result<CacheStats, CacheError> {
            Ok(CacheStats::default())
        }

        async fn stats_bucket(&self, _bucket: &str) -> Result<CacheStats, CacheError> {
            Ok(CacheStats::default())
        }
    }

    #[test]
    fn test_can_define_cache_trait() {
        fn _assert_trait_exists<T: Cache>() {}
    }

    #[test]
    fn test_cache_trait_has_get_method() {
        async fn _test_get<T: Cache>(cache: &T, key: &CacheKey) {
            let _result: Result<Option<CacheEntry>, CacheError> = cache.get(key).await;
        }
    }

    #[test]
    fn test_cache_trait_has_set_method() {
        async fn _test_set<T: Cache>(cache: &T, key: CacheKey, entry: CacheEntry) {
            let _result: Result<(), CacheError> = cache.set(key, entry).await;
        }
    }

    #[test]
    fn test_cache_trait_has_delete_method() {
        async fn _test_delete<T: Cache>(cache: &T, key: &CacheKey) {
            let _result: Result<bool, CacheError> = cache.delete(key).await;
        }
    }

    #[test]
    fn test_cache_trait_has_clear_method() {
        async fn _test_clear<T: Cache>(cache: &T) {
            let _result: Result<(), CacheError> = cache.clear().await;
        }
    }

    #[test]
    fn test_cache_trait_has_stats_method() {
        async fn _test_stats<T: Cache>(cache: &T) {
            let _result: Result<CacheStats, CacheError> = cache.stats().await;
        }
    }

    #[test]
    fn test_cache_trait_compiles_with_signatures() {
        let _cache = MockCache;
    }

    #[tokio::test]
    async fn test_can_create_mock_implementation() {
        let cache = MockCache;
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "key".to_string(),
            etag: None,
            variant: None,
        };

        let get_result = cache.get(&key).await;
        assert!(get_result.is_ok());

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );
        let set_result = cache.set(key.clone(), entry).await;
        assert!(set_result.is_ok());

        let delete_result = cache.delete(&key).await;
        assert!(delete_result.is_ok());

        let clear_result = cache.clear().await;
        assert!(clear_result.is_ok());

        let stats_result = cache.stats().await;
        assert!(stats_result.is_ok());
    }

    #[test]
    fn test_mock_satisfies_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockCache>();
    }
}
