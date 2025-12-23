//! Image optimization security module
//!
//! Provides:
//! - URL signing with HMAC-SHA256
//! - Image bomb protection (dimension validation)
//! - Source URL validation

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::error::ImageError;

type HmacSha256 = Hmac<Sha256>;

/// Security configuration for image optimization
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Whether URL signing is required
    pub signing_required: bool,
    /// HMAC signing key (base64 or hex encoded)
    pub signing_key: Option<Vec<u8>>,
    /// Optional salt to add to signatures
    pub signing_salt: Option<Vec<u8>>,
    /// Maximum allowed source image width
    pub max_source_width: u32,
    /// Maximum allowed source image height
    pub max_source_height: u32,
    /// Maximum allowed total pixels (width * height)
    pub max_source_pixels: u64,
    /// Maximum source file size in bytes
    pub max_source_file_size: usize,
    /// Allowed source URL patterns (glob-style)
    pub allowed_sources: Vec<String>,
    /// Blocked source URL patterns
    pub blocked_sources: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            signing_required: false,
            signing_key: None,
            signing_salt: None,
            max_source_width: 10000,
            max_source_height: 10000,
            max_source_pixels: 100_000_000,         // 100 megapixels
            max_source_file_size: 50 * 1024 * 1024, // 50MB
            allowed_sources: Vec::new(),
            blocked_sources: Vec::new(),
        }
    }
}

impl SecurityConfig {
    /// Create a new security config with signing enabled
    pub fn with_signing(key: impl Into<Vec<u8>>) -> Self {
        Self {
            signing_required: true,
            signing_key: Some(key.into()),
            ..Default::default()
        }
    }
}

/// Generate a URL signature for the given options and source URL
///
/// The signature is computed as:
/// ```text
/// signature = base64url(HMAC-SHA256(key, salt + options + "/" + source_url))
/// ```
///
/// # Arguments
/// * `options` - The transformation options string (e.g., "w:800,h:600")
/// * `source_url` - The source image URL or path
/// * `config` - Security configuration containing the signing key
///
/// # Returns
/// * Base64url-encoded signature string
pub fn generate_signature(
    options: &str,
    source_url: &str,
    config: &SecurityConfig,
) -> Option<String> {
    let key = config.signing_key.as_ref()?;

    let signature =
        compute_hmac_signature(key, config.signing_salt.as_deref(), options, source_url);

    Some(base64_url_encode(&signature))
}

/// Validate a URL signature
///
/// # Arguments
/// * `signature` - The signature from the URL
/// * `options` - The transformation options string
/// * `source_url` - The source image URL or path
/// * `config` - Security configuration
///
/// # Returns
/// * `Ok(())` if signature is valid or signing not required
/// * `Err(ImageError::InvalidSignature)` if signature is invalid
pub fn validate_signature(
    signature: &str,
    options: &str,
    source_url: &str,
    config: &SecurityConfig,
) -> Result<(), ImageError> {
    if !config.signing_required {
        return Ok(());
    }

    let expected =
        generate_signature(options, source_url, config).ok_or(ImageError::InvalidSignature)?;

    // Use constant-time comparison to prevent timing attacks
    if constant_time_compare(signature, &expected) {
        Ok(())
    } else {
        Err(ImageError::InvalidSignature)
    }
}

/// Compute HMAC-SHA256 signature
///
/// Combines salt (if any), options, and source URL into a single message,
/// then computes the HMAC-SHA256.
fn compute_hmac_signature(
    key: &[u8],
    salt: Option<&[u8]>,
    options: &str,
    source_url: &str,
) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");

    // Add salt if present
    if let Some(salt) = salt {
        mac.update(salt);
    }

    // Add options and source URL
    mac.update(options.as_bytes());
    mac.update(b"/");
    mac.update(source_url.as_bytes());

    mac.finalize().into_bytes().to_vec()
}

/// Base64url encode (URL-safe, no padding)
fn base64_url_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// Validate image dimensions against security limits
///
/// This should be called BEFORE fully decoding the image to protect
/// against "image bomb" attacks where a small file decompresses to
/// huge dimensions.
pub fn validate_dimensions(
    width: u32,
    height: u32,
    config: &SecurityConfig,
) -> Result<(), ImageError> {
    // Check individual dimensions
    if width > config.max_source_width {
        return Err(ImageError::ImageBombDetected {
            width,
            height,
            pixels: width as u64 * height as u64,
            max_pixels: config.max_source_pixels,
        });
    }

    if height > config.max_source_height {
        return Err(ImageError::ImageBombDetected {
            width,
            height,
            pixels: width as u64 * height as u64,
            max_pixels: config.max_source_pixels,
        });
    }

    // Check total pixels
    let pixels = width as u64 * height as u64;
    if pixels > config.max_source_pixels {
        return Err(ImageError::ImageBombDetected {
            width,
            height,
            pixels,
            max_pixels: config.max_source_pixels,
        });
    }

    Ok(())
}

/// Validate source file size
pub fn validate_file_size(size: usize, config: &SecurityConfig) -> Result<(), ImageError> {
    if size > config.max_source_file_size {
        return Err(ImageError::FileTooLarge {
            size,
            max_size: config.max_source_file_size,
        });
    }
    Ok(())
}

/// Validate source URL against allowed/blocked lists
pub fn validate_source(source: &str, config: &SecurityConfig) -> Result<(), ImageError> {
    // If no allowed sources configured, allow all (unless blocked)
    let allowed = if config.allowed_sources.is_empty() {
        true
    } else {
        config
            .allowed_sources
            .iter()
            .any(|pattern| glob_match(pattern, source))
    };

    if !allowed {
        return Err(ImageError::SourceNotAllowed {
            source: source.to_string(),
        });
    }

    // Check blocked sources
    let blocked = config
        .blocked_sources
        .iter()
        .any(|pattern| glob_match(pattern, source));

    if blocked {
        return Err(ImageError::SourceNotAllowed {
            source: source.to_string(),
        });
    }

    Ok(())
}

/// Simple glob pattern matching
///
/// Supports:
/// - `*` or `**` alone matches everything
/// - `*suffix` matches text ending with suffix (e.g., `*.example.com`)
/// - `prefix*` matches text starting with prefix (e.g., `https://cdn.*`)
/// - Exact match for patterns without wildcards
///
/// Note: Does not support wildcards in the middle of patterns (e.g., `cdn.*.com`)
fn glob_match(pattern: &str, text: &str) -> bool {
    // Handle ** (match any path)
    if pattern == "**" || pattern == "*" {
        return true;
    }

    // Simple prefix/suffix matching
    if let Some(suffix) = pattern.strip_prefix('*') {
        return text.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return text.starts_with(prefix);
    }

    // Exact match
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_signature() {
        let config = SecurityConfig::with_signing(b"secret-key".to_vec());
        let sig = generate_signature("w:800,h:600", "bucket/image.jpg", &config);
        assert!(sig.is_some());
        assert!(!sig.unwrap().is_empty());
    }

    #[test]
    fn test_validate_signature_success() {
        let config = SecurityConfig::with_signing(b"secret-key".to_vec());
        let sig = generate_signature("w:800", "image.jpg", &config).unwrap();

        let result = validate_signature(&sig, "w:800", "image.jpg", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_signature_failure() {
        let config = SecurityConfig::with_signing(b"secret-key".to_vec());

        let result = validate_signature("invalid-sig", "w:800", "image.jpg", &config);
        assert!(matches!(result, Err(ImageError::InvalidSignature)));
    }

    #[test]
    fn test_validate_signature_not_required() {
        let config = SecurityConfig::default();
        let result = validate_signature("any", "w:800", "image.jpg", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dimensions_ok() {
        let config = SecurityConfig::default();
        let result = validate_dimensions(1000, 1000, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dimensions_width_exceeded() {
        let config = SecurityConfig {
            max_source_width: 5000,
            ..Default::default()
        };
        let result = validate_dimensions(10000, 1000, &config);
        assert!(matches!(result, Err(ImageError::ImageBombDetected { .. })));
    }

    #[test]
    fn test_validate_dimensions_pixels_exceeded() {
        let config = SecurityConfig {
            max_source_pixels: 1_000_000,
            ..Default::default()
        };
        let result = validate_dimensions(2000, 2000, &config); // 4M pixels
        assert!(matches!(result, Err(ImageError::ImageBombDetected { .. })));
    }

    #[test]
    fn test_validate_file_size_ok() {
        let config = SecurityConfig::default();
        let result = validate_file_size(1_000_000, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_size_exceeded() {
        let config = SecurityConfig {
            max_source_file_size: 1_000_000,
            ..Default::default()
        };
        let result = validate_file_size(10_000_000, &config);
        assert!(matches!(result, Err(ImageError::FileTooLarge { .. })));
    }

    #[test]
    fn test_validate_source_allowed() {
        let config = SecurityConfig {
            allowed_sources: vec!["bucket/*".to_string()],
            ..Default::default()
        };
        let result = validate_source("bucket/image.jpg", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_source_not_allowed() {
        let config = SecurityConfig {
            allowed_sources: vec!["bucket/*".to_string()],
            ..Default::default()
        };
        let result = validate_source("other/image.jpg", &config);
        assert!(matches!(result, Err(ImageError::SourceNotAllowed { .. })));
    }

    #[test]
    fn test_validate_source_blocked() {
        let config = SecurityConfig {
            blocked_sources: vec!["internal.*".to_string()],
            ..Default::default()
        };
        let result = validate_source("internal.example.com/image.jpg", &config);
        assert!(matches!(result, Err(ImageError::SourceNotAllowed { .. })));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("**", "anything"));
        assert!(glob_match("*.jpg", "image.jpg"));
        assert!(glob_match("bucket/*", "bucket/image.jpg"));
        assert!(!glob_match("*.png", "image.jpg"));
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
    }
}
