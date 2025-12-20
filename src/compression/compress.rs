//! Core compression and decompression functions
use std::io::{Read, Write};

use super::algorithms::Compression;
use super::error::CompressionError;

/// Compress data using the specified algorithm
///
/// # Arguments
/// * `data` - Input data to compress
/// * `algorithm` - Compression algorithm to use
/// * `level` - Compression level (1-11, meaning varies by algorithm)
///
/// # Returns
/// * `Ok(Vec<u8>)` - Compressed data
/// * `Err(CompressionError)` - Compression failed
pub fn compress(
    data: &[u8],
    algorithm: Compression,
    level: u32,
) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
        Compression::Gzip => compress_gzip(data, level),
        Compression::Brotli => compress_brotli(data, level),
        Compression::Deflate => compress_deflate(data, level),
    }
}

/// Decompress data using the specified algorithm
///
/// # Arguments
/// * `data` - Compressed data
/// * `algorithm` - Compression algorithm used
///
/// # Returns
/// * `Ok(Vec<u8>)` - Decompressed data
/// * `Err(CompressionError)` - Decompression failed
pub fn decompress(data: &[u8], algorithm: Compression) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
        Compression::Gzip => decompress_gzip(data),
        Compression::Brotli => decompress_brotli(data),
        Compression::Deflate => decompress_deflate(data),
    }
}

/// Compress data using gzip
fn compress_gzip(data: &[u8], level: u32) -> Result<Vec<u8>, CompressionError> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(level));
    encoder
        .write_all(data)
        .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
}

/// Decompress gzip data
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
    Ok(result)
}

/// Compress data using brotli
fn compress_brotli(data: &[u8], level: u32) -> Result<Vec<u8>, CompressionError> {
    let mut result = Vec::new();
    let quality = level.clamp(0, 11) as i32;

    // Use brotli's standard compression with quality parameter
    let mut input = std::io::Cursor::new(data);
    brotli::BrotliCompress(
        &mut input,
        &mut result,
        &brotli::enc::BrotliEncoderParams {
            quality,
            ..Default::default()
        },
    )
    .map_err(|_| CompressionError::CompressionFailed("brotli compression failed".to_string()))?;
    Ok(result)
}

/// Decompress brotli data
fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut result = Vec::new();
    let mut decompressor = brotli::Decompressor::new(data, 4096);
    decompressor
        .read_to_end(&mut result)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
    Ok(result)
}

/// Compress data using deflate
fn compress_deflate(data: &[u8], level: u32) -> Result<Vec<u8>, CompressionError> {
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
    encoder
        .write_all(data)
        .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
}

/// Decompress deflate data
fn decompress_deflate(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut decoder = flate2::read::DeflateDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> Vec<u8> {
        // Create repetitive data that compresses well
        let mut data = Vec::new();
        for _ in 0..100 {
            data.extend_from_slice(b"Hello, World! This is test data for compression. ");
        }
        data
    }

    #[test]
    fn test_gzip_compress_decompress() {
        let data = create_test_data();
        let compressed = compress_gzip(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_deflate_compress_decompress() {
        let data = create_test_data();
        let compressed = compress_deflate(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = decompress_deflate(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_brotli_compress_decompress() {
        let data = create_test_data();
        let compressed = compress_brotli(&data, 4).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = decompress_brotli(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
