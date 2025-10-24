// Authentication module

use std::collections::HashMap;

pub fn extract_bearer_token(headers: &HashMap<String, String>) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.trim().is_empty())
        .map(|token| token.to_string())
}

pub fn extract_header_token(
    headers: &HashMap<String, String>,
    header_name: &str,
) -> Option<String> {
    headers.get(header_name).map(|value| value.to_string())
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
}
