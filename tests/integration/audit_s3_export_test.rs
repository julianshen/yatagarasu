// Integration tests for S3 Audit Export
// Phase 33.6: S3 Export for Audit Logs
// Requires Docker - run with: cargo test --test integration_tests audit_s3_export -- --ignored

use std::sync::Arc;
use std::time::Duration;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;
use yatagarasu::audit::{
    AsyncS3AuditExportService, AuditBatch, AuditLogEntry, S3AuditExportConfig, S3AuditExporter,
    S3AuditUploader,
};

/// Create an S3 client configured for LocalStack
async fn create_localstack_s3_client(endpoint: &str) -> aws_sdk_s3::Client {
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url(endpoint)
        .region(aws_config::Region::new("us-east-1"))
        .credentials_provider(aws_credential_types::Credentials::new(
            "test", "test", None, None, "test",
        ))
        .load()
        .await;

    aws_sdk_s3::Client::new(&config)
}

/// Create the audit bucket in LocalStack
async fn create_audit_bucket(client: &aws_sdk_s3::Client, bucket: &str) {
    client
        .create_bucket()
        .bucket(bucket)
        .send()
        .await
        .expect("Failed to create audit bucket");
}

#[test]
#[ignore] // Requires Docker
fn test_uploads_batch_file_to_s3() {
    // Test: Uploads batch file to S3
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let s3_client = create_localstack_s3_client(&endpoint).await;
        create_audit_bucket(&s3_client, "audit-logs").await;

        // Create uploader
        let uploader = S3AuditUploader::new(s3_client.clone(), 3);

        // Create batch with entries
        let mut batch = AuditBatch::new();
        for i in 0..5 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "test-bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            )
            .with_response(200, 1024, 50);
            batch.add(entry);
        }

        // Upload batch
        let object_key = batch.generate_object_key("audit/");
        let result = uploader
            .upload_batch(&batch, "audit-logs", &object_key)
            .await;

        assert!(result.success, "Upload should succeed: {:?}", result.error);
        assert_eq!(result.attempts, 1, "Should succeed on first attempt");

        // Verify file exists in S3
        let get_result = s3_client
            .get_object()
            .bucket("audit-logs")
            .key(&object_key)
            .send()
            .await;

        assert!(
            get_result.is_ok(),
            "Should be able to retrieve uploaded file"
        );

        // Verify content
        let body = get_result
            .unwrap()
            .body
            .collect()
            .await
            .unwrap()
            .into_bytes();
        let content = String::from_utf8(body.to_vec()).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 5, "Should have 5 entries");

        // Verify each line is valid JSON
        for line in lines {
            let parsed: Result<AuditLogEntry, _> = serde_json::from_str(line);
            assert!(parsed.is_ok(), "Each line should be valid JSON");
        }
    });
}

#[test]
#[ignore] // Requires Docker
fn test_handles_s3_upload_failures_with_retries() {
    // Test: Handles S3 upload failures (retries)
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let s3_client = create_localstack_s3_client(&endpoint).await;
        // NOTE: We don't create the bucket, so uploads should fail

        let uploader = S3AuditUploader::new(s3_client.clone(), 3);

        let mut batch = AuditBatch::new();
        batch.add(AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "test-bucket".to_string(),
            "file.txt".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        ));

        let result = uploader
            .upload_batch(&batch, "nonexistent-bucket", "audit/test.jsonl")
            .await;

        // Should fail after all retries
        assert!(!result.success, "Upload to nonexistent bucket should fail");
        assert_eq!(result.attempts, 3, "Should have attempted 3 times");
        assert!(result.error.is_some(), "Should have error message");
    });
}

#[test]
#[ignore] // Requires Docker
fn test_keeps_local_copy_until_upload_succeeds() {
    // Test: Keeps local copy until upload succeeds
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    // Create temp directory for backups
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let backup_dir = temp_dir.path().to_path_buf();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let s3_client = create_localstack_s3_client(&endpoint).await;
        // Don't create bucket - uploads will fail

        let uploader =
            S3AuditUploader::new(s3_client.clone(), 2).with_backup_dir(backup_dir.clone());

        let mut batch = AuditBatch::new();
        batch.add(AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "test-bucket".to_string(),
            "file.txt".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        ));

        let object_key = "audit/failed-upload.jsonl";
        let result = uploader
            .upload_batch(&batch, "nonexistent-bucket", object_key)
            .await;

        // Upload should fail
        assert!(!result.success);

        // But local backup should exist
        let backup_filename = object_key.replace('/', "_");
        let backup_path = backup_dir.join(&backup_filename);

        assert!(
            backup_path.exists(),
            "Backup file should exist at {:?}",
            backup_path
        );

        // Verify backup content
        let backup_content = std::fs::read_to_string(&backup_path).expect("Should read backup");
        let parsed: Result<AuditLogEntry, _> = serde_json::from_str(&backup_content);
        assert!(parsed.is_ok(), "Backup should contain valid JSON");
    });
}

#[test]
#[ignore] // Requires Docker
fn test_export_runs_in_background_task() {
    // Test: Export runs in background task
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let s3_client = create_localstack_s3_client(&endpoint).await;
        create_audit_bucket(&s3_client, "audit-logs").await;

        // Create exporter with short interval for testing
        let config = S3AuditExportConfig {
            bucket: "audit-logs".to_string(),
            prefix: "test/".to_string(),
            export_interval_secs: 1, // 1 second for testing
            max_batch_size: 100,
            max_retries: 3,
        };

        let exporter = Arc::new(S3AuditExporter::new(config));
        let uploader = Arc::new(S3AuditUploader::new(s3_client.clone(), 3));

        let mut service = AsyncS3AuditExportService::new(exporter.clone(), uploader);

        // Start the service
        service.start();

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(service.is_running(), "Service should be running");

        // Add some entries
        for i in 0..5 {
            service.add_entry(AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "test-bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            ));
        }

        // Wait for export interval + buffer
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Shutdown service
        service.shutdown().await;

        assert!(!service.is_running(), "Service should have stopped");

        // Check that files were uploaded
        let list_result = s3_client
            .list_objects_v2()
            .bucket("audit-logs")
            .prefix("test/")
            .send()
            .await
            .expect("Should list objects");

        let objects = list_result.contents();
        assert!(
            !objects.is_empty(),
            "Should have uploaded at least one audit file"
        );

        // Verify object key format
        for obj in objects {
            let key = obj.key().unwrap();
            assert!(
                key.starts_with("test/yatagarasu-audit-"),
                "Key should have correct format: {}",
                key
            );
            assert!(
                key.ends_with(".jsonl"),
                "Key should end with .jsonl: {}",
                key
            );
        }
    });
}

#[test]
#[ignore] // Requires Docker
fn test_does_not_block_request_processing() {
    // Test: Does not block request processing
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let s3_client = create_localstack_s3_client(&endpoint).await;
        create_audit_bucket(&s3_client, "audit-logs").await;

        let config = S3AuditExportConfig {
            bucket: "audit-logs".to_string(),
            prefix: "perf-test/".to_string(),
            export_interval_secs: 60, // Long interval - won't trigger during test
            max_batch_size: 10000,
            max_retries: 3,
        };

        let exporter = Arc::new(S3AuditExporter::new(config));
        let uploader = Arc::new(S3AuditUploader::new(s3_client.clone(), 3));

        let mut service = AsyncS3AuditExportService::new(exporter.clone(), uploader);
        service.start();

        // Measure time to add entries
        let start = std::time::Instant::now();

        for i in 0..1000 {
            service.add_entry(AuditLogEntry::new(
                format!("192.168.1.{}", i % 256),
                "test-bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            ));
        }

        let elapsed = start.elapsed();

        // Adding 1000 entries should be very fast (non-blocking)
        assert!(
            elapsed.as_millis() < 100,
            "Adding entries should be non-blocking, took {:?}",
            elapsed
        );

        service.shutdown().await;
    });
}

#[test]
#[ignore] // Requires Docker
fn test_flushes_remaining_entries_on_shutdown() {
    // Test: Flushes remaining entries on shutdown
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let s3_client = create_localstack_s3_client(&endpoint).await;
        create_audit_bucket(&s3_client, "audit-logs").await;

        let config = S3AuditExportConfig {
            bucket: "audit-logs".to_string(),
            prefix: "shutdown-test/".to_string(),
            export_interval_secs: 3600, // 1 hour - won't trigger during test
            max_batch_size: 10000,
            max_retries: 3,
        };

        let exporter = Arc::new(S3AuditExporter::new(config));
        let uploader = Arc::new(S3AuditUploader::new(s3_client.clone(), 3));

        let mut service = AsyncS3AuditExportService::new(exporter.clone(), uploader);
        service.start();

        // Add entries
        for i in 0..10 {
            service.add_entry(AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "test-bucket".to_string(),
                format!("shutdown-file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            ));
        }

        // Verify entries are queued but not yet uploaded
        // (interval hasn't triggered yet)

        // Shutdown - this should flush remaining entries
        service.shutdown().await;

        // Give a moment for async operations to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Check that entries were uploaded on shutdown
        let list_result = s3_client
            .list_objects_v2()
            .bucket("audit-logs")
            .prefix("shutdown-test/")
            .send()
            .await
            .expect("Should list objects");

        let objects = list_result.contents();
        assert!(
            !objects.is_empty(),
            "Shutdown should have flushed entries to S3"
        );

        // Verify content
        let key = objects.first().unwrap().key().unwrap();
        let get_result = s3_client
            .get_object()
            .bucket("audit-logs")
            .key(key)
            .send()
            .await
            .expect("Should get uploaded file");

        let body = get_result.body.collect().await.unwrap().into_bytes();
        let content = String::from_utf8(body.to_vec()).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(
            lines.len(),
            10,
            "All 10 entries should have been flushed on shutdown"
        );
    });
}
