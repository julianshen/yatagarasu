// Integration tests for replica set failover
// Phase 23: HA Bucket Replication
//
// These tests verify end-to-end failover behavior with real S3 backends (LocalStack).
// Tests require Docker to run LocalStack containers.
//
// NOTE: ReplicaSet module exists (src/replica_set/mod.rs) with comprehensive failover logic,
// but proxy integration for request-level failover is not yet implemented.
// These tests serve as specifications for the integration work.

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
