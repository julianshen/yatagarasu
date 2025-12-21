use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::{ImageFormat, RgbaImage};
use std::io::Cursor;
use yatagarasu::image_optimizer::{process_image, FitMode, ImageParams, OutputFormat};

fn create_bench_image(width: u32, height: u32) -> Vec<u8> {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8, 255]);
    }
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Png).unwrap();
    buffer.into_inner()
}

fn bench_image_resize(c: &mut Criterion) {
    // Generate a reasonably sized input image (e.g. 1920x1080)
    let input_data = create_bench_image(1920, 1080);

    let mut group = c.benchmark_group("image_resize");
    group.sample_size(10); // Image ops are slow, reduce sample size

    group.bench_function("resize_1080p_to_thumbnail_cover", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(yatagarasu::image_optimizer::Dimension::Pixels(200)),
                height: Some(yatagarasu::image_optimizer::Dimension::Pixels(200)),
                fit: FitMode::Cover,
                quality: Some(80),
                format: Some(OutputFormat::Jpeg),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.bench_function("resize_1080p_to_medium_contain", |b| {
        b.iter(|| {
            let params = ImageParams {
                width: Some(yatagarasu::image_optimizer::Dimension::Pixels(800)),
                height: Some(yatagarasu::image_optimizer::Dimension::Pixels(600)),
                fit: FitMode::Contain,
                quality: Some(85),
                format: Some(OutputFormat::WebP),
                ..Default::default()
            };
            process_image(black_box(&input_data), black_box(params)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_image_resize);
criterion_main!(benches);
