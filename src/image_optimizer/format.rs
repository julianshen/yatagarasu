//! Auto-format selection based on Accept header
//!
//! Implements intelligent format selection that balances:
//! - Browser compatibility (via Accept header)
//! - Compression efficiency (AVIF > WebP > JPEG)
//! - Source format characteristics (preserve PNG transparency)

use super::params::OutputFormat;

/// Configuration for auto-format selection
#[derive(Debug, Clone)]
pub struct AutoFormatConfig {
    /// Enable auto-format selection
    pub enabled: bool,
    /// Prefer AVIF when supported
    pub prefer_avif: bool,
    /// Prefer WebP when supported
    pub prefer_webp: bool,
    /// Minimum file size savings (%) to justify format conversion
    pub min_savings_percent: u8,
}

impl Default for AutoFormatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_avif: true,
            prefer_webp: true,
            min_savings_percent: 10,
        }
    }
}

/// Parsed Accept header preference
#[derive(Debug, Clone)]
struct FormatPreference {
    format: OutputFormat,
    quality: f32,
}

/// Select the best output format based on Accept header and source format
///
/// # Arguments
/// * `accept_header` - The Accept header value (e.g., "image/avif,image/webp,image/*")
/// * `source_format` - The detected source image format
/// * `has_transparency` - Whether the source image has transparency
/// * `config` - Auto-format configuration
///
/// # Returns
/// The selected output format
pub fn select_format(
    accept_header: Option<&str>,
    source_format: OutputFormat,
    has_transparency: bool,
    config: &AutoFormatConfig,
) -> OutputFormat {
    if !config.enabled {
        return source_format;
    }

    let accept = match accept_header {
        Some(h) => h,
        None => return source_format, // No Accept header, keep original
    };

    // Parse Accept header
    let preferences = parse_accept_header(accept);

    // If source has transparency, limit to formats that support it
    let candidates: Vec<OutputFormat> = if has_transparency {
        vec![OutputFormat::Avif, OutputFormat::WebP, OutputFormat::Png]
    } else {
        vec![OutputFormat::Avif, OutputFormat::WebP, OutputFormat::Jpeg]
    };

    // Find the best supported format
    for candidate in candidates {
        // Check if this format is acceptable
        if !is_format_acceptable(&preferences, candidate) {
            continue;
        }

        // Apply preferences from config
        match candidate {
            OutputFormat::Avif if !config.prefer_avif => continue,
            OutputFormat::WebP if !config.prefer_webp => continue,
            _ => {}
        }

        return candidate;
    }

    // Fall back to source format
    source_format
}

/// Parse Accept header into format preferences with quality values
fn parse_accept_header(accept: &str) -> Vec<FormatPreference> {
    let mut preferences = Vec::new();

    for part in accept.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Split on semicolon to get quality
        let (media_type, quality) = if let Some((mt, params)) = part.split_once(';') {
            let q = parse_quality(params);
            (mt.trim(), q)
        } else {
            (part, 1.0)
        };

        // Parse media type to format
        if let Some(format) = parse_image_media_type(media_type) {
            preferences.push(FormatPreference { format, quality });
        }
    }

    // Sort by quality (highest first)
    preferences.sort_by(|a, b| {
        b.quality
            .partial_cmp(&a.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    preferences
}

/// Parse quality value from parameters (e.g., "q=0.8")
fn parse_quality(params: &str) -> f32 {
    for param in params.split(';') {
        let param = param.trim();
        if let Some(q) = param.strip_prefix("q=") {
            if let Ok(quality) = q.parse::<f32>() {
                return quality.clamp(0.0, 1.0);
            }
        }
    }
    1.0
}

/// Parse image media type to OutputFormat
fn parse_image_media_type(media_type: &str) -> Option<OutputFormat> {
    match media_type.to_lowercase().as_str() {
        "image/avif" => Some(OutputFormat::Avif),
        "image/webp" => Some(OutputFormat::WebP),
        "image/jpeg" | "image/jpg" => Some(OutputFormat::Jpeg),
        "image/png" => Some(OutputFormat::Png),
        "image/*" => Some(OutputFormat::Auto), // Wildcard
        _ => None,
    }
}

/// Check if a format is acceptable based on parsed preferences
fn is_format_acceptable(preferences: &[FormatPreference], format: OutputFormat) -> bool {
    // If no preferences, accept all
    if preferences.is_empty() {
        return true;
    }

    for pref in preferences {
        // Exact match with non-zero quality
        if pref.format == format && pref.quality > 0.0 {
            return true;
        }

        // Wildcard match
        if pref.format == OutputFormat::Auto && pref.quality > 0.0 {
            return true;
        }
    }

    false
}

/// Detect if an image has transparency based on format
pub fn format_supports_transparency(format: OutputFormat) -> bool {
    matches!(
        format,
        OutputFormat::Png | OutputFormat::WebP | OutputFormat::Avif
    )
}

/// Get the Vary header value for auto-format
pub fn vary_header() -> &'static str {
    "Accept"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_header_simple() {
        let prefs = parse_accept_header("image/webp");
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0].format, OutputFormat::WebP);
        assert_eq!(prefs[0].quality, 1.0);
    }

    #[test]
    fn test_parse_accept_header_with_quality() {
        let prefs = parse_accept_header("image/avif;q=0.9, image/webp;q=0.8");
        assert_eq!(prefs.len(), 2);
        assert_eq!(prefs[0].format, OutputFormat::Avif);
        assert_eq!(prefs[0].quality, 0.9);
        assert_eq!(prefs[1].format, OutputFormat::WebP);
        assert_eq!(prefs[1].quality, 0.8);
    }

    #[test]
    fn test_parse_accept_header_sorted_by_quality() {
        let prefs = parse_accept_header("image/jpeg;q=0.5, image/webp;q=0.9, image/avif;q=0.7");
        assert_eq!(prefs[0].format, OutputFormat::WebP);
        assert_eq!(prefs[1].format, OutputFormat::Avif);
        assert_eq!(prefs[2].format, OutputFormat::Jpeg);
    }

    #[test]
    fn test_select_format_avif_preferred() {
        let config = AutoFormatConfig::default();
        let format = select_format(
            Some("image/avif,image/webp,image/*"),
            OutputFormat::Jpeg,
            false,
            &config,
        );
        assert_eq!(format, OutputFormat::Avif);
    }

    #[test]
    fn test_select_format_webp_fallback() {
        let config = AutoFormatConfig {
            prefer_avif: false,
            ..Default::default()
        };
        let format = select_format(
            Some("image/avif,image/webp,image/*"),
            OutputFormat::Jpeg,
            false,
            &config,
        );
        assert_eq!(format, OutputFormat::WebP);
    }

    #[test]
    fn test_select_format_preserve_transparency() {
        let config = AutoFormatConfig::default();
        let format = select_format(
            Some("image/avif,image/webp,image/jpeg"),
            OutputFormat::Png,
            true, // has transparency
            &config,
        );
        // Should not select JPEG for transparent images
        assert_ne!(format, OutputFormat::Jpeg);
    }

    #[test]
    fn test_select_format_no_accept_header() {
        let config = AutoFormatConfig::default();
        let format = select_format(None, OutputFormat::Jpeg, false, &config);
        assert_eq!(format, OutputFormat::Jpeg); // Keep original
    }

    #[test]
    fn test_select_format_disabled() {
        let config = AutoFormatConfig {
            enabled: false,
            ..Default::default()
        };
        let format = select_format(
            Some("image/avif,image/webp"),
            OutputFormat::Jpeg,
            false,
            &config,
        );
        assert_eq!(format, OutputFormat::Jpeg); // Keep original
    }

    #[test]
    fn test_is_format_acceptable() {
        let prefs = parse_accept_header("image/webp, image/jpeg;q=0.5");
        assert!(is_format_acceptable(&prefs, OutputFormat::WebP));
        assert!(is_format_acceptable(&prefs, OutputFormat::Jpeg));
        assert!(!is_format_acceptable(&prefs, OutputFormat::Avif));
    }

    #[test]
    fn test_is_format_acceptable_wildcard() {
        let prefs = parse_accept_header("image/*");
        assert!(is_format_acceptable(&prefs, OutputFormat::WebP));
        assert!(is_format_acceptable(&prefs, OutputFormat::Avif));
        assert!(is_format_acceptable(&prefs, OutputFormat::Jpeg));
    }

    #[test]
    fn test_format_supports_transparency() {
        assert!(format_supports_transparency(OutputFormat::Png));
        assert!(format_supports_transparency(OutputFormat::WebP));
        assert!(format_supports_transparency(OutputFormat::Avif));
        assert!(!format_supports_transparency(OutputFormat::Jpeg));
    }
}
