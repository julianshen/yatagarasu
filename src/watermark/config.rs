//! Watermark configuration types.
//!
//! This module defines per-bucket watermark configuration including:
//! - Text watermarks with template variable support
//! - Image watermarks from S3 or external URLs
//! - Position modes (9-grid, tiled, diagonal band)
//! - Path pattern matching rules
//!
//! Watermarks are defined inline within bucket rules, not as global definitions.

use serde::{Deserialize, Serialize};

// Default values
fn default_font_size() -> u32 {
    24
}

fn default_color() -> String {
    "#FFFFFF".to_string()
}

fn default_opacity() -> f32 {
    0.5
}

fn default_margin() -> u32 {
    10
}

fn default_cache_ttl() -> u64 {
    3600
}

/// Watermark position on the image.
///
/// Supports 9 fixed positions (grid), plus tiled and diagonal band modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WatermarkPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    /// Repeating tile pattern across the entire image
    Tiled,
    /// Diagonal band across the image (like stock photo watermarks)
    DiagonalBand,
}

impl WatermarkPosition {
    /// Convert position to a string for cache key generation.
    pub fn as_cache_str(&self) -> &'static str {
        match self {
            Self::TopLeft => "tl",
            Self::TopCenter => "tc",
            Self::TopRight => "tr",
            Self::CenterLeft => "cl",
            Self::Center => "c",
            Self::CenterRight => "cr",
            Self::BottomLeft => "bl",
            Self::BottomCenter => "bc",
            Self::BottomRight => "br",
            Self::Tiled => "tile",
            Self::DiagonalBand => "diag",
        }
    }
}

/// Watermark definition - either text or image.
///
/// Uses serde tag to distinguish between types in YAML:
/// ```yaml
/// - type: text
///   text: "Copyright {{jwt.org}}"
///   position: bottom-right
/// - type: image
///   source: "s3://assets/logo.png"
///   position: top-left
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WatermarkDefinition {
    Text(TextWatermarkConfig),
    Image(ImageWatermarkConfig),
}

/// Text watermark configuration.
///
/// Supports template variables in the `text` field:
/// - `{{jwt.sub}}`, `{{jwt.iss}}`, `{{jwt.<custom>}}` - JWT claims
/// - `{{ip}}` - Client IP address
/// - `{{header.X-Name}}` - Request headers
/// - `{{date}}`, `{{datetime}}`, `{{timestamp}}` - Time values
/// - `{{path}}`, `{{bucket}}` - Request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextWatermarkConfig {
    /// Text content with optional template variables (e.g., "Copyright {{jwt.org}}")
    pub text: String,

    /// Font size in pixels (default: 24)
    #[serde(default = "default_font_size")]
    pub font_size: u32,

    /// Text color as hex string (default: "#FFFFFF")
    #[serde(default = "default_color")]
    pub color: String,

    /// Opacity from 0.0 (transparent) to 1.0 (opaque) (default: 0.5)
    #[serde(default = "default_opacity")]
    pub opacity: f32,

    /// Position on the image
    pub position: WatermarkPosition,

    /// Margin from edge in pixels (default: 10)
    #[serde(default = "default_margin")]
    pub margin: u32,

    /// Rotation angle in degrees (for diagonal text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<i16>,

    /// Spacing between tiles in pixels (for tiled position)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_spacing: Option<u32>,
}

/// Image watermark configuration.
///
/// Supports two source formats:
/// - S3: `s3://bucket-name/path/to/image.png`
/// - URL: `https://cdn.example.com/logo.png`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageWatermarkConfig {
    /// Source URL or S3 path (e.g., "s3://assets/logo.png" or "https://...")
    pub source: String,

    /// Resize width in pixels (maintains aspect ratio if height not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    /// Resize height in pixels (maintains aspect ratio if width not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    /// Opacity from 0.0 (transparent) to 1.0 (opaque) (default: 0.5)
    #[serde(default = "default_opacity")]
    pub opacity: f32,

    /// Position on the image
    pub position: WatermarkPosition,

    /// Margin from edge in pixels (default: 10)
    #[serde(default = "default_margin")]
    pub margin: u32,
}

/// Per-bucket watermark configuration.
///
/// Added to `BucketConfig` as an optional field. If absent or disabled,
/// no watermarks are applied to images from this bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketWatermarkConfig {
    /// Enable/disable watermarking for this bucket (default: false)
    #[serde(default)]
    pub enabled: bool,

    /// Cache TTL for fetched watermark images in seconds (default: 3600)
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,

    /// Path-pattern rules for applying watermarks (first match wins)
    #[serde(default)]
    pub rules: Vec<WatermarkRule>,
}

impl Default for BucketWatermarkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_ttl_seconds: default_cache_ttl(),
            rules: Vec::new(),
        }
    }
}

/// A rule that matches paths and applies watermarks.
///
/// Rules are evaluated in order; the first matching rule is used.
/// Use `*` as a catch-all pattern for default watermarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkRule {
    /// Glob pattern to match request paths (e.g., "/products/previews/*")
    pub pattern: String,

    /// Watermarks to apply when this rule matches (in order)
    pub watermarks: Vec<WatermarkDefinition>,
}

impl TextWatermarkConfig {
    /// Validate the text watermark configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.text.is_empty() {
            return Err("Text watermark 'text' field cannot be empty".to_string());
        }

        // Check for NaN/Infinity and valid range
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(format!(
                "Text watermark opacity must be a finite value between 0.0 and 1.0, got {}",
                self.opacity
            ));
        }

        // Validate hex color format (#RGB or #RRGGBB)
        if let Some(hex_part) = self.color.strip_prefix('#') {
            let len = hex_part.len();
            if (len != 3 && len != 6) || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!(
                    "Text watermark color must be in #RGB or #RRGGBB format with valid hex characters, got '{}'",
                    self.color
                ));
            }
        } else {
            return Err(format!(
                "Text watermark color must be a hex string starting with '#', got '{}'",
                self.color
            ));
        }

        Ok(())
    }
}

impl ImageWatermarkConfig {
    /// Allowed URL schemes for image watermark sources.
    /// Note: http:// is intentionally excluded to prevent MITM attacks.
    const ALLOWED_PREFIXES: &'static [&'static str] = &["s3://", "https://"];

    /// Validate the image watermark configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.source.is_empty() {
            return Err("Image watermark 'source' field cannot be empty".to_string());
        }

        // Validate source format (http:// excluded for security)
        if !Self::ALLOWED_PREFIXES
            .iter()
            .any(|p| self.source.starts_with(p))
        {
            return Err(format!(
                "Image watermark source must start with one of {:?}, got '{}'",
                Self::ALLOWED_PREFIXES, self.source
            ));
        }

        // Check for NaN/Infinity and valid range
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(format!(
                "Image watermark opacity must be a finite value between 0.0 and 1.0, got {}",
                self.opacity
            ));
        }

        Ok(())
    }
}

impl WatermarkDefinition {
    /// Validate the watermark definition.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Text(config) => config.validate(),
            Self::Image(config) => config.validate(),
        }
    }
}

impl BucketWatermarkConfig {
    /// Validate the bucket watermark configuration.
    pub fn validate(&self, bucket_name: &str) -> Result<(), String> {
        for (rule_idx, rule) in self.rules.iter().enumerate() {
            if rule.pattern.is_empty() {
                return Err(format!(
                    "Bucket '{}': watermark rule {} has empty pattern",
                    bucket_name, rule_idx
                ));
            }

            if rule.watermarks.is_empty() {
                return Err(format!(
                    "Bucket '{}': watermark rule {} (pattern '{}') has no watermarks defined",
                    bucket_name, rule_idx, rule.pattern
                ));
            }

            for (wm_idx, watermark) in rule.watermarks.iter().enumerate() {
                watermark.validate().map_err(|e| {
                    format!(
                        "Bucket '{}': watermark rule {} (pattern '{}'), watermark {}: {}",
                        bucket_name, rule_idx, rule.pattern, wm_idx, e
                    )
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watermark_position_deserialize() {
        let positions = [
            ("top-left", WatermarkPosition::TopLeft),
            ("top-center", WatermarkPosition::TopCenter),
            ("top-right", WatermarkPosition::TopRight),
            ("center-left", WatermarkPosition::CenterLeft),
            ("center", WatermarkPosition::Center),
            ("center-right", WatermarkPosition::CenterRight),
            ("bottom-left", WatermarkPosition::BottomLeft),
            ("bottom-center", WatermarkPosition::BottomCenter),
            ("bottom-right", WatermarkPosition::BottomRight),
            ("tiled", WatermarkPosition::Tiled),
            ("diagonal-band", WatermarkPosition::DiagonalBand),
        ];

        for (yaml_val, expected) in positions {
            let yaml = format!("\"{}\"", yaml_val);
            let pos: WatermarkPosition = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(pos, expected, "Failed for {}", yaml_val);
        }
    }

    #[test]
    fn test_watermark_position_cache_str() {
        assert_eq!(WatermarkPosition::TopLeft.as_cache_str(), "tl");
        assert_eq!(WatermarkPosition::Center.as_cache_str(), "c");
        assert_eq!(WatermarkPosition::BottomRight.as_cache_str(), "br");
        assert_eq!(WatermarkPosition::Tiled.as_cache_str(), "tile");
        assert_eq!(WatermarkPosition::DiagonalBand.as_cache_str(), "diag");
    }

    #[test]
    fn test_text_watermark_config_deserialize() {
        let yaml = r##"
type: text
text: "Copyright {{jwt.org}} - {{date}}"
font_size: 24
color: "#FFFFFF"
opacity: 0.5
position: bottom-right
margin: 20
"##;
        let def: WatermarkDefinition = serde_yaml::from_str(yaml).unwrap();

        match def {
            WatermarkDefinition::Text(config) => {
                assert_eq!(config.text, "Copyright {{jwt.org}} - {{date}}");
                assert_eq!(config.font_size, 24);
                assert_eq!(config.color, "#FFFFFF");
                assert_eq!(config.opacity, 0.5);
                assert_eq!(config.position, WatermarkPosition::BottomRight);
                assert_eq!(config.margin, 20);
            }
            _ => panic!("Expected Text watermark"),
        }
    }

    #[test]
    fn test_text_watermark_config_defaults() {
        let yaml = r#"
type: text
text: "Test"
position: center
"#;
        let def: WatermarkDefinition = serde_yaml::from_str(yaml).unwrap();

        match def {
            WatermarkDefinition::Text(config) => {
                assert_eq!(config.font_size, 24); // default
                assert_eq!(config.color, "#FFFFFF"); // default
                assert_eq!(config.opacity, 0.5); // default
                assert_eq!(config.margin, 10); // default
            }
            _ => panic!("Expected Text watermark"),
        }
    }

    #[test]
    fn test_text_watermark_with_rotation() {
        let yaml = r#"
type: text
text: "CONFIDENTIAL"
position: tiled
rotation: -45
tile_spacing: 200
"#;
        let def: WatermarkDefinition = serde_yaml::from_str(yaml).unwrap();

        match def {
            WatermarkDefinition::Text(config) => {
                assert_eq!(config.rotation, Some(-45));
                assert_eq!(config.tile_spacing, Some(200));
            }
            _ => panic!("Expected Text watermark"),
        }
    }

    #[test]
    fn test_image_watermark_config_deserialize() {
        let yaml = r#"
type: image
source: "s3://assets/watermarks/logo.png"
width: 100
opacity: 0.7
position: top-left
margin: 15
"#;
        let def: WatermarkDefinition = serde_yaml::from_str(yaml).unwrap();

        match def {
            WatermarkDefinition::Image(config) => {
                assert_eq!(config.source, "s3://assets/watermarks/logo.png");
                assert_eq!(config.width, Some(100));
                assert_eq!(config.height, None);
                assert_eq!(config.opacity, 0.7);
                assert_eq!(config.position, WatermarkPosition::TopLeft);
                assert_eq!(config.margin, 15);
            }
            _ => panic!("Expected Image watermark"),
        }
    }

    #[test]
    fn test_image_watermark_from_url() {
        let yaml = r#"
type: image
source: "https://cdn.example.com/logo.png"
position: center
"#;
        let def: WatermarkDefinition = serde_yaml::from_str(yaml).unwrap();

        match def {
            WatermarkDefinition::Image(config) => {
                assert_eq!(config.source, "https://cdn.example.com/logo.png");
                assert_eq!(config.opacity, 0.5); // default
            }
            _ => panic!("Expected Image watermark"),
        }
    }

    #[test]
    fn test_bucket_watermark_config_deserialize() {
        let yaml = r#"
enabled: true
cache_ttl_seconds: 7200
rules:
  - pattern: "/products/previews/*"
    watermarks:
      - type: text
        text: "Preview"
        position: bottom-right
      - type: image
        source: "s3://assets/logo.png"
        position: top-left
  - pattern: "*"
    watermarks:
      - type: text
        text: "{{jwt.sub}}"
        position: bottom-center
"#;
        let config: BucketWatermarkConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert_eq!(config.cache_ttl_seconds, 7200);
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].pattern, "/products/previews/*");
        assert_eq!(config.rules[0].watermarks.len(), 2);
        assert_eq!(config.rules[1].pattern, "*");
        assert_eq!(config.rules[1].watermarks.len(), 1);
    }

    #[test]
    fn test_bucket_watermark_config_defaults() {
        let yaml = "{}";
        let config: BucketWatermarkConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(!config.enabled);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_text_watermark_validate_ok() {
        let config = TextWatermarkConfig {
            text: "Copyright".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(),
            opacity: 0.5,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_text_watermark_validate_empty_text() {
        let config = TextWatermarkConfig {
            text: "".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(),
            opacity: 0.5,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_text_watermark_validate_invalid_opacity() {
        let config = TextWatermarkConfig {
            text: "Test".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(),
            opacity: 1.5,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("opacity"));
    }

    #[test]
    fn test_text_watermark_validate_invalid_color_format() {
        let config = TextWatermarkConfig {
            text: "Test".to_string(),
            font_size: 24,
            color: "red".to_string(),
            opacity: 0.5,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hex string"));
    }

    #[test]
    fn test_text_watermark_validate_invalid_hex_length() {
        let config = TextWatermarkConfig {
            text: "Test".to_string(),
            font_size: 24,
            color: "#FFFFF".to_string(), // 5 chars, invalid
            opacity: 0.5,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("#RGB or #RRGGBB"));
    }

    #[test]
    fn test_text_watermark_validate_short_hex_ok() {
        let config = TextWatermarkConfig {
            text: "Test".to_string(),
            font_size: 24,
            color: "#FFF".to_string(), // 3 chars, valid short form
            opacity: 0.5,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_image_watermark_validate_ok() {
        let config = ImageWatermarkConfig {
            source: "s3://assets/logo.png".to_string(),
            width: Some(100),
            height: None,
            opacity: 0.7,
            position: WatermarkPosition::TopLeft,
            margin: 10,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_image_watermark_validate_url_ok() {
        let config = ImageWatermarkConfig {
            source: "https://example.com/logo.png".to_string(),
            width: None,
            height: None,
            opacity: 0.5,
            position: WatermarkPosition::Center,
            margin: 10,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_image_watermark_validate_empty_source() {
        let config = ImageWatermarkConfig {
            source: "".to_string(),
            width: None,
            height: None,
            opacity: 0.5,
            position: WatermarkPosition::Center,
            margin: 10,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_image_watermark_validate_invalid_source() {
        let config = ImageWatermarkConfig {
            source: "/local/path/logo.png".to_string(),
            width: None,
            height: None,
            opacity: 0.5,
            position: WatermarkPosition::Center,
            margin: 10,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("s3://"));
    }

    #[test]
    fn test_bucket_watermark_config_validate_ok() {
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "*".to_string(),
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
        assert!(config.validate("test-bucket").is_ok());
    }

    #[test]
    fn test_bucket_watermark_config_validate_empty_pattern() {
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "".to_string(),
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
        let result = config.validate("test-bucket");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty pattern"));
    }

    #[test]
    fn test_bucket_watermark_config_validate_empty_watermarks() {
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "*".to_string(),
                watermarks: vec![],
            }],
        };
        let result = config.validate("test-bucket");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no watermarks defined"));
    }

    #[test]
    fn test_bucket_watermark_config_validate_propagates_watermark_error() {
        let config = BucketWatermarkConfig {
            enabled: true,
            cache_ttl_seconds: 3600,
            rules: vec![WatermarkRule {
                pattern: "*".to_string(),
                watermarks: vec![WatermarkDefinition::Text(TextWatermarkConfig {
                    text: "".to_string(), // Invalid: empty text
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
        let result = config.validate("test-bucket");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("test-bucket"));
        assert!(err.contains("cannot be empty"));
    }

    #[test]
    fn test_text_watermark_validate_nan_opacity() {
        let config = TextWatermarkConfig {
            text: "Test".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(),
            opacity: f32::NAN,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("finite value"));
    }

    #[test]
    fn test_text_watermark_validate_infinity_opacity() {
        let config = TextWatermarkConfig {
            text: "Test".to_string(),
            font_size: 24,
            color: "#FFFFFF".to_string(),
            opacity: f32::INFINITY,
            position: WatermarkPosition::BottomRight,
            margin: 10,
            rotation: None,
            tile_spacing: None,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("finite value"));
    }

    #[test]
    fn test_image_watermark_validate_nan_opacity() {
        let config = ImageWatermarkConfig {
            source: "s3://assets/logo.png".to_string(),
            width: None,
            height: None,
            opacity: f32::NAN,
            position: WatermarkPosition::Center,
            margin: 10,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("finite value"));
    }

    #[test]
    fn test_image_watermark_validate_http_rejected() {
        // http:// is rejected for security reasons (MITM attacks)
        let config = ImageWatermarkConfig {
            source: "http://example.com/logo.png".to_string(),
            width: None,
            height: None,
            opacity: 0.5,
            position: WatermarkPosition::Center,
            margin: 10,
        };
        let result = config.validate();
        assert!(result.is_err());
        // Should suggest s3:// or https://
        let err = result.unwrap_err();
        assert!(err.contains("s3://") || err.contains("https://"));
    }

    #[test]
    fn test_bucket_watermark_config_default_uses_correct_cache_ttl() {
        // Ensure Default implementation uses 3600 (not u64 default of 0)
        let config = BucketWatermarkConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert!(config.rules.is_empty());
    }
}
