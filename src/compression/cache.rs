//! Cache integration for compression (Phase 40.4)
//!
//! This module implements:
//! - Vary header handling for Accept-Encoding
//! - Cache key generation with compression algorithm
//! - Compressed variant caching

use super::algorithms::Compression;

/// Adds or updates Vary header to include Accept-Encoding
///
/// # Arguments
/// * `existing_vary` - Existing Vary header value (optional)
///
/// # Returns
/// * String with updated Vary header value
pub fn add_vary_accept_encoding(existing_vary: Option<&str>) -> String {
    match existing_vary {
        None => "Accept-Encoding".to_string(),
        Some(vary) => {
            let vary_lower = vary.to_lowercase();
            if vary_lower.contains("accept-encoding") {
                // Already has Accept-Encoding, return as-is
                vary.to_string()
            } else {
                // Append Accept-Encoding to existing Vary values
                format!("{}, Accept-Encoding", vary)
            }
        }
    }
}

/// Generates cache key suffix based on compression algorithm
///
/// # Arguments
/// * `algorithm` - Compression algorithm (or None for uncompressed)
///
/// # Returns
/// * String suffix to append to cache key
pub fn cache_key_suffix(algorithm: Option<Compression>) -> String {
    match algorithm {
        None => ":uncompressed".to_string(),
        Some(algo) => format!(":compressed:{}", algo),
    }
}

/// Generates full cache key for compressed response
///
/// # Arguments
/// * `base_key` - Original cache key (e.g., path + query)
/// * `algorithm` - Compression algorithm used
///
/// # Returns
/// * Full cache key including compression variant
pub fn generate_cache_key(base_key: &str, algorithm: Option<Compression>) -> String {
    format!("{}{}", base_key, cache_key_suffix(algorithm))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_vary_accept_encoding_none() {
        let result = add_vary_accept_encoding(None);
        assert_eq!(result, "Accept-Encoding");
    }

    #[test]
    fn test_add_vary_accept_encoding_already_present() {
        let result = add_vary_accept_encoding(Some("Accept-Encoding"));
        assert_eq!(result, "Accept-Encoding");
    }

    #[test]
    fn test_add_vary_accept_encoding_case_insensitive() {
        let result = add_vary_accept_encoding(Some("accept-encoding"));
        assert_eq!(result, "accept-encoding");
    }

    #[test]
    fn test_add_vary_accept_encoding_with_other_values() {
        let result = add_vary_accept_encoding(Some("User-Agent"));
        assert_eq!(result, "User-Agent, Accept-Encoding");
    }

    #[test]
    fn test_add_vary_accept_encoding_multiple_values() {
        let result = add_vary_accept_encoding(Some("User-Agent, Cookie"));
        assert_eq!(result, "User-Agent, Cookie, Accept-Encoding");
    }

    #[test]
    fn test_cache_key_suffix_uncompressed() {
        let result = cache_key_suffix(None);
        assert_eq!(result, ":uncompressed");
    }

    #[test]
    fn test_cache_key_suffix_gzip() {
        let result = cache_key_suffix(Some(Compression::Gzip));
        assert_eq!(result, ":compressed:gzip");
    }

    #[test]
    fn test_cache_key_suffix_brotli() {
        let result = cache_key_suffix(Some(Compression::Brotli));
        assert_eq!(result, ":compressed:br");
    }

    #[test]
    fn test_cache_key_suffix_deflate() {
        let result = cache_key_suffix(Some(Compression::Deflate));
        assert_eq!(result, ":compressed:deflate");
    }

    #[test]
    fn test_generate_cache_key_uncompressed() {
        let result = generate_cache_key("/api/data", None);
        assert_eq!(result, "/api/data:uncompressed");
    }

    #[test]
    fn test_generate_cache_key_gzip() {
        let result = generate_cache_key("/api/data", Some(Compression::Gzip));
        assert_eq!(result, "/api/data:compressed:gzip");
    }

    #[test]
    fn test_generate_cache_key_brotli() {
        let result = generate_cache_key("/api/data", Some(Compression::Brotli));
        assert_eq!(result, "/api/data:compressed:br");
    }

    #[test]
    fn test_generate_cache_key_different_algorithms() {
        let base = "/api/data";
        let gzip_key = generate_cache_key(base, Some(Compression::Gzip));
        let brotli_key = generate_cache_key(base, Some(Compression::Brotli));
        let uncompressed_key = generate_cache_key(base, None);

        // All should be different
        assert_ne!(gzip_key, brotli_key);
        assert_ne!(gzip_key, uncompressed_key);
        assert_ne!(brotli_key, uncompressed_key);
    }
}
