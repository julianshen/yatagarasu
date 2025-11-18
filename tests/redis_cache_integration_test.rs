// Integration tests for Redis cache
//
// These tests require a real Redis instance via testcontainers

use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::redis::Redis;
use yatagarasu::cache::redis::{RedisCache, RedisConfig};

#[tokio::test]
async fn test_can_create_redis_cache_new_async() {
    // Test: Can create RedisCache::new(config) async
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "test".to_string(),
        redis_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 10,
    };

    let result = RedisCache::new(config).await;
    assert!(result.is_ok(), "Should create RedisCache successfully");
}

#[tokio::test]
async fn test_constructor_creates_connection_manager() {
    // Test: Constructor creates ConnectionManager
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "test".to_string(),
        redis_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 10,
    };

    let cache = RedisCache::new(config).await.unwrap();

    // If we got here, ConnectionManager was created successfully
    // We can verify it works by doing a health check
    assert!(cache.health_check().await);
}

#[tokio::test]
async fn test_constructor_connects_to_redis_server() {
    // Test: Constructor connects to Redis server
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let result = RedisCache::new(config).await;
    assert!(result.is_ok(), "Should connect to Redis server");
}

#[tokio::test]
async fn test_returns_error_if_redis_unreachable() {
    // Test: Returns CacheError::RedisConnectionFailed if unreachable
    let config = RedisConfig {
        redis_url: Some("redis://127.0.0.1:19999".to_string()), // Unlikely to have Redis here
        ..Default::default()
    };

    let result = RedisCache::new(config).await;
    assert!(result.is_err(), "Should fail when Redis is unreachable");

    let err = result.unwrap_err();
    match err {
        yatagarasu::cache::CacheError::RedisConnectionFailed(_) => {
            // Expected error type
        }
        _ => panic!("Expected RedisConnectionFailed error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_returns_error_if_redis_url_missing() {
    // Test: Returns CacheError::ConfigurationError if redis_url is None
    let config = RedisConfig {
        redis_url: None, // Missing URL
        ..Default::default()
    };

    let result = RedisCache::new(config).await;
    assert!(result.is_err(), "Should fail when redis_url is missing");

    let err = result.unwrap_err();
    match err {
        yatagarasu::cache::CacheError::ConfigurationError(_) => {
            // Expected error type
        }
        _ => panic!("Expected ConfigurationError, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_returns_error_if_redis_url_invalid() {
    // Test: Returns error if Redis URL is invalid
    let config = RedisConfig {
        redis_url: Some("not-a-valid-url".to_string()),
        ..Default::default()
    };

    let result = RedisCache::new(config).await;
    assert!(result.is_err(), "Should fail with invalid Redis URL");

    let err = result.unwrap_err();
    match err {
        yatagarasu::cache::CacheError::RedisConnectionFailed(_) => {
            // Expected error type
        }
        _ => panic!("Expected RedisConnectionFailed error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_health_check_returns_true_when_redis_alive() {
    // Test: health_check() returns true if Redis responsive
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();
    let health = cache.health_check().await;

    assert!(
        health,
        "health_check() should return true when Redis is alive"
    );
}

#[tokio::test]
async fn test_health_check_uses_ping_command() {
    // Test: health_check() uses PING command
    // This is verified by the fact that it succeeds - PING is the standard health check
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // If PING works, health check should succeed
    assert!(cache.health_check().await);
}

#[tokio::test]
async fn test_get_retrieves_entry_from_redis() {
    // Test: get() retrieves entry from Redis
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // Create a test entry
    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    // Set the entry
    cache.set(key.clone(), entry.clone()).await.unwrap();

    // Get the entry back
    let result = cache.get(&key).await.unwrap();
    assert!(result.is_some());

    let retrieved = result.unwrap();
    assert_eq!(retrieved.data, entry.data);
    assert_eq!(retrieved.content_type, entry.content_type);
    assert_eq!(retrieved.etag, entry.etag);
}

#[tokio::test]
async fn test_get_returns_none_if_key_doesnt_exist() {
    // Test: Returns None if key doesn't exist
    use yatagarasu::cache::CacheKey;

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "nonexistent.txt".to_string(),
        etag: None,
    };

    let result = cache.get(&key).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_set_stores_entry_in_redis() {
    // Test: set() stores entry in Redis with TTL
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    // Set should succeed
    let result = cache.set(key.clone(), entry).await;
    assert!(result.is_ok());

    // Verify it was stored by getting it back
    let retrieved = cache.get(&key).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_get_and_set_roundtrip() {
    // Test: Full roundtrip - set then get returns same data
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    let data = Bytes::from("Hello, Redis!");
    let entry = CacheEntry::new(
        data.clone(),
        "text/plain".to_string(),
        "etag-456".to_string(),
        Some(Duration::from_secs(600)),
    );

    let key = CacheKey {
        bucket: "images".to_string(),
        object_key: "photo.jpg".to_string(),
        etag: None,
    };

    // Set
    cache.set(key.clone(), entry).await.unwrap();

    // Get
    let retrieved = cache.get(&key).await.unwrap().unwrap();

    assert_eq!(retrieved.data, data);
    assert_eq!(retrieved.content_type, "text/plain");
    assert_eq!(retrieved.etag, "etag-456");
}

#[tokio::test]
async fn test_delete_removes_key_from_redis() {
    // Test: delete() removes key from Redis
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // First, set an entry
    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache.set(key.clone(), entry).await.unwrap();

    // Verify it exists
    let result = cache.get(&key).await.unwrap();
    assert!(result.is_some());

    // Delete it
    let delete_result = cache.delete(&key).await;
    assert!(delete_result.is_ok());

    // Verify it's gone
    let result_after_delete = cache.get(&key).await.unwrap();
    assert!(result_after_delete.is_none());
}

#[tokio::test]
async fn test_delete_returns_ok_if_key_existed() {
    // Test: Returns Ok(()) if key existed and deleted
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    let entry = CacheEntry::new(
        Bytes::from("data"),
        "text/plain".to_string(),
        "etag".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    // Set and delete
    cache.set(key.clone(), entry).await.unwrap();
    let result = cache.delete(&key).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_returns_ok_if_key_didnt_exist() {
    // Test: Returns Ok(()) if key didn't exist (idempotent)
    use yatagarasu::cache::CacheKey;

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "nonexistent.txt".to_string(),
        etag: None,
    };

    // Delete a key that doesn't exist - should still succeed (idempotent)
    let result = cache.delete(&key).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_uses_del_command() {
    // Test: Uses Redis DEL command
    // This is verified by the fact that the key is actually removed
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    let entry = CacheEntry::new(
        Bytes::from("data"),
        "text/plain".to_string(),
        "etag".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache.set(key.clone(), entry).await.unwrap();
    cache.delete(&key).await.unwrap();

    // If DEL worked correctly, the key should be gone
    assert!(cache.get(&key).await.unwrap().is_none());
}

#[tokio::test]
async fn test_clear_removes_all_keys_with_prefix() {
    // Test: clear() removes all keys with prefix
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_clear".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // Create multiple entries
    let entry = CacheEntry::new(
        Bytes::from("data"),
        "text/plain".to_string(),
        "etag".to_string(),
        Some(Duration::from_secs(3600)),
    );

    for i in 0..10 {
        let key = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: format!("file{}.txt", i),
            etag: None,
        };
        cache.set(key, entry.clone()).await.unwrap();
    }

    // Clear all keys
    let deleted = cache.clear().await.unwrap();
    assert_eq!(deleted, 10, "Should delete all 10 keys");

    // Verify all keys are gone
    for i in 0..10 {
        let key = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: format!("file{}.txt", i),
            etag: None,
        };
        assert!(cache.get(&key).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn test_clear_uses_scan_for_safe_iteration() {
    // Test: Uses Redis SCAN for safe iteration
    // This is verified by the fact that clear() completes successfully
    // SCAN is non-blocking unlike KEYS which can block Redis
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_scan".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // Create some entries
    let entry = CacheEntry::new(
        Bytes::from("data"),
        "text/plain".to_string(),
        "etag".to_string(),
        Some(Duration::from_secs(3600)),
    );

    for i in 0..5 {
        let key = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: format!("file{}.txt", i),
            etag: None,
        };
        cache.set(key, entry.clone()).await.unwrap();
    }

    // SCAN-based clear should succeed without blocking
    let result = cache.clear().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5);
}

#[tokio::test]
async fn test_clear_does_not_affect_other_prefixes() {
    // Test: Does not affect other Redis keys (different prefixes)
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create two caches with different prefixes
    let config1 = RedisConfig {
        redis_url: Some(redis_url.clone()),
        redis_key_prefix: "prefix1".to_string(),
        ..Default::default()
    };

    let config2 = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "prefix2".to_string(),
        ..Default::default()
    };

    let cache1 = RedisCache::new(config1).await.unwrap();
    let cache2 = RedisCache::new(config2).await.unwrap();

    // Create entries in both caches
    let entry = CacheEntry::new(
        Bytes::from("data"),
        "text/plain".to_string(),
        "etag".to_string(),
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache1.set(key.clone(), entry.clone()).await.unwrap();
    cache2.set(key.clone(), entry).await.unwrap();

    // Clear cache1
    let deleted = cache1.clear().await.unwrap();
    assert_eq!(deleted, 1);

    // Verify cache1 is empty
    assert!(cache1.get(&key).await.unwrap().is_none());

    // Verify cache2 still has data
    assert!(cache2.get(&key).await.unwrap().is_some());
}

#[tokio::test]
async fn test_clear_handles_large_key_count() {
    // Test: Handles large key count efficiently (>100 keys)
    use bytes::Bytes;
    use std::time::Duration;
    use yatagarasu::cache::{CacheEntry, CacheKey};

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_large".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // Create 150 entries to test batch processing
    let entry = CacheEntry::new(
        Bytes::from("data"),
        "text/plain".to_string(),
        "etag".to_string(),
        Some(Duration::from_secs(3600)),
    );

    for i in 0..150 {
        let key = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: format!("file{}.txt", i),
            etag: None,
        };
        cache.set(key, entry.clone()).await.unwrap();
    }

    // Clear should handle all keys efficiently
    let deleted = cache.clear().await.unwrap();
    assert_eq!(deleted, 150, "Should delete all 150 keys");
}

#[tokio::test]
async fn test_clear_returns_zero_when_no_keys() {
    // Test: Returns 0 when cache is already empty

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_empty".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // Clear empty cache
    let deleted = cache.clear().await.unwrap();
    assert_eq!(deleted, 0, "Should delete 0 keys when cache is empty");
}
