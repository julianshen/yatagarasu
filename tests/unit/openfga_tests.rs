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
