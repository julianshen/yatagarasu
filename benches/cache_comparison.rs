//! Comprehensive Cache Comparison Benchmarks
//!
//! Phase 36: Comparative analysis of all cache implementations
//! - Memory (Moka)
//! - Disk (tokio::fs)
//! - Redis (redis crate + MessagePack)
//! - Tiered (Memory -> Disk -> Redis)
//!
//! Run with: cargo bench --bench cache_comparison

use bytes::Bytes;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use yatagarasu::cache::disk::DiskCache;
use yatagarasu::cache::{Cache, CacheEntry, CacheKey, MemoryCache, MemoryCacheConfig};

// =============================================================================
// Test Data Generation (Phase 36.1)
// =============================================================================

/// Standard test data sizes for benchmarking
const SIZE_1KB: usize = 1024;
const SIZE_10KB: usize = 10 * 1024;
const SIZE_100KB: usize = 100 * 1024;
const SIZE_1MB: usize = 1024 * 1024;

/// Generate test data of specified size with realistic content
fn generate_test_data(size: usize) -> Bytes {
    // Generate semi-realistic data (not just zeros)
    let pattern: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let data: Vec<u8> = pattern.iter().cycle().take(size).cloned().collect();
    Bytes::from(data)
}

/// Create a CacheEntry with the given data size
fn create_cache_entry(size: usize) -> CacheEntry {
    let data = generate_test_data(size);
    CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: format!("\"test-etag-{}\"", size),
        created_at: std::time::SystemTime::now(),
        expires_at: std::time::SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: std::time::SystemTime::now(),
    }
}

/// Create a cache key with realistic naming pattern
fn create_cache_key(bucket: &str, prefix: &str, index: usize) -> CacheKey {
    CacheKey {
        bucket: bucket.to_string(),
        object_key: format!("{}/file-{:06}.bin", prefix, index),
        etag: None,
    }
}

/// Create MemoryCache with standard benchmark configuration
fn create_memory_cache() -> MemoryCache {
    let config = MemoryCacheConfig {
        max_item_size_mb: 10,
        max_cache_size_mb: 512, // 512MB cache for benchmarks
        default_ttl_seconds: 3600,
    };
    MemoryCache::new(&config)
}

/// Create DiskCache with standard benchmark configuration
fn create_disk_cache(temp_dir: &TempDir) -> DiskCache {
    DiskCache::with_config(
        temp_dir.path().to_path_buf(),
        512 * 1024 * 1024, // 512MB cache
    )
}

// =============================================================================
// Memory Cache Benchmarks (Phase 36.2)
// =============================================================================

/// Benchmark memory cache set() operations across different sizes
fn bench_memory_cache_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_memory_cache();

    let mut group = c.benchmark_group("memory_cache_set");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    let sizes = [
        ("1kb", SIZE_1KB),
        ("10kb", SIZE_10KB),
        ("100kb", SIZE_100KB),
        ("1mb", SIZE_1MB),
    ];

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("size", name), &size, |b, &size| {
            let mut counter = 0u64;
            b.iter(|| {
                let key = create_cache_key("bench", "memory-set", counter as usize);
                counter = counter.wrapping_add(1);
                let entry = create_cache_entry(size);
                rt.block_on(async {
                    cache.set(black_box(key), black_box(entry)).await.unwrap();
                });
            });
        });
    }

    group.finish();
}

/// Benchmark memory cache get() operations (cache hits)
fn bench_memory_cache_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_memory_cache();

    // Pre-populate cache
    let sizes = [
        ("1kb", SIZE_1KB),
        ("10kb", SIZE_10KB),
        ("100kb", SIZE_100KB),
        ("1mb", SIZE_1MB),
    ];

    rt.block_on(async {
        for (name, size) in &sizes {
            for i in 0..100 {
                let key = create_cache_key("bench", &format!("memory-get-{}", name), i);
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("memory_cache_get");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("size", name), &name, |b, name| {
            let mut counter = 0usize;
            b.iter(|| {
                let key = create_cache_key("bench", &format!("memory-get-{}", name), counter % 100);
                counter += 1;
                rt.block_on(async {
                    let _result = cache.get(black_box(&key)).await;
                });
            });
        });
    }

    group.finish();
}

/// Benchmark memory cache miss performance
fn bench_memory_cache_miss(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_memory_cache();

    let mut group = c.benchmark_group("memory_cache_miss");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    group.bench_function("miss", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "nonexistent", counter);
            counter += 1;
            rt.block_on(async {
                let _result = cache.get(black_box(&key)).await;
            });
        });
    });

    group.finish();
}

// =============================================================================
// Disk Cache Benchmarks (Phase 36.3)
// =============================================================================

/// Benchmark disk cache set() operations across different sizes
fn bench_disk_cache_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = create_disk_cache(&temp_dir);

    let mut group = c.benchmark_group("disk_cache_set");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50); // Fewer samples for disk I/O

    let sizes = [
        ("1kb", SIZE_1KB),
        ("10kb", SIZE_10KB),
        ("100kb", SIZE_100KB),
    ];

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("size", name), &size, |b, &size| {
            let mut counter = 0u64;
            b.iter(|| {
                let key = create_cache_key("bench", "disk-set", counter as usize);
                counter = counter.wrapping_add(1);
                let entry = create_cache_entry(size);
                rt.block_on(async {
                    cache.set(black_box(key), black_box(entry)).await.unwrap();
                });
            });
        });
    }

    group.finish();
}

/// Benchmark disk cache get() operations (cache hits)
fn bench_disk_cache_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = create_disk_cache(&temp_dir);

    // Pre-populate cache
    let sizes = [
        ("1kb", SIZE_1KB),
        ("10kb", SIZE_10KB),
        ("100kb", SIZE_100KB),
    ];

    rt.block_on(async {
        for (name, size) in &sizes {
            for i in 0..50 {
                let key = create_cache_key("bench", &format!("disk-get-{}", name), i);
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("disk_cache_get");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("size", name), &name, |b, name| {
            let mut counter = 0usize;
            b.iter(|| {
                let key = create_cache_key("bench", &format!("disk-get-{}", name), counter % 50);
                counter += 1;
                rt.block_on(async {
                    let result = cache.get(black_box(&key)).await;
                    assert!(result.is_ok());
                });
            });
        });
    }

    group.finish();
}

// =============================================================================
// Comparative Benchmarks (Phase 36.6)
// =============================================================================

/// Direct comparison: Memory vs Disk cache for same operations
fn bench_cache_comparison(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let memory_cache = create_memory_cache();
    let temp_dir = TempDir::new().unwrap();
    let disk_cache = create_disk_cache(&temp_dir);

    // Pre-populate both caches
    rt.block_on(async {
        for i in 0..100 {
            let key = create_cache_key("bench", "compare", i);
            let entry = create_cache_entry(SIZE_10KB);
            memory_cache.set(key.clone(), entry.clone()).await.unwrap();
            disk_cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("cache_comparison_10kb");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    // Memory cache get
    group.bench_function("memory_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "compare", counter % 100);
            counter += 1;
            rt.block_on(async {
                memory_cache.get(black_box(&key)).await.unwrap();
            });
        });
    });

    // Disk cache get
    group.bench_function("disk_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "compare", counter % 100);
            counter += 1;
            rt.block_on(async {
                disk_cache.get(black_box(&key)).await.unwrap();
            });
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Configuration
// =============================================================================

criterion_group! {
    name = memory_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    targets = bench_memory_cache_set, bench_memory_cache_get, bench_memory_cache_miss
}

criterion_group! {
    name = disk_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .sample_size(50);
    targets = bench_disk_cache_set, bench_disk_cache_get
}

criterion_group! {
    name = comparison_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    targets = bench_cache_comparison
}

criterion_main!(memory_benches, disk_benches, comparison_benches);
