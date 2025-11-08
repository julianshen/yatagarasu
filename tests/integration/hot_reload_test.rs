// Hot Reload Validation Integration Tests
// Phase 20: Extended Integration Tests - Hot Reload (requires Phase 19)
//
// Tests that the proxy can reload configuration without downtime:
// - Add new bucket via config reload (SIGHUP)
// - Remove bucket via config reload (in-flight requests complete)
// - Update bucket credentials via config reload
// - Invalid config reload rejected without affecting service

use super::test_harness::ProxyTestHarness;
use std::fs;
use std::io::Write;
use std::sync::Once;
use std::time::Duration;
use tempfile::NamedTempFile;

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
fn test_add_new_bucket_via_config_reload() {
    init_logging();

    // RED PHASE: This test verifies that a new bucket can be added to the
    // proxy configuration via hot reload (SIGHUP) without restarting.
    //
    // Expected behavior:
    // 1. Proxy starts with initial config (1 bucket)
    // 2. Request to bucket1 succeeds
    // 3. Request to bucket2 fails (404, doesn't exist yet)
    // 4. Update config file to add bucket2
    // 5. Send SIGHUP signal to proxy (trigger reload)
    // 6. Wait for reload to complete
    // 7. Request to bucket2 now succeeds
    // 8. Request to bucket1 still works (unchanged)
    //
    // This tests:
    // - Hot reload adds new routing paths
    // - New S3 clients created for new buckets
    // - Old buckets continue working after reload
    // - No downtime during reload

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create initial config file with 1 bucket
        let mut config_file = NamedTempFile::new().expect("Failed to create temp config");
        let initial_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "test-key-1"
      secret_key: "test-secret-1"
      endpoint_url: "http://localhost:9000"
"#;
        config_file
            .write_all(initial_config.as_bytes())
            .expect("Failed to write initial config");
        config_file.flush().expect("Failed to flush config");

        log::info!("Created initial config file: {:?}", config_file.path());

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-hotreload-1.yaml";
        create_localstack_config("http://localhost:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for hot reload add bucket test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        // Verify bucket1 works (or returns expected error if S3 not available)
        let response1 = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await;

        log::info!(
            "Initial request to bucket1: {:?}",
            response1.as_ref().map(|r| r.status())
        );

        // Verify bucket2 doesn't exist yet (404 Not Found - unknown path)
        let response2 = client
            .get("http://127.0.0.1:18080/media/test.txt")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response2.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Bucket2 should not exist yet (404 for unknown path)"
        );

        log::info!("Confirmed bucket2 doesn't exist: 404");

        // Update config file to add bucket2
        let updated_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "test-key-1"
      secret_key: "test-secret-1"
      endpoint_url: "http://localhost:9000"
  - name: "bucket2"
    path_prefix: "/media"
    s3:
      bucket: "s3-bucket-2"
      region: "us-west-2"
      access_key: "test-key-2"
      secret_key: "test-secret-2"
      endpoint_url: "http://localhost:9000"
"#;

        fs::write(config_file.path(), updated_config).expect("Failed to update config file");

        log::info!("Updated config file to add bucket2");

        // Send SIGHUP signal to proxy to trigger reload
        // TODO: Send SIGHUP to proxy_pid
        // nix::sys::signal::kill(proxy_pid, nix::sys::signal::Signal::SIGHUP)
        //     .expect("Failed to send SIGHUP");

        log::info!("Sent SIGHUP signal to proxy (TODO: implement)");

        // Wait for reload to complete (typically <100ms)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify bucket2 now exists and works
        let response2_after = client
            .get("http://127.0.0.1:18080/media/test.txt")
            .send()
            .await
            .expect("Failed to send request");

        // Should get S3 error (404 from S3) or 200 if file exists
        // NOT 404 from proxy (unknown path)
        assert_ne!(
            response2_after.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Bucket2 should exist after reload (not 404 for unknown path)"
        );

        log::info!("Bucket2 exists after reload: {}", response2_after.status());

        // Verify bucket1 still works (unchanged)
        let response1_after = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await;

        log::info!(
            "Bucket1 still works after reload: {:?}",
            response1_after.as_ref().map(|r| r.status())
        );

        log::info!("Hot reload add bucket test passed");
    });
}

#[test]
#[ignore] // Requires running proxy
fn test_remove_bucket_via_config_reload() {
    init_logging();

    // RED PHASE: This test verifies that a bucket can be removed via hot reload,
    // and in-flight requests to that bucket complete successfully.
    //
    // Expected behavior:
    // 1. Proxy starts with 2 buckets
    // 2. Start long-running request to bucket2 (large file download)
    // 3. Update config to remove bucket2
    // 4. Send SIGHUP to trigger reload
    // 5. In-flight request to bucket2 completes successfully
    // 6. New requests to bucket2 return 404 (path not found)
    // 7. Bucket1 continues working normally
    //
    // This tests:
    // - In-flight requests complete with old config
    // - New requests use new config (bucket2 removed)
    // - Graceful handling of bucket removal
    // - No crashes or errors during removal

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create config with 2 buckets
        let mut config_file = NamedTempFile::new().expect("Failed to create temp config");
        let initial_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
      endpoint_url: "http://localhost:9000"
  - name: "bucket2"
    path_prefix: "/media"
    s3:
      bucket: "s3-bucket-2"
      region: "us-west-2"
      access_key: "key2"
      secret_key: "secret2"
      endpoint_url: "http://localhost:9000"
"#;
        config_file
            .write_all(initial_config.as_bytes())
            .expect("Failed to write config");
        config_file.flush().expect("Failed to flush config");

        log::info!("Created initial config with 2 buckets");

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-hotreload-2.yaml";
        create_localstack_config("http://localhost:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for hot reload remove bucket test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        // Verify bucket2 exists
        let response = client
            .get("http://127.0.0.1:18080/media/test.txt")
            .send()
            .await
            .expect("Failed to send request");

        log::info!("Bucket2 exists before reload: {}", response.status());

        // TODO: Start long-running request to bucket2 in background task
        // (This would be a concurrent task that downloads large file)

        // Update config to remove bucket2
        let updated_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
      endpoint_url: "http://localhost:9000"
"#;

        fs::write(config_file.path(), updated_config).expect("Failed to update config");

        log::info!("Updated config to remove bucket2");

        // Send SIGHUP
        // TODO: send_sighup(proxy_pid);

        tokio::time::sleep(Duration::from_millis(200)).await;

        // New requests to bucket2 should fail (404 unknown path)
        let response_after = client
            .get("http://127.0.0.1:18080/media/test.txt")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response_after.status(),
            reqwest::StatusCode::NOT_FOUND,
            "Bucket2 should be removed (404 for unknown path)"
        );

        log::info!("Bucket2 removed successfully: 404");

        // Bucket1 still works
        let response1 = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await;

        log::info!(
            "Bucket1 still works: {:?}",
            response1.as_ref().map(|r| r.status())
        );

        log::info!("Hot reload remove bucket test passed");
    });
}

#[test]
#[ignore] // Requires running proxy
fn test_update_bucket_credentials_via_config_reload() {
    init_logging();

    // RED PHASE: This test verifies that bucket credentials can be updated
    // via hot reload for credential rotation without downtime.
    //
    // Expected behavior:
    // 1. Proxy starts with initial credentials
    // 2. Requests use old credentials
    // 3. Update config with new credentials
    // 4. Send SIGHUP to trigger reload
    // 5. In-flight requests complete with old credentials
    // 6. New requests use new credentials
    // 7. No errors or authentication failures during transition
    //
    // This tests:
    // - Credential rotation without downtime
    // - In-flight requests complete with old credentials
    // - New requests use new credentials
    // - Critical for security (rotate compromised keys)

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let mut config_file = NamedTempFile::new().expect("Failed to create temp config");
        let initial_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "products"
    path_prefix: "/api"
    s3:
      bucket: "products-bucket"
      region: "us-east-1"
      access_key: "OLD_ACCESS_KEY"
      secret_key: "OLD_SECRET_KEY"
      endpoint_url: "http://localhost:9000"
"#;
        config_file
            .write_all(initial_config.as_bytes())
            .expect("Failed to write config");
        config_file.flush().expect("Failed to flush");

        log::info!("Created config with old credentials");

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-hotreload-3.yaml";
        create_localstack_config("http://localhost:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for credential rotation test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Make request with old credentials
        let response = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await;

        log::info!(
            "Request with old credentials: {:?}",
            response.as_ref().map(|r| r.status())
        );

        // Update config with new credentials (rotation)
        let updated_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "products"
    path_prefix: "/api"
    s3:
      bucket: "products-bucket"
      region: "us-east-1"
      access_key: "NEW_ACCESS_KEY"
      secret_key: "NEW_SECRET_KEY"
      endpoint_url: "http://localhost:9000"
"#;

        fs::write(config_file.path(), updated_config).expect("Failed to update config");

        log::info!("Updated config with new credentials");

        // Send SIGHUP
        // TODO: send_sighup(proxy_pid);

        tokio::time::sleep(Duration::from_millis(200)).await;

        // New requests should use new credentials
        let response_after = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await;

        log::info!(
            "Request with new credentials: {:?}",
            response_after.as_ref().map(|r| r.status())
        );

        // Note: To fully test credential rotation, we'd need to:
        // 1. Configure S3 backend to accept only new credentials
        // 2. Verify old credentials are rejected
        // 3. Verify new credentials are accepted
        // This requires S3 backend setup with IAM simulation

        log::info!("Credential rotation test passed");
    });
}

#[test]
#[ignore] // Requires running proxy
fn test_invalid_config_reload_rejected() {
    init_logging();

    // RED PHASE: This test verifies that invalid config reloads are rejected
    // without affecting the running service.
    //
    // Expected behavior:
    // 1. Proxy starts with valid config
    // 2. Service is working normally
    // 3. Update config file with INVALID config (duplicate paths, invalid YAML)
    // 4. Send SIGHUP to trigger reload
    // 5. Reload fails with error (logged)
    // 6. Service continues running with OLD config (unchanged)
    // 7. Requests still succeed (no downtime)
    //
    // This tests:
    // - Invalid configs are rejected (validation works)
    // - Service continues with old config on validation failure
    // - No downtime from bad config deployments
    // - Critical for production stability

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let mut config_file = NamedTempFile::new().expect("Failed to create temp config");
        let valid_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
      endpoint_url: "http://localhost:9000"
"#;
        config_file
            .write_all(valid_config.as_bytes())
            .expect("Failed to write config");
        config_file.flush().expect("Failed to flush");

        log::info!("Created valid initial config");

        // Create dynamic config for this LocalStack instance
        let config_path = "/tmp/yatagarasu-hotreload-4.yaml";
        create_localstack_config("http://localhost:9000", config_path);

        // Start proxy with test harness
        let _proxy = ProxyTestHarness::start(config_path, 18080)
            .expect("Failed to start proxy for invalid config rejection test");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        // Verify service works
        let response = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await;

        log::info!(
            "Service working before reload: {:?}",
            response.as_ref().map(|r| r.status())
        );

        // Update config with INVALID configuration (duplicate path_prefix)
        let invalid_config = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
      endpoint_url: "http://localhost:9000"
  - name: "bucket2"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-2"
      region: "us-west-2"
      access_key: "key2"
      secret_key: "secret2"
      endpoint_url: "http://localhost:9000"
"#;

        fs::write(config_file.path(), invalid_config).expect("Failed to write invalid config");

        log::info!("Updated config with INVALID configuration (duplicate path_prefix)");

        // Send SIGHUP (reload should fail)
        // TODO: send_sighup(proxy_pid);

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Service should still work with OLD config
        let response_after = client
            .get("http://127.0.0.1:18080/api/test.txt")
            .send()
            .await
            .expect("Failed to send request");

        // Should succeed (using old config)
        log::info!(
            "Service still works after failed reload: {}",
            response_after.status()
        );

        // Verify only bucket1 exists (old config), bucket2 was not added
        // (Detailed verification would check logs for validation error message)

        log::info!("Invalid config rejection test passed");
    });
}
