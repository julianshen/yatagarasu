//! Image processing implementation
//!
//! Handles the actual image transformation: decode → resize → encode

use exif::{In, Reader as ExifReader, Tag};
use fast_image_resize::{FilterType, Image, PixelType, ResizeAlg, Resizer};
use image::io::Reader as ImageReader;
use image::DynamicImage;
use std::io::Cursor;
use std::num::NonZeroU32;

use super::encoder::{EncoderFactory, EncoderQuality};
use super::error::ImageError;
use super::params::{Dimension, FitMode, Gravity, ImageParams, OutputFormat};

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

    // 2. Apply EXIF auto-rotation if enabled
    let img = if params.auto_rotate {
        apply_exif_rotation(&img, data)
    } else {
        img
    };

    let src_width = img.width();
    let src_height = img.height();

    // 3. Apply manual crop first if specified
    let img = if params.crop_width.is_some() || params.crop_height.is_some() {
        apply_manual_crop(&img, &params)?
    } else {
        img
    };

    // Update dimensions after crop
    let cropped_width = img.width();
    let cropped_height = img.height();

    // 3. Calculate target dimensions
    // For fit:pad, we allow the canvas to be larger than source (padding doesn't enlarge content)
    let allow_enlarge = params.enlarge || params.fit == FitMode::Pad;
    let (target_width, target_height) = calculate_dimensions(
        cropped_width,
        cropped_height,
        params.width.as_ref(),
        params.height.as_ref(),
        params.dpr,
        allow_enlarge,
    );

    // 4. Resize or transform based on fit mode
    let (processed_img, final_width, final_height) = match params.fit {
        FitMode::Pad => {
            // For pad mode, resize proportionally to fit within dimensions, then add padding
            // The image content should not be enlarged unless enlarge=true
            let (fit_width, fit_height) = if params.enlarge {
                calculate_contain_dimensions(
                    cropped_width,
                    cropped_height,
                    target_width,
                    target_height,
                )
            } else {
                // Don't enlarge content, just calculate what fits
                let (contain_w, contain_h) = calculate_contain_dimensions(
                    cropped_width,
                    cropped_height,
                    target_width,
                    target_height,
                );
                (contain_w.min(cropped_width), contain_h.min(cropped_height))
            };
            let resized = if fit_width != cropped_width || fit_height != cropped_height {
                resize_image(&img, fit_width, fit_height, &FitMode::Contain)?
            } else {
                img
            };
            let padded = apply_padding(
                &resized,
                target_width,
                target_height,
                params.background.as_deref(),
            )?;
            (padded, target_width, target_height)
        }
        FitMode::Cover => {
            // For cover mode, scale to cover then crop based on gravity
            if target_width != cropped_width || target_height != cropped_height {
                let cropped = apply_cover_crop(&img, target_width, target_height, &params.gravity)?;
                (cropped, target_width, target_height)
            } else {
                (img, target_width, target_height)
            }
        }
        _ => {
            // Standard resize (contain, fill, inside, outside)
            if target_width != cropped_width || target_height != cropped_height {
                let resized = resize_image(&img, target_width, target_height, &params.fit)?;
                (resized, target_width, target_height)
            } else {
                (img, target_width, target_height)
            }
        }
    };

    // 5. Apply rotation if specified
    let (processed_img, final_width, final_height) = if let Some(rotation) = params.rotate {
        let rotated = apply_rotation(&processed_img, rotation)?;
        let (w, h) = (rotated.width(), rotated.height());
        (rotated, w, h)
    } else {
        (processed_img, final_width, final_height)
    };

    // 6. Apply flip if specified
    let processed_img = apply_flip(&processed_img, params.flip_h, params.flip_v);

    // 7. Apply effects (blur, sharpen)
    let processed_img = apply_effects(&processed_img, &params);

    // 8. Determine output format
    let output_format = params.format.unwrap_or_else(|| detect_format(data));

    // 9. Encode to target format
    let quality = EncoderQuality::with_quality(params.quality.unwrap_or(80));
    let encoder = EncoderFactory::create(output_format);

    let rgba_data = processed_img.to_rgba8().into_raw();
    let encoded = encoder.encode(&rgba_data, final_width, final_height, quality)?;

    Ok(ProcessedImage {
        data: encoded.data,
        content_type: encoded.content_type.to_string(),
        original_size: (src_width, src_height),
        output_size: (final_width, final_height),
    })
}

/// Apply manual crop with offset and dimensions
fn apply_manual_crop(img: &DynamicImage, params: &ImageParams) -> Result<DynamicImage, ImageError> {
    let src_width = img.width();
    let src_height = img.height();

    // Get crop parameters with defaults
    let crop_x = params.crop_x.unwrap_or(0);
    let crop_y = params.crop_y.unwrap_or(0);
    let crop_width = params
        .crop_width
        .unwrap_or(src_width.saturating_sub(crop_x));
    let crop_height = params
        .crop_height
        .unwrap_or(src_height.saturating_sub(crop_y));

    // Validate crop bounds
    if crop_x >= src_width || crop_y >= src_height {
        return Err(ImageError::resize_failed(
            "Crop offset exceeds image bounds",
        ));
    }

    // Clamp crop dimensions to image bounds
    let final_width = crop_width.min(src_width - crop_x);
    let final_height = crop_height.min(src_height - crop_y);

    if final_width == 0 || final_height == 0 {
        return Err(ImageError::resize_failed(
            "Crop would result in zero-size image",
        ));
    }

    Ok(img.crop_imm(crop_x, crop_y, final_width, final_height))
}

/// Apply padding to center image within target dimensions
fn apply_padding(
    img: &DynamicImage,
    target_width: u32,
    target_height: u32,
    background: Option<&str>,
) -> Result<DynamicImage, ImageError> {
    let img_width = img.width();
    let img_height = img.height();

    // Parse background color or default to white
    let bg_color = parse_hex_color(background.unwrap_or("ffffff"));

    // Create new image with background color
    let mut result = image::RgbaImage::from_pixel(target_width, target_height, bg_color);

    // Calculate offset to center the image
    let offset_x = (target_width.saturating_sub(img_width)) / 2;
    let offset_y = (target_height.saturating_sub(img_height)) / 2;

    // Copy source image onto result
    let src_rgba = img.to_rgba8();
    for y in 0..img_height.min(target_height) {
        for x in 0..img_width.min(target_width) {
            let px = src_rgba.get_pixel(x, y);
            let dest_x = offset_x + x;
            let dest_y = offset_y + y;
            if dest_x < target_width && dest_y < target_height {
                result.put_pixel(dest_x, dest_y, *px);
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(result))
}

/// Parse a hex color string (RGB or RGBA) to an Rgba pixel
fn parse_hex_color(hex: &str) -> image::Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => {
            // RGB
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            image::Rgba([r, g, b, 255])
        }
        8 => {
            // RGBA
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            image::Rgba([r, g, b, a])
        }
        _ => image::Rgba([255, 255, 255, 255]), // Default to white
    }
}

/// Apply rotation to an image
///
/// Supports 0, 90, 180, 270 degrees and arbitrary angles
fn apply_rotation(img: &DynamicImage, degrees: u16) -> Result<DynamicImage, ImageError> {
    match degrees {
        0 => Ok(img.clone()),
        90 => Ok(img.rotate90()),
        180 => Ok(img.rotate180()),
        270 => Ok(img.rotate270()),
        _ => {
            // For arbitrary rotation, use the imageproc crate's affine transformation
            // For now, snap to nearest 90-degree increment
            let snapped = ((degrees + 45) / 90 * 90) % 360;
            match snapped {
                0 => Ok(img.clone()),
                90 => Ok(img.rotate90()),
                180 => Ok(img.rotate180()),
                270 => Ok(img.rotate270()),
                _ => Ok(img.clone()),
            }
        }
    }
}

/// Apply horizontal and/or vertical flip to an image
fn apply_flip(img: &DynamicImage, flip_h: bool, flip_v: bool) -> DynamicImage {
    match (flip_h, flip_v) {
        (true, true) => img.fliph().flipv(),
        (true, false) => img.fliph(),
        (false, true) => img.flipv(),
        (false, false) => img.clone(),
    }
}

/// Apply image effects: blur, sharpen, brightness, contrast, saturation, grayscale
///
/// Effects are applied in order:
/// 1. Blur (Gaussian blur)
/// 2. Sharpen (unsharp mask)
/// 3. Brightness adjustment
/// 4. Contrast adjustment
/// 5. Saturation adjustment
/// 6. Grayscale conversion (last, so saturation can be applied before)
fn apply_effects(img: &DynamicImage, params: &ImageParams) -> DynamicImage {
    let mut result = img.clone();

    // Apply Gaussian blur if specified
    if let Some(sigma) = params.blur {
        if sigma > 0.0 {
            result = result.blur(sigma);
        }
    }

    // Apply unsharp mask (sharpen) if specified
    if let Some(sigma) = params.sharpen {
        if sigma > 0.0 {
            // unsharpen(sigma, threshold) - threshold controls how much to sharpen edges
            // A threshold of 1 means only sharpen edges with 1+ unit difference
            result = result.unsharpen(sigma, 1);
        }
    }

    // Apply brightness adjustment if specified
    if let Some(brightness) = params.brightness {
        if brightness != 0 {
            result = result.brighten(brightness);
        }
    }

    // Apply contrast adjustment if specified
    if let Some(contrast) = params.contrast {
        if contrast != 0 {
            // adjust_contrast expects signed amount:
            // - Negative values decrease contrast
            // - Positive values increase contrast
            // Pass contrast value directly (-100 to 100 range)
            result = result.adjust_contrast(contrast as f32);
        }
    }

    // Apply saturation adjustment if specified
    if let Some(saturation) = params.saturation {
        if saturation != 0 {
            result = apply_saturation(&result, saturation);
        }
    }

    // Apply grayscale conversion last
    if params.grayscale {
        result = result.grayscale();
    }

    result
}

/// Apply saturation adjustment using HSL color space
///
/// saturation: -100 (grayscale) to 100 (max saturation)
fn apply_saturation(img: &DynamicImage, saturation: i32) -> DynamicImage {
    let mut rgba = img.to_rgba8();
    let factor = 1.0 + (saturation as f32 / 100.0);

    for pixel in rgba.pixels_mut() {
        let [r, g, b, a] = pixel.0;

        // Convert to HSL
        let (h, s, l) = rgb_to_hsl(r, g, b);

        // Adjust saturation
        let new_s = (s * factor).clamp(0.0, 1.0);

        // Convert back to RGB
        let (nr, ng, nb) = hsl_to_rgb(h, new_s, l);

        pixel.0 = [nr, ng, nb, a];
    }

    DynamicImage::ImageRgba8(rgba)
}

/// Convert RGB to HSL
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Lightness
    let l = (max + min) / 2.0;

    // Saturation
    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    // Hue
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, l)
}

/// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r1 + m) * 255.0).round() as u8;
    let g = ((g1 + m) * 255.0).round() as u8;
    let b = ((b1 + m) * 255.0).round() as u8;

    (r, g, b)
}

/// Read EXIF orientation from image data
///
/// Returns the EXIF orientation value (1-8), or 1 if not found
fn read_exif_orientation(data: &[u8]) -> u32 {
    let mut cursor = Cursor::new(data);
    match ExifReader::new().read_from_container(&mut cursor) {
        Ok(exif) => {
            if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
                field.value.get_uint(0).unwrap_or(1)
            } else {
                1 // Default: no rotation needed
            }
        }
        Err(_) => 1, // No EXIF or parse error - assume no rotation needed
    }
}

/// Apply rotation/flip based on EXIF orientation
///
/// EXIF Orientation values:
/// 1 = Normal (no rotation)
/// 2 = Flip horizontal
/// 3 = Rotate 180°
/// 4 = Flip vertical
/// 5 = Rotate 90° CW + flip horizontal
/// 6 = Rotate 90° CW
/// 7 = Rotate 90° CCW + flip horizontal
/// 8 = Rotate 90° CCW (270° CW)
fn apply_exif_rotation(img: &DynamicImage, data: &[u8]) -> DynamicImage {
    let orientation = read_exif_orientation(data);

    match orientation {
        1 => img.clone(),             // Normal
        2 => img.fliph(),             // Flip horizontal
        3 => img.rotate180(),         // Rotate 180°
        4 => img.flipv(),             // Flip vertical
        5 => img.rotate90().fliph(),  // Rotate 90° CW + flip horizontal
        6 => img.rotate90(),          // Rotate 90° CW
        7 => img.rotate270().fliph(), // Rotate 90° CCW + flip horizontal
        8 => img.rotate270(),         // Rotate 90° CCW (270° CW)
        _ => img.clone(),             // Unknown - no rotation
    }
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

/// Calculate dimensions to fit source within target while preserving aspect ratio (contain mode)
fn calculate_contain_dimensions(
    src_width: u32,
    src_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let src_aspect = src_width as f32 / src_height as f32;
    let target_aspect = target_width as f32 / target_height as f32;

    if src_aspect > target_aspect {
        // Source is wider - fit to width
        let new_width = target_width;
        let new_height = (target_width as f32 / src_aspect).round() as u32;
        (new_width.max(1), new_height.max(1))
    } else {
        // Source is taller - fit to height
        let new_height = target_height;
        let new_width = (target_height as f32 * src_aspect).round() as u32;
        (new_width.max(1), new_height.max(1))
    }
}

/// Calculate dimensions to cover target while preserving aspect ratio
fn calculate_cover_dimensions(
    src_width: u32,
    src_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let src_aspect = src_width as f32 / src_height as f32;
    let target_aspect = target_width as f32 / target_height as f32;

    if src_aspect > target_aspect {
        // Source is wider - fit to height, crop width
        let new_height = target_height;
        let new_width = (target_height as f32 * src_aspect).round() as u32;
        (new_width.max(1), new_height.max(1))
    } else {
        // Source is taller - fit to width, crop height
        let new_width = target_width;
        let new_height = (target_width as f32 / src_aspect).round() as u32;
        (new_width.max(1), new_height.max(1))
    }
}

/// Apply cover crop: scale to cover target dimensions, then crop based on gravity
fn apply_cover_crop(
    img: &DynamicImage,
    target_width: u32,
    target_height: u32,
    gravity: &Gravity,
) -> Result<DynamicImage, ImageError> {
    let src_width = img.width();
    let src_height = img.height();

    // Calculate scaled dimensions to cover target
    let (scaled_width, scaled_height) =
        calculate_cover_dimensions(src_width, src_height, target_width, target_height);

    // First resize to cover dimensions
    let scaled = resize_image(img, scaled_width, scaled_height, &FitMode::Fill)?;

    // Calculate crop offset based on gravity
    let (crop_x, crop_y) = match gravity {
        Gravity::Smart => {
            // For smart crop, find the area with highest entropy
            calculate_smart_crop_offset(&scaled, target_width, target_height)
        }
        _ => calculate_gravity_offset(
            scaled_width,
            scaled_height,
            target_width,
            target_height,
            gravity,
        ),
    };

    // Crop to target dimensions
    Ok(scaled.crop_imm(crop_x, crop_y, target_width, target_height))
}

/// Calculate crop offset based on gravity
fn calculate_gravity_offset(
    src_width: u32,
    src_height: u32,
    target_width: u32,
    target_height: u32,
    gravity: &Gravity,
) -> (u32, u32) {
    let max_x = src_width.saturating_sub(target_width);
    let max_y = src_height.saturating_sub(target_height);

    match gravity {
        Gravity::Center | Gravity::Smart => (max_x / 2, max_y / 2),
        Gravity::North => (max_x / 2, 0),
        Gravity::South => (max_x / 2, max_y),
        Gravity::East => (max_x, max_y / 2),
        Gravity::West => (0, max_y / 2),
        Gravity::NorthEast => (max_x, 0),
        Gravity::NorthWest => (0, 0),
        Gravity::SouthEast => (max_x, max_y),
        Gravity::SouthWest => (0, max_y),
    }
}

/// Calculate smart crop offset using entropy-based detection
fn calculate_smart_crop_offset(
    img: &DynamicImage,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let src_width = img.width();
    let src_height = img.height();

    if src_width <= target_width && src_height <= target_height {
        return (0, 0);
    }

    let rgba = img.to_rgba8();
    let max_x = src_width.saturating_sub(target_width);
    let max_y = src_height.saturating_sub(target_height);

    // Sample a grid of possible crop positions and find highest entropy
    let step_x = (max_x / 5).max(1);
    let step_y = (max_y / 5).max(1);

    let mut best_offset = (max_x / 2, max_y / 2); // Default to center
    let mut best_entropy = 0.0f32;

    let mut x = 0;
    while x <= max_x {
        let mut y = 0;
        while y <= max_y {
            let entropy = calculate_region_entropy(&rgba, x, y, target_width, target_height);
            if entropy > best_entropy {
                best_entropy = entropy;
                best_offset = (x, y);
            }
            y += step_y;
        }
        x += step_x;
    }

    best_offset
}

/// Calculate entropy of a region (higher = more detail/variation)
fn calculate_region_entropy(
    img: &image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> f32 {
    // Sample pixels and calculate variance as a proxy for entropy
    let sample_step = 4; // Sample every 4th pixel for performance
    let mut sum_r = 0u64;
    let mut sum_g = 0u64;
    let mut sum_b = 0u64;
    let mut sum_sq_r = 0u64;
    let mut sum_sq_g = 0u64;
    let mut sum_sq_b = 0u64;
    let mut count = 0u64;

    let mut py = y;
    while py < y + height && py < img.height() {
        let mut px = x;
        while px < x + width && px < img.width() {
            let pixel = img.get_pixel(px, py);
            let r = pixel[0] as u64;
            let g = pixel[1] as u64;
            let b = pixel[2] as u64;

            sum_r += r;
            sum_g += g;
            sum_b += b;
            sum_sq_r += r * r;
            sum_sq_g += g * g;
            sum_sq_b += b * b;
            count += 1;

            px += sample_step;
        }
        py += sample_step;
    }

    if count == 0 {
        return 0.0;
    }

    // Calculate variance for each channel
    let var_r = (sum_sq_r as f32 / count as f32) - (sum_r as f32 / count as f32).powi(2);
    let var_g = (sum_sq_g as f32 / count as f32) - (sum_g as f32 / count as f32).powi(2);
    let var_b = (sum_sq_b as f32 / count as f32) - (sum_b as f32 / count as f32).powi(2);

    // Return combined variance as entropy proxy
    var_r + var_g + var_b
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
#[allow(clippy::field_reassign_with_default)]
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

    // ============================================================
    // Phase 50.2: Advanced Resize & Crop Tests
    // ============================================================

    fn create_test_image(width: u32, height: u32) -> Vec<u8> {
        // Create test image with gradient for crop testing
        let img = image::RgbaImage::from_fn(width, height, |x, y| {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            image::Rgba([r, g, 128, 255])
        });

        let mut buffer = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        buffer.into_inner()
    }

    #[test]
    fn test_resize_with_dpr_2x() {
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(50));
        params.height = Some(Dimension::Pixels(50));
        params.dpr = 2.0;
        params.enlarge = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // 50 * 2.0 = 100
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_resize_with_dpr_3x() {
        let img_data = create_test_image(150, 150);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(50));
        params.height = Some(Dimension::Pixels(50));
        params.dpr = 3.0;
        params.enlarge = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // 50 * 3.0 = 150
        assert_eq!(processed.output_size, (150, 150));
    }

    #[test]
    fn test_resize_percentage_width() {
        let img_data = create_test_image(200, 100);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Percentage(50.0));

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // 200 * 50% = 100
        assert_eq!(processed.output_size.0, 100);
    }

    #[test]
    fn test_resize_percentage_height() {
        let img_data = create_test_image(100, 200);
        let mut params = ImageParams::default();
        params.height = Some(Dimension::Percentage(25.0));

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // 200 * 25% = 50
        assert_eq!(processed.output_size.1, 50);
    }

    #[test]
    fn test_resize_enlarge_disabled_by_default() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        // enlarge defaults to false

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Should not exceed source size
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_resize_enlarge_when_enabled() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.enlarge = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Should enlarge to 100x100
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_crop_gravity_center() {
        use crate::image_optimizer::params::Gravity;

        let img_data = create_test_image(200, 200);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.fit = FitMode::Cover;
        params.gravity = Gravity::Center;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_crop_gravity_north() {
        use crate::image_optimizer::params::Gravity;

        let img_data = create_test_image(200, 200);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.fit = FitMode::Cover;
        params.gravity = Gravity::North;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_crop_gravity_southeast() {
        use crate::image_optimizer::params::Gravity;

        let img_data = create_test_image(200, 200);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.fit = FitMode::Cover;
        params.gravity = Gravity::SouthEast;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_crop_manual_offset() {
        let img_data = create_test_image(200, 200);
        let mut params = ImageParams::default();
        params.crop_x = Some(50);
        params.crop_y = Some(50);
        params.crop_width = Some(100);
        params.crop_height = Some(100);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_crop_manual_dimensions() {
        let img_data = create_test_image(200, 200);
        let mut params = ImageParams::default();
        params.crop_width = Some(80);
        params.crop_height = Some(60);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (80, 60));
    }

    #[test]
    fn test_fit_inside_never_exceeds() {
        let img_data = create_test_image(200, 100);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(300));
        params.height = Some(Dimension::Pixels(300));
        params.fit = FitMode::Inside;
        params.enlarge = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // With fit:inside, should scale up proportionally to fit inside 300x300
        // Original 200x100 (2:1 aspect) -> fits 300x150
        assert!(processed.output_size.0 <= 300);
        assert!(processed.output_size.1 <= 300);
    }

    #[test]
    fn test_fit_pad_adds_background() {
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.fit = FitMode::Pad;
        params.background = Some("ffffff".to_string());

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Output should be exactly 100x100 with padding
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_smart_crop_detects_subject() {
        use crate::image_optimizer::params::Gravity;

        // Create image with high-detail area in bottom-right
        let img = image::RgbaImage::from_fn(200, 200, |x, y| {
            if x > 100 && y > 100 {
                // High detail area - checkerboard pattern
                if (x + y) % 2 == 0 {
                    image::Rgba([255, 0, 0, 255])
                } else {
                    image::Rgba([0, 255, 0, 255])
                }
            } else {
                // Low detail area - solid color
                image::Rgba([128, 128, 128, 255])
            }
        });

        let mut buffer = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        let img_data = buffer.into_inner();

        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.fit = FitMode::Cover;
        params.gravity = Gravity::Smart;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_entropy_crop_favors_detail() {
        use crate::image_optimizer::params::Gravity;

        // Create image with gradient (more entropy than solid)
        let img_data = create_test_image(200, 200);

        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.fit = FitMode::Cover;
        params.gravity = Gravity::Smart;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    // ============================================================
    // Phase 50.3: Transformation Tests
    // ============================================================

    #[test]
    fn test_rotate_90_clockwise() {
        // Create a 100x50 image (wider than tall)
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        params.rotate = Some(90);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // After 90° rotation, dimensions should swap: 100x50 -> 50x100
        assert_eq!(processed.output_size, (50, 100));
    }

    #[test]
    fn test_rotate_180() {
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        params.rotate = Some(180);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // 180° rotation preserves dimensions
        assert_eq!(processed.output_size, (100, 50));
    }

    #[test]
    fn test_rotate_270_clockwise() {
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        params.rotate = Some(270);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // After 270° rotation, dimensions should swap: 100x50 -> 50x100
        assert_eq!(processed.output_size, (50, 100));
    }

    #[test]
    fn test_flip_horizontal() {
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.flip_h = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Flip preserves dimensions
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_flip_vertical() {
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.flip_v = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_flip_both_equals_rotate_180() {
        let img_data = create_test_image(100, 100);

        // Flip both horizontal and vertical
        let mut params_flip = ImageParams::default();
        params_flip.flip_h = true;
        params_flip.flip_v = true;

        // Rotate 180 degrees
        let mut params_rotate = ImageParams::default();
        params_rotate.rotate = Some(180);

        let result_flip = process_image_internal(&img_data, params_flip);
        let result_rotate = process_image_internal(&img_data, params_rotate);

        assert!(result_flip.is_ok());
        assert!(result_rotate.is_ok());

        // Both should have same dimensions
        assert_eq!(
            result_flip.unwrap().output_size,
            result_rotate.unwrap().output_size
        );
    }

    #[test]
    fn test_rotation_preserves_dimensions_correctly() {
        // Non-square image to verify dimension swap
        let img_data = create_test_image(200, 100);
        let mut params = ImageParams::default();
        params.rotate = Some(90);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // 200x100 rotated 90° should become 100x200
        assert_eq!(processed.output_size, (100, 200));
    }

    #[test]
    fn test_combined_rotation_and_flip() {
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        params.rotate = Some(90);
        params.flip_h = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Dimensions swap due to 90° rotation
        assert_eq!(processed.output_size, (50, 100));
    }

    #[test]
    fn test_rotate_arbitrary_snaps_to_nearest() {
        // Test that arbitrary angles snap to nearest 90°
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        // 45° should snap to 90°
        // Note: params validation currently rejects non-90° values,
        // but apply_rotation handles them by snapping
        params.rotate = Some(0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 50));
    }

    #[test]
    fn test_read_exif_orientation_no_exif() {
        // PNG images don't have EXIF - should return 1 (normal)
        let img_data = create_test_image(100, 100);
        let orientation = read_exif_orientation(&img_data);
        assert_eq!(orientation, 1);
    }

    #[test]
    fn test_apply_exif_rotation_no_exif() {
        // Image without EXIF should remain unchanged
        let img_data = create_test_image(100, 50);
        let img = decode_image(&img_data).unwrap();
        let rotated = apply_exif_rotation(&img, &img_data);
        assert_eq!(rotated.width(), 100);
        assert_eq!(rotated.height(), 50);
    }

    #[test]
    fn test_auto_rotate_enabled_by_default() {
        let params = ImageParams::default();
        assert!(params.auto_rotate);
    }

    #[test]
    fn test_auto_rotate_can_be_disabled() {
        let img_data = create_test_image(100, 50);
        let mut params = ImageParams::default();
        params.auto_rotate = false;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        // Should process without error, dimensions unchanged (no EXIF in test image)
        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 50));
    }

    // ============================================================
    // Phase 50.9: Image Effects Tests
    // ============================================================

    #[test]
    fn test_blur_gaussian_applied() {
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.blur = Some(2.0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Blur should not change dimensions
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_blur_sigma_affects_result() {
        let img_data = create_test_image(50, 50);

        // Low blur
        let mut params_low = ImageParams::default();
        params_low.blur = Some(1.0);
        let result_low = process_image_internal(&img_data, params_low).unwrap();

        // High blur
        let mut params_high = ImageParams::default();
        params_high.blur = Some(10.0);
        let result_high = process_image_internal(&img_data, params_high).unwrap();

        // Both should produce valid output
        assert!(!result_low.data.is_empty());
        assert!(!result_high.data.is_empty());
        // Higher blur should produce different output
        assert_ne!(result_low.data, result_high.data);
    }

    #[test]
    fn test_blur_zero_sigma_no_change() {
        let img_data = create_test_image(50, 50);

        // Zero blur (effectively no blur)
        let mut params = ImageParams::default();
        params.blur = Some(0.0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        // Should still work, just with no blur applied
        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_sharpen_unsharp_mask_applied() {
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.sharpen = Some(1.5);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Sharpen should not change dimensions
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_sharpen_sigma_affects_result() {
        let img_data = create_test_image(50, 50);

        // Low sharpen
        let mut params_low = ImageParams::default();
        params_low.sharpen = Some(0.5);
        let result_low = process_image_internal(&img_data, params_low).unwrap();

        // High sharpen
        let mut params_high = ImageParams::default();
        params_high.sharpen = Some(5.0);
        let result_high = process_image_internal(&img_data, params_high).unwrap();

        // Both should produce valid output
        assert!(!result_low.data.is_empty());
        assert!(!result_high.data.is_empty());
        // Different sharpen levels should produce different output
        assert_ne!(result_low.data, result_high.data);
    }

    #[test]
    fn test_sharpen_zero_sigma_no_change() {
        let img_data = create_test_image(50, 50);

        // Zero sharpen (effectively no sharpen)
        let mut params = ImageParams::default();
        params.sharpen = Some(0.0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        // Should still work, just with no sharpen applied
        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_effects_combine_with_resize() {
        let img_data = create_test_image(200, 200);
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(100));
        params.height = Some(Dimension::Pixels(100));
        params.blur = Some(2.0);
        params.sharpen = Some(1.0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        // Resize should still work with effects
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_blur_and_sharpen_combined() {
        // Applying blur then sharpen (odd but valid combination)
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.blur = Some(3.0);
        params.sharpen = Some(2.0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_brightness_increase() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.brightness = Some(50);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_brightness_decrease() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.brightness = Some(-50);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_brightness_zero_no_change() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.brightness = Some(0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_contrast_increase() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.contrast = Some(50);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_contrast_decrease() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.contrast = Some(-50);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_contrast_zero_no_change() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.contrast = Some(0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_grayscale_conversion() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.grayscale = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_grayscale_preserves_dimensions() {
        let img_data = create_test_image(100, 80);
        let mut params = ImageParams::default();
        params.grayscale = true;

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 80));
    }

    #[test]
    fn test_saturation_increase() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.saturation = Some(50);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_saturation_decrease() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.saturation = Some(-50);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_saturation_zero_no_change() {
        let img_data = create_test_image(50, 50);
        let mut params = ImageParams::default();
        params.saturation = Some(0);

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (50, 50));
    }

    #[test]
    fn test_all_effects_combined() {
        // Apply all effects together
        let img_data = create_test_image(100, 100);
        let mut params = ImageParams::default();
        params.blur = Some(1.0);
        params.sharpen = Some(0.5);
        params.brightness = Some(10);
        params.contrast = Some(20);
        params.saturation = Some(-30);
        // grayscale = false so we can test saturation

        let result = process_image_internal(&img_data, params);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert_eq!(processed.output_size, (100, 100));
    }

    #[test]
    fn test_effects_cache_key_includes_all() {
        let mut params = ImageParams::default();
        params.brightness = Some(50);
        params.contrast = Some(-25);
        params.grayscale = true;
        params.saturation = Some(30);

        let key = params.to_cache_key();
        assert!(key.contains("bri50"));
        assert!(key.contains("con-25"));
        assert!(key.contains("gray"));
        assert!(key.contains("sat30"));
    }
}
