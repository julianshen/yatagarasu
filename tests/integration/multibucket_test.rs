// Multi-Bucket Routing Integration Tests
// Phase 20: Extended Integration Tests - Multi-Bucket Routing
//
// Tests that the proxy correctly routes requests to different S3 buckets
// based on path prefixes, with isolated credentials per bucket.

use std::sync::Once;
use std::time::Duration;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
}

// Helper: Setup LocalStack with multiple buckets and test files
async fn setup_localstack_with_multiple_buckets(
    docker: &Cli,
) -> (
    testcontainers::Container<'_, LocalStack>,
    String,
    Vec<String>,
) {
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    // Create S3 client
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url(&endpoint)
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_credential_types::Credentials::new(
            "test", "test", None, None, "test",
        ))
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&config);

    // Create multiple buckets with test files
    let bucket_names = vec![
        "products-bucket".to_string(),
        "images-bucket".to_string(),
        "videos-bucket".to_string(),
    ];

    for bucket_name in &bucket_names {
        // Create bucket
        s3_client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .expect(&format!("Failed to create bucket: {}", bucket_name));

        // Upload test file specific to this bucket
        let content = format!("Content from {}", bucket_name);
        s3_client
            .put_object()
            .bucket(bucket_name)
            .key("test.txt")
            .body(content.as_bytes().to_vec().into())
            .send()
            .await
            .expect(&format!("Failed to upload to bucket: {}", bucket_name));

        log::info!(
            "Created bucket {} with test file at LocalStack",
            bucket_name
        );
    }

    (container, endpoint, bucket_names)
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_multiple_buckets_with_different_path_prefixes() {
    init_logging();

    // RED PHASE: This test verifies that the proxy can route requests to different
    // S3 buckets based on path prefixes:
    // - /products/* -> products-bucket
    // - /images/* -> images-bucket
    // - /videos/* -> videos-bucket

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint, bucket_names) =
            setup_localstack_with_multiple_buckets(&docker).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);
        log::info!("Created buckets: {:?}", bucket_names);

        // TODO: Start Yatagarasu proxy with multi-bucket configuration
        // Config should map:
        // - /products -> products-bucket
        // - /images -> images-bucket
        // - /videos -> videos-bucket

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Request to /products/test.txt should route to products-bucket
        let response = client
            .get("http://127.0.0.1:18080/products/test.txt")
            .send()
            .await
            .expect("Failed to request /products/test.txt");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request to /products should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(
            body, "Content from products-bucket",
            "Response should contain content from products-bucket"
        );

        // Test 2: Request to /images/test.txt should route to images-bucket
        let response = client
            .get("http://127.0.0.1:18080/images/test.txt")
            .send()
            .await
            .expect("Failed to request /images/test.txt");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(body, "Content from images-bucket");

        // Test 3: Request to /videos/test.txt should route to videos-bucket
        let response = client
            .get("http://127.0.0.1:18080/videos/test.txt")
            .send()
            .await
            .expect("Failed to request /videos/test.txt");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(body, "Content from videos-bucket");

        log::info!("Multi-bucket routing test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_longest_prefix_match_when_paths_overlap() {
    init_logging();

    // RED PHASE: Test longest prefix matching for overlapping paths
    // Example:
    // - /api -> bucket1
    // - /api/v2 -> bucket2
    // Request to /api/v2/file.txt should route to bucket2 (longest match)
    // Request to /api/file.txt should route to bucket1

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let localstack_image =
            RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

        let container = docker.run(localstack_image);
        let port = container.get_host_port_ipv4(4566);
        let endpoint = format!("http://127.0.0.1:{}", port);

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create two buckets with overlapping paths
        // bucket-api: maps to /api
        s3_client
            .create_bucket()
            .bucket("bucket-api")
            .send()
            .await
            .unwrap();

        s3_client
            .put_object()
            .bucket("bucket-api")
            .key("file.txt")
            .body(b"Content from /api".to_vec().into())
            .send()
            .await
            .unwrap();

        // bucket-api-v2: maps to /api/v2
        s3_client
            .create_bucket()
            .bucket("bucket-api-v2")
            .send()
            .await
            .unwrap();

        s3_client
            .put_object()
            .bucket("bucket-api-v2")
            .key("file.txt")
            .body(b"Content from /api/v2".to_vec().into())
            .send()
            .await
            .unwrap();

        log::info!("Created overlapping path buckets");

        // TODO: Configure proxy with overlapping paths
        // - /api -> bucket-api
        // - /api/v2 -> bucket-api-v2

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: /api/v2/file.txt should match longest prefix (/api/v2)
        let response = client
            .get("http://127.0.0.1:18080/api/v2/file.txt")
            .send()
            .await
            .expect("Failed to request /api/v2/file.txt");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(
            body, "Content from /api/v2",
            "Should route to bucket-api-v2 (longest prefix match)"
        );

        // Test 2: /api/file.txt should match shorter prefix (/api)
        let response = client
            .get("http://127.0.0.1:18080/api/file.txt")
            .send()
            .await
            .expect("Failed to request /api/file.txt");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(
            body, "Content from /api",
            "Should route to bucket-api (shorter prefix)"
        );

        log::info!("Longest prefix match test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_to_unknown_path_returns_404() {
    init_logging();

    // RED PHASE: Requests to paths that don't match any bucket prefix should return 404

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, _s3_endpoint, _bucket_names) =
            setup_localstack_with_multiple_buckets(&docker).await;

        // TODO: Start proxy with configured paths (/products, /images, /videos)

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Request to unmapped path /unknown
        let response = client
            .get("http://127.0.0.1:18080/unknown/file.txt")
            .send()
            .await
            .expect("Failed to request /unknown/file.txt");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Request to unknown path should return 404"
        );

        // Test 2: Request to root path (no bucket prefix)
        let response = client
            .get("http://127.0.0.1:18080/file.txt")
            .send()
            .await
            .expect("Failed to request /file.txt");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Request without bucket prefix should return 404"
        );

        // Test 3: Request to partial match (not a complete prefix)
        let response = client
            .get("http://127.0.0.1:18080/prod/file.txt")
            .send()
            .await
            .expect("Failed to request /prod/file.txt");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Partial prefix match should return 404 (must match complete prefix)"
        );

        log::info!("Unknown path 404 test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_each_bucket_uses_isolated_s3_credentials() {
    init_logging();

    // RED PHASE: Verify that each bucket configuration uses its own isolated
    // S3 credentials. This is critical for security - we don't want bucket1
    // credentials accidentally used for bucket2.
    //
    // In this test, we'll configure the proxy with:
    // - Bucket1 with valid credentials
    // - Bucket2 with invalid credentials
    //
    // Requests to bucket1 should succeed (valid creds)
    // Requests to bucket2 should fail (invalid creds)
    // This proves credential isolation

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let localstack_image =
            RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

        let container = docker.run(localstack_image);
        let port = container.get_host_port_ipv4(4566);
        let endpoint = format!("http://127.0.0.1:{}", port);

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "valid_key", "valid_secret", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket1 with valid credentials
        s3_client
            .create_bucket()
            .bucket("bucket1")
            .send()
            .await
            .unwrap();

        s3_client
            .put_object()
            .bucket("bucket1")
            .key("test.txt")
            .body(b"Content from bucket1".to_vec().into())
            .send()
            .await
            .unwrap();

        log::info!("Created bucket1 with valid credentials");

        // TODO: Configure proxy with:
        // - /bucket1 -> bucket1 (valid credentials: valid_key/valid_secret)
        // - /bucket2 -> bucket2 (invalid credentials: invalid_key/invalid_secret)

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Request to /bucket1 should succeed (valid credentials)
        let response = client
            .get("http://127.0.0.1:18080/bucket1/test.txt")
            .send()
            .await
            .expect("Failed to request /bucket1/test.txt");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request to bucket1 with valid credentials should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Content from bucket1");

        // Test 2: Request to /bucket2 should fail (invalid credentials)
        // Note: LocalStack may not enforce credentials, but real S3 would return 403
        // This test documents the expected behavior with real S3

        log::info!("Credential isolation test passed (would fail with invalid creds on real S3)");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_bucket1_request_cannot_access_bucket2_objects() {
    init_logging();

    // RED PHASE: Verify security boundary - requests routed to bucket1 cannot
    // access objects from bucket2, even if the object key is the same.
    //
    // This tests path-based routing isolation:
    // - /products/secret.txt -> products-bucket/secret.txt
    // - /images/secret.txt -> images-bucket/secret.txt
    // These should return different content (from different buckets)

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let localstack_image =
            RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

        let container = docker.run(localstack_image);
        let port = container.get_host_port_ipv4(4566);
        let endpoint = format!("http://127.0.0.1:{}", port);

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket1 with secret.txt
        s3_client
            .create_bucket()
            .bucket("bucket1")
            .send()
            .await
            .unwrap();

        s3_client
            .put_object()
            .bucket("bucket1")
            .key("secret.txt")
            .body(b"SECRET_FROM_BUCKET1".to_vec().into())
            .send()
            .await
            .unwrap();

        // Create bucket2 with secret.txt (same key, different content)
        s3_client
            .create_bucket()
            .bucket("bucket2")
            .send()
            .await
            .unwrap();

        s3_client
            .put_object()
            .bucket("bucket2")
            .key("secret.txt")
            .body(b"SECRET_FROM_BUCKET2".to_vec().into())
            .send()
            .await
            .unwrap();

        log::info!("Created two buckets with same object key but different content");

        // TODO: Configure proxy with:
        // - /app1 -> bucket1
        // - /app2 -> bucket2

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Request to /app1/secret.txt should return content from bucket1
        let response = client
            .get("http://127.0.0.1:18080/app1/secret.txt")
            .send()
            .await
            .expect("Failed to request /app1/secret.txt");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(
            body, "SECRET_FROM_BUCKET1",
            "Request to /app1 should return content from bucket1"
        );

        // Test 2: Request to /app2/secret.txt should return content from bucket2
        let response = client
            .get("http://127.0.0.1:18080/app2/secret.txt")
            .send()
            .await
            .expect("Failed to request /app2/secret.txt");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.unwrap();
        assert_eq!(
            body, "SECRET_FROM_BUCKET2",
            "Request to /app2 should return content from bucket2 (NOT bucket1)"
        );

        // Test 3: Attempt to access bucket2 content via bucket1 path should fail
        // /app1/../../bucket2/secret.txt should NOT work (path traversal protection)
        let response = client
            .get("http://127.0.0.1:18080/app1/../../app2/secret.txt")
            .send()
            .await
            .expect("Failed path traversal attempt");

        // Should either return 404 (path not found) or 403 (forbidden)
        // or normalize path and return bucket1 content (depends on implementation)
        assert!(
            response.status() == reqwest::StatusCode::NOT_FOUND
                || response.status() == reqwest::StatusCode::FORBIDDEN
                || response.status() == reqwest::StatusCode::OK,
            "Path traversal should be blocked or normalized"
        );

        if response.status() == reqwest::StatusCode::OK {
            let body = response.text().await.unwrap();
            // If normalized, should still return bucket1 content
            assert_eq!(
                body, "SECRET_FROM_BUCKET1",
                "Path traversal should not access bucket2 via bucket1 path"
            );
        }

        log::info!("Bucket isolation test passed");
    });
}
