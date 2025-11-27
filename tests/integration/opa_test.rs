//! OPA Integration Tests using testcontainers
//!
//! These tests use testcontainers to run OPA in Docker, making them self-contained.
//! No external OPA server is required.

use serde_json::json;
use std::time::Duration;
use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use yatagarasu::opa::{
    AuthorizationDecision, FailMode, OpaCache, OpaClient, OpaClientConfig, OpaInput,
};

/// Create an OPA container and return the URL
fn create_opa_container(docker: &Cli) -> (testcontainers::Container<'_, GenericImage>, String) {
    // OPA image with server mode
    // Command: opa run --server --addr=0.0.0.0:8181
    // OPA logs to stderr in JSON format
    let opa_image = GenericImage::new("openpolicyagent/opa", "latest")
        .with_exposed_port(8181)
        .with_wait_for(WaitFor::message_on_stderr("Initializing server"));

    // Create RunnableImage with command arguments
    let args: Vec<String> = vec![
        "run".to_string(),
        "--server".to_string(),
        "--addr=0.0.0.0:8181".to_string(),
    ];
    let runnable_image = RunnableImage::from((opa_image, args));

    let container = docker.run(runnable_image);

    let port = container.get_host_port_ipv4(8181);
    let url = format!("http://127.0.0.1:{}", port);

    (container, url)
}

/// Upload a policy to OPA via REST API
async fn upload_policy(opa_url: &str, policy_name: &str, policy_content: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/policies/{}", opa_url, policy_name);

    match client
        .put(&url)
        .header("Content-Type", "text/plain")
        .body(policy_content.to_string())
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                true
            } else {
                eprintln!(
                    "Failed to upload policy: {} - {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                );
                false
            }
        }
        Err(e) => {
            eprintln!("Error uploading policy: {}", e);
            false
        }
    }
}

/// Wait for OPA to be ready
async fn wait_for_opa(opa_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    for _ in 0..30 {
        if let Ok(response) = client.get(&format!("{}/health", opa_url)).send().await {
            if response.status().is_success() {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Standard test policy for authorization
/// Note: Modern Rego syntax requires `if` keyword before rule bodies
const TEST_POLICY: &str = r#"
package yatagarasu.authz

default allow = false

# Allow admins to access everything
allow if {
    input.jwt_claims.roles[_] == "admin"
}

# Allow users to access their allowed bucket
allow if {
    input.bucket == input.jwt_claims.allowed_bucket
}

# Allow access to public paths
allow if {
    startswith(input.path, "/public/")
}
"#;

// ============================================================================
// End-to-End OPA Evaluation Tests
// ============================================================================

#[tokio::test]
async fn test_opa_evaluation_allow_admin() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    // Wait for OPA to be ready
    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");

    // Upload policy
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    // Admin should be allowed
    let input = OpaInput::new(
        json!({"sub": "admin-user", "roles": ["admin"]}),
        "any-bucket".to_string(),
        "/any/path/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let result = client.evaluate(&input).await;
    assert!(
        result.is_ok(),
        "OPA evaluation should succeed: {:?}",
        result
    );
    assert!(result.unwrap(), "Admin should be allowed");
}

#[tokio::test]
async fn test_opa_evaluation_deny_non_admin() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    // Non-admin without matching bucket should be denied
    let input = OpaInput::new(
        json!({"sub": "regular-user", "roles": ["user"]}),
        "restricted-bucket".to_string(),
        "/restricted/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let result = client.evaluate(&input).await;
    assert!(result.is_ok(), "OPA evaluation should succeed");
    assert!(!result.unwrap(), "Non-admin should be denied");
}

#[tokio::test]
async fn test_opa_evaluation_allow_matching_bucket() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    // User with allowed_bucket claim matching bucket should be allowed
    let input = OpaInput::new(
        json!({"sub": "user1", "roles": ["user"], "allowed_bucket": "my-bucket"}),
        "my-bucket".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let result = client.evaluate(&input).await;
    assert!(result.is_ok(), "OPA evaluation should succeed");
    assert!(
        result.unwrap(),
        "User with matching bucket should be allowed"
    );
}

#[tokio::test]
async fn test_opa_evaluation_allow_public_path() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    // Public paths should be allowed for anyone
    let input = OpaInput::new(
        json!({"sub": "anyone", "roles": []}),
        "any-bucket".to_string(),
        "/public/image.png".to_string(),
        "GET".to_string(),
        None,
    );

    let result = client.evaluate(&input).await;
    assert!(result.is_ok(), "OPA evaluation should succeed");
    assert!(result.unwrap(), "Public path should be allowed");
}

// ============================================================================
// Cache Behavior Tests
// ============================================================================

#[tokio::test]
async fn test_opa_cache_hit_returns_same_result() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let cache = OpaCache::new(60);

    let input = OpaInput::new(
        json!({"sub": "user1", "roles": ["admin"]}),
        "bucket1".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let cache_key = input.cache_key();

    // First call - cache miss
    assert!(
        cache.get(&cache_key).await.is_none(),
        "Should be cache miss"
    );

    // Evaluate with OPA
    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };
    let client = OpaClient::new(config);
    let result = client.evaluate(&input).await.unwrap();

    // Store result in cache
    cache.put(cache_key.clone(), result).await;

    // Second call - cache hit
    let cached = cache.get(&cache_key).await;
    assert_eq!(cached, Some(true), "Should return cached result");
}

#[tokio::test]
async fn test_opa_cache_different_inputs_different_keys() {
    let cache = OpaCache::new(60);

    let input1 = OpaInput::new(
        json!({"sub": "user1"}),
        "bucket1".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let input2 = OpaInput::new(
        json!({"sub": "user2"}), // Different user
        "bucket1".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let key1 = input1.cache_key();
    let key2 = input2.cache_key();

    assert_ne!(key1, key2, "Different inputs should have different keys");

    // Cache result for input1
    cache.put(key1.clone(), true).await;

    // input2 should still be a miss
    assert!(
        cache.get(&key2).await.is_none(),
        "input2 should be cache miss"
    );
}

// ============================================================================
// Timeout Handling Tests
// ============================================================================

#[tokio::test]
async fn test_opa_timeout_returns_error() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");

    // Use a very short timeout to trigger timeout error
    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 1, // 1ms timeout - should fail
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    let input = OpaInput::new(
        json!({"sub": "user1"}),
        "bucket".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let result = client.evaluate(&input).await;

    // Should timeout or fail with such a short timeout
    assert!(result.is_err(), "Should fail with very short timeout");
}

#[tokio::test]
async fn test_opa_connection_error_to_invalid_host() {
    let config = OpaClientConfig {
        url: "http://invalid-host-that-does-not-exist:8181".to_string(),
        policy_path: "authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    let input = OpaInput::new(
        json!({"sub": "user1"}),
        "bucket".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let result = client.evaluate(&input).await;
    assert!(result.is_err(), "Should fail to connect to invalid host");
}

// ============================================================================
// Fail Mode Tests
// ============================================================================

#[tokio::test]
async fn test_fail_closed_denies_on_error() {
    use yatagarasu::opa::OpaError;

    let error = OpaError::ConnectionFailed("connection refused".to_string());
    let decision = AuthorizationDecision::from_opa_result(Err(error), FailMode::Closed);

    assert!(!decision.is_allowed(), "Fail-closed should deny on error");
    assert!(decision.error().is_some(), "Should have error info");
    assert!(!decision.is_fail_open_allow(), "Should not be fail-open");
}

#[tokio::test]
async fn test_fail_open_allows_on_error() {
    use yatagarasu::opa::OpaError;

    let error = OpaError::Timeout {
        policy_path: "authz/allow".to_string(),
        timeout_ms: 100,
    };
    let decision = AuthorizationDecision::from_opa_result(Err(error), FailMode::Open);

    assert!(decision.is_allowed(), "Fail-open should allow on error");
    assert!(decision.error().is_some(), "Should preserve error");
    assert!(
        decision.is_fail_open_allow(),
        "Should be marked as fail-open"
    );
}

#[tokio::test]
async fn test_fail_mode_does_not_affect_successful_evaluation() {
    // When OPA succeeds, fail mode shouldn't matter
    let allow_decision = AuthorizationDecision::from_opa_result(Ok(true), FailMode::Closed);
    assert!(allow_decision.is_allowed());
    assert!(!allow_decision.is_fail_open_allow());

    let deny_decision = AuthorizationDecision::from_opa_result(Ok(false), FailMode::Open);
    assert!(!deny_decision.is_allowed());
    assert!(!deny_decision.is_fail_open_allow());
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_opa_evaluation_latency() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    let input = OpaInput::new(
        json!({"sub": "admin-user", "roles": ["admin"]}),
        "bucket".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    // Warm up
    let _ = client.evaluate(&input).await;

    // Measure multiple evaluations
    let mut latencies = Vec::new();
    for _ in 0..10 {
        let start = std::time::Instant::now();
        let _ = client.evaluate(&input).await;
        latencies.push(start.elapsed().as_millis());
    }

    latencies.sort();
    let p95 = latencies[9]; // 95th percentile of 10 samples

    println!("OPA evaluation latencies (ms): {:?}", latencies);
    println!("P95 latency: {}ms", p95);

    // P95 should be under 100ms for containerized OPA
    assert!(p95 < 200, "P95 latency {}ms exceeds 200ms threshold", p95);
}

#[tokio::test]
async fn test_opa_cache_hit_latency() {
    let cache = OpaCache::new(60);

    let input = OpaInput::new(
        json!({"sub": "user1", "roles": ["admin"]}),
        "bucket".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    let cache_key = input.cache_key();
    cache.put(cache_key.clone(), true).await;

    // Measure cache hit latency
    let mut latencies = Vec::new();
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let _ = cache.get(&cache_key).await;
        latencies.push(start.elapsed().as_micros());
    }

    latencies.sort();
    let p95 = latencies[95]; // 95th percentile

    println!("Cache hit latencies (us): {:?}", &latencies[90..100]);
    println!("P95 latency: {}us", p95);

    // Cache hit should be under 1ms (1000us)
    assert!(
        p95 < 1000,
        "P95 cache latency {}us exceeds 1ms threshold",
        p95
    );
}

#[tokio::test]
async fn test_opa_throughput() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url,
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 0, // Disable cache for this test
    };

    let client = std::sync::Arc::new(OpaClient::new(config));

    let start = std::time::Instant::now();
    let count = 50; // Reduced count for containerized tests

    let mut handles = Vec::new();
    for i in 0..count {
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            let input = OpaInput::new(
                json!({"sub": format!("user{}", i), "roles": ["admin"]}),
                "bucket".to_string(),
                "/file.txt".to_string(),
                "GET".to_string(),
                None,
            );
            client.evaluate(&input).await
        }));
    }

    let mut success = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success += 1;
        }
    }

    let elapsed = start.elapsed();
    let throughput = (count as f64) / elapsed.as_secs_f64();

    println!("Completed {} evaluations in {:?}", success, elapsed);
    println!("Throughput: {:.0} evaluations/second", throughput);

    assert!(
        success >= count * 9 / 10,
        "At least 90% of evaluations should succeed"
    );

    // With containerized OPA, should handle at least 10 evaluations/second
    assert!(
        throughput > 10.0,
        "Throughput {:.0}/s below minimum threshold",
        throughput
    );
}

// ============================================================================
// Policy Upload and Update Tests
// ============================================================================

#[tokio::test]
async fn test_opa_policy_can_be_updated() {
    let docker = Cli::default();
    let (_container, opa_url) = create_opa_container(&docker);

    assert!(wait_for_opa(&opa_url).await, "OPA should be ready");

    // Upload initial policy that denies everything
    let deny_policy = r#"
package yatagarasu.authz
default allow = false
"#;
    assert!(
        upload_policy(&opa_url, "authz", deny_policy).await,
        "Initial policy should upload"
    );

    let config = OpaClientConfig {
        url: opa_url.clone(),
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 60,
    };

    let client = OpaClient::new(config);

    let input = OpaInput::new(
        json!({"sub": "admin", "roles": ["admin"]}),
        "bucket".to_string(),
        "/file.txt".to_string(),
        "GET".to_string(),
        None,
    );

    // Should be denied with initial policy
    let result = client.evaluate(&input).await;
    assert!(result.is_ok());
    assert!(!result.unwrap(), "Should be denied with deny policy");

    // Update policy to allow admins
    assert!(
        upload_policy(&opa_url, "authz", TEST_POLICY).await,
        "Updated policy should upload"
    );

    // Should now be allowed
    let result = client.evaluate(&input).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "Should be allowed after policy update");
}
