// Redis cache Prometheus metrics integration tests
//
// Tests that verify metrics are properly collected and can be exported

use bytes::Bytes;
use std::time::Duration;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::redis::Redis;
use yatagarasu::cache::redis::{RedisCache, RedisCacheMetrics, RedisConfig};
use yatagarasu::cache::{CacheEntry, CacheKey};

#[tokio::test]
async fn test_metrics_track_cache_hits() {
    // Test: Metrics track cache hits correctly
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_metrics_hits".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();
    let metrics = RedisCacheMetrics::global();

    // Record initial hit count
    let hits_before = metrics.hits.get();

    // Set and get an entry (should increment hits)
    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        None,
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache.set(key.clone(), entry).await.unwrap();
    cache.get(&key).await.unwrap();

    // Verify hits incremented
    let hits_after = metrics.hits.get();
    assert!(
        hits_after > hits_before,
        "Hits should increment after cache get"
    );
}

#[tokio::test]
async fn test_metrics_track_cache_misses() {
    // Test: Metrics track cache misses correctly
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_metrics_misses".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();
    let metrics = RedisCacheMetrics::global();

    // Record initial miss count
    let misses_before = metrics.misses.get();

    // Get non-existent key (should increment misses)
    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "nonexistent.txt".to_string(),
        etag: None,
    };

    cache.get(&key).await.unwrap();

    // Verify misses incremented
    let misses_after = metrics.misses.get();
    assert!(
        misses_after > misses_before,
        "Misses should increment after cache miss"
    );
}

#[tokio::test]
async fn test_metrics_track_cache_sets() {
    // Test: Metrics track cache sets correctly
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_metrics_sets".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();
    let metrics = RedisCacheMetrics::global();

    // Record initial set count
    let sets_before = metrics.sets.get();

    // Set an entry (should increment sets)
    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        None,
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache.set(key, entry).await.unwrap();

    // Verify sets incremented
    let sets_after = metrics.sets.get();
    assert!(
        sets_after > sets_before,
        "Sets should increment after cache set"
    );
}

#[tokio::test]
async fn test_metrics_track_evictions() {
    // Test: Metrics track evictions correctly
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_metrics_evictions".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();
    let metrics = RedisCacheMetrics::global();

    // Record initial eviction count
    let evictions_before = metrics.evictions.get();

    // Set and delete an entry (should increment evictions)
    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        None,
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache.set(key.clone(), entry).await.unwrap();
    cache.delete(&key).await.unwrap();

    // Verify evictions incremented
    let evictions_after = metrics.evictions.get();
    assert!(
        evictions_after > evictions_before,
        "Evictions should increment after cache delete"
    );
}

#[tokio::test]
async fn test_can_export_metrics_in_prometheus_format() {
    // Test: Can export metrics in Prometheus text format
    use prometheus::Encoder;
    use prometheus::TextEncoder;

    // Initialize metrics by accessing them
    let _metrics = RedisCacheMetrics::global();

    // Create encoder
    let encoder = TextEncoder::new();

    // Gather all metrics
    let metric_families = prometheus::gather();

    // Encode to Prometheus text format
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    // Convert to string
    let metrics_output = String::from_utf8(buffer).unwrap();

    // Verify Yatagarasu metrics are present
    // Note: Some metrics may not appear if they haven't been used yet
    assert!(
        metrics_output.contains("yatagarasu_cache_operations_total")
            || metrics_output.contains("yatagarasu_cache"),
        "Should contain cache operations counter"
    );

    // At least one cache metric should be present
    let has_cache_metrics = metrics_output.contains("yatagarasu_cache_operations_total")
        || metrics_output.contains("yatagarasu_cache_operation_duration")
        || metrics_output.contains("yatagarasu_cache_serialization_duration")
        || metrics_output.contains("yatagarasu_redis");

    assert!(
        has_cache_metrics,
        "Should contain at least one Yatagarasu cache metric"
    );
}

#[tokio::test]
async fn test_metrics_endpoint_format() {
    // Test: Metrics can be formatted for HTTP endpoint
    use prometheus::Encoder;
    use prometheus::TextEncoder;

    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis::default());
    let redis_container = docker.run(redis_image);

    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = RedisConfig {
        redis_url: Some(redis_url),
        redis_key_prefix: "test_metrics_format".to_string(),
        ..Default::default()
    };

    let cache = RedisCache::new(config).await.unwrap();

    // Perform some operations
    let entry = CacheEntry::new(
        Bytes::from("test data"),
        "text/plain".to_string(),
        "etag123".to_string(),
        None,
        Some(Duration::from_secs(3600)),
    );

    let key = CacheKey {
        bucket: "bucket1".to_string(),
        object_key: "file.txt".to_string(),
        etag: None,
    };

    cache.set(key.clone(), entry).await.unwrap();
    cache.get(&key).await.unwrap();

    // Export metrics
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let metrics_output = String::from_utf8(buffer).unwrap();

    // Verify format
    assert!(metrics_output.contains("# HELP"));
    assert!(metrics_output.contains("# TYPE"));
    assert!(metrics_output.contains("yatagarasu"));

    // Print sample output for debugging
    println!("\n=== Sample Metrics Output ===");
    for line in metrics_output.lines().take(20) {
        if line.contains("yatagarasu") {
            println!("{}", line);
        }
    }
}
