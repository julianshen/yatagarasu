//! Per-bucket compression configuration (Phase 40.5)
//!
//! This module implements:
//! - Per-bucket compression settings
//! - Override of global compression settings
//! - Bucket-specific compression levels and algorithms

use super::algorithms::AlgorithmConfig;
use super::error::CompressionError;
use std::collections::HashMap;

/// Per-bucket compression configuration
#[derive(Debug, Clone)]
pub struct BucketCompressionConfig {
    /// Enable/disable compression for this bucket
    pub enabled: Option<bool>,
    /// Default algorithm for this bucket (overrides global)
    pub default_algorithm: Option<String>,
    /// Compression level for this bucket (overrides global)
    pub compression_level: Option<u32>,
    /// Minimum response size to compress (overrides global)
    pub min_response_size_bytes: Option<usize>,
    /// Maximum response size to compress (overrides global)
    pub max_response_size_bytes: Option<usize>,
    /// Per-algorithm settings for this bucket
    pub algorithms: Option<HashMap<String, AlgorithmConfig>>,
}

impl BucketCompressionConfig {
    /// Create a new empty bucket compression config (all None = use global defaults)
    pub fn new() -> Self {
        Self {
            enabled: None,
            default_algorithm: None,
            compression_level: None,
            min_response_size_bytes: None,
            max_response_size_bytes: None,
            algorithms: None,
        }
    }

    /// Check if compression is enabled for this bucket
    /// Returns the bucket setting if present, otherwise None (use global)
    pub fn is_enabled(&self) -> Option<bool> {
        self.enabled
    }

    /// Get compression level for this bucket
    /// Returns the bucket setting if present, otherwise None (use global)
    pub fn get_compression_level(&self) -> Option<u32> {
        self.compression_level
    }

    /// Validate bucket configuration
    pub fn validate(&self) -> Result<(), CompressionError> {
        // Validate compression level if set (use 1-9 as it's safe for all algorithms)
        if let Some(level) = self.compression_level {
            if !(1..=9).contains(&level) {
                return Err(CompressionError::InvalidConfig(format!(
                    "bucket compression level must be 1-9 (safe for all algorithms), got {}",
                    level
                )));
            }
        }

        // Validate size thresholds if both are set
        if let (Some(min), Some(max)) = (self.min_response_size_bytes, self.max_response_size_bytes)
        {
            if min >= max {
                return Err(CompressionError::InvalidConfig(
                    "min_response_size_bytes must be less than max_response_size_bytes".to_string(),
                ));
            }
        }

        // Validate per-algorithm settings if present
        if let Some(algos) = &self.algorithms {
            for (name, config) in algos {
                let max_level = Self::max_level_for_algorithm(name);
                if !(1..=max_level).contains(&config.level) {
                    return Err(CompressionError::InvalidConfig(format!(
                        "algorithm '{}' level must be 1-{}, got {}",
                        name, max_level, config.level
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get the maximum compression level for a given algorithm name
    fn max_level_for_algorithm(name: &str) -> u32 {
        match name.to_lowercase().as_str() {
            "gzip" | "deflate" => 9, // flate2 supports 0-9, we use 1-9
            "br" | "brotli" => 11,   // brotli supports 0-11, we use 1-11
            _ => 9,                  // Safe default for unknown algorithms
        }
    }
}

impl Default for BucketCompressionConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_config_new() {
        let config = BucketCompressionConfig::new();
        assert!(config.enabled.is_none());
        assert!(config.default_algorithm.is_none());
        assert!(config.compression_level.is_none());
        assert!(config.min_response_size_bytes.is_none());
        assert!(config.max_response_size_bytes.is_none());
        assert!(config.algorithms.is_none());
    }

    #[test]
    fn test_bucket_config_default() {
        let config = BucketCompressionConfig::default();
        assert!(config.enabled.is_none());
    }

    #[test]
    fn test_bucket_config_disable_compression() {
        let mut config = BucketCompressionConfig::new();
        config.enabled = Some(false);
        assert_eq!(config.is_enabled(), Some(false));
    }

    #[test]
    fn test_bucket_config_enable_compression() {
        let mut config = BucketCompressionConfig::new();
        config.enabled = Some(true);
        assert_eq!(config.is_enabled(), Some(true));
    }

    #[test]
    fn test_bucket_config_set_compression_level() {
        let mut config = BucketCompressionConfig::new();
        config.compression_level = Some(9);
        assert_eq!(config.get_compression_level(), Some(9));
    }

    #[test]
    fn test_bucket_config_validate_valid() {
        let mut config = BucketCompressionConfig::new();
        config.compression_level = Some(6);
        config.min_response_size_bytes = Some(1024);
        config.max_response_size_bytes = Some(10_000_000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bucket_config_validate_invalid_level() {
        let mut config = BucketCompressionConfig::new();
        // Level 10+ is invalid for bucket config (must be safe for all algorithms)
        config.compression_level = Some(10);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bucket_config_validate_size_mismatch() {
        let mut config = BucketCompressionConfig::new();
        config.min_response_size_bytes = Some(10_000_000);
        config.max_response_size_bytes = Some(1024);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bucket_config_validate_per_algorithm_levels() {
        let mut config = BucketCompressionConfig::new();
        let mut algos = std::collections::HashMap::new();

        // gzip level 10 should fail (max 9)
        algos.insert(
            "gzip".to_string(),
            AlgorithmConfig::new(true, 10).unwrap_or(AlgorithmConfig {
                enabled: true,
                level: 10,
            }),
        );
        config.algorithms = Some(algos.clone());
        assert!(config.validate().is_err());

        // Reset with valid gzip level
        algos.clear();
        algos.insert(
            "gzip".to_string(),
            AlgorithmConfig {
                enabled: true,
                level: 6,
            },
        );
        config.algorithms = Some(algos.clone());
        assert!(config.validate().is_ok());

        // brotli level 11 should succeed
        algos.insert(
            "br".to_string(),
            AlgorithmConfig {
                enabled: true,
                level: 11,
            },
        );
        config.algorithms = Some(algos.clone());
        assert!(config.validate().is_ok());

        // brotli level 12 should fail
        algos.insert(
            "br".to_string(),
            AlgorithmConfig {
                enabled: true,
                level: 12,
            },
        );
        config.algorithms = Some(algos);
        assert!(config.validate().is_err());
    }
}
