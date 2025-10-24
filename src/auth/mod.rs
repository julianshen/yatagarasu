// Authentication module

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    validation.validate_exp = false; // We'll handle expiration separately in later tests
    validation.validate_nbf = false; // We'll handle not-before separately in later tests
    validation.required_spec_claims.clear(); // Don't require exp, nbf, etc.

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;

    Ok(token_data.claims)
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
}
