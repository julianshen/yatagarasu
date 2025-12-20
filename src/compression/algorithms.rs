/// Compression algorithm definitions and utilities
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::error::CompressionError;

/// Supported compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compression {
    /// DEFLATE compression (RFC 1951)
    Deflate,
    /// GZIP compression (RFC 1952)
    Gzip,
    /// Brotli compression (RFC 7932)
    #[serde(rename = "br")]
    Brotli,
}

impl Compression {
    /// Convert compression algorithm to HTTP Content-Encoding header value
    pub fn to_header_value(&self) -> &'static str {
        match self {
            Compression::Gzip => "gzip",
            Compression::Brotli => "br",
            Compression::Deflate => "deflate",
        }
    }

    /// Parse compression algorithm from string (case-insensitive)
    pub fn parse_algorithm(s: &str) -> Result<Self, CompressionError> {
        match s.to_lowercase().as_str() {
            "gzip" => Ok(Compression::Gzip),
            "br" | "brotli" => Ok(Compression::Brotli),
            "deflate" => Ok(Compression::Deflate),
            _ => Err(CompressionError::InvalidAlgorithm(format!(
                "unsupported algorithm: {}",
                s
            ))),
        }
    }

    /// Check if algorithm is enabled in configuration
    pub fn is_enabled(&self, config: &AlgorithmConfig) -> bool {
        config.enabled
    }
}

impl FromStr for Compression {
    type Err = CompressionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_algorithm(s)
    }
}

impl fmt::Display for Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_header_value())
    }
}

/// Default enabled state for algorithms
fn default_enabled() -> bool {
    true
}

/// Default compression level
fn default_level() -> u32 {
    6
}

/// Configuration for a specific compression algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// Whether this algorithm is enabled (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Compression level (1-9 for gzip/deflate, 1-11 for brotli; default: 6)
    #[serde(default = "default_level")]
    pub level: u32,
}

impl AlgorithmConfig {
    /// Create a new algorithm configuration
    pub fn new(enabled: bool, level: u32) -> Result<Self, CompressionError> {
        if !(1..=11).contains(&level) {
            return Err(CompressionError::InvalidConfig(format!(
                "compression level must be 1-11, got {}",
                level
            )));
        }
        Ok(AlgorithmConfig { enabled, level })
    }

    /// Default configuration for gzip (enabled, level 6)
    pub fn gzip_default() -> Self {
        AlgorithmConfig {
            enabled: true,
            level: 6,
        }
    }

    /// Default configuration for brotli (enabled, level 4)
    pub fn brotli_default() -> Self {
        AlgorithmConfig {
            enabled: true,
            level: 4,
        }
    }

    /// Default configuration for deflate (disabled)
    pub fn deflate_default() -> Self {
        AlgorithmConfig {
            enabled: false,
            level: 6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_to_header_value() {
        assert_eq!(Compression::Gzip.to_header_value(), "gzip");
        assert_eq!(Compression::Brotli.to_header_value(), "br");
        assert_eq!(Compression::Deflate.to_header_value(), "deflate");
    }

    #[test]
    fn test_compression_from_str_gzip() {
        assert_eq!(Compression::from_str("gzip").unwrap(), Compression::Gzip);
        assert_eq!(Compression::from_str("GZIP").unwrap(), Compression::Gzip);
        assert_eq!(Compression::from_str("GzIp").unwrap(), Compression::Gzip);
    }

    #[test]
    fn test_compression_from_str_brotli() {
        assert_eq!(Compression::from_str("br").unwrap(), Compression::Brotli);
        assert_eq!(
            Compression::from_str("brotli").unwrap(),
            Compression::Brotli
        );
        assert_eq!(Compression::from_str("BR").unwrap(), Compression::Brotli);
    }

    #[test]
    fn test_compression_from_str_deflate() {
        assert_eq!(
            Compression::from_str("deflate").unwrap(),
            Compression::Deflate
        );
        assert_eq!(
            Compression::from_str("DEFLATE").unwrap(),
            Compression::Deflate
        );
    }

    #[test]
    fn test_compression_from_str_invalid() {
        assert!(Compression::from_str("invalid").is_err());
        assert!(Compression::from_str("").is_err());
    }

    #[test]
    fn test_algorithm_config_valid_levels() {
        for level in 1..=11 {
            assert!(AlgorithmConfig::new(true, level).is_ok());
        }
    }

    #[test]
    fn test_algorithm_config_invalid_levels() {
        assert!(AlgorithmConfig::new(true, 0).is_err());
        assert!(AlgorithmConfig::new(true, 12).is_err());
    }

    #[test]
    fn test_algorithm_config_defaults() {
        let gzip = AlgorithmConfig::gzip_default();
        assert!(gzip.enabled);
        assert_eq!(gzip.level, 6);

        let brotli = AlgorithmConfig::brotli_default();
        assert!(brotli.enabled);
        assert_eq!(brotli.level, 4);

        let deflate = AlgorithmConfig::deflate_default();
        assert!(!deflate.enabled);
    }
}
