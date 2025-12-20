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

    /// Default compression level (1-11)
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
    map.insert("deflate".to_string(), AlgorithmConfig::deflate_default());
    map
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
        // Validate compression level
        if !(1..=11).contains(&self.compression_level) {
            return Err(CompressionError::InvalidConfig(format!(
                "compression_level must be 1-11, got {}",
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

        // Validate each algorithm's level
        for (name, cfg) in &self.algorithms {
            if !(1..=11).contains(&cfg.level) {
                return Err(CompressionError::InvalidConfig(format!(
                    "algorithm {} level must be 1-11, got {}",
                    name, cfg.level
                )));
            }
        }

        Ok(())
    }

    /// Get configuration for a specific algorithm
    pub fn get_algorithm(&self, algo: Compression) -> Option<&AlgorithmConfig> {
        self.algorithms.get(algo.to_header_value())
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

        config.compression_level = 12;
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
