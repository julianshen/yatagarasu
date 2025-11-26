//! OPA (Open Policy Agent) Integration
//!
//! This module provides integration with Open Policy Agent for fine-grained
//! authorization decisions. It includes an HTTP client for communicating
//! with OPA and types for request/response handling.

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Default timeout for OPA requests in milliseconds
const DEFAULT_OPA_TIMEOUT_MS: u64 = 100;

/// Error type for OPA operations
#[derive(Debug)]
pub enum OpaError {
    /// Request to OPA timed out
    Timeout {
        /// The policy path that was being evaluated
        policy_path: String,
        /// The configured timeout in milliseconds
        timeout_ms: u64,
    },
    /// Failed to connect to OPA server
    ConnectionFailed(String),
    /// OPA returned an error (policy evaluation failed)
    PolicyError {
        /// Error message from OPA
        message: String,
    },
    /// OPA response could not be parsed
    InvalidResponse(String),
}

impl fmt::Display for OpaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpaError::Timeout {
                policy_path,
                timeout_ms,
            } => {
                write!(
                    f,
                    "OPA request timed out after {}ms for policy '{}'",
                    timeout_ms, policy_path
                )
            }
            OpaError::ConnectionFailed(msg) => {
                write!(f, "Failed to connect to OPA: {}", msg)
            }
            OpaError::PolicyError { message } => {
                write!(f, "OPA policy error: {}", message)
            }
            OpaError::InvalidResponse(msg) => {
                write!(f, "Invalid OPA response: {}", msg)
            }
        }
    }
}

impl std::error::Error for OpaError {}

/// Fail mode for OPA authorization
///
/// Determines behavior when OPA is unreachable or returns an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailMode {
    /// Fail-open: Allow requests when OPA is unavailable (less secure, higher availability)
    Open,
    /// Fail-closed: Deny requests when OPA is unavailable (more secure, default)
    #[default]
    Closed,
}

impl FromStr for FailMode {
    type Err = Infallible;

    /// Parse fail mode from string
    ///
    /// Returns Closed (deny) for unknown values as a secure default.
    /// This never fails - unknown values default to Closed.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "open" => FailMode::Open,
            _ => FailMode::Closed, // Default to closed for security
        })
    }
}

/// Result of an authorization decision
///
/// Captures the authorization outcome along with any error information
/// for logging and debugging purposes.
#[derive(Debug)]
pub struct AuthorizationDecision {
    /// Whether the request is allowed
    allowed: bool,
    /// Error that occurred during authorization (if any)
    error: Option<OpaError>,
    /// Whether this was a fail-open decision
    fail_open: bool,
    /// Whether authorization was skipped (no OPA config)
    skipped: bool,
}

impl AuthorizationDecision {
    /// Create a decision from an OPA result and fail mode
    pub fn from_opa_result(result: Result<bool, OpaError>, fail_mode: FailMode) -> Self {
        match result {
            Ok(allowed) => AuthorizationDecision {
                allowed,
                error: None,
                fail_open: false,
                skipped: false,
            },
            Err(e) => {
                let allowed = matches!(fail_mode, FailMode::Open);
                AuthorizationDecision {
                    allowed,
                    error: Some(e),
                    fail_open: allowed, // Only fail_open if we're allowing due to error
                    skipped: false,
                }
            }
        }
    }

    /// Create a skipped authorization decision (no OPA configured)
    pub fn skipped() -> Self {
        AuthorizationDecision {
            allowed: true,
            error: None,
            fail_open: false,
            skipped: true,
        }
    }

    /// Check if the request is allowed
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Get the error that occurred (if any)
    pub fn error(&self) -> Option<&OpaError> {
        self.error.as_ref()
    }

    /// Check if this is a fail-open allow (allowed due to OPA error)
    ///
    /// This is useful for logging warnings about fail-open decisions
    pub fn is_fail_open_allow(&self) -> bool {
        self.fail_open
    }

    /// Check if authorization was skipped (no OPA config)
    pub fn is_skipped(&self) -> bool {
        self.skipped
    }
}

/// OPA Client configuration
#[derive(Debug, Clone)]
pub struct OpaClientConfig {
    /// Base URL of the OPA server (e.g., "http://localhost:8181")
    pub url: String,
    /// Path to the policy decision endpoint (e.g., "authz/allow")
    pub policy_path: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Cache TTL in seconds for authorization decisions
    pub cache_ttl_seconds: u64,
}

impl OpaClientConfig {
    /// Get the default timeout value in milliseconds
    ///
    /// Default is 100ms for fast fail behavior in authorization paths
    pub fn default_timeout() -> u64 {
        DEFAULT_OPA_TIMEOUT_MS
    }
}

/// OPA HTTP Client for policy evaluation
///
/// This client is Send + Sync and can be safely shared across threads.
pub struct OpaClient {
    config: OpaClientConfig,
    // HTTP client will be added when we implement actual HTTP calls
    // For now, we just store the config
}

impl OpaClient {
    /// Create a new OPA client with the given configuration
    pub fn new(config: OpaClientConfig) -> Self {
        Self { config }
    }

    /// Get the client configuration
    pub fn config(&self) -> &OpaClientConfig {
        &self.config
    }

    /// Get the full URL for the policy evaluation endpoint
    ///
    /// Returns: `{base_url}/v1/data/{policy_path}`
    pub fn policy_endpoint(&self) -> String {
        format!("{}/v1/data/{}", self.config.url, self.config.policy_path)
    }
}

// Ensure OpaClient is Send + Sync
unsafe impl Send for OpaClient {}
unsafe impl Sync for OpaClient {}

/// Shared OPA client (thread-safe)
pub type SharedOpaClient = Arc<OpaClient>;

/// Input data sent to OPA for policy evaluation
///
/// This structure matches the OPA REST API input format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaInput {
    /// JWT claims from the authenticated token
    jwt_claims: JsonValue,
    /// Name of the bucket being accessed
    bucket: String,
    /// Request path within the bucket
    path: String,
    /// HTTP method (GET, HEAD)
    method: String,
    /// Client IP address (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    client_ip: Option<String>,
}

impl OpaInput {
    /// Create a new OPA input for policy evaluation
    pub fn new(
        jwt_claims: JsonValue,
        bucket: String,
        path: String,
        method: String,
        client_ip: Option<String>,
    ) -> Self {
        Self {
            jwt_claims,
            bucket,
            path,
            method,
            client_ip,
        }
    }

    /// Get the JWT claims
    pub fn jwt_claims(&self) -> &JsonValue {
        &self.jwt_claims
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the HTTP method
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the client IP address
    pub fn client_ip(&self) -> Option<&str> {
        self.client_ip.as_deref()
    }

    /// Generate a deterministic cache key based on input content
    ///
    /// The cache key is a SHA-256 hash of the serialized input, ensuring:
    /// - Same inputs always produce the same key
    /// - Different inputs produce different keys
    /// - The key is a fixed-length hex string
    pub fn cache_key(&self) -> String {
        // Serialize to canonical JSON for deterministic hashing
        let json = serde_json::to_string(self).unwrap_or_default();

        // Hash the JSON content
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let hash = hasher.finalize();

        // Convert to hex string
        hex::encode(hash)
    }
}

/// Cache for OPA authorization decisions
///
/// This cache stores authorization decisions (allow/deny) keyed by
/// a hash of the OpaInput. It uses moka for efficient concurrent access
/// with automatic TTL-based expiration.
pub struct OpaCache {
    cache: Cache<String, bool>,
}

impl OpaCache {
    /// Create a new OPA cache with the specified TTL in seconds
    pub fn new(ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(ttl_seconds))
            .max_capacity(10_000) // Default max entries
            .build();

        Self { cache }
    }

    /// Get a cached authorization decision (async)
    ///
    /// Returns Some(true) if allowed, Some(false) if denied, None if not cached
    pub async fn get(&self, key: &str) -> Option<bool> {
        self.cache.get(key).await
    }

    /// Store an authorization decision in the cache (async)
    pub async fn put(&self, key: String, allowed: bool) {
        self.cache.insert(key, allowed).await;
    }

    /// Check if a key exists in the cache
    pub fn contains(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.entry_count() == 0
    }
}

/// Request wrapper for OPA REST API
///
/// OPA expects requests in the format: `{ "input": { ... } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaRequest {
    /// The input data for policy evaluation
    input: OpaInput,
}

impl OpaRequest {
    /// Create a new OPA request wrapping the given input
    pub fn new(input: OpaInput) -> Self {
        Self { input }
    }

    /// Get the input data
    pub fn input(&self) -> &OpaInput {
        &self.input
    }
}

/// Response from OPA policy evaluation
///
/// Handles multiple OPA response formats:
/// - Simple: `{ "result": true }` or `{ "result": false }`
/// - Detailed: `{ "result": { "allow": true, "reason": "..." } }`
#[derive(Debug, Clone)]
pub struct OpaResponse {
    /// Whether the request is allowed
    allow: bool,
    /// Optional reason for the decision
    reason: Option<String>,
}

impl OpaResponse {
    /// Check if the request is allowed
    pub fn is_allowed(&self) -> bool {
        self.allow
    }

    /// Get the reason for the decision (if provided)
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

impl<'de> Deserialize<'de> for OpaResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawResponse {
            result: JsonValue,
        }

        let raw = RawResponse::deserialize(deserializer)?;

        match &raw.result {
            // Simple boolean response: { "result": true/false }
            JsonValue::Bool(allow) => Ok(OpaResponse {
                allow: *allow,
                reason: None,
            }),
            // Detailed object response: { "result": { "allow": true, "reason": "..." } }
            JsonValue::Object(obj) => {
                let allow = obj.get("allow").and_then(|v| v.as_bool()).unwrap_or(false);
                let reason = obj
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(OpaResponse { allow, reason })
            }
            // Undefined or null result means deny
            _ => Ok(OpaResponse {
                allow: false,
                reason: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opa_client_config_fields() {
        let config = OpaClientConfig {
            url: "http://localhost:8181".to_string(),
            policy_path: "authz/allow".to_string(),
            timeout_ms: 5000,
            cache_ttl_seconds: 60,
        };

        assert_eq!(config.url, "http://localhost:8181");
        assert_eq!(config.policy_path, "authz/allow");
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.cache_ttl_seconds, 60);
    }

    #[test]
    fn test_opa_input_serialization() {
        let input = OpaInput::new(
            serde_json::json!({"sub": "user1"}),
            "bucket".to_string(),
            "/path".to_string(),
            "GET".to_string(),
            Some("1.2.3.4".to_string()),
        );

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"bucket\":\"bucket\""));
        assert!(json.contains("\"path\":\"/path\""));
        assert!(json.contains("\"method\":\"GET\""));
        assert!(json.contains("\"client_ip\":\"1.2.3.4\""));
    }

    #[test]
    fn test_opa_request_wraps_input() {
        let input = OpaInput::new(
            serde_json::json!({}),
            "bucket".to_string(),
            "/path".to_string(),
            "GET".to_string(),
            None,
        );

        let request = OpaRequest::new(input);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"input\":{"));
    }

    #[test]
    fn test_opa_response_simple_true() {
        let json = r#"{"result": true}"#;
        let response: OpaResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_allowed());
        assert!(response.reason().is_none());
    }

    #[test]
    fn test_opa_response_simple_false() {
        let json = r#"{"result": false}"#;
        let response: OpaResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_allowed());
    }

    #[test]
    fn test_opa_response_detailed_with_reason() {
        let json = r#"{"result": {"allow": true, "reason": "Admin access"}}"#;
        let response: OpaResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_allowed());
        assert_eq!(response.reason(), Some("Admin access"));
    }

    #[test]
    fn test_opa_response_undefined_is_deny() {
        let json = r#"{"result": null}"#;
        let response: OpaResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_allowed());
    }
}
