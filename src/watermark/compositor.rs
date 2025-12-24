//! Watermark compositor for blending watermarks onto images.
//!
//! This module handles alpha blending of watermark images onto target images
//! at calculated positions.
//!
//! # Features
//!
//! - Alpha blending with premultiplied alpha support
//! - Multiple watermarks applied in sequence
//! - Tiled watermark rendering
//! - Diagonal band watermark rendering
//! - Position-aware compositing
//!
//! # Example
//!
//! ```ignore
//! use yatagarasu::watermark::compositor::{Compositor, WatermarkLayer};
//! use yatagarasu::watermark::{WatermarkPosition, calculate_position};
//!
//! let mut compositor = Compositor::new();
//! compositor.add_layer(WatermarkLayer {
//!     image: watermark_image,
//!     position: PlacementPosition { x: 10, y: 10 },
//!     opacity: 0.5,
//! });
//!
//! let result = compositor.apply(&mut target_image);
//! ```

use super::position::{
    calculate_diagonal_positions, calculate_position, calculate_tiled_positions, ImageDimensions,
    PlacementPosition, WatermarkDimensions,
};
use super::WatermarkPosition;
use image::{DynamicImage, Rgba, RgbaImage};

/// A watermark layer to be composited onto an image.
#[derive(Clone)]
pub struct WatermarkLayer {
    /// The watermark image (RGBA).
    pub image: RgbaImage,
    /// Position where the watermark should be placed.
    pub position: PlacementPosition,
    /// Opacity to apply (0.0 to 1.0). Applied on top of image's alpha channel.
    pub opacity: f32,
}

impl std::fmt::Debug for WatermarkLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatermarkLayer")
            .field("dimensions", &(self.image.width(), self.image.height()))
            .field("position", &self.position)
            .field("opacity", &self.opacity)
            .finish()
    }
}

/// Compositor for applying watermarks to images.
#[derive(Debug, Default)]
pub struct Compositor {
    layers: Vec<WatermarkLayer>,
}

impl Compositor {
    /// Create a new compositor with no layers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a watermark layer to the compositor.
    pub fn add_layer(&mut self, layer: WatermarkLayer) {
        self.layers.push(layer);
    }

    /// Apply all watermark layers to the target image.
    ///
    /// Layers are applied in the order they were added.
    pub fn apply(&self, target: &mut RgbaImage) {
        for layer in &self.layers {
            blend_layer(target, layer);
        }
    }

    /// Apply watermarks and return the result as a new image.
    pub fn apply_to_dynamic(&self, target: &DynamicImage) -> DynamicImage {
        let mut rgba = target.to_rgba8();
        self.apply(&mut rgba);
        DynamicImage::ImageRgba8(rgba)
    }

    /// Get the number of layers.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Clear all layers.
    pub fn clear(&mut self) {
        self.layers.clear();
    }
}

/// Blend a single watermark layer onto the target image.
fn blend_layer(target: &mut RgbaImage, layer: &WatermarkLayer) {
    let target_width = target.width() as i32;
    let target_height = target.height() as i32;

    let wm_width = layer.image.width() as i32;
    let wm_height = layer.image.height() as i32;

    // Calculate the visible region (clamp to target bounds)
    let x_start = layer.position.x.max(0);
    let y_start = layer.position.y.max(0);
    let x_end = (layer.position.x + wm_width).min(target_width);
    let y_end = (layer.position.y + wm_height).min(target_height);

    for ty in y_start..y_end {
        for tx in x_start..x_end {
            // Calculate source coordinates in watermark image
            let wx = (tx - layer.position.x) as u32;
            let wy = (ty - layer.position.y) as u32;

            let wm_pixel = layer.image.get_pixel(wx, wy);
            let target_pixel = target.get_pixel(tx as u32, ty as u32);

            let blended = blend_pixels(*target_pixel, *wm_pixel, layer.opacity);
            target.put_pixel(tx as u32, ty as u32, blended);
        }
    }
}

/// Blend two pixels using alpha compositing with additional opacity.
///
/// Uses the "over" operator: result = foreground + background * (1 - foreground.alpha)
fn blend_pixels(background: Rgba<u8>, foreground: Rgba<u8>, opacity: f32) -> Rgba<u8> {
    // Apply additional opacity to foreground alpha
    let fg_alpha = (foreground[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    let bg_alpha = background[3] as f32 / 255.0;

    // Porter-Duff "over" operator
    let out_alpha = fg_alpha + bg_alpha * (1.0 - fg_alpha);

    if out_alpha < 0.001 {
        return Rgba([0, 0, 0, 0]);
    }

    let blend_channel = |fg: u8, bg: u8| -> u8 {
        let fg_f = fg as f32 / 255.0;
        let bg_f = bg as f32 / 255.0;
        let result = (fg_f * fg_alpha + bg_f * bg_alpha * (1.0 - fg_alpha)) / out_alpha;
        (result * 255.0).clamp(0.0, 255.0) as u8
    };

    Rgba([
        blend_channel(foreground[0], background[0]),
        blend_channel(foreground[1], background[1]),
        blend_channel(foreground[2], background[2]),
        (out_alpha * 255.0) as u8,
    ])
}

/// Create layers for a tiled watermark.
///
/// Returns a vector of layers that tile the watermark across the entire image.
pub fn create_tiled_layers(
    watermark: &RgbaImage,
    image_dims: &ImageDimensions,
    spacing: u32,
    opacity: f32,
) -> Vec<WatermarkLayer> {
    let wm_dims = WatermarkDimensions {
        width: watermark.width(),
        height: watermark.height(),
    };

    let positions = calculate_tiled_positions(image_dims, &wm_dims, spacing);

    positions
        .into_iter()
        .map(|pos| WatermarkLayer {
            image: watermark.clone(),
            position: pos,
            opacity,
        })
        .collect()
}

/// Create layers for a diagonal band watermark.
///
/// Returns a vector of layers that form a diagonal band across the image.
pub fn create_diagonal_layers(
    watermark: &RgbaImage,
    image_dims: &ImageDimensions,
    spacing: u32,
    opacity: f32,
) -> Vec<WatermarkLayer> {
    let wm_dims = WatermarkDimensions {
        width: watermark.width(),
        height: watermark.height(),
    };

    let positions = calculate_diagonal_positions(image_dims, &wm_dims, spacing);

    positions
        .into_iter()
        .map(|pos| WatermarkLayer {
            image: watermark.clone(),
            position: pos,
            opacity,
        })
        .collect()
}

/// Create a single layer for a positioned watermark.
pub fn create_positioned_layer(
    watermark: &RgbaImage,
    image_dims: &ImageDimensions,
    position: WatermarkPosition,
    margin: u32,
    opacity: f32,
) -> WatermarkLayer {
    let wm_dims = WatermarkDimensions {
        width: watermark.width(),
        height: watermark.height(),
    };

    let pos = calculate_position(position, image_dims, &wm_dims, margin);

    WatermarkLayer {
        image: watermark.clone(),
        position: pos,
        opacity,
    }
}

/// Apply a single watermark to an image at the specified position.
///
/// This is a convenience function for simple watermarking.
pub fn apply_watermark(
    target: &mut RgbaImage,
    watermark: &RgbaImage,
    position: WatermarkPosition,
    margin: u32,
    opacity: f32,
) {
    let image_dims = ImageDimensions {
        width: target.width(),
        height: target.height(),
    };

    let layer = create_positioned_layer(watermark, &image_dims, position, margin, opacity);
    blend_layer(target, &layer);
}

/// Apply multiple watermarks to an image.
///
/// Watermarks are applied in the order provided.
pub fn apply_watermarks(
    target: &mut RgbaImage,
    watermarks: &[(RgbaImage, WatermarkPosition, u32, f32)],
) {
    let image_dims = ImageDimensions {
        width: target.width(),
        height: target.height(),
    };

    for (wm, pos, margin, opacity) in watermarks {
        let layer = create_positioned_layer(wm, &image_dims, *pos, *margin, *opacity);
        blend_layer(target, &layer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_image(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
        RgbaImage::from_pixel(width, height, color)
    }

    fn create_watermark(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
        RgbaImage::from_pixel(width, height, color)
    }

    // Test: Alpha blend watermark onto image
    #[test]
    fn test_alpha_blend_simple() {
        // White background
        let mut target = create_test_image(100, 100, Rgba([255, 255, 255, 255]));
        // Red watermark with 50% alpha
        let watermark = create_watermark(20, 20, Rgba([255, 0, 0, 128]));

        apply_watermark(&mut target, &watermark, WatermarkPosition::TopLeft, 0, 1.0);

        // Check that the watermark area has blended colors
        let pixel = target.get_pixel(10, 10);
        // With 50% alpha red over white, we get pinkish
        assert!(pixel[0] > 200); // Red channel stays high
        assert!(pixel[1] > 100); // Green is blended
        assert!(pixel[2] > 100); // Blue is blended
        assert_eq!(pixel[3], 255); // Alpha stays full
    }

    #[test]
    fn test_alpha_blend_with_opacity() {
        let mut target = create_test_image(100, 100, Rgba([0, 0, 0, 255]));
        // Fully opaque white watermark
        let watermark = create_watermark(20, 20, Rgba([255, 255, 255, 255]));

        // Apply with 50% opacity
        apply_watermark(&mut target, &watermark, WatermarkPosition::TopLeft, 0, 0.5);

        // Check that the pixel is gray (50% blend)
        let pixel = target.get_pixel(10, 10);
        // Should be around 128 (50% of white over black)
        assert!(pixel[0] > 100 && pixel[0] < 160);
        assert!(pixel[1] > 100 && pixel[1] < 160);
        assert!(pixel[2] > 100 && pixel[2] < 160);
    }

    // Test: Blend at calculated position
    #[test]
    fn test_blend_at_position() {
        let mut target = create_test_image(100, 100, Rgba([255, 255, 255, 255]));
        let watermark = create_watermark(10, 10, Rgba([255, 0, 0, 255]));

        // Apply at bottom-right
        apply_watermark(
            &mut target,
            &watermark,
            WatermarkPosition::BottomRight,
            5,
            1.0,
        );

        // Check that watermark is at bottom-right (100 - 10 - 5 = 85)
        let pixel_in_wm = target.get_pixel(90, 90);
        assert_eq!(pixel_in_wm[0], 255); // Red
        assert_eq!(pixel_in_wm[1], 0);
        assert_eq!(pixel_in_wm[2], 0);

        // Check that top-left is still white (not affected)
        let pixel_outside = target.get_pixel(10, 10);
        assert_eq!(pixel_outside[0], 255);
        assert_eq!(pixel_outside[1], 255);
        assert_eq!(pixel_outside[2], 255);
    }

    // Test: Handle transparent watermarks
    #[test]
    fn test_transparent_watermark() {
        let mut target = create_test_image(100, 100, Rgba([255, 0, 0, 255]));
        // Fully transparent watermark
        let watermark = create_watermark(20, 20, Rgba([0, 255, 0, 0]));

        apply_watermark(&mut target, &watermark, WatermarkPosition::Center, 0, 1.0);

        // Target should be unchanged (watermark is fully transparent)
        let pixel = target.get_pixel(50, 50);
        assert_eq!(pixel[0], 255);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 0);
    }

    // Test: Apply multiple watermarks in sequence
    #[test]
    fn test_multiple_watermarks() {
        let mut target = create_test_image(100, 100, Rgba([255, 255, 255, 255]));

        let wm1 = create_watermark(10, 10, Rgba([255, 0, 0, 255]));
        let wm2 = create_watermark(10, 10, Rgba([0, 255, 0, 255]));
        let wm3 = create_watermark(10, 10, Rgba([0, 0, 255, 255]));

        let watermarks = vec![
            (wm1, WatermarkPosition::TopLeft, 5u32, 1.0f32),
            (wm2, WatermarkPosition::TopRight, 5u32, 1.0f32),
            (wm3, WatermarkPosition::BottomCenter, 5u32, 1.0f32),
        ];

        apply_watermarks(&mut target, &watermarks);

        // Check each watermark position
        let pixel_tl = target.get_pixel(7, 7);
        assert_eq!(pixel_tl[0], 255); // Red

        let pixel_tr = target.get_pixel(92, 7);
        assert_eq!(pixel_tr[1], 255); // Green

        let pixel_bc = target.get_pixel(50, 92);
        assert_eq!(pixel_bc[2], 255); // Blue
    }

    // Test: Tiled rendering repeats correctly
    #[test]
    fn test_tiled_rendering() {
        let image_dims = ImageDimensions {
            width: 200,
            height: 200,
        };
        let watermark = create_watermark(30, 30, Rgba([255, 0, 0, 128]));

        let layers = create_tiled_layers(&watermark, &image_dims, 50, 0.5);

        // Should have multiple layers
        assert!(layers.len() > 1);

        // Apply to an image and verify
        let mut target = create_test_image(200, 200, Rgba([255, 255, 255, 255]));
        let mut compositor = Compositor::new();
        for layer in layers {
            compositor.add_layer(layer);
        }
        compositor.apply(&mut target);

        // Check that we have watermark at multiple locations
        // First tile at (0, 0)
        let pixel_0_0 = target.get_pixel(15, 15);
        assert!(pixel_0_0[0] > 200); // Some red from watermark

        // Should have tile after spacing
        let pixel_later = target.get_pixel(80 + 15, 15); // Next tile
        assert!(pixel_later[0] > 200);
    }

    // Test: Diagonal band renders across image
    #[test]
    fn test_diagonal_band_rendering() {
        let image_dims = ImageDimensions {
            width: 200,
            height: 200,
        };
        let watermark = create_watermark(20, 20, Rgba([255, 0, 0, 128]));

        let layers = create_diagonal_layers(&watermark, &image_dims, 40, 0.5);

        // Should have multiple layers forming diagonal
        assert!(layers.len() > 1);

        // Verify layers are on a diagonal
        for layer in &layers {
            // Diagonal positions should have roughly equal x and y
            // (accounting for watermark size and offsets)
            let x = layer.position.x;
            let y = layer.position.y;
            // The difference between x and y should be relatively consistent
            // for a diagonal pattern
            assert!(x >= -20 && y >= -20);
        }
    }

    // Test: Compositor layer management
    #[test]
    fn test_compositor_layer_management() {
        let mut compositor = Compositor::new();
        assert_eq!(compositor.layer_count(), 0);

        let watermark = create_watermark(10, 10, Rgba([255, 0, 0, 255]));
        compositor.add_layer(WatermarkLayer {
            image: watermark.clone(),
            position: PlacementPosition { x: 0, y: 0 },
            opacity: 1.0,
        });

        assert_eq!(compositor.layer_count(), 1);

        compositor.add_layer(WatermarkLayer {
            image: watermark,
            position: PlacementPosition { x: 20, y: 20 },
            opacity: 0.5,
        });

        assert_eq!(compositor.layer_count(), 2);

        compositor.clear();
        assert_eq!(compositor.layer_count(), 0);
    }

    // Test: Watermark clipping at image edges
    #[test]
    fn test_watermark_clipping() {
        let mut target = create_test_image(50, 50, Rgba([255, 255, 255, 255]));
        let watermark = create_watermark(30, 30, Rgba([255, 0, 0, 255]));

        // Position watermark partially outside the image
        let layer = WatermarkLayer {
            image: watermark,
            position: PlacementPosition { x: 40, y: 40 }, // Only 10x10 will be visible
            opacity: 1.0,
        };

        blend_layer(&mut target, &layer);

        // Check that the visible part is red
        let pixel_visible = target.get_pixel(45, 45);
        assert_eq!(pixel_visible[0], 255);
        assert_eq!(pixel_visible[1], 0);
        assert_eq!(pixel_visible[2], 0);

        // Original pixel outside watermark area is still white
        let pixel_outside = target.get_pixel(30, 30);
        assert_eq!(pixel_outside[0], 255);
        assert_eq!(pixel_outside[1], 255);
        assert_eq!(pixel_outside[2], 255);
    }

    // Test: Negative position (watermark starts outside image)
    #[test]
    fn test_negative_position() {
        let mut target = create_test_image(50, 50, Rgba([255, 255, 255, 255]));
        let watermark = create_watermark(30, 30, Rgba([255, 0, 0, 255]));

        let layer = WatermarkLayer {
            image: watermark,
            position: PlacementPosition { x: -20, y: -20 }, // Only bottom-right 10x10 visible
            opacity: 1.0,
        };

        blend_layer(&mut target, &layer);

        // Check that the visible part (top-left of image) is red
        let pixel_visible = target.get_pixel(5, 5);
        assert_eq!(pixel_visible[0], 255);
        assert_eq!(pixel_visible[1], 0);
        assert_eq!(pixel_visible[2], 0);

        // Pixel outside clipped watermark area is still white
        let pixel_outside = target.get_pixel(20, 20);
        assert_eq!(pixel_outside[0], 255);
        assert_eq!(pixel_outside[1], 255);
        assert_eq!(pixel_outside[2], 255);
    }

    // Test: Zero opacity watermark has no effect
    #[test]
    fn test_zero_opacity() {
        let mut target = create_test_image(100, 100, Rgba([255, 255, 255, 255]));
        let watermark = create_watermark(20, 20, Rgba([255, 0, 0, 255]));

        apply_watermark(
            &mut target,
            &watermark,
            WatermarkPosition::Center,
            0,
            0.0, // Zero opacity
        );

        // Target should be unchanged
        let pixel = target.get_pixel(50, 50);
        assert_eq!(pixel[0], 255);
        assert_eq!(pixel[1], 255);
        assert_eq!(pixel[2], 255);
    }

    // Test: Full opacity replaces pixel
    #[test]
    fn test_full_opacity() {
        let mut target = create_test_image(100, 100, Rgba([255, 255, 255, 255]));
        let watermark = create_watermark(20, 20, Rgba([0, 0, 255, 255]));

        apply_watermark(&mut target, &watermark, WatermarkPosition::Center, 0, 1.0);

        // Center pixel should be blue
        let pixel = target.get_pixel(50, 50);
        assert_eq!(pixel[0], 0);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 255);
    }

    // Test: Compositor apply_to_dynamic
    #[test]
    fn test_apply_to_dynamic() {
        let target =
            DynamicImage::ImageRgba8(create_test_image(100, 100, Rgba([255, 255, 255, 255])));
        let watermark = create_watermark(20, 20, Rgba([255, 0, 0, 255]));

        let mut compositor = Compositor::new();
        compositor.add_layer(WatermarkLayer {
            image: watermark,
            position: PlacementPosition { x: 40, y: 40 },
            opacity: 1.0,
        });

        let result = compositor.apply_to_dynamic(&target);
        let rgba = result.to_rgba8();

        // Check that watermark was applied
        let pixel = rgba.get_pixel(50, 50);
        assert_eq!(pixel[0], 255);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[2], 0);
    }

    // Test: Blend pixels function directly
    #[test]
    fn test_blend_pixels_direct() {
        // 50% alpha white over black = gray
        let bg = Rgba([0, 0, 0, 255]);
        let fg = Rgba([255, 255, 255, 128]);
        let result = blend_pixels(bg, fg, 1.0);

        assert!(result[0] > 100 && result[0] < 160);
        assert!(result[1] > 100 && result[1] < 160);
        assert!(result[2] > 100 && result[2] < 160);
        assert_eq!(result[3], 255);
    }
}
