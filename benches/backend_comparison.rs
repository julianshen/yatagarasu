//! Backend performance comparison benchmarks
//!
//! Compares TokioFsBackend (portable) vs UringBackend (Linux only)
//! to validate performance improvements from io-uring with spawn_blocking.
//!
//! Expected results on Linux:
//! - Small files (4KB): 2-3x throughput improvement
//! - Large files (10MB): 20-40% throughput improvement

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use tempfile::TempDir;
use yatagarasu::cache::disk::backend::DiskBackend;
use yatagarasu::cache::disk::tokio_backend::TokioFsBackend;

#[cfg(target_os = "linux")]
use yatagarasu::cache::disk::uring_backend::UringBackend;

/// Benchmark TokioFsBackend for small file (4KB) reads
fn bench_tokio_small_file_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let backend = TokioFsBackend::new();

    // Pre-populate test files
    let test_files: Vec<PathBuf> = rt.block_on(async {
        let mut files = Vec::new();
        for i in 0..100 {
            let path = temp_dir.path().join(format!("small-{}.bin", i));
            let data = Bytes::from(vec![0u8; 4 * 1024]);
            backend.write_file_atomic(&path, data).await.unwrap();
            files.push(path);
        }
        files
    });

    let mut group = c.benchmark_group("backend_tokio_small_file");
    group.bench_function("4kb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = &test_files[counter % 100];
            counter += 1;

            rt.block_on(async {
                let result = backend.read_file(black_box(path)).await.unwrap();
                assert_eq!(result.len(), 4 * 1024);
            });
        });
    });
    group.finish();
}

/// Benchmark TokioFsBackend for small file (4KB) writes
fn bench_tokio_small_file_write(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let backend = TokioFsBackend::new();

    // Pre-generate data
    let data = Bytes::from(vec![0u8; 4 * 1024]);

    let mut group = c.benchmark_group("backend_tokio_small_file");
    group.bench_function("4kb_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = temp_dir.path().join(format!("write-{}.bin", counter));
            counter += 1;

            rt.block_on(async {
                backend
                    .write_file_atomic(black_box(&path), data.clone())
                    .await
                    .unwrap();
            });
        });
    });
    group.finish();
}

/// Benchmark TokioFsBackend for large file (10MB) reads
fn bench_tokio_large_file_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let backend = TokioFsBackend::new();

    // Pre-populate test files
    let test_files: Vec<PathBuf> = rt.block_on(async {
        let mut files = Vec::new();
        for i in 0..10 {
            let path = temp_dir.path().join(format!("large-{}.bin", i));
            let data = Bytes::from(vec![0u8; 10 * 1024 * 1024]);
            backend.write_file_atomic(&path, data).await.unwrap();
            files.push(path);
        }
        files
    });

    let mut group = c.benchmark_group("backend_tokio_large_file");
    group.sample_size(10); // Fewer samples for large files
    group.bench_function("10mb_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = &test_files[counter % 10];
            counter += 1;

            rt.block_on(async {
                let result = backend.read_file(black_box(path)).await.unwrap();
                assert_eq!(result.len(), 10 * 1024 * 1024);
            });
        });
    });
    group.finish();
}

/// Benchmark UringBackend for small file (4KB) reads (Linux only)
#[cfg(target_os = "linux")]
fn bench_uring_small_file_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let backend = UringBackend::new();

    // Pre-populate test files
    let test_files: Vec<PathBuf> = rt.block_on(async {
        let mut files = Vec::new();
        for i in 0..100 {
            let path = temp_dir.path().join(format!("small-uring-{}.bin", i));
            let data = Bytes::from(vec![0u8; 4 * 1024]);
            backend.write_file_atomic(&path, data).await.unwrap();
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

            rt.block_on(async {
                let result = backend.read_file(black_box(path)).await.unwrap();
                assert_eq!(result.len(), 4 * 1024);
            });
        });
    });
    group.finish();
}

/// Benchmark UringBackend for small file (4KB) writes (Linux only)
#[cfg(target_os = "linux")]
fn bench_uring_small_file_write(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let backend = UringBackend::new();

    // Pre-generate data
    let data = Bytes::from(vec![0u8; 4 * 1024]);

    let mut group = c.benchmark_group("backend_uring_small_file");
    group.bench_function("4kb_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let path = temp_dir.path().join(format!("write-uring-{}.bin", counter));
            counter += 1;

            rt.block_on(async {
                backend
                    .write_file_atomic(black_box(&path), data.clone())
                    .await
                    .unwrap();
            });
        });
    });
    group.finish();
}

/// Benchmark UringBackend for large file (10MB) reads (Linux only)
#[cfg(target_os = "linux")]
fn bench_uring_large_file_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let backend = UringBackend::new();

    // Pre-populate test files
    let test_files: Vec<PathBuf> = rt.block_on(async {
        let mut files = Vec::new();
        for i in 0..10 {
            let path = temp_dir.path().join(format!("large-uring-{}.bin", i));
            let data = Bytes::from(vec![0u8; 10 * 1024 * 1024]);
            backend.write_file_atomic(&path, data).await.unwrap();
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

            rt.block_on(async {
                let result = backend.read_file(black_box(path)).await.unwrap();
                assert_eq!(result.len(), 10 * 1024 * 1024);
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
    bench_tokio_small_file_write,
    bench_tokio_large_file_read,
    bench_uring_small_file_read,
    bench_uring_small_file_write,
    bench_uring_large_file_read,
);

#[cfg(not(target_os = "linux"))]
criterion_group!(
    benches,
    bench_tokio_small_file_read,
    bench_tokio_small_file_write,
    bench_tokio_large_file_read,
);

criterion_main!(benches);
