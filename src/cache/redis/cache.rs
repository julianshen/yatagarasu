// Redis cache implementation
//
// Provides distributed caching using Redis with production-ready error handling.

use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::config::RedisConfig;
use super::key;
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
        let redis_url = config.redis_url.as_ref().ok_or_else(|| {
            CacheError::ConfigurationError("redis_url is required".to_string())
        })?;

        // Create Redis client
        let client = Client::open(redis_url.as_str()).map_err(|e| {
            CacheError::RedisConnectionFailed(format!("Invalid Redis URL: {}", e))
        })?;

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
        // Format Redis key
        let redis_key = key::format_key(&self.key_prefix, &key.bucket, &key.object_key);

        // Validate the key
        if let Err(e) = key::validate_key(&redis_key) {
            return Err(CacheError::ConfigurationError(e));
        }

        // Clone connection for async operation
        let mut conn = self.connection.clone();

        // Get bytes from Redis using GET command
        let bytes: Option<Vec<u8>> = conn
            .get(&redis_key)
            .await
            .map_err(|e| CacheError::RedisError(format!("Redis GET failed: {}", e)))?;

        match bytes {
            Some(data) => {
                // Deserialize the entry
                match serialization::deserialize_entry(&data) {
                    Ok(entry) => {
                        // Increment hit counter
                        self.stats.increment_hits();
                        Ok(Some(entry))
                    }
                    Err(e) => {
                        // Increment error counter on deserialization failure
                        self.stats.increment_errors();
                        // Treat deserialization errors as cache miss (return None)
                        // Log the error but don't fail the operation
                        tracing::warn!(
                            "Failed to deserialize cache entry for key '{}': {}",
                            redis_key,
                            e
                        );
                        self.stats.increment_misses();
                        Ok(None)
                    }
                }
            }
            None => {
                // Key not found - increment miss counter
                self.stats.increment_misses();
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
        // Format Redis key
        let redis_key = key::format_key(&self.key_prefix, &key.bucket, &key.object_key);

        // Validate the key
        if let Err(e) = key::validate_key(&redis_key) {
            return Err(CacheError::ConfigurationError(e));
        }

        // Serialize the entry
        let bytes = serialization::serialize_entry(&entry)?;

        // Clone connection for async operation
        let mut conn = self.connection.clone();

        // Calculate TTL in seconds
        let ttl_secs = self.config.redis_ttl_seconds;

        // Use SETEX to set with TTL
        conn.set_ex::<_, _, ()>(&redis_key, bytes, ttl_secs)
            .await
            .map_err(|e| CacheError::RedisError(format!("Redis SETEX failed: {}", e)))?;

        // Increment set counter
        self.stats.increment_sets();

        Ok(())
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
}
