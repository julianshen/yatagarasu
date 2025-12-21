//! Image processing implementation
//!
//! Handles the actual image transformation: decode → resize → encode

use fast_image_resize::{FilterType, Image, PixelType, ResizeAlg, Resizer};
use image::io::Reader as ImageReader;
use image::DynamicImage;
use std::io::Cursor;
use std::num::NonZeroU32;

use super::encoder::{EncoderFactory, EncoderQuality};
use super::error::ImageError;
use super::params::{Dimension, FitMode, ImageParams, OutputFormat};

/// Result of image processing
pub struct ProcessedImage {
    /// The processed image data
    pub data: Vec<u8>,
    /// Content-Type header value
    pub content_type: String,
    /// Original dimensions (width, height)
    pub original_size: (u32, u32),
    /// Output dimensions (width, height)
    pub output_size: (u32, u32),
}

/// Process an image with the given parameters
///
/// # Arguments
/// * `data` - Raw image data bytes
/// * `params` - Transformation parameters
///
/// # Returns
/// * `Ok((data, content_type))` - Processed image data and MIME type
/// * `Err(ImageError)` - If processing fails
pub fn process_image(data: &[u8], params: ImageParams) -> Result<(Vec<u8>, String), String> {
    // Use the new internal function that returns ImageError
    process_image_internal(data, params)
        .map(|result| (result.data, result.content_type))
        .map_err(|e| e.to_string())
}

/// Internal processing with proper error types
pub fn process_image_internal(
    data: &[u8],
    params: ImageParams,
) -> Result<ProcessedImage, ImageError> {
    // 1. Decode the image
    let img = decode_image(data)?;
    let src_width = img.width();
    let src_height = img.height();

    // 2. Calculate target dimensions
    let (target_width, target_height) = calculate_dimensions(
        src_width,
        src_height,
        params.width.as_ref(),
        params.height.as_ref(),
        params.dpr,
        params.enlarge,
    );

    // 3. Resize if dimensions changed
    let processed_img = if target_width != src_width || target_height != src_height {
        resize_image(&img, target_width, target_height, &params.fit)?
    } else {
        img
    };

    // 4. Determine output format
    let output_format = params.format.unwrap_or_else(|| detect_format(data));

    // 5. Encode to target format
    let quality = EncoderQuality::with_quality(params.quality.unwrap_or(80));
    let encoder = EncoderFactory::create(output_format);

    let rgba_data = processed_img.to_rgba8().into_raw();
    let encoded = encoder.encode(&rgba_data, target_width, target_height, quality)?;

    Ok(ProcessedImage {
        data: encoded.data,
        content_type: encoded.content_type.to_string(),
        original_size: (src_width, src_height),
        output_size: (target_width, target_height),
    })
}

/// Decode image data into a DynamicImage
fn decode_image(data: &[u8]) -> Result<DynamicImage, ImageError> {
    ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| ImageError::decode_failed(e.to_string()))?
        .decode()
        .map_err(|e| ImageError::decode_failed(e.to_string()))
}

/// Detect image format from data
fn detect_format(data: &[u8]) -> OutputFormat {
    match image::guess_format(data) {
        Ok(fmt) => match fmt {
            image::ImageFormat::Png => OutputFormat::Png,
            image::ImageFormat::Jpeg => OutputFormat::Jpeg,
            image::ImageFormat::WebP => OutputFormat::WebP,
            image::ImageFormat::Avif => OutputFormat::Avif,
            _ => OutputFormat::Jpeg,
        },
        Err(_) => OutputFormat::Jpeg,
    }
}

/// Calculate target dimensions from parameters
fn calculate_dimensions(
    src_width: u32,
    src_height: u32,
    width: Option<&Dimension>,
    height: Option<&Dimension>,
    dpr: f32,
    enlarge: bool,
) -> (u32, u32) {
    // Resolve dimensions
    let target_width = width.map(|d| d.resolve(src_width)).unwrap_or(src_width);
    let target_height = height.map(|d| d.resolve(src_height)).unwrap_or(src_height);

    // Apply DPR
    let scaled_width = (target_width as f32 * dpr).round() as u32;
    let scaled_height = (target_height as f32 * dpr).round() as u32;

    // Prevent enlargement if not allowed
    if !enlarge {
        let final_width = scaled_width.min(src_width);
        let final_height = scaled_height.min(src_height);
        (final_width.max(1), final_height.max(1))
    } else {
        (scaled_width.max(1), scaled_height.max(1))
    }
}

/// Resize image using fast-image-resize with Lanczos3 filter
fn resize_image(
    img: &DynamicImage,
    target_w: u32,
    target_h: u32,
    _fit: &FitMode,
) -> Result<DynamicImage, ImageError> {
    let src_w = img.width();
    let src_h = img.height();

    let src_width =
        NonZeroU32::new(src_w).ok_or_else(|| ImageError::resize_failed("Source width is 0"))?;
    let src_height =
        NonZeroU32::new(src_h).ok_or_else(|| ImageError::resize_failed("Source height is 0"))?;
    let dst_width =
        NonZeroU32::new(target_w).ok_or_else(|| ImageError::resize_failed("Target width is 0"))?;
    let dst_height =
        NonZeroU32::new(target_h).ok_or_else(|| ImageError::resize_failed("Target height is 0"))?;

    let src_image = Image::from_vec_u8(
        src_width,
        src_height,
        img.to_rgba8().into_raw(),
        PixelType::U8x4,
    )
    .map_err(|e| ImageError::resize_failed(format!("Failed to create source image: {:?}", e)))?;

    let mut dst_image = Image::new(dst_width, dst_height, PixelType::U8x4);

    let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Lanczos3));

    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .map_err(|e| ImageError::resize_failed(format!("Resize operation failed: {:?}", e)))?;

    let result_buf = dst_image.into_vec();
    let rgba_image = image::RgbaImage::from_raw(target_w, target_h, result_buf)
        .ok_or_else(|| ImageError::resize_failed("Failed to create output image buffer"))?;

    Ok(DynamicImage::ImageRgba8(rgba_image))
}

// === Legacy type aliases for backward compatibility ===
// These will be removed in a future version

/// Deprecated: Use `OutputFormat` from params module instead
pub type ImageFormatType = OutputFormat;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_jpeg() -> Vec<u8> {
        // Create a minimal 2x2 JPEG image
        let img = image::RgbaImage::from_fn(2, 2, |x, y| {
            if (x + y) % 2 == 0 {
                image::Rgba([255, 0, 0, 255]) // Red
            } else {
                image::Rgba([0, 0, 255, 255]) // Blue
            }
        });

        let mut buffer = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Jpeg)
            .unwrap();
        buffer.into_inner()
    }

    #[test]
    fn test_decode_image() {
        let jpeg_data = create_test_jpeg();
        let result = decode_image(&jpeg_data);
        assert!(result.is_ok());

        let img = result.unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
    }

    #[test]
    fn test_decode_invalid_data() {
        let invalid_data = vec![0, 1, 2, 3, 4, 5];
        let result = decode_image(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_format() {
        let jpeg_data = create_test_jpeg();
        assert_eq!(detect_format(&jpeg_data), OutputFormat::Jpeg);

        // PNG magic bytes
        let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_format(&png_header), OutputFormat::Png);
    }

    #[test]
    fn test_calculate_dimensions_no_change() {
        let (w, h) = calculate_dimensions(100, 100, None, None, 1.0, false);
        assert_eq!((w, h), (100, 100));
    }

    #[test]
    fn test_calculate_dimensions_with_width() {
        let width = Dimension::Pixels(50);
        let (w, h) = calculate_dimensions(100, 100, Some(&width), None, 1.0, true);
        assert_eq!(w, 50);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_calculate_dimensions_percentage() {
        let width = Dimension::Percentage(50.0);
        let (w, _) = calculate_dimensions(100, 100, Some(&width), None, 1.0, true);
        assert_eq!(w, 50);
    }

    #[test]
    fn test_calculate_dimensions_with_dpr() {
        let width = Dimension::Pixels(100);
        let (w, _) = calculate_dimensions(200, 200, Some(&width), None, 2.0, true);
        assert_eq!(w, 200); // 100 * 2.0 DPR
    }

    #[test]
    fn test_calculate_dimensions_no_enlarge() {
        let width = Dimension::Pixels(200);
        let (w, _) = calculate_dimensions(100, 100, Some(&width), None, 1.0, false);
        assert_eq!(w, 100); // Capped at source size
    }

    #[test]
    fn test_process_image_basic() {
        let jpeg_data = create_test_jpeg();
        let params = ImageParams::default();

        let result = process_image(&jpeg_data, params);
        assert!(result.is_ok());

        let (data, content_type) = result.unwrap();
        assert!(!data.is_empty());
        assert_eq!(content_type, "image/jpeg");
    }

    #[test]
    fn test_process_image_with_resize() {
        let jpeg_data = create_test_jpeg();
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(4));
        params.height = Some(Dimension::Pixels(4));
        params.enlarge = true; // Enable upsizing for this test

        let result = process_image_internal(&jpeg_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (4, 4));
    }

    #[test]
    fn test_process_image_format_conversion() {
        let jpeg_data = create_test_jpeg();
        let mut params = ImageParams::default();
        params.format = Some(OutputFormat::Png);

        let result = process_image(&jpeg_data, params);
        assert!(result.is_ok());

        let (data, content_type) = result.unwrap();
        assert_eq!(content_type, "image/png");
        // PNG magic bytes
        assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}
