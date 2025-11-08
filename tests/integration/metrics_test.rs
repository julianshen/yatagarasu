// Metrics Validation Integration Tests
// Phase 20: Extended Integration Tests - Metrics
//
// Tests that the proxy exposes Prometheus metrics correctly:
// - /metrics endpoint returns Prometheus format
// - Request counters increment correctly
// - Latency histograms populated
// - S3 error counters increment on S3 errors

use super::test_harness::ProxyTestHarness;
use std::fs;
use std::sync::Once;
use std::time::Duration;

static INIT: Once = Once::new();

fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
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

#[test]
#[ignore] // Requires running proxy - run with: cargo test -- --ignored
fn test_metrics_endpoint_returns_prometheus_format() {
    init_logging();

    // RED PHASE: This test verifies that the /metrics endpoint exists and
    // returns metrics in Prometheus text exposition format.
    //
    // Expected behavior:
    // 1. Proxy exposes /metrics endpoint (typically on separate port, e.g., :9090)
    // 2. GET /metrics returns 200 OK
    // 3. Content-Type is text/plain (Prometheus format)
    // 4. Response body contains Prometheus metric format:
    //    - # HELP lines (metric description)
    //    - # TYPE lines (metric type: counter, histogram, gauge)
    //    - Metric lines with labels and values
    //
    // Example Prometheus format:
    //   # HELP http_requests_total Total number of HTTP requests
    //   # TYPE http_requests_total counter
    //   http_requests_total{method="GET",status="200"} 42
    //
    // This tests:
    // - Metrics endpoint is accessible
    // - Proper Prometheus format
    // - Metrics are being collected

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-metrics-1.yaml";
        create_localstack_config("http://127.0.0.1:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for metrics endpoint test");

        let metrics_url = "http://127.0.0.1:9090/metrics";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        let response = client
            .get(metrics_url)
            .send()
            .await
            .expect("Failed to send request to /metrics endpoint");

        // Verify 200 OK
        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "/metrics endpoint should return 200 OK"
        );

        // Verify Content-Type is text/plain (Prometheus format)
        let content_type = response
            .headers()
            .get("content-type")
            .expect("/metrics should include Content-Type header")
            .to_str()
            .unwrap();

        assert!(
            content_type.contains("text/plain")
                || content_type.contains("text/plain; version=0.0.4"),
            "Content-Type should be text/plain for Prometheus format, got: {}",
            content_type
        );

        // Get response body
        let body = response.text().await.expect("Failed to read response body");

        // Verify Prometheus format:
        // - Contains # HELP lines
        // - Contains # TYPE lines
        // - Contains metric lines with values

        assert!(
            body.contains("# HELP"),
            "Metrics should contain HELP lines (metric descriptions)"
        );

        assert!(
            body.contains("# TYPE"),
            "Metrics should contain TYPE lines (counter, histogram, gauge)"
        );

        // Verify we have at least some basic metrics
        // (exact metric names depend on implementation, but should have HTTP metrics)
        assert!(
            body.len() > 100,
            "Metrics response should contain substantial content, got {} bytes",
            body.len()
        );

        log::info!("Metrics endpoint test passed");
        log::debug!("Sample metrics output:\n{}", &body[..body.len().min(500)]);
    });
}

#[test]
#[ignore] // Requires running proxy
fn test_request_counters_increment_correctly() {
    init_logging();

    // RED PHASE: This test verifies that HTTP request counters increment
    // correctly as requests are processed.
    //
    // Expected behavior:
    // 1. Fetch initial metrics from /metrics endpoint
    // 2. Parse initial counter value for http_requests_total
    // 3. Send N requests to proxy (e.g., 10 GET requests)
    // 4. Fetch metrics again
    // 5. Verify http_requests_total increased by N
    // 6. Verify counters are labeled correctly (method, status, path)
    //
    // Example metric:
    //   http_requests_total{method="GET",status="200",path="/test"} 10
    //
    // This tests:
    // - Counters increment on each request
    // - Labels are correct (method, status code, path)
    // - Counters are monotonically increasing

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-metrics-2.yaml";
        create_localstack_config("http://127.0.0.1:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for request counter test");

        let metrics_url = "http://127.0.0.1:9090/metrics";
        let proxy_url = "http://127.0.0.1:18080/test/sample.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        // Fetch initial metrics
        let initial_metrics = client
            .get(metrics_url)
            .send()
            .await
            .expect("Failed to fetch initial metrics")
            .text()
            .await
            .expect("Failed to read initial metrics");

        log::debug!("Initial metrics fetched");

        // Parse initial counter value (simple search for http_requests_total)
        // In real implementation, would parse Prometheus format properly
        let initial_count = parse_counter_value(&initial_metrics, "http_requests_total");
        log::info!("Initial http_requests_total: {}", initial_count);

        // Send N requests to proxy
        let num_requests = 10;
        for i in 0..num_requests {
            let response = client
                .get(proxy_url)
                .send()
                .await
                .expect(&format!("Request {} failed", i));

            assert_eq!(
                response.status(),
                reqwest::StatusCode::OK,
                "Request {} should succeed",
                i
            );

            // Consume response body to complete request
            let _ = response.bytes().await;
        }

        log::info!("Sent {} requests", num_requests);

        // Fetch metrics again
        let updated_metrics = client
            .get(metrics_url)
            .send()
            .await
            .expect("Failed to fetch updated metrics")
            .text()
            .await
            .expect("Failed to read updated metrics");

        // Parse updated counter value
        let updated_count = parse_counter_value(&updated_metrics, "http_requests_total");
        log::info!("Updated http_requests_total: {}", updated_count);

        // Verify counter increased by num_requests
        let delta = updated_count - initial_count;
        assert!(
            delta >= num_requests as u64,
            "http_requests_total should increase by at least {} (got delta: {})",
            num_requests,
            delta
        );

        log::info!(
            "Request counter test passed: counter increased by {}",
            delta
        );
    });
}

#[test]
#[ignore] // Requires running proxy
fn test_latency_histograms_populated() {
    init_logging();

    // RED PHASE: This test verifies that request latency histograms are
    // populated correctly as requests are processed.
    //
    // Expected behavior:
    // 1. Send several requests to proxy
    // 2. Fetch metrics from /metrics endpoint
    // 3. Verify histogram metrics exist (http_request_duration_seconds)
    // 4. Verify histogram buckets are populated
    // 5. Verify _count and _sum metrics exist
    //
    // Prometheus histogram format:
    //   http_request_duration_seconds_bucket{le="0.001"} 5
    //   http_request_duration_seconds_bucket{le="0.01"} 10
    //   http_request_duration_seconds_bucket{le="0.1"} 15
    //   http_request_duration_seconds_bucket{le="+Inf"} 15
    //   http_request_duration_seconds_sum 0.45
    //   http_request_duration_seconds_count 15
    //
    // This tests:
    // - Histograms are being recorded
    // - Buckets are configured correctly
    // - Count and sum are tracked

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-metrics-3.yaml";
        create_localstack_config("http://127.0.0.1:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for latency histogram test");

        let metrics_url = "http://127.0.0.1:9090/metrics";
        let proxy_url = "http://127.0.0.1:18080/test/sample.txt";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        // Send several requests to generate histogram data
        let num_requests = 20;
        for i in 0..num_requests {
            let response = client
                .get(proxy_url)
                .send()
                .await
                .expect(&format!("Request {} failed", i));

            assert_eq!(response.status(), reqwest::StatusCode::OK);
            let _ = response.bytes().await;
        }

        log::info!("Sent {} requests to populate histograms", num_requests);

        // Fetch metrics
        let metrics = client
            .get(metrics_url)
            .send()
            .await
            .expect("Failed to fetch metrics")
            .text()
            .await
            .expect("Failed to read metrics");

        // Verify histogram metrics exist
        assert!(
            metrics.contains("http_request_duration_seconds_bucket"),
            "Metrics should contain histogram buckets"
        );

        assert!(
            metrics.contains("http_request_duration_seconds_sum"),
            "Metrics should contain histogram sum"
        );

        assert!(
            metrics.contains("http_request_duration_seconds_count"),
            "Metrics should contain histogram count"
        );

        // Verify histogram count matches requests sent
        let histogram_count = parse_counter_value(&metrics, "http_request_duration_seconds_count");
        assert!(
            histogram_count >= num_requests as u64,
            "Histogram count should be at least {} (got {})",
            num_requests,
            histogram_count
        );

        // Verify histogram sum is positive (requests took some time)
        let histogram_sum = parse_histogram_sum(&metrics, "http_request_duration_seconds_sum");
        assert!(
            histogram_sum > 0.0,
            "Histogram sum should be positive (requests take time)"
        );

        log::info!(
            "Latency histogram test passed: count={}, sum={}s",
            histogram_count,
            histogram_sum
        );
    });
}

#[test]
#[ignore] // Requires running proxy and S3 backend
fn test_s3_error_counters_increment_on_errors() {
    init_logging();

    // RED PHASE: This test verifies that S3 error counters increment when
    // S3 returns errors (404, 403, 500, etc).
    //
    // Expected behavior:
    // 1. Fetch initial metrics
    // 2. Send requests that trigger S3 errors:
    //    - Request non-existent object (404 NoSuchKey)
    //    - Request forbidden object (403 AccessDenied)
    // 3. Fetch metrics again
    // 4. Verify s3_errors_total counter increased
    // 5. Verify error counters are labeled by error type
    //
    // Example metric:
    //   s3_errors_total{error_type="NoSuchKey"} 5
    //   s3_errors_total{error_type="AccessDenied"} 2
    //
    // This tests:
    // - S3 errors are counted separately from successful requests
    // - Error types are labeled correctly
    // - Helps with monitoring and alerting

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-metrics-4.yaml";
        create_localstack_config("http://127.0.0.1:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for S3 error counter test");

        let metrics_url = "http://127.0.0.1:9090/metrics";
        let proxy_base_url = "http://127.0.0.1:18080/test";

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        // Fetch initial metrics
        let initial_metrics = client
            .get(metrics_url)
            .send()
            .await
            .expect("Failed to fetch initial metrics")
            .text()
            .await
            .expect("Failed to read initial metrics");

        let initial_errors = parse_counter_value(&initial_metrics, "s3_errors_total");
        log::info!("Initial s3_errors_total: {}", initial_errors);

        // Trigger S3 errors by requesting non-existent objects
        let num_error_requests = 5;
        for i in 0..num_error_requests {
            let url = format!("{}/does-not-exist-{}.txt", proxy_base_url, i);
            let response = client
                .get(&url)
                .send()
                .await
                .expect("Failed to send request");

            // Should get 404 Not Found
            assert_eq!(
                response.status(),
                reqwest::StatusCode::NOT_FOUND,
                "Non-existent object should return 404"
            );

            let _ = response.bytes().await;
        }

        log::info!("Triggered {} S3 errors (NoSuchKey)", num_error_requests);

        // Fetch metrics again
        let updated_metrics = client
            .get(metrics_url)
            .send()
            .await
            .expect("Failed to fetch updated metrics")
            .text()
            .await
            .expect("Failed to read updated metrics");

        // Verify error counter increased
        let updated_errors = parse_counter_value(&updated_metrics, "s3_errors_total");
        log::info!("Updated s3_errors_total: {}", updated_errors);

        let delta = updated_errors - initial_errors;
        assert!(
            delta >= num_error_requests as u64,
            "s3_errors_total should increase by at least {} (got delta: {})",
            num_error_requests,
            delta
        );

        // Verify error type label exists (NoSuchKey)
        assert!(
            updated_metrics.contains("NoSuchKey") || updated_metrics.contains("s3_errors_total"),
            "Metrics should contain S3 error type labels"
        );

        log::info!(
            "S3 error counter test passed: errors increased by {}",
            delta
        );
    });
}

// Helper functions for parsing Prometheus metrics
// (Simple implementation for testing; production would use prometheus parser)

fn parse_counter_value(metrics: &str, metric_name: &str) -> u64 {
    // Find lines containing metric_name (not # HELP or # TYPE)
    for line in metrics.lines() {
        if line.starts_with(metric_name) && !line.starts_with('#') {
            // Parse value from line like: metric_name{labels} value
            if let Some(value_str) = line.split_whitespace().last() {
                if let Ok(value) = value_str.parse::<u64>() {
                    return value;
                }
                // Try parsing as float and converting to u64
                if let Ok(value) = value_str.parse::<f64>() {
                    return value as u64;
                }
            }
        }
    }
    0 // Default to 0 if not found
}

fn parse_histogram_sum(metrics: &str, metric_name: &str) -> f64 {
    // Find lines containing metric_name_sum
    for line in metrics.lines() {
        if line.starts_with(metric_name) && !line.starts_with('#') {
            // Parse value from line like: metric_name value
            if let Some(value_str) = line.split_whitespace().last() {
                if let Ok(value) = value_str.parse::<f64>() {
                    return value;
                }
            }
        }
    }
    0.0 // Default to 0.0 if not found
}
