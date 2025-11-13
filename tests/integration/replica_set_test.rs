// Integration tests for replica set failover
// Phase 23: HA Bucket Replication
//
// These tests verify end-to-end failover behavior with real S3 backends (LocalStack).
// Tests require Docker to run LocalStack containers.
//
// NOTE: ReplicaSet module (src/replica_set/mod.rs) is fully integrated with the proxy.
// The proxy now automatically selects healthy replicas for each request based on
// circuit breaker state. These tests verify end-to-end failover behavior.

use super::test_harness::ProxyTestHarness;
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

/// Test: Failover to replica when primary S3 unavailable
///
/// Scenario:
/// 1. Configure bucket with 2 replicas (primary at priority 1, backup at priority 2)
/// 2. Primary replica points to unreachable endpoint (simulates outage)
/// 3. Backup replica points to working LocalStack S3
/// 4. Upload test file to backup S3 bucket
/// 5. Make GET request through proxy
/// 6. Verify: Request fails over to backup replica and succeeds with correct content
#[test]
#[ignore] // Requires Docker - run with: cargo test --test replica_set_test -- --ignored
fn test_failover_to_replica_when_primary_unavailable() {
    init_logging();

    // Create Docker client and start LocalStack
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 endpoint: {}", endpoint);

    // Setup: Create S3 bucket and upload test file to backup replica
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create backup bucket
        s3_client
            .create_bucket()
            .bucket("backup-bucket")
            .send()
            .await
            .expect("Failed to create backup bucket");

        // Upload test file to backup bucket
        let test_content = "Hello from backup replica!";
        s3_client
            .put_object()
            .bucket("backup-bucket")
            .key("test-file.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload test file");

        log::info!("Uploaded test file to backup S3 bucket");
    });

    // Create config file with replica set
    // Primary replica points to unreachable endpoint (port 9999)
    // Backup replica points to working LocalStack
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1:18090"
  threads: 2

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      # Legacy fields kept for backward compatibility
      bucket: "backup-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
      endpoint: "{endpoint}"

      # New replica set configuration (Phase 23)
      replicas:
        - name: "primary"
          bucket: "primary-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "http://127.0.0.1:9999"  # Unreachable endpoint
          priority: 1
          timeout: 2

        - name: "backup"
          bucket: "backup-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "{endpoint}"
          priority: 2
          timeout: 5
"#,
        endpoint = endpoint
    );

    // Write config to temp file
    let config_path = "/tmp/replica_failover_test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config file");

    log::info!("Created proxy config at {}", config_path);

    // Start proxy with test harness
    let proxy_port = 18090;
    let _proxy = ProxyTestHarness::start(config_path, proxy_port)
        .expect("Failed to start proxy for replica failover test");

    // Make GET request through proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/test/test-file.txt", proxy_port);
    log::info!("Requesting: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to send request");

    log::info!("Response status: {}", response.status());

    // Verify: Request should succeed with failover to backup replica
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Request should succeed after failover to backup replica"
    );

    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, "Hello from backup replica!",
        "Response body should match backup replica content"
    );

    log::info!("✅ Failover successful: Primary failed, backup served request");
}

/// Test: Skip unhealthy replicas during failover
///
/// Scenario:
/// 1. Configure bucket with 3 replicas (primary at priority 1, backup1 at priority 2, backup2 at priority 3)
/// 2. Primary and backup1 replicas point to unreachable endpoints (simulate outages)
/// 3. Backup2 replica points to working LocalStack S3
/// 4. Upload test file to backup2 S3 bucket
/// 5. Make GET request through proxy
/// 6. Verify: Request skips unhealthy replicas and succeeds with backup2
#[test]
#[ignore] // Requires Docker - run with: cargo test --test replica_set_test -- --ignored
fn test_skip_unhealthy_replicas_during_failover() {
    init_logging();

    // Create Docker client and start LocalStack
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 endpoint: {}", endpoint);

    // Setup: Create S3 bucket and upload test file to backup2 replica
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create backup2 bucket
        s3_client
            .create_bucket()
            .bucket("backup2-bucket")
            .send()
            .await
            .expect("Failed to create backup2 bucket");

        // Upload test file to backup2 bucket
        let test_content = "Hello from backup2 replica!";
        s3_client
            .put_object()
            .bucket("backup2-bucket")
            .key("test-file.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload test file");

        log::info!("Uploaded test file to backup2 S3 bucket");
    });

    // Create config file with 3 replicas
    // Primary and backup1 point to unreachable endpoints
    // Backup2 points to working LocalStack
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1:18091"
  threads: 2

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      # Legacy fields kept for backward compatibility
      bucket: "backup2-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
      endpoint: "{endpoint}"

      # New replica set configuration (Phase 23)
      replicas:
        - name: "primary"
          bucket: "primary-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "http://127.0.0.1:9999"  # Unreachable endpoint
          priority: 1
          timeout: 2

        - name: "backup1"
          bucket: "backup1-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "http://127.0.0.1:9998"  # Unreachable endpoint
          priority: 2
          timeout: 2

        - name: "backup2"
          bucket: "backup2-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "{endpoint}"
          priority: 3
          timeout: 5
"#,
        endpoint = endpoint
    );

    // Write config to temp file
    let config_path = "/tmp/replica_skip_unhealthy_test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config file");

    log::info!("Created proxy config at {}", config_path);

    // Start proxy with test harness
    let proxy_port = 18091;
    let _proxy = ProxyTestHarness::start(config_path, proxy_port)
        .expect("Failed to start proxy for replica skip unhealthy test");

    // Make GET request through proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/test/test-file.txt", proxy_port);
    log::info!("Requesting: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to send request");

    log::info!("Response status: {}", response.status());

    // Verify: Request should succeed after skipping unhealthy replicas
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Request should succeed after skipping primary and backup1, using backup2"
    );

    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, "Hello from backup2 replica!",
        "Response body should match backup2 replica content"
    );

    log::info!(
        "✅ Failover successful: Skipped unhealthy primary and backup1, backup2 served request"
    );
}

/// Test: Return 502 when all replicas fail
///
/// Scenario:
/// 1. Configure bucket with 2 replicas
/// 2. Both replicas point to unreachable endpoints (simulate all outages)
/// 3. Make GET request through proxy
/// 4. Verify: Request returns 502 Bad Gateway (all replicas unavailable)
#[test]
#[ignore] // Requires Docker - run with: cargo test --test replica_set_test -- --ignored
fn test_return_502_when_all_replicas_fail() {
    init_logging();

    // Create config file with all replicas pointing to unreachable endpoints
    let config_yaml = r#"
server:
  address: "127.0.0.1:18092"
  threads: 2

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      # Legacy fields kept for backward compatibility
      bucket: "backup-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
      endpoint: "http://127.0.0.1:9999"

      # New replica set configuration (Phase 23)
      replicas:
        - name: "primary"
          bucket: "primary-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "http://127.0.0.1:9999"  # Unreachable endpoint
          priority: 1
          timeout: 2

        - name: "backup"
          bucket: "backup-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "http://127.0.0.1:9998"  # Unreachable endpoint
          priority: 2
          timeout: 2
"#;

    // Write config to temp file
    let config_path = "/tmp/replica_all_fail_test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config file");

    log::info!("Created proxy config at {}", config_path);

    // Start proxy with test harness
    let proxy_port = 18092;
    let _proxy = ProxyTestHarness::start(config_path, proxy_port)
        .expect("Failed to start proxy for replica all fail test");

    // Make GET request through proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/test/test-file.txt", proxy_port);
    log::info!("Requesting: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to send request");

    log::info!("Response status: {}", response.status());

    // Verify: Request should fail with 502 Bad Gateway
    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_GATEWAY,
        "Request should return 502 Bad Gateway when all replicas are unavailable"
    );

    log::info!("✅ Correctly returned 502 when all replicas failed");
}

/// Test: No failover on 404 (return to client immediately)
///
/// Scenario:
/// 1. Configure bucket with 2 replicas (primary and backup)
/// 2. Both replicas point to working LocalStack S3
/// 3. Upload test file ONLY to primary bucket
/// 4. Make GET request for NON-EXISTENT file through proxy
/// 5. Verify: Request returns 404 immediately without trying backup replica
#[test]
#[ignore] // Requires Docker - run with: cargo test --test replica_set_test -- --ignored
fn test_no_failover_on_404() {
    init_logging();

    // Create Docker client and start LocalStack
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 endpoint: {}", endpoint);

    // Setup: Create S3 buckets but DON'T upload the requested file
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create primary bucket
        s3_client
            .create_bucket()
            .bucket("primary-bucket")
            .send()
            .await
            .expect("Failed to create primary bucket");

        // Create backup bucket
        s3_client
            .create_bucket()
            .bucket("backup-bucket")
            .send()
            .await
            .expect("Failed to create backup bucket");

        // Upload a different file (not the one we'll request)
        s3_client
            .put_object()
            .bucket("primary-bucket")
            .key("other-file.txt")
            .body("Other content".as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload other file");

        log::info!("Created S3 buckets (without requested file)");
    });

    // Create config file with replica set
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1:18093"
  threads: 2

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      # Legacy fields kept for backward compatibility
      bucket: "primary-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
      endpoint: "{endpoint}"

      # New replica set configuration (Phase 23)
      replicas:
        - name: "primary"
          bucket: "primary-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "{endpoint}"
          priority: 1
          timeout: 5

        - name: "backup"
          bucket: "backup-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "{endpoint}"
          priority: 2
          timeout: 5
"#,
        endpoint = endpoint
    );

    // Write config to temp file
    let config_path = "/tmp/replica_404_test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config file");

    log::info!("Created proxy config at {}", config_path);

    // Start proxy with test harness
    let proxy_port = 18093;
    let _proxy = ProxyTestHarness::start(config_path, proxy_port)
        .expect("Failed to start proxy for replica 404 test");

    // Make GET request for non-existent file through proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/test/non-existent-file.txt", proxy_port);
    log::info!("Requesting: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to send request");

    log::info!("Response status: {}", response.status());

    // Verify: Request should return 404 immediately
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "Request should return 404 for non-existent file without failover"
    );

    log::info!("✅ Correctly returned 404 without failover");
}

/// Test: Backward compatibility - single bucket config works
///
/// Scenario:
/// 1. Configure bucket WITHOUT replicas (legacy single bucket config)
/// 2. Upload test file to S3 bucket
/// 3. Make GET request through proxy
/// 4. Verify: Request succeeds with legacy configuration
#[test]
#[ignore] // Requires Docker - run with: cargo test --test replica_set_test -- --ignored
fn test_backward_compatibility_single_bucket() {
    init_logging();

    // Create Docker client and start LocalStack
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 endpoint: {}", endpoint);

    // Setup: Create S3 bucket and upload test file
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
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
            .bucket("legacy-bucket")
            .send()
            .await
            .expect("Failed to create legacy bucket");

        // Upload test file
        let test_content = "Hello from legacy config!";
        s3_client
            .put_object()
            .bucket("legacy-bucket")
            .key("test-file.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload test file");

        log::info!("Uploaded test file to legacy S3 bucket");
    });

    // Create config file WITHOUT replicas (legacy format)
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1:18094"
  threads: 2

buckets:
  - name: "legacy-bucket"
    path_prefix: "/legacy"
    s3:
      bucket: "legacy-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
      endpoint: "{endpoint}"
"#,
        endpoint = endpoint
    );

    // Write config to temp file
    let config_path = "/tmp/replica_legacy_test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config file");

    log::info!("Created proxy config at {}", config_path);

    // Start proxy with test harness
    let proxy_port = 18094;
    let _proxy = ProxyTestHarness::start(config_path, proxy_port)
        .expect("Failed to start proxy for legacy config test");

    // Make GET request through proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/legacy/test-file.txt", proxy_port);
    log::info!("Requesting: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to send request");

    log::info!("Response status: {}", response.status());

    // Verify: Request should succeed with legacy config
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Request should succeed with legacy single bucket configuration"
    );

    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, "Hello from legacy config!",
        "Response body should match legacy bucket content"
    );

    log::info!("✅ Legacy single bucket configuration works correctly");
}

/// Test: Metrics track replica usage and failover
///
/// Scenario:
/// 1. Configure bucket with 2 replicas (primary and backup)
/// 2. Both replicas point to working LocalStack S3
/// 3. Upload test files to both S3 buckets
/// 4. Make multiple GET requests through proxy
/// 5. Verify: Metrics endpoint shows per-replica request counts and active replica
#[test]
#[ignore] // Requires Docker - run with: cargo test --test replica_set_test -- --ignored
fn test_metrics_track_replica_usage() {
    init_logging();

    // Create Docker client and start LocalStack
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 endpoint: {}", endpoint);

    // Setup: Create S3 buckets and upload test files
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create primary bucket
        s3_client
            .create_bucket()
            .bucket("primary-bucket")
            .send()
            .await
            .expect("Failed to create primary bucket");

        // Upload test file to primary bucket
        let test_content = "Hello from primary!";
        s3_client
            .put_object()
            .bucket("primary-bucket")
            .key("test-file.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload test file to primary");

        log::info!("Uploaded test file to primary S3 bucket");
    });

    // Create config file with replica set
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1:18095"
  threads: 2
  metrics_address: "127.0.0.1:19095"

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      # Legacy fields kept for backward compatibility
      bucket: "primary-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
      endpoint: "{endpoint}"

      # New replica set configuration (Phase 23)
      replicas:
        - name: "primary"
          bucket: "primary-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "{endpoint}"
          priority: 1
          timeout: 5

        - name: "backup"
          bucket: "backup-bucket"
          region: "us-east-1"
          access_key: "test"
          secret_key: "test"
          endpoint: "http://127.0.0.1:9999"  # Unreachable (for testing failover metrics)
          priority: 2
          timeout: 2
"#,
        endpoint = endpoint
    );

    // Write config to temp file
    let config_path = "/tmp/replica_metrics_test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config file");

    log::info!("Created proxy config at {}", config_path);

    // Start proxy with test harness
    let proxy_port = 18095;
    let _proxy = ProxyTestHarness::start(config_path, proxy_port)
        .expect("Failed to start proxy for replica metrics test");

    // Make multiple GET requests through proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/test/test-file.txt", proxy_port);

    // Make 5 requests to generate metrics
    for i in 1..=5 {
        log::info!("Request {}: {}", i, proxy_url);
        let response = client
            .get(&proxy_url)
            .send()
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            reqwest::StatusCode::OK,
            "Request {} should succeed",
            i
        );
    }

    // Query metrics endpoint
    std::thread::sleep(Duration::from_millis(500)); // Allow metrics to be recorded
    let metrics_url = "http://127.0.0.1:19095/metrics";
    log::info!("Querying metrics: {}", metrics_url);

    let metrics_response = client
        .get(metrics_url)
        .send()
        .expect("Failed to query metrics endpoint");

    assert_eq!(
        metrics_response.status(),
        reqwest::StatusCode::OK,
        "Metrics endpoint should be accessible"
    );

    let metrics_body = metrics_response
        .text()
        .expect("Failed to read metrics response");

    log::info!(
        "Metrics response (excerpt):\n{}",
        &metrics_body[..500.min(metrics_body.len())]
    );

    // Verify: Metrics should include per-replica request counts
    // Format: yatagarasu_replica_requests_total{bucket="test-bucket",replica="primary"} N
    assert!(
        metrics_body.contains("yatagarasu_replica_requests_total"),
        "Metrics should include per-replica request counts"
    );

    assert!(
        metrics_body.contains("bucket=\"test-bucket\""),
        "Metrics should include bucket label"
    );

    assert!(
        metrics_body.contains("replica=\"primary\""),
        "Metrics should include replica label for primary"
    );

    // Verify: Active replica should be tracked
    // Format: yatagarasu_active_replica{bucket="test-bucket",replica="primary"} 1
    assert!(
        metrics_body.contains("yatagarasu_active_replica"),
        "Metrics should include active replica gauge"
    );

    log::info!("✅ Metrics successfully track replica usage (request counts and active replica)");
}
