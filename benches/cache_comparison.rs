//! Comprehensive Cache Comparison Benchmarks
//!
//! Phase 36: Comparative analysis of all cache implementations
//! - Memory (Moka)
//! - Disk (tokio::fs)
//! - Redis (redis crate + MessagePack)
//! - Tiered (Memory -> Disk -> Redis)
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all cache benchmarks
//! cargo bench --bench cache_comparison
//!
//! # Run specific benchmark group
//! cargo bench --bench cache_comparison -- "memory_cache"
//! cargo bench --bench cache_comparison -- "disk_cache"
//! cargo bench --bench cache_comparison -- "redis_cache"
//!
//! # Run single benchmark
//! cargo bench --bench cache_comparison -- "memory_cache_set/size/1kb"
//! ```
//!
//! # HTML Reports
//!
//! Criterion generates HTML reports with graphs automatically:
//! - Per-benchmark: `target/criterion/<group>/<bench>/report/index.html`
//! - Summary: `target/criterion/report/index.html`
//!
//! Reports include violin plots, regression analysis, and comparison with previous runs.
//!
//! # Requirements
//!
//! - Redis benchmarks require Docker (uses testcontainers)

use bytes::Bytes;
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use std::time::Duration;
use tempfile::TempDir;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::redis::Redis;
use tokio::runtime::Runtime;
use yatagarasu::cache::disk::DiskCache;
use yatagarasu::cache::redis::{RedisCache, RedisConfig};
use yatagarasu::cache::tiered::TieredCache;
use yatagarasu::cache::{Cache, CacheEntry, CacheKey, MemoryCache, MemoryCacheConfig};

// =============================================================================
// Benchmark Configuration Constants (Phase 36.1)
// =============================================================================

/// Warm-up duration for benchmarks
///
/// Criterion uses time-based warm-up rather than iteration-based.
/// 1 second ensures at least 5+ warm-up iterations even for slow disk I/O,
/// while fast memory operations get thousands of warm-up iterations.
const WARM_UP_TIME_SECS: u64 = 1;

/// Measurement duration for benchmarks
const MEASUREMENT_TIME_SECS: u64 = 5;

/// Sample size for memory-bound operations (fast)
const MEMORY_SAMPLE_SIZE: usize = 100;

/// Sample size for I/O-bound operations (slower, need fewer samples)
const DISK_SAMPLE_SIZE: usize = 50;

/// Sample size for Redis operations (network + I/O, need fewer samples)
const REDIS_SAMPLE_SIZE: usize = 30;

// =============================================================================
// Test Data Generation (Phase 36.1)
// =============================================================================

/// Standard test data sizes for benchmarking
const SIZE_1KB: usize = 1024;
const SIZE_10KB: usize = 10 * 1024;
const SIZE_100KB: usize = 100 * 1024;
const SIZE_1MB: usize = 1024 * 1024;
const SIZE_10MB: usize = 10 * 1024 * 1024;

/// Generate test data of specified size with realistic content
fn generate_test_data(size: usize) -> Bytes {
    // Generate semi-realistic data (not just zeros)
    let pattern: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let data: Vec<u8> = pattern.iter().cycle().take(size).cloned().collect();
    Bytes::from(data)
}

/// Content types for diverse testing
#[derive(Clone, Copy)]
enum ContentType {
    Binary,
    Json,
    Image,
}

impl ContentType {
    fn mime_type(&self) -> &'static str {
        match self {
            ContentType::Binary => "application/octet-stream",
            ContentType::Json => "application/json",
            ContentType::Image => "image/png",
        }
    }
}

/// Generate test data with specific content type characteristics
fn generate_typed_data(size: usize, content_type: ContentType) -> (Bytes, &'static str) {
    let data = match content_type {
        ContentType::Binary => {
            // Random-ish binary pattern (0-255 repeating)
            let pattern: Vec<u8> = (0..256).map(|i| i as u8).collect();
            pattern.iter().cycle().take(size).cloned().collect()
        }
        ContentType::Json => {
            // JSON-like structure (ASCII text with structure)
            let json_template = r#"{"id":12345,"name":"test-object","data":"#;
            let json_end = r#"","timestamp":1699999999}"#;
            let overhead = json_template.len() + json_end.len();
            let filler_size = size.saturating_sub(overhead);
            let filler: String = "x".repeat(filler_size);
            format!("{}{}{}", json_template, filler, json_end).into_bytes()
        }
        ContentType::Image => {
            // PNG-like header + random data (simulates image file)
            let png_header: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            let remaining = size.saturating_sub(png_header.len());
            let pattern: Vec<u8> = (0..256).map(|i| i as u8).collect();
            let body: Vec<u8> = pattern.iter().cycle().take(remaining).cloned().collect();
            [png_header, body].concat()
        }
    };
    (Bytes::from(data), content_type.mime_type())
}

/// Create a CacheEntry with specific content type
fn create_typed_cache_entry(size: usize, content_type: ContentType) -> CacheEntry {
    let (data, mime_type) = generate_typed_data(size, content_type);
    CacheEntry {
        data: data.clone(),
        content_type: mime_type.to_string(),
        content_length: data.len(),
        etag: format!("\"test-etag-{}-{}\"", size, mime_type.replace('/', "-")),
        last_modified: None,
        created_at: std::time::SystemTime::now(),
        expires_at: std::time::SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: std::time::SystemTime::now(),
    }
}

/// Create a CacheEntry with the given data size
fn create_cache_entry(size: usize) -> CacheEntry {
    let data = generate_test_data(size);
    CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: format!("\"test-etag-{}\"", size),
        last_modified: None,
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

/// Create RedisConfig for benchmark with given URL
fn create_redis_config(redis_url: String) -> RedisConfig {
    RedisConfig {
        redis_url: Some(redis_url),
        redis_password: None,
        redis_db: 0,
        redis_key_prefix: "bench".to_string(),
        redis_ttl_seconds: 3600,
        redis_max_ttl_seconds: 86400,
        connection_timeout_ms: 5000,
        operation_timeout_ms: 2000,
        min_pool_size: 1,
        max_pool_size: 10,
    }
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

/// Benchmark memory cache with diverse content types (JSON, Image, Binary)
fn bench_memory_cache_content_types(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_memory_cache();

    let mut group = c.benchmark_group("memory_cache_content_types");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    let content_types = [
        ("binary", ContentType::Binary),
        ("json", ContentType::Json),
        ("image", ContentType::Image),
    ];

    // Test 10KB entries with different content types
    for (name, content_type) in content_types {
        group.throughput(Throughput::Bytes(SIZE_10KB as u64));
        group.bench_with_input(BenchmarkId::new("set", name), &content_type, |b, &ct| {
            let mut counter = 0u64;
            b.iter(|| {
                let key =
                    create_cache_key("bench", &format!("content-type-{}", name), counter as usize);
                counter = counter.wrapping_add(1);
                let entry = create_typed_cache_entry(SIZE_10KB, ct);
                rt.block_on(async {
                    cache.set(black_box(key), black_box(entry)).await.unwrap();
                });
            });
        });
    }

    group.finish();
}

/// Number of concurrent threads for parallel benchmarks
const CONCURRENT_THREADS: usize = 10;

/// Benchmark memory cache concurrent get() operations (10 parallel)
fn bench_memory_cache_concurrent_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = std::sync::Arc::new(create_memory_cache());

    // Pre-populate cache with entries for each size
    let sizes = [("1kb", SIZE_1KB), ("100kb", SIZE_100KB), ("1mb", SIZE_1MB)];

    rt.block_on(async {
        for (name, size) in &sizes {
            for i in 0..100 {
                let key = create_cache_key("bench", &format!("concurrent-get-{}", name), i);
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("memory_cache_concurrent_get");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes((size * CONCURRENT_THREADS) as u64));
        group.bench_with_input(
            BenchmarkId::new("10_threads", name),
            &(name, size),
            |b, (name, _)| {
                let mut counter = 0usize;
                b.iter(|| {
                    let base_counter = counter;
                    counter += CONCURRENT_THREADS;

                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(CONCURRENT_THREADS);
                        for i in 0..CONCURRENT_THREADS {
                            let cache = cache.clone();
                            let key = create_cache_key(
                                "bench",
                                &format!("concurrent-get-{}", name),
                                (base_counter + i) % 100,
                            );
                            handles.push(tokio::spawn(async move {
                                let _result = cache.get(black_box(&key)).await;
                            }));
                        }
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory cache concurrent set() operations (10 parallel)
fn bench_memory_cache_concurrent_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = std::sync::Arc::new(create_memory_cache());

    let sizes = [("1kb", SIZE_1KB), ("100kb", SIZE_100KB)];

    let mut group = c.benchmark_group("memory_cache_concurrent_set");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes((size * CONCURRENT_THREADS) as u64));
        group.bench_with_input(BenchmarkId::new("10_threads", name), &size, |b, &size| {
            let mut counter = 0u64;
            b.iter(|| {
                let base_counter = counter;
                counter += CONCURRENT_THREADS as u64;

                rt.block_on(async {
                    let mut handles = Vec::with_capacity(CONCURRENT_THREADS);
                    for i in 0..CONCURRENT_THREADS {
                        let cache = cache.clone();
                        let key = create_cache_key(
                            "bench",
                            "concurrent-set",
                            (base_counter + i as u64) as usize,
                        );
                        let entry = create_cache_entry(size);
                        handles.push(tokio::spawn(async move {
                            cache.set(black_box(key), black_box(entry)).await.unwrap();
                        }));
                    }
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            });
        });
    }

    group.finish();
}

/// Benchmark memory cache mixed workload (70% read, 30% write)
fn bench_memory_cache_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = std::sync::Arc::new(create_memory_cache());

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..1000 {
            let key = create_cache_key("bench", "mixed", i);
            let entry = create_cache_entry(SIZE_1KB);
            cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("memory_cache_mixed_workload");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Elements(10)); // 10 operations per iteration

    group.bench_function("70_read_30_write", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            rt.block_on(async {
                // 10 operations: 7 reads, 3 writes
                for i in 0..10 {
                    let idx = (counter + i) % 1000;
                    if i < 7 {
                        // Read (70%)
                        let key = create_cache_key("bench", "mixed", idx);
                        let _result = cache.get(black_box(&key)).await;
                    } else {
                        // Write (30%)
                        let key = create_cache_key("bench", "mixed", idx);
                        let entry = create_cache_entry(SIZE_1KB);
                        cache.set(black_box(key), black_box(entry)).await.unwrap();
                    }
                }
            });
            counter += 10;
        });
    });

    group.finish();
}

/// Create a small MemoryCache for eviction testing (fits ~1000 1KB entries)
fn create_small_memory_cache() -> MemoryCache {
    let config = MemoryCacheConfig {
        max_item_size_mb: 1,
        max_cache_size_mb: 1, // 1MB = ~1000 1KB entries
        default_ttl_seconds: 3600,
    };
    MemoryCache::new(&config)
}

/// Benchmark LRU eviction performance when cache is full
fn bench_memory_cache_eviction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_cache_eviction");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    // Test eviction with 1000 entries (1KB each, 1MB cache)
    group.bench_function("eviction_1000_entries", |b| {
        let cache = create_small_memory_cache();

        // Pre-fill cache to capacity
        rt.block_on(async {
            for i in 0..1000 {
                let key = create_cache_key("bench", "evict-1k", i);
                let entry = create_cache_entry(SIZE_1KB);
                cache.set(key, entry).await.unwrap();
            }
        });

        let mut counter = 1000usize;
        b.iter(|| {
            // Each insert triggers eviction since cache is full
            let key = create_cache_key("bench", "evict-1k", counter);
            counter += 1;
            let entry = create_cache_entry(SIZE_1KB);
            rt.block_on(async {
                cache.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    // Test eviction with 10000 entries (100B each to fit in 1MB cache)
    group.bench_function("eviction_10000_entries", |b| {
        // Use 100-byte entries to fit 10000 in ~1MB
        let config = MemoryCacheConfig {
            max_item_size_mb: 1,
            max_cache_size_mb: 1,
            default_ttl_seconds: 3600,
        };
        let cache = MemoryCache::new(&config);

        // Pre-fill cache with smaller entries
        rt.block_on(async {
            for i in 0..10000 {
                let key = create_cache_key("bench", "evict-10k", i);
                let entry = create_cache_entry(100); // 100 bytes
                cache.set(key, entry).await.unwrap();
            }
        });

        let mut counter = 10000usize;
        b.iter(|| {
            let key = create_cache_key("bench", "evict-10k", counter);
            counter += 1;
            let entry = create_cache_entry(100);
            rt.block_on(async {
                cache.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark memory cache delete() operations
fn bench_memory_cache_delete(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = create_memory_cache();

    let mut group = c.benchmark_group("memory_cache_delete");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    // Pre-populate cache for deletion tests
    rt.block_on(async {
        for i in 0..10000 {
            let key = create_cache_key("bench", "delete", i);
            let entry = create_cache_entry(SIZE_1KB);
            cache.set(key, entry).await.unwrap();
        }
    });

    // Benchmark delete of existing entries
    group.bench_function("existing_entry", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "delete", counter % 10000);
            counter += 1;
            rt.block_on(async {
                let _ = cache.delete(black_box(&key)).await;
            });
        });
    });

    // Benchmark delete of non-existing entries (miss case)
    group.bench_function("nonexistent_entry", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "delete-miss", counter);
            counter += 1;
            rt.block_on(async {
                let _ = cache.delete(black_box(&key)).await;
            });
        });
    });

    group.finish();
}

/// Benchmark CacheKey generation and hashing performance
/// This measures the overhead of creating cache keys for lookups
fn bench_cache_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_key_generation");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    // Benchmark key creation with short paths
    group.bench_function("short_path", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "obj", counter);
            counter += 1;
            black_box(key);
        });
    });

    // Benchmark key creation with long paths (common S3 pattern)
    group.bench_function("long_path", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = CacheKey {
                bucket: "my-production-bucket".to_string(),
                object_key: format!(
                    "users/12345/documents/2024/01/15/reports/quarterly-{:06}.pdf",
                    counter
                ),
                etag: None,
            };
            counter += 1;
            black_box(key);
        });
    });

    // Benchmark key Display (string conversion for hashing/storage)
    group.bench_function("display_conversion", |b| {
        let key = CacheKey {
            bucket: "my-production-bucket".to_string(),
            object_key: "users/12345/documents/2024/01/15/reports/quarterly-report.pdf".to_string(),
            etag: None,
        };
        b.iter(|| {
            let s = black_box(key.to_string());
            black_box(s);
        });
    });

    // Benchmark key hash computation (used internally by Moka)
    group.bench_function("hash_computation", |b| {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let key = CacheKey {
            bucket: "my-production-bucket".to_string(),
            object_key: "users/12345/documents/2024/01/15/reports/quarterly-report.pdf".to_string(),
            etag: None,
        };
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            black_box(hasher.finish());
        });
    });

    group.finish();
}

/// Benchmark throughput (operations per second)
fn bench_memory_cache_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = std::sync::Arc::new(create_memory_cache());

    // Pre-populate for get operations
    rt.block_on(async {
        for i in 0..10000 {
            let key = create_cache_key("bench", "throughput", i);
            let entry = create_cache_entry(SIZE_1KB);
            cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("memory_cache_throughput");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Elements(1));

    // Sequential operations baseline
    group.bench_function("sequential_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "throughput", counter % 10000);
            counter += 1;
            rt.block_on(async {
                let _result = cache.get(black_box(&key)).await;
            });
        });
    });

    // 10 concurrent operations
    group.throughput(Throughput::Elements(10));
    group.bench_function("concurrent_10_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let base = counter;
            counter += 10;
            rt.block_on(async {
                let mut handles = Vec::with_capacity(10);
                for i in 0..10 {
                    let cache = cache.clone();
                    let key = create_cache_key("bench", "throughput", (base + i) % 10000);
                    handles.push(tokio::spawn(async move {
                        let _result = cache.get(black_box(&key)).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    // 100 concurrent operations
    group.throughput(Throughput::Elements(100));
    group.bench_function("concurrent_100_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let base = counter;
            counter += 100;
            rt.block_on(async {
                let mut handles = Vec::with_capacity(100);
                for i in 0..100 {
                    let cache = cache.clone();
                    let key = create_cache_key("bench", "throughput", (base + i) % 10000);
                    handles.push(tokio::spawn(async move {
                        let _result = cache.get(black_box(&key)).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    group.finish();
}

// =============================================================================
// Hit Rate Validation Benchmarks (Deferred from Phase 27.10)
// =============================================================================

/// Generate Zipfian-distributed index for realistic cache access patterns
/// Zipfian distribution: P(k) ∝ 1/k^s where s is the skew parameter
/// Higher skew = more concentration on popular items
fn zipfian_index(n: usize, skew: f64, uniform_random: f64) -> usize {
    // Approximate Zipfian using inverse CDF
    // For simplicity, use a power-law approximation
    let rank = (uniform_random.powf(1.0 / (1.0 - skew)) * n as f64) as usize;
    rank.min(n - 1)
}

/// Simple pseudo-random number generator for benchmarks (deterministic)
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as f64) / (u64::MAX as f64)
    }
}

/// Benchmark hit rate under Zipfian access pattern
/// Target: >80% hit rate with Zipfian skew=0.99
fn bench_memory_cache_hit_rate_zipfian(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_cache_hit_rate");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Elements(100)); // 100 accesses per iteration

    // Small cache (100 items) with larger working set (1000 items)
    // Zipfian distribution should achieve >80% hit rate
    group.bench_function("zipfian_skew_0.99", |b| {
        // Using standard memory cache config (512MB)
        // TinyLFU admission policy handles Zipfian access well
        let cache = create_memory_cache();
        let mut rng = SimpleRng::new(42);

        // Pre-populate with some entries
        rt.block_on(async {
            for i in 0..100 {
                let key = create_cache_key("bench", "zipf", i);
                let entry = create_cache_entry(SIZE_1KB);
                cache.set(key, entry).await.unwrap();
            }
        });

        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    // Zipfian distribution with skew 0.99
                    let idx = zipfian_index(1000, 0.99, rng.next_f64());
                    let key = create_cache_key("bench", "zipf", idx);

                    if cache.get(black_box(&key)).await.is_none() {
                        // Cache miss - insert the entry
                        let entry = create_cache_entry(SIZE_1KB);
                        cache.set(key, entry).await.unwrap();
                    }
                }
            });
        });
    });

    // Lower skew (more uniform distribution) - lower hit rate expected
    group.bench_function("zipfian_skew_0.7", |b| {
        let cache = create_memory_cache();
        let mut rng = SimpleRng::new(42);

        rt.block_on(async {
            for i in 0..100 {
                let key = create_cache_key("bench", "zipf-low", i);
                let entry = create_cache_entry(SIZE_1KB);
                cache.set(key, entry).await.unwrap();
            }
        });

        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    let idx = zipfian_index(1000, 0.7, rng.next_f64());
                    let key = create_cache_key("bench", "zipf-low", idx);

                    if cache.get(black_box(&key)).await.is_none() {
                        let entry = create_cache_entry(SIZE_1KB);
                        cache.set(key, entry).await.unwrap();
                    }
                }
            });
        });
    });

    group.finish();
}

/// Benchmark hit rate adaptation when access pattern changes (hot set rotation)
fn bench_memory_cache_hit_rate_adaptation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_cache_adaptation");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Elements(100));

    // Test TinyLFU's ability to adapt to changing access patterns
    group.bench_function("hot_set_rotation", |b| {
        let cache = create_memory_cache();
        let mut rng = SimpleRng::new(42);
        let mut phase = 0usize;

        // Pre-populate cache
        rt.block_on(async {
            for i in 0..100 {
                let key = create_cache_key("bench", "adapt", i);
                let entry = create_cache_entry(SIZE_1KB);
                cache.set(key, entry).await.unwrap();
            }
        });

        b.iter(|| {
            rt.block_on(async {
                // Rotate hot set every iteration
                let hot_set_start = (phase % 10) * 100;
                phase += 1;

                for _ in 0..100 {
                    // Access from current hot set with some noise
                    let base_idx = if rng.next_f64() < 0.8 {
                        hot_set_start // 80% from current hot set
                    } else {
                        0 // 20% from other items
                    };
                    let idx = base_idx + (rng.next_f64() * 100.0) as usize;
                    let key = create_cache_key("bench", "adapt", idx % 1000);

                    if cache.get(black_box(&key)).await.is_none() {
                        let entry = create_cache_entry(SIZE_1KB);
                        cache.set(key, entry).await.unwrap();
                    }
                }
            });
        });
    });

    group.finish();
}

/// Benchmark large file operations (10MB - exceeds typical cache limit)
fn bench_large_file_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let disk_cache = create_disk_cache(&temp_dir);

    let mut group = c.benchmark_group("large_file_10mb");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(10));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10); // Very few samples for large files

    group.throughput(Throughput::Bytes(SIZE_10MB as u64));

    // Disk cache handles large files
    group.bench_function("disk_set", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = create_cache_key("bench", "large-file", counter as usize);
            counter = counter.wrapping_add(1);
            let entry = create_cache_entry(SIZE_10MB);
            rt.block_on(async {
                disk_cache
                    .set(black_box(key), black_box(entry))
                    .await
                    .unwrap();
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
        ("1mb", SIZE_1MB),
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
        ("1mb", SIZE_1MB),
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

/// Benchmark disk cache concurrent get() operations (10 parallel)
fn bench_disk_cache_concurrent_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = std::sync::Arc::new(create_disk_cache(&temp_dir));

    // Pre-populate cache
    let sizes = [("1kb", SIZE_1KB), ("100kb", SIZE_100KB)];

    rt.block_on(async {
        for (name, size) in &sizes {
            for i in 0..50 {
                let key = create_cache_key("bench", &format!("disk-concurrent-{}", name), i);
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("disk_cache_concurrent_get");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(30);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes((size * CONCURRENT_THREADS) as u64));
        group.bench_with_input(
            BenchmarkId::new("10_threads", name),
            &(name, size),
            |b, (name, _)| {
                let mut counter = 0usize;
                b.iter(|| {
                    let base = counter;
                    counter += CONCURRENT_THREADS;
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(CONCURRENT_THREADS);
                        for i in 0..CONCURRENT_THREADS {
                            let cache = cache.clone();
                            let key = create_cache_key(
                                "bench",
                                &format!("disk-concurrent-{}", name),
                                (base + i) % 50,
                            );
                            handles.push(tokio::spawn(async move {
                                let _result = cache.get(black_box(&key)).await;
                            }));
                        }
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark disk cache concurrent set() operations (10 parallel)
fn bench_disk_cache_concurrent_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = std::sync::Arc::new(create_disk_cache(&temp_dir));

    let sizes = [("1kb", SIZE_1KB), ("100kb", SIZE_100KB)];

    let mut group = c.benchmark_group("disk_cache_concurrent_set");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(30);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes((size * CONCURRENT_THREADS) as u64));
        group.bench_with_input(BenchmarkId::new("10_threads", name), &size, |b, &size| {
            let mut counter = 0u64;
            b.iter(|| {
                let base = counter;
                counter += CONCURRENT_THREADS as u64;
                rt.block_on(async {
                    let mut handles = Vec::with_capacity(CONCURRENT_THREADS);
                    for i in 0..CONCURRENT_THREADS {
                        let cache = cache.clone();
                        let key = create_cache_key(
                            "bench",
                            "disk-concurrent-set",
                            (base + i as u64) as usize,
                        );
                        let entry = create_cache_entry(size);
                        handles.push(tokio::spawn(async move {
                            cache.set(black_box(key), black_box(entry)).await.unwrap();
                        }));
                    }
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            });
        });
    }

    group.finish();
}

/// Benchmark disk cache throughput (operations per second)
fn bench_disk_cache_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = std::sync::Arc::new(create_disk_cache(&temp_dir));

    // Pre-populate for get operations
    rt.block_on(async {
        for i in 0..1000 {
            let key = create_cache_key("bench", "disk-throughput", i);
            let entry = create_cache_entry(SIZE_1KB);
            cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("disk_cache_throughput");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);

    // Sequential operations baseline
    group.throughput(Throughput::Elements(1));
    group.bench_function("sequential_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "disk-throughput", counter % 1000);
            counter += 1;
            rt.block_on(async {
                let _result = cache.get(black_box(&key)).await;
            });
        });
    });

    // 10 concurrent operations
    group.throughput(Throughput::Elements(10));
    group.bench_function("concurrent_10_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let base = counter;
            counter += 10;
            rt.block_on(async {
                let mut handles = Vec::with_capacity(10);
                for i in 0..10 {
                    let cache = cache.clone();
                    let key = create_cache_key("bench", "disk-throughput", (base + i) % 1000);
                    handles.push(tokio::spawn(async move {
                        let _result = cache.get(black_box(&key)).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    // 100 concurrent operations (stress test for load testing feasibility)
    group.throughput(Throughput::Elements(100));
    group.bench_function("concurrent_100_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let base = counter;
            counter += 100;
            rt.block_on(async {
                let mut handles = Vec::with_capacity(100);
                for i in 0..100 {
                    let cache = cache.clone();
                    let key = create_cache_key("bench", "disk-throughput", (base + i) % 1000);
                    handles.push(tokio::spawn(async move {
                        let _result = cache.get(black_box(&key)).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    group.finish();
}

// =============================================================================
// Redis Cache Benchmarks (Phase 36.4) - Using Testcontainers
// =============================================================================

/// Benchmark Redis cache set() operations across different sizes
///
/// Uses testcontainers to spin up a real Redis instance.
/// Container lifetime is managed within the benchmark function.
fn bench_redis_cache_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Start Redis container (lives for duration of this benchmark)
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create Redis cache
    let config = create_redis_config(redis_url);
    let cache = rt.block_on(async { RedisCache::new(config).await.unwrap() });

    let mut group = c.benchmark_group("redis_cache_set");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);

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
                let key = create_cache_key("bench", "redis-set", counter as usize);
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

/// Benchmark Redis cache get() operations (cache hits)
fn bench_redis_cache_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = create_redis_config(redis_url);
    let cache = rt.block_on(async { RedisCache::new(config).await.unwrap() });

    // Pre-populate cache
    let sizes = [
        ("1kb", SIZE_1KB),
        ("10kb", SIZE_10KB),
        ("100kb", SIZE_100KB),
    ];

    rt.block_on(async {
        for (name, size) in &sizes {
            for i in 0..30 {
                let key = create_cache_key("bench", &format!("redis-get-{}", name), i);
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("redis_cache_get");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("size", name), &name, |b, name| {
            let mut counter = 0usize;
            b.iter(|| {
                let key = create_cache_key("bench", &format!("redis-get-{}", name), counter % 30);
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

/// Benchmark Redis cache concurrent get() operations (10 parallel)
fn bench_redis_cache_concurrent_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = create_redis_config(redis_url);
    let cache = rt.block_on(async { std::sync::Arc::new(RedisCache::new(config).await.unwrap()) });

    // Pre-populate cache
    let sizes = [("1kb", SIZE_1KB), ("100kb", SIZE_100KB)];

    rt.block_on(async {
        for (name, size) in &sizes {
            for i in 0..30 {
                let key = create_cache_key("bench", &format!("redis-concurrent-{}", name), i);
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("redis_cache_concurrent_get");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes((size * CONCURRENT_THREADS) as u64));
        group.bench_with_input(
            BenchmarkId::new("10_threads", name),
            &(name, size),
            |b, (name, _)| {
                let mut counter = 0usize;
                b.iter(|| {
                    let base = counter;
                    counter += CONCURRENT_THREADS;
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(CONCURRENT_THREADS);
                        for i in 0..CONCURRENT_THREADS {
                            let cache = cache.clone();
                            let key = create_cache_key(
                                "bench",
                                &format!("redis-concurrent-{}", name),
                                (base + i) % 30,
                            );
                            handles.push(tokio::spawn(async move {
                                let _result = cache.get(black_box(&key)).await;
                            }));
                        }
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Redis cache concurrent set() operations (10 parallel)
fn bench_redis_cache_concurrent_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = create_redis_config(redis_url);
    let cache = rt.block_on(async { std::sync::Arc::new(RedisCache::new(config).await.unwrap()) });

    let sizes = [("1kb", SIZE_1KB), ("100kb", SIZE_100KB)];

    let mut group = c.benchmark_group("redis_cache_concurrent_set");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes((size * CONCURRENT_THREADS) as u64));
        group.bench_with_input(BenchmarkId::new("10_threads", name), &size, |b, &size| {
            let mut counter = 0u64;
            b.iter(|| {
                let base = counter;
                counter += CONCURRENT_THREADS as u64;
                rt.block_on(async {
                    let mut handles = Vec::with_capacity(CONCURRENT_THREADS);
                    for i in 0..CONCURRENT_THREADS {
                        let cache = cache.clone();
                        let key = create_cache_key(
                            "bench",
                            "redis-concurrent-set",
                            (base + i as u64) as usize,
                        );
                        let entry = create_cache_entry(size);
                        handles.push(tokio::spawn(async move {
                            cache.set(black_box(key), black_box(entry)).await.unwrap();
                        }));
                    }
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            });
        });
    }

    group.finish();
}

/// Benchmark Redis cache throughput (operations per second)
fn bench_redis_cache_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let config = create_redis_config(redis_url);
    let cache = rt.block_on(async { std::sync::Arc::new(RedisCache::new(config).await.unwrap()) });

    // Pre-populate for get operations
    rt.block_on(async {
        for i in 0..100 {
            let key = create_cache_key("bench", "redis-throughput", i);
            let entry = create_cache_entry(SIZE_1KB);
            cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("redis_cache_throughput");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);

    // Sequential operations baseline
    group.throughput(Throughput::Elements(1));
    group.bench_function("sequential_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "redis-throughput", counter % 100);
            counter += 1;
            rt.block_on(async {
                let _result = cache.get(black_box(&key)).await;
            });
        });
    });

    // 10 concurrent operations
    group.throughput(Throughput::Elements(10));
    group.bench_function("concurrent_10_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let base = counter;
            counter += 10;
            rt.block_on(async {
                let mut handles = Vec::with_capacity(10);
                for i in 0..10 {
                    let cache = cache.clone();
                    let key = create_cache_key("bench", "redis-throughput", (base + i) % 100);
                    handles.push(tokio::spawn(async move {
                        let _result = cache.get(black_box(&key)).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    // 100 concurrent operations (stress test for load testing feasibility)
    group.throughput(Throughput::Elements(100));
    group.bench_function("concurrent_100_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let base = counter;
            counter += 100;
            rt.block_on(async {
                let mut handles = Vec::with_capacity(100);
                for i in 0..100 {
                    let cache = cache.clone();
                    let key = create_cache_key("bench", "redis-throughput", (base + i) % 100);
                    handles.push(tokio::spawn(async move {
                        let _result = cache.get(black_box(&key)).await;
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    group.finish();
}

// =============================================================================
// Comparative Benchmarks (Phase 36.6)
// =============================================================================

/// Direct comparison: Memory vs Disk vs Redis cache for same operations
fn bench_cache_comparison(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let memory_cache = create_memory_cache();
    let temp_dir = TempDir::new().unwrap();
    let disk_cache = create_disk_cache(&temp_dir);

    // Start Redis container for comparison
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    let redis_config = create_redis_config(redis_url);
    let redis_cache = rt.block_on(async { RedisCache::new(redis_config).await.unwrap() });

    // Pre-populate all caches
    rt.block_on(async {
        for i in 0..100 {
            let key = create_cache_key("bench", "compare", i);
            let entry = create_cache_entry(SIZE_10KB);
            memory_cache.set(key.clone(), entry.clone()).await.unwrap();
            disk_cache.set(key.clone(), entry.clone()).await.unwrap();
            redis_cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("cache_comparison_10kb");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    // Memory cache get (fastest)
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

    // Disk cache get (medium)
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

    // Redis cache get (slowest but distributed)
    group.bench_function("redis_get", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "compare", counter % 100);
            counter += 1;
            rt.block_on(async {
                let _result = redis_cache.get(black_box(&key)).await;
            });
        });
    });

    group.finish();
}

/// Compare set operations across all cache types
fn bench_cache_comparison_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let memory_cache = create_memory_cache();
    let temp_dir = TempDir::new().unwrap();
    let disk_cache = create_disk_cache(&temp_dir);

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);
    let redis_config = create_redis_config(redis_url);
    let redis_cache = rt.block_on(async { RedisCache::new(redis_config).await.unwrap() });

    let mut group = c.benchmark_group("cache_comparison_set_10kb");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    // Memory cache set (fastest)
    group.bench_function("memory_set", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = create_cache_key("bench", "compare-set", counter as usize);
            counter = counter.wrapping_add(1);
            let entry = create_cache_entry(SIZE_10KB);
            rt.block_on(async {
                memory_cache
                    .set(black_box(key), black_box(entry))
                    .await
                    .unwrap();
            });
        });
    });

    // Disk cache set (medium)
    group.bench_function("disk_set", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = create_cache_key("bench", "compare-set", counter as usize);
            counter = counter.wrapping_add(1);
            let entry = create_cache_entry(SIZE_10KB);
            rt.block_on(async {
                disk_cache
                    .set(black_box(key), black_box(entry))
                    .await
                    .unwrap();
            });
        });
    });

    // Redis cache set (slowest but distributed)
    group.bench_function("redis_set", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = create_cache_key("bench", "compare-set", counter as usize);
            counter = counter.wrapping_add(1);
            let entry = create_cache_entry(SIZE_10KB);
            rt.block_on(async {
                redis_cache
                    .set(black_box(key), black_box(entry))
                    .await
                    .unwrap();
            });
        });
    });

    group.finish();
}

// =============================================================================
// Tiered Cache Benchmarks (Phase 36.5)
// =============================================================================

/// Create a TieredCache with Memory + Disk layers
fn create_tiered_cache_memory_disk(temp_dir: &TempDir) -> TieredCache {
    let memory_cache = create_memory_cache();
    let disk_cache = create_disk_cache(temp_dir);
    TieredCache::new(vec![Box::new(memory_cache), Box::new(disk_cache)])
}

/// Benchmark tiered cache L1 hit (memory layer)
/// Entry exists in memory - fastest path
fn bench_tiered_cache_l1_hit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let tiered = create_tiered_cache_memory_disk(&temp_dir);

    // Pre-populate memory layer with entries
    rt.block_on(async {
        for i in 0..100 {
            let key = create_cache_key("bench", "tiered-l1", i);
            let entry = create_cache_entry(SIZE_10KB);
            tiered.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("tiered_cache_l1_hit");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    group.bench_function("memory_hit", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "tiered-l1", counter % 100);
            counter += 1;
            rt.block_on(async {
                let result = tiered.get(black_box(&key)).await;
                assert!(result.unwrap().is_some());
            });
        });
    });

    group.finish();
}

/// Benchmark tiered cache L2 hit (disk layer, memory miss)
/// Entry exists in disk but not in memory - triggers promotion
fn bench_tiered_cache_l2_hit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Create separate caches to control where data exists
    let memory_cache = create_memory_cache();
    let disk_cache = create_disk_cache(&temp_dir);

    // Pre-populate DISK only (not memory)
    rt.block_on(async {
        for i in 0..50 {
            let key = create_cache_key("bench", "tiered-l2", i);
            let entry = create_cache_entry(SIZE_10KB);
            disk_cache.set(key, entry).await.unwrap();
        }
    });

    // Create tiered cache with populated disk layer
    let tiered = TieredCache::new(vec![Box::new(memory_cache), Box::new(disk_cache)]);

    let mut group = c.benchmark_group("tiered_cache_l2_hit");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    group.bench_function("disk_hit_with_promotion", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "tiered-l2", counter % 50);
            counter += 1;
            rt.block_on(async {
                let result = tiered.get(black_box(&key)).await;
                assert!(result.unwrap().is_some());
            });
        });
    });

    group.finish();
}

/// Benchmark tiered cache L3 hit (redis layer, memory+disk miss)
/// Entry exists only in Redis - triggers promotion to memory and disk
fn bench_tiered_cache_l3_hit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    // Create separate caches
    let memory_cache = create_memory_cache();
    let disk_cache = create_disk_cache(&temp_dir);
    let redis_config = create_redis_config(redis_url);
    let redis_cache = rt.block_on(async { RedisCache::new(redis_config).await.unwrap() });

    // Pre-populate REDIS only (not memory or disk)
    rt.block_on(async {
        for i in 0..30 {
            let key = create_cache_key("bench", "tiered-l3", i);
            let entry = create_cache_entry(SIZE_10KB);
            redis_cache.set(key, entry).await.unwrap();
        }
    });

    // Create 3-layer tiered cache
    let tiered = TieredCache::new(vec![
        Box::new(memory_cache),
        Box::new(disk_cache),
        Box::new(redis_cache),
    ]);

    let mut group = c.benchmark_group("tiered_cache_l3_hit");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    group.bench_function("redis_hit_with_promotion", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "tiered-l3", counter % 30);
            counter += 1;
            rt.block_on(async {
                let result = tiered.get(black_box(&key)).await;
                assert!(result.unwrap().is_some());
            });
        });
    });

    group.finish();
}

/// Benchmark tiered cache miss (all layers miss)
fn bench_tiered_cache_miss(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let tiered = create_tiered_cache_memory_disk(&temp_dir);

    let mut group = c.benchmark_group("tiered_cache_miss");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);

    group.bench_function("all_layers_miss", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "nonexistent", counter);
            counter += 1;
            rt.block_on(async {
                let result = tiered.get(black_box(&key)).await;
                assert!(result.unwrap().is_none());
            });
        });
    });

    group.finish();
}

/// Benchmark tiered cache set (write-through to all layers)
fn bench_tiered_cache_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let tiered = create_tiered_cache_memory_disk(&temp_dir);

    let mut group = c.benchmark_group("tiered_cache_set");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    group.bench_function("write_through_2_layers", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = create_cache_key("bench", "tiered-set", counter as usize);
            counter = counter.wrapping_add(1);
            let entry = create_cache_entry(SIZE_10KB);
            rt.block_on(async {
                tiered.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark tiered cache set with 3 layers (memory + disk + redis)
fn bench_tiered_cache_set_3_layers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Start Redis container
    let docker = Cli::default();
    let redis_image = RunnableImage::from(Redis);
    let redis_container = docker.run(redis_image);
    let redis_port = redis_container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://127.0.0.1:{}", redis_port);

    let memory_cache = create_memory_cache();
    let disk_cache = create_disk_cache(&temp_dir);
    let redis_config = create_redis_config(redis_url);
    let redis_cache = rt.block_on(async { RedisCache::new(redis_config).await.unwrap() });

    let tiered = TieredCache::new(vec![
        Box::new(memory_cache),
        Box::new(disk_cache),
        Box::new(redis_cache),
    ]);

    let mut group = c.benchmark_group("tiered_cache_set_3_layers");
    group.warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS));
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(REDIS_SAMPLE_SIZE);
    group.throughput(Throughput::Bytes(SIZE_10KB as u64));

    group.bench_function("write_through_all", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = create_cache_key("bench", "tiered-set-3", counter as usize);
            counter = counter.wrapping_add(1);
            let entry = create_cache_entry(SIZE_10KB);
            rt.block_on(async {
                tiered.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark tiered cache delete (remove from all layers)
fn bench_tiered_cache_delete(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let tiered = create_tiered_cache_memory_disk(&temp_dir);

    // Pre-populate for delete
    rt.block_on(async {
        for i in 0..1000 {
            let key = create_cache_key("bench", "tiered-delete", i);
            let entry = create_cache_entry(SIZE_1KB);
            tiered.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("tiered_cache_delete");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);

    group.bench_function("delete_from_all_layers", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = create_cache_key("bench", "tiered-delete", counter % 1000);
            counter += 1;
            rt.block_on(async {
                let _ = tiered.delete(black_box(&key)).await;
            });
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Configuration (Phase 36.1: Statistical Rigor)
// =============================================================================

/// Create a Criterion configuration with statistical rigor settings
///
/// Statistical configuration:
/// - 95% confidence intervals for timing estimates
/// - 5% significance level for regression detection
/// - 1% noise threshold to filter measurement noise
/// - 100,000 bootstrap resamples for robust statistics
fn statistically_rigorous_config() -> Criterion {
    Criterion::default()
        .confidence_level(0.95) // 95% confidence intervals
        .significance_level(0.05) // 5% significance for regression detection
        .noise_threshold(0.01) // Ignore <1% changes (measurement noise)
        .nresamples(100_000) // Bootstrap resamples for statistics
}

criterion_group! {
    name = memory_benches;
    config = statistically_rigorous_config()
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .sample_size(MEMORY_SAMPLE_SIZE);
    targets = bench_memory_cache_set,
              bench_memory_cache_get,
              bench_memory_cache_miss,
              bench_memory_cache_content_types,
              bench_memory_cache_concurrent_get,
              bench_memory_cache_concurrent_set,
              bench_memory_cache_mixed_workload,
              bench_memory_cache_eviction,
              bench_memory_cache_delete,
              bench_cache_key_generation,
              bench_memory_cache_throughput,
              bench_memory_cache_hit_rate_zipfian,
              bench_memory_cache_hit_rate_adaptation
}

criterion_group! {
    name = disk_benches;
    config = statistically_rigorous_config()
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .sample_size(DISK_SAMPLE_SIZE);
    targets = bench_disk_cache_set,
              bench_disk_cache_get,
              bench_disk_cache_concurrent_get,
              bench_disk_cache_concurrent_set,
              bench_disk_cache_throughput,
              bench_large_file_operations
}

criterion_group! {
    name = redis_benches;
    config = statistically_rigorous_config()
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .sample_size(REDIS_SAMPLE_SIZE);
    targets = bench_redis_cache_set,
              bench_redis_cache_get,
              bench_redis_cache_concurrent_get,
              bench_redis_cache_concurrent_set,
              bench_redis_cache_throughput
}

criterion_group! {
    name = comparison_benches;
    config = statistically_rigorous_config()
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .sample_size(MEMORY_SAMPLE_SIZE);
    targets = bench_cache_comparison, bench_cache_comparison_set
}

criterion_group! {
    name = tiered_benches;
    config = statistically_rigorous_config()
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .sample_size(DISK_SAMPLE_SIZE);
    targets = bench_tiered_cache_l1_hit,
              bench_tiered_cache_l2_hit,
              bench_tiered_cache_l3_hit,
              bench_tiered_cache_miss,
              bench_tiered_cache_set,
              bench_tiered_cache_set_3_layers,
              bench_tiered_cache_delete
}

criterion_main!(
    memory_benches,
    disk_benches,
    redis_benches,
    comparison_benches,
    tiered_benches
);
