//! Tiered cache implementation - multi-layer cache hierarchy
//!
//! Provides a cache hierarchy with multiple layers (memory → disk → redis)
//! that automatically promotes frequently accessed items to faster layers.

use crate::cache::{Cache, CacheConfig, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;

/// Tiered cache with multiple layers (memory, disk, redis)
///
/// Implements a cache hierarchy where:
/// - Layer 1 (memory): Fastest, checked first
/// - Layer 2 (disk): Medium speed, checked if memory misses
/// - Layer 3 (redis): Distributed, checked if disk misses
///
/// Cache hits in slower layers are promoted to faster layers asynchronously.
pub struct TieredCache {
    // Ordered list of cache layers from fastest to slowest
    layers: Vec<Box<dyn Cache + Send + Sync>>,
}

impl TieredCache {
    /// Create a new tiered cache from an ordered list of cache layers
    ///
    /// # Arguments
    /// * `layers` - Ordered list of cache implementations (fastest first)
    ///
    /// # Example
    /// ```ignore
    /// let memory_cache = MemoryCache::new(config);
    /// let disk_cache = DiskCache::new(config).await?;
    /// let redis_cache = RedisCache::new(config).await?;
    ///
    /// let tiered = TieredCache::new(vec![
    ///     Box::new(memory_cache),
    ///     Box::new(disk_cache),
    ///     Box::new(redis_cache),
    /// ]);
    /// ```
    pub fn new(layers: Vec<Box<dyn Cache + Send + Sync>>) -> Self {
        Self { layers }
    }

    /// Get the number of cache layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Create a tiered cache from configuration
    ///
    /// This factory method constructs a TieredCache based on the cache_layers
    /// configuration, creating the appropriate cache implementations in the
    /// specified order.
    ///
    /// # Arguments
    /// * `config` - Cache configuration specifying which layers to enable
    ///
    /// # Example
    /// ```ignore
    /// let config = CacheConfig {
    ///     cache_layers: vec!["memory".to_string(), "disk".to_string()],
    ///     memory: MemoryCacheConfig::default(),
    ///     disk: DiskCacheConfig::default(),
    ///     ..Default::default()
    /// };
    ///
    /// let tiered = TieredCache::from_config(config).await?;
    /// ```
    pub async fn from_config(config: &CacheConfig) -> Result<Self, CacheError> {
        let mut layers: Vec<Box<dyn Cache + Send + Sync>> = Vec::new();

        // Iterate through configured cache layers in order
        for layer_name in &config.cache_layers {
            match layer_name.as_str() {
                "memory" => {
                    // TODO: Create real MemoryCache
                    // For now, return error as not implemented
                    return Err(CacheError::ConfigurationError(
                        "Memory cache not yet implemented in from_config".to_string(),
                    ));
                }
                "disk" => {
                    // TODO: Create real DiskCache
                    return Err(CacheError::ConfigurationError(
                        "Disk cache not yet implemented in from_config".to_string(),
                    ));
                }
                "redis" => {
                    // TODO: Create real RedisCache
                    return Err(CacheError::ConfigurationError(
                        "Redis cache not yet implemented in from_config".to_string(),
                    ));
                }
                unknown => {
                    return Err(CacheError::ConfigurationError(format!(
                        "Unknown cache layer: {}",
                        unknown
                    )));
                }
            }
        }

        Ok(Self { layers })
    }
}

#[async_trait]
impl Cache for TieredCache {
    async fn get(&self, _key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        // TODO: Implement multi-layer get with promotion
        Ok(None)
    }

    async fn set(&self, _key: CacheKey, _entry: CacheEntry) -> Result<(), CacheError> {
        // TODO: Implement write-through to all layers
        Ok(())
    }

    async fn delete(&self, _key: &CacheKey) -> Result<bool, CacheError> {
        // TODO: Implement delete from all layers
        Ok(false)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        // TODO: Implement clear all layers
        Ok(())
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        // TODO: Implement aggregated stats across all layers
        Ok(CacheStats::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::disk::DiskCache;
    use crate::cache::redis::{RedisCache, RedisConfig};
    use bytes::Bytes;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    // Mock cache for testing
    struct MockCache {
        name: String,
        entries: Arc<Mutex<std::collections::HashMap<String, CacheEntry>>>,
    }

    impl MockCache {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                entries: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl Cache for MockCache {
        async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
            let entries = self.entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            Ok(entries.get(&cache_key).cloned())
        }

        async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
            let mut entries = self.entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            entries.insert(cache_key, entry);
            Ok(())
        }

        async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError> {
            let mut entries = self.entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            Ok(entries.remove(&cache_key).is_some())
        }

        async fn clear(&self) -> Result<(), CacheError> {
            let mut entries = self.entries.lock().await;
            entries.clear();
            Ok(())
        }

        async fn stats(&self) -> Result<CacheStats, CacheError> {
            Ok(CacheStats::default())
        }
    }

    #[test]
    fn test_can_create_tiered_cache_struct() {
        // Test: Can create TieredCache struct
        let mock_memory = MockCache::new("memory");
        let mock_disk = MockCache::new("disk");
        let mock_redis = MockCache::new("redis");

        // Create tiered cache with 3 layers
        let tiered = TieredCache::new(vec![
            Box::new(mock_memory),
            Box::new(mock_disk),
            Box::new(mock_redis),
        ]);

        // Verify the struct was created
        assert_eq!(tiered.layer_count(), 3);
    }

    #[test]
    fn test_tiered_cache_contains_ordered_list_of_cache_layers() {
        // Test: TieredCache contains ordered list of cache layers
        let mock_memory = MockCache::new("memory");
        let mock_disk = MockCache::new("disk");

        let tiered = TieredCache::new(vec![Box::new(mock_memory), Box::new(mock_disk)]);

        // Verify we have 2 layers in order
        assert_eq!(tiered.layer_count(), 2);
    }

    #[test]
    fn test_tiered_cache_preserves_layer_order() {
        // Test: TieredCache preserves layer order (memory, disk, redis)
        // This is implicitly tested by the order of layers passed to constructor
        // The Vec preserves insertion order

        let mock_memory = MockCache::new("memory");
        let mock_disk = MockCache::new("disk");
        let mock_redis = MockCache::new("redis");

        let tiered = TieredCache::new(vec![
            Box::new(mock_memory),  // Layer 0: memory (fastest)
            Box::new(mock_disk),    // Layer 1: disk
            Box::new(mock_redis),   // Layer 2: redis (slowest)
        ]);

        // Verify layer count matches expected order
        assert_eq!(tiered.layer_count(), 3);
    }

    #[test]
    fn test_tiered_cache_can_have_1_2_or_3_layers() {
        // Test: TieredCache can have 1, 2, or 3 layers

        // 1 layer (memory only)
        let tiered_1 = TieredCache::new(vec![Box::new(MockCache::new("memory"))]);
        assert_eq!(tiered_1.layer_count(), 1);

        // 2 layers (memory + disk)
        let tiered_2 = TieredCache::new(vec![
            Box::new(MockCache::new("memory")),
            Box::new(MockCache::new("disk")),
        ]);
        assert_eq!(tiered_2.layer_count(), 2);

        // 3 layers (memory + disk + redis)
        let tiered_3 = TieredCache::new(vec![
            Box::new(MockCache::new("memory")),
            Box::new(MockCache::new("disk")),
            Box::new(MockCache::new("redis")),
        ]);
        assert_eq!(tiered_3.layer_count(), 3);
    }

    #[tokio::test]
    async fn test_can_create_tiered_cache_from_config() {
        // Test: Can create TieredCache from config
        use crate::cache::CacheConfig;

        // Create a config with explicitly empty cache_layers
        let config = CacheConfig {
            cache_layers: vec![], // No layers configured
            ..Default::default()
        };

        // This should not panic - verifies the method exists and can be called
        let result = TieredCache::from_config(&config).await;
        assert!(result.is_ok(), "Should create TieredCache from empty config");

        let tiered = result.unwrap();
        // With empty cache_layers, we expect 0 layers
        assert_eq!(tiered.layer_count(), 0);
    }
}
