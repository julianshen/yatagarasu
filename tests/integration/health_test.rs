// Health and Readiness Endpoint Integration Tests
// Phase 22: Graceful Shutdown & Observability - Health Endpoints
//
// Tests that the proxy exposes health endpoints correctly:
// - /health endpoint returns 200 OK when proxy is running
// - /health response includes basic status (uptime, version)
// - /health bypasses authentication (always accessible)

use super::test_harness::ProxyTestHarness;
use serde_json::Value;
use std::fs;
use std::sync::Once;
use std::time::Duration;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
}

// Helper: Create config file for LocalStack endpoint
fn create_localstack_config(s3_endpoint: &str, config_path: &str) {
    let config_content = format!(
        r#"server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "test-bucket"
      access_key: "test"
      secret_key: "test"

jwt:
  enabled: false
  secret: "dummy-secret"
  algorithm: "HS256"
  token_sources: []
  claims: []
"#,
        s3_endpoint
    );
    fs::write(config_path, config_content).expect("Failed to write config file");
}

#[tokio::test]
#[ignore] // Requires running proxy with LocalStack
async fn test_health_endpoint_returns_200_ok() {
    init_logging();

    let s3_endpoint = std::env::var("TEST_S3_ENDPOINT").unwrap_or("http://localhost:9000".to_string());
    let config_path = "/tmp/test-health-config.yaml";
    create_localstack_config(&s3_endpoint, config_path);

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test: /health endpoint returns 200 OK
    let response = harness
        .get("/health")
        .await
        .expect("Failed to GET /health");

    assert_eq!(
        response.status(),
        200,
        "/health endpoint should return 200 OK"
    );

    // Response should be JSON
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Missing content-type header");
    assert!(
        content_type.to_str().unwrap().contains("application/json"),
        "Content-Type should be application/json"
    );

    // Parse response body
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let json: Value = serde_json::from_slice(&body).expect("Failed to parse JSON");

    // Check required fields
    assert_eq!(
        json["status"], "healthy",
        "Status should be 'healthy'"
    );

    // uptime_seconds should exist and be a positive number
    assert!(
        json["uptime_seconds"].is_u64(),
        "uptime_seconds should be a number"
    );
    let uptime = json["uptime_seconds"].as_u64().unwrap();
    assert!(uptime >= 0, "uptime_seconds should be >= 0");

    // version should exist and be a string
    assert!(
        json["version"].is_string(),
        "version should be a string"
    );
    let version = json["version"].as_str().unwrap();
    assert!(!version.is_empty(), "version should not be empty");

    harness.shutdown().await;
}

#[tokio::test]
#[ignore] // Requires running proxy with LocalStack
async fn test_health_endpoint_bypasses_authentication() {
    init_logging();

    let s3_endpoint = std::env::var("TEST_S3_ENDPOINT").unwrap_or("http://localhost:9000".to_string());

    // Create config with JWT enabled
    let config_content = format!(
        r#"server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "private-bucket"
    path_prefix: "/private"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "private-bucket"
      access_key: "test"
      secret_key: "test"
    jwt_required: true

jwt:
  enabled: true
  secret: "test-secret-key-12345"
  algorithm: "HS256"
  token_sources:
    - type: "bearer_header"
  claims: []
"#,
        s3_endpoint
    );
    let config_path = "/tmp/test-health-auth-config.yaml";
    fs::write(config_path, config_content).expect("Failed to write config file");

    let harness = ProxyTestHarness::new(config_path, Duration::from_secs(10))
        .await
        .expect("Failed to start proxy");

    // Test: /health endpoint accessible WITHOUT JWT token
    let response = harness
        .get("/health")
        .await
        .expect("Failed to GET /health");

    assert_eq!(
        response.status(),
        200,
        "/health should return 200 even when JWT is required for other endpoints"
    );

    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let json: Value = serde_json::from_slice(&body).expect("Failed to parse JSON");

    assert_eq!(json["status"], "healthy");

    // Verify that /private DOES require auth
    let private_response = harness
        .get("/private/test.txt")
        .await
        .expect("Failed to GET /private/test.txt");

    assert_eq!(
        private_response.status(),
        403,
        "/private should return 403 Forbidden without JWT token"
    );

    harness.shutdown().await;
}
