//! Response compression middleware (Phase 40.2)
//!
//! This module implements:
//! - Response body compression (gzip, brotli, deflate)
//! - Content-Encoding header injection
//! - Size threshold checking
//! - Content-type filtering

use super::algorithms::Compression;
use super::compress::compress;
use super::config::CompressionConfig;
use super::error::CompressionError;

/// Determines if a response should be compressed based on content type
///
/// Compressible types: text/*, application/json, application/xml, application/javascript
/// Non-compressible: image/*, video/*, audio/*, application/octet-stream
pub fn is_compressible_content_type(content_type: Option<&str>) -> bool {
    match content_type {
        None => false,
        Some(ct) => {
            let ct_lower = ct.to_lowercase();
            // Compressible types
            ct_lower.starts_with("text/")
                || ct_lower.contains("json")
                || ct_lower.contains("xml")
                || ct_lower.contains("javascript")
                || ct_lower.contains("svg")
                || ct_lower.contains("wasm")
        }
    }
}

/// Determines if response should be compressed based on size thresholds
pub fn should_compress_by_size(content_length: Option<usize>, config: &CompressionConfig) -> bool {
    match content_length {
        None => false, // Unknown size, don't compress
        Some(size) => {
            size >= config.min_response_size_bytes && size <= config.max_response_size_bytes
        }
    }
}

/// Determines if response is already compressed
pub fn is_already_compressed(content_encoding: Option<&str>) -> bool {
    content_encoding.is_some()
}

/// Compress response body if appropriate
///
/// Returns:
/// - Ok((compressed_data, algorithm)) if compression was applied
/// - Err(CompressionError) if compression failed
pub fn compress_response(
    body: &[u8],
    algorithm: Compression,
    config: &CompressionConfig,
) -> Result<Vec<u8>, CompressionError> {
    let algo_config = config.get_algorithm(algorithm).ok_or_else(|| {
        CompressionError::InvalidConfig(format!("algorithm {} not configured", algorithm))
    })?;

    compress(body, algorithm, algo_config.level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_compressible_content_type_text() {
        assert!(is_compressible_content_type(Some("text/html")));
        assert!(is_compressible_content_type(Some("text/plain")));
        assert!(is_compressible_content_type(Some("text/css")));
    }

    #[test]
    fn test_is_compressible_content_type_json() {
        assert!(is_compressible_content_type(Some("application/json")));
    }

    #[test]
    fn test_is_compressible_content_type_xml() {
        assert!(is_compressible_content_type(Some("application/xml")));
    }

    #[test]
    fn test_is_compressible_content_type_javascript() {
        assert!(is_compressible_content_type(Some("application/javascript")));
        assert!(is_compressible_content_type(Some("text/javascript")));
    }

    #[test]
    fn test_is_compressible_content_type_image() {
        assert!(!is_compressible_content_type(Some("image/png")));
        assert!(!is_compressible_content_type(Some("image/jpeg")));
    }

    #[test]
    fn test_is_compressible_content_type_video() {
        assert!(!is_compressible_content_type(Some("video/mp4")));
    }

    #[test]
    fn test_is_compressible_content_type_none() {
        assert!(!is_compressible_content_type(None));
    }

    #[test]
    fn test_should_compress_by_size_within_range() {
        let config = CompressionConfig::new();
        assert!(should_compress_by_size(Some(10000), &config));
    }

    #[test]
    fn test_should_compress_by_size_too_small() {
        let config = CompressionConfig::new();
        assert!(!should_compress_by_size(Some(100), &config));
    }

    #[test]
    fn test_should_compress_by_size_too_large() {
        let config = CompressionConfig::new();
        assert!(!should_compress_by_size(Some(200_000_000), &config));
    }

    #[test]
    fn test_should_compress_by_size_unknown() {
        let config = CompressionConfig::new();
        assert!(!should_compress_by_size(None, &config));
    }

    #[test]
    fn test_is_already_compressed_yes() {
        assert!(is_already_compressed(Some("gzip")));
    }

    #[test]
    fn test_is_already_compressed_no() {
        assert!(!is_already_compressed(None));
    }
}
