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
        // Validate compression level if set
        if let Some(level) = self.compression_level {
            if !(1..=11).contains(&level) {
                return Err(CompressionError::InvalidConfig(format!(
                    "bucket compression level must be 1-11, got {}",
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
                if !(1..=11).contains(&config.level) {
                    return Err(CompressionError::InvalidConfig(format!(
                        "algorithm {} level must be 1-11, got {}",
                        name, config.level
                    )));
                }
            }
        }

        Ok(())
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
        config.compression_level = Some(12);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bucket_config_validate_size_mismatch() {
        let mut config = BucketCompressionConfig::new();
        config.min_response_size_bytes = Some(10_000_000);
        config.max_response_size_bytes = Some(1024);
        assert!(config.validate().is_err());
    }
}
