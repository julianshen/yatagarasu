//! OPA (Open Policy Agent) Integration
//!
//! This module provides integration with Open Policy Agent for fine-grained
//! authorization decisions. It includes an HTTP client for communicating
//! with OPA and types for request/response handling.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

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
