//! Watermark processor for applying watermarks to images.
//!
//! This module provides the high-level API for applying watermarks
//! to images during the image processing pipeline.
//!
//! # Integration Point
//!
//! Watermarks are applied AFTER image effects (blur, sharpen, etc.)
//! and BEFORE encoding to the output format.
//!
//! # Example
//!
//! ```ignore
//! use yatagarasu::watermark::processor::{WatermarkProcessor, WatermarkContext};
//!
//! let processor = WatermarkProcessor::new(fetcher);
//! let context = WatermarkContext { ... };
//!
//! // Apply watermarks from bucket config
//! let watermarked = processor.apply(&image, &config, &context).await?;
//! ```

use super::{
    apply_watermark, create_diagonal_layers, create_tiled_layers, parse_hex_color, render_text,
    resolve_template, BucketWatermarkConfig, Compositor, ImageDimensions, ImageFetcher,
    TemplateContext, TextRenderOptions, WatermarkDefinition, WatermarkError, WatermarkPosition,
};
use aws_sdk_s3::Client as S3Client;
use image::{DynamicImage, RgbaImage};
use std::hash::{Hash, Hasher};

/// Context for watermark template resolution.
///
/// Contains request-specific information for resolving template variables.
#[derive(Debug, Clone, Default)]
pub struct WatermarkContext {
    /// Client IP address (X-Forwarded-For aware)
    pub client_ip: Option<String>,
    /// JWT claims as key-value pairs
    pub jwt_claims: std::collections::HashMap<String, String>,
    /// Request headers as key-value pairs
    pub headers: std::collections::HashMap<String, String>,
    /// Request path
    pub path: String,
    /// Bucket name
    pub bucket: String,
}

impl WatermarkContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the client IP.
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = Some(ip.into());
        self
    }

    /// Add a JWT claim.
    pub fn with_claim(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.jwt_claims.insert(key.into(), value.into());
        self
    }

    /// Add a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the request path.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set the bucket name.
    pub fn with_bucket(mut self, bucket: impl Into<String>) -> Self {
        self.bucket = bucket.into();
        self
    }

    /// Convert to TemplateContext for template resolution.
    pub fn to_template_context(&self) -> TemplateContext {
        let mut ctx = TemplateContext::new();

        if let Some(ref ip) = self.client_ip {
            ctx.set_ip(ip.clone());
        }

        for (key, value) in &self.jwt_claims {
            ctx.set_jwt_claim(key.clone(), value.clone());
        }

        for (key, value) in &self.headers {
            ctx.set_header(key.clone(), value.clone());
        }

        if !self.path.is_empty() {
            ctx.set_path(self.path.clone());
        }

        if !self.bucket.is_empty() {
            ctx.set_bucket(self.bucket.clone());
        }

        ctx
    }
}

/// Watermark processor for applying watermarks to images.
#[derive(Clone)]
pub struct WatermarkProcessor {
    /// Image fetcher for watermark images.
    fetcher: ImageFetcher,
}

impl WatermarkProcessor {
    /// Create a new watermark processor with the given image fetcher.
    pub fn new(fetcher: ImageFetcher) -> Self {
        Self { fetcher }
    }

    /// Apply watermarks to an image based on bucket configuration.
    ///
    /// Matches the request path against configured rules and applies
    /// matching watermarks in order.
    ///
    /// # Arguments
    ///
    /// * `image` - The image to watermark
    /// * `config` - Bucket watermark configuration
    /// * `context` - Request context for template resolution
    /// * `s3_client` - Optional S3 client for fetching image watermarks
    ///
    /// # Returns
    ///
    /// The watermarked image, or the original if no watermarks apply.
    pub async fn apply(
        &self,
        image: &DynamicImage,
        config: &BucketWatermarkConfig,
        context: &WatermarkContext,
        s3_client: Option<&S3Client>,
    ) -> Result<DynamicImage, WatermarkError> {
        if !config.enabled {
            return Ok(image.clone());
        }

        // Find matching rule
        let matching_rule = config
            .rules
            .iter()
            .find(|rule| glob_match(&rule.pattern, &context.path));

        let rule = match matching_rule {
            Some(r) => r,
            None => return Ok(image.clone()),
        };

        if rule.watermarks.is_empty() {
            return Ok(image.clone());
        }

        // Convert to RGBA for compositing
        let mut rgba = image.to_rgba8();
        let template_context = context.to_template_context();

        // Apply each watermark in order
        for watermark_def in &rule.watermarks {
            match watermark_def {
                WatermarkDefinition::Text(text_config) => {
                    self.apply_text_watermark(&mut rgba, text_config, &template_context)?;
                }
                WatermarkDefinition::Image(image_config) => {
                    self.apply_image_watermark(&mut rgba, image_config, s3_client)
                        .await?;
                }
            }
        }

        Ok(DynamicImage::ImageRgba8(rgba))
    }

    /// Apply a text watermark to the image.
    fn apply_text_watermark(
        &self,
        image: &mut RgbaImage,
        config: &super::TextWatermarkConfig,
        context: &TemplateContext,
    ) -> Result<(), WatermarkError> {
        // Resolve template variables
        let resolved_text = resolve_template(&config.text, context);

        if resolved_text.is_empty() {
            return Ok(());
        }

        // Parse color
        let color = parse_hex_color(&config.color)?;

        // Render text to image
        let options = TextRenderOptions {
            text: resolved_text,
            font_size: config.font_size as f32,
            color,
            opacity: config.opacity,
            rotation_degrees: config.rotation.map(|r| r as f32),
        };

        let text_image = render_text(&options)?;

        let image_dims = ImageDimensions {
            width: image.width(),
            height: image.height(),
        };

        // Apply based on position mode
        match config.position {
            WatermarkPosition::Tiled => {
                let spacing = config.tile_spacing.unwrap_or(100);
                let layers = create_tiled_layers(&text_image, &image_dims, spacing, config.opacity);
                let mut compositor = Compositor::new();
                for layer in layers {
                    compositor.add_layer(layer);
                }
                compositor.apply(image);
            }
            WatermarkPosition::DiagonalBand => {
                let spacing = config.tile_spacing.unwrap_or(100);
                let layers =
                    create_diagonal_layers(&text_image, &image_dims, spacing, config.opacity);
                let mut compositor = Compositor::new();
                for layer in layers {
                    compositor.add_layer(layer);
                }
                compositor.apply(image);
            }
            _ => {
                // Standard 9-grid positioning
                apply_watermark(
                    image,
                    &text_image,
                    config.position,
                    config.margin,
                    config.opacity,
                );
            }
        }

        Ok(())
    }

    /// Apply an image watermark to the image.
    async fn apply_image_watermark(
        &self,
        image: &mut RgbaImage,
        config: &super::ImageWatermarkConfig,
        s3_client: Option<&S3Client>,
    ) -> Result<(), WatermarkError> {
        // Fetch watermark image
        let cached = self.fetcher.fetch(&config.source, s3_client).await?;

        // Resize if dimensions specified
        let watermark_img = if config.width.is_some() || config.height.is_some() {
            resize_watermark_image(&cached.image, config.width, config.height)?
        } else {
            cached.image.to_rgba8()
        };

        let image_dims = ImageDimensions {
            width: image.width(),
            height: image.height(),
        };

        // Apply based on position mode
        // Note: Image watermarks use fixed spacing of 100px for tiled modes
        let default_spacing = 100;

        match config.position {
            WatermarkPosition::Tiled => {
                let layers = create_tiled_layers(
                    &watermark_img,
                    &image_dims,
                    default_spacing,
                    config.opacity,
                );
                let mut compositor = Compositor::new();
                for layer in layers {
                    compositor.add_layer(layer);
                }
                compositor.apply(image);
            }
            WatermarkPosition::DiagonalBand => {
                let layers = create_diagonal_layers(
                    &watermark_img,
                    &image_dims,
                    default_spacing,
                    config.opacity,
                );
                let mut compositor = Compositor::new();
                for layer in layers {
                    compositor.add_layer(layer);
                }
                compositor.apply(image);
            }
            _ => {
                apply_watermark(
                    image,
                    &watermark_img,
                    config.position,
                    config.margin,
                    config.opacity,
                );
            }
        }

        Ok(())
    }

    /// Generate a cache key component for watermark configuration.
    ///
    /// The hash includes resolved template text and all watermark parameters.
    pub fn cache_key_hash(
        config: &BucketWatermarkConfig,
        path: &str,
        context: &WatermarkContext,
    ) -> Option<String> {
        if !config.enabled {
            return None;
        }

        // Find matching rule
        let rule = config
            .rules
            .iter()
            .find(|rule| glob_match(&rule.pattern, path))?;

        if rule.watermarks.is_empty() {
            return None;
        }

        // Build hash from all watermark configurations
        let template_context = context.to_template_context();
        let mut hash_input = String::new();

        for watermark_def in &rule.watermarks {
            match watermark_def {
                WatermarkDefinition::Text(text_config) => {
                    // Include resolved text in hash
                    let resolved = resolve_template(&text_config.text, &template_context);
                    hash_input.push_str(&format!(
                        "text:{}:{}:{}:{:?}:{}:",
                        resolved,
                        text_config.font_size,
                        text_config.color,
                        text_config.position,
                        text_config.opacity
                    ));
                }
                WatermarkDefinition::Image(image_config) => {
                    hash_input.push_str(&format!(
                        "image:{}:{:?}:{:?}:{:?}:{}:",
                        image_config.source,
                        image_config.width,
                        image_config.height,
                        image_config.position,
                        image_config.opacity
                    ));
                }
            }
        }

        // Generate hash from input string
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hash_input.hash(&mut hasher);
        Some(format!("wm_{:x}", hasher.finish()))
    }
}

/// Simple glob pattern matching.
///
/// Supports:
/// - `*` matches any sequence of characters except `/`
/// - `**` matches any sequence including `/`
/// - `?` matches any single character
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let pattern_chars: Vec<char> = pattern.chars().collect();
    let path_chars: Vec<char> = path.chars().collect();

    glob_match_recursive(&pattern_chars, &path_chars, 0, 0)
}

fn glob_match_recursive(pattern: &[char], path: &[char], mut pi: usize, mut si: usize) -> bool {
    while pi < pattern.len() && si < path.len() {
        match pattern[pi] {
            '*' => {
                // Check for **
                if pi + 1 < pattern.len() && pattern[pi + 1] == '*' {
                    // ** matches anything including /
                    pi += 2;
                    // Skip trailing / if present
                    if pi < pattern.len() && pattern[pi] == '/' {
                        pi += 1;
                    }
                    // Try matching rest at each position
                    for i in si..=path.len() {
                        if glob_match_recursive(pattern, path, pi, i) {
                            return true;
                        }
                    }
                    return false;
                } else {
                    // * matches anything except /
                    pi += 1;
                    // Try matching rest at each position (stopping at /)
                    for i in si..=path.len() {
                        if i > si && path[i - 1] == '/' {
                            break;
                        }
                        if glob_match_recursive(pattern, path, pi, i) {
                            return true;
                        }
                    }
                    return false;
                }
            }
            '?' => {
                // ? matches any single character except /
                if path[si] == '/' {
                    return false;
                }
                pi += 1;
                si += 1;
            }
            c => {
                if c != path[si] {
                    return false;
                }
                pi += 1;
                si += 1;
            }
        }
    }

    // Skip trailing wildcards in pattern
    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }

    pi == pattern.len() && si == path.len()
}

/// Resize a watermark image while preserving aspect ratio.
fn resize_watermark_image(
    image: &DynamicImage,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<RgbaImage, WatermarkError> {
    let src_w = image.width();
    let src_h = image.height();

    let (target_w, target_h) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            // Calculate height to preserve aspect ratio
            let h = (w as f32 / src_w as f32 * src_h as f32) as u32;
            (w, h.max(1))
        }
        (None, Some(h)) => {
            // Calculate width to preserve aspect ratio
            let w = (h as f32 / src_h as f32 * src_w as f32) as u32;
            (w.max(1), h)
        }
        (None, None) => return Ok(image.to_rgba8()),
    };

    let resized = image.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
    Ok(resized.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watermark::{ImageFetcherConfig, TextWatermarkConfig, WatermarkRule};

    // Test: Glob pattern matching
    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("/products/image.jpg", "/products/image.jpg"));
        assert!(!glob_match("/products/image.jpg", "/products/other.jpg"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("/products/*", "/products/image.jpg"));
        assert!(glob_match("/products/*.jpg", "/products/image.jpg"));
        assert!(!glob_match("/products/*.jpg", "/products/image.png"));
        assert!(!glob_match("/products/*", "/products/subdir/image.jpg"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("/products/**", "/products/image.jpg"));
        assert!(glob_match("/products/**", "/products/subdir/image.jpg"));
        assert!(glob_match(
            "/products/**/*.jpg",
            "/products/a/b/c/image.jpg"
        ));
        assert!(glob_match("**", "/any/path/here.jpg"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("/products/image?.jpg", "/products/image1.jpg"));
        assert!(!glob_match("/products/image?.jpg", "/products/image12.jpg"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        assert!(glob_match("*", "/products/image.jpg"));
    }

    // Test: Watermark context
    #[test]
    fn test_watermark_context_builder() {
        let context = WatermarkContext::new()
            .with_ip("192.168.1.1")
            .with_claim("sub", "user123")
            .with_header("X-Custom", "value")
            .with_path("/products/image.jpg")
            .with_bucket("my-bucket");

        assert_eq!(context.client_ip, Some("192.168.1.1".to_string()));
        assert_eq!(context.jwt_claims.get("sub"), Some(&"user123".to_string()));
        assert_eq!(context.headers.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(context.path, "/products/image.jpg");
        assert_eq!(context.bucket, "my-bucket");
    }

    #[test]
    fn test_watermark_context_to_template() {
        let context = WatermarkContext::new()
            .with_ip("10.0.0.1")
            .with_claim("org", "Acme");

        let template_ctx = context.to_template_context();
        assert_eq!(template_ctx.ip(), Some("10.0.0.1"));
        assert_eq!(template_ctx.jwt_claim("org"), Some("Acme"));
    }

    // Test: Processor creation
    #[test]
    fn test_processor_creation() {
        let fetcher = ImageFetcher::new(ImageFetcherConfig::default());
        let _processor = WatermarkProcessor::new(fetcher);
    }

    // Test: Cache key hash
    #[test]
    fn test_cache_key_hash_disabled() {
        let config = BucketWatermarkConfig {
            enabled: false,
            cache_ttl_seconds: 3600,
            rules: vec![],
        };
        let context = WatermarkContext::new();

        let hash = WatermarkProcessor::cache_key_hash(&config, "/path", &context);
        assert!(hash.is_none());
    }

    #[test]
    fn test_cache_key_hash_no_matching_rule() {
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "/other/*".to_string(),
                watermarks: vec![WatermarkDefinition::Text(TextWatermarkConfig {
                    text: "Test".to_string(),
                    font_size: 24,
                    color: "#FFFFFF".to_string(),
                    opacity: 0.5,
                    position: WatermarkPosition::BottomRight,
                    margin: 10,
                    rotation: None,
                    tile_spacing: None,
                })],
            }],
        };
        let context = WatermarkContext::new();

        let hash = WatermarkProcessor::cache_key_hash(&config, "/products/image.jpg", &context);
        assert!(hash.is_none());
    }

    #[test]
    fn test_cache_key_hash_with_matching_rule() {
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "/products/*".to_string(),
                watermarks: vec![WatermarkDefinition::Text(TextWatermarkConfig {
                    text: "Copyright {{ip}}".to_string(),
                    font_size: 24,
                    color: "#FFFFFF".to_string(),
                    opacity: 0.5,
                    position: WatermarkPosition::BottomRight,
                    margin: 10,
                    rotation: None,
                    tile_spacing: None,
                })],
            }],
        };
        let context = WatermarkContext::new().with_ip("192.168.1.1");

        let hash = WatermarkProcessor::cache_key_hash(&config, "/products/image.jpg", &context);
        assert!(hash.is_some());

        // Different IP should produce different hash
        let context2 = WatermarkContext::new().with_ip("10.0.0.1");
        let hash2 = WatermarkProcessor::cache_key_hash(&config, "/products/image.jpg", &context2);
        assert!(hash2.is_some());
        assert_ne!(hash, hash2);
    }

    // Test: Resize watermark image
    #[test]
    fn test_resize_watermark_width_only() {
        let img = DynamicImage::new_rgba8(100, 50);
        let resized = resize_watermark_image(&img, Some(50), None).unwrap();
        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 25); // Maintains 2:1 aspect ratio
    }

    #[test]
    fn test_resize_watermark_height_only() {
        let img = DynamicImage::new_rgba8(100, 50);
        let resized = resize_watermark_image(&img, None, Some(25)).unwrap();
        assert_eq!(resized.width(), 50);
        assert_eq!(resized.height(), 25);
    }

    #[test]
    fn test_resize_watermark_both_dimensions() {
        let img = DynamicImage::new_rgba8(100, 50);
        let resized = resize_watermark_image(&img, Some(200), Some(100)).unwrap();
        assert_eq!(resized.width(), 200);
        assert_eq!(resized.height(), 100);
    }

    #[test]
    fn test_resize_watermark_no_dimensions() {
        let img = DynamicImage::new_rgba8(100, 50);
        let resized = resize_watermark_image(&img, None, None).unwrap();
        assert_eq!(resized.width(), 100);
        assert_eq!(resized.height(), 50);
    }

    // Integration test: Apply text watermark
    #[tokio::test]
    async fn test_apply_no_watermark_when_disabled() {
        let fetcher = ImageFetcher::new(ImageFetcherConfig::default());
        let processor = WatermarkProcessor::new(fetcher);

        let img = DynamicImage::new_rgba8(100, 100);
        let config = BucketWatermarkConfig {
            enabled: false,
            cache_ttl_seconds: 3600,
            rules: vec![],
        };
        let context = WatermarkContext::new();

        let result = processor.apply(&img, &config, &context, None).await;
        assert!(result.is_ok());
        // Image should be unchanged
        let output = result.unwrap();
        assert_eq!(output.width(), 100);
        assert_eq!(output.height(), 100);
    }

    #[tokio::test]
    async fn test_apply_no_watermark_when_no_matching_rule() {
        let fetcher = ImageFetcher::new(ImageFetcherConfig::default());
        let processor = WatermarkProcessor::new(fetcher);

        let img = DynamicImage::new_rgba8(100, 100);
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "/other/*".to_string(),
                watermarks: vec![],
            }],
        };
        let context = WatermarkContext::new().with_path("/products/image.jpg");

        let result = processor.apply(&img, &config, &context, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_apply_text_watermark() {
        let fetcher = ImageFetcher::new(ImageFetcherConfig::default());
        let processor = WatermarkProcessor::new(fetcher);

        let img = DynamicImage::new_rgba8(200, 200);
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "*".to_string(),
                watermarks: vec![WatermarkDefinition::Text(TextWatermarkConfig {
                    text: "TEST".to_string(),
                    font_size: 24,
                    color: "#FF0000".to_string(),
                    opacity: 1.0,
                    position: WatermarkPosition::Center,
                    margin: 0,
                    rotation: None,
                    tile_spacing: None,
                })],
            }],
        };
        let context = WatermarkContext::new().with_path("/image.jpg");

        let result = processor.apply(&img, &config, &context, None).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.width(), 200);
        assert_eq!(output.height(), 200);

        // Check that some pixels have been modified (watermark applied)
        let rgba = output.to_rgba8();
        let has_red = rgba.pixels().any(|p| p[0] > 0 && p[3] > 0);
        assert!(has_red, "Watermark should have added red pixels");
    }
}
