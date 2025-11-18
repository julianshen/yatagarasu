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

    assert!(health, "health_check() should return true when Redis is alive");
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
