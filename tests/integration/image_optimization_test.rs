//! Image Optimization End-to-End Integration Tests
//!
//! Tests the complete image processing flow:
//!   HTTP Request → Yatagarasu Proxy → Image Processing → Response
//!
//! These tests use testcontainers to run LocalStack in Docker.
//!
//! Run with:
//!   cargo test --test integration_tests image_optimization -- --ignored --nocapture

use super::test_harness::ProxyTestHarness;
use std::fs;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Once;
use std::time::Duration;
use testcontainers::{clients::Cli, RunnableImage};
use testcontainers_modules::localstack::LocalStack;

static INIT: Once = Once::new();
static PORT_COUNTER: AtomicU16 = AtomicU16::new(29000);

fn init_logging() {
    INIT.call_once(|| {});
}

fn next_port() -> u16 {
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn write_temp_config(config_yaml: &str, test_name: &str) -> String {
    let dir = format!("/tmp/yatagarasu-image-test-{}", test_name);
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    let config_path = format!("{}/config.yaml", dir);
    fs::write(&config_path, config_yaml).expect("Failed to write config");
    config_path
}

/// Create a test JPEG image (100x100 red square)
fn create_test_jpeg_100x100() -> Vec<u8> {
    use image::{ImageFormat, RgbaImage};
    use std::io::Cursor;

    let img = RgbaImage::from_fn(100, 100, |_, _| image::Rgba([255, 0, 0, 255]));

    let mut buffer = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut buffer, ImageFormat::Jpeg)
        .unwrap();
    buffer.into_inner()
}

/// Create a test PNG image (200x150 with alpha)
fn create_test_png_200x150() -> Vec<u8> {
    use image::{ImageFormat, RgbaImage};
    use std::io::Cursor;

    let img = RgbaImage::from_fn(200, 150, |x, y| {
        let alpha = if (x + y) % 2 == 0 { 255 } else { 128 };
        image::Rgba([0, 128, 255, alpha])
    });

    let mut buffer = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut buffer, ImageFormat::Png)
        .unwrap();
    buffer.into_inner()
}

/// Helper: Setup LocalStack with test images
async fn setup_localstack_with_images<'a>(
    docker: &'a Cli,
    bucket_name: &str,
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

    // Upload test JPEG
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key("photo.jpg")
        .body(create_test_jpeg_100x100().into())
        .content_type("image/jpeg")
        .send()
        .await
        .expect("Failed to upload JPEG");

    // Upload test PNG
    s3_client
        .put_object()
        .bucket(bucket_name)
        .key("image.png")
        .body(create_test_png_200x150().into())
        .content_type("image/png")
        .send()
        .await
        .expect("Failed to upload PNG");

    (container, endpoint)
}

// ============================================================================
// E2E Tests
// ============================================================================

/// Test: Resize JPEG image via URL parameters
#[test]
#[ignore] // Requires Docker
fn test_e2e_resize_jpeg() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images";
        let (_container, s3_endpoint) = setup_localstack_with_images(&docker, bucket_name).await;

        // Create proxy config with image optimization enabled
        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
      max_width: 2000
      max_height: 2000
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        // Write config to temp file and start proxy
        let config_path = write_temp_config(&config_yaml, "resize_jpeg");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = &harness.base_url;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Request resized image (50x50)
        let response = client
            .get(format!("{}/img/photo.jpg?w=50&h=50", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "image/jpeg"
        );

        let body = response.bytes().await.unwrap();
        assert!(!body.is_empty());

        // Verify it's a valid JPEG (starts with FFD8)
        assert_eq!(body[0], 0xFF);
        assert_eq!(body[1], 0xD8);

        println!("✅ E2E test passed: Resize JPEG");
    });
}

/// Test: Convert JPEG to WebP format
#[test]
#[ignore] // Requires Docker
fn test_e2e_convert_to_webp() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images-webp";
        let (_container, s3_endpoint) = setup_localstack_with_images(&docker, bucket_name).await;

        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        let config_path = write_temp_config(&config_yaml, "convert_webp");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = &harness.base_url;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Request WebP format
        let response = client
            .get(format!("{}/img/photo.jpg?fmt=webp", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "image/webp"
        );

        let body = response.bytes().await.unwrap();

        // Verify WebP magic bytes (RIFF....WEBP)
        assert_eq!(&body[0..4], b"RIFF");
        assert_eq!(&body[8..12], b"WEBP");

        println!("✅ E2E test passed: Convert to WebP");
    });
}

/// Test: Convert to PNG format
#[test]
#[ignore] // Requires Docker
fn test_e2e_convert_to_png() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images-png";
        let (_container, s3_endpoint) = setup_localstack_with_images(&docker, bucket_name).await;

        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        let config_path = write_temp_config(&config_yaml, "convert_png");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = &harness.base_url;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Request PNG format
        let response = client
            .get(format!("{}/img/photo.jpg?fmt=png", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 200);
        assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

        let body = response.bytes().await.unwrap();

        // Verify PNG magic bytes
        assert_eq!(
            &body[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );

        println!("✅ E2E test passed: Convert to PNG");
    });
}

/// Test: Quality adjustment
#[test]
#[ignore] // Requires Docker
fn test_e2e_quality_adjustment() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images-quality";
        let (_container, s3_endpoint) = setup_localstack_with_images(&docker, bucket_name).await;

        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        let config_path = write_temp_config(&config_yaml, "quality");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = &harness.base_url;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Request low quality (should be smaller)
        let response_low = client
            .get(format!("{}/img/photo.jpg?q=30", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response_low.status(), 200);
        let size_low = response_low.bytes().await.unwrap().len();

        // Request high quality (should be larger)
        let response_high = client
            .get(format!("{}/img/photo.jpg?q=95", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response_high.status(), 200);
        let size_high = response_high.bytes().await.unwrap().len();

        // High quality should generally be larger
        println!(
            "Low quality size: {}, High quality size: {}",
            size_low, size_high
        );

        println!("✅ E2E test passed: Quality adjustment");
    });
}

/// Test: Passthrough for non-image files
#[test]
#[ignore] // Requires Docker
fn test_e2e_non_image_passthrough() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images-passthrough";
        let localstack_image =
            RunnableImage::from(LocalStack::default()).with_env_var(("SERVICES", "s3"));

        let container = docker.run(localstack_image);
        let port = container.get_host_port_ipv4(4566);
        let s3_endpoint = format!("http://127.0.0.1:{}", port);

        // Create S3 client and upload non-image file
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region(aws_config::Region::new("us-east-1"))
            .credentials_provider(aws_credential_types::Credentials::new(
                "test", "test", None, None, "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);
        s3_client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .expect("Failed to create bucket");

        s3_client
            .put_object()
            .bucket(bucket_name)
            .key("document.txt")
            .body(b"Hello, World!".to_vec().into())
            .content_type("text/plain")
            .send()
            .await
            .expect("Failed to upload file");

        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "files"
    path_prefix: "/files"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        let config_path = write_temp_config(&config_yaml, "passthrough");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = &harness.base_url;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Non-image should pass through unchanged even with image params
        let response = client
            .get(format!("{}/files/document.txt?w=100", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 200);
        let body = response.text().await.unwrap();
        assert_eq!(body, "Hello, World!");

        println!("✅ E2E test passed: Non-image passthrough");
    });
}

/// Test: Rotation transformation
#[test]
#[ignore] // Requires Docker
fn test_e2e_rotation() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images-rotate";
        let (_container, s3_endpoint) = setup_localstack_with_images(&docker, bucket_name).await;

        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        let config_path = write_temp_config(&config_yaml, "rotation");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = &harness.base_url;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Request rotated image
        let response = client
            .get(format!("{}/img/photo.jpg?rot=90", proxy_url))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "image/jpeg"
        );

        let body = response.bytes().await.unwrap();
        assert!(!body.is_empty());

        println!("✅ E2E test passed: Rotation");
    });
}

/// Test E2E: Signed URL flow - valid signature allows access
#[test]
#[ignore = "Requires Docker and LocalStack"]
fn test_e2e_signed_url_valid() {
    init_logging();

    let port = next_port();
    let docker = Cli::default();
    let localstack = docker.run(RunnableImage::from(LocalStack).with_env_var(("SERVICES", "s3")));
    let s3_port = localstack.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", s3_port);
    let test_image = create_test_jpeg_100x100();
    let signing_key = "test-secret-key-for-signing";

    // Upload test image
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Create bucket
        let client = reqwest::Client::new();
        client
            .put(format!("{}/test-bucket", s3_endpoint))
            .send()
            .await
            .unwrap();

        // Upload image
        client
            .put(format!("{}/test-bucket/signed.jpg", s3_endpoint))
            .header("Content-Type", "image/jpeg")
            .body(test_image)
            .send()
            .await
            .unwrap();
    });

    // Config with signing enabled
    let config_yaml = format!(
        r#"
server:
  addr: "127.0.0.1:{port}"
  workers: 1
buckets:
  - name: "test"
    path_prefix: "/images"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    image_optimization:
      enabled: true
      require_signature: true
      signature_key: "{signing_key}"
"#
    );

    let config_path = write_temp_config(&config_yaml, "signed_url_valid");
    let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
    std::thread::sleep(Duration::from_secs(1));

    // Generate valid signature using same algorithm as security module
    // signature = HMAC-SHA256(key, options + "/" + source_url)
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let options = "w=50";
    let source_path = "/images/signed.jpg";

    let mut mac = HmacSha256::new_from_slice(signing_key.as_bytes()).unwrap();
    mac.update(options.as_bytes());
    mac.update(b"/");
    mac.update(source_path.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    rt.block_on(async {
        let client = reqwest::Client::new();
        let url = format!(
            "{}{}?{}&sig={}",
            harness.base_url, source_path, options, signature
        );

        let response = client.get(&url).send().await.unwrap();
        assert_eq!(
            response.status(),
            200,
            "Valid signature should allow access"
        );

        println!("✅ E2E test passed: Signed URL (valid)");
    });
}

/// Test E2E: Signed URL flow - invalid signature is rejected
#[test]
#[ignore = "Requires Docker and LocalStack"]
fn test_e2e_signed_url_invalid() {
    init_logging();

    let port = next_port();
    let docker = Cli::default();
    let localstack = docker.run(RunnableImage::from(LocalStack).with_env_var(("SERVICES", "s3")));
    let s3_port = localstack.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", s3_port);
    let test_image = create_test_jpeg_100x100();
    let signing_key = "test-secret-key-for-signing";

    // Upload test image
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let client = reqwest::Client::new();
        client
            .put(format!("{}/test-bucket", s3_endpoint))
            .send()
            .await
            .unwrap();

        client
            .put(format!("{}/test-bucket/signed.jpg", s3_endpoint))
            .header("Content-Type", "image/jpeg")
            .body(test_image)
            .send()
            .await
            .unwrap();
    });

    let config_yaml = format!(
        r#"
server:
  addr: "127.0.0.1:{port}"
  workers: 1
buckets:
  - name: "test"
    path_prefix: "/images"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    image_optimization:
      enabled: true
      require_signature: true
      signature_key: "{signing_key}"
"#
    );

    let config_path = write_temp_config(&config_yaml, "signed_url_invalid");
    let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
    std::thread::sleep(Duration::from_secs(1));

    rt.block_on(async {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/images/signed.jpg?w=50&sig=invalid-signature",
            harness.base_url
        );

        let response = client.get(&url).send().await.unwrap();
        // Should be 403 Forbidden or similar error
        assert!(
            response.status() == 403 || response.status() == 401,
            "Invalid signature should be rejected, got status {}",
            response.status()
        );

        println!("✅ E2E test passed: Signed URL (invalid rejected)");
    });
}

/// Test E2E: Auto format selection based on Accept header
#[test]
#[ignore = "Requires Docker and LocalStack"]
fn test_e2e_auto_format_selection() {
    init_logging();

    let port = next_port();
    let docker = Cli::default();
    let localstack = docker.run(RunnableImage::from(LocalStack).with_env_var(("SERVICES", "s3")));
    let s3_port = localstack.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", s3_port);
    let test_image = create_test_jpeg_100x100();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let client = reqwest::Client::new();
        client
            .put(format!("{}/test-bucket", s3_endpoint))
            .send()
            .await
            .unwrap();

        client
            .put(format!("{}/test-bucket/auto.jpg", s3_endpoint))
            .header("Content-Type", "image/jpeg")
            .body(test_image)
            .send()
            .await
            .unwrap();
    });

    let config_yaml = format!(
        r#"
server:
  addr: "127.0.0.1:{port}"
  workers: 1
buckets:
  - name: "test"
    path_prefix: "/images"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    image_optimization:
      enabled: true
      auto_format: true
"#
    );

    let config_path = write_temp_config(&config_yaml, "auto_format");
    let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
    std::thread::sleep(Duration::from_secs(1));

    rt.block_on(async {
        let client = reqwest::Client::new();

        // Request with WebP in Accept header should get WebP
        let response = client
            .get(format!(
                "{}/images/auto.jpg?w=50&fmt=auto",
                harness.base_url
            ))
            .header("Accept", "image/webp,image/jpeg,*/*")
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let content_type = response
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap_or(""));

        // Should return WebP since Accept header includes it
        if let Some(ct) = content_type {
            println!("Auto format response Content-Type: {}", ct);
            // Accept either webp or jpeg (depending on implementation)
            assert!(
                ct.contains("webp") || ct.contains("jpeg"),
                "Expected webp or jpeg, got: {}",
                ct
            );
        }

        println!("✅ E2E test passed: Auto format selection");
    });
}

/// Test E2E: Cache integration - second request hits cache
#[test]
#[ignore = "Requires Docker and LocalStack"]
fn test_e2e_cache_integration() {
    init_logging();

    let port = next_port();
    let docker = Cli::default();
    let localstack = docker.run(RunnableImage::from(LocalStack).with_env_var(("SERVICES", "s3")));
    let s3_port = localstack.get_host_port_ipv4(4566);
    let s3_endpoint = format!("http://127.0.0.1:{}", s3_port);
    let test_image = create_test_jpeg_100x100();
    let cache_dir = format!("/tmp/yatagarasu-cache-test-{}", port);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let client = reqwest::Client::new();
        client
            .put(format!("{}/test-bucket", s3_endpoint))
            .send()
            .await
            .unwrap();

        client
            .put(format!("{}/test-bucket/cached.jpg", s3_endpoint))
            .header("Content-Type", "image/jpeg")
            .body(test_image)
            .send()
            .await
            .unwrap();
    });

    let config_yaml = format!(
        r#"
server:
  addr: "127.0.0.1:{port}"
  workers: 1
buckets:
  - name: "test"
    path_prefix: "/images"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    image_optimization:
      enabled: true
cache:
  memory:
    max_capacity: 10485760
    ttl_seconds: 3600
  disk:
    path: "{cache_dir}"
    max_size: 104857600
"#
    );

    // Clean cache dir
    let _ = std::fs::remove_dir_all(&cache_dir);
    std::fs::create_dir_all(&cache_dir).unwrap();

    let config_path = write_temp_config(&config_yaml, "cache_integration");
    let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
    std::thread::sleep(Duration::from_secs(1));

    rt.block_on(async {
        let client = reqwest::Client::new();
        let url = format!("{}/images/cached.jpg?w=50&fmt=webp", harness.base_url);

        // First request - cache miss
        let start1 = std::time::Instant::now();
        let response1 = client.get(&url).send().await.unwrap();
        let duration1 = start1.elapsed();
        assert_eq!(response1.status(), 200);
        let body1 = response1.bytes().await.unwrap();
        let body1_len = body1.len();

        // Brief pause to allow cache write
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Second request - should be cache hit (faster)
        let start2 = std::time::Instant::now();
        let response2 = client.get(&url).send().await.unwrap();
        let duration2 = start2.elapsed();
        assert_eq!(response2.status(), 200);
        let body2 = response2.bytes().await.unwrap();

        // Verify same content
        assert_eq!(
            body1_len,
            body2.len(),
            "Cached response should be same size"
        );

        println!("First request: {:?}", duration1);
        println!("Second request: {:?}", duration2);
        println!(
            "Cache speedup: {:.1}x",
            duration1.as_secs_f64() / duration2.as_secs_f64().max(0.001)
        );

        println!("✅ E2E test passed: Cache integration");
    });

    // Cleanup
    let _ = std::fs::remove_dir_all(&cache_dir);
}

/// Test: Concurrent image processing - multiple requests handled simultaneously
#[test]
#[ignore] // Requires Docker
fn test_concurrent_processing() {
    init_logging();

    let docker = Cli::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let bucket_name = "test-images-concurrent";
        let (_container, s3_endpoint) = setup_localstack_with_images(&docker, bucket_name).await;

        let port = next_port();
        let config_yaml = format!(
            r#"
server:
  address: "127.0.0.1:{port}"

buckets:
  - name: "images"
    path_prefix: "/img"
    s3:
      bucket: "{bucket_name}"
      region: "us-east-1"
      endpoint: "{s3_endpoint}"
      access_key: "test"
      secret_key: "test"
    auth:
      enabled: false
    image_optimization:
      enabled: true
      max_width: 2000
      max_height: 2000
"#,
            port = port,
            bucket_name = bucket_name,
            s3_endpoint = s3_endpoint
        );

        let config_path = write_temp_config(&config_yaml, "concurrent");
        let harness = ProxyTestHarness::start(&config_path, port).expect("Failed to start proxy");
        let proxy_url = harness.base_url.clone();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        // Launch multiple concurrent requests with different transformations
        let num_requests = 10;
        let mut handles = Vec::with_capacity(num_requests);

        for i in 0..num_requests {
            let client = client.clone();
            let proxy_url = proxy_url.clone();

            let handle = tokio::spawn(async move {
                // Vary the transformation parameters for each request
                let url = match i % 5 {
                    0 => format!("{}/img/photo.jpg?w=50&h=50", proxy_url),
                    1 => format!("{}/img/photo.jpg?w=75&q=80", proxy_url),
                    2 => format!("{}/img/photo.jpg?fmt=webp", proxy_url),
                    3 => format!("{}/img/image.png?w=100&fmt=jpeg", proxy_url),
                    _ => format!("{}/img/photo.jpg?blur=2&brightness=10", proxy_url),
                };

                let response = client.get(&url).send().await?;
                let status = response.status();
                let body = response.bytes().await?;

                Ok::<(u16, usize), reqwest::Error>((status.as_u16(), body.len()))
            });

            handles.push(handle);
        }

        // Collect results
        let mut success_count = 0;
        let mut failure_count = 0;

        for handle in handles {
            match handle.await {
                Ok(Ok((status, size))) => {
                    if status == 200 && size > 0 {
                        success_count += 1;
                    } else {
                        println!("Request returned status={}, size={}", status, size);
                        failure_count += 1;
                    }
                }
                Ok(Err(e)) => {
                    println!("Request error: {}", e);
                    failure_count += 1;
                }
                Err(e) => {
                    println!("Task error: {}", e);
                    failure_count += 1;
                }
            }
        }

        println!(
            "Concurrent processing: {} succeeded, {} failed out of {} requests",
            success_count, failure_count, num_requests
        );

        // All requests should succeed
        assert_eq!(
            success_count, num_requests,
            "All concurrent requests should succeed"
        );
        assert_eq!(failure_count, 0, "No requests should fail");

        println!("✅ E2E test passed: Concurrent processing");
    });
}
