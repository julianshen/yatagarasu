//! Backend performance comparison benchmarks
//!
//! Compares tokio::fs (portable) vs tokio-uring (Linux only)
//! to validate performance improvements from io-uring.
//!
//! Expected results on Linux:
//! - Small files (4KB): 2-3x throughput improvement
//! - Large files (10MB): 20-40% throughput improvement

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use tempfile::TempDir;

/// Benchmark tokio::fs for small file (4KB) reads
fn bench_tokio_small_file_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Pre-populate test files
    let test_files: Vec<PathBuf> = (0..100)
        .map(|i| {
            let path = temp_dir.path().join(format!("small-{}.bin", i));
            let data = vec![0u8; 4 * 1024];
            rt.block_on(async {
                tokio::fs::write(&path, &data).await.unwrap();
            });
            path
        })
        .collect();

    let mut group = c.benchmark_group("backend_tokio_small_file");
    group.bench_function("4kb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = &test_files[counter % 100];
            counter += 1;

            rt.block_on(async {
                let result = tokio::fs::read(black_box(path)).await.unwrap();
                assert_eq!(result.len(), 4 * 1024);
            });
        });
    });
    group.finish();
}

/// Benchmark tokio::fs for large file (10MB) reads
fn bench_tokio_large_file_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Pre-populate test files
    let test_files: Vec<PathBuf> = (0..10)
        .map(|i| {
            let path = temp_dir.path().join(format!("large-{}.bin", i));
            let data = vec![0u8; 10 * 1024 * 1024];
            rt.block_on(async {
                tokio::fs::write(&path, &data).await.unwrap();
            });
            path
        })
        .collect();

    let mut group = c.benchmark_group("backend_tokio_large_file");
    group.sample_size(10); // Fewer samples for large files
    group.bench_function("10mb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = &test_files[counter % 10];
            counter += 1;

            rt.block_on(async {
                let result = tokio::fs::read(black_box(path)).await.unwrap();
                assert_eq!(result.len(), 10 * 1024 * 1024);
            });
        });
    });
    group.finish();
}

/// Benchmark tokio-uring for small file (4KB) reads (Linux only)
#[cfg(target_os = "linux")]
fn bench_uring_small_file_read(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Pre-populate test files using tokio-uring
    let test_files: Vec<PathBuf> = tokio_uring::start(async {
        let mut files = Vec::new();
        for i in 0..100 {
            let path = temp_dir.path().join(format!("small-uring-{}.bin", i));
            let data = vec![0u8; 4 * 1024];

            let file = tokio_uring::fs::File::create(&path).await.unwrap();
            let (res, _) = file.write_at(data, 0).await;
            res.unwrap();

            files.push(path);
        }
        files
    });

    let mut group = c.benchmark_group("backend_uring_small_file");
    group.bench_function("4kb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = &test_files[counter % 100];
            counter += 1;

            tokio_uring::start(async {
                let file = tokio_uring::fs::File::open(black_box(path)).await.unwrap();

                // Read the file (we know it's 4KB)
                let buf = vec![0u8; 4 * 1024];
                let (res, buf) = file.read_at(buf, 0).await;
                res.unwrap();
                let data = Bytes::from(buf);

                assert_eq!(data.len(), 4 * 1024);
            });
        });
    });
    group.finish();
}

/// Benchmark tokio-uring for large file (10MB) reads (Linux only)
#[cfg(target_os = "linux")]
fn bench_uring_large_file_read(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();

    // Pre-populate test files using tokio-uring
    let test_files: Vec<PathBuf> = tokio_uring::start(async {
        let mut files = Vec::new();
        for i in 0..10 {
            let path = temp_dir.path().join(format!("large-uring-{}.bin", i));
            let data = vec![0u8; 10 * 1024 * 1024];

            let file = tokio_uring::fs::File::create(&path).await.unwrap();
            let (res, _) = file.write_at(data, 0).await;
            res.unwrap();

            files.push(path);
        }
        files
    });

    let mut group = c.benchmark_group("backend_uring_large_file");
    group.sample_size(10); // Fewer samples for large files
    group.bench_function("10mb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = &test_files[counter % 10];
            counter += 1;

            tokio_uring::start(async {
                let file = tokio_uring::fs::File::open(black_box(path)).await.unwrap();

                // Read the file (we know it's 10MB)
                let buf = vec![0u8; 10 * 1024 * 1024];
                let (res, buf) = file.read_at(buf, 0).await;
                res.unwrap();
                let data = Bytes::from(buf);

                assert_eq!(data.len(), 10 * 1024 * 1024);
            });
        });
    });
    group.finish();
}

// Conditionally compile the criterion group based on platform
#[cfg(target_os = "linux")]
criterion_group!(
    benches,
    bench_tokio_small_file_read,
    bench_tokio_large_file_read,
    bench_uring_small_file_read,
    bench_uring_large_file_read,
);

#[cfg(not(target_os = "linux"))]
criterion_group!(
    benches,
    bench_tokio_small_file_read,
    bench_tokio_large_file_read,
);

criterion_main!(benches);
