// Redis cache implementation
//
// Provides distributed caching using Redis with production-ready error handling.

use crate::cache::{CacheEntry, CacheError, CacheKey, CacheStats};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::config::RedisConfig;
use super::key;
use super::metrics::RedisCacheMetrics;
use super::serialization;

/// Statistics tracker for Redis cache operations
#[derive(Debug)]
pub struct RedisCacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
    sets: AtomicU64,
    evictions: AtomicU64,
    errors: AtomicU64,
}

impl RedisCacheStats {
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            sets: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }

    pub fn increment_hits(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_misses(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_sets(&self) {
        self.sets.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_evictions(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            current_size_bytes: 0, // Redis doesn't track size locally
            current_item_count: 0, // Would need DBSIZE call
            max_size_bytes: 0,     // Not applicable to Redis
        }
    }
}

impl Default for RedisCacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Redis-based distributed cache implementation
///
/// Provides production-ready distributed caching with:
/// - Async connection multiplexing via ConnectionManager
/// - MessagePack serialization for efficient storage
/// - Configurable TTL and connection pooling
/// - Comprehensive error handling and retry logic
pub struct RedisCache {
    /// Redis connection manager (async, multiplexed)
    connection: ConnectionManager,

    /// Redis configuration
    config: RedisConfig,

    /// Cache statistics tracker
    stats: Arc<RedisCacheStats>,

    /// Key prefix for all cache entries
    key_prefix: String,
}

impl RedisCache {
    /// Builds a Redis connection URL with authentication and database selection
    ///
    /// Takes the base URL and incorporates password and database number from config.
    /// URL format: redis://[:password@]host[:port][/db]
    ///
    /// # Examples
    ///
    /// - Base URL: `redis://localhost:6379`, password: `secret`, db: `1`
    ///   Result: `redis://:secret@localhost:6379/1`
    fn build_connection_url(base_url: &str, config: &RedisConfig) -> Result<String, CacheError> {
        // Validate scheme first (redis:// or rediss://)
        let (scheme, rest) = if base_url.starts_with("rediss://") {
            ("rediss://", base_url.strip_prefix("rediss://").unwrap())
        } else if base_url.starts_with("redis://") {
            ("redis://", base_url.strip_prefix("redis://").unwrap())
        } else {
            return Err(CacheError::ConfigurationError(format!(
                "Invalid Redis URL scheme. Expected 'redis://' or 'rediss://', got: {}",
                base_url
            )));
        };

        // If no password and db is 0, just return the base URL as-is
        if config.redis_password.is_none() && config.redis_db == 0 {
            return Ok(base_url.to_string());
        }

        // Extract host:port (ignore any existing path/db in the URL)
        let host_port = rest.split('/').next().unwrap_or(rest);

        // Build URL with optional password and database
        let mut url = scheme.to_string();

        // Add password if provided
        if let Some(ref password) = config.redis_password {
            // URL-encode the password to handle special characters
            let encoded_password = urlencoding::encode(password);
            url.push_str(&format!(":{}@", encoded_password));
        }

        // Add host and port
        url.push_str(host_port);

        // Add database number if non-zero
        if config.redis_db > 0 {
            url.push_str(&format!("/{}", config.redis_db));
        }

        Ok(url)
    }

    /// Creates a new RedisCache instance
    ///
    /// # Arguments
    ///
    /// * `config` - Redis configuration with connection details
    ///
    /// # Returns
    ///
    /// Returns a Result with the RedisCache instance or CacheError
    ///
    /// # Errors
    ///
    /// Returns CacheError::RedisConnectionFailed if:
    /// - Redis URL is invalid
    /// - Cannot connect to Redis server
    /// - Authentication fails
    /// - Cannot select database
    pub async fn new(config: RedisConfig) -> Result<Self, CacheError> {
        // Validate that redis_url is provided
        let redis_url = config
            .redis_url
            .as_ref()
            .ok_or_else(|| CacheError::ConfigurationError("redis_url is required".to_string()))?;

        // Build connection URL with authentication if password is provided
        let connection_url = Self::build_connection_url(redis_url, &config)?;

        // Create Redis client
        let client = Client::open(connection_url.as_str())
            .map_err(|e| CacheError::RedisConnectionFailed(format!("Invalid Redis URL: {}", e)))?;

        // Create connection manager (handles connection pooling and reconnection)
        let connection = ConnectionManager::new(client).await.map_err(|e| {
            CacheError::RedisConnectionFailed(format!("Failed to connect to Redis: {}", e))
        })?;

        let key_prefix = config.redis_key_prefix.clone();
        let stats = Arc::new(RedisCacheStats::new());

        Ok(Self {
            connection,
            config,
            stats,
            key_prefix,
        })
    }

    /// Checks if Redis connection is healthy
    ///
    /// Sends a PING command to verify Redis is responsive
    pub async fn health_check(&self) -> bool {
        // Clone the connection for the PING command
        let mut conn = self.connection.clone();

        // Try to PING Redis - returns "PONG" on success
        match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
            Ok(response) => response == "PONG",
            Err(_) => false,
        }
    }

    /// Retrieves an entry from the Redis cache
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(CacheEntry)` if the key exists, `None` if not found
    ///
    /// # Errors
    ///
    /// Returns `CacheError` on Redis connection errors or deserialization failures
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        // Start timing the operation
        let _timer = RedisCacheMetrics::global().start_operation_timer("get");

        // Format Redis key
        let redis_key = key::format_key(&self.key_prefix, &key.bucket, &key.object_key);

        // Validate the key
        if let Err(e) = key::validate_key(&redis_key) {
            return Err(CacheError::ConfigurationError(e));
        }

        // Clone connection for async operation
        let mut conn = self.connection.clone();

        // Get bytes from Redis using GET command
        let bytes: Option<Vec<u8>> = conn.get(&redis_key).await.map_err(|e| {
            RedisCacheMetrics::global().errors.inc();
            CacheError::RedisError(format!("Redis GET failed: {}", e))
        })?;

        match bytes {
            Some(data) => {
                // Deserialize the entry
                let deserialize_result = {
                    let _timer =
                        RedisCacheMetrics::global().start_serialization_timer("deserialize");
                    serialization::deserialize_entry(&data)
                };

                match deserialize_result {
                    Ok(entry) => {
                        // Double-check entry hasn't expired locally (clock skew protection)
                        if entry.expires_at <= std::time::SystemTime::now() {
                            // Entry expired locally - treat as miss and delete from Redis
                            tracing::debug!(
                                "Entry expired locally for key '{}', deleting from Redis",
                                redis_key
                            );
                            self.stats.increment_misses();
                            RedisCacheMetrics::global().misses.inc();

                            // Asynchronously delete the expired entry
                            let mut delete_conn = self.connection.clone();
                            let delete_key = redis_key.clone();
                            tokio::spawn(async move {
                                let _ = delete_conn.del::<_, ()>(&delete_key).await;
                            });

                            return Ok(None);
                        }

                        // Increment hit counter
                        self.stats.increment_hits();
                        RedisCacheMetrics::global().hits.inc();
                        Ok(Some(entry))
                    }
                    Err(e) => {
                        // Increment error counter on deserialization failure
                        self.stats.increment_errors();
                        RedisCacheMetrics::global().errors.inc();
                        // Treat deserialization errors as cache miss (return None)
                        // Log the error but don't fail the operation
                        tracing::warn!(
                            "Failed to deserialize cache entry for key '{}': {}",
                            redis_key,
                            e
                        );
                        self.stats.increment_misses();
                        RedisCacheMetrics::global().misses.inc();
                        Ok(None)
                    }
                }
            }
            None => {
                // Key not found - increment miss counter
                self.stats.increment_misses();
                RedisCacheMetrics::global().misses.inc();
                Ok(None)
            }
        }
    }

    /// Stores an entry in the Redis cache
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key
    /// * `entry` - The cache entry to store
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success
    ///
    /// # Errors
    ///
    /// Returns `CacheError` on Redis connection errors or serialization failures
    pub async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
        // Start timing the operation
        let _timer = RedisCacheMetrics::global().start_operation_timer("set");

        // Format Redis key
        let redis_key = key::format_key(&self.key_prefix, &key.bucket, &key.object_key);

        // Validate the key
        if let Err(e) = key::validate_key(&redis_key) {
            return Err(CacheError::ConfigurationError(e));
        }

        // Serialize the entry with timing
        let bytes = {
            let _timer = RedisCacheMetrics::global().start_serialization_timer("serialize");
            serialization::serialize_entry(&entry)?
        };

        // Clone connection for async operation
        let mut conn = self.connection.clone();

        // Calculate TTL from entry.expires_at
        let ttl_secs = match entry
            .expires_at
            .duration_since(std::time::SystemTime::now())
        {
            Ok(remaining) => {
                let secs = remaining.as_secs();
                // Apply minimum TTL: 1 second (don't set 0 or negative)
                // Apply maximum TTL: configurable (default: 86400 = 1 day)
                secs.max(1).min(self.config.redis_max_ttl_seconds)
            }
            Err(_) => {
                // Entry already expired or clock skew
                // Use minimum TTL (1 second) to allow immediate expiration
                1
            }
        };

        // Use SETEX to set with TTL
        conn.set_ex::<_, _, ()>(&redis_key, bytes, ttl_secs)
            .await
            .map_err(|e| {
                RedisCacheMetrics::global().errors.inc();
                CacheError::RedisError(format!("Redis SETEX failed: {}", e))
            })?;

        // Increment set counter
        self.stats.increment_sets();
        RedisCacheMetrics::global().sets.inc();

        Ok(())
    }

    /// Deletes an entry from the Redis cache
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, whether or not the key existed (idempotent)
    ///
    /// # Errors
    ///
    /// Returns `CacheError` on Redis connection errors
    pub async fn delete(&self, key: &CacheKey) -> Result<(), CacheError> {
        // Start timing the operation
        let _timer = RedisCacheMetrics::global().start_operation_timer("delete");

        // Format Redis key
        let redis_key = key::format_key(&self.key_prefix, &key.bucket, &key.object_key);

        // Validate the key
        if let Err(e) = key::validate_key(&redis_key) {
            return Err(CacheError::ConfigurationError(e));
        }

        // Clone connection for async operation
        let mut conn = self.connection.clone();

        // Use DEL command to remove the key
        // DEL returns the number of keys deleted (0 or 1), but we don't care about the return value
        // because delete is idempotent - we succeed whether the key existed or not
        conn.del::<_, ()>(&redis_key).await.map_err(|e| {
            RedisCacheMetrics::global().errors.inc();
            CacheError::RedisError(format!("Redis DEL failed: {}", e))
        })?;

        // Increment eviction counter
        self.stats.increment_evictions();
        RedisCacheMetrics::global().evictions.inc();

        Ok(())
    }

    /// Clears all entries from the Redis cache with the configured prefix
    ///
    /// Uses Redis SCAN for safe iteration (non-blocking) and deletes keys in batches.
    /// This operation is safe for production use and won't block the Redis server.
    ///
    /// # Returns
    ///
    /// Returns the number of keys deleted
    ///
    /// # Errors
    ///
    /// Returns `CacheError` on Redis connection errors
    pub async fn clear(&self) -> Result<usize, CacheError> {
        // Start timing the operation
        let _timer = RedisCacheMetrics::global().start_operation_timer("clear");

        let mut conn = self.connection.clone();
        let mut cursor: u64 = 0;
        let mut total_deleted = 0;
        let batch_size = 100;

        // Pattern to match all keys with our prefix
        let pattern = format!("{}:*", self.key_prefix);

        loop {
            // Use SCAN with pattern matching
            // SCAN cursor MATCH pattern COUNT batch_size
            let scan_result: (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(batch_size)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    RedisCacheMetrics::global().errors.inc();
                    CacheError::RedisError(format!("Redis SCAN failed: {}", e))
                })?;

            cursor = scan_result.0;
            let keys = scan_result.1;

            // Delete the batch of keys if any were found
            if !keys.is_empty() {
                let deleted: usize = conn.del(&keys).await.map_err(|e| {
                    RedisCacheMetrics::global().errors.inc();
                    CacheError::RedisError(format!("Redis DEL failed: {}", e))
                })?;

                total_deleted += deleted;

                // Update eviction counter for each deleted key
                for _ in 0..deleted {
                    self.stats.increment_evictions();
                    RedisCacheMetrics::global().evictions.inc();
                }
            }

            // SCAN returns 0 when iteration is complete
            if cursor == 0 {
                break;
            }
        }

        Ok(total_deleted)
    }

    /// Clears all entries for a specific bucket from the Redis cache
    ///
    /// Uses Redis SCAN for safe iteration (non-blocking) and deletes keys in batches.
    /// This operation is safe for production use and won't block the Redis server.
    ///
    /// # Arguments
    ///
    /// * `bucket` - The bucket name to clear
    ///
    /// # Returns
    ///
    /// Returns the number of keys deleted
    ///
    /// # Errors
    ///
    /// Returns `CacheError` on Redis connection errors
    ///
    /// # Note
    ///
    /// This method only clears keys matching the pattern `{prefix}:{bucket}:*`.
    /// Keys that were hashed due to length (format: `{prefix}:hash:{sha256}`)
    /// are not cleared as the bucket information is embedded in the hash.
    pub async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError> {
        // Start timing the operation
        let _timer = RedisCacheMetrics::global().start_operation_timer("clear_bucket");

        let mut conn = self.connection.clone();
        let mut cursor: u64 = 0;
        let mut total_deleted = 0;
        let batch_size = 100;

        // Pattern to match all keys for this bucket
        // Format: {prefix}:{bucket}:*
        let pattern = format!("{}:{}:*", self.key_prefix, bucket);

        loop {
            // Use SCAN with pattern matching
            // SCAN cursor MATCH pattern COUNT batch_size
            let scan_result: (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(batch_size)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    RedisCacheMetrics::global().errors.inc();
                    CacheError::RedisError(format!("Redis SCAN failed: {}", e))
                })?;

            cursor = scan_result.0;
            let keys = scan_result.1;

            // Delete the batch of keys if any were found
            if !keys.is_empty() {
                let deleted: usize = conn.del(&keys).await.map_err(|e| {
                    RedisCacheMetrics::global().errors.inc();
                    CacheError::RedisError(format!("Redis DEL failed: {}", e))
                })?;

                total_deleted += deleted;

                // Update eviction counter for each deleted key
                for _ in 0..deleted {
                    self.stats.increment_evictions();
                    RedisCacheMetrics::global().evictions.inc();
                }
            }

            // SCAN returns 0 when iteration is complete
            if cursor == 0 {
                break;
            }
        }

        Ok(total_deleted)
    }

    /// Returns a snapshot of current cache statistics
    ///
    /// # Returns
    ///
    /// Returns a `CacheStats` struct with current statistics:
    /// - hits: Number of successful cache retrievals
    /// - misses: Number of cache misses
    /// - evictions: Number of deleted keys
    /// - current_size_bytes: 0 (Redis doesn't track size locally)
    /// - current_item_count: 0 (would need DBSIZE call)
    /// - max_size_bytes: 0 (not applicable to Redis)
    pub fn stats(&self) -> CacheStats {
        self.stats.snapshot()
    }

    /// Returns statistics for a specific bucket
    ///
    /// Uses Redis SCAN to count entries for the bucket.
    /// Note: This is an expensive operation for large datasets.
    pub async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError> {
        let _timer = RedisCacheMetrics::global().start_operation_timer("stats_bucket");

        let mut conn = self.connection.clone();
        let mut cursor: u64 = 0;
        let mut item_count: u64 = 0;
        let batch_size = 100;

        // Pattern to match all keys for this bucket
        let pattern = format!("{}:{}:*", self.key_prefix, bucket);

        loop {
            let scan_result: (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(batch_size)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    RedisCacheMetrics::global().errors.inc();
                    CacheError::RedisError(format!("Redis SCAN failed: {}", e))
                })?;

            cursor = scan_result.0;
            item_count += scan_result.1.len() as u64;

            if cursor == 0 {
                break;
            }
        }

        Ok(CacheStats {
            hits: 0,               // Not tracked per-bucket
            misses: 0,             // Not tracked per-bucket
            evictions: 0,          // Not tracked per-bucket
            current_size_bytes: 0, // Redis doesn't track size locally
            current_item_count: item_count,
            max_size_bytes: 0, // Not applicable to Redis
        })
    }
}

// ============================================================
// Cache trait implementation for RedisCache
// ============================================================

use crate::cache::Cache;
use async_trait::async_trait;

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        // Delegate to the inherent method
        RedisCache::get(self, key).await
    }

    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError> {
        // Delegate to the inherent method
        RedisCache::set(self, key, entry).await
    }

    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError> {
        // Format Redis key to check if it exists before deletion
        let redis_key = key::format_key(&self.key_prefix, &key.bucket, &key.object_key);

        // Check if key exists
        let mut conn = self.connection.clone();
        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| CacheError::RedisError(format!("Redis EXISTS failed: {}", e)))?;

        // Delete the key (inherent method returns () not bool)
        RedisCache::delete(self, key).await?;

        // Return whether the key existed
        Ok(exists)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        // Delegate to the inherent method (which returns count, we discard it)
        let _count = RedisCache::clear(self).await?;
        Ok(())
    }

    async fn clear_bucket(&self, bucket: &str) -> Result<usize, CacheError> {
        // Delegate to the inherent method
        RedisCache::clear_bucket(self, bucket).await
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        // Delegate to the inherent method (which is sync, wrap in Ok)
        Ok(RedisCache::stats(self))
    }

    async fn stats_bucket(&self, bucket: &str) -> Result<CacheStats, CacheError> {
        // Delegate to the inherent method
        RedisCache::stats_bucket(self, bucket).await
    }

    async fn run_pending_tasks(&self) {
        // No-op for Redis - all operations are immediately persisted
    }
}

// Verify Send + Sync bounds (required for async trait)
fn _assert_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<RedisCache>();
    assert_sync::<RedisCache>();
}

// Manual Debug implementation (ConnectionManager doesn't implement Debug)
impl std::fmt::Debug for RedisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisCache")
            .field("config", &self.config)
            .field("key_prefix", &self.key_prefix)
            .field("connection", &"<ConnectionManager>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_redis_cache_struct() {
        // This test verifies that RedisCache struct can be defined
        // We can't construct it without async, but we can verify the type exists
        fn _check_type(_cache: RedisCache) {}
    }

    #[test]
    fn test_redis_cache_contains_connection_manager() {
        // This test verifies that RedisCache has a ConnectionManager field
        // The struct definition proves this exists
        fn _check_field(cache: RedisCache) {
            let _conn: ConnectionManager = cache.connection;
        }
    }

    #[test]
    fn test_redis_cache_contains_config() {
        // This test verifies that RedisCache has a RedisConfig field
        fn _check_field(cache: RedisCache) {
            let _config: RedisConfig = cache.config;
        }
    }

    #[test]
    fn test_redis_cache_contains_stats() {
        // This test verifies that RedisCache has stats tracking
        fn _check_field(cache: RedisCache) {
            let _stats: Arc<RedisCacheStats> = cache.stats;
        }
    }

    #[test]
    fn test_redis_cache_contains_key_prefix() {
        // This test verifies that RedisCache has a key_prefix field
        fn _check_field(cache: RedisCache) {
            let _prefix: String = cache.key_prefix;
        }
    }

    #[test]
    fn test_redis_cache_is_send_sync() {
        // This test verifies that RedisCache implements Send + Sync
        // Required for async trait and multi-threaded use
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<RedisCache>();
        assert_sync::<RedisCache>();
    }

    #[tokio::test]
    async fn test_redis_cache_stats_tracker() {
        // Test that stats tracker works correctly
        let stats = RedisCacheStats::new();

        assert_eq!(stats.hits.load(Ordering::Relaxed), 0);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 0);

        stats.increment_hits();
        stats.increment_hits();
        stats.increment_misses();

        assert_eq!(stats.hits.load(Ordering::Relaxed), 2);
        assert_eq!(stats.misses.load(Ordering::Relaxed), 1);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.hits, 2);
        assert_eq!(snapshot.misses, 1);
    }

    // ============================================================
    // Phase 29.2: Redis Authentication Tests
    // ============================================================

    #[test]
    fn test_build_connection_url_no_password_no_db() {
        // Test: URL unchanged when no password and db=0
        let config = RedisConfig {
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 0,
            ..Default::default()
        };

        let url = RedisCache::build_connection_url("redis://localhost:6379", &config).unwrap();
        assert_eq!(url, "redis://localhost:6379");
    }

    #[test]
    fn test_build_connection_url_with_password() {
        // Test: Constructor authenticates with password if provided
        let config = RedisConfig {
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: Some("mysecret".to_string()),
            redis_db: 0,
            ..Default::default()
        };

        let url = RedisCache::build_connection_url("redis://localhost:6379", &config).unwrap();
        assert_eq!(url, "redis://:mysecret@localhost:6379");
    }

    #[test]
    fn test_build_connection_url_with_database() {
        // Test: Constructor selects database number (Redis SELECT)
        let config = RedisConfig {
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 5,
            ..Default::default()
        };

        let url = RedisCache::build_connection_url("redis://localhost:6379", &config).unwrap();
        assert_eq!(url, "redis://localhost:6379/5");
    }

    #[test]
    fn test_build_connection_url_with_password_and_database() {
        // Test: Both password and database number
        let config = RedisConfig {
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: Some("secret123".to_string()),
            redis_db: 3,
            ..Default::default()
        };

        let url = RedisCache::build_connection_url("redis://localhost:6379", &config).unwrap();
        assert_eq!(url, "redis://:secret123@localhost:6379/3");
    }

    #[test]
    fn test_build_connection_url_password_with_special_chars() {
        // Test: Password with special characters gets URL-encoded
        let config = RedisConfig {
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: Some("p@ss:word/test".to_string()),
            redis_db: 0,
            ..Default::default()
        };

        let url = RedisCache::build_connection_url("redis://localhost:6379", &config).unwrap();
        // Special characters should be URL-encoded
        assert!(url.contains("p%40ss%3Aword%2Ftest"));
        assert!(url.starts_with("redis://:"));
    }

    #[test]
    fn test_build_connection_url_rediss_scheme() {
        // Test: TLS/SSL connection (rediss://)
        let config = RedisConfig {
            redis_url: Some("rediss://secure.redis.io:6380".to_string()),
            redis_password: Some("tls_pass".to_string()),
            redis_db: 1,
            ..Default::default()
        };

        let url =
            RedisCache::build_connection_url("rediss://secure.redis.io:6380", &config).unwrap();
        assert_eq!(url, "rediss://:tls_pass@secure.redis.io:6380/1");
    }

    #[test]
    fn test_build_connection_url_invalid_scheme() {
        // Test: Invalid URL scheme returns error
        let config = RedisConfig::default();

        let result = RedisCache::build_connection_url("http://localhost:6379", &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CacheError::ConfigurationError(_)));
    }
}
