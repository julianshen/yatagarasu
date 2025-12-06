//! OpenFGA End-to-End Integration Tests
//!
//! These tests verify the complete flow:
//!   HTTP Request → Yatagarasu Proxy → OpenFGA Authorization → S3 Backend
//!
//! Tests use testcontainers to run both OpenFGA and MinIO/LocalStack in Docker,
//! making them self-contained and reproducible.
//!
//! Prerequisites:
//!   - Docker running
//!   - `cargo build --release` (proxy binary needed)
//!
//! Run with:
//!   cargo test --test integration_tests openfga_e2e -- --ignored --nocapture

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use testcontainers_modules::localstack::LocalStack;

use super::test_harness::ProxyTestHarness;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// JWT Claims structure for testing
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<String>,
}

/// Generate a JWT token for testing
fn generate_jwt(secret: &str, subject: &str, exp_offset_seconds: i64) -> String {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + exp_offset_seconds;

    let claims = Claims {
        sub: subject.to_string(),
        exp: expiration as u64,
        aud: None,
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &encoding_key).expect("Failed to generate JWT")
}

/// Create an OpenFGA container and return the URL
fn create_openfga_container(docker: &Cli) -> (testcontainers::Container<'_, GenericImage>, String) {
    let openfga_image = GenericImage::new("openfga/openfga", "latest")
        .with_exposed_port(8080)
        .with_wait_for(WaitFor::message_on_stdout("starting server"));

    let args: Vec<String> = vec!["run".to_string()];
    let runnable_image = RunnableImage::from((openfga_image, args));

    let container = docker.run(runnable_image);
    let port = container.get_host_port_ipv4(8080);
    let url = format!("http://127.0.0.1:{}", port);

    (container, url)
}

/// Wait for OpenFGA to be ready
async fn wait_for_openfga(openfga_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    for _ in 0..30 {
        if let Ok(response) = client.get(&format!("{}/healthz", openfga_url)).send().await {
            if response.status().is_success() {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Create a store in OpenFGA and return the store ID
async fn create_store(openfga_url: &str, store_name: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/stores", openfga_url);

    let response = client
        .post(&url)
        .json(&json!({"name": store_name}))
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    body.get("id")?.as_str().map(|s| s.to_string())
}

/// Write an authorization model to OpenFGA
async fn write_authorization_model(
    openfga_url: &str,
    store_id: &str,
    model: serde_json::Value,
) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/stores/{}/authorization-models", openfga_url, store_id);

    let response = client.post(&url).json(&model).send().await.ok()?;

    if !response.status().is_success() {
        eprintln!("Failed to write model: {:?}", response.text().await);
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    body.get("authorization_model_id")?
        .as_str()
        .map(|s| s.to_string())
}

/// Write a relationship tuple to OpenFGA
async fn write_tuple(
    openfga_url: &str,
    store_id: &str,
    user: &str,
    relation: &str,
    object: &str,
) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/stores/{}/write", openfga_url, store_id);

    let body = json!({
        "writes": {
            "tuple_keys": [{
                "user": user,
                "relation": relation,
                "object": object
            }]
        }
    });

    match client.post(&url).json(&body).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Standard authorization model for E2E testing
fn test_authorization_model() -> serde_json::Value {
    json!({
        "schema_version": "1.1",
        "type_definitions": [
            {
                "type": "user",
                "relations": {}
            },
            {
                "type": "bucket",
                "relations": {
                    "viewer": { "this": {} },
                    "editor": { "this": {} },
                    "owner": { "this": {} }
                },
                "metadata": {
                    "relations": {
                        "viewer": { "directly_related_user_types": [{"type": "user"}] },
                        "editor": { "directly_related_user_types": [{"type": "user"}] },
                        "owner": { "directly_related_user_types": [{"type": "user"}] }
                    }
                }
            },
            {
                "type": "file",
                "relations": {
                    "parent": { "this": {} },
                    "viewer": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "viewer"}}}
                            ]
                        }
                    },
                    "editor": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "editor"}}}
                            ]
                        }
                    },
                    "owner": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "owner"}}}
                            ]
                        }
                    }
                },
                "metadata": {
                    "relations": {
                        "parent": { "directly_related_user_types": [{"type": "bucket"}] },
                        "viewer": { "directly_related_user_types": [{"type": "user"}] },
                        "editor": { "directly_related_user_types": [{"type": "user"}] },
                        "owner": { "directly_related_user_types": [{"type": "user"}] }
                    }
                }
            }
        ]
    })
}

/// Setup LocalStack with test bucket and file
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

    // Upload test files
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key("public/readme.txt")
        .body(b"Public content".to_vec().into())
        .send()
        .await
        .expect("Failed to upload public file");

    s3_client
        .put_object()
        .bucket(bucket_name)
        .key("private/secret.txt")
        .body(b"Secret content - requires authorization".to_vec().into())
        .send()
        .await
        .expect("Failed to upload private file");

    s3_client
        .put_object()
        .bucket(bucket_name)
        .key("docs/report.pdf")
        .body(b"PDF content".to_vec().into())
        .send()
        .await
        .expect("Failed to upload docs file");

    (container, endpoint)
}

/// Create proxy config file for OpenFGA E2E tests
fn create_openfga_proxy_config(
    config_path: &str,
    s3_endpoint: &str,
    openfga_endpoint: &str,
    store_id: &str,
    model_id: &str,
    jwt_secret: &str,
    proxy_port: u16,
) {
    let config_content = format!(
        r#"server:
  address: "127.0.0.1"
  port: {proxy_port}

buckets:
  - name: "openfga-bucket"
    path_prefix: "/protected"
    s3:
      endpoint: "{s3_endpoint}"
      region: "us-east-1"
      bucket: "openfga-bucket"
      access_key: "test"
      secret_key: "test"
    jwt:
      enabled: true
      secret: "{jwt_secret}"
      algorithm: "HS256"
      token_sources:
        - type: bearer_header
      claims: []
    authorization:
      type: "openfga"
      openfga_endpoint: "{openfga_endpoint}"
      openfga_store_id: "{store_id}"
      openfga_authorization_model_id: "{model_id}"
      openfga_timeout_ms: 5000
      openfga_cache_ttl_seconds: 60
      openfga_fail_mode: "closed"
      openfga_user_claim: "sub"
"#
    );

    fs::write(config_path, config_content).expect("Failed to write config file");
    println!("Created OpenFGA proxy config at {}", config_path);
}

// ============================================================================
// E2E Tests
// ============================================================================

/// Test: User with viewer permission can access file via proxy
///
/// Flow: HTTP GET → Proxy → JWT validation → OpenFGA check (allowed) → S3 → 200 OK
#[test]
#[ignore] // Requires Docker and built binary
fn test_e2e_openfga_authorized_user_can_access_file() {
    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // 1. Start OpenFGA
        let (_openfga_container, openfga_url) = create_openfga_container(&docker);
        assert!(
            wait_for_openfga(&openfga_url).await,
            "OpenFGA should be ready"
        );
        println!("OpenFGA running at {}", openfga_url);

        // 2. Setup OpenFGA store and model
        let store_id = create_store(&openfga_url, "e2e-test-store")
            .await
            .expect("Should create store");
        println!("Created store: {}", store_id);

        let model_id =
            write_authorization_model(&openfga_url, &store_id, test_authorization_model())
                .await
                .expect("Should write model");
        println!("Created model: {}", model_id);

        // 3. Grant alice viewer access to the file
        // OpenFGA object format: file:<bucket>/<path>
        assert!(
            write_tuple(
                &openfga_url,
                &store_id,
                "user:alice",
                "viewer",
                "file:openfga-bucket/docs/report.pdf"
            )
            .await,
            "Should write tuple"
        );
        println!("Granted alice viewer access to docs/report.pdf");

        // 4. Start S3 (LocalStack)
        let (_s3_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "openfga-bucket").await;
        println!("S3 running at {}", s3_endpoint);

        // 5. Create proxy config
        let config_path = "/tmp/yatagarasu-openfga-e2e-test.yaml";
        let jwt_secret = "test-secret-for-openfga-e2e";
        let proxy_port = 18090;

        create_openfga_proxy_config(
            config_path,
            &s3_endpoint,
            &openfga_url,
            &store_id,
            &model_id,
            jwt_secret,
            proxy_port,
        );

        // 6. Start the proxy
        let proxy = ProxyTestHarness::start(config_path, proxy_port).expect("Proxy should start");
        println!("Proxy running at {}", proxy.base_url);

        // Wait for proxy to fully initialize
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 7. Make request with valid JWT for alice
        let client = reqwest::Client::new();
        let token = generate_jwt(jwt_secret, "alice", 3600);

        let response = client
            .get(proxy.url("/protected/docs/report.pdf"))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Request should succeed");

        println!("Response status: {}", response.status());

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Alice should be able to access the file (has viewer permission)"
        );

        let body = response.text().await.unwrap();
        assert!(
            body.contains("PDF content"),
            "Response should contain the file content"
        );

        println!("✅ E2E test passed: Authorized user can access file via OpenFGA");
    });
}

/// Test: User without permission is denied access (403)
///
/// Flow: HTTP GET → Proxy → JWT validation → OpenFGA check (denied) → 403 Forbidden
#[test]
#[ignore] // Requires Docker and built binary
fn test_e2e_openfga_unauthorized_user_gets_403() {
    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // 1. Start OpenFGA
        let (_openfga_container, openfga_url) = create_openfga_container(&docker);
        assert!(
            wait_for_openfga(&openfga_url).await,
            "OpenFGA should be ready"
        );

        // 2. Setup OpenFGA store and model (no tuples - bob has no access)
        let store_id = create_store(&openfga_url, "e2e-test-store-denied")
            .await
            .expect("Should create store");

        let model_id =
            write_authorization_model(&openfga_url, &store_id, test_authorization_model())
                .await
                .expect("Should write model");

        // 3. Start S3 (LocalStack)
        let (_s3_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "openfga-bucket").await;

        // 4. Create proxy config
        let config_path = "/tmp/yatagarasu-openfga-e2e-denied.yaml";
        let jwt_secret = "test-secret-for-openfga-e2e";
        let proxy_port = 18091;

        create_openfga_proxy_config(
            config_path,
            &s3_endpoint,
            &openfga_url,
            &store_id,
            &model_id,
            jwt_secret,
            proxy_port,
        );

        // 5. Start the proxy
        let proxy = ProxyTestHarness::start(config_path, proxy_port).expect("Proxy should start");

        tokio::time::sleep(Duration::from_secs(1)).await;

        // 6. Make request with valid JWT for bob (who has no permissions)
        let client = reqwest::Client::new();
        let token = generate_jwt(jwt_secret, "bob", 3600);

        let response = client
            .get(proxy.url("/protected/private/secret.txt"))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Request should complete");

        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "Bob should be denied access (no OpenFGA permission)"
        );

        println!("✅ E2E test passed: Unauthorized user gets 403 via OpenFGA");
    });
}

/// Test: Request without JWT is denied (401/403)
///
/// Flow: HTTP GET → Proxy → No JWT → 401/403
#[test]
#[ignore] // Requires Docker and built binary
fn test_e2e_openfga_missing_jwt_denied() {
    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // 1. Start OpenFGA
        let (_openfga_container, openfga_url) = create_openfga_container(&docker);
        assert!(
            wait_for_openfga(&openfga_url).await,
            "OpenFGA should be ready"
        );

        // 2. Setup OpenFGA
        let store_id = create_store(&openfga_url, "e2e-test-store-nojwt")
            .await
            .expect("Should create store");

        let model_id =
            write_authorization_model(&openfga_url, &store_id, test_authorization_model())
                .await
                .expect("Should write model");

        // 3. Start S3
        let (_s3_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "openfga-bucket").await;

        // 4. Create proxy config
        let config_path = "/tmp/yatagarasu-openfga-e2e-nojwt.yaml";
        let jwt_secret = "test-secret-for-openfga-e2e";
        let proxy_port = 18092;

        create_openfga_proxy_config(
            config_path,
            &s3_endpoint,
            &openfga_url,
            &store_id,
            &model_id,
            jwt_secret,
            proxy_port,
        );

        // 5. Start the proxy
        let proxy = ProxyTestHarness::start(config_path, proxy_port).expect("Proxy should start");

        tokio::time::sleep(Duration::from_secs(1)).await;

        // 6. Make request WITHOUT JWT
        let client = reqwest::Client::new();

        let response = client
            .get(proxy.url("/protected/docs/report.pdf"))
            .send()
            .await
            .expect("Request should complete");

        // Should be denied - JWT is required
        assert!(
            response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::FORBIDDEN,
            "Request without JWT should be denied, got {}",
            response.status()
        );

        println!("✅ E2E test passed: Missing JWT is denied");
    });
}

/// Test: Bucket-level permissions via parent relation
///
/// Flow: Grant viewer on bucket → User can access any file in bucket
#[test]
#[ignore] // Requires Docker and built binary
fn test_e2e_openfga_bucket_level_permission() {
    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // 1. Start OpenFGA
        let (_openfga_container, openfga_url) = create_openfga_container(&docker);
        assert!(
            wait_for_openfga(&openfga_url).await,
            "OpenFGA should be ready"
        );

        // 2. Setup OpenFGA store and model
        let store_id = create_store(&openfga_url, "e2e-test-bucket-perm")
            .await
            .expect("Should create store");

        let model_id =
            write_authorization_model(&openfga_url, &store_id, test_authorization_model())
                .await
                .expect("Should write model");

        // 3. Grant charlie viewer access to the BUCKET (not specific file)
        assert!(
            write_tuple(
                &openfga_url,
                &store_id,
                "user:charlie",
                "viewer",
                "bucket:openfga-bucket"
            )
            .await,
            "Should write bucket tuple"
        );

        // 4. Set file's parent to be the bucket
        assert!(
            write_tuple(
                &openfga_url,
                &store_id,
                "bucket:openfga-bucket",
                "parent",
                "file:openfga-bucket/private/secret.txt"
            )
            .await,
            "Should write parent relation"
        );

        // 5. Start S3
        let (_s3_container, s3_endpoint) =
            setup_localstack_with_bucket(&docker, "openfga-bucket").await;

        // 6. Create proxy config
        let config_path = "/tmp/yatagarasu-openfga-e2e-bucket.yaml";
        let jwt_secret = "test-secret-for-openfga-e2e";
        let proxy_port = 18093;

        create_openfga_proxy_config(
            config_path,
            &s3_endpoint,
            &openfga_url,
            &store_id,
            &model_id,
            jwt_secret,
            proxy_port,
        );

        // 7. Start the proxy
        let proxy = ProxyTestHarness::start(config_path, proxy_port).expect("Proxy should start");

        tokio::time::sleep(Duration::from_secs(1)).await;

        // 8. Make request with charlie's JWT
        let client = reqwest::Client::new();
        let token = generate_jwt(jwt_secret, "charlie", 3600);

        let response = client
            .get(proxy.url("/protected/private/secret.txt"))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Request should succeed");

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Charlie should access file via bucket-level permission"
        );

        println!("✅ E2E test passed: Bucket-level permission grants file access");
    });
}

// ============================================================================
// Test Module Registration
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Marker test to verify module compiles
    #[test]
    fn test_openfga_e2e_module_compiles() {
        // This test just verifies the module structure is correct
        assert!(true);
    }
}
