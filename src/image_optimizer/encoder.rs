//! Image encoder abstraction
//!
//! Provides a trait-based encoder system that allows:
//! - Swapping between basic and optimized encoders
//! - Consistent quality settings across formats
//! - Format-specific configuration options

use super::error::ImageError;
use super::params::OutputFormat;

/// Quality settings for image encoding
#[derive(Debug, Clone, Copy)]
pub struct EncoderQuality {
    /// Quality value (1-100, where 100 is best quality)
    pub quality: u8,
    /// Effort/speed trade-off (0-10, where 10 is slowest/best compression)
    pub effort: u8,
}

impl Default for EncoderQuality {
    fn default() -> Self {
        Self {
            quality: 80,
            effort: 4,
        }
    }
}

impl EncoderQuality {
    /// Create quality settings with specified quality level
    pub fn with_quality(quality: u8) -> Self {
        Self {
            quality: quality.clamp(1, 100),
            effort: 4,
        }
    }

    /// Set the encoding effort (speed vs compression trade-off)
    pub fn with_effort(mut self, effort: u8) -> Self {
        self.effort = effort.clamp(0, 10);
        self
    }
}

/// Result of encoding an image
#[derive(Debug)]
pub struct EncodedImage {
    /// The encoded image data
    pub data: Vec<u8>,
    /// The output format
    pub format: OutputFormat,
    /// Content-Type header value
    pub content_type: &'static str,
}

impl EncodedImage {
    /// Create a new encoded image result
    pub fn new(data: Vec<u8>, format: OutputFormat) -> Self {
        let content_type = format.content_type();
        Self {
            data,
            format,
            content_type,
        }
    }
}

/// Trait for image encoders
///
/// Implementations handle encoding raw image data to specific formats.
/// The trait is object-safe to allow dynamic dispatch when needed.
pub trait ImageEncoder: Send + Sync {
    /// The output format this encoder produces
    fn format(&self) -> OutputFormat;

    /// Encode raw RGBA image data to the target format
    ///
    /// # Arguments
    /// * `data` - Raw pixel data in RGBA format (4 bytes per pixel)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `quality` - Quality settings
    ///
    /// # Returns
    /// * `Ok(EncodedImage)` - Encoded image data with metadata
    /// * `Err(ImageError)` - If encoding fails
    fn encode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError>;

    /// Check if this encoder supports transparency
    fn supports_transparency(&self) -> bool;
}

/// JPEG encoder using the image crate
pub struct JpegEncoder;

impl ImageEncoder for JpegEncoder {
    fn format(&self) -> OutputFormat {
        OutputFormat::Jpeg
    }

    fn encode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError> {
        use image::codecs::jpeg::JpegEncoder as ImageJpegEncoder;
        use image::ImageEncoder as _;
        use std::io::Cursor;

        // Convert RGBA to RGB (JPEG doesn't support alpha)
        let rgb_data = rgba_to_rgb(data);

        let mut output = Cursor::new(Vec::new());
        let encoder = ImageJpegEncoder::new_with_quality(&mut output, quality.quality);

        encoder
            .write_image(&rgb_data, width, height, image::ColorType::Rgb8)
            .map_err(|e| ImageError::encode_failed("jpeg", e.to_string()))?;

        Ok(EncodedImage::new(output.into_inner(), OutputFormat::Jpeg))
    }

    fn supports_transparency(&self) -> bool {
        false
    }
}

/// PNG encoder using the image crate
pub struct PngEncoder;

impl ImageEncoder for PngEncoder {
    fn format(&self) -> OutputFormat {
        OutputFormat::Png
    }

    fn encode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        _quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError> {
        use image::codecs::png::PngEncoder as ImagePngEncoder;
        use image::ImageEncoder as _;
        use std::io::Cursor;

        let mut output = Cursor::new(Vec::new());
        let encoder = ImagePngEncoder::new(&mut output);

        encoder
            .write_image(data, width, height, image::ColorType::Rgba8)
            .map_err(|e| ImageError::encode_failed("png", e.to_string()))?;

        Ok(EncodedImage::new(output.into_inner(), OutputFormat::Png))
    }

    fn supports_transparency(&self) -> bool {
        true
    }
}

/// WebP encoder using the image crate
///
/// Note: The `image` crate only supports lossless WebP encoding.
/// For lossy WebP encoding, consider using the `webp` crate directly.
pub struct WebPEncoder;

impl Default for WebPEncoder {
    fn default() -> Self {
        Self
    }
}

impl ImageEncoder for WebPEncoder {
    fn format(&self) -> OutputFormat {
        OutputFormat::WebP
    }

    fn encode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        _quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError> {
        use image::codecs::webp::WebPEncoder as ImageWebPEncoder;
        use image::ImageEncoder as _;
        use std::io::Cursor;

        let mut output = Cursor::new(Vec::new());
        let encoder = ImageWebPEncoder::new_lossless(&mut output);

        encoder
            .write_image(data, width, height, image::ColorType::Rgba8)
            .map_err(|e| ImageError::encode_failed("webp", e.to_string()))?;

        Ok(EncodedImage::new(output.into_inner(), OutputFormat::WebP))
    }

    fn supports_transparency(&self) -> bool {
        true
    }
}

/// Factory for creating encoders based on output format
pub struct EncoderFactory;

impl EncoderFactory {
    /// Create an encoder for the specified output format
    pub fn create(format: OutputFormat) -> Box<dyn ImageEncoder> {
        match format {
            OutputFormat::Jpeg => Box::new(JpegEncoder),
            OutputFormat::Png => Box::new(PngEncoder),
            OutputFormat::WebP => Box::new(WebPEncoder),
            OutputFormat::Avif => Box::new(AvifEncoder::default()),
            OutputFormat::Auto => Box::new(JpegEncoder), // Default fallback
        }
    }
}

/// AVIF encoder placeholder
///
/// Full AVIF support requires the `ravif` crate. This implementation
/// provides a stub that returns an error if AVIF is not compiled in.
pub struct AvifEncoder {
    /// Speed preset (1-10, where 1 is slowest/best quality)
    pub speed: u8,
}

impl Default for AvifEncoder {
    fn default() -> Self {
        Self { speed: 4 }
    }
}

impl ImageEncoder for AvifEncoder {
    fn format(&self) -> OutputFormat {
        OutputFormat::Avif
    }

    fn encode(
        &self,
        _data: &[u8],
        _width: u32,
        _height: u32,
        _quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError> {
        // AVIF encoding requires the ravif crate
        // This is a placeholder that can be enabled with a feature flag
        Err(ImageError::encode_failed(
            "avif",
            "AVIF encoding not available. Enable the 'avif' feature.",
        ))
    }

    fn supports_transparency(&self) -> bool {
        true
    }
}

/// Convert RGBA to RGB by discarding alpha channel
fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    let pixel_count = rgba.len() / 4;
    let mut rgb = Vec::with_capacity(pixel_count * 3);

    for chunk in rgba.chunks_exact(4) {
        rgb.push(chunk[0]); // R
        rgb.push(chunk[1]); // G
        rgb.push(chunk[2]); // B
                            // Alpha (chunk[3]) is discarded
    }

    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_quality_default() {
        let quality = EncoderQuality::default();
        assert_eq!(quality.quality, 80);
        assert_eq!(quality.effort, 4);
    }

    #[test]
    fn test_encoder_quality_with_quality() {
        let quality = EncoderQuality::with_quality(90);
        assert_eq!(quality.quality, 90);
    }

    #[test]
    fn test_encoder_quality_clamps_values() {
        let quality = EncoderQuality::with_quality(150);
        assert_eq!(quality.quality, 100);

        let quality = EncoderQuality::with_quality(0);
        assert_eq!(quality.quality, 1);
    }

    #[test]
    fn test_encoder_quality_effort() {
        let quality = EncoderQuality::default().with_effort(8);
        assert_eq!(quality.effort, 8);

        let quality = EncoderQuality::default().with_effort(15);
        assert_eq!(quality.effort, 10);
    }

    #[test]
    fn test_encoder_factory_creates_jpeg() {
        let encoder = EncoderFactory::create(OutputFormat::Jpeg);
        assert_eq!(encoder.format(), OutputFormat::Jpeg);
        assert!(!encoder.supports_transparency());
    }

    #[test]
    fn test_encoder_factory_creates_png() {
        let encoder = EncoderFactory::create(OutputFormat::Png);
        assert_eq!(encoder.format(), OutputFormat::Png);
        assert!(encoder.supports_transparency());
    }

    #[test]
    fn test_encoder_factory_creates_webp() {
        let encoder = EncoderFactory::create(OutputFormat::WebP);
        assert_eq!(encoder.format(), OutputFormat::WebP);
        assert!(encoder.supports_transparency());
    }

    #[test]
    fn test_encoder_factory_creates_avif() {
        let encoder = EncoderFactory::create(OutputFormat::Avif);
        assert_eq!(encoder.format(), OutputFormat::Avif);
        assert!(encoder.supports_transparency());
    }

    #[test]
    fn test_rgba_to_rgb() {
        let rgba = vec![255, 128, 64, 255, 0, 0, 0, 128];
        let rgb = rgba_to_rgb(&rgba);
        assert_eq!(rgb, vec![255, 128, 64, 0, 0, 0]);
    }

    #[test]
    fn test_encoded_image_content_type() {
        let encoded = EncodedImage::new(vec![], OutputFormat::Jpeg);
        assert_eq!(encoded.content_type, "image/jpeg");

        let encoded = EncodedImage::new(vec![], OutputFormat::WebP);
        assert_eq!(encoded.content_type, "image/webp");
    }

    #[test]
    fn test_jpeg_encoder_produces_output() {
        // Create a simple 2x2 RGBA image (red, green, blue, white)
        let data = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 255, // White
        ];

        let encoder = JpegEncoder;
        let result = encoder.encode(&data, 2, 2, EncoderQuality::default());
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::Jpeg);
        assert!(!encoded.data.is_empty());
        // JPEG magic bytes: FF D8 FF
        assert_eq!(&encoded.data[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_png_encoder_produces_output() {
        // Create a simple 2x2 RGBA image
        let data = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255,
            128, // Semi-transparent white
        ];

        let encoder = PngEncoder;
        let result = encoder.encode(&data, 2, 2, EncoderQuality::default());
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::Png);
        assert!(!encoded.data.is_empty());
        // PNG magic bytes: 89 50 4E 47
        assert_eq!(&encoded.data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_webp_encoder_produces_output() {
        // Create a simple 2x2 RGBA image
        let data = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];

        let encoder = WebPEncoder::default();
        let result = encoder.encode(&data, 2, 2, EncoderQuality::default());
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::WebP);
        assert!(!encoded.data.is_empty());
        // WebP magic: RIFF....WEBP
        assert_eq!(&encoded.data[0..4], b"RIFF");
        assert_eq!(&encoded.data[8..12], b"WEBP");
    }

    #[test]
    fn test_avif_encoder_not_available() {
        let encoder = AvifEncoder::default();
        let data = vec![255, 0, 0, 255];
        let result = encoder.encode(&data, 1, 1, EncoderQuality::default());
        assert!(result.is_err());
    }
}
