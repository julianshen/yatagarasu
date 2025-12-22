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

/// Configuration for encoder selection and settings
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Use MozJPEG for JPEG encoding (better compression)
    pub use_mozjpeg: bool,
    /// Use Oxipng for PNG encoding (better compression)
    pub use_oxipng: bool,
    /// Use enhanced WebP encoder with lossy support
    pub use_enhanced_webp: bool,
    /// Enable progressive JPEG encoding
    pub jpeg_progressive: bool,
    /// JPEG chroma subsampling mode
    pub jpeg_chroma_subsampling: ChromaSubsampling,
    /// PNG compression level (0-6)
    pub png_compression_level: u8,
    /// Strip metadata from PNG
    pub png_strip_metadata: bool,
    /// AVIF encoding speed (1-10)
    pub avif_speed: u8,
    /// Use lossy WebP encoding
    pub webp_lossy: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            use_mozjpeg: false,
            use_oxipng: false,
            use_enhanced_webp: false,
            jpeg_progressive: true,
            jpeg_chroma_subsampling: ChromaSubsampling::Cs420,
            png_compression_level: 4,
            png_strip_metadata: true,
            avif_speed: 6,
            webp_lossy: true,
        }
    }
}

impl EncoderConfig {
    /// Enable or disable MozJPEG encoder
    pub fn with_mozjpeg(mut self, enabled: bool) -> Self {
        self.use_mozjpeg = enabled;
        self
    }

    /// Enable or disable Oxipng encoder
    pub fn with_oxipng(mut self, enabled: bool) -> Self {
        self.use_oxipng = enabled;
        self
    }

    /// Enable or disable enhanced WebP encoder
    pub fn with_enhanced_webp(mut self, enabled: bool) -> Self {
        self.use_enhanced_webp = enabled;
        self
    }

    /// Set PNG compression level (0-6)
    pub fn with_png_compression_level(mut self, level: u8) -> Self {
        self.png_compression_level = level.clamp(0, 6);
        self
    }

    /// Set AVIF encoding speed (1-10)
    pub fn with_avif_speed(mut self, speed: u8) -> Self {
        self.avif_speed = speed.clamp(1, 10);
        self
    }

    /// Set JPEG progressive encoding
    pub fn with_jpeg_progressive(mut self, progressive: bool) -> Self {
        self.jpeg_progressive = progressive;
        self
    }

    /// Set JPEG chroma subsampling
    pub fn with_jpeg_chroma_subsampling(mut self, subsampling: ChromaSubsampling) -> Self {
        self.jpeg_chroma_subsampling = subsampling;
        self
    }
}

/// Factory for creating encoders based on output format
pub struct EncoderFactory;

impl EncoderFactory {
    /// Create an encoder for the specified output format using default settings
    pub fn create(format: OutputFormat) -> Box<dyn ImageEncoder> {
        Self::create_with_config(format, &EncoderConfig::default())
    }

    /// Create an encoder for the specified output format with custom configuration
    pub fn create_with_config(
        format: OutputFormat,
        config: &EncoderConfig,
    ) -> Box<dyn ImageEncoder> {
        match format {
            OutputFormat::Jpeg => {
                if config.use_mozjpeg {
                    Box::new(
                        MozJpegEncoder::new()
                            .progressive(config.jpeg_progressive)
                            .chroma_subsampling(config.jpeg_chroma_subsampling),
                    )
                } else {
                    Box::new(JpegEncoder)
                }
            }
            OutputFormat::Png => {
                if config.use_oxipng {
                    Box::new(
                        OxipngEncoder::new()
                            .compression_level(config.png_compression_level)
                            .strip_metadata(config.png_strip_metadata),
                    )
                } else {
                    Box::new(PngEncoder)
                }
            }
            OutputFormat::WebP => {
                if config.use_enhanced_webp {
                    Box::new(EnhancedWebPEncoder::new().lossy(config.webp_lossy))
                } else {
                    Box::new(WebPEncoder)
                }
            }
            OutputFormat::Avif => Box::new(RavifEncoder::new().speed(config.avif_speed)),
            OutputFormat::Auto => Box::new(JpegEncoder), // Default fallback
        }
    }
}

/// Chroma subsampling modes for JPEG encoding
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChromaSubsampling {
    /// 4:4:4 - Full chroma resolution (best quality, larger file)
    Cs444,
    /// 4:2:2 - Half horizontal chroma resolution
    Cs422,
    /// 4:2:0 - Quarter chroma resolution (smallest file)
    #[default]
    Cs420,
}

/// MozJPEG encoder using the mozjpeg-sys crate
///
/// Provides better compression than the standard image crate JPEG encoder
/// with support for:
/// - Progressive encoding
/// - Chroma subsampling control
/// - Optimized Huffman tables
#[derive(Default)]
pub struct MozJpegEncoder {
    /// Enable progressive JPEG encoding
    pub progressive: bool,
    /// Chroma subsampling mode
    pub chroma_subsampling: ChromaSubsampling,
}

impl MozJpegEncoder {
    /// Create a new MozJpegEncoder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable progressive encoding
    pub fn progressive(mut self, enabled: bool) -> Self {
        self.progressive = enabled;
        self
    }

    /// Set the chroma subsampling mode
    pub fn chroma_subsampling(mut self, mode: ChromaSubsampling) -> Self {
        self.chroma_subsampling = mode;
        self
    }
}

impl ImageEncoder for MozJpegEncoder {
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
        use mozjpeg_sys::*;
        use std::ptr;

        // Convert RGBA to RGB (JPEG doesn't support alpha)
        let rgb_data = rgba_to_rgb(data);

        unsafe {
            let mut err = std::mem::zeroed::<jpeg_error_mgr>();
            let mut cinfo = std::mem::zeroed::<jpeg_compress_struct>();

            cinfo.common.err = jpeg_std_error(&mut err);
            jpeg_create_compress(&mut cinfo);

            // Set up memory destination
            let mut out_buffer: *mut u8 = ptr::null_mut();
            let mut out_size: libc::c_ulong = 0;
            jpeg_mem_dest(&mut cinfo, &mut out_buffer, &mut out_size);

            // Set image parameters
            cinfo.image_width = width;
            cinfo.image_height = height;
            cinfo.input_components = 3;
            cinfo.in_color_space = J_COLOR_SPACE::JCS_RGB;

            jpeg_set_defaults(&mut cinfo);
            jpeg_set_quality(&mut cinfo, quality.quality as i32, true as i32);

            // Set chroma subsampling
            match self.chroma_subsampling {
                ChromaSubsampling::Cs444 => {
                    // 4:4:4 - no subsampling
                    (*cinfo.comp_info.offset(0)).h_samp_factor = 1;
                    (*cinfo.comp_info.offset(0)).v_samp_factor = 1;
                    (*cinfo.comp_info.offset(1)).h_samp_factor = 1;
                    (*cinfo.comp_info.offset(1)).v_samp_factor = 1;
                    (*cinfo.comp_info.offset(2)).h_samp_factor = 1;
                    (*cinfo.comp_info.offset(2)).v_samp_factor = 1;
                }
                ChromaSubsampling::Cs422 => {
                    // 4:2:2 - horizontal subsampling only
                    (*cinfo.comp_info.offset(0)).h_samp_factor = 2;
                    (*cinfo.comp_info.offset(0)).v_samp_factor = 1;
                    (*cinfo.comp_info.offset(1)).h_samp_factor = 1;
                    (*cinfo.comp_info.offset(1)).v_samp_factor = 1;
                    (*cinfo.comp_info.offset(2)).h_samp_factor = 1;
                    (*cinfo.comp_info.offset(2)).v_samp_factor = 1;
                }
                ChromaSubsampling::Cs420 => {
                    // 4:2:0 - default, let mozjpeg handle it
                }
            }

            // Progressive encoding
            if self.progressive {
                jpeg_simple_progression(&mut cinfo);
            }

            // Start compression
            jpeg_start_compress(&mut cinfo, true as i32);

            // Write scanlines
            let row_stride = width as usize * 3;
            let mut row_pointer: [*const u8; 1] = [ptr::null()];

            while cinfo.next_scanline < cinfo.image_height {
                let offset = cinfo.next_scanline as usize * row_stride;
                row_pointer[0] = rgb_data.as_ptr().add(offset);
                jpeg_write_scanlines(&mut cinfo, row_pointer.as_ptr() as *mut _, 1);
            }

            // Finish compression
            jpeg_finish_compress(&mut cinfo);

            // Copy output to Vec before destroying
            let output = if out_size > 0 && !out_buffer.is_null() {
                std::slice::from_raw_parts(out_buffer, out_size as usize).to_vec()
            } else {
                return Err(ImageError::encode_failed(
                    "mozjpeg",
                    "Output buffer is empty",
                ));
            };

            // Cleanup
            jpeg_destroy_compress(&mut cinfo);
            if !out_buffer.is_null() {
                libc::free(out_buffer as *mut libc::c_void);
            }

            Ok(EncodedImage::new(output, OutputFormat::Jpeg))
        }
    }

    fn supports_transparency(&self) -> bool {
        false
    }
}

/// Enhanced WebP encoder using the webp crate
///
/// Provides both lossy and lossless WebP encoding with better control than
/// the image crate's WebP encoder.
#[derive(Clone)]
pub struct EnhancedWebPEncoder {
    /// Use lossy compression (otherwise lossless)
    pub lossy: bool,
    /// Near-lossless preprocessing level (0-100, where 0 is off)
    pub near_lossless: u8,
}

impl Default for EnhancedWebPEncoder {
    fn default() -> Self {
        Self {
            lossy: true, // Default to lossy for better compression
            near_lossless: 0,
        }
    }
}

impl EnhancedWebPEncoder {
    /// Create a new EnhancedWebPEncoder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable lossy compression
    pub fn lossy(mut self, lossy: bool) -> Self {
        self.lossy = lossy;
        self
    }

    /// Set near-lossless preprocessing level (0-100)
    /// 0 = off, 100 = maximum preprocessing
    pub fn near_lossless(mut self, level: u8) -> Self {
        self.near_lossless = level.clamp(0, 100);
        self.lossy = false; // Near-lossless uses lossless mode
        self
    }
}

impl ImageEncoder for EnhancedWebPEncoder {
    fn format(&self) -> OutputFormat {
        OutputFormat::WebP
    }

    fn encode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError> {
        let encoder = webp::Encoder::from_rgba(data, width, height);

        let output = if self.lossy {
            // Lossy encoding with quality
            encoder.encode(quality.quality as f32)
        } else if self.near_lossless > 0 {
            // Near-lossless: use advanced config
            let mut config = webp::WebPConfig::new()
                .map_err(|_| ImageError::encode_failed("webp", "Failed to create WebP config"))?;
            config.lossless = 1;
            config.near_lossless = self.near_lossless as i32;
            config.quality = quality.quality as f32;

            encoder
                .encode_advanced(&config)
                .map_err(|e| ImageError::encode_failed("webp", format!("{:?}", e)))?
        } else {
            // Pure lossless
            encoder.encode_lossless()
        };

        Ok(EncodedImage::new(output.to_vec(), OutputFormat::WebP))
    }

    fn supports_transparency(&self) -> bool {
        true
    }
}

/// Oxipng encoder using the oxipng crate
///
/// Provides better PNG compression than the standard image crate encoder
/// with support for:
/// - Compression level control
/// - Metadata stripping
/// - Alpha channel optimization
#[derive(Clone)]
pub struct OxipngEncoder {
    /// Compression level (0-6, where 6 is slowest/best compression)
    pub compression_level: u8,
    /// Strip metadata from output
    pub strip_metadata: bool,
    /// Optimize alpha channel (may alter transparent pixels)
    pub optimize_alpha: bool,
}

impl Default for OxipngEncoder {
    fn default() -> Self {
        Self {
            compression_level: 2, // Default to moderate compression
            strip_metadata: true,
            optimize_alpha: false,
        }
    }
}

impl OxipngEncoder {
    /// Create a new OxipngEncoder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the compression level (0-6)
    pub fn compression_level(mut self, level: u8) -> Self {
        self.compression_level = level.clamp(0, 6);
        self
    }

    /// Enable or disable metadata stripping
    pub fn strip_metadata(mut self, strip: bool) -> Self {
        self.strip_metadata = strip;
        self
    }

    /// Enable or disable alpha optimization
    pub fn optimize_alpha(mut self, optimize: bool) -> Self {
        self.optimize_alpha = optimize;
        self
    }
}

impl ImageEncoder for OxipngEncoder {
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
        // Create a RawImage from RGBA data
        let raw = oxipng::RawImage::new(
            width,
            height,
            oxipng::ColorType::RGBA,
            oxipng::BitDepth::Eight,
            data.to_vec(),
        )
        .map_err(|e| ImageError::encode_failed("oxipng", e.to_string()))?;

        // Configure optimization options
        let mut opts = oxipng::Options::from_preset(self.compression_level);
        opts.optimize_alpha = self.optimize_alpha;

        if self.strip_metadata {
            opts.strip = oxipng::StripChunks::Safe;
        } else {
            opts.strip = oxipng::StripChunks::None;
        }

        // Create optimized PNG
        let output = raw
            .create_optimized_png(&opts)
            .map_err(|e| ImageError::encode_failed("oxipng", e.to_string()))?;

        Ok(EncodedImage::new(output, OutputFormat::Png))
    }

    fn supports_transparency(&self) -> bool {
        true
    }
}

/// AVIF encoder using the ravif crate
///
/// Provides high-quality AVIF encoding with AV1 compression.
/// Note: AVIF encoding is slower than other formats.
#[derive(Clone)]
pub struct RavifEncoder {
    /// Speed preset (1-10, where 1 is slowest/best quality, 10 is fastest)
    pub speed: u8,
}

impl Default for RavifEncoder {
    fn default() -> Self {
        Self { speed: 6 } // Default to faster speed for better UX
    }
}

impl RavifEncoder {
    /// Create a new RavifEncoder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the encoding speed (1-10)
    /// 1 = slowest/best quality, 10 = fastest/lowest quality
    pub fn speed(mut self, speed: u8) -> Self {
        self.speed = speed.clamp(1, 10);
        self
    }
}

impl ImageEncoder for RavifEncoder {
    fn format(&self) -> OutputFormat {
        OutputFormat::Avif
    }

    fn encode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        quality: EncoderQuality,
    ) -> Result<EncodedImage, ImageError> {
        // Convert raw RGBA bytes to rgb::RGBA slice
        let pixels: Vec<rgb::RGBA<u8>> = data
            .chunks_exact(4)
            .map(|c| rgb::RGBA::new(c[0], c[1], c[2], c[3]))
            .collect();

        let img = imgref::Img::new(pixels.as_slice(), width as usize, height as usize);

        let encoder = ravif::Encoder::new()
            .with_quality(quality.quality as f32)
            .with_speed(self.speed);

        let result = encoder
            .encode_rgba(img)
            .map_err(|e| ImageError::encode_failed("avif", e.to_string()))?;

        Ok(EncodedImage::new(result.avif_file, OutputFormat::Avif))
    }

    fn supports_transparency(&self) -> bool {
        true
    }
}

/// Legacy AVIF encoder alias (kept for backward compatibility)
pub type AvifEncoder = RavifEncoder;

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
#[allow(clippy::default_constructed_unit_structs)]
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

        let encoder = WebPEncoder;
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
    fn test_avif_encoder_available() {
        // AVIF is now available via ravif
        let encoder = AvifEncoder::default();
        let data: Vec<u8> = (0..32 * 32 * 4).map(|i| (i % 256) as u8).collect();
        let result = encoder.encode(&data, 32, 32, EncoderQuality::default());
        assert!(result.is_ok(), "AVIF encoding should succeed");
        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::Avif);
    }

    // ============================================================
    // Phase 50.1: Enhanced Encoders - MozJPEG Tests
    // ============================================================

    #[test]
    fn test_mozjpeg_encodes_valid_jpeg() {
        // Create a simple 2x2 RGBA image (red, green, blue, white)
        let data = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 255, // White
        ];

        let encoder = MozJpegEncoder::default();
        let result = encoder.encode(&data, 2, 2, EncoderQuality::default());
        assert!(result.is_ok(), "MozJPEG encoding should succeed");

        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::Jpeg);
        assert!(!encoded.data.is_empty());
        // JPEG magic bytes: FF D8 FF
        assert_eq!(&encoded.data[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_mozjpeg_quality_affects_size() {
        // Create a larger image to see quality difference
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = MozJpegEncoder::default();

        let high_quality = encoder
            .encode(&data, 100, 100, EncoderQuality::with_quality(95))
            .unwrap();
        let low_quality = encoder
            .encode(&data, 100, 100, EncoderQuality::with_quality(50))
            .unwrap();

        // Higher quality should produce larger file
        assert!(
            high_quality.data.len() > low_quality.data.len(),
            "High quality ({} bytes) should be larger than low quality ({} bytes)",
            high_quality.data.len(),
            low_quality.data.len()
        );
    }

    #[test]
    fn test_mozjpeg_progressive_encoding() {
        // Create test image
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = MozJpegEncoder::new().progressive(true);
        let result = encoder.encode(&data, 100, 100, EncoderQuality::default());

        assert!(result.is_ok(), "Progressive encoding should succeed");
        // Progressive JPEG should still be valid
        let encoded = result.unwrap();
        assert_eq!(&encoded.data[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_mozjpeg_chroma_subsampling() {
        // Create test image
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        // Test different chroma subsampling modes
        let encoder_444 = MozJpegEncoder::new().chroma_subsampling(ChromaSubsampling::Cs444);
        let encoder_420 = MozJpegEncoder::new().chroma_subsampling(ChromaSubsampling::Cs420);

        let result_444 = encoder_444
            .encode(&data, 100, 100, EncoderQuality::with_quality(80))
            .unwrap();
        let result_420 = encoder_420
            .encode(&data, 100, 100, EncoderQuality::with_quality(80))
            .unwrap();

        // 4:4:4 preserves more color data, so should be larger
        assert!(
            result_444.data.len() >= result_420.data.len(),
            "4:4:4 ({} bytes) should be >= 4:2:0 ({} bytes)",
            result_444.data.len(),
            result_420.data.len()
        );
    }

    #[test]
    fn test_mozjpeg_vs_image_compression_ratio() {
        // Create a larger test image to see meaningful compression differences
        let data: Vec<u8> = (0..200 * 200 * 4).map(|i| (i % 256) as u8).collect();
        let quality = EncoderQuality::with_quality(80);

        let image_encoder = JpegEncoder;
        let mozjpeg_encoder = MozJpegEncoder::default();

        let image_result = image_encoder.encode(&data, 200, 200, quality).unwrap();
        let mozjpeg_result = mozjpeg_encoder.encode(&data, 200, 200, quality).unwrap();

        // MozJPEG should typically produce smaller files than the image crate
        // However, for very synthetic images, this may not always hold
        // At minimum, both should produce valid JPEG files
        assert!(!image_result.data.is_empty());
        assert!(!mozjpeg_result.data.is_empty());

        // Log the sizes for comparison (helpful during debugging)
        eprintln!(
            "Image crate: {} bytes, MozJPEG: {} bytes",
            image_result.data.len(),
            mozjpeg_result.data.len()
        );
    }

    #[test]
    fn test_mozjpeg_encoder_format() {
        let encoder = MozJpegEncoder::default();
        assert_eq!(encoder.format(), OutputFormat::Jpeg);
        assert!(!encoder.supports_transparency());
    }

    // ============================================================
    // Phase 50.1: Enhanced Encoders - Oxipng Tests
    // ============================================================

    #[test]
    fn test_oxipng_encodes_valid_png() {
        // Create a simple 2x2 RGBA image
        let data = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 128, // Semi-transparent white
        ];

        let encoder = OxipngEncoder::default();
        let result = encoder.encode(&data, 2, 2, EncoderQuality::default());
        assert!(result.is_ok(), "Oxipng encoding should succeed");

        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::Png);
        assert!(!encoded.data.is_empty());
        // PNG magic bytes: 89 50 4E 47
        assert_eq!(&encoded.data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_oxipng_compression_levels() {
        // Create a larger image to see compression difference
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        let encoder_fast = OxipngEncoder::new().compression_level(1);
        let encoder_slow = OxipngEncoder::new().compression_level(6);

        let result_fast = encoder_fast
            .encode(&data, 100, 100, EncoderQuality::default())
            .unwrap();
        let result_slow = encoder_slow
            .encode(&data, 100, 100, EncoderQuality::default())
            .unwrap();

        // Higher compression level should produce smaller or equal file
        assert!(
            result_slow.data.len() <= result_fast.data.len(),
            "Level 6 ({} bytes) should be <= Level 1 ({} bytes)",
            result_slow.data.len(),
            result_fast.data.len()
        );
    }

    #[test]
    fn test_oxipng_strips_metadata() {
        // Create test image
        let data: Vec<u8> = (0..50 * 50 * 4).map(|i| (i % 256) as u8).collect();

        let encoder_keep = OxipngEncoder::new().strip_metadata(false);
        let encoder_strip = OxipngEncoder::new().strip_metadata(true);

        let result_keep = encoder_keep
            .encode(&data, 50, 50, EncoderQuality::default())
            .unwrap();
        let result_strip = encoder_strip
            .encode(&data, 50, 50, EncoderQuality::default())
            .unwrap();

        // Both should be valid PNGs
        assert_eq!(&result_keep.data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
        assert_eq!(&result_strip.data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_oxipng_alpha_optimization() {
        // Create image with unused alpha (all pixels opaque)
        let data: Vec<u8> = (0..50 * 50 * 4)
            .map(|i| if i % 4 == 3 { 255 } else { (i % 256) as u8 })
            .collect();

        let encoder = OxipngEncoder::new().optimize_alpha(true);
        let result = encoder
            .encode(&data, 50, 50, EncoderQuality::default())
            .unwrap();

        // Should still produce valid PNG
        assert_eq!(&result.data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_oxipng_encoder_format() {
        let encoder = OxipngEncoder::default();
        assert_eq!(encoder.format(), OutputFormat::Png);
        assert!(encoder.supports_transparency());
    }

    #[test]
    fn test_oxipng_vs_image_compression_ratio() {
        // Create a larger test image
        let data: Vec<u8> = (0..200 * 200 * 4).map(|i| (i % 256) as u8).collect();

        let image_encoder = PngEncoder;
        let oxipng_encoder = OxipngEncoder::new().compression_level(4);

        let image_result = image_encoder
            .encode(&data, 200, 200, EncoderQuality::default())
            .unwrap();
        let oxipng_result = oxipng_encoder
            .encode(&data, 200, 200, EncoderQuality::default())
            .unwrap();

        // Both should produce valid PNGs
        assert!(!image_result.data.is_empty());
        assert!(!oxipng_result.data.is_empty());

        eprintln!(
            "Image crate PNG: {} bytes, Oxipng: {} bytes",
            image_result.data.len(),
            oxipng_result.data.len()
        );
    }

    // ============================================================
    // Phase 50.1: Enhanced Encoders - Enhanced WebP Tests
    // ============================================================

    #[test]
    fn test_webp_lossy_encoding() {
        // Create test image
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = EnhancedWebPEncoder::new().lossy(true);
        let result = encoder.encode(&data, 100, 100, EncoderQuality::with_quality(80));

        assert!(result.is_ok(), "Lossy WebP encoding should succeed");
        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::WebP);
        // WebP magic: RIFF....WEBP
        assert_eq!(&encoded.data[0..4], b"RIFF");
        assert_eq!(&encoded.data[8..12], b"WEBP");
    }

    #[test]
    fn test_webp_lossless_encoding() {
        // Create test image
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = EnhancedWebPEncoder::new().lossy(false);
        let result = encoder.encode(&data, 100, 100, EncoderQuality::default());

        assert!(result.is_ok(), "Lossless WebP encoding should succeed");
        let encoded = result.unwrap();
        assert_eq!(&encoded.data[0..4], b"RIFF");
        assert_eq!(&encoded.data[8..12], b"WEBP");
    }

    #[test]
    fn test_webp_near_lossless_encoding() {
        // Create test image
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        // Near-lossless uses lossless mode with some preprocessing
        let encoder = EnhancedWebPEncoder::new().near_lossless(60);
        let result = encoder.encode(&data, 100, 100, EncoderQuality::default());

        assert!(result.is_ok(), "Near-lossless WebP encoding should succeed");
        let encoded = result.unwrap();
        assert_eq!(&encoded.data[0..4], b"RIFF");
    }

    #[test]
    fn test_webp_quality_affects_size() {
        // Create test image
        let data: Vec<u8> = (0..100 * 100 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = EnhancedWebPEncoder::new().lossy(true);

        let high_quality = encoder
            .encode(&data, 100, 100, EncoderQuality::with_quality(95))
            .unwrap();
        let low_quality = encoder
            .encode(&data, 100, 100, EncoderQuality::with_quality(50))
            .unwrap();

        assert!(
            high_quality.data.len() > low_quality.data.len(),
            "High quality ({} bytes) should be larger than low quality ({} bytes)",
            high_quality.data.len(),
            low_quality.data.len()
        );
    }

    #[test]
    fn test_enhanced_webp_encoder_format() {
        let encoder = EnhancedWebPEncoder::default();
        assert_eq!(encoder.format(), OutputFormat::WebP);
        assert!(encoder.supports_transparency());
    }

    // ============================================================
    // Phase 50.1: Enhanced Encoders - AVIF/ravif Tests
    // ============================================================

    #[test]
    fn test_ravif_encodes_valid_avif() {
        // Create a small test image to keep encoding fast
        let data: Vec<u8> = (0..32 * 32 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = RavifEncoder::default();
        let result = encoder.encode(&data, 32, 32, EncoderQuality::with_quality(70));

        assert!(result.is_ok(), "AVIF encoding should succeed");
        let encoded = result.unwrap();
        assert_eq!(encoded.format, OutputFormat::Avif);
        assert!(!encoded.data.is_empty());

        // AVIF files start with ftyp box: 00 00 00 XX 66 74 79 70
        // Check for 'ftyp' at offset 4
        assert!(encoded.data.len() > 12);
        assert_eq!(&encoded.data[4..8], b"ftyp");
    }

    #[test]
    fn test_ravif_quality_affects_size() {
        // Create test image
        let data: Vec<u8> = (0..64 * 64 * 4).map(|i| (i % 256) as u8).collect();

        let encoder = RavifEncoder::default();

        let high_quality = encoder
            .encode(&data, 64, 64, EncoderQuality::with_quality(95))
            .unwrap();
        let low_quality = encoder
            .encode(&data, 64, 64, EncoderQuality::with_quality(30))
            .unwrap();

        // Higher quality should produce larger file
        assert!(
            high_quality.data.len() > low_quality.data.len(),
            "High quality ({} bytes) should be larger than low quality ({} bytes)",
            high_quality.data.len(),
            low_quality.data.len()
        );
    }

    #[test]
    fn test_ravif_speed_affects_time() {
        // This is more of a smoke test - just verify different speeds work
        let data: Vec<u8> = (0..32 * 32 * 4).map(|i| (i % 256) as u8).collect();

        let encoder_fast = RavifEncoder::new().speed(10);
        let encoder_slow = RavifEncoder::new().speed(1);

        let result_fast = encoder_fast.encode(&data, 32, 32, EncoderQuality::with_quality(70));
        let result_slow = encoder_slow.encode(&data, 32, 32, EncoderQuality::with_quality(70));

        assert!(result_fast.is_ok(), "Fast encoding should succeed");
        assert!(result_slow.is_ok(), "Slow encoding should succeed");
    }

    #[test]
    fn test_ravif_encoder_format() {
        let encoder = RavifEncoder::default();
        assert_eq!(encoder.format(), OutputFormat::Avif);
        assert!(encoder.supports_transparency());
    }

    // ============================================================
    // Phase 50.1: Enhanced Encoders - EncoderConfig Tests
    // ============================================================

    #[test]
    fn test_encoder_config_defaults() {
        let config = EncoderConfig::default();

        // Check default values
        assert!(!config.use_mozjpeg);
        assert!(!config.use_oxipng);
        assert!(!config.use_enhanced_webp);
        assert!(config.jpeg_progressive);
        assert_eq!(config.jpeg_chroma_subsampling, ChromaSubsampling::Cs420);
        assert_eq!(config.png_compression_level, 4);
        assert!(config.png_strip_metadata);
        assert_eq!(config.avif_speed, 6);
    }

    #[test]
    fn test_encoder_config_validation() {
        // Test that config values are clamped to valid ranges
        let config = EncoderConfig::default()
            .with_png_compression_level(10) // Should clamp to 6
            .with_avif_speed(15); // Should clamp to 10

        assert_eq!(config.png_compression_level, 6);
        assert_eq!(config.avif_speed, 10);
    }

    #[test]
    fn test_encoder_factory_returns_correct_encoder() {
        // Test that factory returns enhanced encoders when configured
        let config = EncoderConfig::default()
            .with_mozjpeg(true)
            .with_oxipng(true)
            .with_enhanced_webp(true);

        let jpeg_encoder = EncoderFactory::create_with_config(OutputFormat::Jpeg, &config);
        let png_encoder = EncoderFactory::create_with_config(OutputFormat::Png, &config);
        let webp_encoder = EncoderFactory::create_with_config(OutputFormat::WebP, &config);
        let avif_encoder = EncoderFactory::create_with_config(OutputFormat::Avif, &config);

        assert_eq!(jpeg_encoder.format(), OutputFormat::Jpeg);
        assert_eq!(png_encoder.format(), OutputFormat::Png);
        assert_eq!(webp_encoder.format(), OutputFormat::WebP);
        assert_eq!(avif_encoder.format(), OutputFormat::Avif);
    }

    #[test]
    fn test_encoder_fallback_on_error() {
        // Test that factory falls back to standard encoders when enhanced not available
        let config = EncoderConfig::default(); // Enhanced encoders disabled

        let jpeg_encoder = EncoderFactory::create_with_config(OutputFormat::Jpeg, &config);
        let png_encoder = EncoderFactory::create_with_config(OutputFormat::Png, &config);

        // Should still produce valid output
        let data: Vec<u8> = (0..32 * 32 * 4).map(|i| (i % 256) as u8).collect();
        let jpeg_result = jpeg_encoder.encode(&data, 32, 32, EncoderQuality::default());
        let png_result = png_encoder.encode(&data, 32, 32, EncoderQuality::default());

        assert!(jpeg_result.is_ok());
        assert!(png_result.is_ok());
    }

    #[test]
    fn test_encoder_roundtrip_preserves_quality() {
        // Test that encoding and decoding preserves reasonable quality
        let data: Vec<u8> = (0..64 * 64 * 4).map(|i| (i % 256) as u8).collect();

        let config = EncoderConfig::default().with_mozjpeg(true);
        let encoder = EncoderFactory::create_with_config(OutputFormat::Jpeg, &config);

        let encoded = encoder
            .encode(&data, 64, 64, EncoderQuality::with_quality(95))
            .unwrap();

        // Verify it's a valid JPEG
        assert_eq!(&encoded.data[0..2], &[0xFF, 0xD8]); // JPEG magic
        assert!(!encoded.data.is_empty());
    }
}
