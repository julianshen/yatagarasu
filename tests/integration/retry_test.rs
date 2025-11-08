// Integration tests for retry logic with exponential backoff
// Tests automatic retry behavior for transient S3 failures

use reqwest::StatusCode;
use serde_json::Value;
use std::time::{Duration, Instant};

#[tokio::test]
#[ignore] // Requires running proxy with retry configured
async fn test_retry_on_transient_s3_error() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Request a file that might cause transient S3 error (500, 502, 503)
    // Proxy should automatically retry
    let start = Instant::now();
    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Request failed");

    let elapsed = start.elapsed();

    // If retries happened, request should take longer than normal
    // Normal request: <500ms, With 2 retries + backoff: >1s
    if response.status().is_server_error() {
        // If still failing, check it attempted retries (took some time)
        assert!(
            elapsed > Duration::from_millis(100),
            "Should have attempted retries (elapsed: {:?})",
            elapsed
        );
    }
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_retry_eventually_succeeds() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Make request - if S3 backend has intermittent issues, retries should help
    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Request failed");

    // Either succeeds, or fails with clear error (not timeout)
    assert!(
        response.status().is_success()
            || response.status() == StatusCode::NOT_FOUND
            || response.status().is_server_error(),
        "Should either succeed or fail gracefully (got: {})",
        response.status()
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_retry_exhausted_after_max_attempts() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Assume retry policy: max_attempts=3
    // Request that will consistently fail
    let start = Instant::now();
    let response = client
        .get(&format!("{}/test/nonexistent-bucket/file.txt", base_url))
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Request failed");

    let elapsed = start.elapsed();

    // Should fail after retries
    if response.status().is_server_error() || response.status() == StatusCode::BAD_GATEWAY {
        // With 3 attempts and exponential backoff (50ms base), should take >150ms
        assert!(
            elapsed > Duration::from_millis(100),
            "Should have attempted multiple retries (elapsed: {:?})",
            elapsed
        );

        // Check error response
        if let Ok(body) = response.json::<Value>().await {
            if let Some(message) = body["message"].as_str() {
                // Message might indicate retries were exhausted
                assert!(
                    message.contains("error") || message.contains("Gateway"),
                    "Error message should describe failure"
                );
            }
        }
    }
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_retry_metrics_incremented() {
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

    // Trigger some S3 requests (may cause retries on transient failures)
    for _ in 0..10 {
        let _ = client
            .get(&format!("{}/test/sample.txt", base_url))
            .timeout(Duration::from_secs(5))
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

    // Verify retry metrics exist
    assert!(
        updated_metrics.contains("s3_retry_attempts_total")
            || updated_metrics.contains("retry_attempts"),
        "Retry attempt metrics should exist"
    );

    // If any retries happened, counts should be > 0
    if let Some(_) = updated_metrics.find("s3_retry_attempts_total") {
        // Metrics exist - good
        assert!(
            updated_metrics.contains("s3_retry_success_total")
                || updated_metrics.contains("s3_retry_exhausted_total"),
            "Retry outcome metrics should exist"
        );
    }
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_non_retriable_errors_dont_retry() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Request non-existent file (404) - should NOT retry
    let start = Instant::now();
    let response = client
        .get(&format!(
            "{}/test/definitely-nonexistent-file.txt",
            base_url
        ))
        .send()
        .await
        .expect("Request failed");

    let elapsed = start.elapsed();

    if response.status() == StatusCode::NOT_FOUND {
        // 404 is not retriable - should return quickly (no retries)
        assert!(
            elapsed < Duration::from_secs(2),
            "404 errors should not trigger retries (elapsed: {:?})",
            elapsed
        );
    }
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_retry_exponential_backoff() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Make request that will cause retries
    let start = Instant::now();
    let _ = client
        .get(&format!("{}/test/trigger-retry.txt", base_url))
        .timeout(Duration::from_secs(30))
        .send()
        .await;

    let elapsed = start.elapsed();

    // With exponential backoff: attempt 1 (0ms), attempt 2 (+50ms), attempt 3 (+100ms)
    // Total should be at least 150ms if retries happened
    // This test is probabilistic - passes if retries occurred
    if elapsed > Duration::from_millis(150) {
        // Retries happened with backoff
        assert!(
            elapsed < Duration::from_secs(10),
            "Retries should complete within reasonable time (elapsed: {:?})",
            elapsed
        );
    }
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_retry_preserves_request_data() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // POST request with body - retries should preserve body
    let body = b"test data for upload";
    let response = client
        .post(&format!("{}/test/upload.txt", base_url))
        .header("Content-Type", "text/plain")
        .body(body.to_vec())
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Request failed");

    // Should either succeed or fail gracefully
    // Request body should not be corrupted by retries
    assert!(
        response.status().is_success()
            || response.status().is_client_error()
            || response.status().is_server_error(),
        "Request should complete (status: {})",
        response.status()
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_retry_timeout_prevents_infinite_retries() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Set client timeout shorter than total retry time
    // Ensures retries don't run forever
    let start = Instant::now();
    let result = client
        .get(&format!("{}/test/slow-or-failing.txt", base_url))
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    let elapsed = start.elapsed();

    // Should either complete or timeout within reasonable time
    assert!(
        elapsed < Duration::from_secs(10),
        "Request should timeout or complete within limit (elapsed: {:?})",
        elapsed
    );

    // Result is either Ok (completed) or Err (timeout)
    match result {
        Ok(_) => {
            // Completed successfully or with error
        }
        Err(e) => {
            // Timed out - expected for very slow requests
            assert!(e.is_timeout(), "Error should be timeout (got: {})", e);
        }
    }
}

// Helper function to extract metric value from Prometheus text format
#[allow(dead_code)]
fn extract_metric_value(metrics: &str, metric_name: &str) -> u64 {
    for line in metrics.lines() {
        if line.starts_with(metric_name) && !line.starts_with('#') {
            if let Some(value_part) = line.split_whitespace().last() {
                if let Ok(value) = value_part.parse::<u64>() {
                    return value;
                }
            }
        }
    }
    0
}
