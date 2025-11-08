// Integration tests for request timeout handling

use reqwest::Client;
use std::net::{SocketAddr, TcpListener};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as TokioTcpListener;

/// Helper function to find an available port
fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Create a slow S3 mock server that delays responses
async fn create_slow_s3_server(port: u16, delay_seconds: u64) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = TokioTcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
            .await
            .unwrap();

        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let delay = delay_seconds;
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 1024];

                    // Read the request (don't care about content)
                    let _ = socket.read(&mut buffer).await;

                    // Delay before responding
                    tokio::time::sleep(Duration::from_secs(delay)).await;

                    // Send a valid HTTP 200 response (but this should timeout before it arrives)
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        }
    })
}

#[tokio::test]
#[ignore] // Requires running proxy server
async fn test_slow_s3_request_returns_502_bad_gateway() {
    // Create a slow S3 mock server (10 second delay)
    let s3_port = get_available_port();
    let _slow_server = create_slow_s3_server(s3_port, 10).await;

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create proxy configuration with 1 second S3 timeout
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1"
  port: 18080
  request_timeout: 30

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
      endpoint: "http://127.0.0.1:{}"
      timeout: 1
"#,
        s3_port
    );

    // Write config to temporary file
    let config_path = "/tmp/yatagarasu-timeout-test.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config");

    // Start proxy server in background (assuming it's already running for integration tests)
    // In real integration tests, we would start the actual proxy server here

    // Create HTTP client with long timeout (we're testing server timeout, not client timeout)
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    // Make request to proxy
    let response = client
        .get("http://127.0.0.1:18080/test/sample.txt")
        .send()
        .await
        .expect("Request failed");

    // Verify response status is 502 Bad Gateway
    // Note: Pingora returns 502 for connection timeouts, not 504
    // This is standard behavior for proxies when upstream connection fails
    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_GATEWAY,
        "Slow S3 response should return 502 Bad Gateway (Pingora's timeout behavior)"
    );

    // Cleanup
    std::fs::remove_file(config_path).ok();
}

#[tokio::test]
#[ignore] // Requires running proxy server
async fn test_fast_s3_request_completes_within_timeout() {
    // Create a fast S3 mock server (immediate response)
    let s3_port = get_available_port();
    let server_handle = tokio::spawn(async move {
        let listener = TokioTcpListener::bind(SocketAddr::from(([127, 0, 0, 1], s3_port)))
            .await
            .unwrap();

        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 1024];
                    let _ = socket.read(&mut buffer).await;

                    // Respond immediately
                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create proxy configuration with 5 second S3 timeout
    let config_yaml = format!(
        r#"
server:
  address: "127.0.0.1"
  port: 18080
  request_timeout: 30

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
      endpoint: "http://127.0.0.1:{}"
      timeout: 5
"#,
        s3_port
    );

    let config_path = "/tmp/yatagarasu-timeout-fast.yaml";
    std::fs::write(config_path, config_yaml).expect("Failed to write config");

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Make request to proxy
    let response = client
        .get("http://127.0.0.1:18080/test/sample.txt")
        .send()
        .await
        .expect("Request failed");

    // Verify response status is 200 OK (completed within timeout)
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Fast S3 response should complete successfully"
    );

    // Cleanup
    server_handle.abort();
    std::fs::remove_file(config_path).ok();
}

#[test]
fn test_timeout_configuration_is_applied() {
    use yatagarasu::config::Config;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
  request_timeout: 45

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
      timeout: 15
"#;

    let config = Config::from_yaml_with_env(yaml).unwrap();

    // Verify timeouts are loaded correctly
    assert_eq!(
        config.server.request_timeout, 45,
        "ServerConfig request_timeout should be 45"
    );
    assert_eq!(
        config.buckets[0].s3.timeout, 15,
        "S3Config timeout should be 15"
    );
}
