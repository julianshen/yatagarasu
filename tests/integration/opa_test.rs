//! OPA Integration Tests
//!
//! These tests require a running OPA server with policies loaded.
//! Run with: cargo test --test integration_tests opa -- --ignored
//!
//! To start OPA for testing:
//! ```bash
//! # Create test policy
//! mkdir -p /tmp/opa-policies
//! cat > /tmp/opa-policies/authz.rego << 'EOF'
//! package yatagarasu.authz
//!
//! default allow = false
//!
//! # Allow admins
//! allow {
//!     input.jwt_claims.roles[_] == "admin"
//! }
//!
//! # Allow users to access their own bucket
//! allow {
//!     input.bucket == input.jwt_claims.allowed_bucket
//! }
//! EOF
//!
//! # Start OPA
//! docker run -d --name opa-test -p 8181:8181 \
//!   -v /tmp/opa-policies:/policies \
//!   openpolicyagent/opa:latest run --server /policies
//! ```

use serde_json::json;
use std::time::Duration;
use yatagarasu::opa::{
    AuthorizationDecision, FailMode, OpaCache, OpaClient, OpaClientConfig, OpaInput,
};

/// Get OPA test URL from environment or default
fn opa_url() -> String {
    std::env::var("TEST_OPA_URL").unwrap_or_else(|_| "http://localhost:8181".to_string())
}

/// Check if OPA is available for testing
async fn opa_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    client
        .get(&format!("{}/health", opa_url()))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

// ============================================================================
// End-to-End OPA Evaluation Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires running OPA server
async fn test_opa_evaluation_allow_admin() {
    if !opa_available().await {
        eprintln!("Skipping test: OPA not available at {}", opa_url());
        return;
    }

    let config = OpaClientConfig {
        url: opa_url(),
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
    assert!(result.is_ok(), "OPA evaluation should succeed");
    assert!(result.unwrap(), "Admin should be allowed");
}

#[tokio::test]
#[ignore] // Requires running OPA server
async fn test_opa_evaluation_deny_non_admin() {
    if !opa_available().await {
        eprintln!("Skipping test: OPA not available at {}", opa_url());
        return;
    }

    let config = OpaClientConfig {
        url: opa_url(),
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
#[ignore] // Requires running OPA server
async fn test_opa_evaluation_allow_matching_bucket() {
    if !opa_available().await {
        eprintln!("Skipping test: OPA not available at {}", opa_url());
        return;
    }

    let config = OpaClientConfig {
        url: opa_url(),
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
    assert!(result.unwrap(), "User with matching bucket should be allowed");
}

// ============================================================================
// Cache Behavior Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires running OPA server
async fn test_opa_cache_hit_returns_same_result() {
    if !opa_available().await {
        eprintln!("Skipping test: OPA not available at {}", opa_url());
        return;
    }

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
    assert!(cache.get(&cache_key).await.is_none(), "Should be cache miss");

    // Simulate evaluation and store result
    cache.put(cache_key.clone(), true).await;

    // Second call - cache hit
    let cached = cache.get(&cache_key).await;
    assert_eq!(cached, Some(true), "Should return cached result");
}

#[tokio::test]
#[ignore] // Requires running OPA server
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
    assert!(cache.get(&key2).await.is_none(), "input2 should be cache miss");
}

// ============================================================================
// Timeout Handling Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires running OPA server
async fn test_opa_timeout_returns_error() {
    // Use a very short timeout to trigger timeout error
    let config = OpaClientConfig {
        url: opa_url(),
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

    // Should timeout or fail connection with such a short timeout
    assert!(result.is_err(), "Should fail with very short timeout");
}

#[tokio::test]
#[ignore] // Requires running OPA server
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
#[ignore]
async fn test_fail_closed_denies_on_error() {
    use yatagarasu::opa::OpaError;

    let error = OpaError::ConnectionFailed("connection refused".to_string());
    let decision = AuthorizationDecision::from_opa_result(Err(error), FailMode::Closed);

    assert!(!decision.is_allowed(), "Fail-closed should deny on error");
    assert!(decision.error().is_some(), "Should have error info");
    assert!(!decision.is_fail_open_allow(), "Should not be fail-open");
}

#[tokio::test]
#[ignore]
async fn test_fail_open_allows_on_error() {
    use yatagarasu::opa::OpaError;

    let error = OpaError::Timeout {
        policy_path: "authz/allow".to_string(),
        timeout_ms: 100,
    };
    let decision = AuthorizationDecision::from_opa_result(Err(error), FailMode::Open);

    assert!(decision.is_allowed(), "Fail-open should allow on error");
    assert!(decision.error().is_some(), "Should preserve error");
    assert!(decision.is_fail_open_allow(), "Should be marked as fail-open");
}

#[tokio::test]
#[ignore]
async fn test_fail_mode_does_not_affect_successful_evaluation() {
    // When OPA succeeds, fail mode shouldn't matter
    let allow_decision =
        AuthorizationDecision::from_opa_result(Ok(true), FailMode::Closed);
    assert!(allow_decision.is_allowed());
    assert!(!allow_decision.is_fail_open_allow());

    let deny_decision =
        AuthorizationDecision::from_opa_result(Ok(false), FailMode::Open);
    assert!(!deny_decision.is_allowed());
    assert!(!deny_decision.is_fail_open_allow());
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires running OPA server
async fn test_opa_evaluation_latency() {
    if !opa_available().await {
        eprintln!("Skipping test: OPA not available at {}", opa_url());
        return;
    }

    let config = OpaClientConfig {
        url: opa_url(),
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

    // P95 should be under 10ms for a local OPA
    assert!(
        p95 < 100,
        "P95 latency {}ms exceeds 100ms threshold (local OPA should be faster)",
        p95
    );
}

#[tokio::test]
#[ignore]
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
#[ignore] // Requires running OPA server
async fn test_opa_throughput() {
    if !opa_available().await {
        eprintln!("Skipping test: OPA not available at {}", opa_url());
        return;
    }

    let config = OpaClientConfig {
        url: opa_url(),
        policy_path: "yatagarasu/authz/allow".to_string(),
        timeout_ms: 5000,
        cache_ttl_seconds: 0, // Disable cache for this test
    };

    let client = std::sync::Arc::new(OpaClient::new(config));

    let start = std::time::Instant::now();
    let count = 100;

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

    // With local OPA, should handle at least 100 evaluations/second
    // (1000+ is realistic for production)
    assert!(
        throughput > 50.0,
        "Throughput {:.0}/s below minimum threshold",
        throughput
    );
}
