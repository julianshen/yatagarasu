// Authentication module

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::{ClaimRule, JwtConfig};

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
    get_header_case_insensitive(headers, "Authorization")
        .and_then(|value| value.strip_prefix("Bearer ").map(|s| s.to_string()))
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

pub fn extract_header_token(
    headers: &HashMap<String, String>,
    header_name: &str,
) -> Option<String> {
    get_header_case_insensitive(headers, header_name)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn extract_query_token(
    query_params: &HashMap<String, String>,
    param_name: &str,
) -> Option<String> {
    query_params
        .get(param_name)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn try_extract_token(
    headers: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
    sources: &[crate::config::TokenSource],
) -> Option<String> {
    for source in sources {
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
            _ => None,
        };

        if token.is_some() {
            return token;
        }
    }

    None
}

pub fn validate_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
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

pub fn verify_claims(claims: &Claims, rules: &[ClaimRule]) -> bool {
    for rule in rules {
        let claim_value = claims.custom.get(&rule.claim);

        match rule.operator.as_str() {
            "equals" => {
                if claim_value != Some(&rule.value) {
                    return false;
                }
            }
            _ => return false, // Unknown operator
        }
    }

    true
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

    // Validate JWT
    let claims = validate_jwt(&token, &jwt_config.secret)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    // Verify claims if rules are configured
    if !verify_claims(&claims, &jwt_config.claims) {
        return Err(AuthError::ClaimsVerificationFailed);
    }

    Ok(claims)
}
