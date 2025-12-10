//! JWT configuration types for token authentication.
//!
//! This module defines JWT authentication configuration including:
//! - Single-key and multi-key (rotation) support
//! - HMAC (HS256/384/512) and asymmetric (RS256, ES256) algorithms
//! - Token extraction from headers, query params, or custom sources
//! - Claim validation rules with various operators
//! - JWKS (JSON Web Key Set) URL support for dynamic key fetching
//!
//! # Key Resolution Order
//!
//! When validating JWTs, keys are resolved in the following order of precedence:
//!
//! 1. **JWKS URL** (`jwks_url`) - Dynamic keys fetched from a JSON Web Key Set endpoint.
//!    The `kid` (key ID) from the JWT header is used to select the appropriate key.
//!
//! 2. **Keys array** (`keys`) - Static multi-key configuration for key rotation.
//!    Keys are matched by their `id` field against the JWT's `kid` header claim.
//!
//! 3. **Legacy single-key fields** - For simple single-key configurations:
//!    - `secret` - HMAC secret for HS256/HS384/HS512 algorithms
//!    - `rsa_public_key_path` - RSA public key PEM file for RS256/RS384/RS512
//!    - `ecdsa_public_key_path` - ECDSA public key PEM file for ES256/ES384
//!
//! Multiple key sources can be configured simultaneously; the first matching key wins.
//!
//! # Validation Requirements
//!
//! When `enabled: true`, the following are required:
//! - `secret` must be non-empty for HMAC algorithms (validated in [`Config::validate()`])
//! - At least one `token_sources` entry must be configured
//! - `algorithm` must be a supported algorithm: HS256, HS384, HS512
//!
//! [`Config::validate()`]: super::Config::validate

use serde::{Deserialize, Serialize};

/// Individual JWT key configuration for multi-key support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtKey {
    /// Key identifier (kid) - used for key selection
    pub id: String,
    /// Algorithm for this key (HS256, RS256, ES256, etc.)
    pub algorithm: String,
    /// Secret for HS256/HS384/HS512 algorithms
    #[serde(default)]
    pub secret: Option<String>,
    /// Path to public key PEM file for RS256/RS384/RS512/ES256/ES384 algorithms
    #[serde(default)]
    pub path: Option<String>,
}

/// JWT authentication configuration.
///
/// See the [module-level documentation](self) for key resolution order and validation requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// Whether JWT authentication is enabled for this configuration.
    pub enabled: bool,
    /// Secret for HS256/HS384/HS512 algorithms (legacy single-key mode).
    ///
    /// **Note**: This field defaults to an empty string for deserialization,
    /// but [`Config::validate()`](super::Config::validate) will reject empty secrets
    /// when `enabled: true`. Always provide a non-empty secret for HMAC algorithms.
    #[serde(default)]
    pub secret: String,
    /// Algorithm for single-key configuration.
    ///
    /// Required field. Supported values: `HS256`, `HS384`, `HS512`.
    /// For RSA/ECDSA, set this to the expected algorithm (RS256, ES256, etc.)
    /// and provide the corresponding key path.
    pub algorithm: String,
    /// Path to RSA public key PEM file for RS256/RS384/RS512 algorithms
    #[serde(default)]
    pub rsa_public_key_path: Option<String>,
    /// Path to ECDSA public key PEM file for ES256/ES384 algorithms
    #[serde(default)]
    pub ecdsa_public_key_path: Option<String>,
    #[serde(default)]
    pub token_sources: Vec<TokenSource>,
    #[serde(default)]
    pub claims: Vec<ClaimRule>,
    /// Admin claim rules for cache management API access (Phase 65.1)
    /// These claims are checked for /admin/* endpoints
    #[serde(default)]
    pub admin_claims: Vec<ClaimRule>,
    /// Multiple keys for key rotation support
    #[serde(default)]
    pub keys: Vec<JwtKey>,
    /// URL to fetch JWKS (JSON Web Key Set) for dynamic key validation
    #[serde(default)]
    pub jwks_url: Option<String>,
    /// JWKS cache refresh interval in seconds (default: 3600 = 1 hour)
    #[serde(default)]
    pub jwks_refresh_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRule {
    pub claim: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_key_deserialize() {
        let yaml = r#"
id: "key-1"
algorithm: "HS256"
secret: "my-secret"
"#;
        let key: JwtKey = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(key.id, "key-1");
        assert_eq!(key.algorithm, "HS256");
        assert_eq!(key.secret, Some("my-secret".to_string()));
        assert!(key.path.is_none());
    }

    #[test]
    fn test_jwt_key_with_path() {
        let yaml = r#"
id: "rsa-key"
algorithm: "RS256"
path: "/etc/keys/public.pem"
"#;
        let key: JwtKey = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(key.id, "rsa-key");
        assert_eq!(key.algorithm, "RS256");
        assert!(key.secret.is_none());
        assert_eq!(key.path, Some("/etc/keys/public.pem".to_string()));
    }

    #[test]
    fn test_token_source_bearer() {
        let yaml = r#"
type: bearer
"#;
        let source: TokenSource = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(source.source_type, "bearer");
        assert!(source.name.is_none());
        assert!(source.prefix.is_none());
    }

    #[test]
    fn test_token_source_header() {
        let yaml = r#"
type: header
name: "X-Auth-Token"
"#;
        let source: TokenSource = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(source.source_type, "header");
        assert_eq!(source.name, Some("X-Auth-Token".to_string()));
    }

    #[test]
    fn test_token_source_query() {
        let yaml = r#"
type: query
name: "token"
"#;
        let source: TokenSource = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(source.source_type, "query");
        assert_eq!(source.name, Some("token".to_string()));
    }

    #[test]
    fn test_claim_rule_deserialize() {
        let yaml = r#"
claim: "role"
operator: "equals"
value: "admin"
"#;
        let rule: ClaimRule = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(rule.claim, "role");
        assert_eq!(rule.operator, "equals");
        assert_eq!(rule.value, serde_json::json!("admin"));
    }

    #[test]
    fn test_claim_rule_with_array_value() {
        let yaml = r#"
claim: "roles"
operator: "in"
value: ["admin", "user"]
"#;
        let rule: ClaimRule = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(rule.claim, "roles");
        assert_eq!(rule.operator, "in");
        assert_eq!(rule.value, serde_json::json!(["admin", "user"]));
    }

    #[test]
    fn test_jwt_config_minimal() {
        let yaml = r#"
enabled: true
algorithm: "HS256"
secret: "my-secret"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert_eq!(config.algorithm, "HS256");
        assert_eq!(config.secret, "my-secret");
        assert!(config.token_sources.is_empty());
        assert!(config.claims.is_empty());
        assert!(config.keys.is_empty());
    }

    #[test]
    fn test_jwt_config_with_token_sources() {
        let yaml = r#"
enabled: true
algorithm: "HS256"
secret: "my-secret"
token_sources:
  - type: bearer
  - type: header
    name: "X-Auth-Token"
  - type: query
    name: "token"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.token_sources.len(), 3);
        assert_eq!(config.token_sources[0].source_type, "bearer");
        assert_eq!(config.token_sources[1].source_type, "header");
        assert_eq!(config.token_sources[2].source_type, "query");
    }

    #[test]
    fn test_jwt_config_with_claims() {
        let yaml = r#"
enabled: true
algorithm: "HS256"
secret: "my-secret"
claims:
  - claim: "role"
    operator: "equals"
    value: "admin"
  - claim: "scope"
    operator: "contains"
    value: "read"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.claims.len(), 2);
        assert_eq!(config.claims[0].claim, "role");
        assert_eq!(config.claims[1].claim, "scope");
    }

    #[test]
    fn test_jwt_config_with_admin_claims() {
        let yaml = r#"
enabled: true
algorithm: "HS256"
secret: "my-secret"
admin_claims:
  - claim: "role"
    operator: "equals"
    value: "superadmin"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.admin_claims.len(), 1);
        assert_eq!(config.admin_claims[0].claim, "role");
        assert_eq!(
            config.admin_claims[0].value,
            serde_json::json!("superadmin")
        );
    }

    #[test]
    fn test_jwt_config_with_multiple_keys() {
        let yaml = r#"
enabled: true
algorithm: "HS256"
secret: "default-secret"
keys:
  - id: "key-1"
    algorithm: "HS256"
    secret: "secret-1"
  - id: "key-2"
    algorithm: "RS256"
    path: "/etc/keys/key2.pem"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.keys.len(), 2);
        assert_eq!(config.keys[0].id, "key-1");
        assert_eq!(config.keys[1].id, "key-2");
    }

    #[test]
    fn test_jwt_config_with_jwks() {
        let yaml = r#"
enabled: true
algorithm: "RS256"
jwks_url: "https://auth.example.com/.well-known/jwks.json"
jwks_refresh_interval_secs: 1800
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.jwks_url,
            Some("https://auth.example.com/.well-known/jwks.json".to_string())
        );
        assert_eq!(config.jwks_refresh_interval_secs, Some(1800));
    }

    #[test]
    fn test_jwt_config_rsa_key_path() {
        let yaml = r#"
enabled: true
algorithm: "RS256"
rsa_public_key_path: "/etc/keys/rsa_public.pem"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.rsa_public_key_path,
            Some("/etc/keys/rsa_public.pem".to_string())
        );
    }

    #[test]
    fn test_jwt_config_ecdsa_key_path() {
        let yaml = r#"
enabled: true
algorithm: "ES256"
ecdsa_public_key_path: "/etc/keys/ec_public.pem"
"#;
        let config: JwtConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(
            config.ecdsa_public_key_path,
            Some("/etc/keys/ec_public.pem".to_string())
        );
    }
}
