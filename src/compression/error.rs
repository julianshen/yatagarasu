/// Compression error types
use std::fmt;

/// Errors that can occur during compression/decompression operations
#[derive(Debug, Clone)]
pub enum CompressionError {
    /// Invalid or unsupported compression algorithm
    InvalidAlgorithm(String),
    /// Compression operation failed
    CompressionFailed(String),
    /// Decompression operation failed
    DecompressionFailed(String),
    /// Invalid compression configuration
    InvalidConfig(String),
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressionError::InvalidAlgorithm(msg) => {
                write!(f, "Invalid compression algorithm: {}", msg)
            }
            CompressionError::CompressionFailed(msg) => {
                write!(f, "Compression failed: {}", msg)
            }
            CompressionError::DecompressionFailed(msg) => {
                write!(f, "Decompression failed: {}", msg)
            }
            CompressionError::InvalidConfig(msg) => {
                write!(f, "Invalid compression configuration: {}", msg)
            }
        }
    }
}

impl std::error::Error for CompressionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_algorithm_error_display() {
        let err = CompressionError::InvalidAlgorithm("unknown".to_string());
        assert_eq!(err.to_string(), "Invalid compression algorithm: unknown");
    }

    #[test]
    fn test_compression_failed_error_display() {
        let err = CompressionError::CompressionFailed("buffer too small".to_string());
        assert_eq!(err.to_string(), "Compression failed: buffer too small");
    }

    #[test]
    fn test_decompression_failed_error_display() {
        let err = CompressionError::DecompressionFailed("corrupted data".to_string());
        assert_eq!(err.to_string(), "Decompression failed: corrupted data");
    }

    #[test]
    fn test_invalid_config_error_display() {
        let err = CompressionError::InvalidConfig("level out of range".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid compression configuration: level out of range"
        );
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CompressionError>();
    }
}
