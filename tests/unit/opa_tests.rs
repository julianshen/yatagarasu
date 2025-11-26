//! Tests for OPA (Open Policy Agent) integration
//!
//! Phase 32.2: OPA Client Implementation

// ============================================================================
// Phase 32.2: OPA HTTP Client Tests
// ============================================================================

#[test]
fn test_can_create_opa_client_struct() {
    use yatagarasu::opa::{OpaClient, OpaClientConfig};

    // Test: Can create OpaClient struct with configuration
    let config = OpaClientConfig {
        url: "http://localhost:8181".to_string(),
        policy_path: "authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    // Verify client is created successfully
    assert!(client.config().url == "http://localhost:8181");
}

#[test]
fn test_opa_client_contains_config() {
    use yatagarasu::opa::{OpaClient, OpaClientConfig};

    // Test: OpaClient contains config (URL, timeout, cache TTL)
    let config = OpaClientConfig {
        url: "http://opa.example.com:8181".to_string(),
        policy_path: "myapp/authz".to_string(),
        timeout_ms: 3000,
        cache_ttl_seconds: 120,
    };

    let client = OpaClient::new(config);
    let retrieved_config = client.config();

    assert_eq!(retrieved_config.url, "http://opa.example.com:8181");
    assert_eq!(retrieved_config.policy_path, "myapp/authz");
    assert_eq!(retrieved_config.timeout_ms, 3000);
    assert_eq!(retrieved_config.cache_ttl_seconds, 120);
}

#[test]
fn test_opa_client_is_send_sync() {
    use yatagarasu::opa::{OpaClient, OpaClientConfig};

    // Test: OpaClient is Send + Sync (required for concurrent use)
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OpaClient>();

    // Also verify it can be wrapped in Arc
    let config = OpaClientConfig {
        url: "http://localhost:8181".to_string(),
        policy_path: "authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };
    let client = OpaClient::new(config);
    let _arc_client = std::sync::Arc::new(client);
}

// ============================================================================
// Phase 32.2: OPA Request/Response Types Tests
// ============================================================================

#[test]
fn test_can_create_opa_input_struct() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Can create OpaInput struct (request context)
    let claims = json!({
        "sub": "user123",
        "roles": ["admin"]
    });

    let input = OpaInput::new(
        claims,
        "products".to_string(),
        "/products/file.txt".to_string(),
        "GET".to_string(),
        Some("192.168.1.1".to_string()),
    );

    assert_eq!(input.bucket(), "products");
    assert_eq!(input.path(), "/products/file.txt");
}

#[test]
fn test_opa_input_contains_jwt_claims() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput contains jwt_claims (JSON object)
    let claims = json!({
        "sub": "user456",
        "roles": ["viewer", "editor"],
        "department": "engineering"
    });

    let input = OpaInput::new(
        claims.clone(),
        "bucket".to_string(),
        "/path".to_string(),
        "GET".to_string(),
        None,
    );

    assert_eq!(input.jwt_claims()["sub"], "user456");
    assert_eq!(input.jwt_claims()["department"], "engineering");
}

#[test]
fn test_opa_input_contains_bucket_name() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput contains bucket name
    let input = OpaInput::new(
        json!({}),
        "my-secure-bucket".to_string(),
        "/path".to_string(),
        "GET".to_string(),
        None,
    );

    assert_eq!(input.bucket(), "my-secure-bucket");
}

#[test]
fn test_opa_input_contains_request_path() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput contains request_path
    let input = OpaInput::new(
        json!({}),
        "bucket".to_string(),
        "/products/images/logo.png".to_string(),
        "GET".to_string(),
        None,
    );

    assert_eq!(input.path(), "/products/images/logo.png");
}

#[test]
fn test_opa_input_contains_http_method() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput contains http_method (GET/HEAD)
    let input_get = OpaInput::new(
        json!({}),
        "bucket".to_string(),
        "/path".to_string(),
        "GET".to_string(),
        None,
    );
    assert_eq!(input_get.method(), "GET");

    let input_head = OpaInput::new(
        json!({}),
        "bucket".to_string(),
        "/path".to_string(),
        "HEAD".to_string(),
        None,
    );
    assert_eq!(input_head.method(), "HEAD");
}

#[test]
fn test_opa_input_contains_client_ip() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput contains client_ip
    let input = OpaInput::new(
        json!({}),
        "bucket".to_string(),
        "/path".to_string(),
        "GET".to_string(),
        Some("10.0.0.50".to_string()),
    );

    assert_eq!(input.client_ip(), Some("10.0.0.50"));
}

#[test]
fn test_opa_input_serializes_to_json_correctly() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput serializes to JSON correctly matching OPA REST API format
    let claims = json!({
        "sub": "user123",
        "roles": ["admin", "viewer"],
        "department": "engineering"
    });

    let input = OpaInput::new(
        claims,
        "products".to_string(),
        "/products/secret/file.txt".to_string(),
        "GET".to_string(),
        Some("192.168.1.100".to_string()),
    );

    let serialized = serde_json::to_value(&input).unwrap();

    // Verify the structure matches OPA REST API format
    assert_eq!(serialized["jwt_claims"]["sub"], "user123");
    assert_eq!(serialized["jwt_claims"]["roles"][0], "admin");
    assert_eq!(serialized["bucket"], "products");
    assert_eq!(serialized["path"], "/products/secret/file.txt");
    assert_eq!(serialized["method"], "GET");
    assert_eq!(serialized["client_ip"], "192.168.1.100");
}

#[test]
fn test_opa_input_serializes_without_client_ip_when_none() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: OpaInput serializes correctly when client_ip is None
    let input = OpaInput::new(
        json!({"sub": "user"}),
        "bucket".to_string(),
        "/path".to_string(),
        "GET".to_string(),
        None,
    );

    let serialized = serde_json::to_value(&input).unwrap();

    // client_ip should be null or missing when None
    assert!(serialized.get("client_ip").is_none() || serialized["client_ip"].is_null());
}

#[test]
fn test_can_parse_opa_response_allow_true() {
    use yatagarasu::opa::OpaResponse;

    // Test: Can parse OpaResponse when allow is true
    let json_str = r#"{"result": true}"#;
    let response: OpaResponse = serde_json::from_str(json_str).unwrap();

    assert!(response.is_allowed());
    assert!(response.reason().is_none());
}

#[test]
fn test_can_parse_opa_response_allow_false() {
    use yatagarasu::opa::OpaResponse;

    // Test: Can parse OpaResponse when allow is false
    let json_str = r#"{"result": false}"#;
    let response: OpaResponse = serde_json::from_str(json_str).unwrap();

    assert!(!response.is_allowed());
}

#[test]
fn test_can_parse_opa_response_with_reason() {
    use yatagarasu::opa::OpaResponse;

    // Test: Can parse OpaResponse with optional reason
    let json_str = r#"{"result": {"allow": true, "reason": "User has admin role"}}"#;
    let response: OpaResponse = serde_json::from_str(json_str).unwrap();

    assert!(response.is_allowed());
    assert_eq!(response.reason(), Some("User has admin role"));
}

#[test]
fn test_can_parse_opa_response_deny_with_reason() {
    use yatagarasu::opa::OpaResponse;

    // Test: Can parse OpaResponse deny with reason
    let json_str = r#"{"result": {"allow": false, "reason": "Insufficient permissions"}}"#;
    let response: OpaResponse = serde_json::from_str(json_str).unwrap();

    assert!(!response.is_allowed());
    assert_eq!(response.reason(), Some("Insufficient permissions"));
}

#[test]
fn test_opa_request_format_matches_api_spec() {
    use serde_json::json;
    use yatagarasu::opa::{OpaInput, OpaRequest};

    // Test: Request format matches OPA REST API specification
    // Expected format:
    // {
    //   "input": {
    //     "jwt_claims": {...},
    //     "bucket": "...",
    //     "path": "...",
    //     "method": "...",
    //     "client_ip": "..."
    //   }
    // }

    let claims = json!({
        "sub": "user123",
        "roles": ["admin", "viewer"],
        "department": "engineering"
    });

    let input = OpaInput::new(
        claims,
        "products".to_string(),
        "/products/secret/file.txt".to_string(),
        "GET".to_string(),
        Some("192.168.1.100".to_string()),
    );

    let request = OpaRequest::new(input);
    let serialized = serde_json::to_value(&request).unwrap();

    // Verify wrapped in "input" key
    assert!(serialized.get("input").is_some());
    assert_eq!(serialized["input"]["jwt_claims"]["sub"], "user123");
    assert_eq!(serialized["input"]["bucket"], "products");
    assert_eq!(serialized["input"]["path"], "/products/secret/file.txt");
    assert_eq!(serialized["input"]["method"], "GET");
    assert_eq!(serialized["input"]["client_ip"], "192.168.1.100");
}

// ============================================================================
// Phase 32.3: OPA Policy Evaluation Tests
// ============================================================================

#[test]
fn test_opa_error_variants_exist() {
    use yatagarasu::opa::OpaError;

    // Test: OpaError has all required variants
    let timeout = OpaError::Timeout {
        policy_path: "authz/allow".to_string(),
        timeout_ms: 100,
    };
    let connection_failed = OpaError::ConnectionFailed("connection refused".to_string());
    let policy_error = OpaError::PolicyError {
        message: "undefined decision".to_string(),
    };
    let invalid_response = OpaError::InvalidResponse("malformed JSON".to_string());

    // Verify each variant can be created
    assert!(matches!(timeout, OpaError::Timeout { .. }));
    assert!(matches!(connection_failed, OpaError::ConnectionFailed(_)));
    assert!(matches!(policy_error, OpaError::PolicyError { .. }));
    assert!(matches!(invalid_response, OpaError::InvalidResponse(_)));
}

#[test]
fn test_opa_error_display_timeout() {
    use yatagarasu::opa::OpaError;

    // Test: Timeout error includes policy path for debugging
    let error = OpaError::Timeout {
        policy_path: "authz/allow".to_string(),
        timeout_ms: 100,
    };

    let display = format!("{}", error);
    assert!(
        display.contains("authz/allow"),
        "Display should include policy path"
    );
    assert!(
        display.contains("100"),
        "Display should include timeout value"
    );
}

#[test]
fn test_opa_error_display_connection_failed() {
    use yatagarasu::opa::OpaError;

    // Test: Connection failed error displays message
    let error = OpaError::ConnectionFailed("connection refused".to_string());
    let display = format!("{}", error);
    assert!(
        display.contains("connection refused"),
        "Display should include error message"
    );
}

#[test]
fn test_opa_error_display_policy_error() {
    use yatagarasu::opa::OpaError;

    // Test: Policy error displays OPA error message
    let error = OpaError::PolicyError {
        message: "undefined decision".to_string(),
    };
    let display = format!("{}", error);
    assert!(
        display.contains("undefined decision"),
        "Display should include OPA message"
    );
}

#[test]
fn test_opa_error_display_invalid_response() {
    use yatagarasu::opa::OpaError;

    // Test: Invalid response error displays details
    let error = OpaError::InvalidResponse("malformed JSON".to_string());
    let display = format!("{}", error);
    assert!(
        display.contains("malformed JSON"),
        "Display should include error details"
    );
}

#[test]
fn test_opa_client_builds_correct_endpoint_url() {
    use yatagarasu::opa::{OpaClient, OpaClientConfig};

    // Test: OPA client builds correct endpoint URL
    let config = OpaClientConfig {
        url: "http://localhost:8181".to_string(),
        policy_path: "authz/allow".to_string(),
        timeout_ms: 100,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);
    let endpoint = client.policy_endpoint();

    // Should be: {base_url}/v1/data/{policy_path}
    assert_eq!(endpoint, "http://localhost:8181/v1/data/authz/allow");
}

#[test]
fn test_opa_client_builds_endpoint_with_nested_policy_path() {
    use yatagarasu::opa::{OpaClient, OpaClientConfig};

    // Test: OPA client handles nested policy paths correctly
    let config = OpaClientConfig {
        url: "http://opa.example.com:8181".to_string(),
        policy_path: "myapp/authz/s3/allow".to_string(),
        timeout_ms: 100,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);
    let endpoint = client.policy_endpoint();

    assert_eq!(
        endpoint,
        "http://opa.example.com:8181/v1/data/myapp/authz/s3/allow"
    );
}

#[test]
fn test_opa_client_default_timeout_is_100ms() {
    use yatagarasu::opa::OpaClientConfig;

    // Test: Default timeout is 100ms
    let config = OpaClientConfig::default_timeout();
    assert_eq!(
        config, 100,
        "Default OPA timeout should be 100ms for fast fail"
    );
}

#[test]
fn test_opa_evaluation_result_from_response() {
    use yatagarasu::opa::OpaResponse;

    // Test: Can extract evaluation result from OPA response

    // Test allow=true
    let allow_json = r#"{"result": true}"#;
    let allow_response: OpaResponse = serde_json::from_str(allow_json).unwrap();
    assert!(allow_response.is_allowed());

    // Test allow=false
    let deny_json = r#"{"result": false}"#;
    let deny_response: OpaResponse = serde_json::from_str(deny_json).unwrap();
    assert!(!deny_response.is_allowed());

    // Test undefined (null) - should default to deny
    let undefined_json = r#"{"result": null}"#;
    let undefined_response: OpaResponse = serde_json::from_str(undefined_json).unwrap();
    assert!(!undefined_response.is_allowed());
}

#[test]
fn test_opa_response_handles_empty_result() {
    use yatagarasu::opa::OpaResponse;

    // Test: Returns false when OPA returns empty result (undefined)
    // When policy doesn't match, OPA returns: {}
    let empty_json = r#"{}"#;
    let result = serde_json::from_str::<OpaResponse>(empty_json);

    // Empty result should either be an error or default to deny
    match result {
        Ok(response) => assert!(!response.is_allowed(), "Empty result should deny"),
        Err(_) => {} // Parse error is also acceptable for invalid response
    }
}

// ============================================================================
// Phase 32.4: OPA Response Caching Tests
// ============================================================================

#[test]
fn test_opa_input_cache_key_generation() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Cache key based on hash of OpaInput
    let input = OpaInput::new(
        json!({"sub": "user123", "roles": ["admin"]}),
        "products".to_string(),
        "/products/file.txt".to_string(),
        "GET".to_string(),
        Some("192.168.1.1".to_string()),
    );

    let cache_key = input.cache_key();
    assert!(!cache_key.is_empty(), "Cache key should not be empty");
}

#[test]
fn test_same_input_produces_same_cache_key() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Same input produces same cache key
    let input1 = OpaInput::new(
        json!({"sub": "user123", "roles": ["admin"]}),
        "products".to_string(),
        "/products/file.txt".to_string(),
        "GET".to_string(),
        Some("192.168.1.1".to_string()),
    );

    let input2 = OpaInput::new(
        json!({"sub": "user123", "roles": ["admin"]}),
        "products".to_string(),
        "/products/file.txt".to_string(),
        "GET".to_string(),
        Some("192.168.1.1".to_string()),
    );

    assert_eq!(
        input1.cache_key(),
        input2.cache_key(),
        "Same inputs should produce same cache key"
    );
}

#[test]
fn test_different_inputs_produce_different_cache_keys() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Different inputs produce different cache keys
    let input1 = OpaInput::new(
        json!({"sub": "user123"}),
        "products".to_string(),
        "/products/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let input2 = OpaInput::new(
        json!({"sub": "user456"}), // Different user
        "products".to_string(),
        "/products/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    assert_ne!(
        input1.cache_key(),
        input2.cache_key(),
        "Different inputs should produce different cache keys"
    );
}

#[test]
fn test_cache_key_differs_by_bucket() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Cache key differs when bucket differs
    let input1 = OpaInput::new(
        json!({"sub": "user123"}),
        "products".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let input2 = OpaInput::new(
        json!({"sub": "user123"}),
        "private".to_string(), // Different bucket
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    assert_ne!(
        input1.cache_key(),
        input2.cache_key(),
        "Different buckets should produce different cache keys"
    );
}

#[test]
fn test_cache_key_differs_by_path() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Cache key differs when path differs
    let input1 = OpaInput::new(
        json!({"sub": "user123"}),
        "products".to_string(),
        "/products/file1.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let input2 = OpaInput::new(
        json!({"sub": "user123"}),
        "products".to_string(),
        "/products/file2.txt".to_string(), // Different path
        "GET".to_string(),
        None,
    );

    assert_ne!(
        input1.cache_key(),
        input2.cache_key(),
        "Different paths should produce different cache keys"
    );
}

#[test]
fn test_cache_key_differs_by_method() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Cache key differs when method differs
    let input1 = OpaInput::new(
        json!({"sub": "user123"}),
        "products".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let input2 = OpaInput::new(
        json!({"sub": "user123"}),
        "products".to_string(),
        "/file.txt".to_string(),
        "HEAD".to_string(), // Different method
        None,
    );

    assert_ne!(
        input1.cache_key(),
        input2.cache_key(),
        "Different methods should produce different cache keys"
    );
}

#[test]
fn test_cache_key_is_deterministic() {
    use serde_json::json;
    use yatagarasu::opa::OpaInput;

    // Test: Cache key is deterministic (same input always produces same key)
    let input = OpaInput::new(
        json!({"sub": "user123", "roles": ["admin"], "department": "engineering"}),
        "products".to_string(),
        "/products/secret/file.txt".to_string(),
        "GET".to_string(),
        Some("10.0.0.1".to_string()),
    );

    let key1 = input.cache_key();
    let key2 = input.cache_key();
    let key3 = input.cache_key();

    assert_eq!(key1, key2, "Cache key should be deterministic");
    assert_eq!(key2, key3, "Cache key should be deterministic");
}

#[tokio::test]
async fn test_opa_cache_can_store_and_retrieve() {
    use yatagarasu::opa::OpaCache;

    // Test: OPA cache can store and retrieve decisions
    let cache = OpaCache::new(60); // 60 second TTL

    let key = "test_key_123".to_string();
    cache.put(key.clone(), true).await;

    let result = cache.get(&key).await;
    assert_eq!(result, Some(true), "Should retrieve cached allow decision");
}

#[tokio::test]
async fn test_opa_cache_returns_none_for_missing_key() {
    use yatagarasu::opa::OpaCache;

    // Test: Cache miss returns None
    let cache = OpaCache::new(60);

    let result = cache.get("nonexistent_key").await;
    assert!(result.is_none(), "Cache miss should return None");
}

#[tokio::test]
async fn test_opa_cache_stores_deny_decisions() {
    use yatagarasu::opa::OpaCache;

    // Test: Cache can store deny (false) decisions
    let cache = OpaCache::new(60);

    let key = "deny_key".to_string();
    cache.put(key.clone(), false).await;

    let result = cache.get(&key).await;
    assert_eq!(result, Some(false), "Should retrieve cached deny decision");
}
