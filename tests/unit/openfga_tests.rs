//! Unit tests for OpenFGA client implementation

use yatagarasu::openfga::{Error, OpenFgaClient, TupleKey};

#[test]
fn test_can_create_openfga_client() {
    let endpoint = "http://localhost:8080";
    let store_id = "01H0EXAMPLE";

    let client = OpenFgaClient::new(endpoint, store_id);

    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.endpoint(), endpoint);
    assert_eq!(client.store_id(), store_id);
}

#[test]
fn test_openfga_client_handles_empty_endpoint() {
    let endpoint = "";
    let store_id = "01H0EXAMPLE";

    let result = OpenFgaClient::new(endpoint, store_id);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidConfig(msg) => {
            assert!(msg.contains("endpoint"), "Error should mention endpoint");
        }
        _ => panic!("Expected InvalidConfig error"),
    }
}

#[test]
fn test_openfga_client_handles_empty_store_id() {
    let endpoint = "http://localhost:8080";
    let store_id = "";

    let result = OpenFgaClient::new(endpoint, store_id);

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidConfig(msg) => {
            assert!(msg.contains("store_id"), "Error should mention store_id");
        }
        _ => panic!("Expected InvalidConfig error"),
    }
}

#[test]
fn test_openfga_client_builder_with_api_token() {
    let client = OpenFgaClient::builder("http://localhost:8080", "01H0EXAMPLE")
        .api_token("secret-token")
        .build()
        .unwrap();

    assert_eq!(client.api_token(), Some("secret-token"));
}

#[test]
fn test_openfga_client_builder_with_timeout() {
    let client = OpenFgaClient::builder("http://localhost:8080", "01H0EXAMPLE")
        .timeout_ms(500)
        .build()
        .unwrap();

    assert_eq!(client.timeout().as_millis(), 500);
}

#[test]
fn test_tuple_key_creation() {
    let tuple = TupleKey::new("user:alice", "viewer", "document:readme");

    assert_eq!(tuple.user, "user:alice");
    assert_eq!(tuple.relation, "viewer");
    assert_eq!(tuple.object, "document:readme");
}

#[test]
fn test_tuple_key_with_different_types() {
    // Test with folder type
    let folder_tuple = TupleKey::new("user:bob", "editor", "folder:shared");
    assert_eq!(folder_tuple.user, "user:bob");
    assert_eq!(folder_tuple.relation, "editor");
    assert_eq!(folder_tuple.object, "folder:shared");

    // Test with owner relation
    let owner_tuple = TupleKey::new("user:admin", "owner", "bucket:private");
    assert_eq!(owner_tuple.user, "user:admin");
    assert_eq!(owner_tuple.relation, "owner");
    assert_eq!(owner_tuple.object, "bucket:private");
}

#[tokio::test]
async fn test_check_handles_connection_error() {
    // Client pointing to non-existent server
    let client = OpenFgaClient::builder("http://127.0.0.1:19999", "01H0EXAMPLE")
        .timeout_ms(100) // Short timeout
        .build()
        .unwrap();

    let result = client
        .check("user:alice", "viewer", "document:readme")
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        Error::Connection(msg) => {
            assert!(
                msg.contains("connect") || msg.contains("Connection"),
                "Error should mention connection: {}",
                msg
            );
        }
        err => panic!("Expected Connection error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_check_with_authorization_model_id() {
    // Create client with authorization_model_id set
    let client = OpenFgaClient::builder("http://127.0.0.1:19998", "01H0TEST")
        .authorization_model_id("01GXSA8YR785C4FYS3C0RTG7B1")
        .timeout_ms(100)
        .build()
        .unwrap();

    // Verify the model ID is set
    assert_eq!(
        client.authorization_model_id(),
        Some("01GXSA8YR785C4FYS3C0RTG7B1")
    );

    // The check will fail due to connection error, but that's expected
    let result = client.check("user:test", "viewer", "doc:test").await;
    assert!(result.is_err());
}

// Phase 49.2: Request Authorization Flow Tests
use serde_json::json;
use yatagarasu::openfga::{
    build_openfga_object, extract_user_id, http_method_to_relation, Relation,
};

#[test]
fn test_extract_user_id_from_sub_claim() {
    // Default claim is "sub"
    let claims = json!({
        "sub": "user123",
        "email": "test@example.com"
    });

    let user_id = extract_user_id(&claims, None);
    assert_eq!(user_id, Some("user:user123".to_string()));
}

#[test]
fn test_extract_user_id_from_custom_claim() {
    // Custom claim extraction
    let claims = json!({
        "sub": "user123",
        "user_id": "custom_id_456",
        "email": "test@example.com"
    });

    let user_id = extract_user_id(&claims, Some("user_id"));
    assert_eq!(user_id, Some("user:custom_id_456".to_string()));
}

#[test]
fn test_extract_user_id_missing_claim_returns_none() {
    let claims = json!({
        "email": "test@example.com"
    });

    // No "sub" claim
    let user_id = extract_user_id(&claims, None);
    assert!(user_id.is_none());
}

#[test]
fn test_extract_user_id_from_nested_claim() {
    // Support for nested claims using dot notation
    let claims = json!({
        "user": {
            "id": "nested_user_789"
        },
        "email": "test@example.com"
    });

    let user_id = extract_user_id(&claims, Some("user.id"));
    assert_eq!(user_id, Some("user:nested_user_789".to_string()));
}

#[test]
fn test_build_openfga_object_for_file() {
    // File object: file:{bucket}/{path}
    let object = build_openfga_object("my-bucket", "documents/report.pdf");
    assert_eq!(object, "file:my-bucket/documents/report.pdf");
}

#[test]
fn test_build_openfga_object_for_folder() {
    // Folder object: folder:{bucket}/{path}/
    let object = build_openfga_object("my-bucket", "documents/");
    assert_eq!(object, "folder:my-bucket/documents/");
}

#[test]
fn test_build_openfga_object_for_bucket_root() {
    // Bucket root: bucket:{bucket}
    let object = build_openfga_object("my-bucket", "");
    assert_eq!(object, "bucket:my-bucket");
}

#[test]
fn test_build_openfga_object_normalizes_path() {
    // Should normalize paths (remove leading slash)
    let object = build_openfga_object("my-bucket", "/documents/report.pdf");
    assert_eq!(object, "file:my-bucket/documents/report.pdf");
}

#[test]
fn test_http_method_to_relation_get() {
    let relation = http_method_to_relation("GET");
    assert_eq!(relation, Relation::Viewer);
}

#[test]
fn test_http_method_to_relation_head() {
    let relation = http_method_to_relation("HEAD");
    assert_eq!(relation, Relation::Viewer);
}

#[test]
fn test_http_method_to_relation_put() {
    let relation = http_method_to_relation("PUT");
    assert_eq!(relation, Relation::Editor);
}

#[test]
fn test_http_method_to_relation_post() {
    let relation = http_method_to_relation("POST");
    assert_eq!(relation, Relation::Editor);
}

#[test]
fn test_http_method_to_relation_delete() {
    let relation = http_method_to_relation("DELETE");
    assert_eq!(relation, Relation::Owner);
}

#[test]
fn test_relation_to_string() {
    assert_eq!(Relation::Viewer.as_str(), "viewer");
    assert_eq!(Relation::Editor.as_str(), "editor");
    assert_eq!(Relation::Owner.as_str(), "owner");
}

// Phase 49.2: Authorization Decision Tests
use yatagarasu::openfga::{AuthorizationDecision, FailMode};

#[test]
fn test_authorization_decision_allowed() {
    let decision = AuthorizationDecision::allowed();
    assert!(decision.is_allowed());
    assert!(!decision.is_fail_open_allow());
    assert!(decision.error().is_none());
}

#[test]
fn test_authorization_decision_denied() {
    let decision = AuthorizationDecision::denied();
    assert!(!decision.is_allowed());
    assert!(!decision.is_fail_open_allow());
    assert!(decision.error().is_none());
}

#[test]
fn test_authorization_decision_from_check_result_allowed() {
    // When check returns Ok(true), request should be allowed
    let result: Result<bool, Error> = Ok(true);
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Closed);

    assert!(decision.is_allowed());
    assert!(!decision.is_fail_open_allow());
    assert!(decision.error().is_none());
}

#[test]
fn test_authorization_decision_from_check_result_denied() {
    // When check returns Ok(false), request should be denied (403)
    let result: Result<bool, Error> = Ok(false);
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Closed);

    assert!(
        !decision.is_allowed(),
        "Should return 403 on authorization failure"
    );
    assert!(!decision.is_fail_open_allow());
    assert!(decision.error().is_none());
}

#[test]
fn test_authorization_decision_fail_closed_on_error() {
    // When OpenFGA returns an error with FailMode::Closed, request should be denied (500)
    let result: Result<bool, Error> = Err(Error::Connection("Failed to connect".to_string()));
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Closed);

    assert!(
        !decision.is_allowed(),
        "Should return 500 (deny) on OpenFGA error with fail-closed mode"
    );
    assert!(!decision.is_fail_open_allow());
    assert!(decision.has_error());
    assert!(decision.error().unwrap().contains("Failed to connect"));
}

#[test]
fn test_authorization_decision_fail_open_on_error() {
    // When OpenFGA returns an error with FailMode::Open, request should be allowed
    let result: Result<bool, Error> = Err(Error::Connection("Timeout".to_string()));
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Open);

    assert!(
        decision.is_allowed(),
        "Should allow on OpenFGA error with fail-open mode"
    );
    assert!(
        decision.is_fail_open_allow(),
        "Should be marked as fail-open allow"
    );
    assert!(decision.has_error());
    assert!(decision.error().unwrap().contains("Timeout"));
}

#[test]
fn test_fail_mode_from_str_open() {
    let mode: FailMode = "open".parse().unwrap();
    assert_eq!(mode, FailMode::Open);

    let mode: FailMode = "Open".parse().unwrap();
    assert_eq!(mode, FailMode::Open);

    let mode: FailMode = "OPEN".parse().unwrap();
    assert_eq!(mode, FailMode::Open);
}

#[test]
fn test_fail_mode_from_str_closed() {
    let mode: FailMode = "closed".parse().unwrap();
    assert_eq!(mode, FailMode::Closed);

    let mode: FailMode = "Closed".parse().unwrap();
    assert_eq!(mode, FailMode::Closed);
}

#[test]
fn test_fail_mode_from_str_defaults_to_closed() {
    // Unknown values should default to closed for security
    let mode: FailMode = "unknown".parse().unwrap();
    assert_eq!(mode, FailMode::Closed);

    let mode: FailMode = "".parse().unwrap();
    assert_eq!(mode, FailMode::Closed);
}

#[test]
fn test_fail_mode_default_is_closed() {
    let mode = FailMode::default();
    assert_eq!(mode, FailMode::Closed);
}

#[test]
fn test_authorization_decision_api_error_fail_closed() {
    // API errors (like 404 store not found) should also respect fail mode
    let result: Result<bool, Error> = Err(Error::Api("Store 'nonexistent' not found".to_string()));
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Closed);

    assert!(!decision.is_allowed());
    assert!(decision.has_error());
    assert!(decision.error().unwrap().contains("Store"));
}

#[test]
fn test_authorization_decision_timeout_fail_open() {
    // Timeout errors with fail-open mode should allow
    let result: Result<bool, Error> = Err(Error::Connection("Request timed out".to_string()));
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Open);

    assert!(decision.is_allowed());
    assert!(decision.is_fail_open_allow());
}
