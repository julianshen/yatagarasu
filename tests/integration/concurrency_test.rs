// Concurrent Request Handling Integration Tests
// Phase 20: Extended Integration Tests - Concurrency
//
// Tests that the proxy correctly handles concurrent requests:
// - Many simultaneous requests all succeed
// - No race conditions in routing, auth, or S3 client usage
// - Connection pooling works correctly
// - Memory usage stays constant (no leaks)

use std::sync::Once;
use std::time::Duration;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;
use tokio::task::JoinSet;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
}

// Helper: Setup LocalStack with test files
async fn setup_localstack_with_test_files<'a>(
    docker: &'a Cli,
    bucket_name: &str,
    num_files: usize,
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

    // Upload multiple test files
    for i in 0..num_files {
        let key = format!("file-{}.txt", i);
        let content = format!("Content of file {}", i);
        s3_client
            .put_object()
            .bucket(bucket_name)
            .key(&key)
            .body(content.as_bytes().to_vec().into())
            .send()
            .await
            .expect(&format!("Failed to upload file: {}", key));
    }

    log::info!(
        "Created LocalStack bucket: {} with {} test files",
        bucket_name,
        num_files
    );

    (container, endpoint)
}

#[test]
#[ignore] // Requires Docker and running proxy - run with: cargo test -- --ignored
fn test_100_concurrent_requests_all_succeed() {
    init_logging();

    // RED PHASE: This test verifies that the proxy can handle many concurrent
    // requests without failures, race conditions, or resource exhaustion.
    //
    // Expected behavior:
    // 1. Create 100 test files in S3
    // 2. Send 100 concurrent GET requests to proxy (different files)
    // 3. All requests succeed with 200 OK
    // 4. All responses contain correct file content
    // 5. No errors, no timeouts, no connection refused
    //
    // This tests:
    // - Concurrent routing (no race conditions in path matching)
    // - Concurrent S3 client usage (proper connection pooling)
    // - HTTP server concurrency (async/await handling)
    // - Memory management under load (no leaks)

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let num_files = 100;
        let (_container, s3_endpoint) =
            setup_localstack_with_test_files(&docker, "test-bucket", num_files).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // TODO: Start Yatagarasu proxy server here

        let proxy_base_url = "http://127.0.0.1:18080/test";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(100) // Support many concurrent connections
            .build()
            .expect("Failed to create HTTP client");

        // Launch 100 concurrent requests
        let mut tasks = JoinSet::new();
        for i in 0..num_files {
            let client_clone = client.clone();
            let url = format!("{}/file-{}.txt", proxy_base_url, i);
            let expected_content = format!("Content of file {}", i);

            tasks.spawn(async move {
                let response = client_clone
                    .get(&url)
                    .send()
                    .await
                    .expect(&format!("Failed to send request for file-{}.txt", i));

                // Verify status code
                assert_eq!(
                    response.status(),
                    reqwest::StatusCode::OK,
                    "Request for file-{}.txt should return 200 OK",
                    i
                );

                // Verify response body
                let body = response
                    .text()
                    .await
                    .expect(&format!("Failed to read response body for file-{}.txt", i));

                assert_eq!(
                    body, expected_content,
                    "Content mismatch for file-{}.txt",
                    i
                );

                log::debug!("Request {} completed successfully", i);
                i // Return file index for verification
            });
        }

        // Wait for all tasks to complete
        let start = std::time::Instant::now();
        let mut completed = Vec::new();
        while let Some(result) = tasks.join_next().await {
            completed.push(result.expect("Task panicked"));
        }
        let duration = start.elapsed();

        log::info!(
            "All {} concurrent requests completed in {:?} ({:.2} req/s)",
            num_files,
            duration,
            num_files as f64 / duration.as_secs_f64()
        );

        // Verify all requests completed
        assert_eq!(
            completed.len(),
            num_files,
            "All {} requests should complete",
            num_files
        );

        // Verify no duplicates (each file requested exactly once)
        completed.sort();
        for i in 0..num_files {
            assert_eq!(
                completed[i], i,
                "File {} should be requested exactly once",
                i
            );
        }

        log::info!("100 concurrent requests test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_no_race_conditions_in_request_handling() {
    init_logging();

    // RED PHASE: This test verifies that concurrent requests to the SAME file
    // don't cause race conditions or incorrect responses.
    //
    // Expected behavior:
    // 1. Upload single test file to S3
    // 2. Send 100 concurrent GET requests for the SAME file
    // 3. All requests succeed with 200 OK
    // 4. All responses contain identical correct content
    // 5. No garbled data, no mixed responses, no errors
    //
    // This tests:
    // - S3 client connection pooling (reuse connections safely)
    // - Response streaming isolation (no cross-contamination)
    // - Request context isolation (each request independent)

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_test_files(&docker, "test-bucket", 1).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // TODO: Start Yatagarasu proxy server here

        let proxy_url = "http://127.0.0.1:18080/test/file-0.txt";
        let expected_content = "Content of file 0";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(100)
            .build()
            .expect("Failed to create HTTP client");

        // Launch 100 concurrent requests to the SAME file
        let num_requests = 100;
        let mut tasks = JoinSet::new();

        for i in 0..num_requests {
            let client_clone = client.clone();
            let url = proxy_url.to_string();
            let expected = expected_content.to_string();

            tasks.spawn(async move {
                let response = client_clone
                    .get(&url)
                    .send()
                    .await
                    .expect(&format!("Request {} failed", i));

                assert_eq!(
                    response.status(),
                    reqwest::StatusCode::OK,
                    "Request {} should return 200 OK",
                    i
                );

                let body = response
                    .text()
                    .await
                    .expect(&format!("Failed to read body for request {}", i));

                assert_eq!(
                    body, expected,
                    "Content mismatch for request {} (possible race condition)",
                    i
                );

                log::debug!("Concurrent request {} completed successfully", i);
            });
        }

        // Wait for all tasks to complete
        let start = std::time::Instant::now();
        let mut completed = 0;
        while let Some(result) = tasks.join_next().await {
            result.expect("Task panicked");
            completed += 1;
        }
        let duration = start.elapsed();

        log::info!(
            "All {} concurrent requests to same file completed in {:?} ({:.2} req/s)",
            num_requests,
            duration,
            num_requests as f64 / duration.as_secs_f64()
        );

        assert_eq!(
            completed, num_requests,
            "All {} requests should complete",
            num_requests
        );

        log::info!("Race condition test passed: all responses identical and correct");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_connection_pooling_works_correctly() {
    init_logging();

    // RED PHASE: This test verifies that S3 connection pooling works correctly:
    // connections are reused efficiently, reducing overhead.
    //
    // Expected behavior:
    // 1. Upload test file to S3
    // 2. Send 1000 sequential requests (not concurrent)
    // 3. All requests succeed with 200 OK
    // 4. Requests complete quickly (connection reuse, not new connection each time)
    // 5. Measure throughput - should be high if pooling works
    //
    // Without connection pooling: Each request pays TCP handshake + TLS cost (~50-100ms)
    // With connection pooling: Connections reused, latency ~10ms per request
    //
    // This tests:
    // - S3 client connection pooling configuration
    // - Connection reuse across requests
    // - No connection leak (connections returned to pool)

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let (_container, s3_endpoint) =
            setup_localstack_with_test_files(&docker, "test-bucket", 1).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // TODO: Start Yatagarasu proxy server here

        let proxy_url = "http://127.0.0.1:18080/test/file-0.txt";
        let expected_content = "Content of file 0";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(10) // Connection pool
            .build()
            .expect("Failed to create HTTP client");

        // Send 1000 sequential requests
        let num_requests = 1000;
        let start = std::time::Instant::now();

        for i in 0..num_requests {
            let response = client
                .get(proxy_url)
                .send()
                .await
                .expect(&format!("Request {} failed", i));

            assert_eq!(
                response.status(),
                reqwest::StatusCode::OK,
                "Request {} should return 200 OK",
                i
            );

            let body = response
                .text()
                .await
                .expect(&format!("Failed to read body for request {}", i));

            assert_eq!(body, expected_content, "Content mismatch for request {}", i);

            if (i + 1) % 100 == 0 {
                log::debug!("Completed {} requests", i + 1);
            }
        }

        let duration = start.elapsed();
        let avg_latency = duration.as_millis() as f64 / num_requests as f64;
        let throughput = num_requests as f64 / duration.as_secs_f64();

        log::info!(
            "Completed {} sequential requests in {:?} (avg latency: {:.2}ms, throughput: {:.2} req/s)",
            num_requests,
            duration,
            avg_latency,
            throughput
        );

        // If connection pooling works, avg latency should be low (<50ms)
        // Without pooling, each request would take 50-100ms (new TCP connection)
        assert!(
            avg_latency < 50.0,
            "Average latency should be <50ms with connection pooling (got {:.2}ms)",
            avg_latency
        );

        log::info!("Connection pooling test passed");
    });
}

#[test]
#[ignore] // Requires Docker and running proxy
fn test_memory_usage_stays_constant() {
    init_logging();

    // RED PHASE: This test verifies that memory usage stays constant under
    // sustained load, proving there are no memory leaks.
    //
    // Expected behavior:
    // 1. Baseline memory measurement before load
    // 2. Send 10,000 requests (mix of different files)
    // 3. Memory may increase initially (connection pool, caches)
    // 4. Memory stabilizes and stays constant (no unbounded growth)
    // 5. After requests complete, memory returns to baseline (no leaks)
    //
    // This tests:
    // - No memory leaks in request handling
    // - Response bodies properly freed after streaming
    // - Connection pool bounded (doesn't grow indefinitely)
    // - Request context cleanup (tracing spans, buffers)
    //
    // Note: This is a behavioral test. Real memory leak detection requires
    // tools like valgrind, heaptrack, or Rust's LeakSanitizer.

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let num_files = 10;
        let (_container, s3_endpoint) =
            setup_localstack_with_test_files(&docker, "test-bucket", num_files).await;

        log::info!("LocalStack S3 endpoint: {}", s3_endpoint);

        // TODO: Start Yatagarasu proxy server here
        // TODO: Get proxy process PID for memory monitoring

        let proxy_base_url = "http://127.0.0.1:18080/test";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(50)
            .build()
            .expect("Failed to create HTTP client");

        // Baseline memory measurement
        // In real implementation, use `ps` or `/proc/<pid>/status` to read RSS
        log::info!("TODO: Measure baseline memory usage of proxy process");

        // Send 10,000 requests (cycling through files)
        let num_requests = 10_000;
        let start = std::time::Instant::now();

        for i in 0..num_requests {
            let file_index = i % num_files;
            let url = format!("{}/file-{}.txt", proxy_base_url, file_index);
            let expected_content = format!("Content of file {}", file_index);

            let response = client
                .get(&url)
                .send()
                .await
                .expect(&format!("Request {} failed", i));

            assert_eq!(response.status(), reqwest::StatusCode::OK);

            let body = response.text().await.expect("Failed to read body");
            assert_eq!(body, expected_content);

            // Sample memory usage every 1000 requests
            if (i + 1) % 1000 == 0 {
                log::info!("Completed {} requests (TODO: measure memory)", i + 1);
                // TODO: Measure current memory usage
                // Assert memory hasn't grown significantly from baseline
            }
        }

        let duration = start.elapsed();
        let throughput = num_requests as f64 / duration.as_secs_f64();

        log::info!(
            "Completed {} requests in {:?} ({:.2} req/s)",
            num_requests,
            duration,
            throughput
        );

        // TODO: Final memory measurement
        // Assert memory returned to baseline (within reasonable margin)
        log::info!("TODO: Verify memory returned to baseline");

        log::info!("Memory stability test passed (manual memory verification required)");
    });
}
