//! Logging and metrics module for the proxy.
//!
//! This module provides helpers for metrics recording, circuit breaker updates,
//! S3 error extraction, and audit logging. It extracts logging-related logic
//! from the main proxy module for better organization and testability.
//!
//! # Design
//!
//! Functions return data structures describing what to log/record instead of
//! performing the actual logging. This keeps the logic testable and allows
//! the caller to handle recording appropriately.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Result Types
// ============================================================================

/// Metrics data for a completed request.
#[derive(Debug, Clone)]
pub struct RequestCompletionMetrics {
    /// Request duration in milliseconds.
    pub duration_ms: f64,
    /// HTTP status code.
    pub status_code: u16,
    /// HTTP method.
    pub method: String,
    /// Bucket name (if identified).
    pub bucket: Option<String>,
    /// Bytes sent in response.
    pub bytes_sent: u64,
}

/// Action to take on circuit breaker based on response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerAction {
    /// Record a success (2xx response).
    RecordSuccess,
    /// Record a failure (5xx response).
    RecordFailure,
    /// No action needed (3xx/4xx response or no circuit breaker).
    NoAction,
}

/// S3 error information extracted from response headers.
#[derive(Debug, Clone, Default)]
pub struct S3ErrorInfo {
    /// S3 error code from x-amz-error-code header.
    pub error_code: Option<String>,
    /// S3 error message from x-amz-error-message header.
    pub error_message: Option<String>,
}

impl S3ErrorInfo {
    /// Check if any error information is available.
    pub fn has_error(&self) -> bool {
        self.error_code.is_some() || self.error_message.is_some()
    }
}

/// Data for audit log finalization.
#[derive(Debug, Clone)]
pub struct AuditLogData {
    /// HTTP status code.
    pub status_code: u16,
    /// Response content length.
    pub content_length: u64,
}

/// Request logging context for structured logging.
#[derive(Debug, Clone)]
pub struct RequestLogContext {
    /// Request ID for tracing.
    pub request_id: String,
    /// Client IP address.
    pub client_ip: String,
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// HTTP status code.
    pub status_code: u16,
    /// Duration in milliseconds.
    pub duration_ms: f64,
    /// Bucket name (if identified).
    pub bucket: Option<String>,
    /// S3 error information (if any).
    pub s3_error: Option<S3ErrorInfo>,
}

/// Replica failover log information.
#[derive(Debug, Clone)]
pub struct ReplicaFailoverInfo {
    /// Bucket name.
    pub bucket: String,
    /// Replica identifier.
    pub replica: String,
    /// Status description.
    pub status: String,
}

// ============================================================================
// Duration Calculation
// ============================================================================

/// Calculate request duration from start timestamp.
///
/// # Arguments
///
/// * `start_timestamp` - Start time in seconds since UNIX epoch.
///
/// # Returns
///
/// Duration in milliseconds.
pub fn calculate_duration_ms(start_timestamp: f64) -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as f64;

    let start_ms = start_timestamp * 1000.0; // Convert seconds to milliseconds
    (now - start_ms).max(0.0) // Ensure non-negative
}

/// Calculate request duration from a Duration.
///
/// # Arguments
///
/// * `duration` - The duration to convert.
///
/// # Returns
///
/// Duration in milliseconds.
pub fn duration_to_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

// ============================================================================
// Metrics Preparation
// ============================================================================

/// Prepare metrics data for a completed request.
///
/// # Arguments
///
/// * `status_code` - HTTP response status code.
/// * `method` - HTTP request method.
/// * `bucket` - Bucket name (if identified).
/// * `duration_ms` - Request duration in milliseconds.
/// * `bytes_sent` - Bytes sent in response.
///
/// # Returns
///
/// A `RequestCompletionMetrics` struct with all metrics data.
pub fn prepare_request_metrics(
    status_code: u16,
    method: &str,
    bucket: Option<&str>,
    duration_ms: f64,
    bytes_sent: u64,
) -> RequestCompletionMetrics {
    RequestCompletionMetrics {
        duration_ms,
        status_code,
        method: method.to_string(),
        bucket: bucket.map(|s| s.to_string()),
        bytes_sent,
    }
}

// ============================================================================
// Circuit Breaker Logic
// ============================================================================

/// Determine circuit breaker action based on HTTP status code.
///
/// # Rules
///
/// * 2xx: Success - record success
/// * 5xx: Server error - record failure
/// * 3xx/4xx: Client error/redirect - no action
///
/// # Arguments
///
/// * `status_code` - HTTP response status code.
///
/// # Returns
///
/// The appropriate `CircuitBreakerAction`.
pub fn determine_circuit_breaker_action(status_code: u16) -> CircuitBreakerAction {
    if is_circuit_breaker_success(status_code) {
        CircuitBreakerAction::RecordSuccess
    } else if is_circuit_breaker_failure(status_code) {
        CircuitBreakerAction::RecordFailure
    } else {
        CircuitBreakerAction::NoAction
    }
}

/// Check if status code indicates a circuit breaker success.
pub fn is_circuit_breaker_success(status_code: u16) -> bool {
    (200..300).contains(&status_code)
}

/// Check if status code indicates a circuit breaker failure.
pub fn is_circuit_breaker_failure(status_code: u16) -> bool {
    status_code >= 500
}

// ============================================================================
// S3 Error Extraction
// ============================================================================

/// Extract S3 error information from response headers.
///
/// Looks for x-amz-error-code and x-amz-error-message headers.
///
/// # Arguments
///
/// * `error_code` - Value of x-amz-error-code header (if present).
/// * `error_message` - Value of x-amz-error-message header (if present).
///
/// # Returns
///
/// An `S3ErrorInfo` struct with the extracted information.
pub fn build_s3_error_info(error_code: Option<&str>, error_message: Option<&str>) -> S3ErrorInfo {
    S3ErrorInfo {
        error_code: error_code.map(|s| s.to_string()),
        error_message: error_message.map(|s| s.to_string()),
    }
}

/// Check if a status code should trigger S3 error extraction.
///
/// Only 4xx and 5xx responses may contain S3 error headers.
pub fn should_extract_s3_error(status_code: u16) -> bool {
    status_code >= 400
}

// ============================================================================
// Audit Log Preparation
// ============================================================================

/// Prepare audit log finalization data.
///
/// # Arguments
///
/// * `status_code` - HTTP response status code.
/// * `content_length` - Response content length in bytes.
///
/// # Returns
///
/// An `AuditLogData` struct ready for finalization.
pub fn prepare_audit_data(status_code: u16, content_length: u64) -> AuditLogData {
    AuditLogData {
        status_code,
        content_length,
    }
}

/// Parse content length from header value.
///
/// # Arguments
///
/// * `content_length_str` - Content-Length header value as string.
///
/// # Returns
///
/// Parsed content length, or 0 if parsing fails.
pub fn parse_content_length(content_length_str: Option<&str>) -> u64 {
    content_length_str
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

// ============================================================================
// Logging Context Builders
// ============================================================================

/// Build a request log context for structured logging.
///
/// # Arguments
///
/// * `request_id` - Request ID for tracing.
/// * `client_ip` - Client IP address.
/// * `method` - HTTP method.
/// * `path` - Request path.
/// * `status_code` - HTTP status code.
/// * `duration_ms` - Duration in milliseconds.
/// * `bucket` - Bucket name (if identified).
/// * `s3_error` - S3 error information (if any).
///
/// # Returns
///
/// A `RequestLogContext` for structured logging.
#[allow(clippy::too_many_arguments)]
pub fn build_request_log_context(
    request_id: &str,
    client_ip: &str,
    method: &str,
    path: &str,
    status_code: u16,
    duration_ms: f64,
    bucket: Option<&str>,
    s3_error: Option<S3ErrorInfo>,
) -> RequestLogContext {
    RequestLogContext {
        request_id: request_id.to_string(),
        client_ip: client_ip.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        status_code,
        duration_ms,
        bucket: bucket.map(|s| s.to_string()),
        s3_error,
    }
}

/// Build replica failover information for logging.
///
/// # Arguments
///
/// * `bucket` - Bucket name.
/// * `replica` - Replica identifier.
/// * `status` - Status description.
///
/// # Returns
///
/// A `ReplicaFailoverInfo` for logging.
pub fn build_replica_failover_info(
    bucket: &str,
    replica: &str,
    status: &str,
) -> ReplicaFailoverInfo {
    ReplicaFailoverInfo {
        bucket: bucket.to_string(),
        replica: replica.to_string(),
        status: status.to_string(),
    }
}

// ============================================================================
// Log Level Determination
// ============================================================================

/// Log level for request completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestLogLevel {
    /// Info level - successful requests.
    Info,
    /// Warn level - client errors or S3 errors.
    Warn,
    /// Error level - server errors.
    Error,
}

/// Determine appropriate log level based on status code.
///
/// # Arguments
///
/// * `status_code` - HTTP status code.
///
/// # Returns
///
/// Appropriate `RequestLogLevel`.
pub fn determine_log_level(status_code: u16) -> RequestLogLevel {
    match status_code {
        500.. => RequestLogLevel::Error,
        400..=499 => RequestLogLevel::Warn,
        _ => RequestLogLevel::Info,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Structural verification tests --

    #[test]
    fn test_logging_module_exists() {
        // Phase 37.8 structural verification test
        let _ = RequestCompletionMetrics {
            duration_ms: 0.0,
            status_code: 200,
            method: "GET".to_string(),
            bucket: None,
            bytes_sent: 0,
        };
        let _ = CircuitBreakerAction::NoAction;
        let _ = S3ErrorInfo::default();
        let _ = AuditLogData {
            status_code: 200,
            content_length: 0,
        };
    }

    #[test]
    fn test_record_request_completion_accessible() {
        // Verify prepare_request_metrics is accessible
        let metrics = prepare_request_metrics(200, "GET", Some("bucket"), 100.0, 1024);
        assert_eq!(metrics.status_code, 200);
        assert_eq!(metrics.method, "GET");
        assert_eq!(metrics.bucket, Some("bucket".to_string()));
        assert_eq!(metrics.duration_ms, 100.0);
        assert_eq!(metrics.bytes_sent, 1024);
    }

    #[test]
    fn test_update_circuit_breaker_accessible() {
        // Verify determine_circuit_breaker_action is accessible
        let action = determine_circuit_breaker_action(200);
        assert_eq!(action, CircuitBreakerAction::RecordSuccess);
    }

    #[test]
    fn test_extract_s3_error_accessible() {
        // Verify build_s3_error_info is accessible
        let error =
            build_s3_error_info(Some("NoSuchKey"), Some("The specified key does not exist"));
        assert!(error.has_error());
        assert_eq!(error.error_code, Some("NoSuchKey".to_string()));
    }

    #[test]
    fn test_finalize_audit_log_accessible() {
        // Verify prepare_audit_data is accessible
        let audit = prepare_audit_data(200, 1024);
        assert_eq!(audit.status_code, 200);
        assert_eq!(audit.content_length, 1024);
    }

    // -- Duration calculation tests --

    #[test]
    fn test_duration_to_ms() {
        let duration = Duration::from_millis(1500);
        let ms = duration_to_ms(duration);
        assert!((ms - 1500.0).abs() < 0.001);
    }

    #[test]
    fn test_duration_to_ms_zero() {
        let duration = Duration::ZERO;
        let ms = duration_to_ms(duration);
        assert_eq!(ms, 0.0);
    }

    #[test]
    fn test_calculate_duration_ms_current_time() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Test with current time, should be close to 0
        let duration = calculate_duration_ms(now_s);
        assert!(
            (0.0..100.0).contains(&duration),
            "Duration should be small for current time, got {}",
            duration
        );
    }

    #[test]
    fn test_calculate_duration_ms_past_time() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Test with a time 1 second in the past
        let start_timestamp = now_s - 1.0;
        let duration = calculate_duration_ms(start_timestamp);
        // Should be around 1000ms. Allow for some variance.
        assert!(
            (duration - 1000.0).abs() < 100.0,
            "Duration for 1s ago should be ~1000ms, got {}",
            duration
        );
    }

    #[test]
    fn test_calculate_duration_ms_future_time() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Test with a time in the future, should be 0
        let future_timestamp = now_s + 10.0;
        let duration = calculate_duration_ms(future_timestamp);
        assert_eq!(duration, 0.0, "Duration for future timestamp should be 0");
    }

    // -- Circuit breaker action tests --

    #[test]
    fn test_circuit_breaker_action_success_200() {
        let action = determine_circuit_breaker_action(200);
        assert_eq!(action, CircuitBreakerAction::RecordSuccess);
    }

    #[test]
    fn test_circuit_breaker_action_success_204() {
        let action = determine_circuit_breaker_action(204);
        assert_eq!(action, CircuitBreakerAction::RecordSuccess);
    }

    #[test]
    fn test_circuit_breaker_action_failure_500() {
        let action = determine_circuit_breaker_action(500);
        assert_eq!(action, CircuitBreakerAction::RecordFailure);
    }

    #[test]
    fn test_circuit_breaker_action_failure_502() {
        let action = determine_circuit_breaker_action(502);
        assert_eq!(action, CircuitBreakerAction::RecordFailure);
    }

    #[test]
    fn test_circuit_breaker_action_no_action_404() {
        let action = determine_circuit_breaker_action(404);
        assert_eq!(action, CircuitBreakerAction::NoAction);
    }

    #[test]
    fn test_circuit_breaker_action_no_action_301() {
        let action = determine_circuit_breaker_action(301);
        assert_eq!(action, CircuitBreakerAction::NoAction);
    }

    #[test]
    fn test_is_circuit_breaker_success() {
        assert!(is_circuit_breaker_success(200));
        assert!(is_circuit_breaker_success(201));
        assert!(is_circuit_breaker_success(299));
        assert!(!is_circuit_breaker_success(300));
        assert!(!is_circuit_breaker_success(404));
        assert!(!is_circuit_breaker_success(500));
    }

    #[test]
    fn test_is_circuit_breaker_failure() {
        assert!(is_circuit_breaker_failure(500));
        assert!(is_circuit_breaker_failure(502));
        assert!(is_circuit_breaker_failure(599));
        assert!(!is_circuit_breaker_failure(200));
        assert!(!is_circuit_breaker_failure(404));
        assert!(!is_circuit_breaker_failure(499));
    }

    // -- S3 error extraction tests --

    #[test]
    fn test_s3_error_info_with_code_and_message() {
        let error = build_s3_error_info(Some("AccessDenied"), Some("Access Denied"));
        assert!(error.has_error());
        assert_eq!(error.error_code, Some("AccessDenied".to_string()));
        assert_eq!(error.error_message, Some("Access Denied".to_string()));
    }

    #[test]
    fn test_s3_error_info_code_only() {
        let error = build_s3_error_info(Some("NoSuchKey"), None);
        assert!(error.has_error());
        assert_eq!(error.error_code, Some("NoSuchKey".to_string()));
        assert_eq!(error.error_message, None);
    }

    #[test]
    fn test_s3_error_info_empty() {
        let error = build_s3_error_info(None, None);
        assert!(!error.has_error());
    }

    #[test]
    fn test_should_extract_s3_error() {
        assert!(should_extract_s3_error(400));
        assert!(should_extract_s3_error(404));
        assert!(should_extract_s3_error(500));
        assert!(!should_extract_s3_error(200));
        assert!(!should_extract_s3_error(301));
        assert!(!should_extract_s3_error(399));
    }

    // -- Audit log tests --

    #[test]
    fn test_prepare_audit_data() {
        let data = prepare_audit_data(201, 2048);
        assert_eq!(data.status_code, 201);
        assert_eq!(data.content_length, 2048);
    }

    #[test]
    fn test_parse_content_length_valid() {
        assert_eq!(parse_content_length(Some("1024")), 1024);
        assert_eq!(parse_content_length(Some("0")), 0);
        assert_eq!(parse_content_length(Some("9999999")), 9999999);
    }

    #[test]
    fn test_parse_content_length_invalid() {
        assert_eq!(parse_content_length(Some("not-a-number")), 0);
        assert_eq!(parse_content_length(Some("")), 0);
        assert_eq!(parse_content_length(None), 0);
    }

    // -- Log context tests --

    #[test]
    fn test_build_request_log_context() {
        let ctx = build_request_log_context(
            "req-123",
            "192.168.1.1",
            "GET",
            "/bucket/key",
            200,
            50.5,
            Some("my-bucket"),
            None,
        );

        assert_eq!(ctx.request_id, "req-123");
        assert_eq!(ctx.client_ip, "192.168.1.1");
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/bucket/key");
        assert_eq!(ctx.status_code, 200);
        assert!((ctx.duration_ms - 50.5).abs() < 0.001);
        assert_eq!(ctx.bucket, Some("my-bucket".to_string()));
        assert!(ctx.s3_error.is_none());
    }

    #[test]
    fn test_build_request_log_context_with_error() {
        let s3_error = S3ErrorInfo {
            error_code: Some("NoSuchKey".to_string()),
            error_message: Some("Key not found".to_string()),
        };

        let ctx = build_request_log_context(
            "req-456",
            "10.0.0.1",
            "HEAD",
            "/bucket/missing",
            404,
            10.0,
            Some("test-bucket"),
            Some(s3_error),
        );

        assert_eq!(ctx.status_code, 404);
        assert!(ctx.s3_error.is_some());
        let error = ctx.s3_error.unwrap();
        assert_eq!(error.error_code, Some("NoSuchKey".to_string()));
    }

    #[test]
    fn test_build_replica_failover_info() {
        let info = build_replica_failover_info("my-bucket", "replica-1", "failover");

        assert_eq!(info.bucket, "my-bucket");
        assert_eq!(info.replica, "replica-1");
        assert_eq!(info.status, "failover");
    }

    // -- Log level tests --

    #[test]
    fn test_determine_log_level_info() {
        assert_eq!(determine_log_level(200), RequestLogLevel::Info);
        assert_eq!(determine_log_level(204), RequestLogLevel::Info);
        assert_eq!(determine_log_level(301), RequestLogLevel::Info);
        assert_eq!(determine_log_level(304), RequestLogLevel::Info);
    }

    #[test]
    fn test_determine_log_level_warn() {
        assert_eq!(determine_log_level(400), RequestLogLevel::Warn);
        assert_eq!(determine_log_level(404), RequestLogLevel::Warn);
        assert_eq!(determine_log_level(429), RequestLogLevel::Warn);
        assert_eq!(determine_log_level(499), RequestLogLevel::Warn);
    }

    #[test]
    fn test_determine_log_level_error() {
        assert_eq!(determine_log_level(500), RequestLogLevel::Error);
        assert_eq!(determine_log_level(502), RequestLogLevel::Error);
        assert_eq!(determine_log_level(503), RequestLogLevel::Error);
        assert_eq!(determine_log_level(599), RequestLogLevel::Error);
    }

    // -- Metrics preparation tests --

    #[test]
    fn test_prepare_request_metrics_full() {
        let metrics = prepare_request_metrics(201, "POST", Some("uploads"), 150.5, 4096);

        assert_eq!(metrics.status_code, 201);
        assert_eq!(metrics.method, "POST");
        assert_eq!(metrics.bucket, Some("uploads".to_string()));
        assert!((metrics.duration_ms - 150.5).abs() < 0.001);
        assert_eq!(metrics.bytes_sent, 4096);
    }

    #[test]
    fn test_prepare_request_metrics_no_bucket() {
        let metrics = prepare_request_metrics(404, "GET", None, 5.0, 0);

        assert_eq!(metrics.status_code, 404);
        assert_eq!(metrics.method, "GET");
        assert!(metrics.bucket.is_none());
        assert_eq!(metrics.bytes_sent, 0);
    }
}
