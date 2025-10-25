// Authentication module

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
use jsonwebtoken::{encode, EncodingKey, Header};

use crate::config::{ClaimRule, JwtConfig};

#[derive(Debug, Serialize, Deserialize)]
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
                    extract_header_token(headers, header_name)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracts_token_from_authorization_header_with_bearer_prefix() {
        // Create a simple representation of HTTP headers
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer abc123token".to_string(),
        );

        // Extract token from Authorization header with Bearer prefix
        let token = extract_bearer_token(&headers);

        assert_eq!(
            token,
            Some("abc123token".to_string()),
            "Expected to extract 'abc123token' from 'Bearer abc123token'"
        );
    }

    #[test]
    fn test_extracts_token_from_authorization_header_without_prefix() {
        // Create headers with raw token (no Bearer prefix)
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "abc123token".to_string());

        // Extract token from Authorization header without prefix
        let token = extract_header_token(&headers, "Authorization");

        assert_eq!(
            token,
            Some("abc123token".to_string()),
            "Expected to extract 'abc123token' from raw header value"
        );
    }

    #[test]
    fn test_extracts_token_from_custom_header() {
        // Create headers with custom header
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Auth-Token".to_string(), "custom123token".to_string());

        // Extract token from custom header
        let token = extract_header_token(&headers, "X-Auth-Token");

        assert_eq!(
            token,
            Some("custom123token".to_string()),
            "Expected to extract 'custom123token' from X-Auth-Token header"
        );

        // Test with another custom header name
        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("X-API-Key".to_string(), "apikey456".to_string());

        let token2 = extract_header_token(&headers2, "X-API-Key");

        assert_eq!(
            token2,
            Some("apikey456".to_string()),
            "Expected to extract 'apikey456' from X-API-Key header"
        );
    }

    #[test]
    fn test_returns_none_when_authorization_header_missing() {
        // Create empty headers
        let headers = std::collections::HashMap::new();

        // Try to extract from missing Authorization header
        let token = extract_header_token(&headers, "Authorization");

        assert_eq!(
            token, None,
            "Expected None when Authorization header is missing"
        );

        // Try to extract Bearer token from missing header
        let bearer_token = extract_bearer_token(&headers);

        assert_eq!(
            bearer_token, None,
            "Expected None when Authorization header is missing for Bearer extraction"
        );

        // Try to extract from missing custom header
        let custom_token = extract_header_token(&headers, "X-Auth-Token");

        assert_eq!(
            custom_token, None,
            "Expected None when custom header is missing"
        );
    }

    #[test]
    fn test_returns_none_when_authorization_header_malformed() {
        // Test Bearer prefix with no token
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer ".to_string());

        let token = extract_bearer_token(&headers);

        assert_eq!(
            token, None,
            "Expected None when Authorization header has 'Bearer ' with no token"
        );

        // Test Bearer without space (malformed)
        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("Authorization".to_string(), "Bearer".to_string());

        let token2 = extract_bearer_token(&headers2);

        assert_eq!(
            token2, None,
            "Expected None when Authorization header has 'Bearer' without space"
        );

        // Test empty string
        let mut headers3 = std::collections::HashMap::new();
        headers3.insert("Authorization".to_string(), "".to_string());

        let token3 = extract_bearer_token(&headers3);

        assert_eq!(
            token3, None,
            "Expected None when Authorization header is empty string"
        );

        // Test just whitespace
        let mut headers4 = std::collections::HashMap::new();
        headers4.insert("Authorization".to_string(), "   ".to_string());

        let token4 = extract_bearer_token(&headers4);

        assert_eq!(
            token4, None,
            "Expected None when Authorization header is just whitespace"
        );
    }

    #[test]
    fn test_handles_whitespace_in_authorization_header_value() {
        // Test token with trailing whitespace
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123  ".to_string());

        let token = extract_bearer_token(&headers);

        assert_eq!(
            token,
            Some("token123".to_string()),
            "Expected 'token123' with trailing whitespace trimmed"
        );

        // Test token with leading whitespace after Bearer
        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("Authorization".to_string(), "Bearer   token456".to_string());

        let token2 = extract_bearer_token(&headers2);

        assert_eq!(
            token2,
            Some("token456".to_string()),
            "Expected 'token456' with leading whitespace trimmed"
        );

        // Test token with both leading and trailing whitespace
        let mut headers3 = std::collections::HashMap::new();
        headers3.insert(
            "Authorization".to_string(),
            "Bearer  token789  ".to_string(),
        );

        let token3 = extract_bearer_token(&headers3);

        assert_eq!(
            token3,
            Some("token789".to_string()),
            "Expected 'token789' with both leading and trailing whitespace trimmed"
        );
    }

    #[test]
    fn test_case_insensitive_header_name_matching() {
        // Test lowercase "authorization"
        let mut headers = std::collections::HashMap::new();
        headers.insert("authorization".to_string(), "Bearer token123".to_string());

        let token = extract_bearer_token(&headers);

        assert_eq!(
            token,
            Some("token123".to_string()),
            "Expected to extract token from lowercase 'authorization' header"
        );

        // Test UPPERCASE "AUTHORIZATION"
        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("AUTHORIZATION".to_string(), "Bearer token456".to_string());

        let token2 = extract_bearer_token(&headers2);

        assert_eq!(
            token2,
            Some("token456".to_string()),
            "Expected to extract token from uppercase 'AUTHORIZATION' header"
        );

        // Test mixed case "AuThOrIzAtIoN"
        let mut headers3 = std::collections::HashMap::new();
        headers3.insert("AuThOrIzAtIoN".to_string(), "Bearer token789".to_string());

        let token3 = extract_bearer_token(&headers3);

        assert_eq!(
            token3,
            Some("token789".to_string()),
            "Expected to extract token from mixed case 'AuThOrIzAtIoN' header"
        );

        // Test case-insensitive custom header with extract_header_token
        let mut headers4 = std::collections::HashMap::new();
        headers4.insert("x-auth-token".to_string(), "customtoken".to_string());

        let token4 = extract_header_token(&headers4, "X-Auth-Token");

        assert_eq!(
            token4,
            Some("customtoken".to_string()),
            "Expected to extract token from lowercase 'x-auth-token' when requesting 'X-Auth-Token'"
        );
    }

    #[test]
    fn test_extracts_token_from_query_parameter_by_name() {
        // Create query parameters
        let mut query_params = std::collections::HashMap::new();
        query_params.insert("token".to_string(), "querytoken123".to_string());
        query_params.insert("other".to_string(), "othervalue".to_string());

        // Extract token from query parameter by name
        let token = extract_query_token(&query_params, "token");

        assert_eq!(
            token,
            Some("querytoken123".to_string()),
            "Expected to extract 'querytoken123' from 'token' query parameter"
        );

        // Test with different parameter name
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("access_token".to_string(), "accesstoken456".to_string());

        let token2 = extract_query_token(&query_params2, "access_token");

        assert_eq!(
            token2,
            Some("accesstoken456".to_string()),
            "Expected to extract 'accesstoken456' from 'access_token' query parameter"
        );

        // Test with jwt parameter name
        let mut query_params3 = std::collections::HashMap::new();
        query_params3.insert("jwt".to_string(), "jwttoken789".to_string());

        let token3 = extract_query_token(&query_params3, "jwt");

        assert_eq!(
            token3,
            Some("jwttoken789".to_string()),
            "Expected to extract 'jwttoken789' from 'jwt' query parameter"
        );
    }

    #[test]
    fn test_returns_none_when_query_parameter_missing() {
        // Create empty query parameters
        let query_params = std::collections::HashMap::new();

        // Try to extract from missing query parameter
        let token = extract_query_token(&query_params, "token");

        assert_eq!(token, None, "Expected None when query parameter is missing");

        // Create query params with some parameters but not the one we're looking for
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("other".to_string(), "othervalue".to_string());
        query_params2.insert("foo".to_string(), "bar".to_string());

        // Try to extract a parameter that doesn't exist
        let token2 = extract_query_token(&query_params2, "token");

        assert_eq!(
            token2, None,
            "Expected None when specific query parameter is missing"
        );

        // Test with different parameter name
        let token3 = extract_query_token(&query_params2, "access_token");

        assert_eq!(
            token3, None,
            "Expected None when 'access_token' parameter is missing"
        );
    }

    #[test]
    fn test_handles_url_encoded_token_in_query_parameter() {
        // Note: In a real HTTP server, URL decoding happens before we receive query params
        // This test verifies we correctly handle tokens with special characters that
        // would typically be URL-encoded in transit (like +, /, =, etc.)

        // Test token with characters that would be URL-encoded: + / =
        let mut query_params = std::collections::HashMap::new();
        query_params.insert(
            "token".to_string(),
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U".to_string(),
        );

        let token = extract_query_token(&query_params, "token");

        assert_eq!(
            token,
            Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U".to_string()),
            "Expected to extract JWT token with dots and base64 characters"
        );

        // Test token that was decoded from URL encoding (spaces decoded from %20 or +)
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("token".to_string(), "token with spaces".to_string());

        let token2 = extract_query_token(&query_params2, "token");

        assert_eq!(
            token2,
            Some("token with spaces".to_string()),
            "Expected to extract token with spaces (decoded from URL encoding)"
        );

        // Test token with special characters (already decoded)
        let mut query_params3 = std::collections::HashMap::new();
        query_params3.insert("token".to_string(), "token&special=chars".to_string());

        let token3 = extract_query_token(&query_params3, "token");

        assert_eq!(
            token3,
            Some("token&special=chars".to_string()),
            "Expected to extract token with special characters"
        );
    }

    #[test]
    fn test_handles_multiple_query_parameters_ignores_others() {
        // Test with many query parameters, extracting specific one
        let mut query_params = std::collections::HashMap::new();
        query_params.insert("token".to_string(), "mytoken123".to_string());
        query_params.insert("user".to_string(), "john".to_string());
        query_params.insert("action".to_string(), "download".to_string());
        query_params.insert("file".to_string(), "document.pdf".to_string());
        query_params.insert("version".to_string(), "2".to_string());

        // Extract token parameter, ignoring all others
        let token = extract_query_token(&query_params, "token");

        assert_eq!(
            token,
            Some("mytoken123".to_string()),
            "Expected to extract 'token' parameter while ignoring other parameters"
        );

        // Extract a different parameter from the same set
        let user = extract_query_token(&query_params, "user");

        assert_eq!(
            user,
            Some("john".to_string()),
            "Expected to extract 'user' parameter while ignoring other parameters"
        );

        // Test with similar parameter names (token vs access_token)
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("token".to_string(), "token1".to_string());
        query_params2.insert("access_token".to_string(), "token2".to_string());
        query_params2.insert("refresh_token".to_string(), "token3".to_string());

        let token1 = extract_query_token(&query_params2, "token");
        let token2 = extract_query_token(&query_params2, "access_token");
        let token3 = extract_query_token(&query_params2, "refresh_token");

        assert_eq!(
            token1,
            Some("token1".to_string()),
            "Expected to extract exact 'token' parameter"
        );
        assert_eq!(
            token2,
            Some("token2".to_string()),
            "Expected to extract exact 'access_token' parameter"
        );
        assert_eq!(
            token3,
            Some("token3".to_string()),
            "Expected to extract exact 'refresh_token' parameter"
        );
    }

    #[test]
    fn test_handles_empty_query_parameter_value() {
        // Test with empty string value (e.g., ?token=)
        let mut query_params = std::collections::HashMap::new();
        query_params.insert("token".to_string(), "".to_string());

        let token = extract_query_token(&query_params, "token");

        assert_eq!(
            token, None,
            "Expected None when query parameter value is empty string"
        );

        // Test with empty value alongside other valid parameters
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("token".to_string(), "".to_string());
        query_params2.insert("user".to_string(), "john".to_string());
        query_params2.insert("action".to_string(), "download".to_string());

        let token2 = extract_query_token(&query_params2, "token");
        let user = extract_query_token(&query_params2, "user");

        assert_eq!(
            token2, None,
            "Expected None for empty 'token' parameter even with other valid parameters"
        );
        assert_eq!(
            user,
            Some("john".to_string()),
            "Expected to extract valid 'user' parameter"
        );

        // Test with whitespace-only value (should also be treated as empty/invalid)
        let mut query_params3 = std::collections::HashMap::new();
        query_params3.insert("token".to_string(), "   ".to_string());

        let token3 = extract_query_token(&query_params3, "token");

        assert_eq!(
            token3, None,
            "Expected None when query parameter value is only whitespace"
        );
    }

    #[test]
    fn test_tries_all_configured_sources_in_order() {
        use crate::config::TokenSource;

        // Setup: No token in any source
        let headers = std::collections::HashMap::new();
        let query_params = std::collections::HashMap::new();

        // Configure sources: Bearer header, then custom header, then query param
        let sources = vec![
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
        ];

        // Try all sources - should check all three and return None
        let token = try_extract_token(&headers, &query_params, &sources);

        assert_eq!(
            token, None,
            "Expected None when no token found in any configured source"
        );

        // Setup: Token only in the third source (query parameter)
        let headers2 = std::collections::HashMap::new();
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("token".to_string(), "query_token".to_string());

        // Should try bearer header (none), custom header (none), then query (found!)
        let token2 = try_extract_token(&headers2, &query_params2, &sources);

        assert_eq!(
            token2,
            Some("query_token".to_string()),
            "Expected to find token in third source (query parameter)"
        );

        // Setup: Token in the second source (custom header)
        let mut headers3 = std::collections::HashMap::new();
        headers3.insert("X-Auth-Token".to_string(), "header_token".to_string());
        let query_params3 = std::collections::HashMap::new();

        // Should try bearer header (none), then custom header (found!)
        let token3 = try_extract_token(&headers3, &query_params3, &sources);

        assert_eq!(
            token3,
            Some("header_token".to_string()),
            "Expected to find token in second source (custom header)"
        );

        // Setup: Token in the first source (bearer header)
        let mut headers4 = std::collections::HashMap::new();
        headers4.insert(
            "Authorization".to_string(),
            "Bearer bearer_token".to_string(),
        );
        let query_params4 = std::collections::HashMap::new();

        // Should find immediately in first source
        let token4 = try_extract_token(&headers4, &query_params4, &sources);

        assert_eq!(
            token4,
            Some("bearer_token".to_string()),
            "Expected to find token in first source (bearer header)"
        );
    }

    #[test]
    fn test_returns_first_valid_token_found() {
        use crate::config::TokenSource;

        // Configure sources: Bearer header, then custom header, then query param
        let sources = vec![
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
        ];

        // Setup: Tokens in ALL sources - should return only the first one (bearer)
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer first_token".to_string(),
        );
        headers.insert("X-Auth-Token".to_string(), "second_token".to_string());

        let mut query_params = std::collections::HashMap::new();
        query_params.insert("token".to_string(), "third_token".to_string());

        let token = try_extract_token(&headers, &query_params, &sources);

        assert_eq!(
            token,
            Some("first_token".to_string()),
            "Expected to return first token (bearer) and ignore others"
        );

        // Setup: Tokens in second and third sources - should return second (custom header)
        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("X-Auth-Token".to_string(), "second_token".to_string());

        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("token".to_string(), "third_token".to_string());

        let token2 = try_extract_token(&headers2, &query_params2, &sources);

        assert_eq!(
            token2,
            Some("second_token".to_string()),
            "Expected to return second token (custom header) and ignore query param"
        );

        // Setup: Token only in third source
        let headers3 = std::collections::HashMap::new();
        let mut query_params3 = std::collections::HashMap::new();
        query_params3.insert("token".to_string(), "third_token".to_string());

        let token3 = try_extract_token(&headers3, &query_params3, &sources);

        assert_eq!(
            token3,
            Some("third_token".to_string()),
            "Expected to return third token (query param) when no higher priority sources"
        );
    }

    #[test]
    fn test_returns_none_if_no_sources_have_valid_token() {
        use crate::config::TokenSource;

        // Configure sources: Bearer header, then custom header, then query param
        let sources = vec![
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
        ];

        // Test 1: Empty headers and query params
        let headers = std::collections::HashMap::new();
        let query_params = std::collections::HashMap::new();

        let token = try_extract_token(&headers, &query_params, &sources);

        assert_eq!(
            token, None,
            "Expected None when headers and query params are empty"
        );

        // Test 2: Headers and query params exist but don't contain the configured parameters
        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("Content-Type".to_string(), "application/json".to_string());
        headers2.insert("User-Agent".to_string(), "test-agent".to_string());

        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("user".to_string(), "john".to_string());
        query_params2.insert("action".to_string(), "download".to_string());

        let token2 = try_extract_token(&headers2, &query_params2, &sources);

        assert_eq!(
            token2, None,
            "Expected None when configured token parameters are missing"
        );

        // Test 3: Bearer header exists but malformed (no "Bearer " prefix)
        let mut headers3 = std::collections::HashMap::new();
        headers3.insert("Authorization".to_string(), "InvalidFormat".to_string());

        let query_params3 = std::collections::HashMap::new();

        let token3 = try_extract_token(&headers3, &query_params3, &sources);

        assert_eq!(token3, None, "Expected None when bearer token is malformed");

        // Test 4: Token parameters exist but have empty values
        let mut headers4 = std::collections::HashMap::new();
        headers4.insert("X-Auth-Token".to_string(), "".to_string());

        let mut query_params4 = std::collections::HashMap::new();
        query_params4.insert("token".to_string(), "   ".to_string());

        let token4 = try_extract_token(&headers4, &query_params4, &sources);

        assert_eq!(
            token4, None,
            "Expected None when all token values are empty or whitespace"
        );

        // Test 5: Empty sources list
        let sources_empty: Vec<TokenSource> = vec![];
        let mut headers5 = std::collections::HashMap::new();
        headers5.insert(
            "Authorization".to_string(),
            "Bearer valid_token".to_string(),
        );
        let query_params5 = std::collections::HashMap::new();

        let token5 = try_extract_token(&headers5, &query_params5, &sources_empty);

        assert_eq!(
            token5, None,
            "Expected None when sources list is empty (no configured sources)"
        );
    }

    #[test]
    fn test_header_source_checked_before_query_parameter() {
        use crate::config::TokenSource;

        // Configure sources: Header before query parameter
        let sources = vec![
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
        ];

        // Test 1: Token in both header and query param - should return header token
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Auth-Token".to_string(), "header_token".to_string());

        let mut query_params = std::collections::HashMap::new();
        query_params.insert("token".to_string(), "query_token".to_string());

        let token = try_extract_token(&headers, &query_params, &sources);

        assert_eq!(
            token,
            Some("header_token".to_string()),
            "Expected to return header token (higher priority) and ignore query param token"
        );

        // Test 2: Token only in query param - should return query param token
        let headers2 = std::collections::HashMap::new();
        let mut query_params2 = std::collections::HashMap::new();
        query_params2.insert("token".to_string(), "query_token".to_string());

        let token2 = try_extract_token(&headers2, &query_params2, &sources);

        assert_eq!(
            token2,
            Some("query_token".to_string()),
            "Expected to return query param token when header is missing"
        );

        // Test 3: Token only in header - should return header token
        let mut headers3 = std::collections::HashMap::new();
        headers3.insert("X-Auth-Token".to_string(), "header_token".to_string());
        let query_params3 = std::collections::HashMap::new();

        let token3 = try_extract_token(&headers3, &query_params3, &sources);

        assert_eq!(
            token3,
            Some("header_token".to_string()),
            "Expected to return header token when query param is missing"
        );

        // Test 4: Reverse order - query before header
        let sources_reversed = vec![
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
        ];

        // With reversed order and tokens in both, should return query token
        let mut headers4 = std::collections::HashMap::new();
        headers4.insert("X-Auth-Token".to_string(), "header_token".to_string());

        let mut query_params4 = std::collections::HashMap::new();
        query_params4.insert("token".to_string(), "query_token".to_string());

        let token4 = try_extract_token(&headers4, &query_params4, &sources_reversed);

        assert_eq!(
            token4,
            Some("query_token".to_string()),
            "Expected to return query token (higher priority in reversed order) and ignore header token"
        );
    }

    #[test]
    fn test_configurable_source_order_is_respected() {
        use crate::config::TokenSource;

        // Setup: All three types of tokens present
        let mut headers = std::collections::HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer bearer_token".to_string(),
        );
        headers.insert("X-Auth-Token".to_string(), "header_token".to_string());

        let mut query_params = std::collections::HashMap::new();
        query_params.insert("token".to_string(), "query_token".to_string());

        // Test 1: Order: Bearer, Header, Query
        let order1 = vec![
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
        ];

        let token1 = try_extract_token(&headers, &query_params, &order1);
        assert_eq!(
            token1,
            Some("bearer_token".to_string()),
            "Expected bearer_token with order [Bearer, Header, Query]"
        );

        // Test 2: Order: Bearer, Query, Header
        let order2 = vec![
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
        ];

        let token2 = try_extract_token(&headers, &query_params, &order2);
        assert_eq!(
            token2,
            Some("bearer_token".to_string()),
            "Expected bearer_token with order [Bearer, Query, Header]"
        );

        // Test 3: Order: Header, Bearer, Query
        let order3 = vec![
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
        ];

        let token3 = try_extract_token(&headers, &query_params, &order3);
        assert_eq!(
            token3,
            Some("header_token".to_string()),
            "Expected header_token with order [Header, Bearer, Query]"
        );

        // Test 4: Order: Header, Query, Bearer
        let order4 = vec![
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
        ];

        let token4 = try_extract_token(&headers, &query_params, &order4);
        assert_eq!(
            token4,
            Some("header_token".to_string()),
            "Expected header_token with order [Header, Query, Bearer]"
        );

        // Test 5: Order: Query, Bearer, Header
        let order5 = vec![
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
        ];

        let token5 = try_extract_token(&headers, &query_params, &order5);
        assert_eq!(
            token5,
            Some("query_token".to_string()),
            "Expected query_token with order [Query, Bearer, Header]"
        );

        // Test 6: Order: Query, Header, Bearer
        let order6 = vec![
            TokenSource {
                source_type: "query".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("X-Auth-Token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
        ];

        let token6 = try_extract_token(&headers, &query_params, &order6);
        assert_eq!(
            token6,
            Some("query_token".to_string()),
            "Expected query_token with order [Query, Header, Bearer]"
        );
    }

    #[test]
    fn test_validates_correctly_signed_jwt_with_hs256() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        // Create a test secret
        let secret = "test_secret_key_123";

        // Create test claims
        let mut claims_map = serde_json::Map::new();
        claims_map.insert("sub".to_string(), json!("user123"));
        claims_map.insert("name".to_string(), json!("John Doe"));
        claims_map.insert("admin".to_string(), json!(true));

        // Encode the JWT token with HS256
        let token = encode(
            &Header::default(), // Default uses HS256
            &claims_map,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode token");

        // Validate the token
        let result = validate_jwt(&token, secret);

        assert!(
            result.is_ok(),
            "Expected valid JWT to be accepted, but got error: {:?}",
            result.err()
        );

        let claims = result.unwrap();
        assert_eq!(
            claims.sub,
            Some("user123".to_string()),
            "Expected sub claim to be 'user123'"
        );
        assert_eq!(
            claims.custom.get("name"),
            Some(&json!("John Doe")),
            "Expected custom name claim to be 'John Doe'"
        );
        assert_eq!(
            claims.custom.get("admin"),
            Some(&json!(true)),
            "Expected custom admin claim to be true"
        );
    }

    #[test]
    fn test_rejects_jwt_with_invalid_signature() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        // Create a JWT token with one secret
        let signing_secret = "correct_secret_key";
        let wrong_secret = "wrong_secret_key";

        // Create test claims
        let mut claims_map = serde_json::Map::new();
        claims_map.insert("sub".to_string(), json!("user123"));
        claims_map.insert("name".to_string(), json!("John Doe"));

        // Encode the JWT token with the correct secret
        let token = encode(
            &Header::default(),
            &claims_map,
            &EncodingKey::from_secret(signing_secret.as_ref()),
        )
        .expect("Failed to encode token");

        // Try to validate with wrong secret - should fail
        let result = validate_jwt(&token, wrong_secret);

        assert!(
            result.is_err(),
            "Expected JWT with invalid signature to be rejected, but it was accepted"
        );

        // Verify the error is related to signature validation
        let error = result.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                jsonwebtoken::errors::ErrorKind::InvalidSignature
            ),
            "Expected InvalidSignature error, but got: {:?}",
            error.kind()
        );
    }

    #[test]
    fn test_rejects_completely_tampered_jwt() {
        // Create a valid token first
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;

        let secret = "test_secret";
        let mut claims_map = serde_json::Map::new();
        claims_map.insert("sub".to_string(), json!("user123"));

        let token = encode(
            &Header::default(),
            &claims_map,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode token");

        // Tamper with the token by modifying a character in the signature part
        let parts: Vec<&str> = token.split('.').collect();
        let tampered_token = format!("{}.{}.{}X", parts[0], parts[1], parts[2]);

        // Try to validate the tampered token
        let result = validate_jwt(&tampered_token, secret);

        assert!(
            result.is_err(),
            "Expected tampered JWT to be rejected, but it was accepted"
        );
    }

    #[test]
    fn test_rejects_jwt_with_expired_exp_claim() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret = "test_secret";

        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create claims with exp set to 1 hour ago (expired)
        let mut claims_map = serde_json::Map::new();
        claims_map.insert("sub".to_string(), json!("user123"));
        claims_map.insert("exp".to_string(), json!(now - 3600)); // 1 hour ago

        // Encode the JWT token
        let token = encode(
            &Header::default(),
            &claims_map,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode token");

        // Try to validate the expired token
        let result = validate_jwt(&token, secret);

        assert!(
            result.is_err(),
            "Expected expired JWT to be rejected, but it was accepted"
        );

        // Verify the error is related to expiration
        let error = result.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                jsonwebtoken::errors::ErrorKind::ExpiredSignature
            ),
            "Expected ExpiredSignature error, but got: {:?}",
            error.kind()
        );
    }

    #[test]
    fn test_rejects_jwt_with_future_nbf_claim() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret = "test_secret";

        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create claims with nbf set to 1 hour in the future
        let mut claims_map = serde_json::Map::new();
        claims_map.insert("sub".to_string(), json!("user123"));
        claims_map.insert("nbf".to_string(), json!(now + 3600)); // 1 hour from now

        // Encode the JWT token
        let token = encode(
            &Header::default(),
            &claims_map,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode token");

        // Try to validate the token with future nbf
        let result = validate_jwt(&token, secret);

        assert!(
            result.is_err(),
            "Expected JWT with future nbf to be rejected, but it was accepted"
        );

        // Verify the error is related to nbf validation
        let error = result.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                jsonwebtoken::errors::ErrorKind::ImmatureSignature
            ),
            "Expected ImmatureSignature error, but got: {:?}",
            error.kind()
        );
    }

    #[test]
    fn test_accepts_jwt_with_valid_exp_and_nbf_claims() {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde_json::json;
        use std::time::{SystemTime, UNIX_EPOCH};

        let secret = "test_secret";

        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create claims with valid nbf (1 hour ago) and exp (1 hour from now)
        let mut claims_map = serde_json::Map::new();
        claims_map.insert("sub".to_string(), json!("user123"));
        claims_map.insert("name".to_string(), json!("John Doe"));
        claims_map.insert("nbf".to_string(), json!(now - 3600)); // 1 hour ago (valid)
        claims_map.insert("exp".to_string(), json!(now + 3600)); // 1 hour from now (not expired)

        // Encode the JWT token
        let token = encode(
            &Header::default(),
            &claims_map,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode token");

        // Validate the token - should succeed
        let result = validate_jwt(&token, secret);

        assert!(
            result.is_ok(),
            "Expected JWT with valid exp and nbf to be accepted, but got error: {:?}",
            result.err()
        );

        let claims = result.unwrap();
        assert_eq!(
            claims.sub,
            Some("user123".to_string()),
            "Expected sub claim to be 'user123'"
        );
        assert_eq!(
            claims.custom.get("name"),
            Some(&json!("John Doe")),
            "Expected custom name claim to be 'John Doe'"
        );
        assert_eq!(
            claims.nbf,
            Some(now - 3600),
            "Expected nbf claim to be preserved"
        );
        assert_eq!(
            claims.exp,
            Some(now + 3600),
            "Expected exp claim to be preserved"
        );
    }

    #[test]
    fn test_rejects_malformed_jwt_not_3_parts() {
        let secret = "test_secret";

        // Test 1: JWT with only 2 parts (missing signature)
        let two_part_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIn0";
        let result = validate_jwt(two_part_token, secret);
        assert!(
            result.is_err(),
            "Expected JWT with only 2 parts to be rejected, but it was accepted"
        );

        // Test 2: JWT with only 1 part
        let one_part_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let result2 = validate_jwt(one_part_token, secret);
        assert!(
            result2.is_err(),
            "Expected JWT with only 1 part to be rejected, but it was accepted"
        );

        // Test 3: JWT with 4 parts (too many)
        let four_part_token =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIn0.signature.extra";
        let result3 = validate_jwt(four_part_token, secret);
        assert!(
            result3.is_err(),
            "Expected JWT with 4 parts to be rejected, but it was accepted"
        );

        // Test 4: Empty string
        let empty_token = "";
        let result4 = validate_jwt(empty_token, secret);
        assert!(
            result4.is_err(),
            "Expected empty token to be rejected, but it was accepted"
        );

        // Test 5: Just dots
        let dots_token = "..";
        let result5 = validate_jwt(dots_token, secret);
        assert!(
            result5.is_err(),
            "Expected token with just dots to be rejected, but it was accepted"
        );
    }

    #[test]
    fn test_rejects_jwt_with_invalid_base64_encoding() {
        let secret = "test_secret";

        // Test 1: Invalid Base64 characters in header (@ is not valid base64)
        let invalid_header = "@@@invalid@@@.eyJzdWIiOiJ1c2VyMTIzIn0.signature";
        let result1 = validate_jwt(invalid_header, secret);
        assert!(
            result1.is_err(),
            "Expected JWT with invalid Base64 in header to be rejected"
        );

        // Test 2: Invalid Base64 characters in payload (! is not valid base64)
        let invalid_payload = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.!!!invalid!!!.signature";
        let result2 = validate_jwt(invalid_payload, secret);
        assert!(
            result2.is_err(),
            "Expected JWT with invalid Base64 in payload to be rejected"
        );

        // Test 3: Invalid Base64 characters in signature (spaces are not valid base64)
        let invalid_signature =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIn0.invalid signature with spaces";
        let result3 = validate_jwt(invalid_signature, secret);
        assert!(
            result3.is_err(),
            "Expected JWT with invalid Base64 in signature to be rejected"
        );

        // Test 4: Mix of invalid characters ($, #, %)
        let mixed_invalid = "$invalid#header%.{invalid}payload.signature~with~tildes";
        let result4 = validate_jwt(mixed_invalid, secret);
        assert!(
            result4.is_err(),
            "Expected JWT with multiple invalid Base64 characters to be rejected"
        );
    }

    #[test]
    fn test_rejects_jwt_with_invalid_json_in_payload() {
        let secret = "test_secret";

        // Test 1: Valid Base64 but payload contains plain text instead of JSON
        // Header: {"alg":"HS256","typ":"JWT"} (valid)
        // Payload: "not json at all" (base64: bm90IGpzb24gYXQgYWxs)
        let plain_text_payload =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.bm90IGpzb24gYXQgYWxs.signature";
        let result1 = validate_jwt(plain_text_payload, secret);
        assert!(
            result1.is_err(),
            "Expected JWT with plain text payload to be rejected"
        );

        // Test 2: Malformed JSON - missing closing brace
        // Payload: {"sub":"user123" (base64: eyJzdWIiOiJ1c2VyMTIzIg==)
        let malformed_json =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIg.signature";
        let result2 = validate_jwt(malformed_json, secret);
        assert!(
            result2.is_err(),
            "Expected JWT with malformed JSON in payload to be rejected"
        );

        // Test 3: Invalid JSON - single quotes instead of double quotes
        // Payload: {'sub':'user'} (base64: eydzdWInOid1c2VyJ30=)
        let single_quotes_json =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eydzdWInOid1c2VyJ30.signature";
        let result3 = validate_jwt(single_quotes_json, secret);
        assert!(
            result3.is_err(),
            "Expected JWT with single-quoted JSON to be rejected"
        );

        // Test 4: Just a number (valid JSON but not an object)
        // Payload: 12345 (base64: MTIzNDU=)
        let number_payload = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.MTIzNDU.signature";
        let result4 = validate_jwt(number_payload, secret);
        assert!(
            result4.is_err(),
            "Expected JWT with non-object JSON payload to be rejected"
        );
    }

    #[test]
    fn test_extracts_standard_claims() {
        let secret = "test_secret";

        // Create a JWT with standard claims
        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999), // Far future
            iat: Some(1234567890),
            nbf: Some(1234567890),
            iss: Some("test-issuer".to_string()),
            custom: serde_json::Map::new(),
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Verify standard claims are extracted correctly
        assert_eq!(
            extracted_claims.sub,
            Some("user123".to_string()),
            "Subject claim not extracted correctly"
        );
        assert_eq!(
            extracted_claims.iss,
            Some("test-issuer".to_string()),
            "Issuer claim not extracted correctly"
        );
        assert_eq!(
            extracted_claims.exp,
            Some(9999999999),
            "Expiration claim not extracted correctly"
        );
        assert_eq!(
            extracted_claims.iat,
            Some(1234567890),
            "Issued at claim not extracted correctly"
        );
    }

    #[test]
    fn test_extracts_custom_claims_from_payload() {
        let secret = "test_secret";

        // Create a JWT with custom claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "user_role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );
        custom_map.insert(
            "permissions".to_string(),
            serde_json::Value::String("read,write,delete".to_string()),
        );
        custom_map.insert(
            "user_id".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );
        custom_map.insert("is_verified".to_string(), serde_json::Value::Bool(true));

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Verify custom claims are extracted correctly
        assert_eq!(
            extracted_claims
                .custom
                .get("user_role")
                .and_then(|v| v.as_str()),
            Some("admin"),
            "Custom claim 'user_role' not extracted correctly"
        );
        assert_eq!(
            extracted_claims
                .custom
                .get("permissions")
                .and_then(|v| v.as_str()),
            Some("read,write,delete"),
            "Custom claim 'permissions' not extracted correctly"
        );
        assert_eq!(
            extracted_claims
                .custom
                .get("user_id")
                .and_then(|v| v.as_i64()),
            Some(42),
            "Custom claim 'user_id' not extracted correctly"
        );
        assert_eq!(
            extracted_claims
                .custom
                .get("is_verified")
                .and_then(|v| v.as_bool()),
            Some(true),
            "Custom claim 'is_verified' not extracted correctly"
        );
    }

    #[test]
    fn test_handles_missing_optional_claims_gracefully() {
        let secret = "test_secret";

        // Create a JWT with minimal claims - only exp to ensure it's not expired
        let claims = Claims {
            sub: None,
            exp: Some(9999999999), // Far future to pass expiration validation
            iat: None,
            nbf: None,
            iss: None,
            custom: serde_json::Map::new(),
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(
            result.is_ok(),
            "Expected JWT with missing optional claims to be accepted"
        );

        let extracted_claims = result.unwrap();

        // Verify all optional claims are None
        assert_eq!(
            extracted_claims.sub, None,
            "Expected 'sub' claim to be None when missing"
        );
        assert_eq!(
            extracted_claims.iss, None,
            "Expected 'iss' claim to be None when missing"
        );
        assert_eq!(
            extracted_claims.iat, None,
            "Expected 'iat' claim to be None when missing"
        );
        assert_eq!(
            extracted_claims.nbf, None,
            "Expected 'nbf' claim to be None when missing"
        );

        // Verify exp is still extracted
        assert_eq!(
            extracted_claims.exp,
            Some(9999999999),
            "Expected 'exp' claim to be extracted"
        );
    }

    #[test]
    fn test_handles_nested_claim_structures() {
        let secret = "test_secret";

        // Create a JWT with nested claim structures
        let mut custom_map = serde_json::Map::new();

        // Create nested address object
        let mut address_obj = serde_json::Map::new();
        address_obj.insert(
            "street".to_string(),
            serde_json::Value::String("123 Main St".to_string()),
        );
        address_obj.insert(
            "city".to_string(),
            serde_json::Value::String("San Francisco".to_string()),
        );
        address_obj.insert(
            "zip".to_string(),
            serde_json::Value::String("94102".to_string()),
        );

        custom_map.insert(
            "address".to_string(),
            serde_json::Value::Object(address_obj),
        );

        // Create nested metadata object
        let mut metadata_obj = serde_json::Map::new();
        metadata_obj.insert(
            "created_at".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1234567890)),
        );
        metadata_obj.insert("active".to_string(), serde_json::Value::Bool(true));

        custom_map.insert(
            "metadata".to_string(),
            serde_json::Value::Object(metadata_obj),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Verify nested address object is extracted correctly
        let address = extracted_claims
            .custom
            .get("address")
            .and_then(|v| v.as_object());
        assert!(
            address.is_some(),
            "Expected 'address' nested object to exist"
        );

        let address = address.unwrap();
        assert_eq!(
            address.get("street").and_then(|v| v.as_str()),
            Some("123 Main St"),
            "Nested 'address.street' not extracted correctly"
        );
        assert_eq!(
            address.get("city").and_then(|v| v.as_str()),
            Some("San Francisco"),
            "Nested 'address.city' not extracted correctly"
        );
        assert_eq!(
            address.get("zip").and_then(|v| v.as_str()),
            Some("94102"),
            "Nested 'address.zip' not extracted correctly"
        );

        // Verify nested metadata object is extracted correctly
        let metadata = extracted_claims
            .custom
            .get("metadata")
            .and_then(|v| v.as_object());
        assert!(
            metadata.is_some(),
            "Expected 'metadata' nested object to exist"
        );

        let metadata = metadata.unwrap();
        assert_eq!(
            metadata.get("created_at").and_then(|v| v.as_i64()),
            Some(1234567890),
            "Nested 'metadata.created_at' not extracted correctly"
        );
        assert_eq!(
            metadata.get("active").and_then(|v| v.as_bool()),
            Some(true),
            "Nested 'metadata.active' not extracted correctly"
        );
    }

    #[test]
    fn test_handles_array_claims() {
        let secret = "test_secret";

        // Create a JWT with array claims
        let mut custom_map = serde_json::Map::new();

        // Array of strings
        let roles_array = vec![
            serde_json::Value::String("admin".to_string()),
            serde_json::Value::String("user".to_string()),
            serde_json::Value::String("moderator".to_string()),
        ];
        custom_map.insert("roles".to_string(), serde_json::Value::Array(roles_array));

        // Array of numbers
        let scores_array = vec![
            serde_json::Value::Number(serde_json::Number::from(100)),
            serde_json::Value::Number(serde_json::Number::from(95)),
            serde_json::Value::Number(serde_json::Number::from(87)),
        ];
        custom_map.insert("scores".to_string(), serde_json::Value::Array(scores_array));

        // Array of mixed types
        let mixed_array = vec![
            serde_json::Value::String("test".to_string()),
            serde_json::Value::Number(serde_json::Number::from(42)),
            serde_json::Value::Bool(true),
        ];
        custom_map.insert("mixed".to_string(), serde_json::Value::Array(mixed_array));

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Verify roles array is extracted correctly
        let roles = extracted_claims
            .custom
            .get("roles")
            .and_then(|v| v.as_array());
        assert!(roles.is_some(), "Expected 'roles' array to exist");

        let roles = roles.unwrap();
        assert_eq!(roles.len(), 3, "Expected 3 roles in array");
        assert_eq!(
            roles[0].as_str(),
            Some("admin"),
            "First role should be 'admin'"
        );
        assert_eq!(
            roles[1].as_str(),
            Some("user"),
            "Second role should be 'user'"
        );
        assert_eq!(
            roles[2].as_str(),
            Some("moderator"),
            "Third role should be 'moderator'"
        );

        // Verify scores array is extracted correctly
        let scores = extracted_claims
            .custom
            .get("scores")
            .and_then(|v| v.as_array());
        assert!(scores.is_some(), "Expected 'scores' array to exist");

        let scores = scores.unwrap();
        assert_eq!(scores.len(), 3, "Expected 3 scores in array");
        assert_eq!(scores[0].as_i64(), Some(100), "First score should be 100");
        assert_eq!(scores[1].as_i64(), Some(95), "Second score should be 95");
        assert_eq!(scores[2].as_i64(), Some(87), "Third score should be 87");

        // Verify mixed array is extracted correctly
        let mixed = extracted_claims
            .custom
            .get("mixed")
            .and_then(|v| v.as_array());
        assert!(mixed.is_some(), "Expected 'mixed' array to exist");

        let mixed = mixed.unwrap();
        assert_eq!(mixed.len(), 3, "Expected 3 items in mixed array");
        assert_eq!(
            mixed[0].as_str(),
            Some("test"),
            "First item should be 'test'"
        );
        assert_eq!(mixed[1].as_i64(), Some(42), "Second item should be 42");
        assert_eq!(mixed[2].as_bool(), Some(true), "Third item should be true");
    }

    #[test]
    fn test_handles_null_claim_values() {
        let secret = "test_secret";

        // Create a JWT with null claim values
        let mut custom_map = serde_json::Map::new();
        custom_map.insert("middle_name".to_string(), serde_json::Value::Null);
        custom_map.insert("phone".to_string(), serde_json::Value::Null);
        custom_map.insert(
            "email".to_string(),
            serde_json::Value::String("user@example.com".to_string()),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Verify null values are handled correctly
        let middle_name = extracted_claims.custom.get("middle_name");
        assert!(middle_name.is_some(), "Expected 'middle_name' key to exist");
        assert!(
            middle_name.unwrap().is_null(),
            "Expected 'middle_name' value to be null"
        );

        let phone = extracted_claims.custom.get("phone");
        assert!(phone.is_some(), "Expected 'phone' key to exist");
        assert!(
            phone.unwrap().is_null(),
            "Expected 'phone' value to be null"
        );

        // Verify non-null value still works
        let email = extracted_claims.custom.get("email");
        assert!(email.is_some(), "Expected 'email' key to exist");
        assert_eq!(
            email.unwrap().as_str(),
            Some("user@example.com"),
            "Expected 'email' to have correct value"
        );
    }

    #[test]
    fn test_verifies_string_claim_equals_expected_value() {
        let secret = "test_secret";

        // Create a JWT with a custom string claim
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create claim verification rule
        let rules = vec![ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::String("admin".to_string()),
        }];

        // Verify claims
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass when string claim equals expected value"
        );
    }

    #[test]
    fn test_verifies_numeric_claim_equals_expected_value() {
        let secret = "test_secret";

        // Create a JWT with a custom numeric claim
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "user_id".to_string(),
            serde_json::Value::Number(serde_json::Number::from(12345)),
        );
        custom_map.insert(
            "age".to_string(),
            serde_json::Value::Number(serde_json::Number::from(30)),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create claim verification rule for user_id
        let rules = vec![ClaimRule {
            claim: "user_id".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::Number(serde_json::Number::from(12345)),
        }];

        // Verify claims
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass when numeric claim equals expected value"
        );

        // Create claim verification rule for age
        let rules = vec![ClaimRule {
            claim: "age".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::Number(serde_json::Number::from(30)),
        }];

        // Verify claims
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass when age claim equals expected value"
        );
    }

    #[test]
    fn test_verifies_boolean_claim_equals_expected_value() {
        let secret = "test_secret";

        // Create a JWT with custom boolean claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert("is_admin".to_string(), serde_json::Value::Bool(true));
        custom_map.insert("is_verified".to_string(), serde_json::Value::Bool(false));

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create claim verification rule for is_admin (true)
        let rules = vec![ClaimRule {
            claim: "is_admin".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::Bool(true),
        }];

        // Verify claims
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass when boolean claim equals true"
        );

        // Create claim verification rule for is_verified (false)
        let rules = vec![ClaimRule {
            claim: "is_verified".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::Bool(false),
        }];

        // Verify claims
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass when boolean claim equals false"
        );
    }

    #[test]
    fn test_fails_when_claim_value_doesnt_match() {
        let secret = "test_secret";

        // Create a JWT with custom claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("user".to_string()),
        );
        custom_map.insert(
            "level".to_string(),
            serde_json::Value::Number(serde_json::Number::from(5)),
        );
        custom_map.insert("is_admin".to_string(), serde_json::Value::Bool(false));

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Test 1: String claim value doesn't match
        let rules = vec![ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::String("admin".to_string()), // Expected admin, but JWT has user
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail when string value doesn't match"
        );

        // Test 2: Numeric claim value doesn't match
        let rules = vec![ClaimRule {
            claim: "level".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::Number(serde_json::Number::from(10)), // Expected 10, but JWT has 5
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail when numeric value doesn't match"
        );

        // Test 3: Boolean claim value doesn't match
        let rules = vec![ClaimRule {
            claim: "is_admin".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::Bool(true), // Expected true, but JWT has false
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail when boolean value doesn't match"
        );
    }

    #[test]
    fn test_fails_when_claim_is_missing() {
        let secret = "test_secret";

        // Create a JWT with only some claims (role is present, but admin_level is missing)
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("user".to_string()),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create rule for a claim that doesn't exist in the JWT
        let rules = vec![ClaimRule {
            claim: "admin_level".to_string(), // This claim is not in the JWT
            operator: "equals".to_string(),
            value: serde_json::Value::Number(serde_json::Number::from(5)),
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail when required claim is missing"
        );

        // Create another rule for a different missing claim
        let rules = vec![ClaimRule {
            claim: "department".to_string(), // This claim is also not in the JWT
            operator: "equals".to_string(),
            value: serde_json::Value::String("engineering".to_string()),
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail when required claim is missing (string)"
        );
    }

    #[test]
    fn test_case_sensitive_string_comparison() {
        let secret = "test_secret";

        // Create a JWT with a lowercase role claim
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Test 1: Exact match should pass
        let rules = vec![ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::String("admin".to_string()),
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass with exact case match"
        );

        // Test 2: Different case should fail (Admin vs admin)
        let rules = vec![ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::String("Admin".to_string()), // Capital A
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail with different case (Admin vs admin)"
        );

        // Test 3: All uppercase should fail (ADMIN vs admin)
        let rules = vec![ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::String("ADMIN".to_string()),
        }];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected claim verification to fail with different case (ADMIN vs admin)"
        );
    }

    #[test]
    fn test_passes_when_all_verification_rules_pass() {
        let secret = "test_secret";

        // Create a JWT with multiple claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );
        custom_map.insert(
            "level".to_string(),
            serde_json::Value::Number(serde_json::Number::from(10)),
        );
        custom_map.insert("is_active".to_string(), serde_json::Value::Bool(true));
        custom_map.insert(
            "department".to_string(),
            serde_json::Value::String("engineering".to_string()),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create multiple verification rules (all should pass)
        let rules = vec![
            ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("admin".to_string()),
            },
            ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Number(serde_json::Number::from(10)),
            },
            ClaimRule {
                claim: "is_active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(true),
            },
            ClaimRule {
                claim: "department".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("engineering".to_string()),
            },
        ];

        // Verify all claims
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected claim verification to pass when all rules match (AND logic)"
        );
    }

    #[test]
    fn test_fails_when_any_verification_rule_fails() {
        let secret = "test_secret";

        // Create a JWT with multiple claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );
        custom_map.insert(
            "level".to_string(),
            serde_json::Value::Number(serde_json::Number::from(10)),
        );
        custom_map.insert("is_active".to_string(), serde_json::Value::Bool(true));

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Test 1: First rule fails, rest pass
        let rules = vec![
            ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("user".to_string()), // Wrong value
            },
            ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Number(serde_json::Number::from(10)),
            },
            ClaimRule {
                claim: "is_active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(true),
            },
        ];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected verification to fail when first rule fails"
        );

        // Test 2: Middle rule fails, others pass
        let rules = vec![
            ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("admin".to_string()),
            },
            ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Number(serde_json::Number::from(5)), // Wrong value
            },
            ClaimRule {
                claim: "is_active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(true),
            },
        ];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected verification to fail when middle rule fails"
        );

        // Test 3: Last rule fails, others pass
        let rules = vec![
            ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("admin".to_string()),
            },
            ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Number(serde_json::Number::from(10)),
            },
            ClaimRule {
                claim: "is_active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(false), // Wrong value
            },
        ];

        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected verification to fail when last rule fails"
        );
    }

    #[test]
    fn test_handles_verification_with_empty_rules_list() {
        let secret = "test_secret";

        // Create a JWT with some claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );
        custom_map.insert(
            "level".to_string(),
            serde_json::Value::Number(serde_json::Number::from(10)),
        );

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create empty rules list
        let rules: Vec<ClaimRule> = vec![];

        // Verify with empty rules - should always pass
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            verified,
            "Expected verification to pass when rules list is empty (no requirements)"
        );
    }

    #[test]
    fn test_evaluates_all_rules_even_if_first_fails() {
        let secret = "test_secret";

        // Create a JWT with multiple claims
        let mut custom_map = serde_json::Map::new();
        custom_map.insert(
            "role".to_string(),
            serde_json::Value::String("user".to_string()),
        );
        custom_map.insert(
            "level".to_string(),
            serde_json::Value::Number(serde_json::Number::from(5)),
        );
        custom_map.insert("is_active".to_string(), serde_json::Value::Bool(false));

        let claims = Claims {
            sub: Some("user123".to_string()),
            exp: Some(9999999999),
            iat: None,
            nbf: None,
            iss: None,
            custom: custom_map,
        };

        // Encode the JWT
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to encode JWT");

        // Validate and extract claims
        let result = validate_jwt(&token, secret);
        assert!(result.is_ok(), "Expected valid JWT to be accepted");

        let extracted_claims = result.unwrap();

        // Create rules where multiple rules fail (not just the first one)
        // This tests that we could potentially report ALL failures, not just the first
        let rules = vec![
            ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("admin".to_string()), // Fails: expected admin, got user
            },
            ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Number(serde_json::Number::from(10)), // Fails: expected 10, got 5
            },
            ClaimRule {
                claim: "is_active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(true), // Fails: expected true, got false
            },
        ];

        // Even though all three rules fail, the function should still return false
        // The implementation may evaluate all rules (for future error reporting) or stop early
        // Either way, the result should be false
        let verified = verify_claims(&extracted_claims, &rules);
        assert!(
            !verified,
            "Expected verification to fail when multiple rules fail (supports future detailed error messages)"
        );
    }

    #[test]
    fn test_passes_request_through_when_auth_disabled() {
        use crate::config::TokenSource;

        // Test 1: Auth is explicitly disabled (enabled = false)
        let jwt_config = Some(JwtConfig {
            enabled: false,
            secret: "test_secret".to_string(),
            algorithm: "HS256".to_string(),
            token_sources: vec![],
            claims: vec![],
        });

        let auth_required = is_auth_required(&jwt_config);
        assert!(
            !auth_required,
            "Expected auth not to be required when enabled=false"
        );

        // Test 2: No JWT config at all (None)
        let no_jwt_config: Option<JwtConfig> = None;

        let auth_required = is_auth_required(&no_jwt_config);
        assert!(
            !auth_required,
            "Expected auth not to be required when JWT config is None"
        );

        // Test 3: Auth is enabled (enabled = true)
        let jwt_config_enabled = Some(JwtConfig {
            enabled: true,
            secret: "test_secret".to_string(),
            algorithm: "HS256".to_string(),
            token_sources: vec![TokenSource {
                source_type: "header".to_string(),
                name: Some("Authorization".to_string()),
                prefix: Some("Bearer ".to_string()),
            }],
            claims: vec![],
        });

        let auth_required = is_auth_required(&jwt_config_enabled);
        assert!(
            auth_required,
            "Expected auth to be required when enabled=true"
        );
    }
}
