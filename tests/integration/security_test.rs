// Integration tests for security validations
// Tests path traversal, request size limits, and security metrics

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH};
use serde_json::Value;
use std::str::FromStr;

#[tokio::test]
#[ignore] // Requires running proxy with test configuration
async fn test_path_traversal_blocked() {
    // Test various path traversal patterns
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Test 1: Basic ../ path traversal
    let response = client
        .get(&format!("{}/test/../../../etc/passwd", base_url))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Path traversal with ../ should return 400"
    );

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "Bad Request");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Path traversal attempt detected"));

    // Test 2: URL-encoded path traversal
    let response = client
        .get(&format!("{}/test/%2e%2e%2f%2e%2e%2fetc/passwd", base_url))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "URL-encoded path traversal should return 400"
    );

    // Test 3: Backslash path traversal (Windows-style)
    let response = client
        .get(&format!("{}/test/..\\..\\windows\\system32", base_url))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Backslash path traversal should return 400"
    );

    // Test 4: Valid path should work
    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .send()
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND,
        "Valid path should not be blocked"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_uri_too_long_blocked() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Default limit is 8192 bytes
    // Create a URI that exceeds this limit
    let long_path = "a".repeat(9000);
    let url = format!("{}/test/{}", base_url, long_path);

    let response = client.get(&url).send().await.expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::URI_TOO_LONG,
        "URI exceeding 8KB should return 414"
    );

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "URI Too Long");
    assert!(
        body["message"].as_str().unwrap().contains("URI length")
            && body["message"].as_str().unwrap().contains("exceeds limit")
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_headers_too_large_blocked() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Default limit is 64KB
    // Create headers that exceed this limit
    let mut headers = HeaderMap::new();

    // Add multiple large headers to exceed 64KB total
    for i in 0..100 {
        let header_name = format!("X-Custom-Header-{}", i);
        let header_value = "x".repeat(1000); // 1KB per header = 100KB total
        headers.insert(
            HeaderName::from_str(&header_name).unwrap(),
            HeaderValue::from_str(&header_value).unwrap(),
        );
    }

    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .headers(headers)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
        "Headers exceeding 64KB should return 431"
    );

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "Request Header Fields Too Large");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("Total header size")
            && body["message"].as_str().unwrap().contains("exceeds limit")
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_payload_too_large_blocked() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Default limit is 10MB
    // Create a payload that exceeds this limit
    let large_payload = vec![0u8; 11 * 1024 * 1024]; // 11MB

    let response = client
        .post(&format!("{}/test/upload", base_url))
        .header(CONTENT_LENGTH, large_payload.len().to_string())
        .body(large_payload)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "Payload exceeding 10MB should return 413"
    );

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "Payload Too Large");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("Request payload size")
            && body["message"].as_str().unwrap().contains("exceeds limit")
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_security_metrics_incremented() {
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

    // Trigger path traversal block
    let _ = client
        .get(&format!("{}/test/../../../etc/passwd", base_url))
        .send()
        .await;

    // Trigger URI too long
    let long_path = "a".repeat(9000);
    let _ = client
        .get(&format!("{}/test/{}", base_url, long_path))
        .send()
        .await;

    // Get updated metrics
    let updated_metrics = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to get metrics")
        .text()
        .await
        .expect("Failed to read metrics");

    // Verify security metrics exist and were incremented
    assert!(
        updated_metrics.contains("security_path_traversal_blocked_total"),
        "Path traversal metric should exist"
    );
    assert!(
        updated_metrics.contains("security_uri_too_long_total"),
        "URI too long metric should exist"
    );
    assert!(
        updated_metrics.contains("security_headers_too_large_total"),
        "Headers too large metric should exist"
    );
    assert!(
        updated_metrics.contains("security_payload_too_large_total"),
        "Payload too large metric should exist"
    );

    // Extract metric values (simple parsing for test)
    let path_traversal_count =
        extract_metric_value(&updated_metrics, "security_path_traversal_blocked_total");
    let uri_too_long_count = extract_metric_value(&updated_metrics, "security_uri_too_long_total");

    assert!(
        path_traversal_count > 0,
        "Path traversal metric should be incremented"
    );
    assert!(
        uri_too_long_count > 0,
        "URI too long metric should be incremented"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_valid_requests_not_blocked() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Test 1: Normal GET request
    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .send()
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND,
        "Valid GET request should not be blocked by security"
    );

    // Test 2: Request with reasonable headers
    let mut headers = HeaderMap::new();
    headers.insert("X-Custom-Header", HeaderValue::from_static("test-value"));
    headers.insert("User-Agent", HeaderValue::from_static("Integration-Test"));

    let response = client
        .get(&format!("{}/test/sample.txt", base_url))
        .headers(headers)
        .send()
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND,
        "Valid request with headers should not be blocked"
    );

    // Test 3: POST with reasonable payload
    let small_payload = vec![0u8; 1024]; // 1KB

    let response = client
        .post(&format!("{}/test/upload", base_url))
        .body(small_payload)
        .send()
        .await
        .expect("Request failed");

    // Should not be blocked by security (may fail for other reasons like 404)
    assert_ne!(
        response.status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "Small payload should not trigger size limit"
    );
    assert_ne!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Valid request should not be rejected as malicious"
    );
}

// Helper function to extract metric value from Prometheus text format
fn extract_metric_value(metrics: &str, metric_name: &str) -> u64 {
    for line in metrics.lines() {
        if line.starts_with(metric_name) && !line.starts_with('#') {
            // Format: "metric_name value"
            if let Some(value_str) = line.split_whitespace().nth(1) {
                if let Ok(value) = value_str.parse::<u64>() {
                    return value;
                }
            }
        }
    }
    0
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_security_error_messages_are_clear() {
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:18080";

    // Test path traversal error message
    let response = client
        .get(&format!("{}/test/../../../etc/passwd", base_url))
        .send()
        .await
        .expect("Request failed");

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["error"].is_string(), "Error field should be present");
    assert!(
        body["message"].is_string(),
        "Message field should be present"
    );
    assert!(body["status"].is_number(), "Status field should be present");
    assert_eq!(body["status"], 400, "Status should match HTTP status code");

    // Verify message is descriptive
    let message = body["message"].as_str().unwrap();
    assert!(
        message.contains("Path traversal attempt detected"),
        "Error message should describe the issue"
    );
}
