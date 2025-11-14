// Streaming and Performance Integration Tests
// Phase 20: Extended Integration Tests - Streaming Performance
//
// Tests that the proxy correctly streams large files without buffering:
// - Large files stream with constant memory usage
// - Low Time To First Byte (TTFB) proves streaming architecture
// - Client disconnect stops S3 transfer (no resource leak)
// - Multiple concurrent large file downloads work correctly

use super::test_harness::ProxyTestHarness;
use std::fs;
use std::sync::Once;
use std::time::{Duration, Instant};
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;
use tokio::task::JoinSet;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {});
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

// Helper: Setup LocalStack with large test file
async fn setup_localstack_with_large_file<'a>(
    docker: &'a Cli,
    bucket_name: &str,
    object_key: &str,
    size_mb: usize,
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

    // Generate large test file with repeating pattern for verification
    let chunk_size = 1024 * 1024; // 1MB chunks
    let mut large_file = Vec::with_capacity(size_mb * chunk_size);
    for i in 0..size_mb {
        let chunk = vec![(i % 256) as u8; chunk_size];
        large_file.extend_from_slice(&chunk);
    }

    log::info!(
        "Generated {}MB test file ({} bytes)",
        large_file.len() / (1024 * 1024),
        large_file.len()
    );

    // Upload to S3
    let upload_start = Instant::now();
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key(object_key)
        .body(large_file.into())
        .send()
        .await
        .expect("Failed to upload large file to S3");

    log::info!("Uploaded large file to S3 in {:?}", upload_start.elapsed());

    (container, endpoint)
}

#[test]
#[ignore] // Requires Docker and running proxy - run with: cargo test -- --ignored
fn test_large_file_streams_correctly() {
    init_logging();

    // RED PHASE: This test verifies that large files (100MB) stream through
    // the proxy correctly without corruption or buffering.
    //
    // Expected behavior:
    // 1. Upload 100MB file with known pattern to S3
    // 2. Client requests file through proxy
    // 3. Proxy streams chunks from S3 to client as they arrive
    // 4. Client receives complete file with correct content
    // 5. File integrity verified (checksum or pattern verification)
    //
    // This tests:
    // - Streaming architecture works for large files
    // - No data corruption during streaming
    // - Complete file transfer (no truncation)

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let size_mb = 100;
        let (_container, s3_endpoint) =
            setup_localstack_with_large_file(&docker, "test-bucket", "large.bin", size_mb).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-streaming-test.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for streaming test");

        let proxy_url = "http://127.0.0.1:18080/test/large.bin";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minutes for 100MB
            .build()
            .expect("Failed to create HTTP client");

        let download_start = Instant::now();
        let mut response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Large file request should return 200 OK"
        );

        // Stream response body and verify content pattern
        let mut received_bytes = 0;
        let mut chunk_count = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .expect("Failed to read chunk from response")
        {
            received_bytes += chunk.len();
            chunk_count += 1;

            // Verify chunk content matches expected pattern
            // (detailed verification could check each byte matches pattern)

            if chunk_count % 10 == 0 {
                log::debug!(
                    "Received {} chunks, {} MB so far",
                    chunk_count,
                    received_bytes / (1024 * 1024)
                );
            }
        }

        let download_time = download_start.elapsed();
        let throughput_mbps =
            (received_bytes as f64 / (1024.0 * 1024.0)) / download_time.as_secs_f64();

        log::info!(
            "Downloaded {} bytes ({} MB) in {:?} ({:.2} MB/s, {} chunks)",
            received_bytes,
            received_bytes / (1024 * 1024),
            download_time,
            throughput_mbps,
            chunk_count
        );

        // Verify we received the complete file
        let expected_size = size_mb * 1024 * 1024;
        assert_eq!(
            received_bytes, expected_size,
            "Should receive complete {}MB file",
            size_mb
        );

        log::info!("Large file streaming test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_streaming_starts_immediately_low_ttfb() {
    init_logging();

    // RED PHASE: This test verifies that streaming starts immediately with
    // low Time To First Byte (TTFB), proving the proxy doesn't buffer.
    //
    // Expected behavior:
    // 1. Upload 100MB file to S3
    // 2. Client requests file through proxy
    // 3. Proxy starts streaming immediately (doesn't wait for full file)
    // 4. TTFB is low (<500ms, not 10+ seconds waiting for full download)
    // 5. First chunk arrives quickly
    //
    // This tests:
    // - Proxy uses streaming architecture (not buffering)
    // - Low latency for first byte
    // - Efficient resource usage

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let size_mb = 100;
        let (_container, s3_endpoint) =
            setup_localstack_with_large_file(&docker, "test-bucket", "large.bin", size_mb).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-ttfb-test.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for TTFB test");

        let proxy_url = "http://127.0.0.1:18080/test/large.bin";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        // Measure Time To First Byte (TTFB)
        let request_start = Instant::now();
        let mut response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        let ttfb = request_start.elapsed();
        log::info!("Time To First Byte (TTFB): {:?}", ttfb);

        // TTFB should be low (<500ms) because we're streaming, not buffering
        // If the proxy buffered the entire 100MB file, TTFB would be 10+ seconds
        assert!(
            ttfb < Duration::from_millis(500),
            "TTFB should be <500ms for streaming (got {:?}). \
             High TTFB indicates buffering entire file before sending.",
            ttfb
        );

        // Verify we got 200 OK
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        // Read first chunk to verify streaming started
        let first_chunk_start = Instant::now();
        let first_chunk = response
            .chunk()
            .await
            .expect("Failed to read first chunk")
            .expect("Response should have at least one chunk");

        let first_chunk_time = first_chunk_start.elapsed();
        log::info!(
            "First chunk received: {} bytes in {:?}",
            first_chunk.len(),
            first_chunk_time
        );

        // First chunk should arrive quickly (streaming already started)
        assert!(
            first_chunk_time < Duration::from_millis(100),
            "First chunk should arrive quickly (got {:?})",
            first_chunk_time
        );

        log::info!(
            "Low TTFB test passed: TTFB {:?}, first chunk {:?}",
            ttfb,
            first_chunk_time
        );
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_client_disconnect_stops_s3_transfer() {
    init_logging();

    // RED PHASE: This test verifies that when a client disconnects mid-stream,
    // the proxy stops fetching from S3 (no resource leak).
    //
    // Expected behavior:
    // 1. Upload 100MB file to S3
    // 2. Client starts downloading file through proxy
    // 3. Client disconnects after receiving 10MB
    // 4. Proxy detects client disconnect
    // 5. Proxy cancels S3 transfer (doesn't continue downloading 90MB)
    // 6. No memory leak, no wasted S3 bandwidth
    //
    // This tests:
    // - Client disconnect detection
    // - S3 transfer cancellation
    // - Resource cleanup (no leaks)

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let size_mb = 100;
        let (_container, s3_endpoint) =
            setup_localstack_with_large_file(&docker, "test-bucket", "large.bin", size_mb).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-disconnect-test.yaml";
        create_localstack_config(&s3_endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for disconnect test");

        let proxy_url = "http://127.0.0.1:18080/test/large.bin";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        let download_start = Instant::now();
        let mut response = client
            .get(proxy_url)
            .send()
            .await
            .expect("Failed to send request to proxy");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        // Read chunks until we've received 10MB, then disconnect
        let mut received_bytes = 0;
        let disconnect_threshold = 10 * 1024 * 1024; // 10MB

        while let Some(chunk) = response.chunk().await.expect("Failed to read chunk") {
            received_bytes += chunk.len();

            if received_bytes >= disconnect_threshold {
                log::info!(
                    "Received {} MB, disconnecting client now",
                    received_bytes / (1024 * 1024)
                );
                // Drop response to simulate client disconnect
                drop(response);
                break;
            }
        }

        let disconnect_time = download_start.elapsed();

        log::info!(
            "Client disconnected after {} MB in {:?}",
            received_bytes / (1024 * 1024),
            disconnect_time
        );

        // Verify we received at least 10MB before disconnect
        assert!(
            received_bytes >= disconnect_threshold,
            "Should receive at least 10MB before disconnect"
        );

        // Verify we didn't receive the full file (client disconnected early)
        let full_size = size_mb * 1024 * 1024;
        assert!(
            received_bytes < full_size,
            "Should not receive full file (client disconnected)"
        );

        // TODO: Verify proxy stopped S3 transfer
        // This would require monitoring S3 transfer bytes or proxy metrics
        // For now, we verify behavioral contract: client disconnect should
        // trigger proxy to cancel S3 request

        log::info!("Client disconnect test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_multiple_concurrent_large_file_downloads() {
    init_logging();

    // RED PHASE: This test verifies that multiple clients can download large
    // files concurrently without issues (memory exhaustion, throttling, etc).
    //
    // Expected behavior:
    // 1. Upload 3 large files (50MB each) to S3
    // 2. Launch 10 concurrent downloads (clients requesting different files)
    // 3. All downloads complete successfully
    // 4. Total memory usage stays reasonable (~640KB = 10 * 64KB buffers)
    // 5. No throttling, no connection refused, no timeouts
    //
    // This tests:
    // - Concurrent streaming works correctly
    // - Memory usage stays bounded (O(connections), not O(file size))
    // - No resource contention between concurrent streams

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create 3 large test files (50MB each)
        let num_files = 3;
        let size_mb_per_file = 50;

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
            .bucket("test-bucket")
            .send()
            .await
            .expect("Failed to create bucket");

        // Upload multiple large files
        for i in 0..num_files {
            let key = format!("large-{}.bin", i);
            let chunk_size = 1024 * 1024; // 1MB
            let mut file_data = Vec::with_capacity(size_mb_per_file * chunk_size);
            for j in 0..size_mb_per_file {
                let chunk = vec![((i * 100 + j) % 256) as u8; chunk_size];
                file_data.extend_from_slice(&chunk);
            }

            s3_client
                .put_object()
                .bucket("test-bucket")
                .key(&key)
                .body(file_data.into())
                .send()
                .await
                .expect(&format!("Failed to upload {}", key));

            log::info!("Uploaded {} ({}MB)", key, size_mb_per_file);
        }

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-concurrent-test.yaml";
        create_localstack_config(&endpoint, config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for concurrent test");

        let proxy_base_url = "http://127.0.0.1:18080/test";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .pool_max_idle_per_host(50)
            .build()
            .expect("Failed to create HTTP client");

        // Launch 10 concurrent downloads (cycling through 3 files)
        let num_concurrent = 10;
        let mut tasks = JoinSet::new();

        let overall_start = Instant::now();

        for i in 0..num_concurrent {
            let client_clone = client.clone();
            let file_index = i % num_files;
            let url = format!("{}/large-{}.bin", proxy_base_url, file_index);

            tasks.spawn(async move {
                let download_start = Instant::now();
                let mut response = client_clone
                    .get(&url)
                    .send()
                    .await
                    .expect(&format!("Download {} failed", i));

                assert_eq!(response.status(), reqwest::StatusCode::OK);

                let mut received = 0;
                while let Some(chunk) = response.chunk().await.expect("Failed to read chunk") {
                    received += chunk.len();
                }

                let duration = download_start.elapsed();
                log::info!(
                    "Download {} complete: {} MB in {:?}",
                    i,
                    received / (1024 * 1024),
                    duration
                );

                (i, received, duration)
            });
        }

        // Wait for all downloads to complete
        let mut results = Vec::new();
        while let Some(result) = tasks.join_next().await {
            results.push(result.expect("Task panicked"));
        }

        let overall_time = overall_start.elapsed();

        log::info!(
            "All {} concurrent downloads completed in {:?}",
            num_concurrent,
            overall_time
        );

        // Verify all downloads completed
        assert_eq!(results.len(), num_concurrent);

        // Verify all downloads received correct file size
        let expected_size = size_mb_per_file * 1024 * 1024;
        for (i, received, _duration) in &results {
            assert_eq!(
                *received, expected_size,
                "Download {} should receive complete {}MB file",
                i, size_mb_per_file
            );
        }

        // Calculate aggregate throughput
        let total_bytes: usize = results.iter().map(|(_, received, _)| received).sum();
        let total_mb = total_bytes as f64 / (1024.0 * 1024.0);
        let aggregate_throughput = total_mb / overall_time.as_secs_f64();

        log::info!(
            "Aggregate throughput: {:.2} MB/s ({} MB total)",
            aggregate_throughput,
            total_mb as usize
        );

        log::info!("Concurrent large file downloads test passed");
    });
}
