// Integration tests for security validations
// Tests path traversal, request size limits, and security metrics

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_LENGTH};
use serde_json::Value;
use std::str::FromStr;

// Hyper imports for raw HTTP requests (needed for path traversal tests)
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

/// Helper to make raw HTTP requests without URL normalization
/// This is needed for path traversal tests since reqwest normalizes paths
async fn raw_http_request(path: &str) -> Result<(u16, String), Box<dyn std::error::Error>> {
    // Connect to proxy
    let stream = TcpStream::connect("127.0.0.1:18080").await?;
    let io = TokioIo::new(stream);

    // HTTP/1 handshake
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn connection task
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    // Create request with RAW path (not normalized!)
    let req = hyper::Request::builder()
        .method("GET")
        .uri(path) // Raw path preserved here!
        .header("Host", "127.0.0.1:18080")
        .body(String::new())?;

    // Send request
    let res = sender.send_request(req).await?;

    // Get status
    let status = res.status().as_u16();

    // Read body using http_body_util
    use http_body_util::BodyExt;
    let body_bytes = res.into_body().collect().await?.to_bytes();
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    Ok((status, body))
}

#[tokio::test]
#[ignore] // Requires running proxy with test configuration
async fn test_path_traversal_blocked() {
    // Test 1: Basic ../ path traversal (using hyper for raw path)
    let (status, body) = raw_http_request("/test/../../../etc/passwd")
        .await
        .expect("Request failed");

    assert_eq!(status, 400, "Path traversal with ../ should return 400");

    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(json["error"], "Bad Request");
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Path traversal attempt detected"));

    // Test 2: URL-encoded path traversal
    let (status, _body) = raw_http_request("/test/%2e%2e%2f%2e%2e%2fetc/passwd")
        .await
        .expect("Request failed");
    assert_eq!(status, 400, "URL-encoded path traversal should return 400");

    // Test 3: Backslash path traversal (Windows-style)
    let (status, _body) = raw_http_request("/test/..\\..\\windows\\system32")
        .await
        .expect("Request failed");
    assert_eq!(status, 400, "Backslash path traversal should return 400");

    // Test 4: Valid path should work (can use reqwest for this)
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:18080/test/sample.txt")
        .send()
        .await
        .expect("Request failed");

    // Valid path should not return 400 (Bad Request from path traversal detection)
    // May return 200/404 (file exists/not found), 403 (auth required), or other codes
    assert_ne!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Valid path should not be blocked by path traversal detection (got {})",
        response.status()
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

    // Trigger path traversal block (using raw HTTP request)
    let _ = raw_http_request("/test/../../../etc/passwd").await;

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

    // Should not be blocked by security validation (not 400, 413, 414, 431)
    // May return 200 (success), 403 (auth required), 404 (not found), etc.
    assert!(
        response.status() != reqwest::StatusCode::BAD_REQUEST
            && response.status() != reqwest::StatusCode::PAYLOAD_TOO_LARGE
            && response.status() != reqwest::StatusCode::URI_TOO_LONG
            && response.status() != reqwest::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
        "Valid GET request should not be blocked by security (got {})",
        response.status()
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
        response.status() != reqwest::StatusCode::BAD_REQUEST
            && response.status() != reqwest::StatusCode::PAYLOAD_TOO_LARGE
            && response.status() != reqwest::StatusCode::URI_TOO_LONG
            && response.status() != reqwest::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
        "Valid request with headers should not be blocked (got {})",
        response.status()
    );

    // Test 3: POST with reasonable payload
    let small_payload = vec![0u8; 1024]; // 1KB

    let response = client
        .post(&format!("{}/test/upload", base_url))
        .body(small_payload)
        .send()
        .await
        .expect("Request failed");

    // Should not be blocked by security validation for size
    // Note: May get 400 from S3/backend for other reasons, but not 413 from payload size limit
    assert_ne!(
        response.status(),
        reqwest::StatusCode::PAYLOAD_TOO_LARGE,
        "Small payload (1KB) should not trigger payload size limit (got {})",
        response.status()
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
    // Test path traversal error message (using raw HTTP request)
    let (status, body) = raw_http_request("/test/../../../etc/passwd")
        .await
        .expect("Request failed");

    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert!(json["error"].is_string(), "Error field should be present");
    assert!(
        json["message"].is_string(),
        "Message field should be present"
    );
    assert!(json["status"].is_number(), "Status field should be present");
    assert_eq!(json["status"], 400, "Status should match HTTP status code");
    assert_eq!(status, 400, "HTTP status should be 400");

    // Verify message is descriptive
    let message = json["message"].as_str().unwrap();
    assert!(
        message.contains("Path traversal attempt detected"),
        "Error message should describe the issue"
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_sql_injection_blocked() {
    // Test various SQL injection patterns in path
    // These should be detected and blocked with 400 Bad Request

    // Test 1: Classic SQL injection with OR
    let (status, body) = raw_http_request("/test/file' OR '1'='1.txt")
        .await
        .expect("Request failed");

    assert_eq!(
        status, 400,
        "SQL injection with OR should return 400 Bad Request"
    );

    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(json["error"], "Bad Request");
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains("SQL injection attempt detected"),
        "Error message should indicate SQL injection detection"
    );

    // Test 2: SQL injection with DROP TABLE
    let (status, _body) = raw_http_request("/test/file'; DROP TABLE users--.txt")
        .await
        .expect("Request failed");
    assert_eq!(
        status, 400,
        "SQL injection with DROP TABLE should return 400"
    );

    // Test 3: SQL injection with UNION SELECT
    let (status, _body) = raw_http_request("/test/file' UNION SELECT NULL--.txt")
        .await
        .expect("Request failed");
    assert_eq!(
        status, 400,
        "SQL injection with UNION SELECT should return 400"
    );

    // Test 4: SQL injection with comment terminator
    let (status, _body) = raw_http_request("/test/admin'--.txt")
        .await
        .expect("Request failed");
    assert_eq!(
        status, 400,
        "SQL injection with comment terminator should return 400"
    );

    // Test 5: URL-encoded SQL injection
    let (status, _body) = raw_http_request("/test/file%27%20OR%20%271%27=%271.txt") // ' OR '1'='1
        .await
        .expect("Request failed");
    assert_eq!(status, 400, "URL-encoded SQL injection should return 400");

    // Test 6: Valid path with single quote (should NOT be blocked)
    // File named "user's_document.txt" is a legitimate filename
    let (status, _body) = raw_http_request("/test/user's_document.txt")
        .await
        .expect("Request failed");

    // Should NOT be blocked as SQL injection (may get 403/404 but not 400 with SQL injection message)
    // We allow single quotes in filenames, only block SQL injection patterns
    assert_ne!(
        status, 400,
        "Valid filename with single quote should not be blocked (got {})",
        status
    );
}

// ============================================================================
// Phase 25: HTTP Method Validation (Read-Only Proxy Enforcement)
// ============================================================================

/// Helper to make raw HTTP requests with custom method
async fn raw_http_request_with_method(
    method: &str,
    path: &str,
) -> Result<(u16, String, HeaderMap), Box<dyn std::error::Error>> {
    // Connect to proxy
    let stream = TcpStream::connect("127.0.0.1:18080").await?;
    let io = TokioIo::new(stream);

    // HTTP/1 handshake
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // Spawn connection task
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    // Create request with specified method
    let req = hyper::Request::builder()
        .method(method)
        .uri(path)
        .header("Host", "127.0.0.1:18080")
        .body(String::new())?;

    // Send request
    let res = sender.send_request(req).await?;

    // Get status and headers
    let status = res.status().as_u16();
    let headers = res.headers().clone();

    // Convert hyper::HeaderMap to reqwest::HeaderMap
    let mut reqwest_headers = HeaderMap::new();
    for (name, value) in headers.iter() {
        reqwest_headers.insert(
            HeaderName::from_str(name.as_str()).unwrap(),
            HeaderValue::from_bytes(value.as_bytes()).unwrap(),
        );
    }

    // Read body
    use http_body_util::BodyExt;
    let body_bytes = res.into_body().collect().await?.to_bytes();
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    Ok((status, body, reqwest_headers))
}

#[tokio::test]
#[ignore] // Requires running proxy with test configuration
async fn test_get_requests_allowed() {
    // GET requests to S3 paths should be allowed
    let (status, _body, _headers) = raw_http_request_with_method("GET", "/test/sample.txt")
        .await
        .expect("Request failed");

    // GET should not return 405 Method Not Allowed
    // May return 200 (success), 404 (not found), 403 (auth required), etc.
    assert_ne!(
        status,
        reqwest::StatusCode::METHOD_NOT_ALLOWED.as_u16(),
        "GET requests should be allowed (got {})",
        status
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_head_requests_allowed() {
    // HEAD requests to S3 paths should be allowed
    let (status, _body, _headers) = raw_http_request_with_method("HEAD", "/test/sample.txt")
        .await
        .expect("Request failed");

    // HEAD should not return 405 Method Not Allowed
    assert_ne!(
        status,
        reqwest::StatusCode::METHOD_NOT_ALLOWED.as_u16(),
        "HEAD requests should be allowed (got {})",
        status
    );
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_put_requests_blocked() {
    // PUT requests to S3 paths should return 405 Method Not Allowed
    let (status, body, headers) = raw_http_request_with_method("PUT", "/test/upload.txt")
        .await
        .expect("Request failed");

    assert_eq!(
        status,
        reqwest::StatusCode::METHOD_NOT_ALLOWED.as_u16(),
        "PUT requests should return 405 Method Not Allowed"
    );

    // Verify Allow header is present
    assert!(
        headers.contains_key("allow"),
        "405 response should include Allow header"
    );

    let allow_header = headers.get("allow").unwrap().to_str().unwrap();
    assert!(
        allow_header.contains("GET") && allow_header.contains("HEAD"),
        "Allow header should include GET and HEAD (got: {})",
        allow_header
    );

    // Verify error response body
    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(json["error"], "Method Not Allowed");
    assert_eq!(json["status"], 405);
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("read-only S3 proxy"));
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_post_requests_blocked() {
    // POST requests to S3 paths should return 405 Method Not Allowed
    // (except /admin/reload which is handled separately)
    let (status, body, _headers) = raw_http_request_with_method("POST", "/test/data")
        .await
        .expect("Request failed");

    assert_eq!(
        status,
        reqwest::StatusCode::METHOD_NOT_ALLOWED.as_u16(),
        "POST requests to S3 paths should return 405 Method Not Allowed"
    );

    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(json["error"], "Method Not Allowed");
    assert_eq!(json["status"], 405);
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_delete_requests_blocked() {
    // DELETE requests to S3 paths should return 405 Method Not Allowed
    let (status, body, _headers) = raw_http_request_with_method("DELETE", "/test/file.txt")
        .await
        .expect("Request failed");

    assert_eq!(
        status,
        reqwest::StatusCode::METHOD_NOT_ALLOWED.as_u16(),
        "DELETE requests should return 405 Method Not Allowed"
    );

    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(json["error"], "Method Not Allowed");
    assert_eq!(json["status"], 405);
}

#[tokio::test]
#[ignore] // Requires running proxy
async fn test_patch_requests_blocked() {
    // PATCH requests to S3 paths should return 405 Method Not Allowed
    let (status, body, _headers) = raw_http_request_with_method("PATCH", "/test/file.txt")
        .await
        .expect("Request failed");

    assert_eq!(
        status,
        reqwest::StatusCode::METHOD_NOT_ALLOWED.as_u16(),
        "PATCH requests should return 405 Method Not Allowed"
    );

    let json: Value = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(json["error"], "Method Not Allowed");
    assert_eq!(json["status"], 405);
}
