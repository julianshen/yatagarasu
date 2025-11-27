//! Audit Logging Module (Phase 33)
//!
//! This module provides comprehensive audit logging for all proxy requests,
//! including request details, response status, timing metrics, and cache status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Cache status for a request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CacheStatus {
    /// Cache hit - response served from cache
    Hit,
    /// Cache miss - response fetched from S3
    Miss,
    /// Cache bypass - request bypassed cache (e.g., range request)
    Bypass,
}

/// Audit log entry representing a single request/response cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Timestamp of the request (RFC3339 format)
    pub timestamp: DateTime<Utc>,

    /// Unique correlation ID for request tracing (UUID)
    pub correlation_id: String,

    /// Client IP address (real IP, not proxy IP)
    pub client_ip: String,

    /// Authenticated user (from JWT sub/username claim), None if anonymous
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// S3 bucket name being accessed
    pub bucket: String,

    /// S3 object key (path within bucket)
    pub object_key: String,

    /// HTTP method (GET or HEAD)
    pub http_method: String,

    /// Original URL request path
    pub request_path: String,

    /// HTTP response status code
    pub response_status: u16,

    /// Response body size in bytes
    pub response_size_bytes: u64,

    /// Request processing duration in milliseconds
    pub duration_ms: u64,

    /// Cache status (hit, miss, bypass)
    pub cache_status: CacheStatus,

    /// User-Agent header from request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Referer header from request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referer: Option<String>,
}

impl AuditLogEntry {
    /// Create a new audit log entry with required fields
    pub fn new(
        client_ip: String,
        bucket: String,
        object_key: String,
        http_method: String,
        request_path: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            correlation_id: Uuid::new_v4().to_string(),
            client_ip,
            user: None,
            bucket,
            object_key,
            http_method,
            request_path,
            response_status: 0,
            response_size_bytes: 0,
            duration_ms: 0,
            cache_status: CacheStatus::Miss,
            user_agent: None,
            referer: None,
        }
    }

    /// Set the authenticated user
    pub fn with_user(mut self, user: Option<String>) -> Self {
        self.user = user;
        self
    }

    /// Set response details
    pub fn with_response(mut self, status: u16, size_bytes: u64, duration_ms: u64) -> Self {
        self.response_status = status;
        self.response_size_bytes = size_bytes;
        self.duration_ms = duration_ms;
        self
    }

    /// Set cache status
    pub fn with_cache_status(mut self, status: CacheStatus) -> Self {
        self.cache_status = status;
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// Set referer
    pub fn with_referer(mut self, referer: Option<String>) -> Self {
        self.referer = referer;
        self
    }
}

// ============================================================================
// Sensitive Data Redaction Functions
// ============================================================================

/// Redact JWT tokens in a string
///
/// JWT tokens follow the format: header.payload.signature (base64 encoded)
/// This function detects and redacts them.
pub fn redact_jwt_token(value: &str) -> String {
    // JWT pattern: base64url.base64url.base64url (at minimum header.payload)
    // Header typically starts with "eyJ" (base64 for '{"')
    if value.starts_with("eyJ") && value.contains('.') {
        "[JWT_REDACTED]".to_string()
    } else {
        value.to_string()
    }
}

/// Redact Authorization header value
///
/// Preserves the auth scheme (Bearer, Basic, etc.) but redacts the credential
pub fn redact_authorization_header(value: &str) -> String {
    if value.is_empty() {
        return value.to_string();
    }

    // Split on first space to get scheme and credential
    if let Some(space_idx) = value.find(' ') {
        let scheme = &value[..space_idx];
        format!("{} [REDACTED]", scheme)
    } else {
        // No space, likely just a token - redact entirely
        "[REDACTED]".to_string()
    }
}

/// Redact sensitive query parameters from a URL path
///
/// Replaces values of specified parameter names with [REDACTED]
pub fn redact_query_params(url: &str, sensitive_params: &[&str]) -> String {
    // Split URL into path and query parts
    if let Some(query_start) = url.find('?') {
        let path = &url[..query_start];
        let query = &url[query_start + 1..];

        // Parse and redact query params
        let redacted_params: Vec<String> = query
            .split('&')
            .map(|param| {
                if let Some(eq_idx) = param.find('=') {
                    let key = &param[..eq_idx];
                    // Case-insensitive comparison for sensitive params
                    if sensitive_params.iter().any(|s| s.eq_ignore_ascii_case(key)) {
                        format!("{}=[REDACTED]", key)
                    } else {
                        param.to_string()
                    }
                } else {
                    param.to_string()
                }
            })
            .collect();

        format!("{}?{}", path, redacted_params.join("&"))
    } else {
        url.to_string()
    }
}

/// Redact sensitive headers from a list of header key-value pairs
///
/// Returns a new list with sensitive header values replaced with [REDACTED]
pub fn redact_headers(
    headers: &[(&str, &str)],
    sensitive_headers: &[&str],
) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(key, value)| {
            // Case-insensitive comparison for header names
            if sensitive_headers
                .iter()
                .any(|s| s.eq_ignore_ascii_case(key))
            {
                (key.to_string(), "[REDACTED]".to_string())
            } else {
                (key.to_string(), value.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Phase 33.2: Audit Log Entry Structure Tests
    // ============================================================================

    #[test]
    fn test_can_create_audit_log_entry_struct() {
        // Test: Can create AuditLogEntry struct
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "my-bucket".to_string(),
            "path/to/file.txt".to_string(),
            "GET".to_string(),
            "/my-bucket/path/to/file.txt".to_string(),
        );

        assert_eq!(entry.client_ip, "192.168.1.100");
        assert_eq!(entry.bucket, "my-bucket");
        assert_eq!(entry.object_key, "path/to/file.txt");
        assert_eq!(entry.http_method, "GET");
        assert_eq!(entry.request_path, "/my-bucket/path/to/file.txt");
    }

    #[test]
    fn test_audit_log_entry_contains_timestamp() {
        // Test: Contains timestamp (RFC3339 format)
        let before = Utc::now();
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        let after = Utc::now();

        // Timestamp should be between before and after
        assert!(entry.timestamp >= before);
        assert!(entry.timestamp <= after);

        // Should serialize to RFC3339 format
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("timestamp"));
        // RFC3339 format includes 'T' separator
        assert!(json.contains("T"));
    }

    #[test]
    fn test_audit_log_entry_contains_correlation_id() {
        // Test: Contains correlation_id (UUID)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        // Correlation ID should be a valid UUID
        let parsed = Uuid::parse_str(&entry.correlation_id);
        assert!(
            parsed.is_ok(),
            "correlation_id should be valid UUID: {}",
            entry.correlation_id
        );

        // Each entry should have unique correlation ID
        let entry2 = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        assert_ne!(
            entry.correlation_id, entry2.correlation_id,
            "Each entry should have unique correlation_id"
        );
    }

    #[test]
    fn test_audit_log_entry_contains_client_ip() {
        // Test: Contains client_ip (real IP, not proxy IP)
        let entry = AuditLogEntry::new(
            "10.0.0.50".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        assert_eq!(entry.client_ip, "10.0.0.50");
    }

    #[test]
    fn test_audit_log_entry_contains_user() {
        // Test: Contains user (from JWT sub/username claim, if authenticated)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_user(Some("john.doe@example.com".to_string()));

        assert_eq!(entry.user, Some("john.doe@example.com".to_string()));

        // Anonymous request
        let anon_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        assert!(anon_entry.user.is_none());
    }

    #[test]
    fn test_audit_log_entry_contains_bucket_name() {
        // Test: Contains bucket name
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "production-assets".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        assert_eq!(entry.bucket, "production-assets");
    }

    #[test]
    fn test_audit_log_entry_contains_object_key() {
        // Test: Contains object_key (S3 path)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "images/2024/photo.jpg".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        assert_eq!(entry.object_key, "images/2024/photo.jpg");
    }

    #[test]
    fn test_audit_log_entry_contains_http_method() {
        // Test: Contains http_method (GET/HEAD)
        let get_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        assert_eq!(get_entry.http_method, "GET");

        let head_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "HEAD".to_string(),
            "/path".to_string(),
        );
        assert_eq!(head_entry.http_method, "HEAD");
    }

    #[test]
    fn test_audit_log_entry_contains_request_path() {
        // Test: Contains request_path (original URL path)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/api/v1/files/document.pdf".to_string(),
        );

        assert_eq!(entry.request_path, "/api/v1/files/document.pdf");
    }

    #[test]
    fn test_audit_log_entry_contains_response_status() {
        // Test: Contains response_status (200, 404, 403, etc.)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1024, 50);

        assert_eq!(entry.response_status, 200);

        let not_found = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(404, 0, 10);

        assert_eq!(not_found.response_status, 404);
    }

    #[test]
    fn test_audit_log_entry_contains_response_size_bytes() {
        // Test: Contains response_size_bytes
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1_048_576, 100); // 1 MB

        assert_eq!(entry.response_size_bytes, 1_048_576);
    }

    #[test]
    fn test_audit_log_entry_contains_duration_ms() {
        // Test: Contains duration_ms (request processing time)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1024, 150);

        assert_eq!(entry.duration_ms, 150);
    }

    #[test]
    fn test_audit_log_entry_contains_cache_status() {
        // Test: Contains cache_status (hit, miss, bypass)
        let hit_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_cache_status(CacheStatus::Hit);
        assert_eq!(hit_entry.cache_status, CacheStatus::Hit);

        let miss_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_cache_status(CacheStatus::Miss);
        assert_eq!(miss_entry.cache_status, CacheStatus::Miss);

        let bypass_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_cache_status(CacheStatus::Bypass);
        assert_eq!(bypass_entry.cache_status, CacheStatus::Bypass);
    }

    #[test]
    fn test_audit_log_entry_contains_user_agent() {
        // Test: Contains user_agent (from request headers)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_user_agent(Some(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string(),
        ));

        assert_eq!(
            entry.user_agent,
            Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string())
        );
    }

    #[test]
    fn test_audit_log_entry_contains_referer() {
        // Test: Contains referer (from request headers)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_referer(Some("https://example.com/page".to_string()));

        assert_eq!(entry.referer, Some("https://example.com/page".to_string()));
    }

    // ============================================================================
    // JSON Serialization Tests
    // ============================================================================

    #[test]
    fn test_audit_log_entry_serializes_to_json() {
        // Test: AuditLogEntry serializes to JSON
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "my-bucket".to_string(),
            "path/to/file.txt".to_string(),
            "GET".to_string(),
            "/my-bucket/path/to/file.txt".to_string(),
        )
        .with_response(200, 1024, 50)
        .with_cache_status(CacheStatus::Hit);

        let json_result = serde_json::to_string(&entry);
        assert!(json_result.is_ok(), "Should serialize to JSON");

        let json = json_result.unwrap();
        assert!(json.contains("\"client_ip\":\"192.168.1.100\""));
        assert!(json.contains("\"bucket\":\"my-bucket\""));
        assert!(json.contains("\"response_status\":200"));
    }

    #[test]
    fn test_audit_log_entry_all_fields_in_json() {
        // Test: All fields included in JSON output
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_user(Some("testuser".to_string()))
        .with_response(200, 1024, 50)
        .with_cache_status(CacheStatus::Hit)
        .with_user_agent(Some("TestAgent".to_string()))
        .with_referer(Some("https://ref.example.com".to_string()));

        let json = serde_json::to_string(&entry).unwrap();

        // All required fields should be present
        assert!(json.contains("timestamp"));
        assert!(json.contains("correlation_id"));
        assert!(json.contains("client_ip"));
        assert!(json.contains("user"));
        assert!(json.contains("bucket"));
        assert!(json.contains("object_key"));
        assert!(json.contains("http_method"));
        assert!(json.contains("request_path"));
        assert!(json.contains("response_status"));
        assert!(json.contains("response_size_bytes"));
        assert!(json.contains("duration_ms"));
        assert!(json.contains("cache_status"));
        assert!(json.contains("user_agent"));
        assert!(json.contains("referer"));
    }

    #[test]
    fn test_audit_log_entry_timestamp_iso8601_format() {
        // Test: Timestamp in ISO8601 format
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        let json = serde_json::to_string(&entry).unwrap();

        // ISO8601/RFC3339 format: 2024-01-15T10:30:00.000000Z
        // Should contain date separator, time separator, and timezone indicator
        let timestamp_pattern = regex::Regex::new(
            r#""timestamp":"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}"#,
        )
        .unwrap();
        assert!(
            timestamp_pattern.is_match(&json),
            "Timestamp should be in ISO8601 format: {}",
            json
        );
    }

    #[test]
    fn test_audit_log_entry_handles_special_characters() {
        // Test: Handles special characters correctly
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "path/with spaces/and\"quotes\".txt".to_string(),
            "GET".to_string(),
            "/bucket/path/with spaces/and\"quotes\".txt".to_string(),
        )
        .with_user_agent(Some("Agent/1.0 (Special; Chars: \"test\")".to_string()));

        let json_result = serde_json::to_string(&entry);
        assert!(
            json_result.is_ok(),
            "Should handle special characters: {:?}",
            json_result
        );

        // Should be able to deserialize back
        let json = json_result.unwrap();
        let deserialized: Result<AuditLogEntry, _> = serde_json::from_str(&json);
        assert!(
            deserialized.is_ok(),
            "Should deserialize successfully: {:?}",
            deserialized
        );
    }

    // ============================================================================
    // Sensitive Data Redaction Tests
    // ============================================================================

    #[test]
    fn test_jwt_tokens_redacted_in_logs() {
        // Test: JWT tokens redacted in logs
        let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

        let redacted = redact_jwt_token(jwt_token);
        assert_eq!(redacted, "[JWT_REDACTED]");

        // Partial JWT should also be redacted
        let partial = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.incomplete";
        let redacted_partial = redact_jwt_token(partial);
        assert_eq!(redacted_partial, "[JWT_REDACTED]");

        // Non-JWT should not be redacted
        let non_jwt = "not-a-jwt-token";
        let not_redacted = redact_jwt_token(non_jwt);
        assert_eq!(not_redacted, non_jwt);
    }

    #[test]
    fn test_authorization_header_redacted() {
        // Test: Authorization header redacted (show "Bearer [REDACTED]")
        let auth_header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

        let redacted = redact_authorization_header(auth_header);
        assert_eq!(redacted, "Bearer [REDACTED]");

        // Basic auth should also be redacted
        let basic_auth = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";
        let redacted_basic = redact_authorization_header(basic_auth);
        assert_eq!(redacted_basic, "Basic [REDACTED]");

        // Empty or invalid should stay as is
        let empty = "";
        assert_eq!(redact_authorization_header(empty), "");
    }

    #[test]
    fn test_query_param_tokens_redacted() {
        // Test: Query param tokens redacted
        let url_with_token = "/api/files?token=secret123&file=doc.pdf";
        let redacted = redact_query_params(url_with_token, &["token", "api_key", "access_token"]);
        assert_eq!(redacted, "/api/files?token=[REDACTED]&file=doc.pdf");

        // Multiple sensitive params
        let url_multi = "/api?token=abc&api_key=xyz&name=test";
        let redacted_multi = redact_query_params(url_multi, &["token", "api_key"]);
        assert_eq!(
            redacted_multi,
            "/api?token=[REDACTED]&api_key=[REDACTED]&name=test"
        );

        // No sensitive params
        let url_clean = "/api/files?file=doc.pdf&page=1";
        let not_redacted = redact_query_params(url_clean, &["token"]);
        assert_eq!(not_redacted, "/api/files?file=doc.pdf&page=1");
    }

    #[test]
    fn test_sensitive_custom_headers_redacted() {
        // Test: Sensitive custom headers redacted
        let headers = vec![
            ("X-API-Key", "secret-api-key-123"),
            ("X-Request-ID", "req-123"),
            ("X-Auth-Token", "auth-token-value"),
            ("Content-Type", "application/json"),
        ];

        let sensitive_headers = ["x-api-key", "x-auth-token"];
        let redacted = redact_headers(&headers, &sensitive_headers);

        // Check that sensitive headers are redacted
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "[REDACTED]"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-Auth-Token" && v == "[REDACTED]"));

        // Check that non-sensitive headers are preserved
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-Request-ID" && v == "req-123"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }
}
