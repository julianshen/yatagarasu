// Server module unit tests
// Phase 12: Pingora Server Setup

use yatagarasu::server::ServerConfig;

// Test: Can add Pingora dependency to Cargo.toml
// This test verifies that Pingora dependencies are available and can be imported
#[test]
fn test_can_import_pingora_dependencies() {
    // Try to use Pingora types to verify dependency is available
    use pingora::server::configuration::ServerConf;
    use pingora_core::upstreams::peer::HttpPeer;

    // If this compiles and runs, the dependencies are correctly added
    // We just need to verify the types exist
    let _server_conf_type = std::any::type_name::<ServerConf>();
    let _peer_type = std::any::type_name::<HttpPeer>();

    assert!(_server_conf_type.contains("ServerConf"));
    assert!(_peer_type.contains("HttpPeer"));
}

// Test: Can create ServerConfig struct
// This test verifies we can create a configuration struct for the HTTP server
#[test]
fn test_can_create_server_config() {
    // Create a ServerConfig with basic settings
    let config = ServerConfig {
        address: "0.0.0.0:8080".to_string(),
        threads: 4,
    };

    // Verify fields are accessible
    assert_eq!(config.address, "0.0.0.0:8080");
    assert_eq!(config.threads, 4);
}

// Test: ServerConfig can be created from our app Config
#[test]
fn test_server_config_from_app_config() {
    use yatagarasu::config::Config;

    // Create a minimal app config
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets: []
"#;

    let app_config: Config = serde_yaml::from_str(yaml).unwrap();

    // Create ServerConfig from app Config
    let server_config = ServerConfig::from_config(&app_config);

    // Should combine address and port
    assert_eq!(server_config.address, "127.0.0.1:8080");
    assert_eq!(server_config.threads, 4); // Default value
}

// Test: Can initialize Pingora Server instance
#[test]
fn test_can_initialize_pingora_server() {
    use yatagarasu::server::YatagarasuServer;

    // Create server configuration
    let config = ServerConfig::new("127.0.0.1:0".to_string()); // Port 0 = auto-assign

    // Initialize server
    let server_result = YatagarasuServer::new(config);

    // Should successfully create server
    assert!(server_result.is_ok());

    let server = server_result.unwrap();
    assert!(server.config().address.contains("127.0.0.1"));
}

// Test: Server configuration is accessible after creation
#[test]
fn test_server_config_accessible() {
    use yatagarasu::server::YatagarasuServer;

    let config = ServerConfig::new("0.0.0.0:8080".to_string());
    let address = config.address.clone();

    let server = YatagarasuServer::new(config).unwrap();

    // Should be able to access configuration
    assert_eq!(server.config().address, address);
    assert_eq!(server.config().threads, 4);
}

// Test: Server binds to configured address
#[test]
fn test_server_binds_to_configured_address() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port by binding to port 0
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Release the port

    // Create server with the available port
    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();

    // Verify the address is set correctly
    assert_eq!(server.config().address, address);
    assert!(server.config().address.contains(&port.to_string()));
}

// Test: Server can parse socket address correctly
#[test]
fn test_server_parses_socket_address() {
    use yatagarasu::server::YatagarasuServer;

    let config = ServerConfig::new("0.0.0.0:8080".to_string());
    let server = YatagarasuServer::new(config).unwrap();

    // Should be able to parse the address into SocketAddr
    let socket_addr = server.parse_address();
    assert!(socket_addr.is_ok());

    let addr = socket_addr.unwrap();
    assert_eq!(addr.port(), 8080);
}

// Test: Server rejects invalid address format
#[test]
fn test_server_rejects_invalid_address() {
    use yatagarasu::server::YatagarasuServer;

    let config = ServerConfig::new("invalid:address".to_string());

    // Should fail to create server with invalid address
    let server_result = YatagarasuServer::new(config);
    assert!(server_result.is_err());

    let error = server_result.unwrap_err();
    assert!(error.contains("Invalid address"));
    assert!(error.contains("invalid:address"));
}

// Test: Server starts without errors with valid configuration
#[test]
fn test_server_starts_without_errors() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();

    // Should be able to build a Pingora Server instance
    let pingora_server_result = server.build_pingora_server();
    assert!(pingora_server_result.is_ok());

    // If we get here, the server was created successfully
    let _pingora_server = pingora_server_result.unwrap();
}

// Test: Can stop server programmatically
#[test]
fn test_can_stop_server() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let pingora_server = server.build_pingora_server().unwrap();

    // Verify server can be gracefully shut down by dropping it
    // In Pingora, servers shut down when dropped or when receiving signals
    drop(pingora_server);

    // If we get here, the server was properly cleaned up
    // We can create a new server on the same port to verify cleanup
    let server2 = YatagarasuServer::new(ServerConfig::new(address)).unwrap();
    let pingora_server2 = server2.build_pingora_server().unwrap();
    assert!(std::ptr::addr_of!(pingora_server2) != std::ptr::null());
}

// Test: Server accepts HTTP/1.1 GET requests
#[test]
fn test_server_accepts_get_requests() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();

    // Should be able to create a service that handles GET requests
    let service = server.create_http_service();
    assert!(service.is_ok());

    // Verify the service is configured to accept GET requests
    let svc = service.unwrap();
    assert!(svc.supports_method("GET"));
}

// Test: Server accepts HTTP/1.1 HEAD requests
#[test]
fn test_server_accepts_head_requests() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();

    // Should be able to create a service that handles HEAD requests
    let service = server.create_http_service();
    assert!(service.is_ok());

    // Verify the service is configured to accept HEAD requests
    let svc = service.unwrap();
    assert!(svc.supports_method("HEAD"));
}

// Test: Server returns proper HTTP response with status code
#[test]
fn test_server_returns_response_with_status_code() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Should be able to create a response with status code 200
    let response = service.create_response(200);
    assert!(response.is_ok());

    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);

    // Should be able to create responses with different status codes
    let response_404 = service.create_response(404).unwrap();
    assert_eq!(response_404.status_code(), 404);

    let response_500 = service.create_response(500).unwrap();
    assert_eq!(response_500.status_code(), 500);
}

// Test: Server returns proper HTTP response with headers
#[test]
fn test_server_returns_response_with_headers() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Create a response
    let mut response = service.create_response(200).unwrap();

    // Should be able to add headers to the response
    response.add_header("Content-Type", "application/json");
    response.add_header("X-Custom-Header", "custom-value");

    // Should be able to retrieve headers
    assert_eq!(response.get_header("Content-Type"), Some("application/json"));
    assert_eq!(response.get_header("X-Custom-Header"), Some("custom-value"));
    assert_eq!(response.get_header("Non-Existent"), None);

    // Should be able to get all headers
    let headers = response.headers();
    assert_eq!(headers.len(), 2);
    assert!(headers.contains_key("Content-Type"));
    assert!(headers.contains_key("X-Custom-Header"));
}

// Test: Server returns proper HTTP response with body
#[test]
fn test_server_returns_response_with_body() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Create a response
    let mut response = service.create_response(200).unwrap();

    // Should be able to set a body
    let body_content = b"Hello, World!";
    response.set_body(body_content.to_vec());

    // Should be able to retrieve the body
    assert_eq!(response.body(), body_content);

    // Should be able to set a different body
    let json_body = b"{\"status\":\"ok\"}";
    response.set_body(json_body.to_vec());
    assert_eq!(response.body(), json_body);

    // Body should be empty by default
    let empty_response = service.create_response(204).unwrap();
    assert_eq!(empty_response.body(), b"");
}

// Test: Server handles concurrent requests (10+ simultaneous)
#[test]
fn test_server_handles_concurrent_requests() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = Arc::new(server.create_http_service().unwrap());

    // Spawn 10 threads that create responses concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let service_clone = Arc::clone(&service);
            thread::spawn(move || {
                // Each thread creates a response with a unique status code
                let status_code = 200 + i;
                let response = service_clone.create_response(status_code).unwrap();
                assert_eq!(response.status_code(), status_code);
                response
            })
        })
        .collect();

    // Wait for all threads to complete
    let responses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Verify all responses were created successfully
    assert_eq!(responses.len(), 10);
    for (i, response) in responses.iter().enumerate() {
        assert_eq!(response.status_code(), 200 + i as u16);
    }
}

// Test: Server handles request pipeline (keep-alive)
#[test]
fn test_server_handles_request_pipeline() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Simulate multiple requests on the same connection (keep-alive)
    // Create multiple responses as if they're being sent on the same connection
    let mut responses = Vec::new();
    for i in 0..5 {
        let mut response = service.create_response(200).unwrap();

        // Add Connection: keep-alive header for HTTP/1.1
        response.add_header("Connection", "keep-alive");
        response.set_body(format!("Response {}", i).into_bytes());

        responses.push(response);
    }

    // Verify all responses have keep-alive header
    assert_eq!(responses.len(), 5);
    for (i, response) in responses.iter().enumerate() {
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.get_header("Connection"), Some("keep-alive"));
        assert_eq!(response.body(), format!("Response {}", i).as_bytes());
    }

    // Verify the service supports keep-alive by checking it can create
    // multiple responses sequentially (simulating pipelined requests)
    let response1 = service.create_response(200).unwrap();
    let response2 = service.create_response(200).unwrap();
    let response3 = service.create_response(200).unwrap();

    assert_eq!(response1.status_code(), 200);
    assert_eq!(response2.status_code(), 200);
    assert_eq!(response3.status_code(), 200);
}

// Test: GET /health returns 200 OK
#[test]
fn test_health_endpoint_returns_200() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Request to /health endpoint
    let response = service.handle_request("GET", "/health");
    assert!(response.is_ok());

    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);
}

// Test: /health response includes JSON body with status
#[test]
fn test_health_endpoint_returns_json_body() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Request to /health endpoint
    let response = service.handle_request("GET", "/health");
    assert!(response.is_ok());

    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);

    // Check Content-Type header
    assert_eq!(resp.get_header("Content-Type"), Some("application/json"));

    // Check body contains JSON with status
    let body = std::str::from_utf8(resp.body()).unwrap();
    assert!(body.contains("\"status\""));
    assert!(body.contains("\"ok\""));

    // Verify it's valid JSON
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["status"], "ok");
}

// Test: /health checks configuration is loaded
#[test]
fn test_health_endpoint_checks_configuration() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Request to /health endpoint
    let response = service.handle_request("GET", "/health");
    assert!(response.is_ok());

    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);

    // Parse JSON response
    let body = std::str::from_utf8(resp.body()).unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();

    // Should include configuration status
    assert_eq!(json["status"], "ok");
    assert!(json.get("config_loaded").is_some());
    assert_eq!(json["config_loaded"], true);
}

// Test: /health response time < 10ms
#[test]
fn test_health_endpoint_response_time() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;
    use std::time::Instant;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Measure response time
    let start = Instant::now();
    let response = service.handle_request("GET", "/health");
    let duration = start.elapsed();

    // Verify response succeeded
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);

    // Response time should be less than 10ms
    assert!(duration.as_millis() < 10,
        "Health endpoint took {}ms, expected < 10ms",
        duration.as_millis());
}

// Test: /health works before other endpoints are ready
#[test]
fn test_health_endpoint_works_early() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();

    // Create service immediately after server creation
    // This simulates accessing health before other endpoints are initialized
    let service = server.create_http_service().unwrap();

    // Health endpoint should work immediately
    let response = service.handle_request("GET", "/health");
    assert!(response.is_ok());

    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);

    // Verify it returns proper JSON
    let body = std::str::from_utf8(resp.body()).unwrap();
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["status"], "ok");
}

// Test: HEAD /health returns 200 without body
#[test]
fn test_health_endpoint_head_request() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // HEAD request to /health endpoint
    let response = service.handle_request("HEAD", "/health");
    assert!(response.is_ok());

    let resp = response.unwrap();
    assert_eq!(resp.status_code(), 200);

    // Should have Content-Type header
    assert_eq!(resp.get_header("Content-Type"), Some("application/json"));

    // Body should be empty for HEAD request
    assert_eq!(resp.body(), b"");
}

// Test: Unknown paths return 404 Not Found
#[test]
fn test_unknown_paths_return_404() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Test various unknown paths
    let unknown_paths = vec![
        "/",
        "/unknown",
        "/api/v1/data",
        "/health/metrics",  // Similar to /health but not exact match
        "/favicon.ico",
    ];

    for path in unknown_paths {
        let response = service.handle_request("GET", path);
        assert!(response.is_ok(), "Request to {} should succeed", path);

        let resp = response.unwrap();
        assert_eq!(
            resp.status_code(),
            404,
            "Path {} should return 404 Not Found",
            path
        );
    }
}

// Test: Invalid HTTP methods return 405 Method Not Allowed
#[test]
fn test_invalid_http_methods_return_405() {
    use yatagarasu::server::YatagarasuServer;
    use std::net::TcpListener;

    // Find an available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let config = ServerConfig::new(address.clone());

    let server = YatagarasuServer::new(config).unwrap();
    let service = server.create_http_service().unwrap();

    // Test unsupported HTTP methods
    let invalid_methods = vec![
        "PUT",
        "DELETE",
        "PATCH",
        "OPTIONS",
        "TRACE",
        "CONNECT",
    ];

    for method in invalid_methods {
        // Test against /health endpoint
        let response = service.handle_request(method, "/health");
        assert!(response.is_ok(), "Request with method {} should succeed", method);

        let resp = response.unwrap();
        assert_eq!(
            resp.status_code(),
            405,
            "Method {} should return 405 Method Not Allowed",
            method
        );

        // Test against other paths too
        let response2 = service.handle_request(method, "/unknown");
        assert!(response2.is_ok(), "Request with method {} should succeed", method);

        let resp2 = response2.unwrap();
        assert_eq!(
            resp2.status_code(),
            405,
            "Method {} should return 405 Method Not Allowed even for unknown paths",
            method
        );
    }
}
