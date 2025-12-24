//! Position calculation for watermark placement.
//!
//! This module provides functions to calculate where watermarks should be
//! placed on images based on the configured position mode.
//!
//! # Position Modes
//!
//! - **9-grid positions**: TopLeft, TopCenter, TopRight, CenterLeft, Center,
//!   CenterRight, BottomLeft, BottomCenter, BottomRight
//! - **Tiled**: Repeating grid pattern across the entire image
//! - **DiagonalBand**: Watermarks placed along a diagonal line
//!
//! # Example
//!
//! ```ignore
//! use yatagarasu::watermark::position::{calculate_position, ImageDimensions, WatermarkDimensions};
//! use yatagarasu::watermark::WatermarkPosition;
//!
//! let image = ImageDimensions { width: 800, height: 600 };
//! let watermark = WatermarkDimensions { width: 100, height: 50 };
//! let margin = 10;
//!
//! let (x, y) = calculate_position(WatermarkPosition::BottomRight, &image, &watermark, margin);
//! assert_eq!((x, y), (690, 540)); // 800 - 100 - 10, 600 - 50 - 10
//! ```

use super::WatermarkPosition;

/// Dimensions of the target image.
#[derive(Debug, Clone, Copy)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

/// Dimensions of the watermark to be placed.
#[derive(Debug, Clone, Copy)]
pub struct WatermarkDimensions {
    pub width: u32,
    pub height: u32,
}

/// A single position where a watermark should be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementPosition {
    pub x: i32,
    pub y: i32,
}

impl PlacementPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Calculate the position for a single watermark placement.
///
/// For grid positions (TopLeft, Center, BottomRight, etc.), returns a single
/// position. For Tiled and DiagonalBand, use `calculate_tiled_positions` or
/// `calculate_diagonal_positions` instead.
///
/// # Arguments
///
/// * `position` - The configured position mode
/// * `image` - Dimensions of the target image
/// * `watermark` - Dimensions of the watermark
/// * `margin` - Margin from edges in pixels
///
/// # Returns
///
/// The (x, y) coordinates where the watermark should be placed.
/// Coordinates may be negative if the watermark is larger than the image.
pub fn calculate_position(
    position: WatermarkPosition,
    image: &ImageDimensions,
    watermark: &WatermarkDimensions,
    margin: u32,
) -> PlacementPosition {
    let img_w = image.width as i32;
    let img_h = image.height as i32;
    let wm_w = watermark.width as i32;
    let wm_h = watermark.height as i32;
    let m = margin as i32;

    match position {
        // Top row
        WatermarkPosition::TopLeft => PlacementPosition::new(m, m),
        WatermarkPosition::TopCenter => PlacementPosition::new((img_w - wm_w) / 2, m),
        WatermarkPosition::TopRight => PlacementPosition::new(img_w - wm_w - m, m),

        // Center row
        WatermarkPosition::CenterLeft => PlacementPosition::new(m, (img_h - wm_h) / 2),
        WatermarkPosition::Center => PlacementPosition::new((img_w - wm_w) / 2, (img_h - wm_h) / 2),
        WatermarkPosition::CenterRight => {
            PlacementPosition::new(img_w - wm_w - m, (img_h - wm_h) / 2)
        }

        // Bottom row
        WatermarkPosition::BottomLeft => PlacementPosition::new(m, img_h - wm_h - m),
        WatermarkPosition::BottomCenter => {
            PlacementPosition::new((img_w - wm_w) / 2, img_h - wm_h - m)
        }
        WatermarkPosition::BottomRight => PlacementPosition::new(img_w - wm_w - m, img_h - wm_h - m),

        // For tiled/diagonal, return center as default (use specific functions)
        WatermarkPosition::Tiled | WatermarkPosition::DiagonalBand => {
            PlacementPosition::new((img_w - wm_w) / 2, (img_h - wm_h) / 2)
        }
    }
}

/// Calculate positions for tiled watermark placement.
///
/// Generates a grid of positions covering the entire image with the specified
/// spacing between watermarks.
///
/// # Arguments
///
/// * `image` - Dimensions of the target image
/// * `watermark` - Dimensions of the watermark
/// * `spacing` - Space between watermarks in pixels
///
/// # Returns
///
/// A vector of positions for each watermark instance.
pub fn calculate_tiled_positions(
    image: &ImageDimensions,
    watermark: &WatermarkDimensions,
    spacing: u32,
) -> Vec<PlacementPosition> {
    let mut positions = Vec::new();

    let step_x = watermark.width + spacing;
    let step_y = watermark.height + spacing;

    // Start slightly before the image to handle edge cases
    let start_x = 0i32;
    let start_y = 0i32;

    let mut y = start_y;
    while y < image.height as i32 {
        let mut x = start_x;
        while x < image.width as i32 {
            positions.push(PlacementPosition::new(x, y));
            x += step_x as i32;
        }
        y += step_y as i32;
    }

    positions
}

/// Calculate positions for diagonal band watermark placement.
///
/// Places watermarks along a diagonal line from top-left to bottom-right,
/// with the specified spacing between them.
///
/// # Arguments
///
/// * `image` - Dimensions of the target image
/// * `watermark` - Dimensions of the watermark
/// * `spacing` - Space between watermarks along the diagonal
///
/// # Returns
///
/// A vector of positions along the diagonal.
pub fn calculate_diagonal_positions(
    image: &ImageDimensions,
    watermark: &WatermarkDimensions,
    spacing: u32,
) -> Vec<PlacementPosition> {
    let mut positions = Vec::new();

    // Calculate diagonal step size (45 degree angle)
    let step = (spacing + watermark.width.max(watermark.height)) as i32;

    // Start from top-left corner area
    let diagonal_length = ((image.width.pow(2) + image.height.pow(2)) as f64).sqrt() as i32;

    let mut offset = 0i32;
    while offset < diagonal_length {
        // Calculate position along the diagonal
        let ratio = offset as f64 / diagonal_length as f64;
        let x = (ratio * image.width as f64) as i32 - (watermark.width as i32 / 2);
        let y = (ratio * image.height as f64) as i32 - (watermark.height as i32 / 2);

        // Only add if at least partially visible
        if x + watermark.width as i32 > 0
            && y + watermark.height as i32 > 0
            && x < image.width as i32
            && y < image.height as i32
        {
            positions.push(PlacementPosition::new(x, y));
        }

        offset += step;
    }

    positions
}

/// Clamp a position to ensure the watermark stays within image bounds.
///
/// Returns the adjusted position that keeps as much of the watermark visible
/// as possible while respecting the image boundaries.
///
/// # Arguments
///
/// * `pos` - The calculated position
/// * `image` - Dimensions of the target image
/// * `watermark` - Dimensions of the watermark
///
/// # Returns
///
/// The clamped position within valid bounds.
pub fn clamp_to_bounds(
    pos: PlacementPosition,
    image: &ImageDimensions,
    watermark: &WatermarkDimensions,
) -> PlacementPosition {
    let max_x = (image.width as i32 - watermark.width as i32).max(0);
    let max_y = (image.height as i32 - watermark.height as i32).max(0);

    PlacementPosition::new(pos.x.clamp(0, max_x), pos.y.clamp(0, max_y))
}

/// Check if a position is at least partially visible within the image.
///
/// # Arguments
///
/// * `pos` - The position to check
/// * `image` - Dimensions of the target image
/// * `watermark` - Dimensions of the watermark
///
/// # Returns
///
/// `true` if any part of the watermark would be visible.
pub fn is_visible(
    pos: &PlacementPosition,
    image: &ImageDimensions,
    watermark: &WatermarkDimensions,
) -> bool {
    let wm_right = pos.x + watermark.width as i32;
    let wm_bottom = pos.y + watermark.height as i32;

    pos.x < image.width as i32 && pos.y < image.height as i32 && wm_right > 0 && wm_bottom > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: u32, h: u32) -> ImageDimensions {
        ImageDimensions {
            width: w,
            height: h,
        }
    }

    fn watermark(w: u32, h: u32) -> WatermarkDimensions {
        WatermarkDimensions {
            width: w,
            height: h,
        }
    }

    // Test: calculate_position for all 9 corner/center positions
    #[test]
    fn test_calculate_position_top_left() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::TopLeft, &img, &wm, 10);
        assert_eq!(pos, PlacementPosition::new(10, 10));
    }

    #[test]
    fn test_calculate_position_top_center() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::TopCenter, &img, &wm, 10);
        // (800 - 100) / 2 = 350
        assert_eq!(pos, PlacementPosition::new(350, 10));
    }

    #[test]
    fn test_calculate_position_top_right() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::TopRight, &img, &wm, 10);
        // 800 - 100 - 10 = 690
        assert_eq!(pos, PlacementPosition::new(690, 10));
    }

    #[test]
    fn test_calculate_position_center_left() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::CenterLeft, &img, &wm, 10);
        // (600 - 50) / 2 = 275
        assert_eq!(pos, PlacementPosition::new(10, 275));
    }

    #[test]
    fn test_calculate_position_center() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::Center, &img, &wm, 10);
        assert_eq!(pos, PlacementPosition::new(350, 275));
    }

    #[test]
    fn test_calculate_position_center_right() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::CenterRight, &img, &wm, 10);
        assert_eq!(pos, PlacementPosition::new(690, 275));
    }

    #[test]
    fn test_calculate_position_bottom_left() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::BottomLeft, &img, &wm, 10);
        // 600 - 50 - 10 = 540
        assert_eq!(pos, PlacementPosition::new(10, 540));
    }

    #[test]
    fn test_calculate_position_bottom_center() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::BottomCenter, &img, &wm, 10);
        assert_eq!(pos, PlacementPosition::new(350, 540));
    }

    #[test]
    fn test_calculate_position_bottom_right() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::BottomRight, &img, &wm, 10);
        assert_eq!(pos, PlacementPosition::new(690, 540));
    }

    // Test: Margin applied correctly
    #[test]
    fn test_margin_zero() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::TopLeft, &img, &wm, 0);
        assert_eq!(pos, PlacementPosition::new(0, 0));
    }

    #[test]
    fn test_margin_large() {
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let pos = calculate_position(WatermarkPosition::BottomRight, &img, &wm, 50);
        // 800 - 100 - 50 = 650, 600 - 50 - 50 = 500
        assert_eq!(pos, PlacementPosition::new(650, 500));
    }

    // Test: calculate_tiled_positions returns grid coordinates
    #[test]
    fn test_tiled_positions_basic() {
        let img = image(300, 200);
        let wm = watermark(100, 100);
        let positions = calculate_tiled_positions(&img, &wm, 0);

        // With no spacing: 3 columns (0, 100, 200) x 2 rows (0, 100)
        assert_eq!(positions.len(), 6);
        assert!(positions.contains(&PlacementPosition::new(0, 0)));
        assert!(positions.contains(&PlacementPosition::new(100, 0)));
        assert!(positions.contains(&PlacementPosition::new(200, 0)));
        assert!(positions.contains(&PlacementPosition::new(0, 100)));
        assert!(positions.contains(&PlacementPosition::new(100, 100)));
        assert!(positions.contains(&PlacementPosition::new(200, 100)));
    }

    #[test]
    fn test_tiled_positions_with_spacing() {
        let img = image(400, 300);
        let wm = watermark(100, 100);
        let positions = calculate_tiled_positions(&img, &wm, 50);

        // With 50px spacing: step = 150
        // Columns: 0, 150, 300 (3 columns)
        // Rows: 0, 150 (2 rows)
        assert_eq!(positions.len(), 6);
        assert!(positions.contains(&PlacementPosition::new(0, 0)));
        assert!(positions.contains(&PlacementPosition::new(150, 0)));
        assert!(positions.contains(&PlacementPosition::new(300, 0)));
    }

    #[test]
    fn test_tiled_positions_single() {
        // Watermark larger than spacing allows only one
        let img = image(100, 100);
        let wm = watermark(100, 100);
        let positions = calculate_tiled_positions(&img, &wm, 0);

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0], PlacementPosition::new(0, 0));
    }

    // Test: calculate_diagonal_band with rotation
    #[test]
    fn test_diagonal_positions_basic() {
        let img = image(400, 400);
        let wm = watermark(50, 50);
        let positions = calculate_diagonal_positions(&img, &wm, 50);

        // Should have multiple positions along the diagonal
        assert!(!positions.is_empty());

        // First position should be near top-left
        let first = positions[0];
        assert!(first.x < 100 && first.y < 100);

        // Last position should be near bottom-right
        let last = positions[positions.len() - 1];
        assert!(last.x > 250 && last.y > 250);
    }

    #[test]
    fn test_diagonal_positions_ordering() {
        let img = image(500, 500);
        let wm = watermark(30, 30);
        let positions = calculate_diagonal_positions(&img, &wm, 100);

        // Positions should progress from top-left to bottom-right
        for i in 1..positions.len() {
            assert!(
                positions[i].x >= positions[i - 1].x,
                "X should increase along diagonal"
            );
            assert!(
                positions[i].y >= positions[i - 1].y,
                "Y should increase along diagonal"
            );
        }
    }

    // Test: Position accounts for watermark dimensions
    #[test]
    fn test_large_watermark_center() {
        let img = image(100, 100);
        let wm = watermark(80, 60);
        let pos = calculate_position(WatermarkPosition::Center, &img, &wm, 0);
        // (100 - 80) / 2 = 10, (100 - 60) / 2 = 20
        assert_eq!(pos, PlacementPosition::new(10, 20));
    }

    #[test]
    fn test_watermark_same_size_as_image() {
        let img = image(200, 200);
        let wm = watermark(200, 200);
        let pos = calculate_position(WatermarkPosition::Center, &img, &wm, 0);
        assert_eq!(pos, PlacementPosition::new(0, 0));
    }

    // Test: Position respects image boundaries (via clamp_to_bounds)
    #[test]
    fn test_clamp_to_bounds_negative_position() {
        let pos = PlacementPosition::new(-50, -30);
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let clamped = clamp_to_bounds(pos, &img, &wm);
        assert_eq!(clamped, PlacementPosition::new(0, 0));
    }

    #[test]
    fn test_clamp_to_bounds_exceeds_right() {
        let pos = PlacementPosition::new(750, 300);
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let clamped = clamp_to_bounds(pos, &img, &wm);
        // Max X = 800 - 100 = 700
        assert_eq!(clamped, PlacementPosition::new(700, 300));
    }

    #[test]
    fn test_clamp_to_bounds_exceeds_bottom() {
        let pos = PlacementPosition::new(300, 580);
        let img = image(800, 600);
        let wm = watermark(100, 50);
        let clamped = clamp_to_bounds(pos, &img, &wm);
        // Max Y = 600 - 50 = 550
        assert_eq!(clamped, PlacementPosition::new(300, 550));
    }

    #[test]
    fn test_clamp_to_bounds_watermark_larger_than_image() {
        // Edge case: watermark larger than image
        let pos = PlacementPosition::new(50, 50);
        let img = image(100, 100);
        let wm = watermark(200, 200);
        let clamped = clamp_to_bounds(pos, &img, &wm);
        // Max would be negative, but clamp to 0
        assert_eq!(clamped, PlacementPosition::new(0, 0));
    }

    // Test: is_visible function
    #[test]
    fn test_is_visible_fully_visible() {
        let pos = PlacementPosition::new(100, 100);
        let img = image(800, 600);
        let wm = watermark(50, 50);
        assert!(is_visible(&pos, &img, &wm));
    }

    #[test]
    fn test_is_visible_partially_visible_left() {
        let pos = PlacementPosition::new(-25, 100);
        let img = image(800, 600);
        let wm = watermark(50, 50);
        // Right edge at 25, which is > 0
        assert!(is_visible(&pos, &img, &wm));
    }

    #[test]
    fn test_is_visible_completely_outside_left() {
        let pos = PlacementPosition::new(-100, 100);
        let img = image(800, 600);
        let wm = watermark(50, 50);
        // Right edge at -50, which is <= 0
        assert!(!is_visible(&pos, &img, &wm));
    }

    #[test]
    fn test_is_visible_completely_outside_right() {
        let pos = PlacementPosition::new(850, 100);
        let img = image(800, 600);
        let wm = watermark(50, 50);
        assert!(!is_visible(&pos, &img, &wm));
    }

    // Edge case tests
    #[test]
    fn test_tiny_image() {
        let img = image(10, 10);
        let wm = watermark(5, 5);
        let pos = calculate_position(WatermarkPosition::Center, &img, &wm, 0);
        assert_eq!(pos, PlacementPosition::new(2, 2));
    }

    #[test]
    fn test_asymmetric_dimensions() {
        let img = image(1920, 1080);
        let wm = watermark(200, 50);
        let pos = calculate_position(WatermarkPosition::BottomRight, &img, &wm, 20);
        // 1920 - 200 - 20 = 1700, 1080 - 50 - 20 = 1010
        assert_eq!(pos, PlacementPosition::new(1700, 1010));
    }
}
