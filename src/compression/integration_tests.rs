//! Integration tests for compression (Phase 40.7)
//!
//! This module implements:
//! - End-to-end compression scenarios
//! - Compression with caching
//! - Compression with different content types
//! - Compression reversibility tests

use super::algorithms::Compression;
use super::cache::generate_cache_key;
use super::compress::{compress, decompress};
use super::metrics::CompressionMetrics;
use std::time::Duration;

/// Test data generator for different content types
pub fn generate_test_data(content_type: &str, size: usize) -> Vec<u8> {
    match content_type {
        "text/plain" => {
            // Highly repetitive text (compresses well)
            let base = "The quick brown fox jumps over the lazy dog. ";
            let mut data = Vec::new();
            while data.len() < size {
                data.extend_from_slice(base.as_bytes());
            }
            data.truncate(size);
            data
        }
        "application/json" => {
            // JSON with repetitive structure
            let base = r#"{"id":1,"name":"test","value":42,"active":true},"#;
            let mut data = Vec::new();
            while data.len() < size {
                data.extend_from_slice(base.as_bytes());
            }
            data.truncate(size);
            data
        }
        "image/png" => {
            // Binary data (doesn't compress well)
            (0..size).map(|i| (i % 256) as u8).collect()
        }
        _ => {
            // Default: random-like data
            (0..size).map(|i| ((i * 7) % 256) as u8).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bucket_config::BucketCompressionConfig;
    use super::super::config::CompressionConfig;
    use super::*;

    #[test]
    fn test_compression_reversibility_gzip() {
        let original = generate_test_data("text/plain", 10000);
        let compressed = compress(&original, Compression::Gzip, 6).unwrap();
        let decompressed = decompress(&compressed, Compression::Gzip).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compression_reversibility_brotli() {
        let original = generate_test_data("text/plain", 10000);
        let compressed = compress(&original, Compression::Brotli, 6).unwrap();
        let decompressed = decompress(&compressed, Compression::Brotli).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compression_reversibility_deflate() {
        let original = generate_test_data("text/plain", 10000);
        let compressed = compress(&original, Compression::Deflate, 6).unwrap();
        let decompressed = decompress(&compressed, Compression::Deflate).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compression_ratio_text_vs_binary() {
        let text_data = generate_test_data("text/plain", 10000);
        let binary_data = generate_test_data("image/png", 10000);

        let text_compressed = compress(&text_data, Compression::Gzip, 6).unwrap();
        let binary_compressed = compress(&binary_data, Compression::Gzip, 6).unwrap();

        let text_ratio = text_compressed.len() as f64 / text_data.len() as f64;
        let binary_ratio = binary_compressed.len() as f64 / binary_data.len() as f64;

        // Text should compress much better than binary
        assert!(text_ratio < binary_ratio);
    }

    #[test]
    fn test_compression_with_cache_key() {
        let base_key = "s3://bucket/file.txt";
        let gzip_key = generate_cache_key(base_key, Some(Compression::Gzip));
        let brotli_key = generate_cache_key(base_key, Some(Compression::Brotli));

        // Different algorithms should have different cache keys
        assert_ne!(gzip_key, brotli_key);
        assert!(gzip_key.contains("gzip"));
        assert!(brotli_key.contains("br"));
    }

    #[test]
    fn test_compression_metrics_calculation() {
        let original_size = 10000;
        let compressed_size = 3000;
        let metrics = CompressionMetrics::new(
            original_size,
            compressed_size,
            Compression::Gzip,
            Duration::from_millis(50),
        );

        assert_eq!(metrics.compression_ratio(), 0.3);
        assert_eq!(metrics.bytes_saved(), 7000);
        assert_eq!(metrics.percentage_saved(), 70.0);
        assert!(metrics.throughput_bytes_per_sec() > 0.0);
    }

    #[test]
    fn test_compression_config_with_bucket_override() {
        let mut global_config = CompressionConfig::new();
        global_config.enabled = true;
        global_config.default_algorithm = "gzip".to_string();
        global_config.compression_level = 6;

        // Bucket config overrides global
        let bucket_config = BucketCompressionConfig {
            enabled: Some(false),
            default_algorithm: Some("brotli".to_string()),
            compression_level: Some(9),
            min_response_size_bytes: Some(2048),
            max_response_size_bytes: Some(50_000_000),
            algorithms: None,
        };

        // Verify bucket overrides
        assert_eq!(bucket_config.is_enabled(), Some(false));
        assert_eq!(bucket_config.get_compression_level(), Some(9));
    }

    #[test]
    fn test_compression_empty_data() {
        let empty = vec![];
        let compressed = compress(&empty, Compression::Gzip, 6).unwrap();
        let decompressed = decompress(&compressed, Compression::Gzip).unwrap();
        assert_eq!(decompressed, empty);
    }

    #[test]
    fn test_compression_single_byte() {
        let single = vec![42u8];
        let compressed = compress(&single, Compression::Gzip, 6).unwrap();
        let decompressed = decompress(&compressed, Compression::Gzip).unwrap();
        assert_eq!(decompressed, single);
    }

    #[test]
    fn test_compression_large_data() {
        let large = generate_test_data("text/plain", 1_000_000);
        let compressed = compress(&large, Compression::Gzip, 6).unwrap();
        let decompressed = decompress(&compressed, Compression::Gzip).unwrap();
        assert_eq!(decompressed, large);
    }
}
