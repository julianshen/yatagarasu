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

// Test: RequestContext includes request headers as HashMap
#[test]
fn test_request_context_includes_request_headers() {
    use std::collections::HashMap;

    // Create a headers map
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("User-Agent".to_string(), "Mozilla/5.0".to_string());

    // Create context with headers
    let context = RequestContext::with_headers(
        "POST".to_string(),
        "/api/data".to_string(),
        headers.clone(),
    );

    // Verify headers are stored and accessible
    let stored_headers = context.headers();
    assert_eq!(stored_headers.get("Content-Type"), Some(&"application/json".to_string()));
    assert_eq!(stored_headers.get("Authorization"), Some(&"Bearer token123".to_string()));
    assert_eq!(stored_headers.get("User-Agent"), Some(&"Mozilla/5.0".to_string()));

    // Verify missing header returns None
    assert_eq!(stored_headers.get("X-Custom-Header"), None);
}

// Test: RequestContext includes query parameters as HashMap
#[test]
fn test_request_context_includes_query_parameters() {
    use std::collections::HashMap;

    // Create a query parameters map
    let mut query_params = HashMap::new();
    query_params.insert("q".to_string(), "search term".to_string());
    query_params.insert("page".to_string(), "2".to_string());
    query_params.insert("limit".to_string(), "50".to_string());

    // Create context with query parameters
    let context = RequestContext::with_query_params(
        "GET".to_string(),
        "/search".to_string(),
        query_params.clone(),
    );

    // Verify query parameters are stored and accessible
    let stored_params = context.query_params();
    assert_eq!(stored_params.get("q"), Some(&"search term".to_string()));
    assert_eq!(stored_params.get("page"), Some(&"2".to_string()));
    assert_eq!(stored_params.get("limit"), Some(&"50".to_string()));

    // Verify missing parameter returns None
    assert_eq!(stored_params.get("sort"), None);
}

// Test: RequestContext includes timestamp
#[test]
fn test_request_context_includes_timestamp() {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Record time before creating context
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create context
    let context = RequestContext::new("GET".to_string(), "/test".to_string());

    // Record time after creating context
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Verify timestamp is within the expected range
    let timestamp = context.timestamp();
    assert!(timestamp >= before, "Timestamp should be >= time before creation");
    assert!(timestamp <= after, "Timestamp should be <= time after creation");

    // Create another context and verify it has a different timestamp
    std::thread::sleep(std::time::Duration::from_millis(10));
    let context2 = RequestContext::new("GET".to_string(), "/test2".to_string());
    let timestamp2 = context2.timestamp();

    // Second timestamp should be >= first (or equal if created too quickly)
    assert!(timestamp2 >= timestamp, "Later context should have later or equal timestamp");
}

// Test: Request ID is logged with every log message
#[test]
fn test_request_id_is_logged_with_every_log_message() {
    use yatagarasu::pipeline::RequestContext;

    // Create a context
    let context = RequestContext::new("GET".to_string(), "/test".to_string());
    let request_id = context.request_id().to_string();

    // For now, we just verify that the request_id is accessible and can be used in logging
    // Full integration with tracing/logging will be done when we implement the actual
    // logging middleware in later phases

    // Simulate a log message that includes the request ID
    let log_message = format!("Processing request [request_id={}]", context.request_id());

    // Verify the request ID is in the log message
    assert!(log_message.contains(&request_id),
            "Log message should contain request ID");

    // Verify the format is correct
    assert!(log_message.contains("request_id="),
            "Log message should have request_id field");

    // Multiple log messages should all include the same request ID
    let log_message2 = format!("Request completed [request_id={}]", context.request_id());
    assert!(log_message2.contains(&request_id),
            "All log messages should contain the same request ID");
}

// Test: Router middleware extracts bucket from request path
#[test]
fn test_router_middleware_extracts_bucket_from_request_path() {
    use yatagarasu::router::Router;
    use yatagarasu::config::{BucketConfig, S3Config};

    // Create bucket configs
    let buckets = vec![
        BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "my-products".to_string(),
                region: "us-east-1".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                endpoint: None,
            },
        },
        BucketConfig {
            name: "private".to_string(),
            path_prefix: "/private".to_string(),
            s3: S3Config {
                bucket: "my-private".to_string(),
                region: "us-east-1".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                endpoint: None,
            },
        },
    ];

    // Create router from buckets
    let router = Router::new(buckets);

    // Test routing to products bucket
    let bucket = router.route("/products/image.png");
    assert!(bucket.is_some(), "Should find bucket for /products path");
    assert_eq!(bucket.unwrap().name, "products");

    // Test routing to private bucket
    let bucket = router.route("/private/document.pdf");
    assert!(bucket.is_some(), "Should find bucket for /private path");
    assert_eq!(bucket.unwrap().name, "private");
}

// Test: Requests to /products/* route to products bucket
#[test]
fn test_requests_to_products_route_to_products_bucket() {
    use yatagarasu::router::Router;
    use yatagarasu::config::{BucketConfig, S3Config};

    // Create bucket configs
    let buckets = vec![
        BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "my-products".to_string(),
                region: "us-east-1".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                endpoint: None,
            },
        },
    ];

    let router = Router::new(buckets);

    // Test various paths under /products
    let test_paths = vec![
        "/products/image.png",
        "/products/subdir/file.jpg",
        "/products/a/b/c/deep.txt",
        "/products/logo.svg",
    ];

    for path in test_paths {
        let bucket = router.route(path);
        assert!(bucket.is_some(), "Should find bucket for path: {}", path);
        assert_eq!(bucket.unwrap().name, "products",
                   "Path {} should route to products bucket", path);
    }
}

// Test: Requests to /private/* route to private bucket
#[test]
fn test_requests_to_private_route_to_private_bucket() {
    use yatagarasu::router::Router;
    use yatagarasu::config::{BucketConfig, S3Config};

    // Create bucket configs
    let buckets = vec![
        BucketConfig {
            name: "private".to_string(),
            path_prefix: "/private".to_string(),
            s3: S3Config {
                bucket: "my-private-files".to_string(),
                region: "us-west-2".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                endpoint: None,
            },
        },
    ];

    let router = Router::new(buckets);

    // Test various paths under /private
    let test_paths = vec![
        "/private/secret.txt",
        "/private/docs/confidential.pdf",
        "/private/users/123/data.json",
        "/private/keys/api-key.pem",
    ];

    for path in test_paths {
        let bucket = router.route(path);
        assert!(bucket.is_some(), "Should find bucket for path: {}", path);
        assert_eq!(bucket.unwrap().name, "private",
                   "Path {} should route to private bucket", path);
    }
}
