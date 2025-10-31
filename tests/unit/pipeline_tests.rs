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
