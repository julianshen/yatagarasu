use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tempfile::TempDir;
use tokio::runtime::Runtime;
use yatagarasu::cache::disk::DiskCache;
use yatagarasu::cache::{Cache, CacheEntry, CacheKey};

/// Create a CacheEntry with the given data size
fn create_cache_entry(size: usize) -> CacheEntry {
    let data = Bytes::from(vec![0u8; size]);
    CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: "test-etag".to_string(),
        created_at: std::time::SystemTime::now(),
        expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        last_accessed_at: std::time::SystemTime::now(),
    }
}

/// Benchmark small file (4KB) write operations
fn bench_small_file_write(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 100 * 1024 * 1024);

    let mut group = c.benchmark_group("disk_cache_small_file_write");

    group.bench_function("4kb_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("small-file-{}.bin", counter),
                etag: None,
            };
            counter += 1;

            let entry = create_cache_entry(4 * 1024);
            rt.block_on(async {
                cache.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark small file (4KB) read operations
fn bench_small_file_read(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 100 * 1024 * 1024);

    // Pre-populate cache with test files
    rt.block_on(async {
        for i in 0..100 {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("small-file-{}.bin", i),
                etag: None,
            };
            let entry = create_cache_entry(4 * 1024);
            cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("disk_cache_small_file_read");

    group.bench_function("4kb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("small-file-{}.bin", counter % 100),
                etag: None,
            };
            counter += 1;

            rt.block_on(async {
                let result = cache.get(black_box(&key)).await.unwrap();
                assert!(result.is_some());
            });
        });
    });

    group.finish();
}

/// Benchmark large file (10MB) write operations
fn bench_large_file_write(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 1024 * 1024 * 1024);

    let mut group = c.benchmark_group("disk_cache_large_file_write");
    group.sample_size(10); // Fewer samples for large files

    group.bench_function("10mb_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("large-file-{}.bin", counter),
                etag: None,
            };
            counter += 1;

            let entry = create_cache_entry(10 * 1024 * 1024);
            rt.block_on(async {
                cache.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark large file (10MB) read operations
fn bench_large_file_read(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 1024 * 1024 * 1024);

    // Pre-populate cache with large test files
    rt.block_on(async {
        for i in 0..10 {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("large-file-{}.bin", i),
                etag: None,
            };
            let entry = create_cache_entry(10 * 1024 * 1024);
            cache.set(key, entry).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("disk_cache_large_file_read");
    group.sample_size(10); // Fewer samples for large files

    group.bench_function("10mb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("large-file-{}.bin", counter % 10),
                etag: None,
            };
            counter += 1;

            rt.block_on(async {
                let result = cache.get(black_box(&key)).await.unwrap();
                assert!(result.is_some());
            });
        });
    });

    group.finish();
}

/// Benchmark cache operations with different file sizes
fn bench_mixed_file_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 1024 * 1024 * 1024);

    let file_sizes = vec![
        ("1kb", 1 * 1024),
        ("4kb", 4 * 1024),
        ("16kb", 16 * 1024),
        ("64kb", 64 * 1024),
        ("256kb", 256 * 1024),
        ("1mb", 1 * 1024 * 1024),
    ];

    // Pre-populate cache
    rt.block_on(async {
        for (name, size) in &file_sizes {
            for i in 0..10 {
                let key = CacheKey {
                    bucket: "bench-bucket".to_string(),
                    object_key: format!("{}-file-{}.bin", name, i),
                    etag: None,
                };
                let entry = create_cache_entry(*size);
                cache.set(key, entry).await.unwrap();
            }
        }
    });

    let mut group = c.benchmark_group("disk_cache_mixed_sizes");

    for (name, size) in file_sizes {
        group.bench_with_input(BenchmarkId::new("read", name), &size, |b, _| {
            let mut counter = 0;
            b.iter(|| {
                let key = CacheKey {
                    bucket: "bench-bucket".to_string(),
                    object_key: format!("{}-file-{}.bin", name, counter % 10),
                    etag: None,
                };
                counter += 1;

                rt.block_on(async {
                    let result = cache.get(black_box(&key)).await.unwrap();
                    assert!(result.is_some());
                });
            });
        });
    }

    group.finish();
}

/// Benchmark LRU eviction performance
fn bench_eviction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("disk_cache_eviction");

    // Small cache that triggers frequent evictions
    group.bench_function("eviction_small_cache", |b| {
        let temp_dir = TempDir::new().unwrap();
        // 10KB cache with 4KB entries = max 2-3 entries
        let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 10 * 1024);

        let mut counter = 0;
        b.iter(|| {
            let key = CacheKey {
                bucket: "bench-bucket".to_string(),
                object_key: format!("evict-file-{}.bin", counter),
                etag: None,
            };
            counter += 1;

            let entry = create_cache_entry(4 * 1024);
            rt.block_on(async {
                cache.set(black_box(key), black_box(entry)).await.unwrap();
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_small_file_write,
    bench_small_file_read,
    bench_large_file_write,
    bench_large_file_read,
    bench_mixed_file_sizes,
    bench_eviction,
);
criterion_main!(benches);
