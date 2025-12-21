//! Compression metrics and observability (Phase 40.6)
//!
//! This module implements:
//! - Compression metrics tracking
//! - Compression decision logging
//! - Performance monitoring

use super::algorithms::Compression;
use std::time::Duration;

/// Compression metrics for a single response
#[derive(Debug, Clone)]
pub struct CompressionMetrics {
    /// Original response size in bytes
    pub original_size: usize,
    /// Compressed response size in bytes
    pub compressed_size: usize,
    /// Compression algorithm used
    pub algorithm: Compression,
    /// Time taken to compress
    pub compression_time: Duration,
}

impl CompressionMetrics {
    /// Create new compression metrics
    pub fn new(
        original_size: usize,
        compressed_size: usize,
        algorithm: Compression,
        compression_time: Duration,
    ) -> Self {
        Self {
            original_size,
            compressed_size,
            algorithm,
            compression_time,
        }
    }

    /// Calculate compression ratio (compressed / original)
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            self.compressed_size as f64 / self.original_size as f64
        }
    }

    /// Calculate bytes saved by compression
    pub fn bytes_saved(&self) -> usize {
        self.original_size.saturating_sub(self.compressed_size)
    }

    /// Calculate percentage saved
    pub fn percentage_saved(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            (self.bytes_saved() as f64 / self.original_size as f64) * 100.0
        }
    }

    /// Get compression throughput (bytes/second)
    pub fn throughput_bytes_per_sec(&self) -> f64 {
        if self.compression_time.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.original_size as f64 / self.compression_time.as_secs_f64()
        }
    }
}

/// Compression decision reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressionDecisionReason {
    /// Compression disabled globally
    DisabledGlobally,
    /// Compression disabled for bucket
    DisabledForBucket,
    /// Response already compressed
    AlreadyCompressed,
    /// Content type not compressible
    NonCompressibleContentType,
    /// Response too small
    ResponseTooSmall,
    /// Response too large
    ResponseTooLarge,
    /// No acceptable algorithm from client
    NoAcceptableAlgorithm,
    /// Compression applied successfully
    Compressed,
}

impl CompressionDecisionReason {
    /// Check if this is a successful compression decision
    pub fn is_compressed(&self) -> bool {
        matches!(self, CompressionDecisionReason::Compressed)
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            CompressionDecisionReason::DisabledGlobally => "compression disabled globally",
            CompressionDecisionReason::DisabledForBucket => "compression disabled for bucket",
            CompressionDecisionReason::AlreadyCompressed => "response already compressed",
            CompressionDecisionReason::NonCompressibleContentType => {
                "content type not compressible"
            }
            CompressionDecisionReason::ResponseTooSmall => "response too small",
            CompressionDecisionReason::ResponseTooLarge => "response too large",
            CompressionDecisionReason::NoAcceptableAlgorithm => "no acceptable algorithm",
            CompressionDecisionReason::Compressed => "compression applied",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_metrics_new() {
        let metrics =
            CompressionMetrics::new(1000, 500, Compression::Gzip, Duration::from_millis(10));
        assert_eq!(metrics.original_size, 1000);
        assert_eq!(metrics.compressed_size, 500);
    }

    #[test]
    fn test_compression_ratio() {
        let metrics =
            CompressionMetrics::new(1000, 500, Compression::Gzip, Duration::from_millis(10));
        assert_eq!(metrics.compression_ratio(), 0.5);
    }

    #[test]
    fn test_bytes_saved() {
        let metrics =
            CompressionMetrics::new(1000, 300, Compression::Gzip, Duration::from_millis(10));
        assert_eq!(metrics.bytes_saved(), 700);
    }

    #[test]
    fn test_percentage_saved() {
        let metrics =
            CompressionMetrics::new(1000, 500, Compression::Gzip, Duration::from_millis(10));
        assert_eq!(metrics.percentage_saved(), 50.0);
    }

    #[test]
    fn test_throughput_bytes_per_sec() {
        let metrics = CompressionMetrics::new(1000, 500, Compression::Gzip, Duration::from_secs(1));
        assert_eq!(metrics.throughput_bytes_per_sec(), 1000.0);
    }

    #[test]
    fn test_compression_decision_reason_is_compressed() {
        assert!(CompressionDecisionReason::Compressed.is_compressed());
        assert!(!CompressionDecisionReason::DisabledGlobally.is_compressed());
    }

    #[test]
    fn test_compression_decision_reason_description() {
        assert_eq!(
            CompressionDecisionReason::Compressed.description(),
            "compression applied"
        );
        assert_eq!(
            CompressionDecisionReason::ResponseTooSmall.description(),
            "response too small"
        );
    }
}
