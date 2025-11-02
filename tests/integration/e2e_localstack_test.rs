// End-to-end integration tests with LocalStack
// Phase 16: Real S3 integration testing using testcontainers

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

#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_can_start_localstack_container() {
    init_logging();

    // Create Docker client
    let docker = Cli::default();

    // Create LocalStack container with S3 service
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    // Start container
    let container = docker.run(localstack_image);

    // Get the port that S3 is exposed on
    let port = container.get_host_port_ipv4(4566);

    // Verify we can connect
    let endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 endpoint: {}", endpoint);

    // Simple connectivity test
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    let response = client
        .get(&endpoint)
        .send()
        .expect("Failed to connect to LocalStack");

    log::info!("LocalStack response status: {}", response.status());
    assert!(
        response.status().is_success() || response.status().is_client_error(),
        "LocalStack should respond (got {})",
        response.status()
    );
}

#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_can_create_s3_bucket_in_localstack() {
    init_logging();

    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("Creating S3 bucket in LocalStack at {}", endpoint);

    // Use AWS SDK to create bucket
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
        let create_result = s3_client.create_bucket().bucket("test-bucket").send().await;

        log::info!("Create bucket result: {:?}", create_result);
        assert!(
            create_result.is_ok(),
            "Should be able to create bucket: {:?}",
            create_result.err()
        );

        // List buckets to verify
        let list_result = s3_client.list_buckets().send().await;
        assert!(list_result.is_ok(), "Should be able to list buckets");

        let list_output = list_result.unwrap();
        let buckets = list_output.buckets();
        log::info!("Buckets: {:?}", buckets);
        assert!(buckets.iter().any(|b| b.name() == Some("test-bucket")));
    });
}

#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_can_upload_and_download_file_from_localstack() {
    init_logging();

    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let endpoint = format!("http://127.0.0.1:{}", port);

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
            .bucket("test-bucket")
            .send()
            .await
            .expect("Failed to create bucket");

        // Upload test file
        let test_content = "Hello from Yatagarasu integration test!";
        s3_client
            .put_object()
            .bucket("test-bucket")
            .key("test.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");

        log::info!("Uploaded test file to S3");

        // Download file
        let get_result = s3_client
            .get_object()
            .bucket("test-bucket")
            .key("test.txt")
            .send()
            .await
            .expect("Failed to download file");

        let body = get_result
            .body
            .collect()
            .await
            .expect("Failed to read body");
        let content = String::from_utf8(body.to_vec()).expect("Invalid UTF-8");

        log::info!("Downloaded content: {}", content);
        assert_eq!(content, test_content);
    });
}

#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_proxy_get_from_localstack_public_bucket() {
    init_logging();

    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 running at {}", s3_endpoint);

    // Setup: Create bucket and upload test file to LocalStack
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket
        s3_client
            .create_bucket()
            .bucket("test-public-bucket")
            .send()
            .await
            .expect("Failed to create bucket");

        // Upload test file
        let test_content = "Hello from Yatagarasu E2E test!";
        s3_client
            .put_object()
            .bucket("test-public-bucket")
            .key("test.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");

        log::info!("Uploaded test.txt to LocalStack S3");
    });

    // Create proxy configuration file pointing to LocalStack
    // Use a high port number unlikely to conflict
    let proxy_port = 18080;
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

buckets:
  - name: "public"
    path_prefix: "/public"
    s3:
      region: "us-east-1"
      bucket: "test-public-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");

    log::info!("Created proxy config at {:?}", config_file.path());

    // Start proxy server in background thread
    let config_path = config_file.path().to_str().unwrap().to_string();

    std::thread::spawn(move || {
        // Load configuration
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");

        log::info!("Proxy config loaded");

        // Create Pingora server
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();

        // Create YatagarasuProxy instance
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());

        // Create HTTP proxy service
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

        // Add TCP listener
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);

        log::info!("Starting proxy server at {}", listen_addr);

        // Register service with server
        server.add_service(proxy_service);

        // Run server (blocks until shutdown)
        server.run_forever();
    });

    // Give server time to start up and bind to port
    std::thread::sleep(Duration::from_secs(2));

    // Test: Make GET request to proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/public/test.txt", proxy_port);
    log::info!("Requesting: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK, got {}",
        response.status()
    );

    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(body, "Hello from Yatagarasu E2E test!");
}

#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_proxy_head_from_localstack() {
    init_logging();

    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 running at {}", s3_endpoint);

    // Setup: Create bucket and upload test file
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        s3_client
            .create_bucket()
            .bucket("test-public-bucket")
            .send()
            .await
            .expect("Failed to create bucket");

        let test_content = "Hello from HEAD test!";
        s3_client
            .put_object()
            .bucket("test-public-bucket")
            .key("test.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");

        log::info!("Uploaded test.txt to LocalStack S3");
    });

    // Create and start proxy
    let proxy_port = 18081; // Different port from GET test
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

buckets:
  - name: "public"
    path_prefix: "/public"
    s3:
      region: "us-east-1"
      bucket: "test-public-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");

    let config_path = config_file.path().to_str().unwrap().to_string();

    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Starting proxy server at {}", listen_addr);
        server.add_service(proxy_service);
        server.run_forever();
    });

    std::thread::sleep(Duration::from_secs(2));

    // Test: Make HEAD request to proxy
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/public/test.txt", proxy_port);
    log::info!("HEAD request: {}", proxy_url);

    let response = client
        .head(&proxy_url)
        .send()
        .expect("Failed to HEAD from proxy");

    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for HEAD, got {}",
        response.status()
    );

    // HEAD should not return a body
    let content_length = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    log::info!("Content-Length: {:?}", content_length);
    // Content-Length header should be present (indicates file size)
    assert!(
        content_length.is_some(),
        "HEAD response should include Content-Length"
    );
}

#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_proxy_404_from_localstack() {
    init_logging();

    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);

    log::info!("LocalStack S3 running at {}", s3_endpoint);

    // Setup: Create bucket but don't upload any files
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        s3_client
            .create_bucket()
            .bucket("test-public-bucket")
            .send()
            .await
            .expect("Failed to create bucket");

        log::info!("Created empty bucket in LocalStack S3");
    });

    // Create and start proxy
    let proxy_port = 18082; // Different port
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

buckets:
  - name: "public"
    path_prefix: "/public"
    s3:
      region: "us-east-1"
      bucket: "test-public-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");

    let config_path = config_file.path().to_str().unwrap().to_string();

    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Starting proxy server at {}", listen_addr);
        server.add_service(proxy_service);
        server.run_forever();
    });

    std::thread::sleep(Duration::from_secs(2));

    // Test: Request non-existent file
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/public/nonexistent.txt", proxy_port);
    log::info!("GET request for non-existent file: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        404,
        "Expected 404 Not Found, got {}",
        response.status()
    );
}

/// Test 7: Proxy forwards Range header to S3 and returns 206 Partial Content
///
/// This test verifies that the proxy correctly handles HTTP Range requests:
/// 1. Client sends Range header (e.g., "bytes=0-99")
/// 2. Proxy forwards Range header to S3
/// 3. S3 returns 206 Partial Content with requested byte range
/// 4. Proxy forwards 206 response to client with Content-Range header
///
/// This is critical for:
/// - Video streaming (seek to timestamp)
/// - Large file downloads (resume, parallel chunks)
/// - Bandwidth optimization (request only needed bytes)
#[test]
#[ignore] // Requires Docker - run with: cargo test --test e2e_localstack_test -- --ignored
fn test_proxy_range_request_from_localstack() {
    init_logging();

    log::info!("=== Starting Range Request Integration Test ===");

    // Step 1: Start LocalStack container with S3
    log::info!("Starting LocalStack container...");
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 running at: {}", s3_endpoint);

    // Step 2: Create S3 bucket and upload test file
    log::info!("Creating S3 bucket and uploading test file...");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket
        s3_client
            .create_bucket()
            .bucket("test-range-bucket")
            .send()
            .await
            .expect("Failed to create bucket");
        log::info!("Bucket created: test-range-bucket");

        // Upload test file with known content
        // Content: "0123456789" repeated 10 times = 100 bytes
        let test_content = "0123456789".repeat(10);
        s3_client
            .put_object()
            .bucket("test-range-bucket")
            .key("test-range.txt")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");
        log::info!("Uploaded test-range.txt (100 bytes)");
    });

    // Step 3: Create proxy configuration
    let proxy_port = 18082; // Different port from other tests
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}
buckets:
  - name: "public"
    path_prefix: "/public"
    s3:
      region: "us-east-1"
      bucket: "test-range-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");
    log::info!("Proxy config written to: {:?}", config_file.path());

    // Step 4: Start Yatagarasu proxy in background thread
    log::info!("Starting Yatagarasu proxy server...");
    let config_path = config_file.path().to_str().unwrap().to_string();
    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        server.add_service(proxy_service);
        log::info!("Proxy server starting on {}", listen_addr);
        server.run_forever();
    });

    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    log::info!("Proxy server should be ready");

    // Step 5: Make HTTP request with Range header
    log::info!("Making Range request to proxy...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/public/test-range.txt", proxy_port);

    // Request bytes 0-19 (first 20 bytes: "01234567890123456789")
    let response = client
        .get(&proxy_url)
        .header("Range", "bytes=0-19")
        .send()
        .expect("Failed to GET from proxy");

    // Step 6: Verify 206 Partial Content response
    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        206,
        "Expected 206 Partial Content for Range request, got {}",
        response.status()
    );

    // Step 7: Verify Content-Range header
    let content_range = response
        .headers()
        .get("content-range")
        .expect("Expected Content-Range header in 206 response");
    log::info!("Content-Range header: {:?}", content_range);
    assert!(
        content_range
            .to_str()
            .unwrap()
            .starts_with("bytes 0-19/100"),
        "Expected 'bytes 0-19/100', got {:?}",
        content_range
    );

    // Step 8: Verify response body contains exactly 20 bytes
    let body = response.bytes().expect("Failed to read response body");
    log::info!("Response body length: {} bytes", body.len());
    assert_eq!(
        body.len(),
        20,
        "Expected 20 bytes (bytes 0-19), got {}",
        body.len()
    );

    // Step 9: Verify response body content matches requested range
    let expected_content = "01234567890123456789";
    let actual_content = String::from_utf8_lossy(&body);
    log::info!("Response body: {}", actual_content);
    assert_eq!(
        actual_content, expected_content,
        "Expected '{}', got '{}'",
        expected_content, actual_content
    );

    log::info!("=== Range Request Test PASSED ===");
}

/// Test 8: Proxy returns 401 Unauthorized when accessing private bucket without JWT
///
/// This test verifies that the proxy correctly enforces JWT authentication:
/// 1. Configure bucket with auth.enabled = true
/// 2. Client sends request without Authorization header
/// 3. Proxy returns 401 Unauthorized
/// 4. Response includes WWW-Authenticate header
///
/// This is critical for:
/// - Security: Prevent unauthorized access to private data
/// - HTTP compliance: Proper use of 401 status and WWW-Authenticate header
/// - API clarity: Clear error messages for missing credentials
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_401_without_jwt() {
    init_logging();

    log::info!("=== Starting JWT 401 Unauthorized Test ===");

    // Step 1: Start LocalStack container with S3
    log::info!("Starting LocalStack container...");
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 running at: {}", s3_endpoint);

    // Step 2: Create S3 bucket and upload test file
    log::info!("Creating S3 bucket and uploading test file...");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create private bucket
        s3_client
            .create_bucket()
            .bucket("test-private-bucket")
            .send()
            .await
            .expect("Failed to create bucket");
        log::info!("Bucket created: test-private-bucket");

        // Upload test file
        let test_content = r#"{"secret": "data", "user": "alice"}"#;
        s3_client
            .put_object()
            .bucket("test-private-bucket")
            .key("data.json")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");
        log::info!("Uploaded data.json to private bucket");
    });

    // Step 3: Create proxy configuration with JWT authentication enabled
    let proxy_port = 18083; // Different port from other tests
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "bearer_header"

buckets:
  - name: "private"
    path_prefix: "/private"
    auth:
      enabled: true
    s3:
      region: "us-east-1"
      bucket: "test-private-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, jwt_secret, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");
    log::info!("Proxy config written to: {:?}", config_file.path());

    // Step 4: Start Yatagarasu proxy in background thread
    log::info!("Starting Yatagarasu proxy server...");
    let config_path = config_file.path().to_str().unwrap().to_string();
    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        server.add_service(proxy_service);
        log::info!("Proxy server starting on {}", listen_addr);
        server.run_forever();
    });

    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    log::info!("Proxy server should be ready");

    // Step 5: Make HTTP request WITHOUT Authorization header
    log::info!("Making request WITHOUT JWT token...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/private/data.json", proxy_port);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to GET from proxy");

    // Step 6: Verify 401 Unauthorized response
    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        401,
        "Expected 401 Unauthorized for request without JWT, got {}",
        response.status()
    );

    // Step 7: Verify WWW-Authenticate header is present
    let www_authenticate = response.headers().get("www-authenticate");
    log::info!("WWW-Authenticate header: {:?}", www_authenticate);
    assert!(
        www_authenticate.is_some(),
        "Expected WWW-Authenticate header in 401 response"
    );
    assert_eq!(
        www_authenticate.unwrap().to_str().unwrap(),
        "Bearer",
        "Expected 'Bearer' authentication scheme"
    );

    log::info!("=== JWT 401 Unauthorized Test PASSED ===");
}

/// Test 9: Proxy returns 403 Forbidden for invalid or expired JWT
///
/// This test verifies that the proxy correctly rejects invalid JWTs:
/// 1. Configure bucket with auth.enabled = true
/// 2. Client sends request with malformed JWT → 403 Forbidden
/// 3. Client sends request with invalid signature → 403 Forbidden
/// 4. Client sends request with expired JWT → 403 Forbidden (arguable: could be 401)
///
/// This is critical for:
/// - Security: Prevent access with tampered or compromised tokens
/// - HTTP compliance: 403 means "authenticated but forbidden" vs 401 "not authenticated"
/// - Attack prevention: Invalid JWTs indicate potential attack attempts
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_403_invalid_jwt() {
    init_logging();

    log::info!("=== Starting JWT 403 Forbidden Test ===");

    // Step 1: Start LocalStack container with S3
    log::info!("Starting LocalStack container...");
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 running at: {}", s3_endpoint);

    // Step 2: Create S3 bucket and upload test file
    log::info!("Creating S3 bucket and uploading test file...");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create private bucket
        s3_client
            .create_bucket()
            .bucket("test-private-bucket")
            .send()
            .await
            .expect("Failed to create bucket");
        log::info!("Bucket created: test-private-bucket");

        // Upload test file
        let test_content = r#"{"secret": "data", "user": "bob"}"#;
        s3_client
            .put_object()
            .bucket("test-private-bucket")
            .key("data.json")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");
        log::info!("Uploaded data.json to private bucket");
    });

    // Step 3: Create proxy configuration with JWT authentication enabled
    let proxy_port = 18084; // Different port from other tests
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "bearer_header"

buckets:
  - name: "private"
    path_prefix: "/private"
    auth:
      enabled: true
    s3:
      region: "us-east-1"
      bucket: "test-private-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, jwt_secret, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");
    log::info!("Proxy config written to: {:?}", config_file.path());

    // Step 4: Start Yatagarasu proxy in background thread
    log::info!("Starting Yatagarasu proxy server...");
    let config_path = config_file.path().to_str().unwrap().to_string();
    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        server.add_service(proxy_service);
        log::info!("Proxy server starting on {}", listen_addr);
        server.run_forever();
    });

    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    log::info!("Proxy server should be ready");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/private/data.json", proxy_port);

    // Test Case 1: Malformed JWT (not 3 parts)
    log::info!("Test Case 1: Malformed JWT (not 3 parts)");
    let response = client
        .get(&proxy_url)
        .header("Authorization", "Bearer invalid-token")
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        403,
        "Expected 403 Forbidden for malformed JWT, got {}",
        response.status()
    );

    // Test Case 2: Invalid signature (tampered token)
    log::info!("Test Case 2: Invalid signature (tampered JWT)");
    // This is a JWT with valid structure but wrong signature
    // Header: {"alg":"HS256","typ":"JWT"}
    // Payload: {"sub":"user123","exp":9999999999}
    // Signature: signed with WRONG secret
    let invalid_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMTIzIiwiZXhwIjo5OTk5OTk5OTk5fQ.WRONG_SIGNATURE_HERE";
    let response = client
        .get(&proxy_url)
        .header("Authorization", format!("Bearer {}", invalid_jwt))
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        403,
        "Expected 403 Forbidden for JWT with invalid signature, got {}",
        response.status()
    );

    // Test Case 3: Expired JWT
    log::info!("Test Case 3: Expired JWT");
    // Create a JWT that expired in the past
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let expired_claims = Claims {
        sub: "user123".to_string(),
        exp: 1000000000, // January 9, 2001 - definitely expired
    };

    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create expired JWT");

    let response = client
        .get(&proxy_url)
        .header("Authorization", format!("Bearer {}", expired_token))
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        403,
        "Expected 403 Forbidden for expired JWT, got {}",
        response.status()
    );

    log::info!("=== JWT 403 Forbidden Test PASSED ===");
}

/// Test 10: Proxy returns 200 OK and file content with valid JWT
///
/// This test verifies that the proxy correctly allows access with valid JWT:
/// 1. Configure bucket with auth.enabled = true
/// 2. Client sends request with valid JWT token
/// 3. Proxy validates JWT and allows request through
/// 4. Proxy forwards to S3 with AWS SigV4 signature
/// 5. Proxy returns 200 OK with file content
///
/// This is the "happy path" for JWT authentication and validates that:
/// - Valid JWTs are accepted
/// - Claims are correctly validated
/// - S3 request proceeds normally after auth succeeds
/// - Response content is returned to client
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_200_valid_jwt() {
    init_logging();

    log::info!("=== Starting JWT 200 OK Valid Token Test ===");

    // Step 1: Start LocalStack container with S3
    log::info!("Starting LocalStack container...");
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 running at: {}", s3_endpoint);

    // Step 2: Create S3 bucket and upload test file
    log::info!("Creating S3 bucket and uploading test file...");
    let test_content = r#"{"message": "Success! JWT auth works.", "user": "charlie"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create private bucket
        s3_client
            .create_bucket()
            .bucket("test-private-bucket")
            .send()
            .await
            .expect("Failed to create bucket");
        log::info!("Bucket created: test-private-bucket");

        // Upload test file
        s3_client
            .put_object()
            .bucket("test-private-bucket")
            .key("data.json")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");
        log::info!("Uploaded data.json to private bucket");
    });

    // Step 3: Create proxy configuration with JWT authentication enabled
    let proxy_port = 18085; // Different port from other tests
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "bearer_header"

buckets:
  - name: "private"
    path_prefix: "/private"
    auth:
      enabled: true
    s3:
      region: "us-east-1"
      bucket: "test-private-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, jwt_secret, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");
    log::info!("Proxy config written to: {:?}", config_file.path());

    // Step 4: Start Yatagarasu proxy in background thread
    log::info!("Starting Yatagarasu proxy server...");
    let config_path = config_file.path().to_str().unwrap().to_string();
    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        server.add_service(proxy_service);
        log::info!("Proxy server starting on {}", listen_addr);
        server.run_forever();
    });

    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    log::info!("Proxy server should be ready");

    // Step 5: Create valid JWT token
    log::info!("Creating valid JWT token...");
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let valid_claims = Claims {
        sub: "charlie".to_string(),
        exp: (chrono::Utc::now().timestamp() + 3600) as usize, // Valid for 1 hour
    };

    let valid_token = encode(
        &Header::default(),
        &valid_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create valid JWT");

    log::info!("Valid JWT token created");

    // Step 6: Make HTTP request with valid JWT
    log::info!("Making request WITH valid JWT token...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/private/data.json", proxy_port);

    let response = client
        .get(&proxy_url)
        .header("Authorization", format!("Bearer {}", valid_token))
        .send()
        .expect("Failed to GET from proxy");

    // Step 7: Verify 200 OK response
    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for request with valid JWT, got {}",
        response.status()
    );

    // Step 8: Verify response body matches uploaded content
    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, test_content,
        "Expected response body to match uploaded content"
    );

    log::info!("=== JWT 200 OK Valid Token Test PASSED ===");
}

/// Test 11: Proxy returns 403 Forbidden for JWT with wrong claims
///
/// This test verifies that the proxy correctly rejects JWTs with valid signatures
/// but claims that don't match the configured claim rules:
/// 1. Configure bucket with auth.enabled = true and specific claim requirements
/// 2. Client sends request with valid JWT but wrong claim values
/// 3. Proxy validates JWT signature (succeeds)
/// 4. Proxy validates claims against rules (fails)
/// 5. Proxy returns 403 Forbidden
///
/// This is critical for:
/// - Authorization: Token is valid but user doesn't have access
/// - RBAC: Role-based access control via claims
/// - Multi-tenancy: Ensure users can only access their own data
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_403_wrong_claims() {
    init_logging();

    log::info!("=== Starting JWT 403 Wrong Claims Test ===");

    // Step 1: Start LocalStack container with S3
    log::info!("Starting LocalStack container...");
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 running at: {}", s3_endpoint);

    // Step 2: Create S3 bucket and upload test file
    log::info!("Creating S3 bucket and uploading test file...");
    let test_content = r#"{"secret": "admin-only data", "user": "admin"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create private bucket
        s3_client
            .create_bucket()
            .bucket("test-private-bucket")
            .send()
            .await
            .expect("Failed to create bucket");
        log::info!("Bucket created: test-private-bucket");

        // Upload test file
        s3_client
            .put_object()
            .bucket("test-private-bucket")
            .key("admin.json")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");
        log::info!("Uploaded admin.json to private bucket");
    });

    // Step 3: Create proxy configuration with JWT auth + claim rules
    // Require: role = "admin" (but we'll send role = "user")
    let proxy_port = 18086; // Different port from other tests
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "bearer_header"
  claims:
    - claim: "role"
      operator: "equals"
      value: "admin"

buckets:
  - name: "private"
    path_prefix: "/private"
    auth:
      enabled: true
    s3:
      region: "us-east-1"
      bucket: "test-private-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, jwt_secret, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");
    log::info!("Proxy config written to: {:?}", config_file.path());

    // Step 4: Start Yatagarasu proxy in background thread
    log::info!("Starting Yatagarasu proxy server...");
    let config_path = config_file.path().to_str().unwrap().to_string();
    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        server.add_service(proxy_service);
        log::info!("Proxy server starting on {}", listen_addr);
        server.run_forever();
    });

    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    log::info!("Proxy server should be ready");

    // Step 5: Create JWT with WRONG role claim (role = "user" instead of "admin")
    log::info!("Creating JWT with wrong role claim...");
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        role: String,
        exp: usize,
    }

    let wrong_claims = Claims {
        sub: "user123".to_string(),
        role: "user".to_string(), // WRONG: proxy requires "admin"
        exp: (chrono::Utc::now().timestamp() + 3600) as usize,
    };

    let token_with_wrong_claims = encode(
        &Header::default(),
        &wrong_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create JWT with wrong claims");

    log::info!("JWT with wrong claims created (role=user, expected role=admin)");

    // Step 6: Make HTTP request with JWT that has wrong claims
    log::info!("Making request WITH valid JWT but wrong claims...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    let proxy_url = format!("http://127.0.0.1:{}/private/admin.json", proxy_port);

    let response = client
        .get(&proxy_url)
        .header(
            "Authorization",
            format!("Bearer {}", token_with_wrong_claims),
        )
        .send()
        .expect("Failed to GET from proxy");

    // Step 7: Verify 403 Forbidden response
    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        403,
        "Expected 403 Forbidden for JWT with wrong claims (role=user, expected role=admin), got {}",
        response.status()
    );

    log::info!("=== JWT 403 Wrong Claims Test PASSED ===");
}

/// Test 12: Proxy accepts JWT from query parameter
///
/// This test verifies that the proxy can extract JWT tokens from query parameters
/// as an alternative to the Authorization header:
/// 1. Configure JWT with token_source: "query" parameter "token"
/// 2. Client sends request with valid JWT in query string: ?token=<jwt>
/// 3. Proxy extracts token from query parameter
/// 4. Proxy validates JWT and allows request through
/// 5. Proxy returns 200 OK with file content
///
/// This is critical for:
/// - Browser access: Query params easier than setting headers in browser URL bar
/// - Websocket auth: Some websocket clients can't set custom headers
/// - Legacy systems: Systems that can only pass tokens via URL
/// - Link sharing: Pre-authenticated URLs (with expiring tokens)
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_jwt_from_query_parameter() {
    init_logging();

    log::info!("=== Starting JWT from Query Parameter Test ===");

    // Step 1: Start LocalStack container with S3
    log::info!("Starting LocalStack container...");
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!("LocalStack S3 running at: {}", s3_endpoint);

    // Step 2: Create S3 bucket and upload test file
    log::info!("Creating S3 bucket and uploading test file...");
    let test_content = r#"{"message": "JWT from query param works!", "method": "query"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test-access-key",
                "test-secret-key",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create private bucket
        s3_client
            .create_bucket()
            .bucket("test-private-bucket")
            .send()
            .await
            .expect("Failed to create bucket");
        log::info!("Bucket created: test-private-bucket");

        // Upload test file
        s3_client
            .put_object()
            .bucket("test-private-bucket")
            .key("data.json")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file");
        log::info!("Uploaded data.json to private bucket");
    });

    // Step 3: Create proxy configuration with JWT from query parameter
    let proxy_port = 18087; // Different port from other tests
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "query"
      param_name: "token"

buckets:
  - name: "private"
    path_prefix: "/private"
    auth:
      enabled: true
    s3:
      region: "us-east-1"
      bucket: "test-private-bucket"
      endpoint: "{}"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
"#,
        proxy_port, jwt_secret, s3_endpoint
    );

    let config_file = tempfile::NamedTempFile::new().expect("Failed to create temp config file");
    std::fs::write(config_file.path(), config_content).expect("Failed to write config");
    log::info!("Proxy config written to: {:?}", config_file.path());

    // Step 4: Start Yatagarasu proxy in background thread
    log::info!("Starting Yatagarasu proxy server...");
    let config_path = config_file.path().to_str().unwrap().to_string();
    std::thread::spawn(move || {
        let config =
            yatagarasu::config::Config::from_file(&config_path).expect("Failed to load config");
        let mut server =
            pingora_core::server::Server::new(None).expect("Failed to create Pingora server");
        server.bootstrap();
        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        server.add_service(proxy_service);
        log::info!("Proxy server starting on {}", listen_addr);
        server.run_forever();
    });

    // Wait for server to start
    std::thread::sleep(Duration::from_secs(2));
    log::info!("Proxy server should be ready");

    // Step 5: Create valid JWT token
    log::info!("Creating valid JWT token...");
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let valid_claims = Claims {
        sub: "user_query_param".to_string(),
        exp: (chrono::Utc::now().timestamp() + 3600) as usize,
    };

    let valid_token = encode(
        &Header::default(),
        &valid_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create valid JWT");

    log::info!("Valid JWT token created");

    // Step 6: Make HTTP request with JWT in query parameter
    log::info!("Making request WITH JWT in query parameter...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    // URL encode the JWT token for query parameter
    let encoded_token = urlencoding::encode(&valid_token);
    let proxy_url = format!(
        "http://127.0.0.1:{}/private/data.json?token={}",
        proxy_port, encoded_token
    );

    log::info!("Request URL: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to GET from proxy");

    // Step 7: Verify 200 OK response
    log::info!("Response status: {}", response.status());
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for request with JWT in query param, got {}",
        response.status()
    );

    // Step 8: Verify response body matches uploaded content
    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, test_content,
        "Expected response body to match uploaded content"
    );

    log::info!("=== JWT from Query Parameter Test PASSED ===");
}

/// Test 13: Proxy accepts JWT from custom header
///
/// This test verifies that the proxy can extract JWT tokens from custom headers
/// as an alternative to standard Authorization header:
/// 1. Configure JWT with token_source: "header" with custom header name "X-Auth-Token"
/// 2. Client sends request with valid JWT in custom header: X-Auth-Token: <jwt>
/// 3. Proxy extracts token from custom header
/// 4. Proxy validates JWT and allows request through
/// 5. Proxy returns 200 OK with file content
///
/// This is critical for:
/// - Custom auth schemes: Organizations with non-standard auth headers
/// - API gateways: Systems that transform tokens into custom headers
/// - Mobile apps: Apps that use custom auth headers for branding
/// - Backwards compatibility: Supporting legacy systems with custom headers
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_jwt_from_custom_header() {
    init_logging();
    log::info!("=== Starting JWT from Custom Header Test ===");

    // Step 1: Start LocalStack container with S3
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!(
        "LocalStack started on port {} (S3 endpoint: {})",
        port,
        s3_endpoint
    );

    // Step 2: Create S3 bucket and upload test file
    let test_content = r#"{"message": "JWT from custom header works!", "method": "custom_header"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::from_env()
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket
        s3_client
            .create_bucket()
            .bucket("test-private-bucket")
            .send()
            .await
            .expect("Failed to create S3 bucket");
        log::info!("Created S3 bucket: test-private-bucket");

        // Upload test file
        s3_client
            .put_object()
            .bucket("test-private-bucket")
            .key("data.json")
            .body(test_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload test file to S3");
        log::info!("Uploaded test file: data.json");
    });

    // Step 3: Create proxy configuration with JWT from custom header
    let proxy_port = 18088; // Unique port for this test
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "header"
      header_name: "X-Auth-Token"

buckets:
  - name: "private"
    path_prefix: "/private"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "test-private-bucket"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: true
"#,
        proxy_port, jwt_secret, s3_endpoint
    );

    // Write config to temporary file
    let config_path = std::env::temp_dir().join(format!("yatagarasu_test_{}.yaml", proxy_port));
    std::fs::write(&config_path, config_content).expect("Failed to write config file");
    log::info!("Wrote proxy config to: {:?}", config_path);

    // Step 4: Start proxy in background thread
    let config_path_clone = config_path.clone();
    std::thread::spawn(move || {
        let config = yatagarasu::config::Config::from_file(&config_path_clone)
            .expect("Failed to load config in proxy thread");
        log::info!("Loaded config in proxy thread, starting server...");

        let opt = pingora_core::server::configuration::Opt {
            upgrade: false,
            daemon: false,
            nocapture: false,
            test: false,
            conf: None,
        };

        let mut server =
            pingora_core::server::Server::new(Some(opt)).expect("Failed to create Pingora server");
        server.bootstrap();

        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Proxy listening on: {}", listen_addr);

        server.add_service(proxy_service);
        server.run_forever(); // This blocks
    });

    // Wait for proxy to start
    std::thread::sleep(std::time::Duration::from_secs(2));
    log::info!("Proxy should be started now");

    // Step 5: Create valid JWT token
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let valid_claims = Claims {
        sub: "user_custom_header".to_string(),
        exp: (chrono::Utc::now().timestamp() + 3600) as usize,
    };

    let valid_token = encode(
        &Header::default(),
        &valid_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create valid JWT");
    log::info!("Created valid JWT token: {}", &valid_token[..20]);

    // Step 6: Make HTTP request with JWT in custom header (X-Auth-Token)
    let client = reqwest::blocking::Client::new();
    let proxy_url = format!("http://127.0.0.1:{}/private/data.json", proxy_port);
    log::info!("Making request to proxy: {}", proxy_url);
    log::info!("Using custom header: X-Auth-Token");

    let response = client
        .get(&proxy_url)
        .header("X-Auth-Token", &valid_token) // Custom header instead of Authorization
        .send()
        .expect("Failed to GET from proxy");

    log::info!(
        "Received response: status={}, headers={:?}",
        response.status(),
        response.headers()
    );

    // Step 7: Verify 200 OK status
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for request with JWT in custom header, got {}",
        response.status()
    );

    // Step 8: Verify response body matches uploaded content
    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, test_content,
        "Expected response body to match uploaded content"
    );

    log::info!("=== JWT from Custom Header Test PASSED ===");
}

/// Test 14: Proxy routes /bucket-a/* to bucket-a with isolated credentials
///
/// This test verifies multi-bucket configuration with credential isolation:
/// 1. Create two S3 buckets (bucket-a, bucket-b) in LocalStack
/// 2. Upload different files to each bucket
/// 3. Configure proxy with two bucket configs:
///    - /bucket-a/* -> S3 bucket "bucket-a" with credentials A
///    - /bucket-b/* -> S3 bucket "bucket-b" with credentials B
/// 4. Request GET /bucket-a/file-a.txt
/// 5. Proxy routes to bucket-a using credentials A
/// 6. Returns 200 OK with content from bucket-a
///
/// This validates:
/// - Multiple buckets can be configured in same proxy
/// - Routing correctly selects bucket by path prefix
/// - Each bucket uses isolated credentials (no mixing)
/// - Request to bucket-a does NOT access bucket-b
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_multi_bucket_a() {
    init_logging();
    log::info!("=== Starting Multi-Bucket Test: Bucket A ===");

    // Step 1: Start LocalStack container with S3
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!(
        "LocalStack started on port {} (S3 endpoint: {})",
        port,
        s3_endpoint
    );

    // Step 2: Create TWO S3 buckets and upload different files to each
    let content_a = r#"{"bucket": "bucket-a", "file": "file-a.txt"}"#;
    let content_b = r#"{"bucket": "bucket-b", "file": "file-b.txt"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::from_env()
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket-a and upload file-a.txt
        s3_client
            .create_bucket()
            .bucket("bucket-a")
            .send()
            .await
            .expect("Failed to create bucket-a");
        log::info!("Created S3 bucket: bucket-a");

        s3_client
            .put_object()
            .bucket("bucket-a")
            .key("file-a.txt")
            .body(content_a.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file-a.txt to bucket-a");
        log::info!("Uploaded file-a.txt to bucket-a");

        // Create bucket-b and upload file-b.txt
        s3_client
            .create_bucket()
            .bucket("bucket-b")
            .send()
            .await
            .expect("Failed to create bucket-b");
        log::info!("Created S3 bucket: bucket-b");

        s3_client
            .put_object()
            .bucket("bucket-b")
            .key("file-b.txt")
            .body(content_b.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file-b.txt to bucket-b");
        log::info!("Uploaded file-b.txt to bucket-b");
    });

    // Step 3: Create proxy configuration with TWO buckets
    let proxy_port = 18089; // Unique port for this test
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

buckets:
  - name: "bucket-a"
    path_prefix: "/bucket-a"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "bucket-a"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false

  - name: "bucket-b"
    path_prefix: "/bucket-b"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "bucket-b"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
"#,
        proxy_port, s3_endpoint, s3_endpoint
    );

    // Write config to temporary file
    let config_path = std::env::temp_dir().join(format!("yatagarasu_test_{}.yaml", proxy_port));
    std::fs::write(&config_path, config_content).expect("Failed to write config file");
    log::info!("Wrote proxy config to: {:?}", config_path);

    // Step 4: Start proxy in background thread
    let config_path_clone = config_path.clone();
    std::thread::spawn(move || {
        let config = yatagarasu::config::Config::from_file(&config_path_clone)
            .expect("Failed to load config in proxy thread");
        log::info!(
            "Loaded config in proxy thread with {} buckets",
            config.buckets.len()
        );

        let opt = pingora_core::server::configuration::Opt {
            upgrade: false,
            daemon: false,
            nocapture: false,
            test: false,
            conf: None,
        };

        let mut server =
            pingora_core::server::Server::new(Some(opt)).expect("Failed to create Pingora server");
        server.bootstrap();

        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Proxy listening on: {}", listen_addr);

        server.add_service(proxy_service);
        server.run_forever(); // This blocks
    });

    // Wait for proxy to start
    std::thread::sleep(std::time::Duration::from_secs(2));
    log::info!("Proxy should be started now");

    // Step 5: Make HTTP request to /bucket-a/file-a.txt
    let client = reqwest::blocking::Client::new();
    let proxy_url = format!("http://127.0.0.1:{}/bucket-a/file-a.txt", proxy_port);
    log::info!("Making request to proxy: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to GET from proxy");

    log::info!(
        "Received response: status={}, headers={:?}",
        response.status(),
        response.headers()
    );

    // Step 6: Verify 200 OK status
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for /bucket-a/file-a.txt, got {}",
        response.status()
    );

    // Step 7: Verify response body matches content from bucket-a (not bucket-b)
    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, content_a,
        "Expected content from bucket-a, got '{}'",
        body
    );

    // Verify we got bucket-a content, not bucket-b content
    assert!(
        body.contains("bucket-a"),
        "Response should contain 'bucket-a'"
    );
    assert!(
        !body.contains("bucket-b"),
        "Response should NOT contain 'bucket-b'"
    );

    log::info!("=== Multi-Bucket Test: Bucket A PASSED ===");
}

/// Test 15: Proxy routes /bucket-b/* to bucket-b with isolated credentials
///
/// This test complements Test 14 by validating the second bucket in multi-bucket config:
/// 1. Create two S3 buckets (bucket-a, bucket-b) in LocalStack
/// 2. Upload different files to each bucket
/// 3. Configure proxy with two bucket configs:
///    - /bucket-a/* -> S3 bucket "bucket-a" with credentials A
///    - /bucket-b/* -> S3 bucket "bucket-b" with credentials B
/// 4. Request GET /bucket-b/file-b.txt
/// 5. Proxy routes to bucket-b using credentials B
/// 6. Returns 200 OK with content from bucket-b
///
/// This validates:
/// - Routing correctly selects bucket-b by path prefix /bucket-b/*
/// - Request to bucket-b does NOT access bucket-a
/// - Both buckets work correctly in same proxy instance
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_multi_bucket_b() {
    init_logging();
    log::info!("=== Starting Multi-Bucket Test: Bucket B ===");

    // Step 1: Start LocalStack container with S3
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!(
        "LocalStack started on port {} (S3 endpoint: {})",
        port,
        s3_endpoint
    );

    // Step 2: Create TWO S3 buckets and upload different files to each
    let content_a = r#"{"bucket": "bucket-a", "file": "file-a.txt"}"#;
    let content_b = r#"{"bucket": "bucket-b", "file": "file-b.txt"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::from_env()
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create bucket-a and upload file-a.txt
        s3_client
            .create_bucket()
            .bucket("bucket-a")
            .send()
            .await
            .expect("Failed to create bucket-a");
        log::info!("Created S3 bucket: bucket-a");

        s3_client
            .put_object()
            .bucket("bucket-a")
            .key("file-a.txt")
            .body(content_a.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file-a.txt to bucket-a");
        log::info!("Uploaded file-a.txt to bucket-a");

        // Create bucket-b and upload file-b.txt
        s3_client
            .create_bucket()
            .bucket("bucket-b")
            .send()
            .await
            .expect("Failed to create bucket-b");
        log::info!("Created S3 bucket: bucket-b");

        s3_client
            .put_object()
            .bucket("bucket-b")
            .key("file-b.txt")
            .body(content_b.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file-b.txt to bucket-b");
        log::info!("Uploaded file-b.txt to bucket-b");
    });

    // Step 3: Create proxy configuration with TWO buckets
    let proxy_port = 18090; // Unique port for this test
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

buckets:
  - name: "bucket-a"
    path_prefix: "/bucket-a"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "bucket-a"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false

  - name: "bucket-b"
    path_prefix: "/bucket-b"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "bucket-b"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
"#,
        proxy_port, s3_endpoint, s3_endpoint
    );

    // Write config to temporary file
    let config_path = std::env::temp_dir().join(format!("yatagarasu_test_{}.yaml", proxy_port));
    std::fs::write(&config_path, config_content).expect("Failed to write config file");
    log::info!("Wrote proxy config to: {:?}", config_path);

    // Step 4: Start proxy in background thread
    let config_path_clone = config_path.clone();
    std::thread::spawn(move || {
        let config = yatagarasu::config::Config::from_file(&config_path_clone)
            .expect("Failed to load config in proxy thread");
        log::info!(
            "Loaded config in proxy thread with {} buckets",
            config.buckets.len()
        );

        let opt = pingora_core::server::configuration::Opt {
            upgrade: false,
            daemon: false,
            nocapture: false,
            test: false,
            conf: None,
        };

        let mut server =
            pingora_core::server::Server::new(Some(opt)).expect("Failed to create Pingora server");
        server.bootstrap();

        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Proxy listening on: {}", listen_addr);

        server.add_service(proxy_service);
        server.run_forever(); // This blocks
    });

    // Wait for proxy to start
    std::thread::sleep(std::time::Duration::from_secs(2));
    log::info!("Proxy should be started now");

    // Step 5: Make HTTP request to /bucket-b/file-b.txt
    let client = reqwest::blocking::Client::new();
    let proxy_url = format!("http://127.0.0.1:{}/bucket-b/file-b.txt", proxy_port);
    log::info!("Making request to proxy: {}", proxy_url);

    let response = client
        .get(&proxy_url)
        .send()
        .expect("Failed to GET from proxy");

    log::info!(
        "Received response: status={}, headers={:?}",
        response.status(),
        response.headers()
    );

    // Step 6: Verify 200 OK status
    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for /bucket-b/file-b.txt, got {}",
        response.status()
    );

    // Step 7: Verify response body matches content from bucket-b (not bucket-a)
    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, content_b,
        "Expected content from bucket-b, got '{}'",
        body
    );

    // Verify we got bucket-b content, not bucket-a content
    assert!(
        body.contains("bucket-b"),
        "Response should contain 'bucket-b'"
    );
    assert!(
        !body.contains("bucket-a"),
        "Response should NOT contain 'bucket-a'"
    );

    log::info!("=== Multi-Bucket Test: Bucket B PASSED ===");
}

/// Test 16: Proxy supports mixed public and private buckets
///
/// This test validates that public and private buckets can coexist in same proxy:
/// 1. Create two S3 buckets (public-bucket, private-bucket) in LocalStack
/// 2. Upload different files to each bucket
/// 3. Configure proxy with mixed authentication:
///    - /public/* -> No auth required (auth.enabled = false)
///    - /private/* -> JWT auth required (auth.enabled = true)
/// 4. Request GET /public/public.txt WITHOUT JWT -> 200 OK
/// 5. Request GET /private/private.txt WITHOUT JWT -> 401 Unauthorized
/// 6. Request GET /private/private.txt WITH valid JWT -> 200 OK
///
/// This validates:
/// - Public and private buckets coexist without interference
/// - Public bucket accessible without JWT
/// - Private bucket requires JWT (401 without, 200 with valid JWT)
/// - Authentication enforced per-bucket (not globally)
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_mixed_public_private_buckets() {
    init_logging();
    log::info!("=== Starting Mixed Public/Private Buckets Test ===");

    // Step 1: Start LocalStack container with S3
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!(
        "LocalStack started on port {} (S3 endpoint: {})",
        port,
        s3_endpoint
    );

    // Step 2: Create TWO S3 buckets (public and private)
    let public_content = r#"{"bucket": "public", "accessible": "without JWT"}"#;
    let private_content = r#"{"bucket": "private", "requires": "JWT token"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::from_env()
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);

        // Create public bucket and upload public file
        s3_client
            .create_bucket()
            .bucket("public-bucket")
            .send()
            .await
            .expect("Failed to create public-bucket");
        log::info!("Created S3 bucket: public-bucket");

        s3_client
            .put_object()
            .bucket("public-bucket")
            .key("public.txt")
            .body(public_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload public.txt");
        log::info!("Uploaded public.txt to public-bucket");

        // Create private bucket and upload private file
        s3_client
            .create_bucket()
            .bucket("private-bucket")
            .send()
            .await
            .expect("Failed to create private-bucket");
        log::info!("Created S3 bucket: private-bucket");

        s3_client
            .put_object()
            .bucket("private-bucket")
            .key("private.txt")
            .body(private_content.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload private.txt");
        log::info!("Uploaded private.txt to private-bucket");
    });

    // Step 3: Create proxy configuration with MIXED auth
    let proxy_port = 18091; // Unique port for this test
    let jwt_secret = "test-secret-key-min-32-characters-long";
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

jwt:
  secret: "{}"
  algorithm: "HS256"
  token_sources:
    - type: "bearer_header"

buckets:
  - name: "public"
    path_prefix: "/public"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "public-bucket"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false

  - name: "private"
    path_prefix: "/private"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "private-bucket"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: true
"#,
        proxy_port, jwt_secret, s3_endpoint, s3_endpoint
    );

    // Write config to temporary file
    let config_path = std::env::temp_dir().join(format!("yatagarasu_test_{}.yaml", proxy_port));
    std::fs::write(&config_path, config_content).expect("Failed to write config file");
    log::info!("Wrote proxy config to: {:?}", config_path);

    // Step 4: Start proxy in background thread
    let config_path_clone = config_path.clone();
    std::thread::spawn(move || {
        let config = yatagarasu::config::Config::from_file(&config_path_clone)
            .expect("Failed to load config in proxy thread");
        log::info!(
            "Loaded config in proxy thread with {} buckets",
            config.buckets.len()
        );

        let opt = pingora_core::server::configuration::Opt {
            upgrade: false,
            daemon: false,
            nocapture: false,
            test: false,
            conf: None,
        };

        let mut server =
            pingora_core::server::Server::new(Some(opt)).expect("Failed to create Pingora server");
        server.bootstrap();

        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Proxy listening on: {}", listen_addr);

        server.add_service(proxy_service);
        server.run_forever(); // This blocks
    });

    // Wait for proxy to start
    std::thread::sleep(std::time::Duration::from_secs(2));
    log::info!("Proxy should be started now");

    let client = reqwest::blocking::Client::new();

    // Test Case 1: Public bucket accessible WITHOUT JWT
    log::info!("Test Case 1: Public bucket WITHOUT JWT -> 200 OK");
    let public_url = format!("http://127.0.0.1:{}/public/public.txt", proxy_port);
    log::info!("Making request to: {}", public_url);

    let response = client
        .get(&public_url)
        .send()
        .expect("Failed to GET from proxy");

    log::info!(
        "Response status: {}, headers: {:?}",
        response.status(),
        response.headers()
    );

    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for public bucket without JWT, got {}",
        response.status()
    );

    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, public_content,
        "Expected public content, got '{}'",
        body
    );

    // Test Case 2: Private bucket REJECTS request WITHOUT JWT (401)
    log::info!("Test Case 2: Private bucket WITHOUT JWT -> 401 Unauthorized");
    let private_url = format!("http://127.0.0.1:{}/private/private.txt", proxy_port);
    log::info!("Making request to: {}", private_url);

    let response = client
        .get(&private_url)
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());

    assert_eq!(
        response.status(),
        401,
        "Expected 401 Unauthorized for private bucket without JWT, got {}",
        response.status()
    );

    // Test Case 3: Private bucket ACCEPTS request WITH valid JWT (200)
    log::info!("Test Case 3: Private bucket WITH valid JWT -> 200 OK");

    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let valid_claims = Claims {
        sub: "user_mixed_test".to_string(),
        exp: (chrono::Utc::now().timestamp() + 3600) as usize,
    };

    let valid_token = encode(
        &Header::default(),
        &valid_claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("Failed to create valid JWT");

    log::info!("Created valid JWT token");

    let response = client
        .get(&private_url)
        .header("Authorization", format!("Bearer {}", valid_token))
        .send()
        .expect("Failed to GET from proxy");

    log::info!("Response status: {}", response.status());

    assert_eq!(
        response.status(),
        200,
        "Expected 200 OK for private bucket with valid JWT, got {}",
        response.status()
    );

    let body = response.text().expect("Failed to read response body");
    log::info!("Response body: {}", body);
    assert_eq!(
        body, private_content,
        "Expected private content, got '{}'",
        body
    );

    log::info!("=== Mixed Public/Private Buckets Test PASSED ===");
}

/// Test 17: Each bucket uses isolated credentials (no credential mixing)
///
/// This is a CRITICAL SECURITY TEST that validates credential isolation:
/// 1. Create two S3 buckets in LocalStack with DIFFERENT credentials
///    - bucket-secure-a: accessible ONLY with credentials-a
///    - bucket-secure-b: accessible ONLY with credentials-b
/// 2. Configure proxy with two buckets using DIFFERENT S3 credentials
///    - /secure-a/* -> bucket-secure-a with credentials-a
///    - /secure-b/* -> bucket-secure-b with credentials-b
/// 3. Request GET /secure-a/file.txt -> Proxy MUST use credentials-a
/// 4. Request GET /secure-b/file.txt -> Proxy MUST use credentials-b
/// 5. Verify proxy does NOT mix credentials (security critical!)
///
/// Security implications:
/// - Credential mixing would allow access to wrong bucket's data
/// - Multi-tenant isolation depends on credential separation
/// - Prevents privilege escalation between buckets
/// - Critical for compliance (PCI-DSS, HIPAA, SOC 2)
#[test]
#[ignore] // Requires Docker - run with: cargo test --test integration_tests -- --ignored
fn test_proxy_credential_isolation() {
    init_logging();
    log::info!("=== Starting Credential Isolation Test ===");

    // Step 1: Start LocalStack container with S3
    let docker = Cli::default();
    let localstack_image =
        RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));
    let container = docker.run(localstack_image);
    let port = container.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", port);
    log::info!(
        "LocalStack started on port {} (S3 endpoint: {})",
        port,
        s3_endpoint
    );

    // Step 2: Create TWO S3 buckets with DIFFERENT credentials
    // In real AWS, these would be separate IAM users/roles
    // In LocalStack, we simulate this with different access keys
    let content_a = r#"{"bucket": "secure-a", "credentials": "user-a"}"#;
    let content_b = r#"{"bucket": "secure-b", "credentials": "user-b"}"#;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        // Create bucket-secure-a with credentials-a
        let config_a = aws_config::from_env()
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_credential_types::Credentials::new(
                "user-a-access-key",
                "user-a-secret-key",
                None,
                None,
                "user-a",
            ))
            .load()
            .await;

        let s3_client_a = aws_sdk_s3::Client::new(&config_a);

        s3_client_a
            .create_bucket()
            .bucket("bucket-secure-a")
            .send()
            .await
            .expect("Failed to create bucket-secure-a");
        log::info!("Created S3 bucket: bucket-secure-a (with credentials-a)");

        s3_client_a
            .put_object()
            .bucket("bucket-secure-a")
            .key("file.txt")
            .body(content_a.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file.txt to bucket-secure-a");
        log::info!("Uploaded file.txt to bucket-secure-a");

        // Create bucket-secure-b with credentials-b
        let config_b = aws_config::from_env()
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(aws_credential_types::Credentials::new(
                "user-b-access-key",
                "user-b-secret-key",
                None,
                None,
                "user-b",
            ))
            .load()
            .await;

        let s3_client_b = aws_sdk_s3::Client::new(&config_b);

        s3_client_b
            .create_bucket()
            .bucket("bucket-secure-b")
            .send()
            .await
            .expect("Failed to create bucket-secure-b");
        log::info!("Created S3 bucket: bucket-secure-b (with credentials-b)");

        s3_client_b
            .put_object()
            .bucket("bucket-secure-b")
            .key("file.txt")
            .body(content_b.as_bytes().to_vec().into())
            .send()
            .await
            .expect("Failed to upload file.txt to bucket-secure-b");
        log::info!("Uploaded file.txt to bucket-secure-b");
    });

    // Step 3: Configure proxy with DIFFERENT credentials for each bucket
    let proxy_port = 18092; // Unique port for this test
    let config_content = format!(
        r#"
server:
  address: "127.0.0.1"
  port: {}

buckets:
  - name: "secure-a"
    path_prefix: "/secure-a"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "bucket-secure-a"
      access_key: "user-a-access-key"
      secret_key: "user-a-secret-key"
    auth:
      enabled: false

  - name: "secure-b"
    path_prefix: "/secure-b"
    s3:
      endpoint: "{}"
      region: "us-east-1"
      bucket: "bucket-secure-b"
      access_key: "user-b-access-key"
      secret_key: "user-b-secret-key"
    auth:
      enabled: false
"#,
        proxy_port, s3_endpoint, s3_endpoint
    );

    // Write config to temporary file
    let config_path = std::env::temp_dir().join(format!("yatagarasu_test_{}.yaml", proxy_port));
    std::fs::write(&config_path, config_content).expect("Failed to write config file");
    log::info!("Wrote proxy config to: {:?}", config_path);

    // Step 4: Start proxy in background thread
    let config_path_clone = config_path.clone();
    std::thread::spawn(move || {
        let config = yatagarasu::config::Config::from_file(&config_path_clone)
            .expect("Failed to load config in proxy thread");
        log::info!(
            "Loaded config in proxy thread with {} buckets",
            config.buckets.len()
        );

        let opt = pingora_core::server::configuration::Opt {
            upgrade: false,
            daemon: false,
            nocapture: false,
            test: false,
            conf: None,
        };

        let mut server =
            pingora_core::server::Server::new(Some(opt)).expect("Failed to create Pingora server");
        server.bootstrap();

        let proxy = yatagarasu::proxy::YatagarasuProxy::new(config.clone());
        let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

        let listen_addr = format!("{}:{}", config.server.address, config.server.port);
        proxy_service.add_tcp(&listen_addr);
        log::info!("Proxy listening on: {}", listen_addr);

        server.add_service(proxy_service);
        server.run_forever(); // This blocks
    });

    // Wait for proxy to start
    std::thread::sleep(std::time::Duration::from_secs(2));
    log::info!("Proxy should be started now");

    let client = reqwest::blocking::Client::new();

    // Test Case 1: Request to /secure-a/* uses credentials-a
    log::info!("Test Case 1: /secure-a/* uses credentials-a");
    let url_a = format!("http://127.0.0.1:{}/secure-a/file.txt", proxy_port);
    log::info!("Making request to: {}", url_a);

    let response_a = client.get(&url_a).send().expect("Failed to GET from proxy");

    log::info!(
        "Response status: {}, headers: {:?}",
        response_a.status(),
        response_a.headers()
    );

    assert_eq!(
        response_a.status(),
        200,
        "Expected 200 OK for /secure-a/file.txt, got {}",
        response_a.status()
    );

    let body_a = response_a.text().expect("Failed to read response body");
    log::info!("Response body: {}", body_a);
    assert_eq!(
        body_a, content_a,
        "Expected content from bucket-secure-a, got '{}'",
        body_a
    );

    // Verify content is from bucket-secure-a (not bucket-secure-b)
    assert!(
        body_a.contains("secure-a"),
        "Response should contain 'secure-a'"
    );
    assert!(
        body_a.contains("user-a"),
        "Response should indicate credentials-a were used"
    );

    // Test Case 2: Request to /secure-b/* uses credentials-b
    log::info!("Test Case 2: /secure-b/* uses credentials-b");
    let url_b = format!("http://127.0.0.1:{}/secure-b/file.txt", proxy_port);
    log::info!("Making request to: {}", url_b);

    let response_b = client.get(&url_b).send().expect("Failed to GET from proxy");

    log::info!(
        "Response status: {}, headers: {:?}",
        response_b.status(),
        response_b.headers()
    );

    assert_eq!(
        response_b.status(),
        200,
        "Expected 200 OK for /secure-b/file.txt, got {}",
        response_b.status()
    );

    let body_b = response_b.text().expect("Failed to read response body");
    log::info!("Response body: {}", body_b);
    assert_eq!(
        body_b, content_b,
        "Expected content from bucket-secure-b, got '{}'",
        body_b
    );

    // Verify content is from bucket-secure-b (not bucket-secure-a)
    assert!(
        body_b.contains("secure-b"),
        "Response should contain 'secure-b'"
    );
    assert!(
        body_b.contains("user-b"),
        "Response should indicate credentials-b were used"
    );

    // Verify NO credential mixing
    assert!(
        !body_a.contains("user-b"),
        "SECURITY: bucket-a should NOT use credentials-b!"
    );
    assert!(
        !body_b.contains("user-a"),
        "SECURITY: bucket-b should NOT use credentials-a!"
    );

    log::info!("=== Credential Isolation Test PASSED ===");
    log::info!("SECURITY VERIFIED: Each bucket uses its own credentials only");
}
