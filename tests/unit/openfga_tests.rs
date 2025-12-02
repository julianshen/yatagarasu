//! Unit tests for OpenFGA client implementation

use yatagarasu::openfga::{Error, OpenFgaClient};

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
