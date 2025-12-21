/// Compression configuration structures
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use super::algorithms::{AlgorithmConfig, Compression};
use super::error::CompressionError;

/// Global compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression globally
    #[serde(default)]
    pub enabled: bool,

    /// Default compression algorithm to use
    #[serde(default = "default_algorithm")]
    pub default_algorithm: String,

    /// Default compression level (1-9, safe for all algorithms)
    #[serde(default = "default_compression_level")]
    pub compression_level: u32,

    /// Minimum response size in bytes to compress (default: 1024)
    #[serde(default = "default_min_size")]
    pub min_response_size_bytes: usize,

    /// Maximum response size in bytes to compress (default: 100MB)
    #[serde(default = "default_max_size")]
    pub max_response_size_bytes: usize,

    /// Per-algorithm configuration
    #[serde(default = "default_algorithms")]
    pub algorithms: HashMap<String, AlgorithmConfig>,
}

fn default_algorithm() -> String {
    "gzip".to_string()
}

fn default_compression_level() -> u32 {
    6
}

fn default_min_size() -> usize {
    1024 // 1KB
}

fn default_max_size() -> usize {
    104857600 // 100MB
}

fn default_algorithms() -> HashMap<String, AlgorithmConfig> {
    let mut map = HashMap::new();
    map.insert("gzip".to_string(), AlgorithmConfig::gzip_default());
    map.insert("br".to_string(), AlgorithmConfig::brotli_default());
    map.insert("brotli".to_string(), AlgorithmConfig::brotli_default()); // Accept both "br" and "brotli"
    map.insert("deflate".to_string(), AlgorithmConfig::deflate_default());
    map
}

/// Get the maximum compression level for a given algorithm name
fn max_level_for_algorithm(name: &str) -> u32 {
    match name.to_lowercase().as_str() {
        "gzip" | "deflate" => 9, // flate2 supports 0-9, we use 1-9
        "br" | "brotli" => 11,   // brotli supports 0-11, we use 1-11
        _ => 9,                  // Safe default for unknown algorithms
    }
}

impl CompressionConfig {
    /// Create a new compression configuration with defaults
    pub fn new() -> Self {
        CompressionConfig {
            enabled: false,
            default_algorithm: default_algorithm(),
            compression_level: default_compression_level(),
            min_response_size_bytes: default_min_size(),
            max_response_size_bytes: default_max_size(),
            algorithms: default_algorithms(),
        }
    }

    /// Validate the compression configuration
    pub fn validate(&self) -> Result<(), CompressionError> {
        // Validate compression level (use 1-9 as it's safe for all algorithms)
        if !(1..=9).contains(&self.compression_level) {
            return Err(CompressionError::InvalidConfig(format!(
                "compression_level must be 1-9 (safe for all algorithms), got {}",
                self.compression_level
            )));
        }

        // Validate size thresholds
        if self.min_response_size_bytes >= self.max_response_size_bytes {
            return Err(CompressionError::InvalidConfig(
                "min_response_size_bytes must be less than max_response_size_bytes".to_string(),
            ));
        }

        // Validate default algorithm exists
        Compression::from_str(&self.default_algorithm)?;

        // Validate at least one algorithm is enabled
        if !self.algorithms.values().any(|cfg| cfg.enabled) {
            return Err(CompressionError::InvalidConfig(
                "at least one compression algorithm must be enabled".to_string(),
            ));
        }

        // Validate each algorithm's level based on its type
        for (name, cfg) in &self.algorithms {
            let max_level = max_level_for_algorithm(name);
            if !(1..=max_level).contains(&cfg.level) {
                return Err(CompressionError::InvalidConfig(format!(
                    "algorithm '{}' level must be 1-{}, got {}",
                    name, max_level, cfg.level
                )));
            }
        }

        Ok(())
    }

    /// Get configuration for a specific algorithm
    /// Looks up by header value first ("br"), then by full name ("brotli")
    pub fn get_algorithm(&self, algo: Compression) -> Option<&AlgorithmConfig> {
        // Try header value first (e.g., "br", "gzip", "deflate")
        self.algorithms
            .get(algo.to_header_value())
            // Also try alternative names for brotli
            .or_else(|| match algo {
                Compression::Brotli => self.algorithms.get("brotli"),
                _ => None,
            })
    }

    /// Check if an algorithm is enabled
    pub fn is_algorithm_enabled(&self, algo: Compression) -> bool {
        self.get_algorithm(algo)
            .map(|cfg| cfg.enabled)
            .unwrap_or(false)
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        CompressionConfig::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_config_new() {
        let config = CompressionConfig::new();
        assert!(!config.enabled);
        assert_eq!(config.default_algorithm, "gzip");
        assert_eq!(config.compression_level, 6);
        assert_eq!(config.min_response_size_bytes, 1024);
        assert_eq!(config.max_response_size_bytes, 104857600);
    }

    #[test]
    fn test_compression_config_validate_valid() {
        let config = CompressionConfig::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_compression_config_validate_invalid_level() {
        let mut config = CompressionConfig::new();
        config.compression_level = 0;
        assert!(config.validate().is_err());

        // Level 10+ is invalid for global config (must be safe for all algorithms)
        config.compression_level = 10;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_compression_config_validate_per_algorithm_levels() {
        let mut config = CompressionConfig::new();

        // gzip level 10 should fail (max 9)
        config.algorithms.get_mut("gzip").unwrap().level = 10;
        assert!(config.validate().is_err());

        // Reset gzip to valid level
        config.algorithms.get_mut("gzip").unwrap().level = 6;

        // brotli level 11 should succeed (brotli supports 1-11)
        config.algorithms.get_mut("br").unwrap().level = 11;
        assert!(config.validate().is_ok());

        // brotli level 12 should fail
        config.algorithms.get_mut("br").unwrap().level = 12;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_compression_config_validate_size_mismatch() {
        let mut config = CompressionConfig::new();
        config.min_response_size_bytes = 1000;
        config.max_response_size_bytes = 500;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_compression_config_get_algorithm() {
        let config = CompressionConfig::new();
        assert!(config.get_algorithm(Compression::Gzip).is_some());
        assert!(config.get_algorithm(Compression::Brotli).is_some());
    }

    #[test]
    fn test_compression_config_is_algorithm_enabled() {
        let config = CompressionConfig::new();
        assert!(config.is_algorithm_enabled(Compression::Gzip));
        assert!(config.is_algorithm_enabled(Compression::Brotli));
        assert!(!config.is_algorithm_enabled(Compression::Deflate));
    }
}
