// Error Handling and Edge Cases Integration Tests
// Phase 20: Extended Integration Tests - Error Scenarios
//
// Tests that the proxy correctly handles various error conditions:
// - S3 errors (404, 403, network failures)
// - Malformed client requests (400)
// - Timeouts (504)
// - Large file streaming without buffering

use std::sync::Once;
use std::time::Duration;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
}

// Helper: Setup LocalStack with a test bucket
async fn setup_localstack_with_bucket<'a>(
    docker: &'a Cli,
    bucket_name: &str,
) -> (testcontainers::Container<'a, LocalStack>, String) {
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    // Create S3 client
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
        .expect("Failed to create bucket");

    log::info!("Created LocalStack bucket: {}", bucket_name);

    (container, endpoint)
}

#[test]
#[ignore] // Requires Docker and running proxy - run with: cargo test -- --ignored
fn test_nonexistent_s3_object_returns_404() {
    init_logging();

    // RED PHASE: This test verifies that when an S3 object does not exist,
    // the proxy returns 404 Not Found to the client (not 500 Internal Server Error).
    //
    // Expected behavior:
    // 1. Client requests /test/does-not-exist.txt
    // 2. Proxy forwards to S3: GET s3://test-bucket/does-not-exist.txt
    // 3. S3 returns NoSuchKey error
    // 4. Proxy maps NoSuchKey to 404 Not Found
    // 5. Client receives 404 with appropriate error message

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) = setup_localstack_with_bucket(&docker, "test-bucket").await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // Note: We intentionally do NOT upload any files to the bucket
        // so that requests will trigger NoSuchKey errors

        // TODO: Start Yatagarasu proxy server here
        // For now, this test will fail with "Connection refused"

        let proxy_url = "http://127.0.0.1:18080/test/does-not-exist.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        let response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        // Verify response status is 404 Not Found
        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Non-existent S3 object should return 404 Not Found"
        );

        // Verify response body contains helpful error message
        let body = response.text().await.expect("Failed to read response body");
        assert!(
            body.contains("not found") || body.contains("NoSuchKey"),
            "Error message should indicate object not found, got: {}",
            body
        );

        log::info!("404 Not Found test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_s3_access_denied_returns_403() {
    init_logging();

    // RED PHASE: This test verifies that when S3 returns AccessDenied,
    // the proxy returns 403 Forbidden to the client.
    //
    // Expected behavior:
    // 1. Client requests /test/forbidden.txt
    // 2. Proxy uses credentials without permission
    // 3. S3 returns AccessDenied error
    // 4. Proxy maps AccessDenied to 403 Forbidden
    // 5. Client receives 403 with appropriate error message
    //
    // Setup challenge: LocalStack with free tier may not enforce IAM permissions
    // properly. This test demonstrates the expected behavior but may need
    // real AWS credentials or moto library for proper testing.

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) = setup_localstack_with_bucket(&docker, "test-bucket").await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // TODO: Configure bucket policy to deny access to specific object
        // This may require additional LocalStack setup or real AWS testing

        let proxy_url = "http://127.0.0.1:18080/test/forbidden.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        let response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        // Verify response status is 403 Forbidden
        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "S3 AccessDenied should return 403 Forbidden"
        );

        // Verify response body contains helpful error message
        let body = response.text().await.expect("Failed to read response body");
        assert!(
            body.contains("forbidden") || body.contains("access denied"),
            "Error message should indicate access denied, got: {}",
            body
        );

        log::info!("403 Forbidden test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_network_error_to_s3_returns_502() {
    init_logging();

    // RED PHASE: This test verifies that when the proxy cannot reach S3
    // (network error, connection refused, DNS failure), it returns
    // 502 Bad Gateway to the client.
    //
    // Expected behavior:
    // 1. Client requests /test/file.txt
    // 2. Proxy attempts to connect to S3 endpoint
    // 3. Connection fails (S3 endpoint unreachable)
    // 4. Proxy catches network error
    // 5. Proxy returns 502 Bad Gateway to client
    //
    // Test approach: Configure proxy with invalid S3 endpoint (non-existent host)

    // This test requires special proxy configuration with invalid S3 endpoint
    // We'll simulate this by using a non-routable IP address or invalid hostname

    let proxy_url = "http://127.0.0.1:18080/test/file.txt";

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // TODO: This test requires proxy to be configured with invalid S3 endpoint
        // For example: endpoint_url: "http://192.0.2.1:9000" (TEST-NET-1, non-routable)

        let response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        // Verify response status is 502 Bad Gateway
        assert_eq!(
            response.status(),
            reqwest::StatusCode::BAD_GATEWAY,
            "Network error to S3 should return 502 Bad Gateway"
        );

        // Verify response body contains helpful error message
        let body = response.text().await.expect("Failed to read response body");
        assert!(
            body.contains("bad gateway")
                || body.contains("upstream")
                || body.contains("connection"),
            "Error message should indicate upstream connection failure, got: {}",
            body
        );

        log::info!("502 Bad Gateway test passed");
    });
}

#[test]
#[ignore] // Requires running proxy
fn test_malformed_request_returns_400() {
    init_logging();

    // RED PHASE: This test verifies that malformed client requests
    // return 400 Bad Request instead of causing server errors.
    //
    // Expected behavior:
    // 1. Client sends malformed request (invalid path, headers, etc.)
    // 2. Proxy validates request
    // 3. Proxy detects invalid request
    // 4. Proxy returns 400 Bad Request with helpful error message
    //
    // Examples of malformed requests:
    // - Path with null bytes
    // - Path with invalid URL encoding
    // - Missing required headers
    // - Invalid Range header format

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        // Test 1: Path with invalid URL encoding
        let invalid_url = "http://127.0.0.1:18080/test/%ZZ%invalid.txt";
        let response = client
            .get(invalid_url)
            .send()
            .await
            .expect("Failed to send malformed request");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::BAD_REQUEST,
            "Invalid URL encoding should return 400 Bad Request"
        );

        // Test 2: Invalid Range header format
        let proxy_url = "http://127.0.0.1:18080/test/file.txt";
        let response = client
            .get(proxy_url)
            .header("Range", "invalid-range-format")
            .send()
            .await
            .expect("Failed to send request with invalid Range header");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::BAD_REQUEST,
            "Invalid Range header should return 400 Bad Request"
        );

        log::info!("400 Bad Request test passed");
    });
}

#[test]
#[ignore] // Requires running proxy with timeout configuration
fn test_request_timeout_returns_504() {
    init_logging();

    // RED PHASE: This test verifies that when S3 takes too long to respond,
    // the proxy returns 504 Gateway Timeout instead of hanging indefinitely.
    //
    // Expected behavior:
    // 1. Client requests /test/slow-file.txt
    // 2. Proxy forwards request to S3
    // 3. S3 is slow to respond (simulated by firewall rule or slow endpoint)
    // 4. Proxy timeout expires (e.g., 30 seconds)
    // 5. Proxy cancels S3 request
    // 6. Proxy returns 504 Gateway Timeout to client
    //
    // Test approach: Configure proxy with very short timeout, use slow S3 endpoint

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // TODO: This test requires:
        // 1. Proxy configured with short timeout (e.g., 1 second)
        // 2. S3 endpoint that responds slowly (can use tc/netem to add latency)

        let proxy_url = "http://127.0.0.1:18080/test/slow-file.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10)) // Client timeout > proxy timeout
            .build()
            .expect("Failed to create HTTP client");

        let response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        // Verify response status is 504 Gateway Timeout
        assert_eq!(
            response.status(),
            reqwest::StatusCode::GATEWAY_TIMEOUT,
            "S3 timeout should return 504 Gateway Timeout"
        );

        // Verify response body contains helpful error message
        let body = response.text().await.expect("Failed to read response body");
        assert!(
            body.contains("timeout") || body.contains("timed out"),
            "Error message should indicate timeout, got: {}",
            body
        );

        log::info!("504 Gateway Timeout test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_large_file_streams_without_buffering() {
    init_logging();

    // RED PHASE: This test verifies that large files (100MB+) stream through
    // the proxy without buffering the entire file in memory.
    //
    // Expected behavior:
    // 1. Upload 100MB file to S3
    // 2. Client requests large file from proxy
    // 3. Proxy starts streaming immediately (low TTFB)
    // 4. Proxy uses constant memory (~64KB buffer) regardless of file size
    // 5. Client receives complete file
    //
    // Verification:
    // - Monitor proxy memory usage during transfer (should stay constant)
    // - Verify TTFB is low (<500ms, not waiting for full file)
    // - Verify complete file is received correctly

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) = setup_localstack_with_bucket(&docker, "test-bucket").await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // Create S3 client and upload large file
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Generate 100MB test file (repeating pattern for verification)
        let chunk_size = 1024 * 1024; // 1MB chunks
        let num_chunks = 100; // 100 chunks = 100MB
        let mut large_file = Vec::with_capacity(chunk_size * num_chunks);
        for i in 0..num_chunks {
            let chunk = vec![(i % 256) as u8; chunk_size];
            large_file.extend_from_slice(&chunk);
        }

        log::info!(
            "Generated {}MB test file ({} bytes)",
            large_file.len() / (1024 * 1024),
            large_file.len()
        );

        // Upload to S3
        let upload_start = std::time::Instant::now();
        s3_client
            .put_object()
            .bucket("test-bucket")
            .key("large-file.bin")
            .body(large_file.clone().into())
            .send()
            .await
            .expect("Failed to upload large file to S3");

        log::info!("Uploaded large file to S3 in {:?}", upload_start.elapsed());

        // TODO: Start Yatagarasu proxy server here

        let proxy_url = "http://127.0.0.1:18080/test/large-file.bin";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120)) // 2 minutes for large file
            .build()
            .expect("Failed to create HTTP client");

        // Measure Time To First Byte (TTFB)
        let request_start = std::time::Instant::now();
        let mut response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        let ttfb = request_start.elapsed();
        log::info!("Time To First Byte (TTFB): {:?}", ttfb);

        // TTFB should be low (<500ms) because we're streaming, not buffering
        assert!(
            ttfb < Duration::from_millis(500),
            "TTFB should be <500ms for streaming (got {:?})",
            ttfb
        );

        // Verify we got 200 OK
        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Large file request should return 200 OK"
        );

        // Stream response body and verify content
        let mut received_bytes = 0;
        while let Some(chunk) = response.chunk().await.expect("Failed to read chunk") {
            received_bytes += chunk.len();
            // Verify chunk content matches expected pattern
            // (detailed verification omitted for brevity)
        }

        let download_time = request_start.elapsed();
        log::info!(
            "Downloaded {} bytes in {:?} ({:.2} MB/s)",
            received_bytes,
            download_time,
            (received_bytes as f64 / (1024.0 * 1024.0)) / download_time.as_secs_f64()
        );

        // Verify we received the complete file
        assert_eq!(
            received_bytes,
            large_file.len(),
            "Should receive complete file"
        );

        log::info!("Large file streaming test passed");
    });
}
