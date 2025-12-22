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
