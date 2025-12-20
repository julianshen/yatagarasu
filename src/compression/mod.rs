//! Compression module for Yatagarasu S3 proxy
//!
//! This module provides request/response compression support with:
//! - Multiple algorithms (gzip, brotli, deflate)
//! - Accept-Encoding negotiation
//! - Configurable compression levels and thresholds
//! - Cache integration with Vary header support
//!
//! # Module Organization
//!
//! - [`algorithms`] - Compression algorithm definitions
//! - [`config`] - Configuration structures
//! - [`error`] - Error types
//! - [`negotiation`] - Accept-Encoding header parsing and algorithm selection
//! - [`response`] - Response compression middleware
//! - [`request`] - Request decompression middleware
//! - [`cache`] - Cache integration
//! - [`metrics`] - Compression metrics

pub mod algorithms;
pub mod bucket_config;
pub mod cache;
pub mod compress;
pub mod config;
pub mod error;
pub mod negotiation;
pub mod request;
pub mod response;

// Re-export public types
pub use algorithms::{AlgorithmConfig, Compression};
pub use bucket_config::BucketCompressionConfig;
pub use cache::{add_vary_accept_encoding, cache_key_suffix, generate_cache_key};
pub use compress::{compress, decompress};
pub use config::CompressionConfig;
pub use error::CompressionError;
pub use negotiation::negotiate_compression;
pub use request::{decompress_request_body, parse_content_encoding};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_module_exports() {
        // Verify key types are exported
        let _: Compression = Compression::Gzip;
        let _: CompressionConfig = CompressionConfig::new();
        let _: CompressionError = CompressionError::InvalidAlgorithm("test".to_string());
    }
}
