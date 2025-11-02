// HTTP Range Request Integration Tests
// Phase 20: Extended Integration Tests - Range Request Support
//
// Tests that the proxy correctly handles HTTP Range requests (RFC 7233)
// for partial content delivery, video seeking, and parallel downloads.

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

// Helper: Setup LocalStack with a test file
async fn setup_localstack_with_test_file<'a>(
    docker: &'a Cli,
    bucket_name: &str,
    object_key: &str,
    content: &[u8],
) -> (testcontainers::Container<'a, LocalStack>, String) {
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    // Create S3 client and upload test file
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

    // Upload test file
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key(object_key)
        .body(content.to_vec().into())
        .send()
        .await
        .expect("Failed to upload test file");

    log::info!(
        "Uploaded test file to LocalStack: s3://{}/{}  ({} bytes)",
        bucket_name,
        object_key,
        content.len()
    );

    (container, endpoint)
}

#[test]
#[ignore] // Requires Docker and running proxy - run with: cargo test -- --ignored
fn test_range_request_returns_206_partial_content() {
    init_logging();

    // RED PHASE: This test will fail because we haven't implemented Range request
    // support in the proxy yet. The proxy should:
    // 1. Parse Range header from client request
    // 2. Forward Range header to S3 backend
    // 3. Return 206 Partial Content status (not 200 OK)
    // 4. Include Content-Range header in response
    // 5. Stream only the requested byte range

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create test file with known content
        let test_content = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let (_container, s3_endpoint) =
            setup_localstack_with_test_file(&docker, "test-bucket", "test.txt", test_content)
                .await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // TODO: Start Yatagarasu proxy server here
        // For now, this test will fail with "Connection refused" because proxy isn't running
        // Once we implement proxy startup in tests, this will test the actual Range support

        let proxy_url = "http://127.0.0.1:18080/test/test.txt";

        // Make Range request: bytes=0-9 (first 10 bytes: "0123456789")
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        let response = client
            .get(proxy_url)
            .header("Range", "bytes=0-9")
            .send()
            .await
            .expect("Failed to send Range request to proxy");

        // Verify response status is 206 Partial Content
        assert_eq!(
            response.status(),
            reqwest::StatusCode::PARTIAL_CONTENT,
            "Range request should return 206 Partial Content"
        );

        // Verify Content-Range header
        let content_range = response
            .headers()
            .get("content-range")
            .expect("Response should include Content-Range header");

        let content_range_str = content_range.to_str().unwrap();
        assert!(
            content_range_str.starts_with("bytes 0-9/"),
            "Content-Range should be 'bytes 0-9/<total>', got: {}",
            content_range_str
        );

        // Verify response body contains correct byte range
        let body = response.bytes().await.expect("Failed to read response body");
        assert_eq!(
            body.len(),
            10,
            "Response body should contain 10 bytes (0-9)"
        );
        assert_eq!(
            &body[..],
            b"0123456789",
            "Response body should contain bytes 0-9"
        );

        log::info!("Range request test passed: received correct partial content");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_range_request_includes_content_range_header() {
    init_logging();

    // RED PHASE: Verify that Content-Range header is correctly formatted
    // Format: "Content-Range: bytes <start>-<end>/<total>"
    // Example: "Content-Range: bytes 0-1023/5000"

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let test_content = vec![0u8; 5000]; // 5000 bytes of zeros
        let (_container, _s3_endpoint) =
            setup_localstack_with_test_file(&docker, "test-bucket", "data.bin", &test_content)
                .await;

        let proxy_url = "http://127.0.0.1:18080/test/data.bin";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request bytes 100-199 (100 bytes starting at offset 100)
        let response = client
            .get(proxy_url)
            .header("Range", "bytes=100-199")
            .send()
            .await
            .expect("Failed to send Range request");

        assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);

        let content_range = response
            .headers()
            .get("content-range")
            .expect("Response must include Content-Range header")
            .to_str()
            .unwrap()
            .to_string();

        // Verify format: "bytes 100-199/5000"
        assert_eq!(
            content_range, "bytes 100-199/5000",
            "Content-Range header should match requested range and total size"
        );

        let body = response.bytes().await.unwrap();
        assert_eq!(body.len(), 100, "Should receive exactly 100 bytes");

        log::info!("Content-Range header test passed: {}", content_range);
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_range_request_returns_correct_byte_range() {
    init_logging();

    // RED PHASE: Verify that the response body contains the exact bytes requested
    // This is critical for video seeking and parallel downloads

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create test file with identifiable pattern
        let mut test_content = Vec::new();
        for i in 0..1000u16 {
            test_content.extend_from_slice(&i.to_be_bytes());
        }
        // Total: 2000 bytes (1000 * 2 bytes)

        let (_container, _s3_endpoint) =
            setup_localstack_with_test_file(&docker, "test-bucket", "pattern.bin", &test_content)
                .await;

        let proxy_url = "http://127.0.0.1:18080/test/pattern.bin";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request bytes 100-109 (5 u16 values: 50, 51, 52, 53, 54)
        let response = client
            .get(proxy_url)
            .header("Range", "bytes=100-109")
            .send()
            .await
            .expect("Failed to send Range request");

        assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);

        let body = response.bytes().await.unwrap();
        assert_eq!(body.len(), 10, "Should receive exactly 10 bytes");

        // Verify content: bytes 100-109 contain u16 values 50-54
        let expected: Vec<u8> = vec![
            0, 50, // 50
            0, 51, // 51
            0, 52, // 52
            0, 53, // 53
            0, 54, // 54
        ];

        assert_eq!(
            &body[..],
            &expected[..],
            "Response should contain exact byte range from original file"
        );

        log::info!("Byte range verification test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_suffix_range_returns_last_n_bytes() {
    init_logging();

    // RED PHASE: Test suffix-byte-range-spec (RFC 7233 Section 2.1)
    // Format: "Range: bytes=-<suffix-length>"
    // Example: "Range: bytes=-1000" returns last 1000 bytes

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let test_content = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        // 62 bytes total
        let total_len = test_content.len();

        let (_container, _s3_endpoint) =
            setup_localstack_with_test_file(&docker, "test-bucket", "suffix.txt", test_content)
                .await;

        let proxy_url = "http://127.0.0.1:18080/test/suffix.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request last 10 bytes: "stuvwxyz" (offset 52-61)
        let response = client
            .get(proxy_url)
            .header("Range", "bytes=-10")
            .send()
            .await
            .expect("Failed to send suffix Range request");

        assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);

        let content_range = response
            .headers()
            .get("content-range")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let expected_range = format!("bytes {}-{}/{}", total_len - 10, total_len - 1, total_len);
        assert_eq!(
            content_range, expected_range,
            "Content-Range should indicate last 10 bytes"
        );

        let body = response.bytes().await.unwrap();
        assert_eq!(body.len(), 10);
        assert_eq!(&body[..], b"stuvwxyz\n\n"); // Last 10 bytes

        log::info!("Suffix range test passed: {}", content_range);
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_open_ended_range_returns_from_offset_to_end() {
    init_logging();

    // RED PHASE: Test open-ended range (RFC 7233)
    // Format: "Range: bytes=<offset>-"
    // Example: "Range: bytes=1000-" returns from byte 1000 to EOF

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let test_content = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let total_len = test_content.len();

        let (_container, _s3_endpoint) = setup_localstack_with_test_file(
            &docker,
            "test-bucket",
            "openended.txt",
            test_content,
        )
        .await;

        let proxy_url = "http://127.0.0.1:18080/test/openended.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Request from byte 50 to end: "YZabcdefghijklmnopqrstuvwxyz"
        let response = client
            .get(proxy_url)
            .header("Range", "bytes=50-")
            .send()
            .await
            .expect("Failed to send open-ended Range request");

        assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);

        let content_range = response
            .headers()
            .get("content-range")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let expected_range = format!("bytes 50-{}/{}", total_len - 1, total_len);
        assert_eq!(
            content_range, expected_range,
            "Content-Range should indicate offset to end"
        );

        let body = response.bytes().await.unwrap();
        let expected_len = total_len - 50;
        assert_eq!(body.len(), expected_len);
        assert_eq!(&body[..], &test_content[50..]);

        log::info!("Open-ended range test passed: {}", content_range);
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_invalid_range_returns_416_range_not_satisfiable() {
    init_logging();

    // RED PHASE: Test invalid range handling (RFC 7233 Section 4.4)
    // If range is invalid (start > end, or offset > file size), return 416

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let test_content = b"0123456789"; // 10 bytes
        let total_len = test_content.len();

        let (_container, _s3_endpoint) =
            setup_localstack_with_test_file(&docker, "test-bucket", "invalid.txt", test_content)
                .await;

        let proxy_url = "http://127.0.0.1:18080/test/invalid.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Test 1: Range start beyond file size (bytes=100-200, but file is only 10 bytes)
        let response = client
            .get(proxy_url)
            .header("Range", "bytes=100-200")
            .send()
            .await
            .expect("Failed to send invalid Range request");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::RANGE_NOT_SATISFIABLE,
            "Range beyond file size should return 416"
        );

        // Verify Content-Range header shows valid range
        let content_range = response
            .headers()
            .get("content-range")
            .expect("416 response should include Content-Range with valid range")
            .to_str()
            .unwrap();

        let expected_content_range = format!("bytes */{}", total_len);
        assert_eq!(
            content_range, expected_content_range,
            "Content-Range should indicate valid range for 416 response"
        );

        // Test 2: Invalid range where start > end (bytes=50-20)
        let response2 = client
            .get(proxy_url)
            .header("Range", "bytes=50-20")
            .send()
            .await
            .expect("Failed to send invalid Range request");

        assert_eq!(
            response2.status(),
            reqwest::StatusCode::RANGE_NOT_SATISFIABLE,
            "Range where start > end should return 416"
        );

        log::info!("Invalid range test passed: returned 416 as expected");
    });
}
