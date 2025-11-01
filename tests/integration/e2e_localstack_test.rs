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
