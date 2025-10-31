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
