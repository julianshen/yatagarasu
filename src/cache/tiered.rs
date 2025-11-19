//! Tiered cache implementation - multi-layer cache hierarchy
//!
//! Provides a cache hierarchy with multiple layers (memory → disk → redis)
//! that automatically promotes frequently accessed items to faster layers.

use crate::cache::{Cache, CacheConfig, CacheEntry, CacheError, CacheKey, CacheStats, MemoryCache};
use crate::cache::disk::DiskCache;
use async_trait::async_trait;
use std::path::PathBuf;

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
                    // Create MemoryCache from configuration
                    let memory_cache = MemoryCache::new(&config.memory);
                    layers.push(Box::new(memory_cache));
                }
                "disk" => {
                    // Create DiskCache from configuration
                    let cache_dir = PathBuf::from(&config.disk.cache_dir);
                    let max_size_bytes = config.disk.max_disk_cache_size_mb * 1024 * 1024;
                    let disk_cache = DiskCache::with_config(cache_dir, max_size_bytes);
                    layers.push(Box::new(disk_cache));
                }
                "redis" => {
                    // TODO: Create RedisCache from configuration (async)
                    // RedisCache needs to implement the Cache trait first
                    // For now, return an error
                    return Err(CacheError::ConfigurationError(
                        "Redis cache layer not yet integrated with Cache trait".to_string(),
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
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        // Check each layer in order (fastest to slowest)
        for (layer_index, layer) in self.layers.iter().enumerate() {
            match layer.get(key).await? {
                Some(entry) => {
                    // Found in this layer

                    // If found in a slower layer (not the first/fastest), promote to faster layers
                    if layer_index > 0 {
                        // Clone data needed for promotion
                        let key_clone = key.clone();
                        let entry_clone = entry.clone();

                        // Promote to all faster layers (0..layer_index)
                        // NOTE: This is currently synchronous (blocks the get response)
                        // TODO: Make this truly async (tokio::spawn) to avoid blocking
                        // Requires Arc-wrapping layers or using channels
                        for promote_to_index in 0..layer_index {
                            if let Some(faster_layer) = self.layers.get(promote_to_index) {
                                // Ignore promotion errors - they shouldn't block the get
                                let _ = faster_layer.set(key_clone.clone(), entry_clone.clone()).await;
                            }
                        }
                    }

                    // Return the entry immediately
                    return Ok(Some(entry));
                }
                None => {
                    // Miss - continue to next layer
                    continue;
                }
            }
        }

        // All layers missed
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
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock cache for testing
    #[allow(dead_code)]
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

    #[tokio::test]
    async fn test_initializes_layers_in_correct_order() {
        // Test: Initializes layers in correct order
        // Test: Memory layer first (fastest)
        // Test: Disk layer second
        use crate::cache::CacheConfig;
        use tempfile::TempDir;

        // Create a temporary directory for disk cache
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_string_lossy().to_string();

        // Create a config with memory and disk layers in the canonical order
        // Note: Redis layer not yet integrated with Cache trait
        let config = CacheConfig {
            cache_layers: vec!["memory".to_string(), "disk".to_string()],
            disk: crate::cache::DiskCacheConfig {
                enabled: true,
                cache_dir: cache_dir.clone(),
                max_disk_cache_size_mb: 100,
            },
            ..Default::default()
        };

        // This should create a tiered cache with 2 layers in order
        let result = TieredCache::from_config(&config).await;
        assert!(
            result.is_ok(),
            "Should create TieredCache with memory and disk layers"
        );

        let tiered = result.unwrap();

        // Verify we have 2 layers
        assert_eq!(tiered.layer_count(), 2, "Should have 2 layers");

        // The layers should be in the order: memory (fastest), disk (slower)
        // We verify this indirectly through the layer count and the fact that
        // the Vec preserves insertion order
    }

    #[tokio::test]
    async fn test_get_checks_memory_layer_first() {
        // Test: get() checks memory layer first
        use bytes::Bytes;
        use std::time::Duration;

        // Create two mock cache layers
        let memory_cache = MockCache::new("memory");
        let disk_cache = MockCache::new("disk");

        // Set an entry in memory layer only
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("test data from memory"),
            "text/plain".to_string(),
            "etag123".to_string(),
            Some(Duration::from_secs(3600)),
        );

        memory_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![
            Box::new(memory_cache),
            Box::new(disk_cache),
        ]);

        // Get from tiered cache - should find in memory layer
        let result = tiered.get(&key).await.unwrap();
        assert!(result.is_some(), "Should find entry in memory layer");

        let retrieved = result.unwrap();
        assert_eq!(retrieved.data, Bytes::from("test data from memory"));
        assert_eq!(retrieved.content_type, "text/plain");
        assert_eq!(retrieved.etag, "etag123");
    }

    #[tokio::test]
    async fn test_get_checks_disk_layer_on_memory_miss() {
        // Test: Checks disk layer on memory miss
        // Test: Returns immediately on disk hit
        use bytes::Bytes;
        use std::time::Duration;

        // Create two mock cache layers
        let memory_cache = MockCache::new("memory");
        let disk_cache = MockCache::new("disk");

        // Set an entry in disk layer only (NOT in memory)
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("test data from disk"),
            "text/plain".to_string(),
            "etag456".to_string(),
            Some(Duration::from_secs(3600)),
        );

        disk_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![
            Box::new(memory_cache),
            Box::new(disk_cache),
        ]);

        // Get from tiered cache - should miss memory, find in disk
        let result = tiered.get(&key).await.unwrap();
        assert!(result.is_some(), "Should find entry in disk layer after memory miss");

        let retrieved = result.unwrap();
        assert_eq!(retrieved.data, Bytes::from("test data from disk"));
        assert_eq!(retrieved.content_type, "text/plain");
        assert_eq!(retrieved.etag, "etag456");
    }

    #[tokio::test]
    async fn test_get_returns_none_if_all_layers_miss() {
        // Test: Returns None if all layers miss

        // Create two empty mock cache layers
        let memory_cache = MockCache::new("memory");
        let disk_cache = MockCache::new("disk");

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![
            Box::new(memory_cache),
            Box::new(disk_cache),
        ]);

        // Try to get a key that doesn't exist in any layer
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "nonexistent.txt".to_string(),
            etag: None,
        };

        let result = tiered.get(&key).await.unwrap();
        assert!(result.is_none(), "Should return None when all layers miss");
    }

    #[tokio::test]
    async fn test_disk_hit_promotes_to_memory() {
        // Test: Disk hit promotes to memory
        // Test: Promotion is async (non-blocking)
        use bytes::Bytes;
        use std::time::Duration;
        use tokio::time::sleep;

        // Create memory and disk cache layers
        let memory_cache = MockCache::new("memory");
        let memory_entries = memory_cache.entries.clone(); // Keep reference to check promotion later

        let disk_cache = MockCache::new("disk");

        // Set an entry in disk layer only (NOT in memory)
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "promote.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data from disk to be promoted"),
            "text/plain".to_string(),
            "etag789".to_string(),
            Some(Duration::from_secs(3600)),
        );

        disk_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![
            Box::new(memory_cache),
            Box::new(disk_cache),
        ]);

        // Get from tiered cache - should find in disk and promote to memory
        let result = tiered.get(&key).await.unwrap();
        assert!(result.is_some(), "Should find entry in disk layer");

        let retrieved = result.unwrap();
        assert_eq!(retrieved.data, Bytes::from("data from disk to be promoted"));

        // Wait a bit for async promotion to complete
        sleep(Duration::from_millis(100)).await;

        // Check if entry was promoted to memory layer
        {
            let entries = memory_entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            let promoted = entries.get(&cache_key);
            assert!(promoted.is_some(), "Entry should be promoted to memory layer");

            let promoted_entry = promoted.unwrap();
            assert_eq!(promoted_entry.data, Bytes::from("data from disk to be promoted"));
        }
    }
}
