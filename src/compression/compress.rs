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
/// * `max_size` - Maximum allowed decompressed size in bytes
///
/// # Returns
/// * `Ok(Vec<u8>)` - Decompressed data
/// * `Err(CompressionError)` - Decompression failed or size limit exceeded
pub fn decompress(
    data: &[u8],
    algorithm: Compression,
    max_size: usize,
) -> Result<Vec<u8>, CompressionError> {
    match algorithm {
        Compression::Gzip => decompress_gzip(data, max_size),
        Compression::Brotli => decompress_brotli(data, max_size),
        Compression::Deflate => decompress_deflate(data, max_size),
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
fn decompress_gzip(data: &[u8], max_size: usize) -> Result<Vec<u8>, CompressionError> {
    let decoder = flate2::read::GzDecoder::new(data);
    let mut reader = decoder.take((max_size + 1) as u64);
    let mut result = Vec::new();
    reader
        .read_to_end(&mut result)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;

    if result.len() > max_size {
        return Err(CompressionError::DecompressionFailed(format!(
            "decompressed size exceeds maximum allowed size {}",
            max_size
        )));
    }
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
fn decompress_brotli(data: &[u8], max_size: usize) -> Result<Vec<u8>, CompressionError> {
    let mut result = Vec::new();
    let decompressor = brotli::Decompressor::new(data, 4096);
    let mut reader = decompressor.take((max_size + 1) as u64);
    reader
        .read_to_end(&mut result)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;

    if result.len() > max_size {
        return Err(CompressionError::DecompressionFailed(format!(
            "decompressed size exceeds maximum allowed size {}",
            max_size
        )));
    }
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
fn decompress_deflate(data: &[u8], max_size: usize) -> Result<Vec<u8>, CompressionError> {
    let decoder = flate2::read::DeflateDecoder::new(data);
    let mut reader = decoder.take((max_size + 1) as u64);
    let mut result = Vec::new();
    reader
        .read_to_end(&mut result)
        .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;

    if result.len() > max_size {
        return Err(CompressionError::DecompressionFailed(format!(
            "decompressed size exceeds maximum allowed size {}",
            max_size
        )));
    }
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
        let decompressed = decompress_gzip(&compressed, 1024 * 1024).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_deflate_compress_decompress() {
        let data = create_test_data();
        let compressed = compress_deflate(&data, 6).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = decompress_deflate(&compressed, 1024 * 1024).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_brotli_compress_decompress() {
        let data = create_test_data();
        let compressed = compress_brotli(&data, 4).unwrap();
        assert!(compressed.len() < data.len());
        let decompressed = decompress_brotli(&compressed, 1024 * 1024).unwrap();
        assert_eq!(decompressed, data);
    }
}
