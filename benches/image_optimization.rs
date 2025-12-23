//! Image Optimization Benchmarks
//!
//! Benchmarks for image processing performance including:
//! - Resize operations at various sizes
//! - Format conversion (JPEG, WebP, PNG, AVIF)
//! - Quality adjustments
//! - Transformations (rotation, flip)
//!
//! Run with: `cargo bench --bench image_optimization`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use image::{ImageFormat, RgbaImage};
use std::io::Cursor;
use std::time::Duration;
use yatagarasu::image_optimizer::{process_image, Dimension, FitMode, ImageParams, OutputFormat};

/// Create a test image with gradient pattern (more realistic than solid color)
fn create_bench_image(width: u32, height: u32) -> Vec<u8> {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8, 255]);
    }
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png).unwrap();
    buffer.into_inner()
}

/// Create a JPEG test image (for format-specific benchmarks)
fn create_bench_jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8, 255]);
    }
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Jpeg).unwrap();
    buffer.into_inner()
}

/// Benchmark resize operations at different image sizes
fn bench_image_resize(c: &mut Criterion) {
    let input_data = create_bench_image(1920, 1080);

    let mut group = c.benchmark_group("image_resize");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    group.bench_function("1080p_to_thumbnail_200x200_cover", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(200)),
                height: Some(Dimension::Pixels(200)),
                fit: FitMode::Cover,
                quality: Some(80),
                format: Some(OutputFormat::Jpeg),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("1080p_to_medium_800x600_contain", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(800)),
                height: Some(Dimension::Pixels(600)),
                fit: FitMode::Contain,
                quality: Some(85),
                format: Some(OutputFormat::WebP),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("1080p_to_720p_scale_down", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(1280)),
                height: Some(Dimension::Pixels(720)),
                fit: FitMode::Contain,
                quality: Some(85),
                format: Some(OutputFormat::Jpeg),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.finish();
}

/// Benchmark resize at different source image sizes (1MP, 4MP, 12MP)
fn bench_image_resize_by_source_size(c: &mut Criterion) {
    let sizes = [
        ("1mp_1000x1000", create_bench_image(1000, 1000)),
        ("4mp_2000x2000", create_bench_image(2000, 2000)),
        ("12mp_4000x3000", create_bench_image(4000, 3000)),
    ];

    let mut group = c.benchmark_group("resize_by_source_size");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    for (name, input_data) in &sizes {
        group.bench_with_input(
            BenchmarkId::new("to_400x400", name),
            input_data,
            |b, data| {
                b.iter(|| {
                    let params = ImageParams {
                        width: Some(Dimension::Pixels(400)),
                        height: Some(Dimension::Pixels(400)),
                        fit: FitMode::Cover,
                        quality: Some(80),
                        format: Some(OutputFormat::Jpeg),
                        ..Default::default()
                    };
                    process_image(black_box(data), black_box(params)).unwrap();
                })
            },
        );
    }

    group.finish();
}

/// Benchmark format conversion (JPEG to WebP, PNG, AVIF)
fn bench_format_conversion(c: &mut Criterion) {
    let jpeg_data = create_bench_jpeg(1920, 1080);

    let mut group = c.benchmark_group("format_conversion");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("jpeg_to_webp", |b| {
        b.iter(|| {
            let params = ImageParams {
                format: Some(OutputFormat::WebP),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&jpeg_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("jpeg_to_png", |b| {
        b.iter(|| {
            let params = ImageParams {
                format: Some(OutputFormat::Png),
                ..Default::default()
            };
            process_image(black_box(&jpeg_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("jpeg_to_avif", |b| {
        b.iter(|| {
            let params = ImageParams {
                format: Some(OutputFormat::Avif),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&jpeg_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("jpeg_passthrough", |b| {
        b.iter(|| {
            let params = ImageParams {
                format: Some(OutputFormat::Jpeg),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&jpeg_data), black_box(params)).unwrap();
        })
    });

    group.finish();
}

/// Benchmark quality adjustment impact on processing time
fn bench_quality_levels(c: &mut Criterion) {
    let input_data = create_bench_image(1920, 1080);

    let mut group = c.benchmark_group("quality_levels");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for quality in [30, 60, 80, 95] {
        group.bench_with_input(
            BenchmarkId::new("jpeg_q", quality),
            &quality,
            |b, &quality| {
                b.iter(|| {
                    let params = ImageParams {
                        width: Some(Dimension::Pixels(800)),
                        height: Some(Dimension::Pixels(600)),
                        fit: FitMode::Contain,
                        quality: Some(quality),
                        format: Some(OutputFormat::Jpeg),
                        ..Default::default()
                    };
                    process_image(black_box(&input_data), black_box(params)).unwrap();
                })
            },
        );
    }

    group.finish();
}

/// Benchmark transformation operations (rotation, flip)
fn bench_transformations(c: &mut Criterion) {
    let input_data = create_bench_image(1920, 1080);

    let mut group = c.benchmark_group("transformations");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("rotate_90", |b| {
        b.iter(|| {
            let params = ImageParams {
                rotate: Some(90),
                format: Some(OutputFormat::Jpeg),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("rotate_180", |b| {
        b.iter(|| {
            let params = ImageParams {
                rotate: Some(180),
                format: Some(OutputFormat::Jpeg),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("flip_horizontal", |b| {
        b.iter(|| {
            let params = ImageParams {
                flip_h: true,
                format: Some(OutputFormat::Jpeg),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("flip_vertical", |b| {
        b.iter(|| {
            let params = ImageParams {
                flip_v: true,
                format: Some(OutputFormat::Jpeg),
                quality: Some(80),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.finish();
}

/// Benchmark combined operations (resize + format + quality)
fn bench_combined_operations(c: &mut Criterion) {
    let input_data = create_bench_image(1920, 1080);

    let mut group = c.benchmark_group("combined_operations");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("thumbnail_pipeline", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(150)),
                height: Some(Dimension::Pixels(150)),
                fit: FitMode::Cover,
                quality: Some(75),
                format: Some(OutputFormat::WebP),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("social_media_share", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(1200)),
                height: Some(Dimension::Pixels(630)),
                fit: FitMode::Cover,
                quality: Some(85),
                format: Some(OutputFormat::Jpeg),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("responsive_mobile", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(640)),
                height: None,
                fit: FitMode::Contain,
                quality: Some(80),
                format: Some(OutputFormat::WebP),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("high_dpi_2x", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(Dimension::Pixels(400)),
                height: Some(Dimension::Pixels(300)),
                dpr: 2.0,
                fit: FitMode::Contain,
                quality: Some(80),
                format: Some(OutputFormat::WebP),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_image_resize,
    bench_image_resize_by_source_size,
    bench_format_conversion,
    bench_quality_levels,
    bench_transformations,
    bench_combined_operations,
);
criterion_main!(benches);
