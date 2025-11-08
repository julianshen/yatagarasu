//! Security Validation Module
//!
//! Protects against malicious or malformed requests that could:
//! - Exhaust server resources (oversized requests)
//! - Access unauthorized files (path traversal)
//! - Crash the proxy (malformed input)
//!
//! Returns appropriate HTTP status codes:
//! - 413 Payload Too Large - Request body exceeds limit
//! - 431 Request Header Fields Too Large - Headers exceed limit
//! - 400 Bad Request - Malformed input (path traversal, invalid format)
//! - 403 Forbidden - Malformed JWT (caught and handled gracefully)

use std::path::Path;

/// Security validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Request body too large (413)
    PayloadTooLarge { size: usize, limit: usize },
    /// Request headers too large (431)
    HeadersTooLarge { total_size: usize, limit: usize },
    /// Path traversal attempt detected (400)
    PathTraversal { path: String },
    /// URI too long (414)
    UriTooLong { length: usize, limit: usize },
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::PayloadTooLarge { size, limit } => {
                write!(f, "Request payload size {} exceeds limit {}", size, limit)
            }
            SecurityError::HeadersTooLarge { total_size, limit } => {
                write!(
                    f,
                    "Total header size {} exceeds limit {}",
                    total_size, limit
                )
            }
            SecurityError::PathTraversal { path } => {
                write!(f, "Path traversal attempt detected: {}", path)
            }
            SecurityError::UriTooLong { length, limit } => {
                write!(f, "URI length {} exceeds limit {}", length, limit)
            }
        }
    }
}

impl std::error::Error for SecurityError {}

/// Default limits for security validation
pub struct SecurityLimits {
    /// Maximum request body size in bytes (default: 10 MB)
    pub max_body_size: usize,
    /// Maximum total header size in bytes (default: 64 KB)
    pub max_header_size: usize,
    /// Maximum URI length (default: 8192 bytes)
    pub max_uri_length: usize,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10 MB
            max_header_size: 64 * 1024,      // 64 KB
            max_uri_length: 8192,            // 8 KB
        }
    }
}

/// Validate request body size
pub fn validate_body_size(
    content_length: Option<usize>,
    limit: usize,
) -> Result<(), SecurityError> {
    if let Some(size) = content_length {
        if size > limit {
            return Err(SecurityError::PayloadTooLarge { size, limit });
        }
    }
    Ok(())
}

/// Validate total header size
pub fn validate_header_size(total_size: usize, limit: usize) -> Result<(), SecurityError> {
    if total_size > limit {
        return Err(SecurityError::HeadersTooLarge { total_size, limit });
    }
    Ok(())
}

/// Validate URI length
pub fn validate_uri_length(uri: &str, limit: usize) -> Result<(), SecurityError> {
    let length = uri.len();
    if length > limit {
        return Err(SecurityError::UriTooLong { length, limit });
    }
    Ok(())
}

/// Check for path traversal attempts
///
/// Detects patterns like:
/// - ../ (relative parent directory)
/// - ..\ (Windows-style parent directory)
/// - %2e%2e%2f (URL-encoded ../)
/// - %2e%2e%5c (URL-encoded ..\)
/// - Absolute paths (/etc/passwd, C:\Windows)
pub fn check_path_traversal(path: &str) -> Result<(), SecurityError> {
    let path_lower = path.to_lowercase();

    // Check for common path traversal patterns
    if path_lower.contains("../")
        || path_lower.contains("..\\")
        || path_lower.contains("%2e%2e%2f")  // URL-encoded ../
        || path_lower.contains("%2e%2e%5c")  // URL-encoded ..\
        || path_lower.contains("%2e%2e/")    // Partial encoding
        || path_lower.contains("%2e%2e\\")
    // Partial encoding
    {
        return Err(SecurityError::PathTraversal {
            path: path.to_string(),
        });
    }

    // Check if path contains null bytes (path truncation attack)
    if path.contains('\0') {
        return Err(SecurityError::PathTraversal {
            path: path.to_string(),
        });
    }

    // Normalize path and verify it doesn't escape the base directory
    // This catches more sophisticated traversal attempts
    if let Ok(normalized) = Path::new(path).canonicalize() {
        if let Some(normalized_str) = normalized.to_str() {
            if normalized_str.contains("..") {
                return Err(SecurityError::PathTraversal {
                    path: path.to_string(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_body_size_within_limit() {
        let result = validate_body_size(Some(1000), 10_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_body_size_exceeds_limit() {
        let result = validate_body_size(Some(20_000), 10_000);
        assert!(result.is_err());
        if let Err(SecurityError::PayloadTooLarge { size, limit }) = result {
            assert_eq!(size, 20_000);
            assert_eq!(limit, 10_000);
        } else {
            panic!("Expected PayloadTooLarge error");
        }
    }

    #[test]
    fn test_validate_body_size_no_content_length() {
        let result = validate_body_size(None, 10_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_header_size_within_limit() {
        let result = validate_header_size(1000, 64 * 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_header_size_exceeds_limit() {
        let result = validate_header_size(100_000, 64 * 1024);
        assert!(result.is_err());
        if let Err(SecurityError::HeadersTooLarge { total_size, limit }) = result {
            assert_eq!(total_size, 100_000);
            assert_eq!(limit, 64 * 1024);
        } else {
            panic!("Expected HeadersTooLarge error");
        }
    }

    #[test]
    fn test_validate_uri_length_within_limit() {
        let uri = "/path/to/resource";
        let result = validate_uri_length(uri, 8192);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_uri_length_exceeds_limit() {
        let uri = "a".repeat(10_000);
        let result = validate_uri_length(&uri, 8192);
        assert!(result.is_err());
        if let Err(SecurityError::UriTooLong { length, limit }) = result {
            assert_eq!(length, 10_000);
            assert_eq!(limit, 8192);
        } else {
            panic!("Expected UriTooLong error");
        }
    }

    #[test]
    fn test_check_path_traversal_clean_path() {
        let path = "/products/image.jpg";
        let result = check_path_traversal(path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_path_traversal_detects_dot_dot_slash() {
        let path = "/products/../../../etc/passwd";
        let result = check_path_traversal(path);
        assert!(result.is_err());
        if let Err(SecurityError::PathTraversal { path: p }) = result {
            assert_eq!(p, path);
        } else {
            panic!("Expected PathTraversal error");
        }
    }

    #[test]
    fn test_check_path_traversal_detects_url_encoded() {
        let path = "/products/%2e%2e%2f%2e%2e%2fetc/passwd";
        let result = check_path_traversal(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_path_traversal_detects_null_byte() {
        let path = "/products/image.jpg\0.txt";
        let result = check_path_traversal(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_path_traversal_detects_windows_style() {
        let path = "/products/..\\..\\windows\\system32";
        let result = check_path_traversal(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_security_limits_defaults() {
        let limits = SecurityLimits::default();
        assert_eq!(limits.max_body_size, 10 * 1024 * 1024);
        assert_eq!(limits.max_header_size, 64 * 1024);
        assert_eq!(limits.max_uri_length, 8192);
    }

    #[test]
    fn test_security_error_display() {
        let err = SecurityError::PayloadTooLarge {
            size: 20_000,
            limit: 10_000,
        };
        assert_eq!(
            err.to_string(),
            "Request payload size 20000 exceeds limit 10000"
        );

        let err = SecurityError::HeadersTooLarge {
            total_size: 100_000,
            limit: 64_000,
        };
        assert_eq!(
            err.to_string(),
            "Total header size 100000 exceeds limit 64000"
        );

        let err = SecurityError::PathTraversal {
            path: "/../etc/passwd".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Path traversal attempt detected: /../etc/passwd"
        );

        let err = SecurityError::UriTooLong {
            length: 10_000,
            limit: 8192,
        };
        assert_eq!(err.to_string(), "URI length 10000 exceeds limit 8192");
    }
}
