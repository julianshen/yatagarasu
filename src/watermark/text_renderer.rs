//! Text watermark rendering.
//!
//! This module renders text watermarks to RGBA images that can be
//! composited onto target images.
//!
//! # Features
//!
//! - Hex color parsing (#RGB and #RRGGBB formats)
//! - Configurable font size and opacity
//! - Text rotation for diagonal watermarks
//! - Embedded default font (no external dependencies)
//!
//! # Example
//!
//! ```ignore
//! use yatagarasu::watermark::text_renderer::{render_text, TextRenderOptions, parse_hex_color};
//!
//! let options = TextRenderOptions {
//!     text: "Copyright 2025".to_string(),
//!     font_size: 24.0,
//!     color: parse_hex_color("#FFFFFF").unwrap(),
//!     opacity: 0.5,
//!     rotation_degrees: None,
//! };
//!
//! let image = render_text(&options).unwrap();
//! ```

use super::WatermarkError;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use std::sync::OnceLock;

/// Default embedded font (DejaVu Sans Mono - subset for common characters).
/// Using a monospace font for predictable width calculations.
/// This is a minimal embedded font for basic text rendering.
static DEFAULT_FONT: OnceLock<FontRef<'static>> = OnceLock::new();

/// Embedded font data (Liberation Mono - OFL licensed, commonly available).
/// For production, this could be replaced with a configurable font path.
const EMBEDDED_FONT_DATA: &[u8] = include_bytes!("fonts/DejaVuSansMono.ttf");

/// Get the default font, initializing it lazily.
fn get_default_font() -> Result<&'static FontRef<'static>, WatermarkError> {
    DEFAULT_FONT.get_or_init(|| {
        FontRef::try_from_slice(EMBEDDED_FONT_DATA)
            .expect("Failed to load embedded font - this is a bug")
    });

    DEFAULT_FONT
        .get()
        .ok_or_else(|| WatermarkError::RenderError("Failed to initialize font".to_string()))
}

/// Parsed RGBA color from hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// White color.
    pub fn white() -> Self {
        Self::new(255, 255, 255)
    }

    /// Black color.
    pub fn black() -> Self {
        Self::new(0, 0, 0)
    }
}

/// Options for text rendering.
#[derive(Debug, Clone)]
pub struct TextRenderOptions {
    /// The text to render.
    pub text: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// Text color (RGB).
    pub color: Color,
    /// Opacity (0.0 to 1.0).
    pub opacity: f32,
    /// Rotation in degrees (clockwise). None means no rotation.
    pub rotation_degrees: Option<f32>,
}

impl Default for TextRenderOptions {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_size: 24.0,
            color: Color::white(),
            opacity: 0.5,
            rotation_degrees: None,
        }
    }
}

/// Parse a hex color string into RGB components.
///
/// Supports both #RGB and #RRGGBB formats.
///
/// # Arguments
///
/// * `hex` - Color string starting with '#'
///
/// # Returns
///
/// Parsed Color or error if invalid format.
///
/// # Examples
///
/// ```ignore
/// let white = parse_hex_color("#FFF").unwrap();
/// assert_eq!(white, Color::new(255, 255, 255));
///
/// let red = parse_hex_color("#FF0000").unwrap();
/// assert_eq!(red, Color::new(255, 0, 0));
/// ```
pub fn parse_hex_color(hex: &str) -> Result<Color, WatermarkError> {
    let hex = hex
        .strip_prefix('#')
        .ok_or_else(|| WatermarkError::RenderError("Color must start with '#'".to_string()))?;

    match hex.len() {
        3 => {
            // #RGB format - each character represents a hex digit, doubled
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| WatermarkError::RenderError("Invalid hex digit".to_string()))?;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| WatermarkError::RenderError("Invalid hex digit".to_string()))?;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| WatermarkError::RenderError("Invalid hex digit".to_string()))?;
            // Double each component: 0xF -> 0xFF, 0xA -> 0xAA
            Ok(Color::new(r * 17, g * 17, b * 17))
        }
        6 => {
            // #RRGGBB format
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| WatermarkError::RenderError("Invalid hex digit".to_string()))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| WatermarkError::RenderError("Invalid hex digit".to_string()))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| WatermarkError::RenderError("Invalid hex digit".to_string()))?;
            Ok(Color::new(r, g, b))
        }
        _ => Err(WatermarkError::RenderError(format!(
            "Color must be #RGB or #RRGGBB format, got {} characters",
            hex.len()
        ))),
    }
}

/// Calculate the dimensions of rendered text.
///
/// Returns (width, height) in pixels.
pub fn measure_text(text: &str, font_size: f32) -> Result<(u32, u32), WatermarkError> {
    let font = get_default_font()?;
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);

    let mut width = 0.0f32;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

    for c in text.chars() {
        let glyph_id = scaled_font.glyph_id(c);

        // Add kerning if there's a previous glyph
        if let Some(prev) = prev_glyph {
            width += scaled_font.kern(prev, glyph_id);
        }

        width += scaled_font.h_advance(glyph_id);
        prev_glyph = Some(glyph_id);
    }

    let height = scaled_font.height();

    // Add small padding
    let padding = 2;
    Ok((
        width.ceil() as u32 + padding,
        height.ceil() as u32 + padding,
    ))
}

/// Render text to an RGBA image.
///
/// Creates a new image with transparent background containing the rendered text.
///
/// # Arguments
///
/// * `options` - Text rendering options
///
/// # Returns
///
/// An RGBA image containing the rendered text, or error if rendering fails.
pub fn render_text(options: &TextRenderOptions) -> Result<RgbaImage, WatermarkError> {
    if options.text.is_empty() {
        return Err(WatermarkError::RenderError(
            "Cannot render empty text".to_string(),
        ));
    }

    let font = get_default_font()?;
    let scale = PxScale::from(options.font_size);
    let scaled_font = font.as_scaled(scale);

    // Calculate dimensions
    let (width, height) = measure_text(&options.text, options.font_size)?;

    // Handle rotation - need larger canvas
    let (canvas_width, canvas_height, offset_x, offset_y) =
        if let Some(degrees) = options.rotation_degrees {
            let radians = degrees.to_radians();
            let cos = radians.cos().abs();
            let sin = radians.sin().abs();

            // Rotated bounding box
            let rotated_width = (width as f32 * cos + height as f32 * sin).ceil() as u32;
            let rotated_height = (width as f32 * sin + height as f32 * cos).ceil() as u32;

            // Offset to center the text in the rotated canvas
            let ox = (rotated_width.saturating_sub(width)) / 2;
            let oy = (rotated_height.saturating_sub(height)) / 2;

            (rotated_width.max(1), rotated_height.max(1), ox, oy)
        } else {
            (width.max(1), height.max(1), 0, 0)
        };

    // Create transparent image
    let mut image = RgbaImage::new(canvas_width, canvas_height);

    // Calculate alpha from opacity
    let alpha = (options.opacity.clamp(0.0, 1.0) * 255.0) as u8;

    // Baseline position
    let ascent = scaled_font.ascent();
    let baseline_y = offset_y as f32 + ascent;

    // Render each glyph
    let mut cursor_x = offset_x as f32;
    let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

    for c in options.text.chars() {
        let glyph_id = scaled_font.glyph_id(c);

        // Add kerning
        if let Some(prev) = prev_glyph {
            cursor_x += scaled_font.kern(prev, glyph_id);
        }

        // Create positioned glyph
        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(cursor_x, baseline_y));

        // Render the glyph
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();

            outlined.draw(|px, py, coverage| {
                let x = px as i32 + bounds.min.x as i32;
                let y = py as i32 + bounds.min.y as i32;

                if x >= 0 && y >= 0 && x < canvas_width as i32 && y < canvas_height as i32 {
                    let pixel_alpha = (coverage * alpha as f32) as u8;
                    let pixel = Rgba([
                        options.color.r,
                        options.color.g,
                        options.color.b,
                        pixel_alpha,
                    ]);

                    // Blend with existing pixel (for anti-aliasing)
                    let existing = image.get_pixel(x as u32, y as u32);
                    let blended = blend_pixels(*existing, pixel);
                    image.put_pixel(x as u32, y as u32, blended);
                }
            });
        }

        cursor_x += scaled_font.h_advance(glyph_id);
        prev_glyph = Some(glyph_id);
    }

    // Apply rotation if needed
    if let Some(degrees) = options.rotation_degrees {
        image = rotate_image(&image, degrees);
    }

    Ok(image)
}

/// Blend two RGBA pixels using alpha compositing.
fn blend_pixels(bottom: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    let top_alpha = top[3] as f32 / 255.0;
    let bottom_alpha = bottom[3] as f32 / 255.0;

    let out_alpha = top_alpha + bottom_alpha * (1.0 - top_alpha);

    if out_alpha < 0.001 {
        return Rgba([0, 0, 0, 0]);
    }

    let blend = |t: u8, b: u8| -> u8 {
        let t = t as f32 / 255.0;
        let b = b as f32 / 255.0;
        let result = (t * top_alpha + b * bottom_alpha * (1.0 - top_alpha)) / out_alpha;
        (result * 255.0) as u8
    };

    Rgba([
        blend(top[0], bottom[0]),
        blend(top[1], bottom[1]),
        blend(top[2], bottom[2]),
        (out_alpha * 255.0) as u8,
    ])
}

/// Rotate an image by the specified degrees (clockwise).
fn rotate_image(image: &RgbaImage, degrees: f32) -> RgbaImage {
    let radians = -degrees.to_radians(); // Negative for clockwise
    let cos = radians.cos();
    let sin = radians.sin();

    let src_w = image.width() as f32;
    let src_h = image.height() as f32;
    let cx = src_w / 2.0;
    let cy = src_h / 2.0;

    // Calculate rotated bounding box
    let corners = [
        (-cx, -cy),
        (src_w - cx, -cy),
        (-cx, src_h - cy),
        (src_w - cx, src_h - cy),
    ];

    let rotated_corners: Vec<(f32, f32)> = corners
        .iter()
        .map(|(x, y)| (x * cos - y * sin, x * sin + y * cos))
        .collect();

    let min_x = rotated_corners
        .iter()
        .map(|(x, _)| *x)
        .fold(f32::INFINITY, f32::min);
    let max_x = rotated_corners
        .iter()
        .map(|(x, _)| *x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = rotated_corners
        .iter()
        .map(|(_, y)| *y)
        .fold(f32::INFINITY, f32::min);
    let max_y = rotated_corners
        .iter()
        .map(|(_, y)| *y)
        .fold(f32::NEG_INFINITY, f32::max);

    let dst_w = (max_x - min_x).ceil() as u32;
    let dst_h = (max_y - min_y).ceil() as u32;

    let mut rotated = RgbaImage::new(dst_w.max(1), dst_h.max(1));

    let dst_cx = dst_w as f32 / 2.0;
    let dst_cy = dst_h as f32 / 2.0;

    // Inverse rotation for sampling
    let inv_cos = (-radians).cos();
    let inv_sin = (-radians).sin();

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            // Transform destination pixel to source coordinates
            let rx = dx as f32 - dst_cx;
            let ry = dy as f32 - dst_cy;

            let sx = rx * inv_cos - ry * inv_sin + cx;
            let sy = rx * inv_sin + ry * inv_cos + cy;

            // Bilinear interpolation
            if sx >= 0.0 && sx < src_w - 1.0 && sy >= 0.0 && sy < src_h - 1.0 {
                let x0 = sx.floor() as u32;
                let y0 = sy.floor() as u32;
                let x1 = x0 + 1;
                let y1 = y0 + 1;

                let fx = sx - x0 as f32;
                let fy = sy - y0 as f32;

                let p00 = image.get_pixel(x0, y0);
                let p10 = image.get_pixel(x1, y0);
                let p01 = image.get_pixel(x0, y1);
                let p11 = image.get_pixel(x1, y1);

                let interpolate = |c: usize| -> u8 {
                    let v00 = p00[c] as f32;
                    let v10 = p10[c] as f32;
                    let v01 = p01[c] as f32;
                    let v11 = p11[c] as f32;

                    let v = v00 * (1.0 - fx) * (1.0 - fy)
                        + v10 * fx * (1.0 - fy)
                        + v01 * (1.0 - fx) * fy
                        + v11 * fx * fy;

                    v.clamp(0.0, 255.0) as u8
                };

                rotated.put_pixel(
                    dx,
                    dy,
                    Rgba([
                        interpolate(0),
                        interpolate(1),
                        interpolate(2),
                        interpolate(3),
                    ]),
                );
            }
        }
    }

    rotated
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test: Hex color parsing (#RGB, #RRGGBB)
    #[test]
    fn test_parse_hex_color_rrggbb() {
        let color = parse_hex_color("#FF0000").unwrap();
        assert_eq!(color, Color::new(255, 0, 0));

        let color = parse_hex_color("#00FF00").unwrap();
        assert_eq!(color, Color::new(0, 255, 0));

        let color = parse_hex_color("#0000FF").unwrap();
        assert_eq!(color, Color::new(0, 0, 255));

        let color = parse_hex_color("#FFFFFF").unwrap();
        assert_eq!(color, Color::new(255, 255, 255));

        let color = parse_hex_color("#000000").unwrap();
        assert_eq!(color, Color::new(0, 0, 0));
    }

    #[test]
    fn test_parse_hex_color_rgb() {
        let color = parse_hex_color("#F00").unwrap();
        assert_eq!(color, Color::new(255, 0, 0));

        let color = parse_hex_color("#0F0").unwrap();
        assert_eq!(color, Color::new(0, 255, 0));

        let color = parse_hex_color("#00F").unwrap();
        assert_eq!(color, Color::new(0, 0, 255));

        let color = parse_hex_color("#FFF").unwrap();
        assert_eq!(color, Color::new(255, 255, 255));

        let color = parse_hex_color("#ABC").unwrap();
        // A=10*17=170, B=11*17=187, C=12*17=204
        assert_eq!(color, Color::new(170, 187, 204));
    }

    #[test]
    fn test_parse_hex_color_lowercase() {
        let color = parse_hex_color("#ff0000").unwrap();
        assert_eq!(color, Color::new(255, 0, 0));

        let color = parse_hex_color("#abc").unwrap();
        assert_eq!(color, Color::new(170, 187, 204));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        // Missing #
        assert!(parse_hex_color("FF0000").is_err());

        // Wrong length
        assert!(parse_hex_color("#FF00").is_err());
        assert!(parse_hex_color("#FF00000").is_err());

        // Invalid hex
        assert!(parse_hex_color("#GGGGGG").is_err());
    }

    // Tests for text rendering with embedded font

    #[test]
    fn test_render_text_creates_rgba_image() {
        let options = TextRenderOptions {
            text: "Hello".to_string(),
            font_size: 24.0,
            color: Color::white(),
            opacity: 1.0,
            rotation_degrees: None,
        };

        let image = render_text(&options).unwrap();

        // Image should have reasonable dimensions
        assert!(image.width() > 0);
        assert!(image.height() > 0);

        // Should have some non-transparent pixels
        let has_content = image.pixels().any(|p| p[3] > 0);
        assert!(has_content, "Rendered text should have visible pixels");
    }

    #[test]
    fn test_render_text_opacity_affects_alpha() {
        let options_full = TextRenderOptions {
            text: "Test".to_string(),
            font_size: 24.0,
            color: Color::white(),
            opacity: 1.0,
            rotation_degrees: None,
        };

        let options_half = TextRenderOptions {
            text: "Test".to_string(),
            font_size: 24.0,
            color: Color::white(),
            opacity: 0.5,
            rotation_degrees: None,
        };

        let image_full = render_text(&options_full).unwrap();
        let image_half = render_text(&options_half).unwrap();

        // Find max alpha in each image
        let max_alpha_full = image_full.pixels().map(|p| p[3]).max().unwrap_or(0);
        let max_alpha_half = image_half.pixels().map(|p| p[3]).max().unwrap_or(0);

        // Half opacity should have roughly half the max alpha
        assert!(max_alpha_half < max_alpha_full);
    }

    #[test]
    fn test_font_size_affects_dimensions() {
        let (w1, h1) = measure_text("Hello", 12.0).unwrap();
        let (w2, h2) = measure_text("Hello", 24.0).unwrap();
        let (w3, h3) = measure_text("Hello", 48.0).unwrap();

        // Larger font should produce larger dimensions
        assert!(w2 > w1);
        assert!(h2 > h1);
        assert!(w3 > w2);
        assert!(h3 > h2);
    }

    #[test]
    fn test_render_text_with_rotation() {
        let options = TextRenderOptions {
            text: "Rotated".to_string(),
            font_size: 24.0,
            color: Color::white(),
            opacity: 1.0,
            rotation_degrees: Some(45.0),
        };

        let image = render_text(&options).unwrap();

        // Rotated image should still have content
        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }

    #[test]
    fn test_render_empty_text_error() {
        let options = TextRenderOptions {
            text: String::new(),
            font_size: 24.0,
            color: Color::white(),
            opacity: 1.0,
            rotation_degrees: None,
        };

        let result = render_text(&options);
        assert!(result.is_err());
    }

    // Test color helpers
    #[test]
    fn test_color_helpers() {
        assert_eq!(Color::white(), Color::new(255, 255, 255));
        assert_eq!(Color::black(), Color::new(0, 0, 0));
    }

    // Test default options
    #[test]
    fn test_text_render_options_default() {
        let options = TextRenderOptions::default();
        assert!(options.text.is_empty());
        assert_eq!(options.font_size, 24.0);
        assert_eq!(options.color, Color::white());
        assert_eq!(options.opacity, 0.5);
        assert!(options.rotation_degrees.is_none());
    }
}
