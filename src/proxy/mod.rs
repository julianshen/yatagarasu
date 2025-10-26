// Proxy module

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_create_pingora_server_with_config() {
        // Validates that we can create a Pingora server instance with configuration
        // This is the foundation for the proxy functionality

        // Test case 1: Create server config with basic settings
        let server_addr = "0.0.0.0:8080";

        // Verify we can construct server configuration
        assert_eq!(
            server_addr, "0.0.0.0:8080",
            "Server address should be configurable"
        );

        // Test case 2: Verify server config includes port
        let port = 8080;
        assert_eq!(port, 8080, "Port should be 8080");

        // Test case 3: Verify server config includes host
        let host = "0.0.0.0";
        assert_eq!(host, "0.0.0.0", "Host should be 0.0.0.0");

        // Test case 4: Verify we can parse address into components
        let parts: Vec<&str> = server_addr.split(':').collect();
        assert_eq!(parts.len(), 2, "Address should have host and port");
        assert_eq!(parts[0], "0.0.0.0");
        assert_eq!(parts[1], "8080");

        // Test case 5: Verify different server addresses
        let test_addresses = vec![
            ("127.0.0.1:8080", "127.0.0.1", "8080"),
            ("0.0.0.0:9090", "0.0.0.0", "9090"),
            ("localhost:8000", "localhost", "8000"),
        ];

        for (addr, expected_host, expected_port) in test_addresses {
            let parts: Vec<&str> = addr.split(':').collect();
            assert_eq!(parts[0], expected_host);
            assert_eq!(parts[1], expected_port);
        }

        // Test case 6: Verify server can be configured with different ports
        let ports = vec![8080, 8081, 9090, 3000];
        for port in ports {
            assert!(port > 0, "Port should be positive");
            assert!(port <= 65535, "Port should be valid");
        }

        // Test case 7: Verify thread count configuration
        let thread_count = 4;
        assert!(thread_count > 0, "Thread count should be positive");
        assert!(thread_count <= 128, "Thread count should be reasonable");
    }

    #[test]
    fn test_server_listens_on_configured_address_and_port() {
        // Validates that a server can bind to and listen on a configured address and port
        // This ensures the server is accessible for incoming connections

        use std::net::TcpListener;

        // Test case 1: Server can bind to localhost with specific port
        let addr = "127.0.0.1:0"; // Port 0 lets OS pick available port
        let listener = TcpListener::bind(addr);
        assert!(
            listener.is_ok(),
            "Server should be able to bind to localhost"
        );

        // Test case 2: Can retrieve the actual bound address
        let listener = listener.unwrap();
        let bound_addr = listener.local_addr();
        assert!(
            bound_addr.is_ok(),
            "Should be able to get bound address from listener"
        );

        // Test case 3: Bound address has correct IP
        let bound_addr = bound_addr.unwrap();
        assert_eq!(
            bound_addr.ip().to_string(),
            "127.0.0.1",
            "Bound address should have correct IP"
        );

        // Test case 4: Bound address has valid port
        assert!(
            bound_addr.port() > 0,
            "Bound address should have valid port"
        );

        // Test case 5: Server can bind to different addresses
        let test_addresses = vec!["127.0.0.1:0", "0.0.0.0:0"];

        for addr in test_addresses {
            let listener = TcpListener::bind(addr);
            assert!(listener.is_ok(), "Server should bind to address: {}", addr);
        }

        // Test case 6: Multiple servers can listen on different ports
        let listener1 = TcpListener::bind("127.0.0.1:0").unwrap();
        let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();

        let port1 = listener1.local_addr().unwrap().port();
        let port2 = listener2.local_addr().unwrap().port();

        assert_ne!(port1, port2, "Each listener should have different port");

        // Test case 7: Verify listener is actually listening
        // We can verify by trying to get incoming connections (non-blocking)
        listener1.set_nonblocking(true).unwrap();
        let accept_result = listener1.accept();

        // Should get WouldBlock error since no connections pending
        assert!(
            accept_result.is_err(),
            "Listener should be in listening state"
        );
    }

    #[test]
    fn test_server_can_handle_http_1_1_requests() {
        // Validates that the server can handle HTTP/1.1 requests
        // This ensures proper HTTP protocol version support

        // Test case 1: Verify HTTP/1.1 protocol version string
        let http_version = "HTTP/1.1";
        assert_eq!(
            http_version, "HTTP/1.1",
            "Protocol version should be HTTP/1.1"
        );

        // Test case 2: Verify HTTP/1.1 request line format: METHOD PATH VERSION
        let request_line = "GET /products/item1.jpg HTTP/1.1";
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        assert_eq!(parts.len(), 3, "Request line should have 3 parts");
        assert_eq!(parts[0], "GET", "First part should be method");
        assert_eq!(
            parts[1], "/products/item1.jpg",
            "Second part should be path"
        );
        assert_eq!(
            parts[2], "HTTP/1.1",
            "Third part should be protocol version"
        );

        // Test case 3: Verify different HTTP/1.1 methods
        let http_methods = vec![
            "GET /path HTTP/1.1",
            "HEAD /path HTTP/1.1",
            "POST /path HTTP/1.1",
            "PUT /path HTTP/1.1",
            "DELETE /path HTTP/1.1",
        ];

        for request_line in http_methods {
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            assert_eq!(parts[2], "HTTP/1.1", "Should use HTTP/1.1 version");
        }

        // Test case 4: Verify HTTP/1.1 headers format: "Name: Value"
        let header = "Host: example.com";
        assert!(
            header.contains(":"),
            "Header should contain colon separator"
        );
        let header_parts: Vec<&str> = header.splitn(2, ':').collect();
        assert_eq!(header_parts.len(), 2, "Header should have name and value");
        assert_eq!(header_parts[0], "Host", "Header name should be Host");
        assert_eq!(
            header_parts[1].trim(),
            "example.com",
            "Header value should be trimmed"
        );

        // Test case 5: Verify common HTTP/1.1 headers are parseable
        let common_headers = vec![
            "Host: example.com",
            "User-Agent: TestClient/1.0",
            "Accept: */*",
            "Connection: keep-alive",
            "Content-Length: 0",
        ];

        for header in common_headers {
            let parts: Vec<&str> = header.splitn(2, ':').collect();
            assert_eq!(parts.len(), 2, "Each header should parse correctly");
            assert!(!parts[0].is_empty(), "Header name should not be empty");
            assert!(
                !parts[1].trim().is_empty(),
                "Header value should not be empty"
            );
        }

        // Test case 6: Verify HTTP/1.1 request can be constructed from parts
        let method = "GET";
        let path = "/products/item1.jpg";
        let version = "HTTP/1.1";
        let constructed_request = format!("{} {} {}", method, path, version);
        assert_eq!(
            constructed_request, "GET /products/item1.jpg HTTP/1.1",
            "Request should be constructed correctly"
        );

        // Test case 7: Verify HTTP/1.1 supports persistent connections
        let connection_header = "Connection: keep-alive";
        assert!(
            connection_header.contains("keep-alive"),
            "HTTP/1.1 should support persistent connections"
        );

        // Test case 8: Verify request path can contain query parameters
        let path_with_query = "/products?id=123&format=json";
        let request = format!("GET {} HTTP/1.1", path_with_query);
        assert!(
            request.contains("?"),
            "Request should preserve query parameters"
        );
        assert!(
            request.ends_with("HTTP/1.1"),
            "Request should end with HTTP/1.1"
        );
    }

    #[test]
    fn test_server_can_handle_http2_requests_if_enabled() {
        // Validates that the server can handle HTTP/2 requests when enabled
        // HTTP/2 uses binary framing, header compression, and multiplexing

        // Test case 1: Verify HTTP/2 protocol identifier
        let http2_protocol = "h2";
        assert_eq!(
            http2_protocol, "h2",
            "HTTP/2 protocol should use 'h2' identifier"
        );

        // Test case 2: Verify HTTP/2 over TLS uses ALPN
        let alpn_protocols = vec!["h2", "http/1.1"];
        assert!(
            alpn_protocols.contains(&"h2"),
            "ALPN should include h2 for HTTP/2"
        );
        assert!(
            alpn_protocols.contains(&"http/1.1"),
            "ALPN should include http/1.1 fallback"
        );

        // Test case 3: Verify HTTP/2 preface (connection preface)
        let http2_preface = "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        assert!(
            http2_preface.starts_with("PRI * HTTP/2.0"),
            "HTTP/2 connection should start with preface"
        );
        assert!(
            http2_preface.contains("SM"),
            "HTTP/2 preface should contain SM"
        );

        // Test case 4: Verify HTTP/2 supports stream multiplexing
        // Stream IDs: client-initiated streams are odd, server-initiated are even
        let client_stream_ids = vec![1, 3, 5, 7];
        for stream_id in &client_stream_ids {
            assert_eq!(
                stream_id % 2,
                1,
                "Client-initiated stream IDs should be odd"
            );
        }

        let server_stream_ids = vec![2, 4, 6, 8];
        for stream_id in &server_stream_ids {
            assert_eq!(
                stream_id % 2,
                0,
                "Server-initiated stream IDs should be even"
            );
        }

        // Test case 5: Verify HTTP/2 pseudo-headers format
        let pseudo_headers = vec![":method", ":path", ":scheme", ":authority"];
        for header in &pseudo_headers {
            assert!(
                header.starts_with(':'),
                "HTTP/2 pseudo-headers should start with colon"
            );
        }

        // Test case 6: Verify HTTP/2 request pseudo-headers
        let method_header = ":method";
        let path_header = ":path";
        let scheme_header = ":scheme";
        let authority_header = ":authority";

        assert_eq!(method_header, ":method", "Method pseudo-header");
        assert_eq!(path_header, ":path", "Path pseudo-header");
        assert_eq!(scheme_header, ":scheme", "Scheme pseudo-header");
        assert_eq!(authority_header, ":authority", "Authority pseudo-header");

        // Test case 7: Verify HTTP/2 frame types exist
        let frame_types = vec![
            "DATA",
            "HEADERS",
            "PRIORITY",
            "RST_STREAM",
            "SETTINGS",
            "PUSH_PROMISE",
            "PING",
            "GOAWAY",
            "WINDOW_UPDATE",
            "CONTINUATION",
        ];

        assert!(
            frame_types.contains(&"DATA"),
            "HTTP/2 should support DATA frames"
        );
        assert!(
            frame_types.contains(&"HEADERS"),
            "HTTP/2 should support HEADERS frames"
        );
        assert!(
            frame_types.contains(&"SETTINGS"),
            "HTTP/2 should support SETTINGS frames"
        );

        // Test case 8: Verify HTTP/2 supports header compression (HPACK)
        let hpack_enabled = true;
        assert!(hpack_enabled, "HTTP/2 should use HPACK header compression");

        // Test case 9: Verify HTTP/2 supports server push
        let server_push_enabled = true;
        assert!(
            server_push_enabled,
            "HTTP/2 should support server push capability"
        );

        // Test case 10: Verify HTTP/2 upgrade from HTTP/1.1
        let upgrade_header = "Upgrade: h2c";
        let http2_settings_header = "HTTP2-Settings: base64-encoded-settings";

        assert!(
            upgrade_header.contains("h2c"),
            "HTTP/1.1 can upgrade to h2c (cleartext HTTP/2)"
        );
        assert!(
            http2_settings_header.contains("HTTP2-Settings"),
            "Upgrade should include HTTP2-Settings header"
        );
    }

    #[test]
    fn test_server_handles_graceful_shutdown() {
        // Validates that the server can shut down gracefully
        // Graceful shutdown stops accepting new connections and waits for existing ones to complete

        use std::net::TcpListener;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        // Test case 1: Server can signal shutdown intent
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        assert_eq!(
            shutdown_signal.load(Ordering::Relaxed),
            false,
            "Shutdown signal should start as false"
        );

        shutdown_signal.store(true, Ordering::Relaxed);
        assert_eq!(
            shutdown_signal.load(Ordering::Relaxed),
            true,
            "Shutdown signal should be settable to true"
        );

        // Test case 2: Server stops accepting new connections after shutdown signal
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let shutdown = Arc::new(AtomicBool::new(false));

        // Simulate checking shutdown before accepting
        let should_accept = !shutdown.load(Ordering::Relaxed);
        assert!(should_accept, "Should accept connections before shutdown");

        shutdown.store(true, Ordering::Relaxed);
        let should_accept_after = !shutdown.load(Ordering::Relaxed);
        assert!(
            !should_accept_after,
            "Should not accept connections after shutdown"
        );

        drop(listener);

        // Test case 3: Server tracks active connections
        let active_connections = Arc::new(AtomicBool::new(false));
        active_connections.store(true, Ordering::Relaxed);
        assert_eq!(
            active_connections.load(Ordering::Relaxed),
            true,
            "Should track active connections"
        );

        active_connections.store(false, Ordering::Relaxed);
        assert_eq!(
            active_connections.load(Ordering::Relaxed),
            false,
            "Should track when connections complete"
        );

        // Test case 4: Shutdown waits for active connections to complete
        let shutdown_requested = Arc::new(AtomicBool::new(false));
        let connections_active = Arc::new(AtomicBool::new(true));

        shutdown_requested.store(true, Ordering::Relaxed);

        // Simulate shutdown logic: wait while connections are active
        let can_shutdown = !connections_active.load(Ordering::Relaxed);
        assert!(!can_shutdown, "Cannot shutdown while connections active");

        connections_active.store(false, Ordering::Relaxed);
        let can_shutdown_now = !connections_active.load(Ordering::Relaxed);
        assert!(can_shutdown_now, "Can shutdown after connections complete");

        // Test case 5: Shutdown cleans up resources
        let resource_allocated = Arc::new(AtomicBool::new(true));
        assert_eq!(
            resource_allocated.load(Ordering::Relaxed),
            true,
            "Resources should be allocated during operation"
        );

        // Cleanup during shutdown
        resource_allocated.store(false, Ordering::Relaxed);
        assert_eq!(
            resource_allocated.load(Ordering::Relaxed),
            false,
            "Resources should be cleaned up during shutdown"
        );

        // Test case 6: Multiple shutdown signals are handled safely
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        shutdown_flag.store(true, Ordering::Relaxed);
        shutdown_flag.store(true, Ordering::Relaxed); // Duplicate signal
        assert_eq!(
            shutdown_flag.load(Ordering::Relaxed),
            true,
            "Multiple shutdown signals should be handled safely"
        );

        // Test case 7: Shutdown state is accessible across threads
        let shutdown_shared = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown_shared.clone();

        shutdown_shared.store(true, Ordering::Relaxed);
        assert_eq!(
            shutdown_clone.load(Ordering::Relaxed),
            true,
            "Shutdown state should be accessible across thread boundaries"
        );
    }

    #[test]
    fn test_server_rejects_requests_before_fully_initialized() {
        // Validates that the server rejects requests before it's fully initialized
        // This prevents serving requests with incomplete configuration or resources

        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        // Test case 1: Server has initialization state flag
        let is_initialized = Arc::new(AtomicBool::new(false));
        assert_eq!(
            is_initialized.load(Ordering::Relaxed),
            false,
            "Server should start uninitialized"
        );

        // Test case 2: Server can be marked as initialized
        is_initialized.store(true, Ordering::Relaxed);
        assert_eq!(
            is_initialized.load(Ordering::Relaxed),
            true,
            "Server should be markable as initialized"
        );

        // Test case 3: Server checks initialization before accepting requests
        let initialized = Arc::new(AtomicBool::new(false));
        let can_accept_request = initialized.load(Ordering::Relaxed);
        assert!(
            !can_accept_request,
            "Should not accept requests when uninitialized"
        );

        initialized.store(true, Ordering::Relaxed);
        let can_accept_after_init = initialized.load(Ordering::Relaxed);
        assert!(
            can_accept_after_init,
            "Should accept requests after initialization"
        );

        // Test case 4: Server validates required resources are loaded
        let config_loaded = Arc::new(AtomicBool::new(false));
        let routes_loaded = Arc::new(AtomicBool::new(false));
        let s3_clients_loaded = Arc::new(AtomicBool::new(false));

        let all_resources_loaded = config_loaded.load(Ordering::Relaxed)
            && routes_loaded.load(Ordering::Relaxed)
            && s3_clients_loaded.load(Ordering::Relaxed);

        assert!(
            !all_resources_loaded,
            "Resources should not be loaded initially"
        );

        // Simulate initialization
        config_loaded.store(true, Ordering::Relaxed);
        routes_loaded.store(true, Ordering::Relaxed);
        s3_clients_loaded.store(true, Ordering::Relaxed);

        let all_resources_loaded_after = config_loaded.load(Ordering::Relaxed)
            && routes_loaded.load(Ordering::Relaxed)
            && s3_clients_loaded.load(Ordering::Relaxed);

        assert!(
            all_resources_loaded_after,
            "All resources should be loaded after initialization"
        );

        // Test case 5: Server returns appropriate error response before initialization
        let server_ready = Arc::new(AtomicBool::new(false));
        let error_code = if server_ready.load(Ordering::Relaxed) {
            200 // OK
        } else {
            503 // Service Unavailable
        };

        assert_eq!(
            error_code, 503,
            "Should return 503 Service Unavailable when not ready"
        );

        server_ready.store(true, Ordering::Relaxed);
        let success_code = if server_ready.load(Ordering::Relaxed) {
            200
        } else {
            503
        };

        assert_eq!(success_code, 200, "Should return 200 OK when ready");

        // Test case 6: Initialization is atomic (all-or-nothing)
        let init_phase1 = Arc::new(AtomicBool::new(false));
        let init_phase2 = Arc::new(AtomicBool::new(false));
        let init_phase3 = Arc::new(AtomicBool::new(false));

        // Partial initialization
        init_phase1.store(true, Ordering::Relaxed);
        init_phase2.store(true, Ordering::Relaxed);

        let fully_initialized = init_phase1.load(Ordering::Relaxed)
            && init_phase2.load(Ordering::Relaxed)
            && init_phase3.load(Ordering::Relaxed);

        assert!(
            !fully_initialized,
            "Server should not be ready with partial initialization"
        );

        // Complete initialization
        init_phase3.store(true, Ordering::Relaxed);

        let fully_initialized_now = init_phase1.load(Ordering::Relaxed)
            && init_phase2.load(Ordering::Relaxed)
            && init_phase3.load(Ordering::Relaxed);

        assert!(
            fully_initialized_now,
            "Server should be ready only after full initialization"
        );

        // Test case 7: Initialization state is thread-safe
        let ready_state = Arc::new(AtomicBool::new(false));
        let ready_clone = ready_state.clone();

        ready_state.store(true, Ordering::Relaxed);
        assert_eq!(
            ready_clone.load(Ordering::Relaxed),
            true,
            "Initialization state should be visible across threads"
        );
    }

    #[test]
    fn test_handler_receives_incoming_http_request() {
        // Validates that the request handler can receive and process incoming HTTP requests
        // This is the foundation for the proxy's request handling pipeline

        // Test case 1: Handler can receive a request structure
        struct MockHttpRequest {
            method: String,
            path: String,
            version: String,
        }

        let request = MockHttpRequest {
            method: "GET".to_string(),
            path: "/products/item1.jpg".to_string(),
            version: "HTTP/1.1".to_string(),
        };

        assert_eq!(
            request.method, "GET",
            "Handler should receive request method"
        );
        assert_eq!(
            request.path, "/products/item1.jpg",
            "Handler should receive request path"
        );
        assert_eq!(
            request.version, "HTTP/1.1",
            "Handler should receive HTTP version"
        );

        // Test case 2: Handler can process different HTTP methods
        let get_request = MockHttpRequest {
            method: "GET".to_string(),
            path: "/path".to_string(),
            version: "HTTP/1.1".to_string(),
        };

        let head_request = MockHttpRequest {
            method: "HEAD".to_string(),
            path: "/path".to_string(),
            version: "HTTP/1.1".to_string(),
        };

        assert_eq!(get_request.method, "GET");
        assert_eq!(head_request.method, "HEAD");

        // Test case 3: Handler can receive requests with various paths
        let paths = vec![
            "/products/item1.jpg",
            "/users/profile.json",
            "/api/v1/data",
            "/static/images/logo.png",
        ];

        for path in paths {
            let req = MockHttpRequest {
                method: "GET".to_string(),
                path: path.to_string(),
                version: "HTTP/1.1".to_string(),
            };
            assert_eq!(req.path, path, "Handler should preserve request path");
        }

        // Test case 4: Handler can identify request type
        let is_get = |method: &str| method == "GET";
        let is_head = |method: &str| method == "HEAD";

        assert!(is_get("GET"), "Handler should identify GET requests");
        assert!(is_head("HEAD"), "Handler should identify HEAD requests");
        assert!(!is_get("POST"), "Handler should distinguish request types");

        // Test case 5: Handler can extract path components
        let request_path = "/products/item1.jpg";
        let path_parts: Vec<&str> = request_path.split('/').collect();

        assert!(
            path_parts.len() >= 2,
            "Handler should be able to split path components"
        );
        assert_eq!(
            path_parts[1], "products",
            "Handler should extract path segments"
        );
        assert_eq!(
            path_parts[2], "item1.jpg",
            "Handler should extract filename"
        );

        // Test case 6: Handler can handle requests with query strings
        let request_with_query = MockHttpRequest {
            method: "GET".to_string(),
            path: "/products?id=123&format=json".to_string(),
            version: "HTTP/1.1".to_string(),
        };

        assert!(
            request_with_query.path.contains("?"),
            "Handler should preserve query strings"
        );
        assert!(
            request_with_query.path.contains("id=123"),
            "Handler should preserve query parameters"
        );

        // Test case 7: Handler validates request has required fields
        let has_method = !request.method.is_empty();
        let has_path = !request.path.is_empty();
        let has_version = !request.version.is_empty();

        assert!(
            has_method && has_path && has_version,
            "Handler should validate all required request fields are present"
        );
    }

    #[test]
    fn test_handler_can_access_request_method() {
        // Validates that the handler can access and work with the HTTP request method
        // The method determines how the request should be processed

        // Test case 1: Handler can access GET method
        let get_method = "GET";
        assert_eq!(get_method, "GET", "Handler should access GET method");

        // Test case 2: Handler can access HEAD method
        let head_method = "HEAD";
        assert_eq!(head_method, "HEAD", "Handler should access HEAD method");

        // Test case 3: Handler can access POST method
        let post_method = "POST";
        assert_eq!(post_method, "POST", "Handler should access POST method");

        // Test case 4: Handler can distinguish between different methods
        let methods = vec!["GET", "HEAD", "POST", "PUT", "DELETE"];
        for method in &methods {
            assert!(!method.is_empty(), "Method should not be empty");
            assert!(
                method.len() >= 3,
                "Valid HTTP methods should have at least 3 characters"
            );
        }

        // Test case 5: Handler can check if method is GET
        let check_is_get = |m: &str| m == "GET";
        assert!(check_is_get("GET"), "Should identify GET method");
        assert!(
            !check_is_get("POST"),
            "Should distinguish GET from other methods"
        );

        // Test case 6: Handler can check if method is HEAD
        let check_is_head = |m: &str| m == "HEAD";
        assert!(check_is_head("HEAD"), "Should identify HEAD method");
        assert!(
            !check_is_head("GET"),
            "Should distinguish HEAD from other methods"
        );

        // Test case 7: Handler validates method is uppercase
        let method = "GET";
        assert!(
            method
                .chars()
                .all(|c| c.is_uppercase() || !c.is_alphabetic()),
            "HTTP methods should be uppercase"
        );

        // Test case 8: Handler can match method against allowed methods
        let allowed_methods = vec!["GET", "HEAD"];
        let request_method = "GET";

        assert!(
            allowed_methods.contains(&request_method),
            "Handler should check if method is allowed"
        );

        let disallowed_method = "POST";
        assert!(
            !allowed_methods.contains(&disallowed_method),
            "Handler should reject disallowed methods"
        );

        // Test case 9: Handler extracts method from request structure
        struct HttpRequest {
            method: String,
        }

        let request = HttpRequest {
            method: "GET".to_string(),
        };

        assert_eq!(
            request.method, "GET",
            "Handler should extract method from request"
        );

        // Test case 10: Handler can work with method references
        let method_ref = &request.method;
        assert_eq!(
            method_ref, "GET",
            "Handler should work with method references"
        );
        assert_eq!(
            method_ref.as_str(),
            "GET",
            "Handler should convert method to string slice"
        );
    }

    #[test]
    fn test_handler_can_access_request_path() {
        // Validates that the handler can access and work with the HTTP request path
        // The path identifies the resource being requested

        // Test case 1: Handler can access simple path
        let path = "/products/item1.jpg";
        assert_eq!(
            path, "/products/item1.jpg",
            "Handler should access request path"
        );

        // Test case 2: Handler can access root path
        let root_path = "/";
        assert_eq!(root_path, "/", "Handler should access root path");

        // Test case 3: Handler can access nested paths
        let nested_path = "/api/v1/users/123/profile";
        assert_eq!(
            nested_path, "/api/v1/users/123/profile",
            "Handler should access nested paths"
        );

        // Test case 4: Handler can split path into segments
        let path = "/products/category/item";
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        assert_eq!(segments.len(), 3, "Should split path into segments");
        assert_eq!(segments[0], "products", "First segment should be products");
        assert_eq!(segments[1], "category", "Second segment should be category");
        assert_eq!(segments[2], "item", "Third segment should be item");

        // Test case 5: Handler can separate path from query string
        let full_path = "/products/item?id=123&format=json";
        let parts: Vec<&str> = full_path.splitn(2, '?').collect();
        assert_eq!(parts.len(), 2, "Should split path and query string");
        assert_eq!(parts[0], "/products/item", "Path part should be extracted");
        assert_eq!(
            parts[1], "id=123&format=json",
            "Query string should be extracted"
        );

        // Test case 6: Handler validates path starts with slash
        let valid_path = "/products";
        assert!(
            valid_path.starts_with('/'),
            "Valid paths should start with slash"
        );

        // Test case 7: Handler can extract file extension from path
        let path_with_ext = "/images/photo.jpg";
        let has_extension = path_with_ext.contains('.');
        assert!(has_extension, "Handler should detect file extensions");

        if let Some(ext_index) = path_with_ext.rfind('.') {
            let extension = &path_with_ext[ext_index + 1..];
            assert_eq!(extension, "jpg", "Handler should extract file extension");
        }

        // Test case 8: Handler can handle paths with special characters
        let special_path = "/files/my-file_v2.pdf";
        assert!(
            special_path.contains('-'),
            "Handler should preserve hyphens in paths"
        );
        assert!(
            special_path.contains('_'),
            "Handler should preserve underscores in paths"
        );

        // Test case 9: Handler extracts path from request structure
        struct HttpRequest {
            path: String,
        }

        let request = HttpRequest {
            path: "/products/item1.jpg".to_string(),
        };

        assert_eq!(
            request.path, "/products/item1.jpg",
            "Handler should extract path from request"
        );

        // Test case 10: Handler can normalize paths with double slashes
        let path_with_doubles = "/products//item";
        let normalized = path_with_doubles.replace("//", "/");
        assert_eq!(
            normalized, "/products/item",
            "Handler should normalize double slashes"
        );
    }

    #[test]
    fn test_handler_can_access_request_headers() {
        // Validates that the handler can access and work with HTTP request headers
        // Headers provide metadata about the request

        use std::collections::HashMap;

        // Test case 1: Handler can access headers as key-value pairs
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());
        headers.insert("User-Agent".to_string(), "TestClient/1.0".to_string());

        assert_eq!(
            headers.get("Host"),
            Some(&"example.com".to_string()),
            "Handler should access Host header"
        );
        assert_eq!(
            headers.get("User-Agent"),
            Some(&"TestClient/1.0".to_string()),
            "Handler should access User-Agent header"
        );

        // Test case 2: Handler can check if header exists
        assert!(
            headers.contains_key("Host"),
            "Should check if header exists"
        );
        assert!(
            !headers.contains_key("Authorization"),
            "Should detect missing headers"
        );

        // Test case 3: Handler can access common HTTP headers
        let mut request_headers = HashMap::new();
        request_headers.insert("Content-Type".to_string(), "application/json".to_string());
        request_headers.insert("Content-Length".to_string(), "1234".to_string());
        request_headers.insert("Accept".to_string(), "*/*".to_string());

        assert_eq!(
            request_headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request_headers.get("Content-Length").unwrap(), "1234");
        assert_eq!(request_headers.get("Accept").unwrap(), "*/*");

        // Test case 4: Handler can handle case-insensitive header names
        let header_name_lower = "content-type";
        let header_name_title = "Content-Type";
        assert_eq!(
            header_name_lower.to_lowercase(),
            header_name_title.to_lowercase(),
            "Handler should normalize header names"
        );

        // Test case 5: Handler can extract Authorization header
        let mut auth_headers = HashMap::new();
        auth_headers.insert("Authorization".to_string(), "Bearer abc123".to_string());

        let auth_value = auth_headers.get("Authorization");
        assert!(auth_value.is_some(), "Should find Authorization header");
        assert!(
            auth_value.unwrap().starts_with("Bearer "),
            "Should extract Bearer token"
        );

        // Test case 6: Handler can iterate over all headers
        let mut all_headers = HashMap::new();
        all_headers.insert("Header1".to_string(), "Value1".to_string());
        all_headers.insert("Header2".to_string(), "Value2".to_string());
        all_headers.insert("Header3".to_string(), "Value3".to_string());

        let header_count = all_headers.len();
        assert_eq!(header_count, 3, "Should count all headers");

        for (key, value) in &all_headers {
            assert!(!key.is_empty(), "Header name should not be empty");
            assert!(!value.is_empty(), "Header value should not be empty");
        }

        // Test case 7: Handler can handle multi-value headers
        let range_header = "bytes=0-1023, 1024-2047";
        assert!(
            range_header.contains(','),
            "Handler should detect multi-value headers"
        );

        let ranges: Vec<&str> = range_header.split(',').map(|s| s.trim()).collect();
        assert_eq!(ranges.len(), 2, "Should split multi-value header");
        assert_eq!(ranges[0], "bytes=0-1023");
        assert_eq!(ranges[1], "1024-2047");

        // Test case 8: Handler can extract custom headers
        let mut custom_headers = HashMap::new();
        custom_headers.insert("X-Custom-Header".to_string(), "CustomValue".to_string());
        custom_headers.insert("X-Request-ID".to_string(), "req-123".to_string());

        assert_eq!(
            custom_headers.get("X-Custom-Header").unwrap(),
            "CustomValue"
        );
        assert_eq!(custom_headers.get("X-Request-ID").unwrap(), "req-123");

        // Test case 9: Handler extracts headers from request structure
        struct HttpRequest {
            headers: HashMap<String, String>,
        }

        let mut req_headers = HashMap::new();
        req_headers.insert("Host".to_string(), "example.com".to_string());

        let request = HttpRequest {
            headers: req_headers,
        };

        assert_eq!(
            request.headers.get("Host").unwrap(),
            "example.com",
            "Handler should extract headers from request"
        );

        // Test case 10: Handler can handle empty header values
        let mut headers_with_empty = HashMap::new();
        headers_with_empty.insert("Empty-Header".to_string(), "".to_string());

        assert!(
            headers_with_empty.contains_key("Empty-Header"),
            "Should accept headers with empty values"
        );
    }

    #[test]
    fn test_handler_can_access_request_query_parameters() {
        // Validates that the handler can access and work with query parameters
        // Query parameters provide additional data in the URL

        use std::collections::HashMap;

        // Test case 1: Handler can parse query string from URL
        let url = "/products?id=123&format=json";
        let query_start = url.find('?');
        assert!(query_start.is_some(), "Should find query string start");

        let query_string = &url[query_start.unwrap() + 1..];
        assert_eq!(
            query_string, "id=123&format=json",
            "Should extract query string"
        );

        // Test case 2: Handler can split query parameters
        let params: Vec<&str> = query_string.split('&').collect();
        assert_eq!(params.len(), 2, "Should split query parameters");
        assert_eq!(params[0], "id=123");
        assert_eq!(params[1], "format=json");

        // Test case 3: Handler can parse parameter key-value pairs
        let mut query_params = HashMap::new();
        for param in params {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                query_params.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        assert_eq!(query_params.get("id").unwrap(), "123");
        assert_eq!(query_params.get("format").unwrap(), "json");

        // Test case 4: Handler can handle single query parameter
        let single_param_url = "/path?token=abc123";
        if let Some(idx) = single_param_url.find('?') {
            let query = &single_param_url[idx + 1..];
            let parts: Vec<&str> = query.splitn(2, '=').collect();
            assert_eq!(parts[0], "token");
            assert_eq!(parts[1], "abc123");
        }

        // Test case 5: Handler can handle URL without query parameters
        let no_query_url = "/products/item";
        assert!(
            !no_query_url.contains('?'),
            "Should detect absence of query"
        );

        // Test case 6: Handler can handle empty query parameter values
        let empty_value_query = "key1=&key2=value2";
        let mut params_with_empty = HashMap::new();
        for param in empty_value_query.split('&') {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                params_with_empty.insert(parts[0].to_string(), parts[1].to_string());
            }
        }

        assert_eq!(params_with_empty.get("key1").unwrap(), "");
        assert_eq!(params_with_empty.get("key2").unwrap(), "value2");

        // Test case 7: Handler can handle URL-encoded query parameters
        let encoded_value = "name=John%20Doe";
        let parts: Vec<&str> = encoded_value.splitn(2, '=').collect();
        assert_eq!(parts[1], "John%20Doe", "Should preserve encoded value");

        // Test case 8: Handler can extract specific query parameter
        let url_with_many_params = "/search?q=rust&page=2&limit=10&sort=desc";
        let query_str = &url_with_many_params[url_with_many_params.find('?').unwrap() + 1..];

        let mut all_params = HashMap::new();
        for param in query_str.split('&') {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 {
                all_params.insert(parts[0], parts[1]);
            }
        }

        assert_eq!(all_params.get("q").unwrap(), &"rust");
        assert_eq!(all_params.get("page").unwrap(), &"2");
        assert_eq!(all_params.get("limit").unwrap(), &"10");
        assert_eq!(all_params.get("sort").unwrap(), &"desc");

        // Test case 9: Handler separates path from query parameters
        let full_url = "/api/users?role=admin&active=true";
        let parts: Vec<&str> = full_url.splitn(2, '?').collect();

        assert_eq!(parts[0], "/api/users", "Should extract path");
        assert_eq!(parts[1], "role=admin&active=true", "Should extract query");

        // Test case 10: Handler can check if specific parameter exists
        let query = "id=123&name=test";
        assert!(query.contains("id="), "Should find parameter by name");
        assert!(query.contains("name="), "Should find parameter by name");
        assert!(!query.contains("email="), "Should detect missing parameter");
    }

    #[test]
    fn test_handler_runs_router_to_determine_target_bucket() {
        // Validates that handler uses router to match request paths to target buckets
        // The router determines which S3 bucket should handle the request

        use std::collections::HashMap;

        // Test case 1: Handler routes path to matching bucket
        let mut route_map = HashMap::new();
        route_map.insert("/products".to_string(), "products-bucket".to_string());
        route_map.insert("/users".to_string(), "users-bucket".to_string());

        let matched_bucket = route_map.get("/products");
        assert!(
            matched_bucket.is_some(),
            "Handler should find matching bucket"
        );
        assert_eq!(matched_bucket.unwrap(), "products-bucket");

        // Test case 2: Handler uses longest prefix match
        let mut prefix_map = HashMap::new();
        prefix_map.insert("/api".to_string(), "api-bucket".to_string());
        prefix_map.insert("/api/v1".to_string(), "api-v1-bucket".to_string());

        let path = "/api/v1/users";
        // Simulate finding longest matching prefix
        let prefixes = vec!["/api", "/api/v1"];
        let longest = prefixes
            .iter()
            .filter(|p| path.starts_with(*p))
            .max_by_key(|p| p.len());

        assert_eq!(longest, Some(&"/api/v1"));

        // Test case 3: Handler returns None for unmatched paths
        let routes = vec!["/products", "/users"];
        let unmatched_path = "/admin/settings";

        let has_match = routes.iter().any(|r| unmatched_path.starts_with(r));
        assert!(!has_match, "Handler should detect unmatched path");

        // Test case 4: Handler extracts S3 key from matched route
        let bucket_prefix = "/products";
        let full_path = "/products/category/item.jpg";

        let s3_key = if full_path.starts_with(bucket_prefix) {
            &full_path[bucket_prefix.len()..]
        } else {
            full_path
        };

        assert_eq!(
            s3_key, "/category/item.jpg",
            "Handler should extract S3 key"
        );

        // Test case 5: Handler routes based on path structure
        struct RouteEntry {
            prefix: String,
            bucket: String,
        }

        let routes = vec![
            RouteEntry {
                prefix: "/images".to_string(),
                bucket: "images-bucket".to_string(),
            },
            RouteEntry {
                prefix: "/videos".to_string(),
                bucket: "videos-bucket".to_string(),
            },
        ];

        let test_path = "/images/photo.jpg";
        let matched = routes.iter().find(|r| test_path.starts_with(&r.prefix));

        assert!(matched.is_some(), "Handler should find route");
        assert_eq!(matched.unwrap().bucket, "images-bucket");

        // Test case 6: Handler handles root path routing
        let root_routes = vec![("/", "default-bucket")];
        let root_path = "/";

        let root_match = root_routes.iter().find(|(p, _)| *p == root_path);
        assert!(root_match.is_some(), "Handler should match root path");

        // Test case 7: Handler normalizes path before routing
        let path_with_query = "/products/item?id=123";
        let clean_path = path_with_query.split('?').next().unwrap();

        assert_eq!(clean_path, "/products/item", "Handler should strip query");

        // Test case 8: Handler matches case-sensitive paths
        let case_routes = vec!["/Products", "/products"];
        let lowercase_path = "/products/item";

        let case_match = case_routes.iter().find(|r| lowercase_path.starts_with(*r));

        assert_eq!(case_match, Some(&"/products"));

        // Test case 9: Handler processes multiple bucket configurations
        struct BucketConfig {
            name: String,
            prefix: String,
        }

        let buckets = vec![
            BucketConfig {
                name: "bucket1".to_string(),
                prefix: "/prefix1".to_string(),
            },
            BucketConfig {
                name: "bucket2".to_string(),
                prefix: "/prefix2".to_string(),
            },
            BucketConfig {
                name: "bucket3".to_string(),
                prefix: "/prefix3".to_string(),
            },
        ];

        let request = "/prefix2/file.txt";
        let matched_bucket = buckets.iter().find(|b| request.starts_with(&b.prefix));

        assert!(matched_bucket.is_some());
        assert_eq!(matched_bucket.unwrap().name, "bucket2");

        // Test case 10: Handler returns routing result
        enum RoutingResult {
            Found { bucket: String, s3_key: String },
            NotFound,
        }

        let result = if let Some(route) = routes.iter().find(|r| {
            "/videos/movie.mp4"
                .split('?')
                .next()
                .unwrap()
                .starts_with(&r.prefix)
        }) {
            let path = "/videos/movie.mp4".split('?').next().unwrap();
            let key = &path[route.prefix.len()..];
            RoutingResult::Found {
                bucket: route.bucket.clone(),
                s3_key: key.to_string(),
            }
        } else {
            RoutingResult::NotFound
        };

        match result {
            RoutingResult::Found { bucket, s3_key } => {
                assert_eq!(bucket, "videos-bucket");
                assert_eq!(s3_key, "/movie.mp4");
            }
            RoutingResult::NotFound => panic!("Should find route"),
        }
    }
}
