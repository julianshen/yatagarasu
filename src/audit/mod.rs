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
}
