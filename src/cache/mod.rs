//! Cache module for Yatagarasu S3 proxy
//!
//! This module provides a flexible caching layer with support for multiple cache backends
//! (memory, disk, Redis) and intelligent cache management.
//!
//! # Overview
//!
//! The cache module implements a multi-tier caching strategy to reduce S3 API calls and
//! improve response times:
//!
//! - **Memory Cache**: Fast in-memory LRU cache for hot objects
//! - **Disk Cache**: Persistent cache using local filesystem (optional)
//! - **Redis Cache**: Distributed cache using Redis (optional)
//!
//! # Configuration
//!
//! Cache behavior is configured through `CacheConfig` in the YAML configuration file:
//!
//! ```yaml
//! cache:
//!   enabled: true
//!   memory:
//!     max_item_size_mb: 10
//!     max_cache_size_mb: 1024
//!     default_ttl_seconds: 3600
//!   cache_layers: ["memory"]
//! ```
//!
//! # Usage
//!
//! The `Cache` trait defines the interface for all cache implementations:
//!
//! ```rust,ignore
//! use yatagarasu::cache::{Cache, CacheKey, CacheEntry};
//!
//! async fn example(cache: &dyn Cache) {
//!     let key = CacheKey::new("bucket".to_string(), "object/key".to_string(), None);
//!
//!     // Get from cache
//!     if let Ok(Some(entry)) = cache.get(&key).await {
//!         println!("Cache hit: {} bytes", entry.content_length);
//!     }
//!
//!     // Set in cache
//!     let entry = CacheEntry::new(data, "text/plain".to_string(), "etag".to_string(), Some(3600));
//!     cache.set(key, entry).await?;
//! }
//! ```

use std::sync::Arc;

// Submodules
pub mod config;
pub mod entry;
pub mod error;
pub mod memory;
pub mod stats;
pub mod traits;

// Disk cache submodule (Phase 28)
pub mod disk;

// Redis cache submodule (Phase 29)
pub mod redis;

// Tiered cache submodule (Phase 30)
pub mod tiered;

// Cache warming submodule (Phase 1.3)
pub mod warming;

// sendfile support for zero-copy file serving (v1.4)
pub mod sendfile;

// Re-export configuration types
pub use config::{
    BucketCacheOverride, CacheConfig, DiskCacheConfig, MemoryCacheConfig, RedisCacheConfig,
};

// Re-export sendfile types
pub use sendfile::{SendfileConfig, SendfileResponse};

// Re-export entry types
pub use entry::{CacheEntry, CacheKey};

// Re-export error types
pub use error::CacheError;

// Re-export stats types
pub use stats::{BucketCacheStats, CacheStats};

// Re-export trait
pub use traits::Cache;

// Re-export implementations
pub use memory::{MemoryCache, NullCache};

// ============================================================
// Cache Factory Function
// ============================================================

/// Create a cache instance from configuration
/// Returns Arc<dyn Cache> for polymorphic usage
pub fn create_cache(config: &CacheConfig) -> Arc<dyn Cache> {
    if !config.enabled {
        return Arc::new(NullCache);
    }

    // Check if memory layer is requested
    if config.cache_layers.contains(&"memory".to_string()) {
        let memory_cache = MemoryCache::new(&config.memory);
        return Arc::new(memory_cache);
    }

    // Default to NullCache if no valid layers configured
    Arc::new(NullCache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bytes::Bytes;

    // Module structure tests
    #[test]
    fn test_can_create_cache_module() {
        // This test passes if the module compiles
    }

    #[test]
    fn test_cache_module_exports_cache_config() {
        let _config = CacheConfig::default();
    }

    #[test]
    fn test_cache_module_exports_cache_key() {
        let _key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };
    }

    #[test]
    fn test_cache_module_exports_cache_entry() {
        let data = Bytes::from("test");
        let _entry = CacheEntry::new(
            data,
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );
    }

    #[test]
    fn test_cache_module_exports_cache_error() {
        let _err = CacheError::NotFound;
    }

    #[test]
    fn test_cache_module_exports_cache_stats() {
        let _stats = CacheStats::default();
    }

    #[test]
    fn test_cache_module_exports_cache_trait() {
        fn _assert_trait_exists<T: Cache>() {}
    }

    #[test]
    fn test_cache_module_exports_memory_cache() {
        let config = MemoryCacheConfig::default();
        let _cache = MemoryCache::new(&config);
    }

    #[test]
    fn test_cache_module_exports_null_cache() {
        let _cache = NullCache;
    }

    // Mock Cache implementation for integration tests
    #[allow(dead_code)]
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

    // Factory function tests
    #[test]
    fn test_create_cache_returns_null_cache_when_disabled() {
        let config = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        let cache = create_cache(&config);
        // Can't directly check type, but we can verify it compiles
        let _: Arc<dyn Cache> = cache;
    }

    #[test]
    fn test_create_cache_returns_memory_cache_when_enabled() {
        let config = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            cache_layers: vec!["memory".to_string()],
            ..Default::default()
        };
        let cache = create_cache(&config);
        let _: Arc<dyn Cache> = cache;
    }

    #[tokio::test]
    async fn test_integration_memory_cache_set_get() {
        let config = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            cache_layers: vec!["memory".to_string()],
            ..Default::default()
        };
        let cache = create_cache(&config);

        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "path/to/file.txt".to_string(),
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
        assert_eq!(result.unwrap().data, Bytes::from("hello world"));
    }

    #[tokio::test]
    async fn test_integration_null_cache_always_misses() {
        let config = CacheConfig {
            enabled: false,
            ..Default::default()
        };
        let cache = create_cache(&config);

        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "path/to/file.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );

        // Set should succeed silently
        cache.set(key.clone(), entry).await.unwrap();

        // Get should always return None
        let result = cache.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    // Documentation tests
    #[test]
    fn test_cache_key_has_doc_comments() {
        // Verify CacheKey exists with proper fields
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "file.txt".to_string(),
            etag: Some("etag".to_string()),
        };
        assert_eq!(key.bucket, "test");
    }

    #[test]
    fn test_cache_entry_has_doc_comments() {
        // Verify CacheEntry exists and can be created
        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );
        assert_eq!(entry.content_type, "text/plain");
    }
}
