// Cache Metrics Tests (Phase 65.2)
// Tests for per-bucket and per-layer cache metrics

use std::collections::HashMap;

/// Phase 65.2: Test cache hit metrics with bucket and layer labels
#[test]
fn test_cache_hit_with_labels() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Record cache hits for different buckets and layers
    metrics.increment_cache_hit_with_labels("products", "memory");
    metrics.increment_cache_hit_with_labels("products", "memory");
    metrics.increment_cache_hit_with_labels("products", "disk");
    metrics.increment_cache_hit_with_labels("images", "memory");
    metrics.increment_cache_hit_with_labels("images", "redis");

    // Verify per-bucket-layer counts
    let hits = metrics.get_cache_hits_by_bucket_layer();
    assert_eq!(
        hits.get("products:memory"),
        Some(&2),
        "products:memory should have 2 hits"
    );
    assert_eq!(
        hits.get("products:disk"),
        Some(&1),
        "products:disk should have 1 hit"
    );
    assert_eq!(
        hits.get("images:memory"),
        Some(&1),
        "images:memory should have 1 hit"
    );
    assert_eq!(
        hits.get("images:redis"),
        Some(&1),
        "images:redis should have 1 hit"
    );

    // Verify total count (global counter should also be updated)
    let prometheus_output = metrics.export_prometheus();
    assert!(
        prometheus_output.contains("yatagarasu_cache_hits_total 5"),
        "Total cache hits should be 5"
    );
}

/// Phase 65.2: Test cache miss metrics with bucket and layer labels
#[test]
fn test_cache_miss_with_labels() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Record cache misses for different buckets and layers
    metrics.increment_cache_miss_with_labels("products", "memory");
    metrics.increment_cache_miss_with_labels("products", "disk");
    metrics.increment_cache_miss_with_labels("products", "redis");
    metrics.increment_cache_miss_with_labels("images", "memory");

    // Verify per-bucket-layer counts
    let misses = metrics.get_cache_misses_by_bucket_layer();
    assert_eq!(
        misses.get("products:memory"),
        Some(&1),
        "products:memory should have 1 miss"
    );
    assert_eq!(
        misses.get("products:disk"),
        Some(&1),
        "products:disk should have 1 miss"
    );
    assert_eq!(
        misses.get("products:redis"),
        Some(&1),
        "products:redis should have 1 miss"
    );
    assert_eq!(
        misses.get("images:memory"),
        Some(&1),
        "images:memory should have 1 miss"
    );

    // Verify total count
    let prometheus_output = metrics.export_prometheus();
    assert!(
        prometheus_output.contains("yatagarasu_cache_misses_total 4"),
        "Total cache misses should be 4"
    );
}

/// Phase 65.2: Test cache eviction metrics with layer labels
#[test]
fn test_cache_eviction_with_layer() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Record evictions for different layers
    metrics.increment_cache_eviction_with_layer("memory");
    metrics.increment_cache_eviction_with_layer("memory");
    metrics.increment_cache_eviction_with_layer("disk");
    metrics.increment_cache_eviction_with_layer("redis");

    // Verify per-layer counts
    let evictions = metrics.get_cache_evictions_by_layer();
    assert_eq!(
        evictions.get("memory"),
        Some(&2),
        "memory layer should have 2 evictions"
    );
    assert_eq!(
        evictions.get("disk"),
        Some(&1),
        "disk layer should have 1 eviction"
    );
    assert_eq!(
        evictions.get("redis"),
        Some(&1),
        "redis layer should have 1 eviction"
    );

    // Verify total count
    let prometheus_output = metrics.export_prometheus();
    assert!(
        prometheus_output.contains("yatagarasu_cache_evictions_total 4"),
        "Total cache evictions should be 4"
    );
}

/// Phase 65.2: Test cache size gauge with layer labels
#[test]
fn test_cache_size_with_layer() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Set cache sizes for different layers
    metrics.set_cache_size_with_layer("memory", 1024 * 1024); // 1MB
    metrics.set_cache_size_with_layer("disk", 10 * 1024 * 1024); // 10MB
    metrics.set_cache_size_with_layer("redis", 5 * 1024 * 1024); // 5MB

    // Verify per-layer sizes
    let sizes = metrics.get_cache_size_by_layer();
    assert_eq!(
        sizes.get("memory"),
        Some(&(1024 * 1024)),
        "memory should be 1MB"
    );
    assert_eq!(
        sizes.get("disk"),
        Some(&(10 * 1024 * 1024)),
        "disk should be 10MB"
    );
    assert_eq!(
        sizes.get("redis"),
        Some(&(5 * 1024 * 1024)),
        "redis should be 5MB"
    );
}

/// Phase 65.2: Test cache item count gauge with layer labels
#[test]
fn test_cache_items_with_layer() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Set item counts for different layers
    metrics.set_cache_items_with_layer("memory", 100);
    metrics.set_cache_items_with_layer("disk", 500);
    metrics.set_cache_items_with_layer("redis", 250);

    // Verify per-layer item counts
    let items = metrics.get_cache_items_by_layer();
    assert_eq!(
        items.get("memory"),
        Some(&100),
        "memory should have 100 items"
    );
    assert_eq!(items.get("disk"), Some(&500), "disk should have 500 items");
    assert_eq!(
        items.get("redis"),
        Some(&250),
        "redis should have 250 items"
    );
}

/// Phase 65.2: Test Prometheus output includes labeled metrics
#[test]
fn test_prometheus_output_includes_labeled_metrics() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Record various metrics
    metrics.increment_cache_hit_with_labels("bucket1", "memory");
    metrics.increment_cache_miss_with_labels("bucket1", "disk");
    metrics.increment_cache_eviction_with_layer("memory");
    metrics.set_cache_size_with_layer("memory", 1000);
    metrics.set_cache_items_with_layer("memory", 10);

    let output = metrics.export_prometheus();

    // Verify labeled metrics in Prometheus output
    assert!(
        output.contains(
            "yatagarasu_cache_hits_by_bucket_layer{bucket=\"bucket1\",layer=\"memory\"} 1"
        ),
        "Output should contain labeled cache hits"
    );
    assert!(
        output.contains(
            "yatagarasu_cache_misses_by_bucket_layer{bucket=\"bucket1\",layer=\"disk\"} 1"
        ),
        "Output should contain labeled cache misses"
    );
    assert!(
        output.contains("yatagarasu_cache_evictions_by_layer{layer=\"memory\"} 1"),
        "Output should contain labeled evictions"
    );
    assert!(
        output.contains("yatagarasu_cache_size_by_layer{layer=\"memory\"} 1000"),
        "Output should contain labeled cache size"
    );
    assert!(
        output.contains("yatagarasu_cache_items_by_layer{layer=\"memory\"} 10"),
        "Output should contain labeled item count"
    );
}

/// Phase 65.2: Test metrics maintain global counters alongside labeled ones
#[test]
fn test_global_and_labeled_counters_synchronized() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();

    // Record multiple hits across different buckets/layers
    for _ in 0..10 {
        metrics.increment_cache_hit_with_labels("bucket-a", "memory");
    }
    for _ in 0..5 {
        metrics.increment_cache_hit_with_labels("bucket-b", "disk");
    }
    for _ in 0..3 {
        metrics.increment_cache_hit_with_labels("bucket-c", "redis");
    }

    // Global counter should be sum of all labeled counters
    let output = metrics.export_prometheus();
    assert!(
        output.contains("yatagarasu_cache_hits_total 18"),
        "Total should be 10+5+3=18"
    );

    // Verify labeled counts
    let hits = metrics.get_cache_hits_by_bucket_layer();
    assert_eq!(hits.get("bucket-a:memory"), Some(&10));
    assert_eq!(hits.get("bucket-b:disk"), Some(&5));
    assert_eq!(hits.get("bucket-c:redis"), Some(&3));
}

/// Phase 65.2: Test Prometheus metric format compliance
#[test]
fn test_prometheus_format_compliance() {
    use yatagarasu::metrics::Metrics;

    let metrics = Metrics::new();
    metrics.increment_cache_hit_with_labels("test", "memory");

    let output = metrics.export_prometheus();

    // Check for proper HELP and TYPE declarations
    assert!(
        output.contains("# HELP yatagarasu_cache_hits_by_bucket_layer"),
        "Should have HELP for labeled hits"
    );
    assert!(
        output.contains("# TYPE yatagarasu_cache_hits_by_bucket_layer counter"),
        "Should have TYPE counter for labeled hits"
    );
    assert!(
        output.contains("# HELP yatagarasu_cache_size_by_layer"),
        "Should have HELP for size gauge"
    );
    assert!(
        output.contains("# TYPE yatagarasu_cache_size_by_layer gauge"),
        "Should have TYPE gauge for size"
    );
}
