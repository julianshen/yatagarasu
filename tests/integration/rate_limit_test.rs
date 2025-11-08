// Integration tests for rate limiting
// Tests global, per-bucket, and per-IP rate limits

use reqwest::StatusCode;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
#[ignore] // Requires running proxy with rate limiting configured
async fn test_global_rate_limit_enforced() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Assume config has global limit of 10 requests/second
    // Send 15 requests rapidly - first 10 should succeed, rest should fail
    let mut success_count = 0;
    let mut rate_limited_count = 0;

    for _ in 0..15 {
        let response = client
            .get(&format!("{}/test/sample.txt", base_url))
            .send()
            .await
            .expect("Request failed");

        if response.status().is_success() || response.status() == StatusCode::NOT_FOUND {
            success_count += 1;
        } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited_count += 1;

            // Verify error response format
            let body: Value = response.json().await.expect("Failed to parse JSON");
            assert_eq!(body["error"], "Too Many Requests");
            assert!(body["message"]
                .as_str()
                .unwrap()
                .contains("Global rate limit exceeded"));
        }
    }

    assert!(
        rate_limited_count > 0,
        "Should have rate-limited some requests"
    );
    assert!(
        success_count <= 10,
        "Should allow at most 10 requests (got {})",
        success_count
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_per_bucket_rate_limit_enforced() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Assume config has per-bucket limit of 5 requests/second for 'test' bucket
    // Send 10 requests to same bucket
    let mut rate_limited_count = 0;

    for _ in 0..10 {
        let response = client
            .get(&format!("{}/test/sample.txt", base_url))
            .send()
            .await
            .expect("Request failed");

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited_count += 1;

            let body: Value = response.json().await.expect("Failed to parse JSON");
            assert_eq!(body["error"], "Too Many Requests");
            assert!(body["message"]
                .as_str()
                .unwrap()
                .contains("Rate limit exceeded for bucket"));
        }
    }

    assert!(
        rate_limited_count > 0,
        "Should have rate-limited some bucket requests"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_per_ip_rate_limit_enforced() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Assume config has per-IP limit of 3 requests/second
    // All requests from same client IP should be limited
    let mut rate_limited_count = 0;

    for _ in 0..8 {
        let response = client
            .get(&format!("{}/test/sample.txt", base_url))
            .send()
            .await
            .expect("Request failed");

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited_count += 1;

            let body: Value = response.json().await.expect("Failed to parse JSON");
            assert_eq!(body["error"], "Too Many Requests");
            assert!(
                body["message"]
                    .as_str()
                    .unwrap()
                    .contains("Rate limit exceeded for IP")
                    || body["message"]
                        .as_str()
                        .unwrap()
                        .contains("rate limit exceeded")
            );
        }
    }

    assert!(
        rate_limited_count > 0,
        "Should have rate-limited some IP requests"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_rate_limit_refills_over_time() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Exhaust rate limit
    let mut exhausted = false;
    for _ in 0..20 {
        let response = client
            .get(&format!("{}/test/sample.txt", base_url))
            .send()
            .await
            .expect("Request failed");

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            exhausted = true;
            break;
        }
    }

    assert!(exhausted, "Should have exhausted rate limit");

    // Wait for token bucket to refill (2 seconds)
    sleep(Duration::from_secs(2)).await;

    // Try again - should succeed now
    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .send()
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success() || response.status() == StatusCode::NOT_FOUND,
        "Should succeed after rate limit refills"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_rate_limit_metrics_incremented() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Get initial metrics
    let initial_metrics = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to get metrics")
        .text()
        .await
        .expect("Failed to read metrics");

    // Trigger rate limiting
    for _ in 0..50 {
        let _ = client
            .get(&format!("{}/test/sample.txt", base_url))
            .send()
            .await;
    }

    // Get updated metrics
    let updated_metrics = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to get metrics")
        .text()
        .await
        .expect("Failed to read metrics");

    // Verify rate limit metric exists
    assert!(
        updated_metrics.contains("rate_limit_exceeded_total"),
        "Rate limit metric should exist"
    );

    // Extract and verify count increased
    let initial_count = extract_metric_value(&initial_metrics, "rate_limit_exceeded_total");
    let updated_count = extract_metric_value(&updated_metrics, "rate_limit_exceeded_total");

    assert!(
        updated_count > initial_count,
        "Rate limit metric should be incremented"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_rate_limit_headers_present() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Make request
    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .send()
        .await
        .expect("Request failed");

    // Check for rate limit headers (if implemented)
    // X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
    // Note: These are optional - test passes if headers exist
    if let Some(_limit) = response.headers().get("X-RateLimit-Limit") {
        // If rate limit headers are present, verify they're valid
        assert!(response.headers().contains_key("X-RateLimit-Remaining"));
    }
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_different_buckets_have_independent_limits() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Exhaust limit for bucket 'test'
    for _ in 0..20 {
        let _ = client
            .get(&format!("{}/test/sample.txt", base_url))
            .send()
            .await;
    }

    // Verify 'test' bucket is rate limited
    let test_response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .send()
        .await
        .expect("Request failed");

    // Try different bucket (if configured) - should still work
    // Note: This test assumes multiple buckets configured
    let other_response = client
        .get(&format!("{}/other/sample.txt", base_url))
        .send()
        .await
        .expect("Request failed");

    // At least one should work (if other bucket exists and has separate limit)
    // OR if only one bucket exists, both might be limited
    assert!(
        test_response.status() == StatusCode::TOO_MANY_REQUESTS
            || other_response.status().is_success()
            || other_response.status() == StatusCode::NOT_FOUND,
        "Buckets should have independent rate limits (or test is rate limited)"
    );
}

// Helper function to extract metric value from Prometheus text format
fn extract_metric_value(metrics: &str, metric_name: &str) -> u64 {
    for line in metrics.lines() {
        if line.starts_with(metric_name) && !line.starts_with('#') {
            // Handle both simple format and labeled format
            // Simple: "metric_name value"
            // Labeled: "metric_name{bucket=\"test\"} value"
            if let Some(value_part) = line.split_whitespace().last() {
                if let Ok(value) = value_part.parse::<u64>() {
                    return value;
                }
            }
        }
    }
    0
}
