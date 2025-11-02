// JWT Authentication End-to-End Integration Tests
// Phase 20: Extended Integration Tests - JWT Authentication
//
// Tests that the proxy correctly validates JWT tokens for authentication,
// supporting multiple token sources (header, query param, custom header)
// and custom claims validation.

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Once;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;

use super::test_harness::ProxyTestHarness;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
}

// JWT Claims structure for testing
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // Subject (user ID)
    exp: u64,    // Expiration time (Unix timestamp)
    #[serde(skip_serializing_if = "Option::is_none")]
    bucket: Option<String>, // Custom claim: bucket access
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>, // Custom claim: user role
}

// Helper: Generate a valid JWT token
fn generate_jwt(secret: &str, exp_offset_seconds: i64, custom_claims: Option<Claims>) -> String {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + exp_offset_seconds;

    let claims = custom_claims.unwrap_or(Claims {
        sub: "user123".to_string(),
        exp: expiration as u64,
        bucket: None,
        role: None,
    });

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &encoding_key).expect("Failed to generate JWT")
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
    log::info!(
        "Created config file at {} for endpoint {}",
        config_path,
        s3_endpoint
    );
}

// Helper: Setup LocalStack with test bucket and file
async fn setup_localstack_with_bucket<'a>(
    docker: &'a Cli,
    bucket_name: &str,
) -> (testcontainers::Container<'a, LocalStack>, String) {
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

    // Create bucket
    s3_client
        .create_bucket()
        .bucket(bucket_name)
        .send()
        .await
        .expect(&format!("Failed to create bucket: {}", bucket_name));

    // Upload test file
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key("test.txt")
        .body(b"Secret content - JWT required".to_vec().into())
        .send()
        .await
        .expect(&format!("Failed to upload to bucket: {}", bucket_name));

    log::info!("Created bucket {} with test file", bucket_name);

    (container, endpoint)
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_with_valid_jwt_bearer_token_succeeds() {
    init_logging();

    // RED PHASE: Test that requests with valid JWT Bearer token succeed
    // Authorization: Bearer <jwt_token>

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-1.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for valid JWT bearer token test");

        let jwt_secret = "test-secret-key";
        let valid_token = generate_jwt(jwt_secret, 3600, None); // Valid for 1 hour

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request with valid JWT Bearer token
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", valid_token))
            .send()
            .await
            .expect("Failed to send request with JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request with valid JWT should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Secret content - JWT required");

        log::info!("Valid JWT authentication test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_without_jwt_to_private_bucket_returns_401() {
    init_logging();

    // RED PHASE: Requests to JWT-protected buckets without token return 401 Unauthorized

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-2.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for missing JWT test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request without JWT token
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .send()
            .await
            .expect("Failed to send request without JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "Request without JWT to private bucket should return 401"
        );

        // Response should include WWW-Authenticate header
        let auth_header = response.headers().get("www-authenticate");
        assert!(
            auth_header.is_some(),
            "401 response should include WWW-Authenticate header"
        );

        log::info!("Missing JWT authentication test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_with_invalid_jwt_returns_403() {
    init_logging();

    // RED PHASE: Requests with invalid JWT signature return 403 Forbidden
    // Invalid = wrong secret used to sign the token

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-3.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for invalid JWT test");

        // Generate JWT with WRONG secret
        let wrong_secret = "wrong-secret-key";
        let invalid_token = generate_jwt(wrong_secret, 3600, None);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request with invalid JWT
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", invalid_token))
            .send()
            .await
            .expect("Failed to send request with invalid JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "Request with invalid JWT should return 403 Forbidden"
        );

        log::info!("Invalid JWT test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_with_expired_jwt_returns_403() {
    init_logging();

    // RED PHASE: Requests with expired JWT return 403 Forbidden

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-4.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for expired JWT test");

        let jwt_secret = "test-secret-key";
        // Generate JWT that expired 1 hour ago
        let expired_token = generate_jwt(jwt_secret, -3600, None);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request with expired JWT
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", expired_token))
            .send()
            .await
            .expect("Failed to send request with expired JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "Request with expired JWT should return 403 Forbidden"
        );

        log::info!("Expired JWT test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_with_jwt_in_query_parameter_succeeds() {
    init_logging();

    // RED PHASE: Test JWT in query parameter (alternative to Authorization header)
    // Useful for scenarios where headers can't be set (e.g., HTML <img> tags, <video> sources)
    // URL: /private/test.txt?token=<jwt>

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-5.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for JWT in query parameter test");

        let jwt_secret = "test-secret-key";
        let valid_token = generate_jwt(jwt_secret, 3600, None);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request with JWT in query parameter
        let url = format!(
            "http://127.0.0.1:18080/private/test.txt?token={}",
            valid_token
        );
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to send request with JWT in query param");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request with JWT in query parameter should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Secret content - JWT required");

        log::info!("JWT in query parameter test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_request_with_jwt_in_custom_header_succeeds() {
    init_logging();

    // RED PHASE: Test JWT in custom header (X-API-Token, X-Auth-Token, etc.)
    // Useful for API clients that prefer custom headers over Authorization

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-6.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for JWT in custom header test");

        let jwt_secret = "test-secret-key";
        let valid_token = generate_jwt(jwt_secret, 3600, None);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request with JWT in custom header X-API-Token
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("X-API-Token", valid_token)
            .send()
            .await
            .expect("Failed to send request with JWT in custom header");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request with JWT in custom header should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Secret content - JWT required");

        log::info!("JWT in custom header test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_public_bucket_accessible_without_jwt() {
    init_logging();

    // RED PHASE: Test that public buckets (auth disabled) don't require JWT
    // Mixed configuration: /private requires JWT, /public does not

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

        // Create public bucket
        s3_client
            .create_bucket()
            .bucket("public-bucket")
            .send()
            .await
            .unwrap();

        s3_client
            .put_object()
            .bucket("public-bucket")
            .key("test.txt")
            .body(b"Public content - no JWT required".to_vec().into())
            .send()
            .await
            .unwrap();

        log::info!("Created public bucket");

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-7.yaml";
        create_localstack_config(&endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for public bucket access test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request to public bucket WITHOUT JWT
        let response = client
            .get("http://127.0.0.1:18080/public/test.txt")
            .send()
            .await
            .expect("Failed to send request to public bucket");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request to public bucket without JWT should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Public content - no JWT required");

        log::info!("Public bucket access without JWT test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_custom_claims_validation() {
    init_logging();

    // RED PHASE: Test custom JWT claims validation
    // Example: bucket=products claim required to access /products
    // JWT must contain: {"bucket": "products"} to access /products/*

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "products-bucket").await;

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-jwt-8.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for custom claims validation test");

        let jwt_secret = "test-secret-key";

        // Test 1: JWT with correct bucket claim should succeed
        let claims_with_bucket = Claims {
            sub: "user123".to_string(),
            exp: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600) as u64,
            bucket: Some("products".to_string()),
            role: None,
        };

        let valid_token = generate_jwt(jwt_secret, 3600, Some(claims_with_bucket));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let response = client
            .get("http://127.0.0.1:18080/products/test.txt")
            .header("Authorization", format!("Bearer {}", valid_token))
            .send()
            .await
            .expect("Failed to send request with correct claim");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request with correct bucket claim should succeed"
        );

        // Test 2: JWT with WRONG bucket claim should fail (403)
        let claims_wrong_bucket = Claims {
            sub: "user123".to_string(),
            exp: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600) as u64,
            bucket: Some("images".to_string()), // Wrong bucket
            role: None,
        };

        let wrong_token = generate_jwt(jwt_secret, 3600, Some(claims_wrong_bucket));

        let response = client
            .get("http://127.0.0.1:18080/products/test.txt")
            .header("Authorization", format!("Bearer {}", wrong_token))
            .send()
            .await
            .expect("Failed to send request with wrong claim");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "Request with wrong bucket claim should return 403 Forbidden"
        );

        // Test 3: JWT without bucket claim should fail (403)
        let claims_no_bucket = Claims {
            sub: "user123".to_string(),
            exp: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600) as u64,
            bucket: None, // Missing required claim
            role: None,
        };

        let no_claim_token = generate_jwt(jwt_secret, 3600, Some(claims_no_bucket));

        let response = client
            .get("http://127.0.0.1:18080/products/test.txt")
            .header("Authorization", format!("Bearer {}", no_claim_token))
            .send()
            .await
            .expect("Failed to send request without required claim");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "Request without required bucket claim should return 403 Forbidden"
        );

        log::info!("Custom claims validation test passed");
    });
}
