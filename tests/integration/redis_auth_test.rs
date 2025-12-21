// Redis Authentication Integration Tests
// Phase 53.2: Redis Advanced Configuration Tests
//
// Tests Redis cache with authentication enabled:
// - Connection with valid password
// - Rejection with invalid password
// - TTL expiration behavior

use bytes::Bytes;
use std::time::Duration;
use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use testcontainers_modules::redis::Redis;
use yatagarasu::cache::redis::{RedisCache, RedisConfig};
use yatagarasu::cache::{CacheEntry, CacheKey};

/// Create a Redis container with password authentication
fn create_redis_with_password<'a>(
    docker: &'a Cli,
    password: &str,
) -> (testcontainers::Container<'a, GenericImage>, u16) {
    let redis_image = GenericImage::new("redis", "7")
        .with_exposed_port(6379)
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

    let args: Vec<String> = vec![
        "redis-server".to_string(),
        "--requirepass".to_string(),
        password.to_string(),
    ];
    let runnable_image = RunnableImage::from((redis_image, args));

    let container = docker.run(runnable_image);
    let port = container.get_host_port_ipv4(6379);

    (container, port)
}

/// Create a Redis container with maxmemory and eviction policy
fn create_redis_with_maxmemory<'a>(
    docker: &'a Cli,
    maxmemory: &str,
    policy: &str,
) -> (testcontainers::Container<'a, GenericImage>, u16) {
    let redis_image = GenericImage::new("redis", "7")
        .with_exposed_port(6379)
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

    let args: Vec<String> = vec![
        "redis-server".to_string(),
        "--maxmemory".to_string(),
        maxmemory.to_string(),
        "--maxmemory-policy".to_string(),
        policy.to_string(),
    ];
    let runnable_image = RunnableImage::from((redis_image, args));

    let container = docker.run(runnable_image);
    let port = container.get_host_port_ipv4(6379);

    (container, port)
}

/// Helper to create a test cache key
fn test_key(name: &str) -> CacheKey {
    CacheKey {
        bucket: "test-bucket".to_string(),
        object_key: format!("test/{}.txt", name),
        etag: None,
        variant: None,
    }
}

/// Helper to create a test cache entry
fn test_entry(data: &[u8], ttl_secs: Option<u64>) -> CacheEntry {
    let ttl = ttl_secs.map(Duration::from_secs);
    CacheEntry::new(
        Bytes::from(data.to_vec()),
        "text/plain".to_string(),
        "test-etag".to_string(),
        None,
        ttl,
    )
}

/// Test: Redis with authentication - successful connection
#[tokio::test]
#[ignore] // Requires Docker: cargo test --test integration_tests -- redis_auth --ignored --nocapture
async fn test_redis_with_authentication_connects_successfully() {
    // Start Redis with password authentication using --requirepass flag
    let docker = Cli::default();
    let (_container, redis_port) = create_redis_with_password(&docker, "testsecret123");
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create config with correct password
    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: Some("testsecret123".to_string()),
        redis_db: 0,
        redis_key_prefix: "auth-test".to_string(),
        redis_ttl_seconds: 60,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    // Create Redis cache - should succeed
    let cache = RedisCache::new(config).await;
    assert!(cache.is_ok(), "Should connect with valid password");

    let cache = cache.unwrap();

    // Test set operation
    let key = test_key("auth-test");
    let entry = test_entry(b"authenticated content", None);
    let result = cache.set(key.clone(), entry).await;
    assert!(
        result.is_ok(),
        "Should set value with authenticated connection"
    );

    // Test get operation
    let result = cache.get(&key).await;
    assert!(
        result.is_ok(),
        "Should get value with authenticated connection"
    );
    let data = result.unwrap();
    assert!(data.is_some(), "Should find cached value");
    assert_eq!(data.unwrap().data.as_ref(), b"authenticated content");

    // Cleanup
    let _ = cache.delete(&key).await;
}

/// Test: Redis with wrong password - connection fails
#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_with_wrong_password_fails() {
    // Start Redis with password authentication
    let docker = Cli::default();
    let (_container, redis_port) = create_redis_with_password(&docker, "correctpassword");
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create config with WRONG password
    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: Some("wrongpassword".to_string()),
        redis_db: 0,
        redis_key_prefix: "auth-test".to_string(),
        redis_ttl_seconds: 60,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    // Create Redis cache - should fail with auth error
    let cache = RedisCache::new(config).await;

    // Either connection fails, or operations fail
    if let Ok(cache) = cache {
        // If connection pool was created, operations should fail
        let key = test_key("wrong-password");
        let result = cache.get(&key).await;
        assert!(
            result.is_err(),
            "Operations should fail with wrong password"
        );
    }
    // Connection failure is also acceptable
}

/// Test: Redis TTL expiration works correctly
#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_ttl_expiration() {
    // Start Redis without password for simplicity
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let redis_port = container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create config with short TTL (2 seconds)
    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "ttl-test".to_string(),
        redis_ttl_seconds: 2, // 2 seconds
        redis_max_ttl_seconds: 10,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    let cache = RedisCache::new(config).await.expect("Should connect");

    // Set a value with explicit 2 second TTL
    // Note: CacheEntry::new TTL takes precedence, not config TTL
    let key = test_key("ttl-test");
    let entry = test_entry(b"expires soon", Some(2)); // 2 seconds TTL
    cache
        .set(key.clone(), entry)
        .await
        .expect("Should set value");

    // Verify it exists
    let result = cache.get(&key).await.expect("Should get value");
    assert!(result.is_some(), "Value should exist immediately");

    // Wait for TTL to expire (2 seconds + buffer)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Value should be expired now
    let result = cache.get(&key).await.expect("Get should succeed");
    assert!(result.is_none(), "Value should have expired after TTL");
}

/// Test: Redis database selection (isolation between databases)
#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_database_selection() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let redis_port = container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create cache on database 0
    let config_db0 = RedisConfig {
        redis_url: Some(redis_url.clone()),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "db-test".to_string(),
        redis_ttl_seconds: 60,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    // Create cache on database 1
    let config_db1 = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 1,
        redis_key_prefix: "db-test".to_string(),
        redis_ttl_seconds: 60,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    let cache_db0 = RedisCache::new(config_db0)
        .await
        .expect("Should connect to db 0");
    let cache_db1 = RedisCache::new(config_db1)
        .await
        .expect("Should connect to db 1");

    // Set same key in both databases
    let key = test_key("shared-key");
    let entry_db0 = test_entry(b"value in db 0", None);
    let entry_db1 = test_entry(b"value in db 1", None);

    cache_db0
        .set(key.clone(), entry_db0)
        .await
        .expect("Set in db 0");
    cache_db1
        .set(key.clone(), entry_db1)
        .await
        .expect("Set in db 1");

    // Each should see its own value
    let val0 = cache_db0.get(&key).await.unwrap().unwrap();
    let val1 = cache_db1.get(&key).await.unwrap().unwrap();

    assert_eq!(val0.data.as_ref(), b"value in db 0");
    assert_eq!(val1.data.as_ref(), b"value in db 1");

    // Cleanup
    let _ = cache_db0.delete(&key).await;
    let _ = cache_db1.delete(&key).await;
}

/// Test: Redis key prefix isolation
#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_key_prefix_isolation() {
    let docker = Cli::default();
    let container = docker.run(Redis::default());
    let redis_port = container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create cache with prefix "app1"
    let config_app1 = RedisConfig {
        redis_url: Some(redis_url.clone()),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "app1".to_string(),
        redis_ttl_seconds: 60,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    // Create cache with prefix "app2"
    let config_app2 = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "app2".to_string(),
        redis_ttl_seconds: 60,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    let cache_app1 = RedisCache::new(config_app1)
        .await
        .expect("Should create app1 cache");
    let cache_app2 = RedisCache::new(config_app2)
        .await
        .expect("Should create app2 cache");

    // Set same logical key in both caches
    let key = test_key("shared-key");
    let entry_app1 = test_entry(b"app1 data", None);
    let entry_app2 = test_entry(b"app2 data", None);

    cache_app1
        .set(key.clone(), entry_app1)
        .await
        .expect("Set in app1");
    cache_app2
        .set(key.clone(), entry_app2)
        .await
        .expect("Set in app2");

    // Each should see its own value (isolated by prefix)
    let val1 = cache_app1.get(&key).await.unwrap().unwrap();
    let val2 = cache_app2.get(&key).await.unwrap().unwrap();

    assert_eq!(val1.data.as_ref(), b"app1 data");
    assert_eq!(val2.data.as_ref(), b"app2 data");

    // Cleanup
    let _ = cache_app1.delete(&key).await;
    let _ = cache_app2.delete(&key).await;
}

/// Test: Redis maxmemory-policy=allkeys-lru evicts old keys when memory limit reached
#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_maxmemory_allkeys_lru_eviction() {
    // Start Redis with 4MB maxmemory and allkeys-lru eviction
    // Need enough space for Redis overhead (~2MB) plus some data
    let docker = Cli::default();
    let (_container, redis_port) = create_redis_with_maxmemory(&docker, "4mb", "allkeys-lru");
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "eviction-test".to_string(),
        redis_ttl_seconds: 3600,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    let cache = RedisCache::new(config).await.expect("Should connect");

    // Create data larger than 4MB total to trigger evictions
    // Each entry is ~10KB, so 600 entries = ~6MB (exceeds 4MB limit)
    let data_10kb = vec![b'x'; 10 * 1024];
    let num_entries = 600;

    // Insert many entries - this should trigger evictions
    for i in 0..num_entries {
        let key = CacheKey {
            bucket: "eviction-bucket".to_string(),
            object_key: format!("file-{:04}.bin", i),
            etag: None,
            variant: None,
        };
        let entry = CacheEntry::new(
            Bytes::from(data_10kb.clone()),
            "application/octet-stream".to_string(),
            format!("etag-{}", i),
            None,
            Some(Duration::from_secs(3600)),
        );

        // Sets should succeed even when memory is full (LRU eviction kicks in)
        let result = cache.set(key, entry).await;
        assert!(
            result.is_ok(),
            "Set should succeed with LRU eviction policy, got error at entry {}: {:?}",
            i,
            result.err()
        );
    }

    // Access early entries to check they were evicted
    // With LRU policy, early entries should have been evicted
    let mut evicted_count = 0;
    for i in 0..100 {
        let key = CacheKey {
            bucket: "eviction-bucket".to_string(),
            object_key: format!("file-{:04}.bin", i),
            etag: None,
            variant: None,
        };
        if let Ok(None) = cache.get(&key).await {
            evicted_count += 1;
        }
    }

    // We expect some early entries to be evicted (at least 50% should be gone)
    // since we wrote 6MB of data to a 4MB cache
    assert!(
        evicted_count > 50,
        "Expected at least 50 early entries to be evicted, but only {} were evicted",
        evicted_count
    );

    // Recent entries should still exist
    let mut recent_found = 0;
    for i in (num_entries - 50)..num_entries {
        let key = CacheKey {
            bucket: "eviction-bucket".to_string(),
            object_key: format!("file-{:04}.bin", i),
            etag: None,
            variant: None,
        };
        if let Ok(Some(_)) = cache.get(&key).await {
            recent_found += 1;
        }
    }

    // Most recent entries should still be present
    assert!(
        recent_found >= 30,
        "Expected at least 30 of 50 recent entries to exist, but only {} found",
        recent_found
    );
}

/// Test: Redis with noeviction policy returns error when memory full
#[tokio::test]
#[ignore] // Requires Docker
async fn test_redis_maxmemory_noeviction_fails() {
    // Start Redis with very small maxmemory (500KB) and noeviction policy
    let docker = Cli::default();
    let (_container, redis_port) = create_redis_with_maxmemory(&docker, "500kb", "noeviction");
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "noeviction-test".to_string(),
        redis_ttl_seconds: 3600,
        redis_max_ttl_seconds: 3600,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 5,
    };

    let cache = RedisCache::new(config).await.expect("Should connect");

    // Create data to exceed 500KB limit
    let data_100kb = vec![b'y'; 100 * 1024];
    let mut error_occurred = false;

    // Try to insert 10 entries (1MB total) - should eventually fail
    for i in 0..10 {
        let key = CacheKey {
            bucket: "noeviction-bucket".to_string(),
            object_key: format!("bigfile-{}.bin", i),
            etag: None,
            variant: None,
        };
        let entry = CacheEntry::new(
            Bytes::from(data_100kb.clone()),
            "application/octet-stream".to_string(),
            format!("etag-{}", i),
            None,
            Some(Duration::from_secs(3600)),
        );

        if cache.set(key, entry).await.is_err() {
            error_occurred = true;
            break;
        }
    }

    // With noeviction policy, we should get an error when memory is full
    assert!(
        error_occurred,
        "Expected error when exceeding maxmemory with noeviction policy"
    );
}
