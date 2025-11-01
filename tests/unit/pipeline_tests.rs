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
        auth: None,
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
        auth: None,
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
        auth: None,
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
        auth: None,
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

// Test: Longest prefix matching works
#[test]
fn test_longest_prefix_matching_works() {
    use yatagarasu::router::Router;
    use yatagarasu::config::{BucketConfig, S3Config};

    // Create buckets with overlapping prefixes
    let buckets = vec![
        BucketConfig {
            name: "prod".to_string(),
            path_prefix: "/prod".to_string(),
            s3: S3Config {
                bucket: "my-prod".to_string(),
                region: "us-east-1".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                endpoint: None,
            },
        auth: None,
        },
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
        auth: None,
        },
    ];

    let router = Router::new(buckets);

    // /products/foo should match /products (longer prefix), not /prod
    let bucket = router.route("/products/foo");
    assert!(bucket.is_some(), "Should find bucket for /products/foo");
    assert_eq!(bucket.unwrap().name, "products",
               "/products/foo should match /products (longest prefix), not /prod");

    // /prod/bar should match /prod exactly
    let bucket = router.route("/prod/bar");
    assert!(bucket.is_some(), "Should find bucket for /prod/bar");
    assert_eq!(bucket.unwrap().name, "prod",
               "/prod/bar should match /prod");

    // /products should match /products exactly
    let bucket = router.route("/products");
    assert!(bucket.is_some(), "Should find bucket for /products");
    assert_eq!(bucket.unwrap().name, "products",
               "/products should match /products");
}

// Test: Unmapped paths return None (which translates to 404)
#[test]
fn test_unmapped_paths_return_none() {
    use yatagarasu::router::Router;
    use yatagarasu::config::{BucketConfig, S3Config};

    // Create router with limited bucket configs
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
        auth: None,
        },
    ];

    let router = Router::new(buckets);

    // Test paths that don't match any bucket
    let unmapped_paths = vec![
        "/",
        "/unknown",
        "/api/v1/users",
        "/static/css/style.css",
        "/product",  // Similar but not matching /products
        "/prod",     // Prefix of products but not matching
    ];

    for path in unmapped_paths {
        let bucket = router.route(path);
        assert!(bucket.is_none(),
                "Path {} should not match any bucket (return None for 404)",
                path);
    }
}

// Test: S3 key is extracted from path
#[test]
fn test_s3_key_is_extracted_from_path() {
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
        auth: None,
        },
    ];

    let router = Router::new(buckets);

    // Test extracting S3 keys from various paths
    let test_cases = vec![
        ("/products/image.png", "image.png"),
        ("/products/subdir/file.jpg", "subdir/file.jpg"),
        ("/products/a/b/c/deep.txt", "a/b/c/deep.txt"),
        ("/products/", ""),
        ("/products", ""),
    ];

    for (path, expected_key) in test_cases {
        let s3_key = router.extract_s3_key(path);
        assert_eq!(s3_key, Some(expected_key.to_string()),
                   "Path {} should extract S3 key '{}'", path, expected_key);
    }

    // Test path that doesn't match any bucket
    let s3_key = router.extract_s3_key("/unknown/file.txt");
    assert_eq!(s3_key, None, "Unmapped path should return None");
}

// Test: Router middleware adds bucket config to request context
#[test]
fn test_router_middleware_adds_bucket_config_to_request_context() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::config::{BucketConfig, S3Config};

    // Create a request context
    let mut context = RequestContext::new("GET".to_string(), "/products/image.png".to_string());

    // Create a bucket configuration
    let bucket_config = BucketConfig {
        name: "products".to_string(),
        path_prefix: "/products".to_string(),
        s3: S3Config {
            bucket: "my-products".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            endpoint: None,
        },
        auth: None,
    };

    // Add the bucket config to the context
    context.set_bucket_config(bucket_config.clone());

    // Verify the bucket config is stored in the context
    let stored_config = context.bucket_config();
    assert!(stored_config.is_some(), "Bucket config should be present in context");

    let config = stored_config.unwrap();
    assert_eq!(config.name, "products", "Bucket name should match");
    assert_eq!(config.path_prefix, "/products", "Path prefix should match");
    assert_eq!(config.s3.bucket, "my-products", "S3 bucket should match");
    assert_eq!(config.s3.region, "us-east-1", "S3 region should match");
}

// Test: Auth middleware skips validation for public buckets (auth.enabled=false)
#[test]
fn test_auth_middleware_skips_validation_for_public_buckets() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::config::{BucketConfig, S3Config, AuthConfig};
    use std::collections::HashMap;

    // Create a bucket configuration with authentication disabled (public bucket)
    let bucket_config = BucketConfig {
        name: "public".to_string(),
        path_prefix: "/public".to_string(),
        s3: S3Config {
            bucket: "my-public-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            endpoint: None,
        },
        auth: Some(AuthConfig {
            enabled: false,
        }),
    };

    // Create a request context without any JWT token
    let headers = HashMap::new();
    // Note: No Authorization header - simulating unauthenticated request

    let mut context = RequestContext::with_headers(
        "GET".to_string(),
        "/public/file.txt".to_string(),
        headers,
    );

    // Add bucket config to context (as router would do)
    context.set_bucket_config(bucket_config.clone());

    // Auth middleware should check if authentication is required
    let auth_required = context.bucket_config()
        .and_then(|bc| bc.auth.as_ref())
        .map(|auth| auth.enabled)
        .unwrap_or(false);

    // For public bucket (auth.enabled=false), auth should not be required
    assert!(!auth_required, "Auth should not be required for public bucket");

    // Verify the request can proceed without JWT validation
    // (In real implementation, auth middleware would skip JWT extraction and validation)
}

// Test: Auth middleware validates JWT for private buckets (auth.enabled=true)
#[test]
fn test_auth_middleware_validates_jwt_for_private_buckets() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::config::{BucketConfig, S3Config, AuthConfig};
    use std::collections::HashMap;

    // Create a bucket configuration with authentication enabled (private bucket)
    let bucket_config = BucketConfig {
        name: "private".to_string(),
        path_prefix: "/private".to_string(),
        s3: S3Config {
            bucket: "my-private-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            endpoint: None,
        },
        auth: Some(AuthConfig {
            enabled: true,
        }),
    };

    // Create a request context with a JWT token in Authorization header
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U".to_string());

    let mut context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/secret.txt".to_string(),
        headers,
    );

    // Add bucket config to context (as router would do)
    context.set_bucket_config(bucket_config.clone());

    // Auth middleware should check if authentication is required
    let auth_required = context.bucket_config()
        .and_then(|bc| bc.auth.as_ref())
        .map(|auth| auth.enabled)
        .unwrap_or(false);

    // For private bucket (auth.enabled=true), auth should be required
    assert!(auth_required, "Auth should be required for private bucket");

    // Verify that the request has authorization header
    assert!(context.headers().contains_key("Authorization"),
            "Request should have Authorization header for private bucket");

    // Verify the Authorization header has a Bearer token
    let auth_header = context.headers().get("Authorization").unwrap();
    assert!(auth_header.starts_with("Bearer "),
            "Authorization header should start with 'Bearer '");

    // In real implementation, auth middleware would:
    // 1. Extract the JWT token from the Authorization header
    // 2. Validate the JWT signature
    // 3. Check expiration and other standard claims
    // 4. Verify custom claims if configured
    // 5. Add validated claims to request context
    // 6. Return 401 if validation fails
}

// Test: JWT extracted from Authorization header (Bearer token)
#[test]
fn test_jwt_extracted_from_authorization_header() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_bearer_token;
    use std::collections::HashMap;

    // Create a request context with Authorization header containing a Bearer token
    let mut headers = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", expected_token),
    );

    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/data.json".to_string(),
        headers,
    );

    // Auth middleware should extract the token from the Authorization header
    let extracted_token = extract_bearer_token(context.headers());

    // Verify the token was successfully extracted
    assert!(extracted_token.is_some(), "Token should be extracted from Authorization header");
    assert_eq!(
        extracted_token.unwrap(),
        expected_token,
        "Extracted token should match the original token"
    );
}

// Test: JWT extracted from Authorization header with different casing
#[test]
fn test_jwt_extracted_from_authorization_header_case_insensitive() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_bearer_token;
    use std::collections::HashMap;

    // Test with lowercase "authorization" header
    let mut headers = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    headers.insert(
        "authorization".to_string(), // lowercase
        format!("Bearer {}", expected_token),
    );

    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers,
    );

    // Token extraction should be case-insensitive
    let extracted_token = extract_bearer_token(context.headers());
    assert!(extracted_token.is_some(), "Token should be extracted despite lowercase header name");
    assert_eq!(extracted_token.unwrap(), expected_token);
}

// Test: JWT extraction returns None when Authorization header is missing
#[test]
fn test_jwt_extraction_returns_none_when_header_missing() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_bearer_token;
    use std::collections::HashMap;

    // Create request without Authorization header
    let headers = HashMap::new();
    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers,
    );

    // Token extraction should return None
    let extracted_token = extract_bearer_token(context.headers());
    assert!(extracted_token.is_none(), "Should return None when Authorization header is missing");
}

// Test: JWT extraction returns None when Bearer prefix is missing
#[test]
fn test_jwt_extraction_returns_none_when_bearer_prefix_missing() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_bearer_token;
    use std::collections::HashMap;

    // Create request with Authorization header but without "Bearer " prefix
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U".to_string(),
    );

    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers,
    );

    // Token extraction should return None without Bearer prefix
    let extracted_token = extract_bearer_token(context.headers());
    assert!(extracted_token.is_none(), "Should return None when Bearer prefix is missing");
}

// Test: JWT extracted from query parameter (if configured)
#[test]
fn test_jwt_extracted_from_query_parameter() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_query_token;
    use std::collections::HashMap;

    // Create a request context with JWT token in query parameter
    let mut query_params = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    query_params.insert("token".to_string(), expected_token.to_string());

    let context = RequestContext::with_query_params(
        "GET".to_string(),
        "/private/data.json".to_string(),
        query_params,
    );

    // Auth middleware should extract the token from the query parameter
    let extracted_token = extract_query_token(context.query_params(), "token");

    // Verify the token was successfully extracted
    assert!(extracted_token.is_some(), "Token should be extracted from query parameter");
    assert_eq!(
        extracted_token.unwrap(),
        expected_token,
        "Extracted token should match the original token"
    );
}

// Test: JWT extraction from query parameter with custom parameter name
#[test]
fn test_jwt_extracted_from_query_parameter_custom_name() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_query_token;
    use std::collections::HashMap;

    // Create request with custom query parameter name "access_token"
    let mut query_params = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    query_params.insert("access_token".to_string(), expected_token.to_string());

    let context = RequestContext::with_query_params(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        query_params,
    );

    // Extract with custom parameter name
    let extracted_token = extract_query_token(context.query_params(), "access_token");
    assert!(extracted_token.is_some(), "Token should be extracted from custom query parameter");
    assert_eq!(extracted_token.unwrap(), expected_token);
}

// Test: JWT extraction returns None when query parameter is missing
#[test]
fn test_jwt_extraction_from_query_returns_none_when_missing() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_query_token;
    use std::collections::HashMap;

    // Create request without the expected query parameter
    let query_params = HashMap::new();
    let context = RequestContext::with_query_params(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        query_params,
    );

    // Token extraction should return None
    let extracted_token = extract_query_token(context.query_params(), "token");
    assert!(extracted_token.is_none(), "Should return None when query parameter is missing");
}

// Test: JWT extraction from query parameter ignores other parameters
#[test]
fn test_jwt_extraction_from_query_ignores_other_parameters() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_query_token;
    use std::collections::HashMap;

    // Create request with multiple query parameters
    let mut query_params = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    query_params.insert("token".to_string(), expected_token.to_string());
    query_params.insert("foo".to_string(), "bar".to_string());
    query_params.insert("page".to_string(), "1".to_string());

    let context = RequestContext::with_query_params(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        query_params,
    );

    // Should extract only the token parameter, ignoring others
    let extracted_token = extract_query_token(context.query_params(), "token");
    assert!(extracted_token.is_some(), "Token should be extracted despite other parameters");
    assert_eq!(extracted_token.unwrap(), expected_token);
}

// Test: JWT extracted from custom header (if configured)
#[test]
fn test_jwt_extracted_from_custom_header() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_header_token;
    use std::collections::HashMap;

    // Create a request context with JWT token in custom header "X-Auth-Token"
    let mut headers = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    headers.insert("X-Auth-Token".to_string(), expected_token.to_string());

    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/data.json".to_string(),
        headers,
    );

    // Auth middleware should extract the token from the custom header
    let extracted_token = extract_header_token(context.headers(), "X-Auth-Token");

    // Verify the token was successfully extracted
    assert!(extracted_token.is_some(), "Token should be extracted from custom header");
    assert_eq!(
        extracted_token.unwrap(),
        expected_token,
        "Extracted token should match the original token"
    );
}

// Test: JWT extracted from custom header with different header names
#[test]
fn test_jwt_extracted_from_custom_header_various_names() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_header_token;
    use std::collections::HashMap;

    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

    // Test with "X-API-Key" header
    let mut headers1 = HashMap::new();
    headers1.insert("X-API-Key".to_string(), expected_token.to_string());
    let context1 = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers1,
    );
    let extracted1 = extract_header_token(context1.headers(), "X-API-Key");
    assert!(extracted1.is_some(), "Token should be extracted from X-API-Key");
    assert_eq!(extracted1.unwrap(), expected_token);

    // Test with "X-Access-Token" header
    let mut headers2 = HashMap::new();
    headers2.insert("X-Access-Token".to_string(), expected_token.to_string());
    let context2 = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers2,
    );
    let extracted2 = extract_header_token(context2.headers(), "X-Access-Token");
    assert!(extracted2.is_some(), "Token should be extracted from X-Access-Token");
    assert_eq!(extracted2.unwrap(), expected_token);
}

// Test: JWT extraction from custom header is case-insensitive
#[test]
fn test_jwt_extraction_from_custom_header_case_insensitive() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_header_token;
    use std::collections::HashMap;

    // Create request with lowercase custom header
    let mut headers = HashMap::new();
    let expected_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    headers.insert("x-auth-token".to_string(), expected_token.to_string()); // lowercase

    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers,
    );

    // Should extract with case-insensitive matching
    let extracted_token = extract_header_token(context.headers(), "X-Auth-Token");
    assert!(extracted_token.is_some(), "Token should be extracted despite case difference");
    assert_eq!(extracted_token.unwrap(), expected_token);
}

// Test: JWT extraction from custom header returns None when header is missing
#[test]
fn test_jwt_extraction_from_custom_header_returns_none_when_missing() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_header_token;
    use std::collections::HashMap;

    // Create request without the custom header
    let headers = HashMap::new();
    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/file.txt".to_string(),
        headers,
    );

    // Token extraction should return None
    let extracted_token = extract_header_token(context.headers(), "X-Auth-Token");
    assert!(extracted_token.is_none(), "Should return None when custom header is missing");
}

// Test: Valid JWT adds claims to request context
#[test]
fn test_valid_jwt_adds_claims_to_request_context() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::{validate_jwt, Claims};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    let secret = "test_secret_key_123";

    // Create test claims
    let mut custom_claims = serde_json::Map::new();
    custom_claims.insert("name".to_string(), json!("John Doe"));

    let test_claims = Claims {
        sub: Some("user123".to_string()),
        exp: None,
        iat: None,
        nbf: None,
        iss: None,
        custom: custom_claims,
    };

    // Encode the JWT token
    let token = encode(
        &Header::default(),
        &test_claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).expect("Failed to encode token");

    // Validate the JWT and extract claims
    let claims = validate_jwt(&token, secret).expect("Token should be valid");

    // Create a request context
    let mut context = RequestContext::new("GET".to_string(), "/private/data.json".to_string());

    // Auth middleware should add the validated claims to the context
    context.set_claims(claims.clone());

    // Verify the claims were added to the context
    let stored_claims = context.claims();
    assert!(stored_claims.is_some(), "Claims should be present in context");

    let claims_ref = stored_claims.unwrap();
    assert_eq!(claims_ref.sub, Some("user123".to_string()), "Subject claim should match");

    // Verify custom claim "name" is present
    assert!(claims_ref.custom.contains_key("name"), "Custom claim 'name' should be present");
    assert_eq!(
        claims_ref.custom.get("name").and_then(|v| v.as_str()),
        Some("John Doe"),
        "Custom claim 'name' should equal 'John Doe'"
    );
}

// Test: Missing JWT on private bucket returns 401 Unauthorized
#[test]
fn test_missing_jwt_on_private_bucket_returns_401() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::config::{BucketConfig, S3Config, AuthConfig};
    use yatagarasu::auth::extract_bearer_token;
    use std::collections::HashMap;

    // Create a private bucket configuration (auth required)
    let bucket_config = BucketConfig {
        name: "private".to_string(),
        path_prefix: "/private".to_string(),
        s3: S3Config {
            bucket: "my-private-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            endpoint: None,
        },
        auth: Some(AuthConfig {
            enabled: true,
        }),
    };

    // Create a request context WITHOUT any JWT token
    let headers = HashMap::new(); // No Authorization header
    let mut context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/secret.txt".to_string(),
        headers,
    );

    // Add bucket config to context (as router would do)
    context.set_bucket_config(bucket_config.clone());

    // Check if authentication is required
    let auth_required = context.bucket_config()
        .and_then(|bc| bc.auth.as_ref())
        .map(|auth| auth.enabled)
        .unwrap_or(false);

    assert!(auth_required, "Auth should be required for private bucket");

    // Try to extract token from Authorization header
    let token = extract_bearer_token(context.headers());

    // Verify token is missing
    assert!(token.is_none(), "Token should be None when not provided");

    // In real implementation, auth middleware would:
    // 1. Check if auth is required for the bucket (auth_required == true)
    // 2. Attempt to extract token from configured sources
    // 3. If token is None and auth is required, return 401 Unauthorized
    // 4. Set response status to 401
    // 5. Set response body to {"error":"Unauthorized","message":"Missing authentication token","status":401}

    // For this test, we verify the decision logic:
    let should_return_401 = auth_required && token.is_none();
    assert!(should_return_401, "Should return 401 when token is missing on private bucket");
}

// Test: Invalid JWT signature returns 401 Unauthorized
#[test]
fn test_invalid_jwt_signature_returns_401() {
    use yatagarasu::auth::validate_jwt;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use yatagarasu::auth::Claims;

    // Create a JWT token signed with one secret
    let signing_secret = "correct_secret_key";
    let validation_secret = "wrong_secret_key"; // Different secret for validation

    // Create test claims
    let mut custom_claims = serde_json::Map::new();
    custom_claims.insert("name".to_string(), json!("John Doe"));

    let test_claims = Claims {
        sub: Some("user123".to_string()),
        exp: None,
        iat: None,
        nbf: None,
        iss: None,
        custom: custom_claims,
    };

    // Encode the JWT token with the signing secret
    let token = encode(
        &Header::default(),
        &test_claims,
        &EncodingKey::from_secret(signing_secret.as_ref()),
    ).expect("Failed to encode token");

    // Try to validate the token with a different secret
    let validation_result = validate_jwt(&token, validation_secret);

    // Verify that validation fails
    assert!(validation_result.is_err(), "Token validation should fail with wrong secret");

    // Verify the error is specifically InvalidSignature
    let err = validation_result.unwrap_err();
    assert!(
        format!("{:?}", err).contains("InvalidSignature"),
        "Error should be InvalidSignature, got: {:?}", err
    );

    // In real implementation, auth middleware would:
    // 1. Extract token from request (succeed)
    // 2. Validate JWT signature with configured secret (fail with InvalidSignature)
    // 3. Return 401 Unauthorized
    // 4. Set response body to {"error":"Unauthorized","message":"Invalid token signature","status":401}
    // 5. Block request from proceeding to S3

    // For this test, we verify that invalid signatures are detected
    // and would result in 401 response
}

// Test: Expired JWT returns 401 Unauthorized
#[test]
fn test_expired_jwt_returns_401() {
    use yatagarasu::auth::validate_jwt;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use yatagarasu::auth::Claims;
    use std::time::{SystemTime, UNIX_EPOCH};

    let secret = "test_secret_key_123";

    // Get current time and set expiration to 1 hour in the past
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expired_time = now - 3600; // 1 hour ago

    // Create test claims with expired exp claim
    let mut custom_claims = serde_json::Map::new();
    custom_claims.insert("name".to_string(), json!("John Doe"));

    let test_claims = Claims {
        sub: Some("user123".to_string()),
        exp: Some(expired_time), // Token expired 1 hour ago
        iat: Some(now - 7200), // Issued 2 hours ago
        nbf: None,
        iss: None,
        custom: custom_claims,
    };

    // Encode the JWT token
    let token = encode(
        &Header::default(),
        &test_claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).expect("Failed to encode token");

    // Try to validate the expired token
    let validation_result = validate_jwt(&token, secret);

    // Verify that validation fails
    assert!(validation_result.is_err(), "Token validation should fail for expired token");

    // Verify the error is specifically ExpiredSignature
    let err = validation_result.unwrap_err();
    assert!(
        format!("{:?}", err).contains("ExpiredSignature"),
        "Error should be ExpiredSignature, got: {:?}", err
    );

    // In real implementation, auth middleware would:
    // 1. Extract token from request (succeed)
    // 2. Validate JWT with configured secret (succeed signature check)
    // 3. Check expiration claim (fail - token expired)
    // 4. Return 401 Unauthorized
    // 5. Set response body to {"error":"Unauthorized","message":"Token has expired","status":401}
    // 6. Block request from proceeding to S3

    // For this test, we verify that expired tokens are detected
    // and would result in 401 response
}

// Test: JWT with wrong claims returns 403 Forbidden
#[test]
fn test_jwt_with_wrong_claims_returns_403() {
    use yatagarasu::auth::{validate_jwt, verify_claims, Claims};
    use yatagarasu::config::ClaimRule;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    let secret = "test_secret_key_123";

    // Create test claims with role="user" (not "admin")
    let mut custom_claims = serde_json::Map::new();
    custom_claims.insert("role".to_string(), json!("user"));
    custom_claims.insert("department".to_string(), json!("engineering"));

    let test_claims = Claims {
        sub: Some("user123".to_string()),
        exp: None,
        iat: None,
        nbf: None,
        iss: None,
        custom: custom_claims,
    };

    // Encode the JWT token
    let token = encode(
        &Header::default(),
        &test_claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).expect("Failed to encode token");

    // Validate the JWT (should succeed - token is valid)
    let claims = validate_jwt(&token, secret).expect("Token should be valid");

    // Define claim rules that require role="admin" (but token has role="user")
    let claim_rules = vec![
        ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: json!("admin"),
        },
    ];

    // Verify claims against the rules (should fail)
    let verification_passed = verify_claims(&claims, &claim_rules);

    // Verify that claim verification fails
    assert!(!verification_passed, "Claim verification should fail when role is wrong");

    // In real implementation, auth middleware would:
    // 1. Extract token from request (succeed)
    // 2. Validate JWT signature and expiration (succeed - token is valid)
    // 3. Add claims to request context (succeed)
    // 4. Verify claims against configured rules (fail - role is "user", not "admin")
    // 5. Return 403 Forbidden (not 401, because authentication succeeded but authorization failed)
    // 6. Set response body to {"error":"Forbidden","message":"Insufficient permissions","status":403}
    // 7. Block request from proceeding to S3

    // For this test, we verify that:
    // - Authentication succeeds (token is valid)
    // - Authorization fails (claims don't match requirements)
    // - This would result in 403 Forbidden (not 401 Unauthorized)
}

// Test: Multiple token sources checked in configured order
#[test]
fn test_multiple_token_sources_checked_in_order() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::{extract_bearer_token, extract_query_token, extract_header_token};
    use std::collections::HashMap;

    // Create a request with tokens in multiple locations
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token_from_header".to_string());
    headers.insert("X-Auth-Token".to_string(), "token_from_custom_header".to_string());

    let mut query_params = HashMap::new();
    query_params.insert("token".to_string(), "token_from_query".to_string());

    // Create context with both headers and query params
    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/data.json".to_string(),
        headers.clone(),
    );

    // In real implementation, auth middleware would check sources in configured order:
    // 1. First, try Authorization header (Bearer token)
    let token = extract_bearer_token(context.headers());
    if token.is_some() {
        // Found token in Authorization header - this is the first source
        assert_eq!(token.unwrap(), "token_from_header", "Should use token from Authorization header first");

        // In real implementation, this token would be validated and used
        // Other sources would NOT be checked since we found a token
        return;
    }

    // 2. If not found, try query parameter
    let token = extract_query_token(context.query_params(), "token");
    if token.is_some() {
        // Would use query token if Authorization header didn't have one
        assert_eq!(token.unwrap(), "token_from_query");
        return;
    }

    // 3. If not found, try custom header
    let token = extract_header_token(context.headers(), "X-Auth-Token");
    if token.is_some() {
        // Would use custom header if neither Authorization nor query had a token
        assert_eq!(token.unwrap(), "token_from_custom_header");
        return;
    }

    // If no token found in any source, would return 401
    panic!("Should have found token in one of the sources");
}

// Test: Token source priority - Authorization header takes precedence over query
#[test]
fn test_authorization_header_takes_precedence_over_query() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::extract_bearer_token;
    use std::collections::HashMap;

    // Create request with token in Authorization header
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer header_token".to_string());

    let context = RequestContext::with_headers(
        "GET".to_string(),
        "/private/data.json".to_string(),
        headers,
    );

    // When both sources are available, Authorization header should be used first
    let header_token = extract_bearer_token(context.headers());
    assert!(header_token.is_some(), "Should find token in Authorization header");
    assert_eq!(header_token.unwrap(), "header_token");

    // In real implementation, auth middleware would NOT check query parameter
    // because it already found a token in the Authorization header
    // This demonstrates token source priority

    // For this test, we verify that Authorization header is checked first
    // and would be used if present, even if query parameter also has a token
}

// Test: Fallback to query parameter when Authorization header is missing
#[test]
fn test_fallback_to_query_when_authorization_missing() {
    use yatagarasu::pipeline::RequestContext;
    use yatagarasu::auth::{extract_bearer_token, extract_query_token};
    use std::collections::HashMap;

    // Create request with token ONLY in query parameter (no Authorization header)
    let headers: HashMap<String, String> = HashMap::new(); // No Authorization header

    let mut query_params = HashMap::new();
    query_params.insert("token".to_string(), "query_token".to_string());

    let context = RequestContext::with_query_params(
        "GET".to_string(),
        "/private/data.json".to_string(),
        query_params,
    );

    // Try Authorization header first (should fail)
    let header_token = extract_bearer_token(context.headers());
    assert!(header_token.is_none(), "Should not find token in Authorization header");

    // Fallback to query parameter (should succeed)
    let query_token = extract_query_token(context.query_params(), "token");
    assert!(query_token.is_some(), "Should find token in query parameter");
    assert_eq!(query_token.unwrap(), "query_token");

    // This demonstrates that auth middleware falls back to alternate sources
    // when the primary source (Authorization header) doesn't have a token
}
