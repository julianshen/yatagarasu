//! Security validation orchestration for the proxy.
//!
//! This module provides helper functions that combine security validation
//! with logging and error message generation. It wraps the core validation
//! functions from `crate::security` with structured error reporting.
//!
//! # Design
//!
//! Each function follows the pattern:
//! 1. Validate using `crate::security` functions
//! 2. On error: log the issue and return structured error info
//! 3. Return `None` if validation passed, `Some(SecurityViolation)` if failed
//!
//! The caller is responsible for writing the HTTP response and updating metrics.
//! This separation avoids borrow checker issues with session references.

use crate::metrics::Metrics;
use crate::security::{self, SecurityLimits};

/// Information about a security violation that should result in an error response.
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// HTTP status code to return
    pub status: u16,
    /// JSON error response body
    pub error_body: String,
    /// Action to update metrics
    pub metric_action: SecurityMetricAction,
}

/// Metric action to take after a security violation.
#[derive(Debug, Clone, Copy)]
pub enum SecurityMetricAction {
    UriTooLong,
    HeadersTooLarge,
    PayloadTooLarge,
    PathTraversalBlocked,
    SqlInjectionBlocked,
}

impl SecurityMetricAction {
    /// Update metrics for this security action.
    pub fn update_metrics(&self, metrics: &Metrics, status: u16) {
        metrics.increment_status_count(status);
        match self {
            SecurityMetricAction::UriTooLong => metrics.increment_security_uri_too_long(),
            SecurityMetricAction::HeadersTooLarge => metrics.increment_security_headers_too_large(),
            SecurityMetricAction::PayloadTooLarge => metrics.increment_security_payload_too_large(),
            SecurityMetricAction::PathTraversalBlocked => {
                metrics.increment_security_path_traversal_blocked()
            }
            SecurityMetricAction::SqlInjectionBlocked => {
                metrics.increment_security_sql_injection_blocked()
            }
        }
    }
}

/// Build a JSON error response body.
fn build_error_body(error_type: &str, message: &str, status: u16) -> String {
    serde_json::json!({
        "error": error_type,
        "message": message,
        "status": status
    })
    .to_string()
}

/// Validate URI length against configured limits.
///
/// Returns `None` if validation passed.
/// Returns `Some(SecurityViolation)` if URI is too long.
pub fn check_uri_length(
    request_id: &str,
    client_ip: &str,
    uri: &str,
    limit: usize,
) -> Option<SecurityViolation> {
    if let Err(security_error) = security::validate_uri_length(uri, limit) {
        tracing::warn!(
            request_id = %request_id,
            client_ip = %client_ip,
            uri_length = uri.len(),
            limit = limit,
            error = %security_error,
            "URI too long"
        );

        return Some(SecurityViolation {
            status: 414,
            error_body: build_error_body("URI Too Long", &security_error.to_string(), 414),
            metric_action: SecurityMetricAction::UriTooLong,
        });
    }
    None
}

/// Validate total header size against configured limits.
///
/// Returns `None` if validation passed.
/// Returns `Some(SecurityViolation)` if headers are too large.
pub fn check_header_size(
    request_id: &str,
    client_ip: &str,
    total_size: usize,
    limit: usize,
) -> Option<SecurityViolation> {
    if let Err(security_error) = security::validate_header_size(total_size, limit) {
        tracing::warn!(
            request_id = %request_id,
            client_ip = %client_ip,
            header_size = total_size,
            limit = limit,
            error = %security_error,
            "Headers too large"
        );

        return Some(SecurityViolation {
            status: 431,
            error_body: build_error_body(
                "Request Header Fields Too Large",
                &security_error.to_string(),
                431,
            ),
            metric_action: SecurityMetricAction::HeadersTooLarge,
        });
    }
    None
}

/// Validate request body size from Content-Length header.
///
/// Returns `None` if validation passed.
/// Returns `Some(SecurityViolation)` if body size exceeds limit.
pub fn check_body_size(
    request_id: &str,
    client_ip: &str,
    content_length: Option<usize>,
    limit: usize,
) -> Option<SecurityViolation> {
    if let Err(security_error) = security::validate_body_size(content_length, limit) {
        tracing::warn!(
            request_id = %request_id,
            client_ip = %client_ip,
            content_length = ?content_length,
            limit = limit,
            error = %security_error,
            "Request payload too large"
        );

        return Some(SecurityViolation {
            status: 413,
            error_body: build_error_body("Payload Too Large", &security_error.to_string(), 413),
            metric_action: SecurityMetricAction::PayloadTooLarge,
        });
    }
    None
}

/// Check for path traversal attempts in the URI.
///
/// Returns `None` if no attack detected.
/// Returns `Some(SecurityViolation)` if path traversal is detected.
pub fn check_path_traversal(
    request_id: &str,
    client_ip: &str,
    uri: &str,
) -> Option<SecurityViolation> {
    if let Err(security_error) = security::check_path_traversal(uri) {
        tracing::warn!(
            request_id = %request_id,
            client_ip = %client_ip,
            uri = %uri,
            error = %security_error,
            "Path traversal attempt detected in raw URI"
        );

        return Some(SecurityViolation {
            status: 400,
            error_body: build_error_body("Bad Request", &security_error.to_string(), 400),
            metric_action: SecurityMetricAction::PathTraversalBlocked,
        });
    }
    None
}

/// Check for SQL injection attempts in the URI.
///
/// Returns `None` if no attack detected.
/// Returns `Some(SecurityViolation)` if SQL injection is detected.
pub fn check_sql_injection(
    request_id: &str,
    client_ip: &str,
    uri: &str,
) -> Option<SecurityViolation> {
    if let Err(security_error) = security::check_sql_injection(uri) {
        tracing::warn!(
            request_id = %request_id,
            client_ip = %client_ip,
            uri = %uri,
            error = %security_error,
            "SQL injection attempt detected in raw URI"
        );

        return Some(SecurityViolation {
            status: 400,
            error_body: build_error_body("Bad Request", &security_error.to_string(), 400),
            metric_action: SecurityMetricAction::SqlInjectionBlocked,
        });
    }
    None
}

/// Perform all security validations in sequence.
///
/// This is a convenience function that runs all security checks in order:
/// 1. URI length validation
/// 2. Header size validation
/// 3. Body size validation (if Content-Length present)
/// 4. Path traversal detection
/// 5. SQL injection detection
///
/// Returns `None` if all validations passed.
/// Returns `Some(SecurityViolation)` on the first validation failure.
pub fn validate_request_security(
    request_id: &str,
    client_ip: &str,
    uri: &str,
    total_header_size: usize,
    content_length: Option<usize>,
    limits: &SecurityLimits,
) -> Option<SecurityViolation> {
    // 1. Validate URI length
    if let Some(violation) = check_uri_length(request_id, client_ip, uri, limits.max_uri_length) {
        return Some(violation);
    }

    // 2. Validate header size
    if let Some(violation) = check_header_size(
        request_id,
        client_ip,
        total_header_size,
        limits.max_header_size,
    ) {
        return Some(violation);
    }

    // 3. Validate body size
    if let Some(violation) =
        check_body_size(request_id, client_ip, content_length, limits.max_body_size)
    {
        return Some(violation);
    }

    // 4. Check for path traversal
    if let Some(violation) = check_path_traversal(request_id, client_ip, uri) {
        return Some(violation);
    }

    // 5. Check for SQL injection
    if let Some(violation) = check_sql_injection(request_id, client_ip, uri) {
        return Some(violation);
    }

    None
}

// SecurityLimits is imported from crate::security and used in function signatures

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecurityError;

    #[test]
    fn test_build_error_body() {
        let body = build_error_body("Test Error", "Something went wrong", 400);
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();

        assert_eq!(parsed["error"], "Test Error");
        assert_eq!(parsed["message"], "Something went wrong");
        assert_eq!(parsed["status"], 400);
    }

    #[test]
    fn test_security_module_exists() {
        // Phase 37.1 structural verification test
        // This test verifies that the security module exists and exports the expected types
        let _: fn(&str, usize) -> Result<(), SecurityError> = security::validate_uri_length;
        let _: fn(usize, usize) -> Result<(), SecurityError> = security::validate_header_size;
        let _: fn(Option<usize>, usize) -> Result<(), SecurityError> = security::validate_body_size;
        let _: fn(&str) -> Result<(), SecurityError> = security::check_path_traversal;
        let _: fn(&str) -> Result<(), SecurityError> = security::check_sql_injection;
    }

    #[test]
    fn test_check_uri_length_pass() {
        let result = check_uri_length("test-req", "127.0.0.1", "/short/path", 8192);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_uri_length_fail() {
        let long_uri = "/".to_string() + &"a".repeat(10000);
        let result = check_uri_length("test-req", "127.0.0.1", &long_uri, 8192);
        assert!(result.is_some());
        let violation = result.unwrap();
        assert_eq!(violation.status, 414);
        assert!(matches!(
            violation.metric_action,
            SecurityMetricAction::UriTooLong
        ));
    }

    #[test]
    fn test_check_path_traversal_pass() {
        let result = check_path_traversal("test-req", "127.0.0.1", "/products/image.jpg");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_path_traversal_fail() {
        let result = check_path_traversal("test-req", "127.0.0.1", "/products/../../../etc/passwd");
        assert!(result.is_some());
        let violation = result.unwrap();
        assert_eq!(violation.status, 400);
        assert!(matches!(
            violation.metric_action,
            SecurityMetricAction::PathTraversalBlocked
        ));
    }

    #[test]
    fn test_validate_request_security_all_pass() {
        let limits = SecurityLimits::default();
        let result = validate_request_security(
            "test-req",
            "127.0.0.1",
            "/products/image.jpg",
            1000,
            Some(5000),
            &limits,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_request_security_uri_too_long() {
        let limits = SecurityLimits {
            max_uri_length: 100,
            ..SecurityLimits::default()
        };
        let long_uri = "/".to_string() + &"a".repeat(200);
        let result =
            validate_request_security("test-req", "127.0.0.1", &long_uri, 1000, None, &limits);
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, 414);
    }
}
