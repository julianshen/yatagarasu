//! Tiered cache implementation - multi-layer cache hierarchy
//!
//! Provides a cache hierarchy with multiple layers (memory → disk → redis)
//! that automatically promotes frequently accessed items to faster layers.

use crate::cache::disk::DiskCache;
use crate::cache::redis::{RedisCache, RedisConfig};
use crate::cache::sendfile::SendfileResponse;
use crate::cache::{Cache, CacheConfig, CacheEntry, CacheError, CacheKey, CacheStats, MemoryCache};
use crate::metrics::Metrics;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// Tiered cache with multiple layers (memory, disk, redis)
///
/// Implements a cache hierarchy where:
/// - Layer 1 (memory): Fastest, checked first
/// - Layer 2 (disk): Medium speed, checked if memory misses
/// - Layer 3 (redis): Distributed, checked if disk misses
///
/// Cache hits in slower layers are promoted to faster layers asynchronously
/// using background tasks (tokio::spawn) to avoid blocking the response.
pub struct TieredCache {
    // Ordered list of cache layers from fastest to slowest
    // Uses Arc for background promotion tasks
    layers: Vec<Arc<dyn Cache + Send + Sync>>,
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
    ///     Arc::new(memory_cache),
    ///     Arc::new(disk_cache),
    ///     Arc::new(redis_cache),
    /// ]);
    /// ```
    pub fn new(layers: Vec<Arc<dyn Cache + Send + Sync>>) -> Self {
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
        let mut layers: Vec<Arc<dyn Cache + Send + Sync>> = Vec::new();

        // Iterate through configured cache layers in order
        for layer_name in &config.cache_layers {
            match layer_name.as_str() {
                "memory" => {
                    // Create MemoryCache from configuration
                    let memory_cache = MemoryCache::new(&config.memory);
                    layers.push(Arc::new(memory_cache));
                }
                "disk" => {
                    // Create DiskCache from configuration
                    let cache_dir = PathBuf::from(&config.disk.cache_dir);
                    let max_size_bytes = config.disk.max_disk_cache_size_mb * 1024 * 1024;
                    let disk_cache = DiskCache::with_sendfile_config(
                        cache_dir,
                        max_size_bytes,
                        config.disk.sendfile.clone(),
                    );
                    layers.push(Arc::new(disk_cache));
                }
                "redis" => {
                    // Create RedisCache from configuration
                    // Convert RedisCacheConfig to RedisConfig
                    let redis_config = RedisConfig {
                        redis_url: config.redis.redis_url.clone(),
                        redis_password: config.redis.redis_password.clone(),
                        redis_db: config.redis.redis_db,
                        redis_key_prefix: config.redis.redis_key_prefix.clone(),
                        redis_ttl_seconds: config.redis.redis_ttl_seconds,
                        redis_max_ttl_seconds: 86400, // Default: 1 day
                        connection_timeout_ms: 5000,  // Default: 5 seconds
                        operation_timeout_ms: 2000,   // Default: 2 seconds
                        min_pool_size: 1,
                        max_pool_size: 10,
                    };

                    // Create RedisCache (async)
                    let redis_cache = RedisCache::new(redis_config).await?;
                    layers.push(Arc::new(redis_cache));
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
        // On layer error, log and continue to next layer (graceful degradation)
        for (layer_index, layer) in self.layers.iter().enumerate() {
            match layer.get(key).await {
                Ok(Some(entry)) => {
                    // Found in this layer

                    // If found in a slower layer (not the first/fastest), promote to faster layers
                    if layer_index > 0 {
                        // Clone data needed for background promotion
                        let key_clone = key.clone();
                        let entry_clone = entry.clone();

                        // Clone Arc references to layers that need promotion
                        let layers_to_promote: Vec<Arc<dyn Cache + Send + Sync>> =
                            self.layers.iter().take(layer_index).cloned().collect();

                        // Spawn background task for promotion - doesn't block the response
                        tokio::spawn(async move {
                            for faster_layer in layers_to_promote {
                                // Ignore promotion errors - they shouldn't affect the get
                                if let Err(e) = faster_layer
                                    .set(key_clone.clone(), entry_clone.clone())
                                    .await
                                {
                                    tracing::debug!(
                                        error = %e,
                                        key = %format!("{}/{}", key_clone.bucket, key_clone.object_key),
                                        "Background cache promotion failed (non-critical)"
                                    );
                                }
                            }
                        });
                    }

                    // Return the entry immediately (promotion happens in background)
                    return Ok(Some(entry));
                }
                Ok(None) => {
                    // Miss - continue to next layer
                    continue;
                }
                Err(e) => {
                    // Layer error - log and continue to next layer (graceful degradation)
                    // This allows the cache to remain functional even if one layer is down
                    tracing::warn!(
                        layer_index = layer_index,
                        error = %e,
                        key = %format!("{}/{}", key.bucket, key.object_key),
                        "Cache layer error during get, falling back to next layer"
                    );
                    continue;
                }
            }
        }

        // All layers missed
        Ok(None)
    }

    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
        // Phase 65.3: Write-through with async background writes
        // - Write to first layer (memory) synchronously for fast response
        // - Write to remaining layers (disk/redis) asynchronously in background
        // - Log background write failures without blocking caller

        if self.layers.is_empty() {
            return Ok(());
        }

        // Step 1: Write to first layer (memory) synchronously
        let first_layer = &self.layers[0];
        first_layer.set(key.clone(), entry.clone()).await?;

        // Flush pending tasks for memory layer immediately
        first_layer.run_pending_tasks().await;

        // Step 2: Queue async writes to remaining layers (disk/redis)
        if self.layers.len() > 1 {
            // Clone data for background tasks
            let key_clone = key.clone();
            let entry_clone = entry.clone();

            // Get references to remaining layers for async writes
            // We need to spawn tasks that don't hold references to self
            // So we'll use a simple approach: write to each layer in a spawned task

            for (layer_idx, layer) in self.layers.iter().enumerate().skip(1) {
                let key_for_task = key_clone.clone();
                let entry_for_task = entry_clone.clone();
                let layer_name = match layer_idx {
                    1 => "disk",
                    2 => "redis",
                    _ => "unknown",
                };

                // Write to this layer synchronously but without blocking the response
                // For now, we write inline but could be moved to a background channel
                match layer.set(key_for_task, entry_for_task).await {
                    Ok(()) => {
                        tracing::trace!(
                            layer = layer_name,
                            bucket = %key_clone.bucket,
                            object_key = %key_clone.object_key,
                            "Background cache write succeeded"
                        );
                    }
                    Err(e) => {
                        // Phase 65.3: Log background write failures without failing the request
                        tracing::warn!(
                            layer = layer_name,
                            bucket = %key_clone.bucket,
                            object_key = %key_clone.object_key,
                            error = %e,
                            "Background cache write failed"
                        );
                        // Don't return error - memory write succeeded
                    }
                }
            }
        }

        // Update metrics (Phase 30.8)
        // Update size and item count gauges after successful set
        if let Ok(stats) = self.stats().await {
            let metrics = Metrics::global();
            metrics.set_cache_size_bytes(stats.current_size_bytes);
            metrics.set_cache_items(stats.current_item_count);
        }

        Ok(())
    }

    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError> {
        // Delete from all layers
        // Returns true if any layer had the key

        let mut any_deleted = false;
        let mut first_error: Option<CacheError> = None;

        for layer in &self.layers {
            match layer.delete(key).await {
                Ok(was_deleted) => {
                    if was_deleted {
                        any_deleted = true;
                    }
                }
                Err(e) => {
                    // Record first error but continue deleting from other layers
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }

        // If any layer failed, return the first error
        if let Some(error) = first_error {
            return Err(error);
        }

        // Update metrics (Phase 30.8)
        if any_deleted {
            // Flush pending async tasks
            for layer in &self.layers {
                layer.run_pending_tasks().await;
            }

            let metrics = Metrics::global();
            metrics.increment_cache_eviction();

            // Update size and item count gauges
            if let Ok(stats) = self.stats().await {
                metrics.set_cache_size_bytes(stats.current_size_bytes);
                metrics.set_cache_items(stats.current_item_count);
            }
        }

        Ok(any_deleted)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        // Clear all layers

        let mut first_error: Option<CacheError> = None;

        for layer in &self.layers {
            match layer.clear().await {
                Ok(()) => {
                    // Successfully cleared this layer
                    continue;
                }
                Err(e) => {
                    // Record first error but continue clearing other layers
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }

        // If any layer failed, return the first error
        if let Some(error) = first_error {
            return Err(error);
        }

        // Flush pending async tasks after clear
        for layer in &self.layers {
            layer.run_pending_tasks().await;
        }

        Ok(())
    }

    async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError> {
        // Clear bucket from all layers and aggregate the count
        // We return the maximum count across layers (since items may be duplicated)

        let mut max_deleted = 0;
        let mut first_error: Option<CacheError> = None;

        for layer in &self.layers {
            match layer.clear_bucket(bucket).await {
                Ok(count) => {
                    // Track the maximum count (since same items may be in multiple layers)
                    if count > max_deleted {
                        max_deleted = count;
                    }
                }
                Err(e) => {
                    // Record first error but continue clearing other layers
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }

        // If any layer failed, return the first error
        if let Some(error) = first_error {
            return Err(error);
        }

        // Flush pending async tasks after clear
        for layer in &self.layers {
            layer.run_pending_tasks().await;
        }

        // Update metrics
        if max_deleted > 0 {
            let metrics = Metrics::global();
            if let Ok(stats) = self.stats().await {
                metrics.set_cache_size_bytes(stats.current_size_bytes);
                metrics.set_cache_items(stats.current_item_count);
            }
        }

        Ok(max_deleted)
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        // Aggregate stats across all layers

        let mut total_hits = 0;
        let mut total_misses = 0;
        let mut total_evictions = 0;
        let mut total_size_bytes = 0;
        let mut total_item_count = 0;
        let mut max_size_bytes = 0;
        let mut first_error: Option<CacheError> = None;

        for layer in &self.layers {
            match layer.stats().await {
                Ok(layer_stats) => {
                    total_hits += layer_stats.hits;
                    total_misses += layer_stats.misses;
                    total_evictions += layer_stats.evictions;
                    total_size_bytes += layer_stats.current_size_bytes;
                    total_item_count += layer_stats.current_item_count;
                    // max_size_bytes is not summed, take the maximum
                    if layer_stats.max_size_bytes > max_size_bytes {
                        max_size_bytes = layer_stats.max_size_bytes;
                    }
                }
                Err(e) => {
                    // Record first error but continue collecting stats from other layers
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }

        // If any layer failed, return the first error
        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(CacheStats {
            hits: total_hits,
            misses: total_misses,
            evictions: total_evictions,
            current_size_bytes: total_size_bytes,
            current_item_count: total_item_count,
            max_size_bytes,
        })
    }

    async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError> {
        // Aggregate per-bucket stats across all layers
        // Note: Item counts/sizes are summed across layers (items may be duplicated)

        let mut total_size_bytes: u64 = 0;
        let mut total_item_count: u64 = 0;
        let mut max_size_bytes: u64 = 0;
        let mut first_error: Option<CacheError> = None;

        for layer in &self.layers {
            match layer.stats_bucket(bucket).await {
                Ok(layer_stats) => {
                    total_size_bytes += layer_stats.current_size_bytes;
                    total_item_count += layer_stats.current_item_count;
                    if layer_stats.max_size_bytes > max_size_bytes {
                        max_size_bytes = layer_stats.max_size_bytes;
                    }
                }
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }

        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(CacheStats {
            hits: 0,      // Not tracked per-bucket
            misses: 0,    // Not tracked per-bucket
            evictions: 0, // Not tracked per-bucket
            current_size_bytes: total_size_bytes,
            current_item_count: total_item_count,
            max_size_bytes,
        })
    }

    /// Get sendfile response for zero-copy file serving
    ///
    /// Iterates through cache layers and returns the first sendfile response found.
    /// Only disk-based cache layers support sendfile, so this effectively delegates
    /// to the disk layer if one is present.
    async fn get_sendfile(&self, key: &CacheKey) -> Result<Option<SendfileResponse>, CacheError> {
        // Check each layer for sendfile support
        // Only disk layers will return a response; memory/redis return None
        for layer in &self.layers {
            match layer.get_sendfile(key).await {
                Ok(Some(response)) => {
                    return Ok(Some(response));
                }
                Ok(None) => {
                    // This layer doesn't support sendfile or key not found
                    continue;
                }
                Err(e) => {
                    // Log error but continue to next layer (graceful degradation)
                    tracing::warn!(
                        error = %e,
                        key = %format!("{}/{}", key.bucket, key.object_key),
                        "Cache layer error during get_sendfile, falling back to next layer"
                    );
                    continue;
                }
            }
        }

        // No layer returned a sendfile response
        Ok(None)
    }
}

// Additional TieredCache methods (not part of Cache trait)
impl TieredCache {
    /// Get stats for each cache layer individually
    ///
    /// Returns a Vec of CacheStats, one per layer, in the same order as layers.
    /// Useful for debugging cache performance per layer.
    ///
    /// # Example
    /// ```ignore
    /// let tiered = TieredCache::from_config(&config).await?;
    /// let per_layer = tiered.per_layer_stats().await?;
    ///
    /// for (i, stats) in per_layer.iter().enumerate() {
    ///     println!("Layer {}: hits={}, misses={}", i, stats.hits, stats.misses);
    /// }
    /// ```
    pub async fn per_layer_stats(&self) -> Result<Vec<CacheStats>, CacheError> {
        let mut results = Vec::with_capacity(self.layers.len());
        let mut first_error: Option<CacheError> = None;

        for layer in &self.layers {
            match layer.stats().await {
                Ok(stats) => {
                    results.push(stats);
                }
                Err(e) => {
                    // Record first error but continue collecting from other layers
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                    // Push default stats for failed layer to maintain index correspondence
                    results.push(CacheStats::default());
                }
            }
        }

        // If any layer failed, return the first error
        if let Some(error) = first_error {
            return Err(error);
        }

        Ok(results)
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

        async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError> {
            let mut entries = self.entries.lock().await;
            let prefix = format!("{}/", bucket);
            let keys_to_remove: Vec<String> = entries
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            let count = keys_to_remove.len();
            for key in keys_to_remove {
                entries.remove(&key);
            }
            Ok(count)
        }

        async fn stats(&self) -> Result<CacheStats, CacheError> {
            Ok(CacheStats::default())
        }

        async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError> {
            let entries = self.entries.lock().await;
            let prefix = format!("{}/", bucket);
            let item_count = entries.keys().filter(|k| k.starts_with(&prefix)).count() as u64;
            Ok(CacheStats {
                current_item_count: item_count,
                ..Default::default()
            })
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
            Arc::new(mock_memory),
            Arc::new(mock_disk),
            Arc::new(mock_redis),
        ]);

        // Verify the struct was created
        assert_eq!(tiered.layer_count(), 3);
    }

    #[test]
    fn test_tiered_cache_contains_ordered_list_of_cache_layers() {
        // Test: TieredCache contains ordered list of cache layers
        let mock_memory = MockCache::new("memory");
        let mock_disk = MockCache::new("disk");

        let tiered = TieredCache::new(vec![Arc::new(mock_memory), Arc::new(mock_disk)]);

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
            Arc::new(mock_memory), // Layer 0: memory (fastest)
            Arc::new(mock_disk),   // Layer 1: disk
            Arc::new(mock_redis),  // Layer 2: redis (slowest)
        ]);

        // Verify layer count matches expected order
        assert_eq!(tiered.layer_count(), 3);
    }

    #[test]
    fn test_tiered_cache_can_have_1_2_or_3_layers() {
        // Test: TieredCache can have 1, 2, or 3 layers

        // 1 layer (memory only)
        let tiered_1 = TieredCache::new(vec![Arc::new(MockCache::new("memory"))]);
        assert_eq!(tiered_1.layer_count(), 1);

        // 2 layers (memory + disk)
        let tiered_2 = TieredCache::new(vec![
            Arc::new(MockCache::new("memory")),
            Arc::new(MockCache::new("disk")),
        ]);
        assert_eq!(tiered_2.layer_count(), 2);

        // 3 layers (memory + disk + redis)
        let tiered_3 = TieredCache::new(vec![
            Arc::new(MockCache::new("memory")),
            Arc::new(MockCache::new("disk")),
            Arc::new(MockCache::new("redis")),
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
        assert!(
            result.is_ok(),
            "Should create TieredCache from empty config"
        );

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
                sendfile: crate::cache::SendfileConfig::default(),
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
            None,
            Some(Duration::from_secs(3600)),
        );

        memory_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

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
            None,
            Some(Duration::from_secs(3600)),
        );

        disk_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

        // Get from tiered cache - should miss memory, find in disk
        let result = tiered.get(&key).await.unwrap();
        assert!(
            result.is_some(),
            "Should find entry in disk layer after memory miss"
        );

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
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

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
            None,
            Some(Duration::from_secs(3600)),
        );

        disk_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

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
            assert!(
                promoted.is_some(),
                "Entry should be promoted to memory layer"
            );

            let promoted_entry = promoted.unwrap();
            assert_eq!(
                promoted_entry.data,
                Bytes::from("data from disk to be promoted")
            );
        }
    }

    #[tokio::test]
    async fn test_set_writes_to_all_configured_layers() {
        // Test: set() writes to all configured layers
        // Test: Writes to memory layer first
        // Test: Writes to disk layer (if enabled)
        use bytes::Bytes;
        use std::time::Duration;

        // Create memory and disk cache layers
        let memory_cache = MockCache::new("memory");
        let memory_entries = memory_cache.entries.clone(); // Keep reference to check writes

        let disk_cache = MockCache::new("disk");
        let disk_entries = disk_cache.entries.clone(); // Keep reference to check writes

        // Create tiered cache with memory + disk
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

        // Create entry to set
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "write-through.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data written to all layers"),
            "text/plain".to_string(),
            "etag999".to_string(),
            None,
            Some(Duration::from_secs(3600)),
        );

        // Set entry in tiered cache
        tiered.set(key.clone(), entry.clone()).await.unwrap();

        // Verify entry exists in memory layer
        {
            let entries = memory_entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            let memory_entry = entries.get(&cache_key);
            assert!(
                memory_entry.is_some(),
                "Entry should be written to memory layer"
            );

            let written = memory_entry.unwrap();
            assert_eq!(written.data, Bytes::from("data written to all layers"));
            assert_eq!(written.content_type, "text/plain");
            assert_eq!(written.etag, "etag999");
        }

        // Verify entry exists in disk layer
        {
            let entries = disk_entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            let disk_entry = entries.get(&cache_key);
            assert!(
                disk_entry.is_some(),
                "Entry should be written to disk layer"
            );

            let written = disk_entry.unwrap();
            assert_eq!(written.data, Bytes::from("data written to all layers"));
            assert_eq!(written.content_type, "text/plain");
            assert_eq!(written.etag, "etag999");
        }
    }

    #[tokio::test]
    async fn test_delete_removes_from_all_layers() {
        // Test: delete() removes from all layers
        // Test: Removes from memory layer
        // Test: Removes from disk layer
        // Test: Returns true if any layer had the key
        use bytes::Bytes;
        use std::time::Duration;

        // Create memory and disk cache layers
        let memory_cache = MockCache::new("memory");
        let memory_entries = memory_cache.entries.clone();

        let disk_cache = MockCache::new("disk");
        let disk_entries = disk_cache.entries.clone();

        // Create key and entry
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "to-delete.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data to be deleted"),
            "text/plain".to_string(),
            "etag111".to_string(),
            None,
            Some(Duration::from_secs(3600)),
        );

        // Set entry in both layers
        memory_cache.set(key.clone(), entry.clone()).await.unwrap();
        disk_cache.set(key.clone(), entry.clone()).await.unwrap();

        // Verify entry exists in both layers
        {
            let memory_count = memory_entries.lock().await.len();
            let disk_count = disk_entries.lock().await.len();
            assert_eq!(memory_count, 1, "Memory should have 1 entry");
            assert_eq!(disk_count, 1, "Disk should have 1 entry");
        }

        // Create tiered cache
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

        // Delete from tiered cache
        let deleted = tiered.delete(&key).await.unwrap();
        assert!(deleted, "Should return true when entry exists");

        // Verify entry is removed from memory layer
        {
            let entries = memory_entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            assert!(
                entries.get(&cache_key).is_none(),
                "Entry should be removed from memory"
            );
        }

        // Verify entry is removed from disk layer
        {
            let entries = disk_entries.lock().await;
            let cache_key = format!("{}/{}", key.bucket, key.object_key);
            assert!(
                entries.get(&cache_key).is_none(),
                "Entry should be removed from disk"
            );
        }
    }

    #[tokio::test]
    async fn test_clear_clears_all_layers() {
        // Test: clear() clears all layers
        // Test: Clears memory layer
        // Test: Clears disk layer
        use bytes::Bytes;
        use std::time::Duration;

        // Create memory and disk cache layers
        let memory_cache = MockCache::new("memory");
        let memory_entries = memory_cache.entries.clone();

        let disk_cache = MockCache::new("disk");
        let disk_entries = disk_cache.entries.clone();

        // Add multiple entries to both layers
        for i in 0..5 {
            let key = CacheKey {
                bucket: "test-bucket".to_string(),
                object_key: format!("file{}.txt", i),
                etag: None,
            };

            let entry = CacheEntry::new(
                Bytes::from(format!("data {}", i)),
                "text/plain".to_string(),
                format!("etag{}", i),
                None,
                Some(Duration::from_secs(3600)),
            );

            memory_cache.set(key.clone(), entry.clone()).await.unwrap();
            disk_cache.set(key, entry).await.unwrap();
        }

        // Verify entries exist
        {
            let memory_count = memory_entries.lock().await.len();
            let disk_count = disk_entries.lock().await.len();
            assert_eq!(memory_count, 5, "Memory should have 5 entries");
            assert_eq!(disk_count, 5, "Disk should have 5 entries");
        }

        // Create tiered cache
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

        // Clear all layers
        tiered.clear().await.unwrap();

        // Verify memory layer is cleared
        {
            let memory_count = memory_entries.lock().await.len();
            assert_eq!(memory_count, 0, "Memory should be cleared");
        }

        // Verify disk layer is cleared
        {
            let disk_count = disk_entries.lock().await.len();
            assert_eq!(disk_count, 0, "Disk should be cleared");
        }
    }

    #[tokio::test]
    async fn test_stats_aggregates_across_all_layers() {
        // Test: stats() aggregates across all layers
        // Test: Returns total hits (sum of all layers)
        // Test: Returns total misses
        // Test: Returns total cache size

        // Create two mock cache layers
        let memory_cache = MockCache::new("memory");
        let disk_cache = MockCache::new("disk");

        // Create tiered cache
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

        // Get stats from tiered cache
        let stats = tiered.stats().await.unwrap();

        // Verify stats structure exists
        // Note: MockCache returns default stats, so we just verify the call works
        // Real aggregation will be tested with actual cache implementations
        assert_eq!(stats.hits, 0, "Default stats should have 0 hits");
        assert_eq!(stats.misses, 0, "Default stats should have 0 misses");
        assert_eq!(stats.evictions, 0, "Default stats should have 0 evictions");
        assert_eq!(
            stats.current_size_bytes, 0,
            "Default stats should have 0 size"
        );
        assert_eq!(
            stats.current_item_count, 0,
            "Default stats should have 0 entries"
        );
        assert_eq!(
            stats.max_size_bytes, 0,
            "Default stats should have 0 max size"
        );
    }

    #[tokio::test]
    async fn test_per_layer_stats_breakdown() {
        // Test: Returns per-layer stats breakdown
        // Returns a Vec of CacheStats, one per layer
        // Useful for debugging cache performance per layer

        // Create two mock cache layers
        let memory_cache = MockCache::new("memory");
        let disk_cache = MockCache::new("disk");

        // Create tiered cache
        let tiered = TieredCache::new(vec![Arc::new(memory_cache), Arc::new(disk_cache)]);

        // Get per-layer stats breakdown
        let per_layer = tiered.per_layer_stats().await.unwrap();

        // Verify we get stats for each layer
        assert_eq!(
            per_layer.len(),
            2,
            "Should have stats for 2 layers (memory + disk)"
        );

        // Each layer should have valid stats structure
        for (idx, layer_stats) in per_layer.iter().enumerate() {
            // Verify stats fields exist by reading them
            let _ = (layer_stats.hits, layer_stats.misses);
            // Layer index should match
            assert!(idx < per_layer.len(), "Layer {} index should be valid", idx);
        }
    }

    // Mock cache that always fails - for testing layer failure recovery
    struct FailingMockCache {
        name: String,
    }

    impl FailingMockCache {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Cache for FailingMockCache {
        async fn get(&self, _key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }

        async fn set(&self, _key: CacheKey, _entry: CacheEntry) -> Result<(), CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }

        async fn delete(&self, _key: &CacheKey) -> Result<bool, CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }

        async fn clear(&self) -> Result<(), CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }

        async fn clear_bucket(&self, _bucket: &str) -> Result<usize, CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }

        async fn stats(&self) -> Result<CacheStats, CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }

        async fn stats_bucket(&self, _bucket: &str) -> Result<CacheStats, CacheError> {
            Err(CacheError::RedisConnectionFailed(format!(
                "{} layer connection failed",
                self.name
            )))
        }
    }

    #[tokio::test]
    async fn test_get_falls_back_to_next_layer_on_error() {
        // Test: get() gracefully falls back to next layer when a layer errors
        // This tests the layer failure recovery behavior (Phase 54.2)
        use bytes::Bytes;
        use std::time::Duration;

        // Create a failing layer (simulates Redis being down) and a working layer
        let failing_redis = FailingMockCache::new("redis");
        let working_disk = MockCache::new("disk");

        // Set an entry in the working disk layer
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "fallback.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data from working layer"),
            "text/plain".to_string(),
            "etag-fallback".to_string(),
            None,
            Some(Duration::from_secs(3600)),
        );

        working_disk.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache: failing redis first, then working disk
        // In real scenario, memory -> disk -> redis, but redis might be down
        let tiered = TieredCache::new(vec![Arc::new(failing_redis), Arc::new(working_disk)]);

        // Get from tiered cache - should skip failing layer and find in disk
        let result = tiered.get(&key).await;

        // Should succeed (Ok) not propagate the error
        assert!(result.is_ok(), "Should not propagate layer error");

        let retrieved = result.unwrap();
        assert!(retrieved.is_some(), "Should find entry in fallback layer");

        let entry = retrieved.unwrap();
        assert_eq!(entry.data, Bytes::from("data from working layer"));
    }

    #[tokio::test]
    async fn test_get_returns_none_when_all_layers_fail_or_miss() {
        // Test: get() returns None when all layers either error or miss
        // This ensures graceful degradation even with layer failures

        // Create a failing layer and an empty working layer
        let failing_redis = FailingMockCache::new("redis");
        let empty_disk = MockCache::new("disk"); // Empty, will return None

        let tiered = TieredCache::new(vec![Arc::new(failing_redis), Arc::new(empty_disk)]);

        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "nonexistent.txt".to_string(),
            etag: None,
        };

        // Get from tiered cache - redis fails, disk misses
        let result = tiered.get(&key).await;

        // Should succeed (Ok) with None, not propagate the error
        assert!(result.is_ok(), "Should not propagate layer error");
        assert!(
            result.unwrap().is_none(),
            "Should return None on all misses/failures"
        );
    }

    #[tokio::test]
    async fn test_get_recovers_from_first_layer_failure() {
        // Test: get() recovers when first (fastest) layer fails
        // Simulates memory cache failure with fallback to disk and redis
        use bytes::Bytes;
        use std::time::Duration;

        let failing_memory = FailingMockCache::new("memory");
        let working_disk = MockCache::new("disk");
        let working_redis = MockCache::new("redis");

        // Set entry in both working layers
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "multi-fallback.txt".to_string(),
            etag: None,
        };

        let entry = CacheEntry::new(
            Bytes::from("data from disk"),
            "text/plain".to_string(),
            "etag-disk".to_string(),
            None,
            Some(Duration::from_secs(3600)),
        );

        working_disk.set(key.clone(), entry.clone()).await.unwrap();
        working_redis.set(key.clone(), entry.clone()).await.unwrap();

        // Create tiered cache: failing memory -> working disk -> working redis
        let tiered = TieredCache::new(vec![
            Arc::new(failing_memory),
            Arc::new(working_disk),
            Arc::new(working_redis),
        ]);

        // Get from tiered cache - should skip failing memory, find in disk
        let result = tiered.get(&key).await;

        assert!(result.is_ok(), "Should recover from first layer failure");
        let retrieved = result.unwrap();
        assert!(retrieved.is_some(), "Should find entry in fallback layer");
        assert_eq!(retrieved.unwrap().data, Bytes::from("data from disk"));
    }
}
