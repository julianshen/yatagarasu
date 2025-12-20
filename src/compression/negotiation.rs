/// Accept-Encoding header parsing and compression algorithm negotiation
use std::str::FromStr;

use super::algorithms::Compression;
use super::config::CompressionConfig;

/// Represents a single encoding in Accept-Encoding header with quality value
#[derive(Debug, Clone, PartialEq)]
struct EncodingPreference {
    encoding: String,
    quality: f32,
}

impl EncodingPreference {
    /// Parse a single encoding preference (e.g., "gzip;q=0.8")
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        let parts: Vec<&str> = s.split(';').collect();
        let encoding = parts[0].trim().to_lowercase();

        let quality = if parts.len() > 1 {
            // Parse quality value (e.g., "q=0.8")
            let q_part = parts[1].trim();
            if let Some(q_value) = q_part.strip_prefix("q=") {
                q_value.parse::<f32>().unwrap_or(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        Some(EncodingPreference { encoding, quality })
    }
}

/// Negotiate compression algorithm based on Accept-Encoding header and server config
///
/// # Arguments
/// * `accept_encoding` - Value of Accept-Encoding header (or None if not present)
/// * `config` - Compression configuration
///
/// # Returns
/// * `Some(Compression)` - Selected compression algorithm
/// * `None` - No acceptable compression algorithm
///
/// # Algorithm
/// 1. Parse Accept-Encoding header into preferences with quality values
/// 2. Sort by quality (highest first)
/// 3. Select first enabled algorithm from preferences
/// 4. Fall back to default algorithm if available
pub fn negotiate_compression(
    accept_encoding: Option<&str>,
    config: &CompressionConfig,
) -> Option<Compression> {
    if !config.enabled {
        return None;
    }

    let accept_encoding = accept_encoding?;

    // Parse all preferences
    let mut preferences: Vec<EncodingPreference> = accept_encoding
        .split(',')
        .filter_map(EncodingPreference::parse)
        .collect();

    // Sort by quality (highest first)
    preferences.sort_by(|a, b| {
        b.quality
            .partial_cmp(&a.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Try each preference in order
    for pref in preferences {
        if pref.quality == 0.0 {
            // Quality 0 means "not acceptable"
            continue;
        }

        // Try to parse as compression algorithm
        if let Ok(algo) = Compression::from_str(&pref.encoding) {
            if config.is_algorithm_enabled(algo) {
                return Some(algo);
            }
        }

        // Handle wildcard
        if pref.encoding == "*" {
            // Return first enabled algorithm
            if config.is_algorithm_enabled(Compression::Gzip) {
                return Some(Compression::Gzip);
            }
            if config.is_algorithm_enabled(Compression::Brotli) {
                return Some(Compression::Brotli);
            }
            if config.is_algorithm_enabled(Compression::Deflate) {
                return Some(Compression::Deflate);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_preference_parse_simple() {
        let pref = EncodingPreference::parse("gzip").unwrap();
        assert_eq!(pref.encoding, "gzip");
        assert_eq!(pref.quality, 1.0);
    }

    #[test]
    fn test_encoding_preference_parse_with_quality() {
        let pref = EncodingPreference::parse("gzip;q=0.8").unwrap();
        assert_eq!(pref.encoding, "gzip");
        assert_eq!(pref.quality, 0.8);
    }

    #[test]
    fn test_encoding_preference_parse_case_insensitive() {
        let pref = EncodingPreference::parse("GZIP").unwrap();
        assert_eq!(pref.encoding, "gzip");
    }

    #[test]
    fn test_negotiate_compression_disabled() {
        let config = CompressionConfig::new();
        assert_eq!(negotiate_compression(Some("gzip"), &config), None);
    }

    #[test]
    fn test_negotiate_compression_no_header() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        assert_eq!(negotiate_compression(None, &config), None);
    }

    #[test]
    fn test_negotiate_compression_gzip() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        assert_eq!(
            negotiate_compression(Some("gzip"), &config),
            Some(Compression::Gzip)
        );
    }

    #[test]
    fn test_negotiate_compression_brotli() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        assert_eq!(
            negotiate_compression(Some("br"), &config),
            Some(Compression::Brotli)
        );
    }

    #[test]
    fn test_negotiate_compression_multiple_with_quality() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        // Client prefers brotli (0.9) over gzip (0.8)
        assert_eq!(
            negotiate_compression(Some("gzip;q=0.8, br;q=0.9"), &config),
            Some(Compression::Brotli)
        );
    }

    #[test]
    fn test_negotiate_compression_quality_zero() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        // gzip with q=0 means not acceptable
        assert_eq!(
            negotiate_compression(Some("gzip;q=0, br"), &config),
            Some(Compression::Brotli)
        );
    }

    #[test]
    fn test_negotiate_compression_wildcard() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        // Wildcard should match first enabled algorithm
        let result = negotiate_compression(Some("*"), &config);
        assert!(result.is_some());
    }

    #[test]
    fn test_negotiate_compression_disabled_algorithm() {
        let mut config = CompressionConfig::new();
        config.enabled = true;
        config.algorithms.get_mut("deflate").unwrap().enabled = true;
        config.algorithms.get_mut("gzip").unwrap().enabled = false;
        config.algorithms.get_mut("br").unwrap().enabled = false;

        assert_eq!(
            negotiate_compression(Some("gzip, br, deflate"), &config),
            Some(Compression::Deflate)
        );
    }
}
