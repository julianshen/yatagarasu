// Authentication module

use std::collections::HashMap;

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
}

pub fn extract_query_token(
    query_params: &HashMap<String, String>,
    param_name: &str,
) -> Option<String> {
    query_params.get(param_name).cloned()
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
}
