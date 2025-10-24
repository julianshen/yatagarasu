// Authentication module

use std::collections::HashMap;

pub fn extract_bearer_token(headers: &HashMap<String, String>) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
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
}
