//! Authorization configuration types for external policy engines.
//!
//! This module defines bucket-level authorization configuration supporting:
//! - Open Policy Agent (OPA) integration for policy-as-code
//! - OpenFGA integration for relationship-based access control (ReBAC)
//!
//! Both integrations support configurable timeouts, caching, and fail modes.
//! Default values for timeouts and cache TTLs are sourced from `crate::constants`.

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_OPA_CACHE_TTL_SECS, DEFAULT_OPA_TIMEOUT_MS, DEFAULT_OPENFGA_CACHE_TTL_SECS,
    DEFAULT_OPENFGA_TIMEOUT_MS,
};

/// Default OPA timeout in milliseconds
fn default_opa_timeout_ms() -> u64 {
    DEFAULT_OPA_TIMEOUT_MS
}

/// Default OPA cache TTL in seconds
fn default_opa_cache_ttl_seconds() -> u64 {
    DEFAULT_OPA_CACHE_TTL_SECS
}

/// Default OpenFGA timeout in milliseconds
fn default_openfga_timeout_ms() -> u64 {
    DEFAULT_OPENFGA_TIMEOUT_MS
}

/// Default OpenFGA cache TTL in seconds
fn default_openfga_cache_ttl_seconds() -> u64 {
    DEFAULT_OPENFGA_CACHE_TTL_SECS
}

/// Authorization configuration for bucket-level access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization type (e.g., "opa" for Open Policy Agent)
    #[serde(rename = "type")]
    pub auth_type: String,

    /// OPA REST API endpoint URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opa_url: Option<String>,

    /// OPA policy path (e.g., "yatagarasu/authz/allow")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opa_policy_path: Option<String>,

    /// Timeout for OPA requests in milliseconds (default: 100ms)
    #[serde(default = "default_opa_timeout_ms")]
    pub opa_timeout_ms: u64,

    /// Cache TTL for OPA decisions in seconds (default: 60s)
    #[serde(default = "default_opa_cache_ttl_seconds")]
    pub opa_cache_ttl_seconds: u64,

    /// Fail mode: "open" (allow on OPA failure) or "closed" (deny on failure, default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opa_fail_mode: Option<String>,

    // OpenFGA configuration fields
    /// OpenFGA server endpoint URL (e.g., "http://localhost:8080")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openfga_endpoint: Option<String>,

    /// OpenFGA store ID (ULID format, e.g., "01ARZ3NDEKTSV4RRFFQ69G5FAV")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openfga_store_id: Option<String>,

    /// OpenFGA authorization model ID (optional, ULID format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openfga_authorization_model_id: Option<String>,

    /// OpenFGA API token for cloud deployments (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openfga_api_token: Option<String>,

    /// Timeout for OpenFGA requests in milliseconds (default: 100ms)
    #[serde(default = "default_openfga_timeout_ms")]
    pub openfga_timeout_ms: u64,

    /// Cache TTL for OpenFGA decisions in seconds (default: 60s)
    #[serde(default = "default_openfga_cache_ttl_seconds")]
    pub openfga_cache_ttl_seconds: u64,

    /// Fail mode: "open" (allow on OpenFGA failure) or "closed" (deny on failure, default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openfga_fail_mode: Option<String>,

    /// JWT claim to use for user ID extraction in OpenFGA (default: "sub")
    /// Supports dot notation for nested claims (e.g., "user.id")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openfga_user_claim: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_config_opa_minimal() {
        let yaml = r#"
type: opa
opa_url: "http://localhost:8181"
opa_policy_path: "yatagarasu/authz/allow"
"#;
        let config: AuthorizationConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.auth_type, "opa");
        assert_eq!(config.opa_url, Some("http://localhost:8181".to_string()));
        assert_eq!(
            config.opa_policy_path,
            Some("yatagarasu/authz/allow".to_string())
        );
        assert_eq!(config.opa_timeout_ms, DEFAULT_OPA_TIMEOUT_MS);
        assert_eq!(config.opa_cache_ttl_seconds, DEFAULT_OPA_CACHE_TTL_SECS);
        assert!(config.opa_fail_mode.is_none());
    }

    #[test]
    fn test_authorization_config_opa_full() {
        let yaml = r#"
type: opa
opa_url: "http://opa.example.com:8181"
opa_policy_path: "custom/policy/allow"
opa_timeout_ms: 200
opa_cache_ttl_seconds: 120
opa_fail_mode: "open"
"#;
        let config: AuthorizationConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.auth_type, "opa");
        assert_eq!(config.opa_timeout_ms, 200);
        assert_eq!(config.opa_cache_ttl_seconds, 120);
        assert_eq!(config.opa_fail_mode, Some("open".to_string()));
    }

    #[test]
    fn test_authorization_config_openfga_minimal() {
        let yaml = r#"
type: openfga
openfga_endpoint: "http://localhost:8080"
openfga_store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"
"#;
        let config: AuthorizationConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.auth_type, "openfga");
        assert_eq!(
            config.openfga_endpoint,
            Some("http://localhost:8080".to_string())
        );
        assert_eq!(
            config.openfga_store_id,
            Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string())
        );
        assert_eq!(config.openfga_timeout_ms, DEFAULT_OPENFGA_TIMEOUT_MS);
        assert_eq!(
            config.openfga_cache_ttl_seconds,
            DEFAULT_OPENFGA_CACHE_TTL_SECS
        );
    }

    #[test]
    fn test_authorization_config_openfga_full() {
        let yaml = r#"
type: openfga
openfga_endpoint: "https://api.openfga.example.com"
openfga_store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"
openfga_authorization_model_id: "01HXJR6WBQD3ABCDEFGHIJKLMN"
openfga_api_token: "secret-token"
openfga_timeout_ms: 150
openfga_cache_ttl_seconds: 300
openfga_fail_mode: "closed"
openfga_user_claim: "user.id"
"#;
        let config: AuthorizationConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.auth_type, "openfga");
        assert_eq!(
            config.openfga_authorization_model_id,
            Some("01HXJR6WBQD3ABCDEFGHIJKLMN".to_string())
        );
        assert_eq!(config.openfga_api_token, Some("secret-token".to_string()));
        assert_eq!(config.openfga_timeout_ms, 150);
        assert_eq!(config.openfga_cache_ttl_seconds, 300);
        assert_eq!(config.openfga_fail_mode, Some("closed".to_string()));
        assert_eq!(config.openfga_user_claim, Some("user.id".to_string()));
    }

    #[test]
    fn test_authorization_config_defaults() {
        let yaml = r#"
type: opa
"#;
        let config: AuthorizationConfig = serde_yaml::from_str(yaml).unwrap();

        // OPA defaults
        assert_eq!(config.opa_timeout_ms, DEFAULT_OPA_TIMEOUT_MS);
        assert_eq!(config.opa_cache_ttl_seconds, DEFAULT_OPA_CACHE_TTL_SECS);

        // OpenFGA defaults
        assert_eq!(config.openfga_timeout_ms, DEFAULT_OPENFGA_TIMEOUT_MS);
        assert_eq!(
            config.openfga_cache_ttl_seconds,
            DEFAULT_OPENFGA_CACHE_TTL_SECS
        );
    }
}
