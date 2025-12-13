//! JWT Authentication Module
//!
//! Provides flexible JWT token validation with support for multiple algorithms,
//! key sources, and custom claim verification.
//!
//! # Features
//!
//! - **Multiple token sources**: Extract tokens from Authorization header, custom headers, or query parameters
//! - **Algorithm support**: HS256, HS384, HS512 (HMAC), RS256/384/512 (RSA), ES256/384 (ECDSA)
//! - **Key management**: Static secrets, PEM files, or dynamic JWKS endpoints
//! - **Custom claim rules**: Verify claims with the `equals` operator
//! - **Admin claim support**: Separate claims for admin access verification
//!
//! # Token Sources
//!
//! Tokens can be extracted from (configured per bucket):
//! - `bearer`: Standard `Authorization: Bearer <token>` header
//! - `header`: Custom header with optional prefix (e.g., `X-Auth-Token`)
//! - `query`: Query parameter (e.g., `?token=<jwt>`)
//!
//! # Example Configuration
//!
//! ```yaml
//! jwt:
//!   enabled: true
//!   algorithm: RS256
//!   keys:
//!     - id: primary
//!       algorithm: RS256
//!       path: /etc/jwt/public.pem
//!   token_sources:
//!     - source_type: bearer
//!   claims:
//!     - claim: role
//!       operator: in
//!       value: ["admin", "user"]
//! ```

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::config::{ClaimRule, JwtConfig};

pub mod jwks;
pub mod jwks_client;

// Re-export JWKS client types for convenience
pub use jwks_client::{JwksClient, JwksClientConfig, JwksClientError, SharedJwksClient};

/// Error type for key loading operations
#[derive(Debug)]
pub enum KeyLoadError {
    FileNotFound(String),
    InvalidKeyFormat(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for KeyLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyLoadError::FileNotFound(path) => {
                write!(f, "Key file not found: {}", path)
            }
            KeyLoadError::InvalidKeyFormat(reason) => {
                write!(f, "Invalid key format: {}", reason)
            }
            KeyLoadError::IoError(err) => {
                write!(f, "IO error reading key file: {}", err)
            }
        }
    }
}

impl std::error::Error for KeyLoadError {}

impl From<std::io::Error> for KeyLoadError {
    fn from(err: std::io::Error) -> Self {
        KeyLoadError::IoError(err)
    }
}

/// Load RSA public key from PEM file
pub fn load_rsa_public_key(path: &str) -> Result<DecodingKey, KeyLoadError> {
    let key_path = Path::new(path);
    if !key_path.exists() {
        return Err(KeyLoadError::FileNotFound(path.to_string()));
    }

    let pem_content = fs::read(key_path)?;

    DecodingKey::from_rsa_pem(&pem_content)
        .map_err(|e| KeyLoadError::InvalidKeyFormat(format!("Invalid RSA PEM format: {}", e)))
}

/// Load ECDSA public key from PEM file
pub fn load_ecdsa_public_key(path: &str) -> Result<DecodingKey, KeyLoadError> {
    let key_path = Path::new(path);
    if !key_path.exists() {
        return Err(KeyLoadError::FileNotFound(path.to_string()));
    }

    let pem_content = fs::read(key_path)?;

    DecodingKey::from_ec_pem(&pem_content)
        .map_err(|e| KeyLoadError::InvalidKeyFormat(format!("Invalid ECDSA PEM format: {}", e)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Option<String>,
    pub exp: Option<u64>,
    pub iat: Option<u64>,
    pub nbf: Option<u64>,
    pub iss: Option<String>,
    #[serde(flatten)]
    pub custom: serde_json::Map<String, serde_json::Value>,
}

// Helper function to get header value with case-insensitive matching
fn get_header_case_insensitive(
    headers: &HashMap<String, String>,
    header_name: &str,
) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(header_name))
        .map(|(_, value)| value.to_string())
}

pub fn extract_bearer_token(headers: &HashMap<String, String>) -> Option<String> {
    get_header_case_insensitive(headers, "Authorization").and_then(|value| {
        value.strip_prefix("Bearer ").and_then(|s| {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
    })
}

pub fn extract_header_token(
    headers: &HashMap<String, String>,
    header_name: &str,
) -> Option<String> {
    get_header_case_insensitive(headers, header_name).and_then(|s| {
        let trimmed = s.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

pub fn extract_query_token(
    query_params: &HashMap<String, String>,
    param_name: &str,
) -> Option<String> {
    query_params.get(param_name).and_then(|s| {
        let trimmed = s.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

pub fn try_extract_token(
    headers: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
    sources: &[crate::config::TokenSource],
) -> Option<String> {
    for source in sources {
        tracing::debug!(
            "Attempting to extract JWT from source type: {}",
            source.source_type
        );

        let token = match source.source_type.as_str() {
            "bearer" => extract_bearer_token(headers),
            "header" => {
                if let Some(ref header_name) = source.name {
                    if let Some(value) = extract_header_token(headers, header_name) {
                        // Strip prefix if configured
                        if let Some(ref prefix) = source.prefix {
                            value.strip_prefix(prefix).map(|s| s.trim().to_string())
                        } else {
                            Some(value)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "query" => {
                if let Some(ref param_name) = source.name {
                    extract_query_token(query_params, param_name)
                } else {
                    None
                }
            }
            _ => {
                tracing::warn!(
                    "Unknown token source type '{}' - this should have been caught by config validation",
                    source.source_type
                );
                None
            }
        };

        if let Some(ref token_value) = token {
            tracing::debug!(
                "Successfully extracted JWT token from source type '{}' (length: {} chars)",
                source.source_type,
                token_value.len()
            );
            return token;
        } else {
            tracing::debug!("No token found in source type '{}'", source.source_type);
        }
    }

    tracing::debug!("JWT token not found in any configured source");
    None
}

/// Parse algorithm string to Algorithm enum
pub fn parse_algorithm(algorithm: &str) -> Algorithm {
    match algorithm {
        "HS256" => Algorithm::HS256,
        "HS384" => Algorithm::HS384,
        "HS512" => Algorithm::HS512,
        "RS256" => Algorithm::RS256,
        "RS384" => Algorithm::RS384,
        "RS512" => Algorithm::RS512,
        "ES256" => Algorithm::ES256,
        "ES384" => Algorithm::ES384,
        _ => Algorithm::HS256, // Default to HS256 for backward compatibility
    }
}

/// Validate JWT with HMAC secret (HS256, HS384, HS512)
pub fn validate_jwt(
    token: &str,
    secret: &str,
    algorithm: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let algo = parse_algorithm(algorithm);

    let mut validation = Validation::new(algo);
    validation.validate_exp = true; // Validate expiration if present
    validation.validate_nbf = true; // Validate not-before if present
    validation.required_spec_claims.clear(); // Don't require exp, nbf, etc. (but validate if present)

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Validate JWT with a DecodingKey (for RS256, ES256, etc.)
pub fn validate_jwt_with_key(
    token: &str,
    key: &DecodingKey,
    algorithm: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let algo = parse_algorithm(algorithm);

    let mut validation = Validation::new(algo);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.required_spec_claims.clear();

    let token_data = decode::<Claims>(token, key, &validation)?;

    Ok(token_data.claims)
}

/// Extract kid (Key ID) from JWT header without validating
pub fn extract_kid_from_token(token: &str) -> Option<String> {
    use jsonwebtoken::decode_header;

    match decode_header(token) {
        Ok(header) => header.kid,
        Err(e) => {
            tracing::debug!("Failed to decode JWT header: {}", e);
            None
        }
    }
}

/// Validate JWT against multiple configured keys
/// Tries each key in order until one successfully validates the token.
/// Returns the validated claims and the key ID that succeeded.
pub fn validate_jwt_with_keys(
    token: &str,
    keys: &[crate::config::JwtKey],
) -> Result<(Claims, String), AuthError> {
    if keys.is_empty() {
        return Err(AuthError::InvalidToken(
            "No validation keys configured".to_string(),
        ));
    }

    let mut last_error = None;

    for key_config in keys {
        let result = match key_config.algorithm.as_str() {
            "HS256" | "HS384" | "HS512" => {
                if let Some(ref secret) = key_config.secret {
                    validate_jwt(token, secret, &key_config.algorithm)
                } else {
                    tracing::debug!(
                        "Key '{}' has no secret configured for HMAC algorithm",
                        key_config.id
                    );
                    continue;
                }
            }
            "RS256" | "RS384" | "RS512" => {
                if let Some(ref path) = key_config.path {
                    match load_rsa_public_key(path) {
                        Ok(decoding_key) => {
                            validate_jwt_with_key(token, &decoding_key, &key_config.algorithm)
                        }
                        Err(e) => {
                            tracing::debug!("Failed to load RSA key '{}': {}", key_config.id, e);
                            continue;
                        }
                    }
                } else {
                    tracing::debug!(
                        "Key '{}' has no path configured for RSA algorithm",
                        key_config.id
                    );
                    continue;
                }
            }
            "ES256" | "ES384" => {
                if let Some(ref path) = key_config.path {
                    match load_ecdsa_public_key(path) {
                        Ok(decoding_key) => {
                            validate_jwt_with_key(token, &decoding_key, &key_config.algorithm)
                        }
                        Err(e) => {
                            tracing::debug!("Failed to load ECDSA key '{}': {}", key_config.id, e);
                            continue;
                        }
                    }
                } else {
                    tracing::debug!(
                        "Key '{}' has no path configured for ECDSA algorithm",
                        key_config.id
                    );
                    continue;
                }
            }
            _ => {
                tracing::debug!(
                    "Unsupported algorithm '{}' for key '{}'",
                    key_config.algorithm,
                    key_config.id
                );
                continue;
            }
        };

        match result {
            Ok(claims) => {
                tracing::debug!("JWT validated successfully with key '{}'", key_config.id);
                return Ok((claims, key_config.id.clone()));
            }
            Err(e) => {
                tracing::debug!("Key '{}' failed to validate: {}", key_config.id, e);
                last_error = Some(e);
            }
        }
    }

    Err(AuthError::InvalidToken(format!(
        "Token validation failed with all {} configured keys: {}",
        keys.len(),
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "No keys could be used".to_string())
    )))
}

/// Validate JWT with kid (Key ID) header support
/// First tries to find a key matching the kid header, if present.
/// If no kid header or no matching key, falls back to trying all keys.
pub fn validate_jwt_with_keys_and_kid(
    token: &str,
    keys: &[crate::config::JwtKey],
) -> Result<(Claims, String), AuthError> {
    if keys.is_empty() {
        return Err(AuthError::InvalidToken(
            "No validation keys configured".to_string(),
        ));
    }

    // Try to extract kid from token header
    if let Some(kid) = extract_kid_from_token(token) {
        tracing::debug!("JWT has kid header: {}", kid);

        // Find key matching the kid
        if let Some(key_config) = keys.iter().find(|k| k.id == kid) {
            tracing::debug!("Found key matching kid '{}'", kid);

            // Validate with the matching key
            let result = validate_single_key(token, key_config);
            return match result {
                Ok(claims) => Ok((claims, kid)),
                Err(e) => Err(AuthError::InvalidToken(format!(
                    "Token validation failed with key '{}': {}",
                    kid, e
                ))),
            };
        } else {
            tracing::debug!("No key found matching kid '{}', returning error", kid);
            return Err(AuthError::InvalidToken(format!(
                "No key configured with id '{}'",
                kid
            )));
        }
    }

    tracing::debug!("No kid header in JWT, falling back to trying all keys");
    validate_jwt_with_keys(token, keys)
}

/// Validate a token with a single key configuration
fn validate_single_key(
    token: &str,
    key_config: &crate::config::JwtKey,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    match key_config.algorithm.as_str() {
        "HS256" | "HS384" | "HS512" => {
            if let Some(ref secret) = key_config.secret {
                validate_jwt(token, secret, &key_config.algorithm)
            } else {
                Err(jsonwebtoken::errors::Error::from(
                    jsonwebtoken::errors::ErrorKind::InvalidSignature,
                ))
            }
        }
        "RS256" | "RS384" | "RS512" => {
            if let Some(ref path) = key_config.path {
                let decoding_key = load_rsa_public_key(path).map_err(|_| {
                    jsonwebtoken::errors::Error::from(
                        jsonwebtoken::errors::ErrorKind::InvalidRsaKey("Failed to load key".into()),
                    )
                })?;
                validate_jwt_with_key(token, &decoding_key, &key_config.algorithm)
            } else {
                Err(jsonwebtoken::errors::Error::from(
                    jsonwebtoken::errors::ErrorKind::InvalidSignature,
                ))
            }
        }
        "ES256" | "ES384" => {
            if let Some(ref path) = key_config.path {
                let decoding_key = load_ecdsa_public_key(path).map_err(|_| {
                    jsonwebtoken::errors::Error::from(
                        jsonwebtoken::errors::ErrorKind::InvalidEcdsaKey,
                    )
                })?;
                validate_jwt_with_key(token, &decoding_key, &key_config.algorithm)
            } else {
                Err(jsonwebtoken::errors::Error::from(
                    jsonwebtoken::errors::ErrorKind::InvalidSignature,
                ))
            }
        }
        _ => Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidAlgorithm,
        )),
    }
}

pub fn verify_claims(claims: &Claims, rules: &[ClaimRule]) -> bool {
    for rule in rules {
        let claim_value = claims.custom.get(&rule.claim);

        let matches = match rule.operator.as_str() {
            "equals" => {
                // Exact match
                claim_value == Some(&rule.value)
            }
            "in" => {
                // Check if claim value is in the rule's array
                if let Some(claim_val) = claim_value {
                    if let Some(allowed_values) = rule.value.as_array() {
                        allowed_values.contains(claim_val)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "contains" => {
                // Check if claim string contains the rule value substring
                if let (Some(claim_val), Some(search_str)) =
                    (claim_value.and_then(|v| v.as_str()), rule.value.as_str())
                {
                    claim_val.contains(search_str)
                } else {
                    false
                }
            }
            "gt" => {
                // Greater than (numeric)
                compare_numeric(claim_value, &rule.value, |a, b| a > b)
            }
            "lt" => {
                // Less than (numeric)
                compare_numeric(claim_value, &rule.value, |a, b| a < b)
            }
            "gte" => {
                // Greater than or equal (numeric)
                compare_numeric(claim_value, &rule.value, |a, b| a >= b)
            }
            "lte" => {
                // Less than or equal (numeric)
                compare_numeric(claim_value, &rule.value, |a, b| a <= b)
            }
            _ => {
                tracing::warn!("Unknown claim operator: {}", rule.operator);
                false
            }
        };

        if !matches {
            return false;
        }
    }

    true
}

/// Helper function for numeric comparisons
fn compare_numeric<F>(
    claim_value: Option<&serde_json::Value>,
    rule_value: &serde_json::Value,
    cmp: F,
) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let claim_num = claim_value.and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_i64().map(|i| i as f64))
            .or_else(|| v.as_u64().map(|u| u as f64))
    });
    let rule_num = rule_value
        .as_f64()
        .or_else(|| rule_value.as_i64().map(|i| i as f64))
        .or_else(|| rule_value.as_u64().map(|u| u as f64));

    match (claim_num, rule_num) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => false,
    }
}

/// Verify admin claims for cache management API access (Phase 65.1)
/// Returns true if admin_claims is empty (no admin restriction) or all admin claims match
pub fn verify_admin_claims(claims: &Claims, admin_rules: &[ClaimRule]) -> bool {
    // If no admin rules configured, admin access is not restricted
    if admin_rules.is_empty() {
        return true;
    }
    // All admin claim rules must match
    verify_claims(claims, admin_rules)
}

pub fn is_auth_required(jwt_config: &Option<JwtConfig>) -> bool {
    match jwt_config {
        Some(config) => config.enabled,
        None => false, // No JWT config means auth is not required
    }
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    ClaimsVerificationFailed,
    /// Admin claim verification failed (Phase 65.1)
    AdminAccessDenied,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => {
                write!(f, "Authentication token not found in request")
            }
            AuthError::InvalidToken(reason) => {
                write!(f, "Invalid authentication token: {}", reason)
            }
            AuthError::ClaimsVerificationFailed => {
                write!(
                    f,
                    "JWT claims verification failed: required claims do not match"
                )
            }
            AuthError::AdminAccessDenied => {
                write!(
                    f,
                    "Admin access denied: JWT does not contain required admin claims"
                )
            }
        }
    }
}

pub fn authenticate_request(
    headers: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
    jwt_config: &JwtConfig,
) -> Result<Claims, AuthError> {
    // Extract token from configured sources
    let token = try_extract_token(headers, query_params, &jwt_config.token_sources)
        .ok_or(AuthError::MissingToken)?;

    // Validate JWT with configured algorithm
    tracing::debug!(
        "Validating JWT signature with algorithm: {}",
        jwt_config.algorithm
    );

    // Determine validation method based on algorithm
    let claims = match jwt_config.algorithm.as_str() {
        "RS256" | "RS384" | "RS512" => {
            // Use RSA public key for RS* algorithms
            let key_path = jwt_config.rsa_public_key_path.as_ref().ok_or_else(|| {
                AuthError::InvalidToken(format!(
                    "RSA public key path not configured for {} algorithm",
                    jwt_config.algorithm
                ))
            })?;

            let decoding_key = load_rsa_public_key(key_path).map_err(|e| {
                tracing::error!("Failed to load RSA public key: {}", e);
                AuthError::InvalidToken(format!("Failed to load RSA public key: {}", e))
            })?;

            validate_jwt_with_key(&token, &decoding_key, &jwt_config.algorithm)
        }
        "ES256" | "ES384" => {
            // Use ECDSA public key for ES* algorithms
            let key_path = jwt_config.ecdsa_public_key_path.as_ref().ok_or_else(|| {
                AuthError::InvalidToken(format!(
                    "ECDSA public key path not configured for {} algorithm",
                    jwt_config.algorithm
                ))
            })?;

            let decoding_key = load_ecdsa_public_key(key_path).map_err(|e| {
                tracing::error!("Failed to load ECDSA public key: {}", e);
                AuthError::InvalidToken(format!("Failed to load ECDSA public key: {}", e))
            })?;

            validate_jwt_with_key(&token, &decoding_key, &jwt_config.algorithm)
        }
        _ => {
            // Use HMAC secret for HS* algorithms (default)
            validate_jwt(&token, &jwt_config.secret, &jwt_config.algorithm)
        }
    }
    .map_err(|e| {
        tracing::warn!("JWT signature validation failed: {}", e);
        AuthError::InvalidToken(e.to_string())
    })?;

    tracing::debug!("JWT signature valid, checking claims");

    // Verify claims if rules are configured
    if !jwt_config.claims.is_empty() {
        tracing::debug!("Verifying {} custom claim rules", jwt_config.claims.len());
        if !verify_claims(&claims, &jwt_config.claims) {
            tracing::warn!("JWT claims verification failed");
            return Err(AuthError::ClaimsVerificationFailed);
        }
        tracing::debug!("All JWT claims verified successfully");
    }

    tracing::debug!("JWT authentication successful");
    Ok(claims)
}

/// Validate JWT using JWKS (JSON Web Key Set) from a remote endpoint
///
/// This function:
/// 1. Extracts the kid (Key ID) from the JWT header
/// 2. Fetches/uses cached JWKS from the configured URL
/// 3. Finds the matching key and validates the JWT
///
/// # Arguments
/// * `token` - The JWT token string to validate
/// * `jwks_client` - A shared JWKS client with caching
///
/// # Returns
/// * `Ok((Claims, String))` - The validated claims and the kid that was used
/// * `Err(AuthError)` - If validation fails
pub async fn validate_jwt_with_jwks(
    token: &str,
    jwks_client: &JwksClient,
) -> Result<(Claims, String), AuthError> {
    // Extract kid from token header
    let kid = extract_kid_from_token(token).ok_or_else(|| {
        AuthError::InvalidToken("JWT does not contain a 'kid' (Key ID) header".to_string())
    })?;

    tracing::debug!("Validating JWT with kid '{}' using JWKS", kid);

    // Ensure JWKS is loaded/refreshed
    jwks_client.get_jwks().await.map_err(|e| {
        tracing::error!("Failed to fetch JWKS: {}", e);
        AuthError::InvalidToken(format!("Failed to fetch JWKS: {}", e))
    })?;

    // Get the decoding key for this kid
    let decoding_key = jwks_client.get_decoding_key(&kid).map_err(|e| {
        tracing::warn!("Key '{}' not found in JWKS: {}", kid, e);
        AuthError::InvalidToken(format!("Key '{}' not found in JWKS", kid))
    })?;

    // Determine algorithm from the JWK
    let jwk = jwks_client
        .find_key(&kid)
        .ok_or_else(|| AuthError::InvalidToken(format!("Key '{}' not found in JWKS", kid)))?;

    let algorithm = jwk.algorithm().unwrap_or("RS256");

    // Validate the JWT
    let claims = validate_jwt_with_key(token, &decoding_key, algorithm).map_err(|e| {
        tracing::warn!("JWT validation failed with key '{}': {}", kid, e);
        AuthError::InvalidToken(format!("JWT validation failed: {}", e))
    })?;

    tracing::debug!("JWT validated successfully using JWKS key '{}'", kid);
    Ok((claims, kid))
}

/// Authenticate request using JWKS
///
/// Similar to `authenticate_request` but uses JWKS for key lookup.
/// This is useful when keys are managed externally (e.g., Auth0, Keycloak).
pub async fn authenticate_request_with_jwks(
    headers: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
    jwt_config: &JwtConfig,
    jwks_client: &JwksClient,
) -> Result<Claims, AuthError> {
    // Extract token from configured sources
    let token = try_extract_token(headers, query_params, &jwt_config.token_sources)
        .ok_or(AuthError::MissingToken)?;

    // Validate using JWKS
    let (claims, kid) = validate_jwt_with_jwks(&token, jwks_client).await?;

    tracing::debug!("JWT validated with key '{}'", kid);

    // Verify claims if rules are configured
    if !jwt_config.claims.is_empty() {
        tracing::debug!("Verifying {} custom claim rules", jwt_config.claims.len());
        if !verify_claims(&claims, &jwt_config.claims) {
            tracing::warn!("JWT claims verification failed");
            return Err(AuthError::ClaimsVerificationFailed);
        }
        tracing::debug!("All JWT claims verified successfully");
    }

    tracing::debug!("JWT authentication with JWKS successful");
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Helper to create Claims with custom fields
    fn make_claims(custom: serde_json::Map<String, serde_json::Value>) -> Claims {
        Claims {
            sub: None,
            exp: None,
            iat: None,
            nbf: None,
            iss: None,
            custom,
        }
    }

    fn make_rule(claim: &str, operator: &str, value: serde_json::Value) -> ClaimRule {
        ClaimRule {
            claim: claim.to_string(),
            operator: operator.to_string(),
            value,
        }
    }

    // ========================
    // equals operator tests
    // ========================

    #[test]
    fn test_verify_claims_equals_string_match() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("admin"));
        let claims = make_claims(custom);

        let rules = vec![make_rule("role", "equals", json!("admin"))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_equals_string_mismatch() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("user"));
        let claims = make_claims(custom);

        let rules = vec![make_rule("role", "equals", json!("admin"))];
        assert!(!verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_equals_missing_claim() {
        let claims = make_claims(serde_json::Map::new());
        let rules = vec![make_rule("role", "equals", json!("admin"))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // in operator tests
    // ========================

    #[test]
    fn test_verify_claims_in_string_found() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("editor"));
        let claims = make_claims(custom);

        let rules = vec![make_rule(
            "role",
            "in",
            json!(["admin", "editor", "viewer"]),
        )];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_in_string_not_found() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("guest"));
        let claims = make_claims(custom);

        let rules = vec![make_rule(
            "role",
            "in",
            json!(["admin", "editor", "viewer"]),
        )];
        assert!(!verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_in_number_found() {
        let mut custom = serde_json::Map::new();
        custom.insert("level".to_string(), json!(5));
        let claims = make_claims(custom);

        let rules = vec![make_rule("level", "in", json!([1, 3, 5, 7]))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_in_empty_array() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("admin"));
        let claims = make_claims(custom);

        let rules = vec![make_rule("role", "in", json!([]))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // contains operator tests
    // ========================

    #[test]
    fn test_verify_claims_contains_substring_found() {
        let mut custom = serde_json::Map::new();
        custom.insert("email".to_string(), json!("user@company.com"));
        let claims = make_claims(custom);

        let rules = vec![make_rule("email", "contains", json!("@company.com"))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_contains_substring_not_found() {
        let mut custom = serde_json::Map::new();
        custom.insert("email".to_string(), json!("user@gmail.com"));
        let claims = make_claims(custom);

        let rules = vec![make_rule("email", "contains", json!("@company.com"))];
        assert!(!verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_contains_non_string_fails() {
        let mut custom = serde_json::Map::new();
        custom.insert("count".to_string(), json!(123));
        let claims = make_claims(custom);

        let rules = vec![make_rule("count", "contains", json!("12"))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // gt operator tests
    // ========================

    #[test]
    fn test_verify_claims_gt_true() {
        let mut custom = serde_json::Map::new();
        custom.insert("age".to_string(), json!(25));
        let claims = make_claims(custom);

        let rules = vec![make_rule("age", "gt", json!(18))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_gt_false_equal() {
        let mut custom = serde_json::Map::new();
        custom.insert("age".to_string(), json!(18));
        let claims = make_claims(custom);

        let rules = vec![make_rule("age", "gt", json!(18))];
        assert!(!verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_gt_false_less() {
        let mut custom = serde_json::Map::new();
        custom.insert("age".to_string(), json!(15));
        let claims = make_claims(custom);

        let rules = vec![make_rule("age", "gt", json!(18))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // lt operator tests
    // ========================

    #[test]
    fn test_verify_claims_lt_true() {
        let mut custom = serde_json::Map::new();
        custom.insert("priority".to_string(), json!(3));
        let claims = make_claims(custom);

        let rules = vec![make_rule("priority", "lt", json!(5))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_lt_false() {
        let mut custom = serde_json::Map::new();
        custom.insert("priority".to_string(), json!(7));
        let claims = make_claims(custom);

        let rules = vec![make_rule("priority", "lt", json!(5))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // gte operator tests
    // ========================

    #[test]
    fn test_verify_claims_gte_greater() {
        let mut custom = serde_json::Map::new();
        custom.insert("score".to_string(), json!(100));
        let claims = make_claims(custom);

        let rules = vec![make_rule("score", "gte", json!(50))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_gte_equal() {
        let mut custom = serde_json::Map::new();
        custom.insert("score".to_string(), json!(50));
        let claims = make_claims(custom);

        let rules = vec![make_rule("score", "gte", json!(50))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_gte_less() {
        let mut custom = serde_json::Map::new();
        custom.insert("score".to_string(), json!(30));
        let claims = make_claims(custom);

        let rules = vec![make_rule("score", "gte", json!(50))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // lte operator tests
    // ========================

    #[test]
    fn test_verify_claims_lte_less() {
        let mut custom = serde_json::Map::new();
        custom.insert("limit".to_string(), json!(5));
        let claims = make_claims(custom);

        let rules = vec![make_rule("limit", "lte", json!(10))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_lte_equal() {
        let mut custom = serde_json::Map::new();
        custom.insert("limit".to_string(), json!(10));
        let claims = make_claims(custom);

        let rules = vec![make_rule("limit", "lte", json!(10))];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_lte_greater() {
        let mut custom = serde_json::Map::new();
        custom.insert("limit".to_string(), json!(15));
        let claims = make_claims(custom);

        let rules = vec![make_rule("limit", "lte", json!(10))];
        assert!(!verify_claims(&claims, &rules));
    }

    // ========================
    // Multiple rules tests
    // ========================

    #[test]
    fn test_verify_claims_multiple_rules_all_pass() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("admin"));
        custom.insert("age".to_string(), json!(25));
        custom.insert("email".to_string(), json!("admin@company.com"));
        let claims = make_claims(custom);

        let rules = vec![
            make_rule("role", "equals", json!("admin")),
            make_rule("age", "gte", json!(18)),
            make_rule("email", "contains", json!("@company.com")),
        ];
        assert!(verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_multiple_rules_one_fails() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("admin"));
        custom.insert("age".to_string(), json!(15)); // Too young
        let claims = make_claims(custom);

        let rules = vec![
            make_rule("role", "equals", json!("admin")),
            make_rule("age", "gte", json!(18)),
        ];
        assert!(!verify_claims(&claims, &rules));
    }

    #[test]
    fn test_verify_claims_empty_rules_passes() {
        let claims = make_claims(serde_json::Map::new());
        let rules: Vec<ClaimRule> = vec![];
        assert!(verify_claims(&claims, &rules));
    }

    // ========================
    // Unknown operator test
    // ========================

    #[test]
    fn test_verify_claims_unknown_operator_fails() {
        let mut custom = serde_json::Map::new();
        custom.insert("role".to_string(), json!("admin"));
        let claims = make_claims(custom);

        let rules = vec![make_rule("role", "unknown", json!("admin"))];
        assert!(!verify_claims(&claims, &rules));
    }
}
