//! Request decompression middleware (Phase 40.3)
//!
//! This module implements:
//! - Request body decompression (gzip, brotli, deflate)
//! - Content-Encoding header parsing
//! - Error handling for invalid compressed data
//! - Protection against decompression bombs with size limits

use super::algorithms::Compression;
use super::compress::decompress;
use super::error::CompressionError;

/// Default maximum decompressed size (100MB) to prevent decompression bombs
pub const DEFAULT_MAX_DECOMPRESSED_SIZE: usize = 100 * 1024 * 1024;

/// Parse Content-Encoding header to determine compression algorithm
///
/// # Arguments
/// * `content_encoding` - Value of Content-Encoding header (e.g., "gzip", "br", "deflate")
///
/// # Returns
/// * `Ok(Compression)` - Parsed compression algorithm
/// * `Err(CompressionError)` - Unsupported or invalid encoding
pub fn parse_content_encoding(content_encoding: &str) -> Result<Compression, CompressionError> {
    let encoding = content_encoding.trim().to_lowercase();
    Compression::parse_algorithm(&encoding)
}

/// Decompress request body if Content-Encoding header is present
///
/// # Arguments
/// * `body` - Request body data
/// * `content_encoding` - Value of Content-Encoding header (optional)
///
/// # Returns
/// * `Ok(Vec<u8>)` - Decompressed body (or original if no encoding)
/// * `Err(CompressionError)` - Decompression failed or unsupported encoding
///
/// # Security
/// Uses DEFAULT_MAX_DECOMPRESSED_SIZE to prevent decompression bombs.
/// For custom limits, use `decompress_request_body_with_limit`.
pub fn decompress_request_body(
    body: &[u8],
    content_encoding: Option<&str>,
) -> Result<Vec<u8>, CompressionError> {
    decompress_request_body_with_limit(body, content_encoding, DEFAULT_MAX_DECOMPRESSED_SIZE)
}

/// Decompress request body with a custom size limit
///
/// # Arguments
/// * `body` - Request body data
/// * `content_encoding` - Value of Content-Encoding header (optional)
/// * `max_size` - Maximum allowed decompressed size in bytes
///
/// # Returns
/// * `Ok(Vec<u8>)` - Decompressed body (or original if no encoding)
/// * `Err(CompressionError)` - Decompression failed, unsupported encoding, or size limit exceeded
pub fn decompress_request_body_with_limit(
    body: &[u8],
    content_encoding: Option<&str>,
    max_size: usize,
) -> Result<Vec<u8>, CompressionError> {
    match content_encoding {
        None => Ok(body.to_vec()), // No compression
        Some(encoding) => {
            let algorithm = parse_content_encoding(encoding)?;
            let decompressed = decompress(body, algorithm, max_size)?;
            Ok(decompressed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_encoding_gzip() {
        let result = parse_content_encoding("gzip");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "gzip");
    }

    #[test]
    fn test_parse_content_encoding_brotli() {
        let result = parse_content_encoding("br");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "br");
    }

    #[test]
    fn test_parse_content_encoding_deflate() {
        let result = parse_content_encoding("deflate");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "deflate");
    }

    #[test]
    fn test_parse_content_encoding_case_insensitive() {
        assert!(parse_content_encoding("GZIP").is_ok());
        assert!(parse_content_encoding("Gzip").is_ok());
        assert!(parse_content_encoding("BR").is_ok());
        assert!(parse_content_encoding("Br").is_ok());
    }

    #[test]
    fn test_parse_content_encoding_with_whitespace() {
        assert!(parse_content_encoding("  gzip  ").is_ok());
        assert!(parse_content_encoding("\tbr\t").is_ok());
    }

    #[test]
    fn test_parse_content_encoding_invalid() {
        let result = parse_content_encoding("invalid");
        assert!(result.is_err());
        match result {
            Err(CompressionError::InvalidAlgorithm(_)) => (),
            _ => panic!("Expected InvalidAlgorithm error"),
        }
    }

    #[test]
    fn test_decompress_request_body_no_encoding() {
        let body = b"Hello, World!";
        let result = decompress_request_body(body, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), body);
    }

    #[test]
    fn test_decompress_request_body_gzip() {
        // Create test data
        let original = b"Hello, World! This is test data for decompression.";
        let mut compressed = Vec::new();
        {
            use std::io::Write;
            let mut encoder =
                flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::default());
            encoder.write_all(original).unwrap();
            encoder.finish().unwrap();
        }

        let result = decompress_request_body(&compressed, Some("gzip"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), original);
    }

    #[test]
    fn test_decompress_request_body_invalid_data() {
        let invalid_data = b"not compressed data";
        let result = decompress_request_body(invalid_data, Some("gzip"));
        assert!(result.is_err());
        match result {
            Err(CompressionError::DecompressionFailed(_)) => (),
            _ => panic!("Expected DecompressionFailed error"),
        }
    }

    #[test]
    fn test_decompress_request_body_unsupported_encoding() {
        let body = b"some data";
        let result = decompress_request_body(body, Some("unknown"));
        assert!(result.is_err());
        match result {
            Err(CompressionError::InvalidAlgorithm(_)) => (),
            _ => panic!("Expected InvalidAlgorithm error"),
        }
    }

    #[test]
    fn test_decompress_request_body_with_limit_exceeds() {
        // Create test data that decompresses to more than the limit
        let original = vec![b'A'; 1000]; // 1000 bytes
        let mut compressed = Vec::new();
        {
            use std::io::Write;
            let mut encoder =
                flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::default());
            encoder.write_all(&original).unwrap();
            encoder.finish().unwrap();
        }

        // Set limit to 500 bytes - should fail
        let result = decompress_request_body_with_limit(&compressed, Some("gzip"), 500);
        assert!(result.is_err());
        match result {
            Err(CompressionError::DecompressionFailed(msg)) => {
                assert!(msg.contains("exceeds maximum allowed size"));
            }
            _ => panic!("Expected DecompressionFailed error for size limit"),
        }
    }

    #[test]
    fn test_decompress_request_body_with_limit_within() {
        // Create test data that decompresses within the limit
        let original = vec![b'A'; 500]; // 500 bytes
        let mut compressed = Vec::new();
        {
            use std::io::Write;
            let mut encoder =
                flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::default());
            encoder.write_all(&original).unwrap();
            encoder.finish().unwrap();
        }

        // Set limit to 1000 bytes - should succeed
        let result = decompress_request_body_with_limit(&compressed, Some("gzip"), 1000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), original);
    }

    #[test]
    fn test_default_max_decompressed_size() {
        // Verify the default constant is 100MB
        assert_eq!(DEFAULT_MAX_DECOMPRESSED_SIZE, 100 * 1024 * 1024);
    }
}
