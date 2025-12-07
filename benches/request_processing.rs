//! Phase 42: Request Processing Benchmarks
//!
//! Benchmarks for request parsing, range header parsing, and response header construction.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use yatagarasu::s3::parse_range_header;

/// Benchmark single range parsing (most common case)
fn bench_single_range_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_parsing_single");

    // Standard range (bytes=0-1023)
    group.bench_function("standard_range", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-1023")))
    });

    // Open-ended range (bytes=1000-)
    group.bench_function("open_ended_range", |b| {
        b.iter(|| parse_range_header(black_box("bytes=1000-")))
    });

    // Suffix range (bytes=-1000)
    group.bench_function("suffix_range", |b| {
        b.iter(|| parse_range_header(black_box("bytes=-1000")))
    });

    // Large range numbers
    group.bench_function("large_numbers", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-1073741823")))
    });

    // Very large range numbers (1TB file)
    group.bench_function("very_large_numbers", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-1099511627775")))
    });

    group.finish();
}

/// Benchmark multi-range parsing (video seeking, parallel downloads)
fn bench_multi_range_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_parsing_multi");

    // 2 ranges
    group.bench_function("2_ranges", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-100,200-300")))
    });

    // 5 ranges
    group.bench_function("5_ranges", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-100,200-300,400-500,600-700,800-900")))
    });

    // 10 ranges
    group.bench_function("10_ranges", |b| {
        b.iter(|| {
            parse_range_header(black_box(
                "bytes=0-100,200-300,400-500,600-700,800-900,1000-1100,1200-1300,1400-1500,1600-1700,1800-1900",
            ))
        })
    });

    group.finish();
}

/// Benchmark invalid range parsing (error cases should be fast)
fn bench_invalid_range_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_parsing_invalid");

    // Missing unit
    group.bench_function("missing_unit", |b| {
        b.iter(|| parse_range_header(black_box("0-1023")))
    });

    // Invalid format
    group.bench_function("invalid_format", |b| {
        b.iter(|| parse_range_header(black_box("bytes:0-1023")))
    });

    // Empty range
    group.bench_function("empty_range", |b| {
        b.iter(|| parse_range_header(black_box("bytes=")))
    });

    // Non-numeric
    group.bench_function("non_numeric", |b| {
        b.iter(|| parse_range_header(black_box("bytes=abc-def")))
    });

    // Missing both start and end
    group.bench_function("missing_both", |b| {
        b.iter(|| parse_range_header(black_box("bytes=-")))
    });

    group.finish();
}

/// Benchmark range parsing with various string lengths
fn bench_range_parsing_string_lengths(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_parsing_string_length");

    // Short string
    group.bench_function("short", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-9")))
    });

    // Medium string
    group.bench_function("medium", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-1000000")))
    });

    // Long string with multiple ranges
    let long_range = "bytes=".to_string()
        + &(0..20)
            .map(|i| format!("{}-{}", i * 1000, i * 1000 + 100))
            .collect::<Vec<_>>()
            .join(",");
    group.bench_function("long_20_ranges", |b| {
        b.iter(|| parse_range_header(black_box(&long_range)))
    });

    group.finish();
}

/// Benchmark range parsing with whitespace (edge cases)
fn bench_range_parsing_whitespace(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_parsing_whitespace");

    // No whitespace (clean)
    group.bench_function("no_whitespace", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-1023")))
    });

    // Leading/trailing whitespace
    group.bench_function("leading_trailing", |b| {
        b.iter(|| parse_range_header(black_box("  bytes=0-1023  ")))
    });

    // Whitespace in ranges
    group.bench_function("whitespace_in_ranges", |b| {
        b.iter(|| parse_range_header(black_box("bytes= 0 - 1023 ")))
    });

    // Whitespace between ranges
    group.bench_function("whitespace_between", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-100, 200-300, 400-500")))
    });

    group.finish();
}

/// Benchmark ByteRange operations
fn bench_byte_range_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("byte_range_operations");

    // Size calculation for standard range
    group.bench_function("size_standard", |b| {
        let range = parse_range_header("bytes=0-1023").unwrap();
        b.iter(|| black_box(range.ranges[0].size()))
    });

    // Size calculation for open-ended (None)
    group.bench_function("size_open_ended", |b| {
        let range = parse_range_header("bytes=1000-").unwrap();
        b.iter(|| black_box(range.ranges[0].size()))
    });

    // Size calculation for suffix (None)
    group.bench_function("size_suffix", |b| {
        let range = parse_range_header("bytes=-1000").unwrap();
        b.iter(|| black_box(range.ranges[0].size()))
    });

    group.finish();
}

/// Benchmark video seeking scenario (common real-world pattern)
/// Simulates a video player seeking to different positions
fn bench_video_seeking_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("video_seeking_scenario");

    // Initial buffering (start of file)
    group.bench_function("initial_buffer", |b| {
        b.iter(|| parse_range_header(black_box("bytes=0-2097151")))
    });

    // Mid-video seek
    group.bench_function("mid_seek", |b| {
        b.iter(|| parse_range_header(black_box("bytes=52428800-54525951")))
    });

    // End of video seek
    group.bench_function("end_seek", |b| {
        b.iter(|| parse_range_header(black_box("bytes=104857600-")))
    });

    // Multiple chunk prefetch (adaptive streaming)
    group.bench_function("chunk_prefetch", |b| {
        b.iter(|| {
            parse_range_header(black_box(
                "bytes=52428800-54525951,54525952-56623103,56623104-58720255",
            ))
        })
    });

    group.finish();
}

/// Benchmark parallel download scenario (common real-world pattern)
/// Simulates download accelerators splitting file into chunks
fn bench_parallel_download_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_download_scenario");

    // 4-way parallel download (100MB file split into 4 chunks)
    group.bench_function("4_way_split", |b| {
        b.iter(|| {
            // Each chunk is ~25MB
            let _ = parse_range_header(black_box("bytes=0-26214399"));
            let _ = parse_range_header(black_box("bytes=26214400-52428799"));
            let _ = parse_range_header(black_box("bytes=52428800-78643199"));
            let _ = parse_range_header(black_box("bytes=78643200-104857599"));
        })
    });

    // Single request with 4 ranges (less common but valid)
    group.bench_function("4_ranges_single_request", |b| {
        b.iter(|| {
            parse_range_header(black_box(
                "bytes=0-26214399,26214400-52428799,52428800-78643199,78643200-104857599",
            ))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_range_parsing,
    bench_multi_range_parsing,
    bench_invalid_range_parsing,
    bench_range_parsing_string_lengths,
    bench_range_parsing_whitespace,
    bench_byte_range_operations,
    bench_video_seeking_scenario,
    bench_parallel_download_scenario,
);
criterion_main!(benches);
