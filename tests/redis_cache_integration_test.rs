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
