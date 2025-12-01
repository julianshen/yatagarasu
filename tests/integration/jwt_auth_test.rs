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
    INIT.call_once(|| {});
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

// ============================================================================
// Phase 47.2: Integration Tests for Extended JWT Support (RS256, ES256, JWKS)
// ============================================================================

// Helper: Generate RS256 JWT token using private key
fn generate_rs256_jwt(private_key_path: &str, exp_offset_seconds: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + exp_offset_seconds;

    let claims = serde_json::json!({
        "sub": "rs256_user",
        "exp": expiration as u64,
        "iat": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u64,
    });

    let private_key_pem = fs::read(private_key_path).expect("Failed to read RSA private key");
    let encoding_key =
        EncodingKey::from_rsa_pem(&private_key_pem).expect("Failed to create RSA encoding key");

    let header = Header::new(Algorithm::RS256);
    encode(&header, &claims, &encoding_key).expect("Failed to generate RS256 JWT")
}

// Helper: Generate ES256 JWT token using private key
fn generate_es256_jwt(private_key_path: &str, exp_offset_seconds: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + exp_offset_seconds;

    let claims = serde_json::json!({
        "sub": "es256_user",
        "exp": expiration as u64,
        "iat": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u64,
    });

    let private_key_pem = fs::read(private_key_path).expect("Failed to read EC private key");
    let encoding_key =
        EncodingKey::from_ec_pem(&private_key_pem).expect("Failed to create EC encoding key");

    let header = Header::new(Algorithm::ES256);
    encode(&header, &claims, &encoding_key).expect("Failed to generate ES256 JWT")
}

// Helper: Create RS256 config file with public key path
fn create_rs256_config(s3_endpoint: &str, public_key_path: &str, config_path: &str) {
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
    jwt:
      enabled: true
      algorithm: "RS256"
      public_key_path: "{}"
      token_sources:
        - type: "bearer_header"
      claims: []
"#,
        s3_endpoint, public_key_path
    );

    fs::write(config_path, config_content).expect("Failed to write RS256 config file");
    log::info!(
        "Created RS256 config file at {} for endpoint {}",
        config_path,
        s3_endpoint
    );
}

// Helper: Create ES256 config file with public key path
fn create_es256_config(s3_endpoint: &str, public_key_path: &str, config_path: &str) {
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
    jwt:
      enabled: true
      algorithm: "ES256"
      public_key_path: "{}"
      token_sources:
        - type: "bearer_header"
      claims: []
"#,
        s3_endpoint, public_key_path
    );

    fs::write(config_path, config_content).expect("Failed to write ES256 config file");
    log::info!(
        "Created ES256 config file at {} for endpoint {}",
        config_path,
        s3_endpoint
    );
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_e2e_rs256_jwt_authentication() {
    init_logging();

    // Phase 47.2: End-to-end test with RS256 JWT
    // Test that RS256 algorithm works correctly in a full proxy flow

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Use absolute path for public key
        let public_key_path = std::env::current_dir()
            .unwrap()
            .join("tests/fixtures/rsa_public.pem")
            .to_string_lossy()
            .to_string();

        let config_path = "/tmp/yatagarasu-rs256-e2e.yaml";
        create_rs256_config(&s3_endpoint, &public_key_path, config_path);

        // Start proxy with RS256 config
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for RS256 E2E test");

        let private_key_path = "tests/fixtures/rsa_private.pem";
        let valid_token = generate_rs256_jwt(private_key_path, 3600);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Valid RS256 JWT should succeed
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", valid_token))
            .send()
            .await
            .expect("Failed to send request with RS256 JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request with valid RS256 JWT should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Secret content - JWT required");

        // Test 2: Expired RS256 JWT should fail
        let expired_token = generate_rs256_jwt(private_key_path, -3600);

        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", expired_token))
            .send()
            .await
            .expect("Failed to send request with expired RS256 JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "Request with expired RS256 JWT should return 403"
        );

        // Test 3: HS256 JWT should be rejected (algorithm mismatch)
        let hs256_token = generate_jwt("any-secret", 3600, None);

        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", hs256_token))
            .send()
            .await
            .expect("Failed to send request with HS256 JWT to RS256 endpoint");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "HS256 JWT should be rejected when RS256 is configured"
        );

        log::info!("RS256 E2E authentication test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_e2e_es256_jwt_authentication() {
    init_logging();

    // Phase 47.2: End-to-end test with ES256 JWT
    // Test that ES256 (ECDSA) algorithm works correctly in a full proxy flow

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Use absolute path for public key
        let public_key_path = std::env::current_dir()
            .unwrap()
            .join("tests/fixtures/ecdsa_public.pem")
            .to_string_lossy()
            .to_string();

        let config_path = "/tmp/yatagarasu-es256-e2e.yaml";
        create_es256_config(&s3_endpoint, &public_key_path, config_path);

        // Start proxy with ES256 config
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for ES256 E2E test");

        let private_key_path = "tests/fixtures/ecdsa_private.pem";
        let valid_token = generate_es256_jwt(private_key_path, 3600);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Valid ES256 JWT should succeed
        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", valid_token))
            .send()
            .await
            .expect("Failed to send request with ES256 JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request with valid ES256 JWT should succeed"
        );

        let body = response.text().await.unwrap();
        assert_eq!(body, "Secret content - JWT required");

        // Test 2: Expired ES256 JWT should fail
        let expired_token = generate_es256_jwt(private_key_path, -3600);

        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", expired_token))
            .send()
            .await
            .expect("Failed to send request with expired ES256 JWT");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "Request with expired ES256 JWT should return 403"
        );

        // Test 3: RS256 JWT should be rejected (algorithm mismatch)
        let rs256_token = generate_rs256_jwt("tests/fixtures/rsa_private.pem", 3600);

        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", rs256_token))
            .send()
            .await
            .expect("Failed to send request with RS256 JWT to ES256 endpoint");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "RS256 JWT should be rejected when ES256 is configured"
        );

        log::info!("ES256 E2E authentication test passed");
    });
}

// Note: JWKS E2E test requires a mock JWKS server
// For now, we test JWKS functionality via unit tests in auth_tests.rs
// A full E2E test would require spinning up a mock JWKS endpoint
#[test]
#[ignore] // Requires JWKS mock server - covered by unit tests
fn test_e2e_jwks_authentication() {
    init_logging();

    // Phase 47.2: End-to-end test with JWKS
    // This test is marked as ignored because it requires a running JWKS endpoint
    // JWKS functionality is tested via unit tests in tests/unit/auth_tests.rs

    log::info!("JWKS E2E test - see unit tests for JWKS validation coverage");

    // In a real implementation, this would:
    // 1. Start a mock JWKS server (e.g., using wiremock or mockito)
    // 2. Configure the proxy to use the mock JWKS URL
    // 3. Sign JWTs with the mock JWKS keys
    // 4. Verify requests succeed/fail appropriately
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_key_rotation_scenario() {
    init_logging();

    // Phase 47.2: Key rotation scenario
    // Test that both old and new keys work during a rotation window
    // This simulates a key rotation where both keys are temporarily valid

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "private-bucket").await;

        // Use RS256 for this test
        let public_key_path = std::env::current_dir()
            .unwrap()
            .join("tests/fixtures/rsa_public.pem")
            .to_string_lossy()
            .to_string();

        let config_path = "/tmp/yatagarasu-key-rotation.yaml";
        create_rs256_config(&s3_endpoint, &public_key_path, config_path);

        // Start proxy
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for key rotation test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Token signed with the current (active) key should work
        let current_key_token = generate_rs256_jwt("tests/fixtures/rsa_private.pem", 3600);

        let response = client
            .get("http://127.0.0.1:18080/private/test.txt")
            .header("Authorization", format!("Bearer {}", current_key_token))
            .send()
            .await
            .expect("Failed to send request with current key token");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Token signed with current key should succeed"
        );

        // In a real key rotation scenario with JWKS, both old and new keys would be
        // present in the JWKS endpoint. Here we verify the single-key case works.
        // Full multi-key rotation requires JWKS support (tested in unit tests).

        log::info!("Key rotation scenario test passed (single key case)");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_multi_algorithm_configuration() {
    init_logging();

    // Phase 47.2: Multi-algorithm configuration
    // Test that different buckets can use different JWT algorithms

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

        // Create two buckets for different algorithms
        for bucket_name in &["rs256-bucket", "es256-bucket"] {
            s3_client
                .create_bucket()
                .bucket(*bucket_name)
                .send()
                .await
                .expect(&format!("Failed to create bucket: {}", bucket_name));

            s3_client
                .put_object()
                .bucket(*bucket_name)
                .key("test.txt")
                .body(
                    format!("Content from {} bucket", bucket_name)
                        .into_bytes()
                        .into(),
                )
                .send()
                .await
                .expect(&format!("Failed to upload to bucket: {}", bucket_name));
        }

        // Create multi-algorithm config
        let rsa_public_key_path = std::env::current_dir()
            .unwrap()
            .join("tests/fixtures/rsa_public.pem")
            .to_string_lossy()
            .to_string();

        let ec_public_key_path = std::env::current_dir()
            .unwrap()
            .join("tests/fixtures/ecdsa_public.pem")
            .to_string_lossy()
            .to_string();

        let config_content = format!(
            r#"server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "rs256-bucket"
    path_prefix: "/rs256"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "rs256-bucket"
      access_key: "test"
      secret_key: "test"
    jwt:
      enabled: true
      algorithm: "RS256"
      public_key_path: "{}"
      token_sources:
        - type: "bearer_header"
      claims: []

  - name: "es256-bucket"
    path_prefix: "/es256"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "es256-bucket"
      access_key: "test"
      secret_key: "test"
    jwt:
      enabled: true
      algorithm: "ES256"
      public_key_path: "{}"
      token_sources:
        - type: "bearer_header"
      claims: []
"#,
            endpoint, rsa_public_key_path, endpoint, ec_public_key_path
        );

        let config_path = "/tmp/yatagarasu-multi-alg.yaml";
        fs::write(config_path, config_content).expect("Failed to write multi-alg config");

        // Start proxy with multi-algorithm config
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for multi-algorithm test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: RS256 bucket with RS256 token should succeed
        let rs256_token = generate_rs256_jwt("tests/fixtures/rsa_private.pem", 3600);

        let response = client
            .get("http://127.0.0.1:18080/rs256/test.txt")
            .header("Authorization", format!("Bearer {}", rs256_token))
            .send()
            .await
            .expect("Failed to send RS256 request to RS256 bucket");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "RS256 token to RS256 bucket should succeed"
        );

        // Test 2: ES256 bucket with ES256 token should succeed
        let es256_token = generate_es256_jwt("tests/fixtures/ecdsa_private.pem", 3600);

        let response = client
            .get("http://127.0.0.1:18080/es256/test.txt")
            .header("Authorization", format!("Bearer {}", es256_token))
            .send()
            .await
            .expect("Failed to send ES256 request to ES256 bucket");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "ES256 token to ES256 bucket should succeed"
        );

        // Test 3: RS256 token to ES256 bucket should fail
        let response = client
            .get("http://127.0.0.1:18080/es256/test.txt")
            .header("Authorization", format!("Bearer {}", rs256_token))
            .send()
            .await
            .expect("Failed to send RS256 token to ES256 bucket");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "RS256 token to ES256 bucket should be rejected"
        );

        // Test 4: ES256 token to RS256 bucket should fail
        let response = client
            .get("http://127.0.0.1:18080/rs256/test.txt")
            .header("Authorization", format!("Bearer {}", es256_token))
            .send()
            .await
            .expect("Failed to send ES256 token to RS256 bucket");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "ES256 token to RS256 bucket should be rejected"
        );

        log::info!("Multi-algorithm configuration test passed");
    });
}
