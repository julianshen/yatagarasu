// Integration tests for structured logging
//
// Tests verify that:
// - X-Request-ID header is returned in responses
// - Logging doesn't crash the proxy
// - Request correlation works across requests
//
// Note: Direct log output verification is done through unit tests and manual inspection.
// Integration tests focus on observable behavior (headers, response codes).

use crate::integration::test_harness::ProxyTestHarness;
use hyper::StatusCode;
use std::collections::HashSet;
use std::fs;
use std::time::Duration;

fn init_logging() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn create_test_config(config_path: &str) {
    let config_content = r#"server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      endpoint: "http://localhost:9000"
      region: "us-east-1"
      bucket: "test-bucket"
      access_key: "test"
      secret_key: "test"

jwt:
  enabled: false
  secret: "test-secret"
  algorithm: "HS256"
  token_sources: []
  claims: []
"#;
    fs::write(config_path, config_content).expect("Failed to write config file");
}

fn create_jwt_config(config_path: &str) {
    let config_content = r#"server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "private-bucket"
    path_prefix: "/private"
    s3:
      endpoint: "http://localhost:9000"
      region: "us-east-1"
      bucket: "private-bucket"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: true

jwt:
  enabled: true
  secret: "test-secret-key-for-jwt-validation"
  algorithm: "HS256"
  token_sources:
    - type: "bearer"
  claims: []
"#;
    fs::write(config_path, config_content).expect("Failed to write config file");
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_x_request_id_header_returned_in_responses() {
    init_logging();

    let config_path = "/tmp/test-logging-request-id-config.yaml";
    create_test_config(config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test: Every response should include X-Request-ID header
    let response = harness
        .get("/test/file.txt")
        .await
        .expect("Failed to GET /test/file.txt");

    // Check X-Request-ID header is present
    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("X-Request-ID header should be present");

    // Verify it's a valid UUID v4 format (8-4-4-4-12 hex digits)
    let request_id_str = request_id.to_str().expect("X-Request-ID should be valid UTF-8");
    let uuid_pattern = regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
        .expect("Failed to compile UUID regex");

    assert!(
        uuid_pattern.is_match(request_id_str),
        "X-Request-ID should be a valid UUID v4: {}",
        request_id_str
    );

    harness.shutdown().await;
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_each_request_gets_unique_request_id() {
    init_logging();

    let config_path = "/tmp/test-logging-unique-id-config.yaml";
    create_test_config(config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Make multiple requests and collect request IDs
    let mut request_ids = HashSet::new();

    for _ in 0..5 {
        let response = harness
            .get("/test/file.txt")
            .await
            .expect("Failed to GET /test/file.txt");

        let request_id = response
            .headers()
            .get("x-request-id")
            .expect("X-Request-ID header should be present")
            .to_str()
            .expect("X-Request-ID should be valid UTF-8")
            .to_string();

        request_ids.insert(request_id);
    }

    // All request IDs should be unique
    assert_eq!(
        request_ids.len(),
        5,
        "Each request should get a unique request_id"
    );

    harness.shutdown().await;
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_x_request_id_present_on_error_responses() {
    init_logging();

    let config_path = "/tmp/test-logging-error-id-config.yaml";
    create_test_config(config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test 404 error includes X-Request-ID
    let response = harness
        .get("/unknown-path")
        .await
        .expect("Failed to GET /unknown-path");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("X-Request-ID header should be present even on errors");

    assert!(!request_id.is_empty(), "X-Request-ID should not be empty");

    harness.shutdown().await;
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_x_request_id_present_on_auth_errors() {
    init_logging();

    let config_path = "/tmp/test-logging-auth-error-id-config.yaml";
    create_jwt_config(config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test 401 auth error includes X-Request-ID
    let response = harness
        .get("/private/file.txt")
        .await
        .expect("Failed to GET /private/file.txt");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("X-Request-ID header should be present even on auth errors");

    assert!(!request_id.is_empty(), "X-Request-ID should not be empty");

    harness.shutdown().await;
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_health_endpoint_includes_x_request_id() {
    init_logging();

    let config_path = "/tmp/test-logging-health-id-config.yaml";
    create_test_config(config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test /health endpoint includes X-Request-ID
    let response = harness
        .get("/health")
        .await
        .expect("Failed to GET /health");

    assert_eq!(response.status(), StatusCode::OK);

    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("X-Request-ID header should be present on /health");

    assert!(!request_id.is_empty(), "X-Request-ID should not be empty");

    harness.shutdown().await;
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_metrics_endpoint_includes_x_request_id() {
    init_logging();

    let config_path = "/tmp/test-logging-metrics-id-config.yaml";
    create_test_config(config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test /metrics endpoint includes X-Request-ID
    let response = harness
        .get("/metrics")
        .await
        .expect("Failed to GET /metrics");

    assert_eq!(response.status(), StatusCode::OK);

    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("X-Request-ID header should be present on /metrics");

    assert!(!request_id.is_empty(), "X-Request-ID should not be empty");

    harness.shutdown().await;
}

// Note: The following structured logging features are verified through unit tests
// and manual inspection, as capturing log output in integration tests is complex:
//
// - request_id included in all log statements (verified in unit tests)
// - Client IP logging with X-Forwarded-For support (verified in unit tests)
// - S3 error codes and messages logged (verified in unit tests)
// - No sensitive data in logs (verified through code audit + docs/SECURITY_LOGGING.md)
// - Request duration logged on completion (verified in unit tests)
// - Log fields include timestamp, level, message, request_id, bucket, path, status (verified in code review)
//
// To manually verify structured logging:
// 1. Run the proxy with RUST_LOG=debug
// 2. Make requests and observe log output
// 3. Check that logs include request_id, client_ip, bucket, path, status_code, duration_ms
// 4. Check that JWT tokens are not logged (only token length)
// 5. Check that S3 credentials are not logged
// 6. Check that S3 errors include s3_error_code and s3_error_message when available
