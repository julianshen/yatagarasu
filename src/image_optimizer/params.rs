//! Image transformation parameter parsing
//!
//! Supports two URL formats:
//! 1. Query parameters: `?w=800&h=600&q=80&fmt=webp`
//! 2. Path-based options: `/w:800,h:600,q:80,f:webp/`

use std::collections::HashMap;
use std::str::FromStr;

use super::error::ImageError;

/// Output image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Jpeg,
    Png,
    WebP,
    Avif,
    /// Auto-select based on Accept header
    Auto,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Auto => "auto",
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
            Self::Auto => "image/jpeg", // Fallback, should be resolved before use
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Auto => "jpg",
        }
    }
}

impl FromStr for OutputFormat {
    type Err = ImageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "jpeg" | "jpg" => Ok(OutputFormat::Jpeg),
            "png" => Ok(OutputFormat::Png),
            "webp" => Ok(OutputFormat::WebP),
            "avif" => Ok(OutputFormat::Avif),
            "auto" => Ok(OutputFormat::Auto),
            _ => Err(ImageError::invalid_param(
                "format",
                format!("unknown format: {}", s),
            )),
        }
    }
}

/// How to fit the image within target dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FitMode {
    /// Crop to fill target dimensions (default)
    #[default]
    Cover,
    /// Scale to fit within dimensions, preserving aspect ratio
    Contain,
    /// Stretch to fill exactly (may distort)
    Fill,
    /// Scale down only, never up
    Inside,
    /// Scale to cover, may exceed target
    Outside,
    /// Add padding to fill dimensions
    Pad,
}

impl FromStr for FitMode {
    type Err = ImageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cover" => Ok(FitMode::Cover),
            "contain" => Ok(FitMode::Contain),
            "fill" => Ok(FitMode::Fill),
            "inside" => Ok(FitMode::Inside),
            "outside" => Ok(FitMode::Outside),
            "pad" => Ok(FitMode::Pad),
            _ => Err(ImageError::invalid_param(
                "fit",
                format!("unknown fit mode: {}", s),
            )),
        }
    }
}

/// Gravity/anchor point for cropping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gravity {
    #[default]
    Center,
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    /// Content-aware smart crop
    Smart,
}

impl FromStr for Gravity {
    type Err = ImageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "center" | "c" | "ce" => Ok(Gravity::Center),
            "north" | "n" | "no" => Ok(Gravity::North),
            "south" | "s" | "so" => Ok(Gravity::South),
            "east" | "e" | "ea" => Ok(Gravity::East),
            "west" | "w" | "we" => Ok(Gravity::West),
            "northeast" | "ne" => Ok(Gravity::NorthEast),
            "northwest" | "nw" => Ok(Gravity::NorthWest),
            "southeast" | "se" => Ok(Gravity::SouthEast),
            "southwest" | "sw" => Ok(Gravity::SouthWest),
            "smart" | "sm" => Ok(Gravity::Smart),
            _ => Err(ImageError::invalid_param(
                "gravity",
                format!("unknown gravity: {}", s),
            )),
        }
    }
}

/// Image transformation parameters
#[derive(Debug, Clone)]
pub struct ImageParams {
    // === Resize ===
    /// Target width in pixels (or percentage if ends with 'p')
    pub width: Option<Dimension>,
    /// Target height in pixels (or percentage if ends with 'p')
    pub height: Option<Dimension>,
    /// Device pixel ratio (1.0-4.0)
    pub dpr: f32,
    /// How to fit image in target dimensions
    pub fit: FitMode,
    /// Allow upscaling beyond original size
    pub enlarge: bool,

    // === Crop ===
    /// Gravity/anchor for crop operations
    pub gravity: Gravity,
    /// Crop offset X
    pub crop_x: Option<u32>,
    /// Crop offset Y
    pub crop_y: Option<u32>,
    /// Crop width
    pub crop_width: Option<u32>,
    /// Crop height
    pub crop_height: Option<u32>,

    // === Format & Quality ===
    /// Output format (None = preserve original)
    pub format: Option<OutputFormat>,
    /// Output quality (1-100)
    pub quality: Option<u8>,
    /// Strip metadata (EXIF, ICC, etc.)
    pub strip_metadata: bool,
    /// Use progressive encoding (JPEG)
    pub progressive: bool,

    // === Effects ===
    /// Rotation in degrees (0, 90, 180, 270)
    pub rotate: Option<u16>,
    /// Auto-rotate based on EXIF orientation
    pub auto_rotate: bool,
    /// Flip horizontally
    pub flip_h: bool,
    /// Flip vertically
    pub flip_v: bool,
    /// Gaussian blur sigma
    pub blur: Option<f32>,
    /// Sharpen sigma
    pub sharpen: Option<f32>,

    // === Background ===
    /// Background color for padding (hex RGB)
    pub background: Option<String>,
}

/// Dimension that can be pixels or percentage
#[derive(Debug, Clone, Copy)]
pub enum Dimension {
    Pixels(u32),
    Percentage(f32),
}

impl Dimension {
    /// Resolve to actual pixels given a source dimension
    pub fn resolve(&self, source: u32) -> u32 {
        match self {
            Dimension::Pixels(px) => *px,
            Dimension::Percentage(pct) => ((source as f32) * pct / 100.0).round() as u32,
        }
    }
}

impl FromStr for Dimension {
    type Err = ImageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.ends_with('p') || s.ends_with('%') {
            let num = s.trim_end_matches(['p', '%']);
            let pct: f32 = num
                .parse()
                .map_err(|_| ImageError::invalid_param("dimension", "invalid percentage"))?;
            if !(0.0..=1000.0).contains(&pct) {
                return Err(ImageError::invalid_param(
                    "dimension",
                    "percentage must be 0-1000",
                ));
            }
            Ok(Dimension::Percentage(pct))
        } else {
            let px: u32 = s
                .parse()
                .map_err(|_| ImageError::invalid_param("dimension", "invalid pixel value"))?;
            Ok(Dimension::Pixels(px))
        }
    }
}

impl Default for ImageParams {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            dpr: 1.0,
            fit: FitMode::Cover,
            enlarge: false,
            gravity: Gravity::Center,
            crop_x: None,
            crop_y: None,
            crop_width: None,
            crop_height: None,
            format: None,
            quality: None,
            strip_metadata: true,
            progressive: true,
            rotate: None,
            auto_rotate: true, // Auto-rotate based on EXIF by default
            flip_h: false,
            flip_v: false,
            blur: None,
            sharpen: None,
            background: None,
        }
    }
}

impl ImageParams {
    /// Parse from query parameters map (legacy compatibility method)
    ///
    /// Returns None if no optimization parameters are present or if parsing fails.
    /// Parsing errors are silently ignored for backward compatibility.
    pub fn from_params(params: &HashMap<String, String>) -> Option<Self> {
        Self::from_query(params).and_then(|r| r.ok())
    }

    /// Parse from query parameters (e.g., ?w=800&h=600&q=80)
    ///
    /// Returns None if no optimization parameters are present
    pub fn from_query(params: &HashMap<String, String>) -> Option<Result<Self, ImageError>> {
        // Check if any image params are present
        let image_params = [
            "w", "h", "q", "fmt", "f", "fit", "dpr", "g", "r", "blur", "sharpen",
        ];
        let has_params = image_params.iter().any(|p| params.contains_key(*p));

        if !has_params {
            return None;
        }

        Some(Self::parse_query(params))
    }

    fn parse_query(params: &HashMap<String, String>) -> Result<Self, ImageError> {
        let mut result = Self::default();

        // Width
        if let Some(w) = params.get("w") {
            result.width = Some(w.parse()?);
        }

        // Height
        if let Some(h) = params.get("h") {
            result.height = Some(h.parse()?);
        }

        // Quality
        if let Some(q) = params.get("q") {
            let quality: u8 = q
                .parse()
                .map_err(|_| ImageError::invalid_param("q", "must be 1-100"))?;
            if !(1..=100).contains(&quality) {
                return Err(ImageError::InvalidQuality { quality });
            }
            result.quality = Some(quality);
        }

        // Format (fmt or f)
        if let Some(fmt) = params.get("fmt").or_else(|| params.get("f")) {
            result.format = Some(fmt.parse()?);
        }

        // Fit mode
        if let Some(fit) = params.get("fit") {
            result.fit = fit.parse()?;
        }

        // DPR
        if let Some(dpr) = params.get("dpr") {
            let dpr_val: f32 = dpr
                .parse()
                .map_err(|_| ImageError::invalid_param("dpr", "must be 1-4"))?;
            if !(1.0..=4.0).contains(&dpr_val) {
                return Err(ImageError::invalid_param("dpr", "must be between 1 and 4"));
            }
            result.dpr = dpr_val;
        }

        // Gravity
        if let Some(g) = params.get("g") {
            result.gravity = g.parse()?;
        }

        // Rotation
        if let Some(r) = params.get("r") {
            let rotation: u16 = r
                .parse()
                .map_err(|_| ImageError::invalid_param("r", "must be 0, 90, 180, or 270"))?;
            if ![0, 90, 180, 270].contains(&rotation) {
                return Err(ImageError::invalid_param("r", "must be 0, 90, 180, or 270"));
            }
            result.rotate = Some(rotation);
        }

        // Auto-rotate (EXIF-based), enabled by default
        if let Some(auto) = params.get("auto_rotate") {
            result.auto_rotate = auto != "0" && auto != "false";
        }

        // Flip
        if let Some(flip) = params.get("flip") {
            match flip.as_str() {
                "h" => result.flip_h = true,
                "v" => result.flip_v = true,
                "hv" | "vh" => {
                    result.flip_h = true;
                    result.flip_v = true;
                }
                _ => return Err(ImageError::invalid_param("flip", "must be h, v, or hv")),
            }
        }

        // Blur
        if let Some(blur) = params.get("blur") {
            let sigma: f32 = blur
                .parse()
                .map_err(|_| ImageError::invalid_param("blur", "must be a number"))?;
            if !(0.0..=100.0).contains(&sigma) {
                return Err(ImageError::invalid_param("blur", "sigma must be 0-100"));
            }
            result.blur = Some(sigma);
        }

        // Sharpen
        if let Some(sharpen) = params.get("sharpen") {
            let sigma: f32 = sharpen
                .parse()
                .map_err(|_| ImageError::invalid_param("sharpen", "must be a number"))?;
            if !(0.0..=10.0).contains(&sigma) {
                return Err(ImageError::invalid_param("sharpen", "sigma must be 0-10"));
            }
            result.sharpen = Some(sigma);
        }

        // Enlarge
        if let Some(enlarge) = params.get("enlarge") {
            result.enlarge = enlarge == "1" || enlarge == "true";
        }

        // Strip metadata
        if let Some(strip) = params.get("strip") {
            result.strip_metadata = strip != "0" && strip != "false";
        }

        // Progressive
        if let Some(progressive) = params.get("progressive") {
            result.progressive = progressive != "0" && progressive != "false";
        }

        // Background color
        if let Some(bg) = params.get("bg") {
            result.background = Some(bg.clone());
        }

        // Crop dimensions
        if let Some(cx) = params.get("cx") {
            result.crop_x = Some(
                cx.parse()
                    .map_err(|_| ImageError::invalid_param("cx", "must be a number"))?,
            );
        }
        if let Some(cy) = params.get("cy") {
            result.crop_y = Some(
                cy.parse()
                    .map_err(|_| ImageError::invalid_param("cy", "must be a number"))?,
            );
        }
        if let Some(cw) = params.get("cw") {
            result.crop_width = Some(
                cw.parse()
                    .map_err(|_| ImageError::invalid_param("cw", "must be a number"))?,
            );
        }
        if let Some(ch) = params.get("ch") {
            result.crop_height = Some(
                ch.parse()
                    .map_err(|_| ImageError::invalid_param("ch", "must be a number"))?,
            );
        }

        Ok(result)
    }

    /// Parse from path-based options (e.g., /w:800,h:600,q:80/)
    pub fn from_path_options(options: &str) -> Result<Self, ImageError> {
        let mut params = HashMap::new();

        for part in options.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            // Split on first colon
            if let Some((key, value)) = part.split_once(':') {
                params.insert(key.to_string(), value.to_string());
            } else {
                // Handle boolean flags without values
                params.insert(part.to_string(), "1".to_string());
            }
        }

        Self::parse_query(&params)
    }

    /// Generate a cache key suffix for this parameter set
    pub fn to_cache_key(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref w) = self.width {
            match w {
                Dimension::Pixels(px) => parts.push(format!("w{}", px)),
                Dimension::Percentage(pct) => parts.push(format!("w{}p", pct)),
            }
        }

        if let Some(ref h) = self.height {
            match h {
                Dimension::Pixels(px) => parts.push(format!("h{}", px)),
                Dimension::Percentage(pct) => parts.push(format!("h{}p", pct)),
            }
        }

        if let Some(q) = self.quality {
            parts.push(format!("q{}", q));
        }

        if let Some(ref f) = self.format {
            parts.push(format!("f{}", f.as_str()));
        }

        if self.fit != FitMode::Cover {
            parts.push(format!("fit{:?}", self.fit).to_lowercase());
        }

        if self.dpr != 1.0 {
            parts.push(format!("dpr{}", self.dpr));
        }

        if let Some(r) = self.rotate {
            if r != 0 {
                parts.push(format!("r{}", r));
            }
        }

        // auto_rotate is true by default, only include if disabled
        if !self.auto_rotate {
            parts.push("noauto".to_string());
        }

        if self.flip_h {
            parts.push("fh".to_string());
        }
        if self.flip_v {
            parts.push("fv".to_string());
        }

        if let Some(blur) = self.blur {
            parts.push(format!("blur{}", blur));
        }
        if let Some(sharpen) = self.sharpen {
            parts.push(format!("sharp{}", sharpen));
        }

        if parts.is_empty() {
            "default".to_string()
        } else {
            parts.join("_")
        }
    }

    /// Check if any transformations are requested
    pub fn has_transformations(&self) -> bool {
        self.width.is_some()
            || self.height.is_some()
            || self.format.is_some()
            || self.quality.is_some()
            || self.rotate.is_some()
            || self.flip_h
            || self.flip_v
            || self.blur.is_some()
            || self.sharpen.is_some()
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("jpeg".parse::<OutputFormat>().unwrap(), OutputFormat::Jpeg);
        assert_eq!("jpg".parse::<OutputFormat>().unwrap(), OutputFormat::Jpeg);
        assert_eq!("png".parse::<OutputFormat>().unwrap(), OutputFormat::Png);
        assert_eq!("webp".parse::<OutputFormat>().unwrap(), OutputFormat::WebP);
        assert_eq!("avif".parse::<OutputFormat>().unwrap(), OutputFormat::Avif);
        assert!("tga".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_fit_mode_from_str() {
        assert_eq!("cover".parse::<FitMode>().unwrap(), FitMode::Cover);
        assert_eq!("contain".parse::<FitMode>().unwrap(), FitMode::Contain);
        assert_eq!("fill".parse::<FitMode>().unwrap(), FitMode::Fill);
        assert!("unknown".parse::<FitMode>().is_err());
    }

    #[test]
    fn test_gravity_from_str() {
        assert_eq!("center".parse::<Gravity>().unwrap(), Gravity::Center);
        assert_eq!("ne".parse::<Gravity>().unwrap(), Gravity::NorthEast);
        assert_eq!("smart".parse::<Gravity>().unwrap(), Gravity::Smart);
    }

    #[test]
    fn test_dimension_pixels() {
        let dim: Dimension = "800".parse().unwrap();
        assert_eq!(dim.resolve(1000), 800);
    }

    #[test]
    fn test_dimension_percentage() {
        let dim: Dimension = "50p".parse().unwrap();
        assert_eq!(dim.resolve(1000), 500);

        let dim2: Dimension = "50%".parse().unwrap();
        assert_eq!(dim2.resolve(1000), 500);
    }

    #[test]
    fn test_params_from_query() {
        let mut query = HashMap::new();
        query.insert("w".to_string(), "800".to_string());
        query.insert("h".to_string(), "600".to_string());
        query.insert("q".to_string(), "80".to_string());
        query.insert("fmt".to_string(), "webp".to_string());

        let params = ImageParams::from_query(&query).unwrap().unwrap();
        assert!(matches!(params.width, Some(Dimension::Pixels(800))));
        assert!(matches!(params.height, Some(Dimension::Pixels(600))));
        assert_eq!(params.quality, Some(80));
        assert_eq!(params.format, Some(OutputFormat::WebP));
    }

    #[test]
    fn test_params_from_path_options() {
        let params = ImageParams::from_path_options("w:800,h:600,q:80,f:webp").unwrap();
        assert!(matches!(params.width, Some(Dimension::Pixels(800))));
        assert!(matches!(params.height, Some(Dimension::Pixels(600))));
        assert_eq!(params.quality, Some(80));
        assert_eq!(params.format, Some(OutputFormat::WebP));
    }

    #[test]
    fn test_params_no_image_params() {
        let query = HashMap::new();
        assert!(ImageParams::from_query(&query).is_none());
    }

    #[test]
    fn test_params_invalid_quality() {
        let mut query = HashMap::new();
        query.insert("q".to_string(), "150".to_string());
        let result = ImageParams::from_query(&query).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn test_params_cache_key() {
        let mut params = ImageParams::default();
        params.width = Some(Dimension::Pixels(800));
        params.height = Some(Dimension::Pixels(600));
        params.quality = Some(80);
        params.format = Some(OutputFormat::WebP);

        let key = params.to_cache_key();
        assert!(key.contains("w800"));
        assert!(key.contains("h600"));
        assert!(key.contains("q80"));
        assert!(key.contains("fwebp"));
    }

    #[test]
    fn test_params_rotation_validation() {
        let mut query = HashMap::new();
        query.insert("r".to_string(), "45".to_string());
        let result = ImageParams::from_query(&query).unwrap();
        assert!(result.is_err());

        query.insert("r".to_string(), "90".to_string());
        let result = ImageParams::from_query(&query).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_params_dpr() {
        let mut query = HashMap::new();
        query.insert("w".to_string(), "100".to_string());
        query.insert("dpr".to_string(), "2".to_string());
        let params = ImageParams::from_query(&query).unwrap().unwrap();
        assert_eq!(params.dpr, 2.0);
    }
}
