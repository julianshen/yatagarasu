// Pipeline integration unit tests
// Phase 13: Request Pipeline Integration

use yatagarasu::pipeline::RequestContext;

// Test: Can create RequestContext from HTTP request
#[test]
fn test_can_create_request_context_from_http_request() {
    // Create a RequestContext with minimal HTTP request information
    let method = "GET";
    let path = "/products/image.png";

    let context = RequestContext::new(method.to_string(), path.to_string());

    // Verify the context was created successfully
    assert_eq!(context.method(), "GET");
    assert_eq!(context.path(), "/products/image.png");
}

// Test: RequestContext includes request ID (UUID)
#[test]
fn test_request_context_includes_request_id() {
    // Create two RequestContext instances
    let context1 = RequestContext::new("GET".to_string(), "/test1".to_string());
    let context2 = RequestContext::new("GET".to_string(), "/test2".to_string());

    // Each context should have a unique request ID
    let id1 = context1.request_id();
    let id2 = context2.request_id();

    // Request IDs should not be empty
    assert!(!id1.is_empty(), "Request ID should not be empty");
    assert!(!id2.is_empty(), "Request ID should not be empty");

    // Request IDs should be unique (different for each request)
    assert_ne!(id1, id2, "Each request should have a unique ID");

    // Request ID should be a valid UUID format (basic check: contains hyphens)
    assert!(id1.contains('-'), "Request ID should be in UUID format");
    assert_eq!(id1.len(), 36, "UUID should be 36 characters long");
}

// Test: RequestContext includes request method
#[test]
fn test_request_context_includes_request_method() {
    // Create contexts with different HTTP methods
    let get_context = RequestContext::new("GET".to_string(), "/test".to_string());
    let post_context = RequestContext::new("POST".to_string(), "/api/data".to_string());
    let put_context = RequestContext::new("PUT".to_string(), "/resource/123".to_string());
    let delete_context = RequestContext::new("DELETE".to_string(), "/item/456".to_string());

    // Verify each context stores and returns the correct method
    assert_eq!(get_context.method(), "GET");
    assert_eq!(post_context.method(), "POST");
    assert_eq!(put_context.method(), "PUT");
    assert_eq!(delete_context.method(), "DELETE");
}

// Test: RequestContext includes request path
#[test]
fn test_request_context_includes_request_path() {
    // Create contexts with different request paths
    let simple_path = RequestContext::new("GET".to_string(), "/".to_string());
    let api_path = RequestContext::new("GET".to_string(), "/api/v1/users".to_string());
    let resource_path = RequestContext::new("GET".to_string(), "/products/123/details".to_string());
    let file_path = RequestContext::new("GET".to_string(), "/images/logo.png".to_string());
    let query_path = RequestContext::new("GET".to_string(), "/search?q=test&page=1".to_string());

    // Verify each context stores and returns the correct path
    assert_eq!(simple_path.path(), "/");
    assert_eq!(api_path.path(), "/api/v1/users");
    assert_eq!(resource_path.path(), "/products/123/details");
    assert_eq!(file_path.path(), "/images/logo.png");
    assert_eq!(query_path.path(), "/search?q=test&page=1");
}
