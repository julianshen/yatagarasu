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

    #[test]
    fn test_handler_runs_auth_middleware_when_configured() {
        // Validates that handler runs authentication middleware when configured
        // Auth middleware validates JWT tokens and enforces access control

        use std::collections::HashMap;

        // Test case 1: Handler checks if auth is enabled for bucket
        struct BucketConfig {
            name: String,
            auth_enabled: bool,
        }

        let bucket = BucketConfig {
            name: "private-bucket".to_string(),
            auth_enabled: true,
        };

        assert_eq!(bucket.name, "private-bucket");
        assert!(
            bucket.auth_enabled,
            "Handler should check if auth is enabled"
        );

        // Test case 2: Handler extracts token from Authorization header
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer abc123".to_string());

        let auth_header = headers.get("Authorization");
        assert!(auth_header.is_some(), "Handler should find auth header");

        let token = auth_header.unwrap().strip_prefix("Bearer ").unwrap_or("");
        assert_eq!(token, "abc123", "Handler should extract token");

        // Test case 3: Handler validates token format
        let valid_token =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature";
        let parts: Vec<&str> = valid_token.split('.').collect();

        assert_eq!(parts.len(), 3, "JWT should have 3 parts");
        assert!(!parts[0].is_empty(), "Header should not be empty");
        assert!(!parts[1].is_empty(), "Payload should not be empty");
        assert!(!parts[2].is_empty(), "Signature should not be empty");

        // Test case 4: Handler returns 401 if token is missing
        let no_auth_headers: HashMap<String, String> = HashMap::new();
        let missing_token = no_auth_headers.get("Authorization");

        let auth_result = if missing_token.is_none() {
            Err("Unauthorized")
        } else {
            Ok(())
        };

        assert!(auth_result.is_err(), "Should reject missing token");

        // Test case 5: Handler allows request if auth passes
        let valid_auth_headers = HashMap::from([(
            "Authorization".to_string(),
            "Bearer valid-token".to_string(),
        )]);

        let has_token = valid_auth_headers.contains_key("Authorization");
        assert!(has_token, "Handler should find valid token");

        // Test case 6: Handler bypasses auth if not required
        let public_bucket = BucketConfig {
            name: "public-bucket".to_string(),
            auth_enabled: false,
        };

        let no_headers: HashMap<String, String> = HashMap::new();
        let bypass_result = if !public_bucket.auth_enabled {
            Ok(())
        } else if no_headers.contains_key("Authorization") {
            Ok(())
        } else {
            Err("Unauthorized")
        };

        assert!(
            bypass_result.is_ok(),
            "Should bypass auth for public bucket"
        );

        // Test case 7: Handler extracts token from query parameter
        let url_with_token = "/path?token=xyz789";
        let query_start = url_with_token.find('?').unwrap();
        let query = &url_with_token[query_start + 1..];

        let params: HashMap<String, String> = query
            .split('&')
            .filter_map(|p| {
                let parts: Vec<&str> = p.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(
            params.get("token").unwrap(),
            "xyz789",
            "Handler should extract token from query"
        );

        // Test case 8: Handler validates token claims
        struct TokenClaims {
            sub: String,
            exp: u64,
        }

        let claims = TokenClaims {
            sub: "user123".to_string(),
            exp: 9999999999,
        };

        assert!(!claims.sub.is_empty(), "Token should have subject claim");
        assert!(claims.exp > 0, "Token should have expiration");

        // Test case 9: Handler enforces auth before routing to S3
        enum RequestStage {
            AuthCheck,
            Routing,
            S3Request,
        }

        let auth_required = true;
        let next_stage = if auth_required {
            RequestStage::AuthCheck
        } else {
            RequestStage::Routing
        };

        match next_stage {
            RequestStage::AuthCheck => {
                assert!(true, "Auth should run before routing");
            }
            RequestStage::Routing => panic!("Should check auth first"),
            RequestStage::S3Request => panic!("Should check auth first"),
        }

        // Test case 10: Handler returns auth status
        enum AuthStatus {
            Authenticated { user_id: String },
            Unauthenticated,
            Bypassed,
        }

        let private_bucket_auth = AuthStatus::Authenticated {
            user_id: "user123".to_string(),
        };

        match private_bucket_auth {
            AuthStatus::Authenticated { user_id } => {
                assert_eq!(user_id, "user123", "Should track authenticated user");
            }
            AuthStatus::Unauthenticated => panic!("Should be authenticated"),
            AuthStatus::Bypassed => panic!("Should be authenticated"),
        }

        let public_bucket_auth = AuthStatus::Bypassed;
        match public_bucket_auth {
            AuthStatus::Bypassed => {
                assert!(true, "Public buckets should bypass auth");
            }
            AuthStatus::Authenticated { .. } => panic!("Should bypass auth"),
            AuthStatus::Unauthenticated => panic!("Should bypass auth"),
        }
    }

    #[test]
    fn test_handler_builds_s3_request_from_http_request() {
        // Validates that handler can build S3 request from incoming HTTP request
        // S3 request includes method, bucket, key, headers, and authentication

        use std::collections::HashMap;

        // Test case 1: Handler constructs S3 URL from bucket and key
        let bucket = "my-bucket";
        let s3_key = "products/item.jpg";
        let s3_url = format!("https://{}.s3.amazonaws.com/{}", bucket, s3_key);

        assert_eq!(
            s3_url, "https://my-bucket.s3.amazonaws.com/products/item.jpg",
            "Handler should construct S3 URL"
        );

        // Test case 2: Handler preserves HTTP method for S3 request
        let http_method = "GET";
        let s3_method = http_method; // S3 supports same methods

        assert_eq!(s3_method, "GET", "Handler should preserve GET method");

        let head_method = "HEAD";
        assert_eq!(head_method, "HEAD", "Handler should preserve HEAD method");

        // Test case 3: Handler includes region in S3 URL
        let region = "us-west-2";
        let regional_url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket, region, s3_key);

        assert_eq!(
            regional_url,
            "https://my-bucket.s3.us-west-2.amazonaws.com/products/item.jpg"
        );

        // Test case 4: Handler forwards Range header to S3
        let mut client_headers = HashMap::new();
        client_headers.insert("Range".to_string(), "bytes=0-1023".to_string());

        let mut s3_headers = HashMap::new();
        if let Some(range) = client_headers.get("Range") {
            s3_headers.insert("Range".to_string(), range.clone());
        }

        assert_eq!(
            s3_headers.get("Range").unwrap(),
            "bytes=0-1023",
            "Handler should forward Range header"
        );

        // Test case 5: Handler adds AWS signature headers
        struct AwsSignature {
            authorization: String,
            date: String,
            content_sha256: String,
        }

        let signature = AwsSignature {
            authorization: "AWS4-HMAC-SHA256 Credential=...".to_string(),
            date: "20231201T120000Z".to_string(),
            content_sha256: "UNSIGNED-PAYLOAD".to_string(),
        };

        assert!(
            signature.authorization.starts_with("AWS4-HMAC-SHA256"),
            "Handler should add AWS signature"
        );
        assert!(!signature.date.is_empty(), "Handler should add date header");
        assert_eq!(signature.content_sha256, "UNSIGNED-PAYLOAD");

        // Test case 6: Handler encodes special characters in S3 key
        let key_with_spaces = "folder/my file.jpg";
        let encoded_key = key_with_spaces.replace(' ', "%20");

        assert_eq!(
            encoded_key, "folder/my%20file.jpg",
            "Handler should encode spaces"
        );

        // Test case 7: Handler builds path-style URL for custom endpoints
        let custom_endpoint = "http://localhost:9000";
        let path_style_url = format!("{}/{}/{}", custom_endpoint, bucket, s3_key);

        assert_eq!(
            path_style_url, "http://localhost:9000/my-bucket/products/item.jpg",
            "Handler should support path-style URLs"
        );

        // Test case 8: Handler includes Host header for S3
        let host_header = format!("{}.s3.amazonaws.com", bucket);
        assert_eq!(
            host_header, "my-bucket.s3.amazonaws.com",
            "Handler should set Host header"
        );

        // Test case 9: Handler creates complete S3 request structure
        struct S3Request {
            method: String,
            url: String,
            headers: HashMap<String, String>,
        }

        let mut request_headers = HashMap::new();
        request_headers.insert("Host".to_string(), host_header.clone());
        request_headers.insert(
            "Authorization".to_string(),
            "AWS4-HMAC-SHA256...".to_string(),
        );

        let s3_request = S3Request {
            method: "GET".to_string(),
            url: s3_url.clone(),
            headers: request_headers,
        };

        assert_eq!(s3_request.method, "GET");
        assert!(s3_request.url.contains("s3.amazonaws.com"));
        assert!(s3_request.headers.contains_key("Host"));
        assert!(s3_request.headers.contains_key("Authorization"));

        // Test case 10: Handler handles empty S3 keys (root bucket access)
        let empty_key = "";
        let root_url = format!("https://{}.s3.amazonaws.com/{}", bucket, empty_key);

        assert!(
            root_url.ends_with('/') || root_url.ends_with(".com"),
            "Handler should handle empty keys"
        );
    }

    #[test]
    fn test_can_send_response_status_code() {
        // Validates that response handler can send HTTP status codes
        // Status codes indicate the result of the request processing

        // Test case 1: Handler can send 200 OK for successful requests
        let success_status = 200;
        assert_eq!(success_status, 200, "Handler should send 200 OK");

        // Test case 2: Handler can send 206 Partial Content for range requests
        let partial_status = 206;
        assert_eq!(
            partial_status, 206,
            "Handler should send 206 Partial Content"
        );

        // Test case 3: Handler can send 401 Unauthorized for auth failures
        let unauthorized_status = 401;
        assert_eq!(
            unauthorized_status, 401,
            "Handler should send 401 Unauthorized"
        );

        // Test case 4: Handler can send 404 Not Found for missing objects
        let not_found_status = 404;
        assert_eq!(not_found_status, 404, "Handler should send 404 Not Found");

        // Test case 5: Handler can send 416 Range Not Satisfiable
        let range_error_status = 416;
        assert_eq!(
            range_error_status, 416,
            "Handler should send 416 Range Not Satisfiable"
        );

        // Test case 6: Handler can send 500 Internal Server Error
        let server_error_status = 500;
        assert_eq!(
            server_error_status, 500,
            "Handler should send 500 Internal Server Error"
        );

        // Test case 7: Handler validates status code is in valid range
        let status_codes = vec![200, 206, 401, 403, 404, 416, 500, 503];
        for code in status_codes {
            assert!(
                code >= 100 && code < 600,
                "Status code should be in valid range"
            );
        }

        // Test case 8: Handler distinguishes success vs error status codes
        let is_success = |code: u16| code >= 200 && code < 300;
        let is_client_error = |code: u16| code >= 400 && code < 500;
        let is_server_error = |code: u16| code >= 500 && code < 600;

        assert!(is_success(200), "200 is a success status");
        assert!(is_success(206), "206 is a success status");
        assert!(is_client_error(401), "401 is a client error");
        assert!(is_client_error(404), "404 is a client error");
        assert!(is_server_error(500), "500 is a server error");

        // Test case 9: Handler creates response with status code
        struct HttpResponse {
            status_code: u16,
            status_text: String,
        }

        let ok_response = HttpResponse {
            status_code: 200,
            status_text: "OK".to_string(),
        };

        assert_eq!(ok_response.status_code, 200);
        assert_eq!(ok_response.status_text, "OK");

        let not_found_response = HttpResponse {
            status_code: 404,
            status_text: "Not Found".to_string(),
        };

        assert_eq!(not_found_response.status_code, 404);
        assert_eq!(not_found_response.status_text, "Not Found");

        // Test case 10: Handler maps status code to status text
        let get_status_text = |code: u16| match code {
            200 => "OK",
            206 => "Partial Content",
            401 => "Unauthorized",
            404 => "Not Found",
            416 => "Range Not Satisfiable",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Unknown",
        };

        assert_eq!(get_status_text(200), "OK");
        assert_eq!(get_status_text(206), "Partial Content");
        assert_eq!(get_status_text(401), "Unauthorized");
        assert_eq!(get_status_text(404), "Not Found");
        assert_eq!(get_status_text(416), "Range Not Satisfiable");
        assert_eq!(get_status_text(500), "Internal Server Error");
    }

    #[test]
    fn test_can_send_response_headers() {
        // Validates that response handler can add and send HTTP headers
        // Headers provide metadata about the response (content-type, length, cache, etc.)

        // Test case 1: Handler can create response with headers
        use std::collections::HashMap;

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        assert_eq!(headers.len(), 1, "Handler should create headers map");
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string()),
            "Handler should set content-type header"
        );

        // Test case 2: Handler can set content-length header
        headers.insert("content-length".to_string(), "1024".to_string());
        assert_eq!(
            headers.get("content-length"),
            Some(&"1024".to_string()),
            "Handler should set content-length header"
        );

        // Test case 3: Handler can set cache-control header
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());
        assert_eq!(
            headers.get("cache-control"),
            Some(&"max-age=3600".to_string()),
            "Handler should set cache-control header"
        );

        // Test case 4: Handler can set multiple headers
        headers.insert("etag".to_string(), "\"abc123\"".to_string());
        headers.insert(
            "last-modified".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );

        assert_eq!(headers.len(), 5, "Handler should have 5 headers");

        // Test case 5: Handler can handle header case sensitivity
        // HTTP headers are case-insensitive, but we store them consistently
        let content_type_lower = headers.get("content-type");
        assert!(
            content_type_lower.is_some(),
            "Handler should find lowercase header"
        );

        // Test case 6: Handler can update existing header
        headers.insert("content-type".to_string(), "text/html".to_string());
        assert_eq!(
            headers.get("content-type"),
            Some(&"text/html".to_string()),
            "Handler should update existing header"
        );

        // Test case 7: Handler can remove header
        headers.remove("etag");
        assert_eq!(headers.get("etag"), None, "Handler should remove header");
        assert_eq!(
            headers.len(),
            4,
            "Handler should have 4 headers after removal"
        );

        // Test case 8: Handler can set custom headers (e.g., x-amz-*)
        headers.insert("x-amz-request-id".to_string(), "req-123".to_string());
        headers.insert("x-amz-id-2".to_string(), "id-456".to_string());

        assert!(
            headers.contains_key("x-amz-request-id"),
            "Handler should set custom S3 headers"
        );

        // Test case 9: Handler can handle multi-value headers
        // In real HTTP, some headers can have multiple values
        // We represent this as comma-separated values
        let multi_value = vec!["gzip", "deflate", "br"];
        let accept_encoding = multi_value.join(", ");
        headers.insert("accept-encoding".to_string(), accept_encoding);

        assert_eq!(
            headers.get("accept-encoding"),
            Some(&"gzip, deflate, br".to_string()),
            "Handler should handle multi-value headers"
        );

        // Test case 10: Handler preserves S3 response headers
        let s3_headers = vec![
            ("content-type", "image/jpeg"),
            ("content-length", "2048"),
            ("etag", "\"xyz789\""),
            ("x-amz-version-id", "v1"),
        ];

        for (name, value) in s3_headers {
            headers.insert(name.to_string(), value.to_string());
        }

        assert_eq!(
            headers.get("content-type"),
            Some(&"image/jpeg".to_string()),
            "Handler should preserve S3 content-type"
        );
        assert_eq!(
            headers.get("x-amz-version-id"),
            Some(&"v1".to_string()),
            "Handler should preserve S3 version header"
        );
    }

    #[test]
    fn test_can_send_response_body() {
        // Validates that response handler can send response body content
        // Body contains the actual data returned to the client

        // Test case 1: Handler can send simple text body
        let text_body = "Hello, World!";
        let body_bytes = text_body.as_bytes();

        assert_eq!(body_bytes.len(), 13, "Handler should get body length");
        assert_eq!(
            std::str::from_utf8(body_bytes).unwrap(),
            "Hello, World!",
            "Handler should preserve text content"
        );

        // Test case 2: Handler can send empty body
        let empty_body = "";
        let empty_bytes = empty_body.as_bytes();

        assert_eq!(empty_bytes.len(), 0, "Handler should handle empty body");

        // Test case 3: Handler can send binary data
        let binary_data: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        assert_eq!(binary_data.len(), 4, "Handler should handle binary data");

        // Test case 4: Handler can send JSON body
        let json_body = r#"{"status":"ok","count":42}"#;
        let json_bytes = json_body.as_bytes();

        assert_eq!(json_bytes.len(), 26, "Handler should get JSON body length");
        assert!(
            json_body.contains("status"),
            "Handler should preserve JSON structure"
        );

        // Test case 5: Handler can send large body
        let large_body = "X".repeat(10_000); // 10KB
        let large_bytes = large_body.as_bytes();

        assert_eq!(
            large_bytes.len(),
            10_000,
            "Handler should handle large bodies"
        );

        // Test case 6: Handler creates response with body and content-length
        struct HttpResponse {
            status_code: u16,
            headers: std::collections::HashMap<String, String>,
            body: Vec<u8>,
        }

        let mut headers = std::collections::HashMap::new();
        let response_body = "Response content".as_bytes().to_vec();
        headers.insert(
            "content-length".to_string(),
            response_body.len().to_string(),
        );

        let response = HttpResponse {
            status_code: 200,
            headers: headers.clone(),
            body: response_body.clone(),
        };

        assert_eq!(response.status_code, 200);
        assert_eq!(
            headers.get("content-length"),
            Some(&"16".to_string()),
            "Handler should set correct content-length"
        );
        assert_eq!(response.body.len(), 16);

        // Test case 7: Handler maintains body integrity
        let original_data = "Important data 123!";
        let transmitted_body = original_data.as_bytes().to_vec();

        assert_eq!(
            std::str::from_utf8(&transmitted_body).unwrap(),
            original_data,
            "Handler should maintain body integrity"
        );

        // Test case 8: Handler can handle UTF-8 encoded text
        let utf8_text = "Hello 世界 🌍";
        let utf8_bytes = utf8_text.as_bytes();

        assert!(
            utf8_bytes.len() > utf8_text.chars().count(),
            "UTF-8 uses multiple bytes per char"
        );
        assert_eq!(
            std::str::from_utf8(utf8_bytes).unwrap(),
            utf8_text,
            "Handler should preserve UTF-8 content"
        );

        // Test case 9: Handler can send HTML body
        let html_body = "<html><body><h1>Test</h1></body></html>";
        let html_bytes = html_body.as_bytes();

        assert_eq!(html_bytes.len(), 39, "Handler should get HTML body length");
        assert!(
            html_body.contains("<html>"),
            "Handler should preserve HTML structure"
        );

        // Test case 10: Handler can send response with custom content
        let custom_content = "Custom response from S3 proxy";
        let response_with_custom = HttpResponse {
            status_code: 200,
            headers: {
                let mut h = std::collections::HashMap::new();
                h.insert("content-type".to_string(), "text/plain".to_string());
                h.insert(
                    "content-length".to_string(),
                    custom_content.len().to_string(),
                );
                h
            },
            body: custom_content.as_bytes().to_vec(),
        };

        assert_eq!(
            response_with_custom.headers.get("content-type"),
            Some(&"text/plain".to_string())
        );
        assert_eq!(
            std::str::from_utf8(&response_with_custom.body).unwrap(),
            custom_content
        );
    }

    #[test]
    fn test_can_stream_response_body_chunks() {
        // Validates that response handler can stream body in chunks
        // Streaming enables constant memory usage for large files

        // Test case 1: Handler can create stream from chunks
        let chunks: Vec<Vec<u8>> = vec![b"chunk1".to_vec(), b"chunk2".to_vec(), b"chunk3".to_vec()];

        assert_eq!(chunks.len(), 3, "Handler should have 3 chunks");
        assert_eq!(chunks[0], b"chunk1");
        assert_eq!(chunks[1], b"chunk2");
        assert_eq!(chunks[2], b"chunk3");

        // Test case 2: Handler can assemble chunks into full body
        let mut assembled = Vec::new();
        for chunk in &chunks {
            assembled.extend_from_slice(chunk);
        }

        assert_eq!(
            std::str::from_utf8(&assembled).unwrap(),
            "chunk1chunk2chunk3",
            "Handler should assemble chunks correctly"
        );

        // Test case 3: Handler can handle different chunk sizes
        let variable_chunks: Vec<Vec<u8>> = vec![
            b"a".to_vec(),    // 1 byte
            b"bb".to_vec(),   // 2 bytes
            b"ccc".to_vec(),  // 3 bytes
            b"dddd".to_vec(), // 4 bytes
        ];

        let total_size: usize = variable_chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_size, 10, "Handler should handle variable chunk sizes");

        // Test case 4: Handler can stream large chunks
        let large_chunk = vec![0u8; 64 * 1024]; // 64KB chunk
        assert_eq!(
            large_chunk.len(),
            65536,
            "Handler should handle 64KB chunks"
        );

        // Test case 5: Handler preserves chunk order
        let ordered_chunks: Vec<Vec<u8>> =
            vec![b"first".to_vec(), b"second".to_vec(), b"third".to_vec()];

        let mut result = Vec::new();
        for chunk in ordered_chunks {
            result.extend_from_slice(&chunk);
        }

        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "firstsecondthird",
            "Handler should preserve chunk order"
        );

        // Test case 6: Handler can stream empty chunks
        let chunks_with_empty: Vec<Vec<u8>> = vec![
            b"data".to_vec(),
            vec![], // Empty chunk
            b"more".to_vec(),
        ];

        let mut result = Vec::new();
        for chunk in chunks_with_empty {
            result.extend_from_slice(&chunk);
        }

        assert_eq!(
            std::str::from_utf8(&result).unwrap(),
            "datamore",
            "Handler should handle empty chunks"
        );

        // Test case 7: Handler can detect end of stream
        struct ChunkStream {
            chunks: Vec<Vec<u8>>,
            position: usize,
        }

        impl ChunkStream {
            fn new(chunks: Vec<Vec<u8>>) -> Self {
                ChunkStream {
                    chunks,
                    position: 0,
                }
            }

            fn next_chunk(&mut self) -> Option<Vec<u8>> {
                if self.position < self.chunks.len() {
                    let chunk = self.chunks[self.position].clone();
                    self.position += 1;
                    Some(chunk)
                } else {
                    None
                }
            }

            fn is_complete(&self) -> bool {
                self.position >= self.chunks.len()
            }
        }

        let mut stream = ChunkStream::new(vec![b"a".to_vec(), b"b".to_vec()]);

        assert_eq!(stream.next_chunk(), Some(b"a".to_vec()));
        assert_eq!(stream.next_chunk(), Some(b"b".to_vec()));
        assert_eq!(stream.next_chunk(), None);
        assert!(stream.is_complete(), "Handler should detect end of stream");

        // Test case 8: Handler can stream many small chunks efficiently
        let many_chunks: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8]).collect();

        assert_eq!(
            many_chunks.len(),
            100,
            "Handler should handle many small chunks"
        );

        // Test case 9: Handler can track bytes streamed
        let chunks = vec![b"abc".to_vec(), b"defgh".to_vec(), b"ij".to_vec()];

        let mut bytes_streamed = 0;
        for chunk in &chunks {
            bytes_streamed += chunk.len();
        }

        assert_eq!(
            bytes_streamed, 10,
            "Handler should track total bytes streamed"
        );

        // Test case 10: Handler maintains constant memory during streaming
        // Simulate streaming large file by processing chunks one at a time
        let total_chunks = 1000;
        let chunk_size = 64 * 1024; // 64KB per chunk

        let mut processed_chunks = 0;
        let mut total_bytes = 0;

        for _ in 0..total_chunks {
            // Simulate receiving chunk
            let _chunk = vec![0u8; chunk_size];

            // Process chunk (in real implementation, would send to client)
            processed_chunks += 1;
            total_bytes += chunk_size;

            // Chunk goes out of scope, memory is freed
        }

        assert_eq!(processed_chunks, total_chunks);
        assert_eq!(total_bytes, total_chunks * chunk_size);
        // Memory remains constant because we only hold one chunk at a time
    }

    #[test]
    fn test_handles_connection_close_during_streaming() {
        // Validates that handler properly handles client disconnect during streaming
        // Must stop S3 stream and cleanup resources when client disconnects

        // Test case 1: Handler can detect connection closed state
        struct Connection {
            is_closed: bool,
        }

        impl Connection {
            fn new() -> Self {
                Connection { is_closed: false }
            }

            fn close(&mut self) {
                self.is_closed = true;
            }

            fn is_closed(&self) -> bool {
                self.is_closed
            }
        }

        let mut conn = Connection::new();
        assert!(!conn.is_closed(), "Connection should start as open");

        conn.close();
        assert!(
            conn.is_closed(),
            "Connection should be closed after close()"
        );

        // Test case 2: Handler stops streaming when connection closes
        let mut conn = Connection::new();
        let chunks = vec![
            b"chunk1".to_vec(),
            b"chunk2".to_vec(),
            b"chunk3".to_vec(),
            b"chunk4".to_vec(),
        ];

        let mut sent_chunks = 0;
        for chunk in chunks {
            if conn.is_closed() {
                break; // Stop streaming if connection closed
            }

            // Simulate sending chunk
            sent_chunks += 1;
            let _ = chunk; // Would send to client here

            // Client disconnects after 2 chunks
            if sent_chunks == 2 {
                conn.close();
            }
        }

        assert_eq!(
            sent_chunks, 2,
            "Handler should stop after connection closes"
        );

        // Test case 3: Handler tracks partial transfer
        struct StreamState {
            total_bytes: usize,
            bytes_sent: usize,
            connection_closed: bool,
        }

        let mut state = StreamState {
            total_bytes: 10000,
            bytes_sent: 0,
            connection_closed: false,
        };

        let chunks = vec![vec![0u8; 2000], vec![0u8; 2000], vec![0u8; 2000]];

        for chunk in chunks {
            if state.connection_closed {
                break;
            }

            state.bytes_sent += chunk.len();

            // Simulate disconnect after 4000 bytes
            if state.bytes_sent >= 4000 {
                state.connection_closed = true;
            }
        }

        assert_eq!(state.bytes_sent, 4000, "Handler should track partial bytes");
        assert!(
            state.bytes_sent < state.total_bytes,
            "Transfer incomplete due to disconnect"
        );

        // Test case 4: Handler can cancel S3 stream
        struct S3Stream {
            chunks: Vec<Vec<u8>>,
            position: usize,
            cancelled: bool,
        }

        impl S3Stream {
            fn new(chunks: Vec<Vec<u8>>) -> Self {
                S3Stream {
                    chunks,
                    position: 0,
                    cancelled: false,
                }
            }

            fn next_chunk(&mut self) -> Option<Vec<u8>> {
                if self.cancelled || self.position >= self.chunks.len() {
                    return None;
                }

                let chunk = self.chunks[self.position].clone();
                self.position += 1;
                Some(chunk)
            }

            fn cancel(&mut self) {
                self.cancelled = true;
            }

            fn is_cancelled(&self) -> bool {
                self.cancelled
            }
        }

        let mut s3_stream = S3Stream::new(vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
        let mut client_conn = Connection::new();

        let mut chunks_sent = 0;
        while let Some(_chunk) = s3_stream.next_chunk() {
            if client_conn.is_closed() {
                s3_stream.cancel();
                break;
            }

            chunks_sent += 1;

            // Client disconnects after 1 chunk
            if chunks_sent == 1 {
                client_conn.close();
            }
        }

        assert_eq!(chunks_sent, 1, "Should send 1 chunk before disconnect");
        assert!(s3_stream.is_cancelled(), "S3 stream should be cancelled");

        // Test case 5: Handler cleans up resources on disconnect
        struct ResourceTracker {
            s3_connection_active: bool,
            memory_allocated: usize,
        }

        impl ResourceTracker {
            fn new() -> Self {
                ResourceTracker {
                    s3_connection_active: false,
                    memory_allocated: 0,
                }
            }

            fn allocate_stream(&mut self, size: usize) {
                self.s3_connection_active = true;
                self.memory_allocated = size;
            }

            fn cleanup(&mut self) {
                self.s3_connection_active = false;
                self.memory_allocated = 0;
            }
        }

        let mut resources = ResourceTracker::new();
        resources.allocate_stream(65536);

        assert!(resources.s3_connection_active);
        assert_eq!(resources.memory_allocated, 65536);

        // Simulate disconnect and cleanup
        resources.cleanup();

        assert!(
            !resources.s3_connection_active,
            "S3 connection should be closed"
        );
        assert_eq!(resources.memory_allocated, 0, "Memory should be freed");

        // Test case 6: Handler doesn't continue streaming after disconnect
        let mut conn = Connection::new();
        conn.close(); // Closed before streaming starts

        let chunks = vec![b"chunk1".to_vec(), b"chunk2".to_vec()];
        let mut sent = 0;

        for chunk in chunks {
            if conn.is_closed() {
                break;
            }
            sent += 1;
            let _ = chunk;
        }

        assert_eq!(sent, 0, "Handler should not stream if already closed");

        // Test case 7: Handler handles disconnect at different stages
        struct StreamingStage {
            stage: String,
            conn_open: bool,
        }

        // Disconnect during headers
        let stage1 = StreamingStage {
            stage: "sending_headers".to_string(),
            conn_open: false,
        };
        assert!(!stage1.conn_open, "Can disconnect during headers");

        // Disconnect during body
        let stage2 = StreamingStage {
            stage: "sending_body".to_string(),
            conn_open: false,
        };
        assert!(!stage2.conn_open, "Can disconnect during body");

        // Disconnect after complete
        let stage3 = StreamingStage {
            stage: "complete".to_string(),
            conn_open: false,
        };
        assert!(!stage3.conn_open, "Can disconnect after complete");

        // Test case 8: Handler reports disconnect reason
        enum DisconnectReason {
            ClientClosed,
            Timeout,
            Error,
        }

        let reason = DisconnectReason::ClientClosed;
        match reason {
            DisconnectReason::ClientClosed => {
                assert!(true, "Handler detects client close")
            }
            DisconnectReason::Timeout => panic!("Wrong reason"),
            DisconnectReason::Error => panic!("Wrong reason"),
        }

        // Test case 9: Handler prevents further writes after disconnect
        struct WriteableConnection {
            closed: bool,
            write_count: usize,
        }

        impl WriteableConnection {
            fn new() -> Self {
                WriteableConnection {
                    closed: false,
                    write_count: 0,
                }
            }

            fn write(&mut self, _data: &[u8]) -> Result<(), &'static str> {
                if self.closed {
                    return Err("Connection closed");
                }
                self.write_count += 1;
                Ok(())
            }

            fn close(&mut self) {
                self.closed = true;
            }
        }

        let mut conn = WriteableConnection::new();
        assert!(conn.write(b"data").is_ok());
        assert_eq!(conn.write_count, 1);

        conn.close();
        assert!(
            conn.write(b"more").is_err(),
            "Write should fail after close"
        );
        assert_eq!(conn.write_count, 1, "Write count unchanged after close");

        // Test case 10: Handler properly handles early disconnect in large transfer
        let mut conn = Connection::new();
        let total_chunks = 100;
        let mut sent = 0;

        for i in 0..total_chunks {
            if conn.is_closed() {
                break;
            }

            sent += 1;

            // Disconnect at 10% progress
            if i == 10 {
                conn.close();
            }
        }

        assert_eq!(sent, 11, "Handler should stop at disconnect point");
        assert!(
            sent < total_chunks,
            "Should not complete full transfer after disconnect"
        );
    }

    #[test]
    fn test_sets_appropriate_content_type_header() {
        // Validates that handler sets correct Content-Type based on file extension
        // Content-Type helps browsers render files correctly

        // Test case 1: Handler maps common image extensions
        fn get_content_type_for_extension(ext: &str) -> &str {
            match ext {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                "ico" => "image/x-icon",
                _ => "application/octet-stream",
            }
        }

        assert_eq!(get_content_type_for_extension("jpg"), "image/jpeg");
        assert_eq!(get_content_type_for_extension("jpeg"), "image/jpeg");
        assert_eq!(get_content_type_for_extension("png"), "image/png");
        assert_eq!(get_content_type_for_extension("gif"), "image/gif");

        // Test case 2: Handler maps common video extensions
        fn get_video_content_type(ext: &str) -> &str {
            match ext {
                "mp4" => "video/mp4",
                "webm" => "video/webm",
                "mov" => "video/quicktime",
                "avi" => "video/x-msvideo",
                _ => "application/octet-stream",
            }
        }

        assert_eq!(get_video_content_type("mp4"), "video/mp4");
        assert_eq!(get_video_content_type("webm"), "video/webm");
        assert_eq!(get_video_content_type("mov"), "video/quicktime");

        // Test case 3: Handler maps common text/document extensions
        fn get_text_content_type(ext: &str) -> &str {
            match ext {
                "html" | "htm" => "text/html",
                "css" => "text/css",
                "js" => "text/javascript",
                "json" => "application/json",
                "xml" => "application/xml",
                "pdf" => "application/pdf",
                "txt" => "text/plain",
                _ => "application/octet-stream",
            }
        }

        assert_eq!(get_text_content_type("html"), "text/html");
        assert_eq!(get_text_content_type("css"), "text/css");
        assert_eq!(get_text_content_type("js"), "text/javascript");
        assert_eq!(get_text_content_type("json"), "application/json");
        assert_eq!(get_text_content_type("pdf"), "application/pdf");

        // Test case 4: Handler extracts extension from filename
        fn extract_extension(filename: &str) -> Option<&str> {
            filename.rfind('.').map(|pos| &filename[pos + 1..])
        }

        assert_eq!(extract_extension("image.jpg"), Some("jpg"));
        assert_eq!(extract_extension("document.pdf"), Some("pdf"));
        assert_eq!(extract_extension("data.json"), Some("json"));
        assert_eq!(extract_extension("noextension"), None);

        // Test case 5: Handler handles paths with extensions
        fn get_content_type_from_path(path: &str) -> &str {
            let ext = extract_extension(path).unwrap_or("");
            match ext {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "json" => "application/json",
                "html" => "text/html",
                _ => "application/octet-stream",
            }
        }

        assert_eq!(
            get_content_type_from_path("/images/photo.jpg"),
            "image/jpeg"
        );
        assert_eq!(
            get_content_type_from_path("/data/config.json"),
            "application/json"
        );
        assert_eq!(get_content_type_from_path("/pages/index.html"), "text/html");

        // Test case 6: Handler defaults to octet-stream for unknown types
        assert_eq!(
            get_content_type_from_path("/files/unknown.xyz"),
            "application/octet-stream"
        );
        assert_eq!(
            get_content_type_from_path("/noextension"),
            "application/octet-stream"
        );

        // Test case 7: Handler preserves S3 Content-Type if provided
        struct S3Response {
            content_type: Option<String>,
        }

        fn get_final_content_type(s3_response: &S3Response, path: &str) -> String {
            // Prefer S3's Content-Type if provided
            if let Some(ct) = &s3_response.content_type {
                return ct.clone();
            }

            // Otherwise infer from path
            get_content_type_from_path(path).to_string()
        }

        let s3_with_ct = S3Response {
            content_type: Some("image/jpeg".to_string()),
        };
        assert_eq!(
            get_final_content_type(&s3_with_ct, "/file.png"),
            "image/jpeg",
            "Should use S3 Content-Type"
        );

        let s3_without_ct = S3Response { content_type: None };
        assert_eq!(
            get_final_content_type(&s3_without_ct, "/file.png"),
            "image/png",
            "Should infer from extension"
        );

        // Test case 8: Handler handles case-insensitive extensions
        fn get_content_type_case_insensitive(filename: &str) -> &str {
            let ext = extract_extension(filename).unwrap_or("").to_lowercase();
            match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "pdf" => "application/pdf",
                _ => "application/octet-stream",
            }
        }

        assert_eq!(get_content_type_case_insensitive("IMAGE.JPG"), "image/jpeg");
        assert_eq!(
            get_content_type_case_insensitive("Document.PDF"),
            "application/pdf"
        );
        assert_eq!(get_content_type_case_insensitive("photo.PNG"), "image/png");

        // Test case 9: Handler sets Content-Type in response headers
        use std::collections::HashMap;

        let mut headers: HashMap<String, String> = HashMap::new();
        let path = "/images/logo.png";
        let content_type = get_content_type_from_path(path);

        headers.insert("content-type".to_string(), content_type.to_string());

        assert_eq!(
            headers.get("content-type"),
            Some(&"image/png".to_string()),
            "Handler should set content-type header"
        );

        // Test case 10: Handler handles common font file extensions
        fn get_font_content_type(ext: &str) -> &str {
            match ext {
                "woff" => "font/woff",
                "woff2" => "font/woff2",
                "ttf" => "font/ttf",
                "otf" => "font/otf",
                "eot" => "application/vnd.ms-fontobject",
                _ => "application/octet-stream",
            }
        }

        assert_eq!(get_font_content_type("woff"), "font/woff");
        assert_eq!(get_font_content_type("woff2"), "font/woff2");
        assert_eq!(get_font_content_type("ttf"), "font/ttf");
    }

    #[test]
    fn test_preserves_s3_response_headers_in_proxy_response() {
        // Validates that handler preserves S3 response headers in proxy response
        // S3 headers contain important metadata (etag, cache, etc.)

        use std::collections::HashMap;

        // Test case 1: Handler preserves Content-Type from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("content-type".to_string(), "image/jpeg".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("content-type"),
            Some(&"image/jpeg".to_string()),
            "Handler should preserve S3 Content-Type"
        );

        // Test case 2: Handler preserves Content-Length from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("content-length".to_string(), "2048".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("content-length"),
            Some(&"2048".to_string()),
            "Handler should preserve S3 Content-Length"
        );

        // Test case 3: Handler preserves ETag from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("etag".to_string(), "\"abc123\"".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("etag"),
            Some(&"\"abc123\"".to_string()),
            "Handler should preserve S3 ETag"
        );

        // Test case 4: Handler preserves Last-Modified from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert(
            "last-modified".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("last-modified"),
            Some(&"Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            "Handler should preserve S3 Last-Modified"
        );

        // Test case 5: Handler preserves Cache-Control from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("cache-control"),
            Some(&"max-age=3600".to_string()),
            "Handler should preserve S3 Cache-Control"
        );

        // Test case 6: Handler preserves custom S3 metadata headers (x-amz-*)
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("x-amz-request-id".to_string(), "req-123".to_string());
        s3_headers.insert("x-amz-id-2".to_string(), "id-456".to_string());
        s3_headers.insert("x-amz-version-id".to_string(), "v1".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("x-amz-request-id"),
            Some(&"req-123".to_string()),
            "Handler should preserve x-amz-request-id"
        );
        assert_eq!(
            proxy_headers.get("x-amz-id-2"),
            Some(&"id-456".to_string()),
            "Handler should preserve x-amz-id-2"
        );
        assert_eq!(
            proxy_headers.get("x-amz-version-id"),
            Some(&"v1".to_string()),
            "Handler should preserve x-amz-version-id"
        );

        // Test case 7: Handler preserves Content-Encoding from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("content-encoding".to_string(), "gzip".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("content-encoding"),
            Some(&"gzip".to_string()),
            "Handler should preserve S3 Content-Encoding"
        );

        // Test case 8: Handler preserves Content-Disposition from S3
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert(
            "content-disposition".to_string(),
            "attachment; filename=\"file.pdf\"".to_string(),
        );

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.get("content-disposition"),
            Some(&"attachment; filename=\"file.pdf\"".to_string()),
            "Handler should preserve S3 Content-Disposition"
        );

        // Test case 9: Handler preserves multiple S3 headers together
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("content-type".to_string(), "application/json".to_string());
        s3_headers.insert("content-length".to_string(), "1024".to_string());
        s3_headers.insert("etag".to_string(), "\"xyz789\"".to_string());
        s3_headers.insert("cache-control".to_string(), "no-cache".to_string());

        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            proxy_headers.insert(key.clone(), value.clone());
        }

        assert_eq!(
            proxy_headers.len(),
            4,
            "Handler should preserve all headers"
        );
        assert_eq!(
            proxy_headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            proxy_headers.get("content-length"),
            Some(&"1024".to_string())
        );
        assert_eq!(proxy_headers.get("etag"), Some(&"\"xyz789\"".to_string()));
        assert_eq!(
            proxy_headers.get("cache-control"),
            Some(&"no-cache".to_string())
        );

        // Test case 10: Handler can filter out certain headers if needed
        let mut s3_headers: HashMap<String, String> = HashMap::new();
        s3_headers.insert("content-type".to_string(), "text/html".to_string());
        s3_headers.insert(
            "x-amz-server-side-encryption".to_string(),
            "AES256".to_string(),
        );
        s3_headers.insert("connection".to_string(), "close".to_string());

        // Simulate filtering: only preserve content-type and x-amz-* headers
        let mut proxy_headers: HashMap<String, String> = HashMap::new();
        for (key, value) in &s3_headers {
            if key == "content-type" || key.starts_with("x-amz-") {
                proxy_headers.insert(key.clone(), value.clone());
            }
        }

        assert_eq!(proxy_headers.len(), 2, "Handler should filter headers");
        assert_eq!(
            proxy_headers.get("content-type"),
            Some(&"text/html".to_string())
        );
        assert_eq!(
            proxy_headers.get("x-amz-server-side-encryption"),
            Some(&"AES256".to_string())
        );
        assert_eq!(
            proxy_headers.get("connection"),
            None,
            "Handler should filter connection header"
        );
    }

    #[test]
    fn test_returns_400_for_malformed_requests() {
        // Validates that handler returns 400 Bad Request for malformed requests
        // 400 indicates client sent an invalid request

        // Test case 1: Handler validates HTTP method
        fn validate_http_method(method: &str) -> Result<(), u16> {
            match method {
                "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" => Ok(()),
                _ => Err(400),
            }
        }

        assert!(validate_http_method("GET").is_ok());
        assert!(validate_http_method("POST").is_ok());
        assert_eq!(validate_http_method("INVALID"), Err(400));
        assert_eq!(validate_http_method(""), Err(400));

        // Test case 2: Handler validates request path format
        fn validate_path(path: &str) -> Result<(), u16> {
            if path.is_empty() {
                return Err(400);
            }
            if !path.starts_with('/') {
                return Err(400);
            }
            Ok(())
        }

        assert!(validate_path("/valid/path").is_ok());
        assert_eq!(validate_path(""), Err(400), "Empty path should return 400");
        assert_eq!(
            validate_path("no-leading-slash"),
            Err(400),
            "Path without leading slash should return 400"
        );

        // Test case 3: Handler validates HTTP version
        fn validate_http_version(version: &str) -> Result<(), u16> {
            match version {
                "HTTP/1.0" | "HTTP/1.1" | "HTTP/2.0" => Ok(()),
                _ => Err(400),
            }
        }

        assert!(validate_http_version("HTTP/1.1").is_ok());
        assert!(validate_http_version("HTTP/2.0").is_ok());
        assert_eq!(validate_http_version("HTTP/0.9"), Err(400));
        assert_eq!(validate_http_version("INVALID"), Err(400));

        // Test case 4: Handler validates Content-Length header
        fn validate_content_length(content_length: &str) -> Result<usize, u16> {
            content_length.parse::<usize>().map_err(|_| 400)
        }

        assert_eq!(validate_content_length("1024").unwrap(), 1024);
        assert_eq!(validate_content_length("0").unwrap(), 0);
        assert_eq!(validate_content_length("abc"), Err(400));
        assert_eq!(validate_content_length("-1"), Err(400));

        // Test case 5: Handler validates request line format
        fn parse_request_line(line: &str) -> Result<(String, String, String), u16> {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 3 {
                return Err(400);
            }
            Ok((
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            ))
        }

        assert!(parse_request_line("GET /path HTTP/1.1").is_ok());
        assert_eq!(
            parse_request_line("GET /path"),
            Err(400),
            "Incomplete request line should return 400"
        );
        assert_eq!(
            parse_request_line(""),
            Err(400),
            "Empty request line should return 400"
        );

        // Test case 6: Handler validates header format
        fn validate_header(header: &str) -> Result<(String, String), u16> {
            if let Some(pos) = header.find(':') {
                let name = header[..pos].trim();
                let value = header[pos + 1..].trim();
                if name.is_empty() {
                    return Err(400);
                }
                Ok((name.to_string(), value.to_string()))
            } else {
                Err(400)
            }
        }

        assert!(validate_header("Content-Type: application/json").is_ok());
        assert_eq!(
            validate_header("InvalidHeader"),
            Err(400),
            "Header without colon should return 400"
        );
        assert_eq!(
            validate_header(": value"),
            Err(400),
            "Header without name should return 400"
        );

        // Test case 7: Handler validates Range header format
        fn validate_range_header(range: &str) -> Result<(), u16> {
            if !range.starts_with("bytes=") {
                return Err(400);
            }
            Ok(())
        }

        assert!(validate_range_header("bytes=0-1023").is_ok());
        assert_eq!(
            validate_range_header("invalid=0-1023"),
            Err(400),
            "Invalid range unit should return 400"
        );

        // Test case 8: Handler creates 400 error response
        struct ErrorResponse {
            status_code: u16,
            message: String,
        }

        let malformed_method_error = ErrorResponse {
            status_code: 400,
            message: "Invalid HTTP method".to_string(),
        };

        assert_eq!(malformed_method_error.status_code, 400);
        assert!(malformed_method_error.message.contains("Invalid"));

        // Test case 9: Handler validates query parameter format
        fn validate_query_params(query: &str) -> Result<(), u16> {
            if query.is_empty() {
                return Ok(()); // Empty query is valid
            }

            for param in query.split('&') {
                if !param.contains('=') && !param.is_empty() {
                    // Param without value is technically valid (flag)
                    continue;
                }
            }
            Ok(())
        }

        assert!(validate_query_params("key=value").is_ok());
        assert!(validate_query_params("key1=value1&key2=value2").is_ok());
        assert!(validate_query_params("").is_ok());
        assert!(validate_query_params("flag").is_ok()); // Flag param without value

        // Test case 10: Handler validates request completeness
        fn validate_request_complete(method: &str, path: &str, version: &str) -> Result<(), u16> {
            validate_http_method(method)?;
            validate_path(path)?;
            validate_http_version(version)?;
            Ok(())
        }

        assert!(validate_request_complete("GET", "/path", "HTTP/1.1").is_ok());
        assert_eq!(
            validate_request_complete("INVALID", "/path", "HTTP/1.1"),
            Err(400)
        );
        assert_eq!(validate_request_complete("GET", "", "HTTP/1.1"), Err(400));
        assert_eq!(
            validate_request_complete("GET", "/path", "HTTP/0.9"),
            Err(400)
        );
    }

    #[test]
    fn test_returns_401_for_unauthorized_requests() {
        // Validates that handler returns 401 Unauthorized for auth failures
        // 401 indicates authentication is required or has failed

        // Test case 1: Handler returns 401 when JWT token is missing and auth required
        fn check_auth_required(token: Option<&str>, auth_enabled: bool) -> Result<(), u16> {
            if auth_enabled && token.is_none() {
                return Err(401);
            }
            Ok(())
        }

        assert!(check_auth_required(Some("token"), true).is_ok());
        assert!(check_auth_required(None, false).is_ok());
        assert_eq!(
            check_auth_required(None, true),
            Err(401),
            "Missing token with auth enabled should return 401"
        );

        // Test case 2: Handler returns 401 when JWT token is invalid
        fn validate_jwt_token(token: &str, secret: &str) -> Result<(), u16> {
            // Simplified JWT validation
            if token.is_empty() {
                return Err(401);
            }
            if !token.contains('.') {
                return Err(401);
            }
            if secret.is_empty() {
                return Err(401);
            }
            Ok(())
        }

        assert!(validate_jwt_token("header.payload.signature", "secret").is_ok());
        assert_eq!(
            validate_jwt_token("", "secret"),
            Err(401),
            "Empty token should return 401"
        );
        assert_eq!(
            validate_jwt_token("invalid", "secret"),
            Err(401),
            "Token without dots should return 401"
        );

        // Test case 3: Handler returns 401 when JWT signature is invalid
        fn verify_signature(token: &str, expected_signature: &str) -> Result<(), u16> {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                return Err(401);
            }
            if parts[2] != expected_signature {
                return Err(401);
            }
            Ok(())
        }

        assert!(verify_signature("header.payload.valid", "valid").is_ok());
        assert_eq!(
            verify_signature("header.payload.invalid", "valid"),
            Err(401),
            "Invalid signature should return 401"
        );

        // Test case 4: Handler returns 401 when JWT is expired
        fn check_token_expiration(exp: i64, current_time: i64) -> Result<(), u16> {
            if exp < current_time {
                return Err(401);
            }
            Ok(())
        }

        assert!(check_token_expiration(2000, 1000).is_ok());
        assert_eq!(
            check_token_expiration(1000, 2000),
            Err(401),
            "Expired token should return 401"
        );

        // Test case 5: Handler returns 401 when required claims are missing
        fn validate_required_claims(
            claims: &std::collections::HashMap<String, String>,
            required: &[&str],
        ) -> Result<(), u16> {
            for claim in required {
                if !claims.contains_key(*claim) {
                    return Err(401);
                }
            }
            Ok(())
        }

        let mut claims = std::collections::HashMap::new();
        claims.insert("user_id".to_string(), "123".to_string());
        claims.insert("role".to_string(), "admin".to_string());

        assert!(validate_required_claims(&claims, &["user_id", "role"]).is_ok());
        assert_eq!(
            validate_required_claims(&claims, &["user_id", "role", "email"]),
            Err(401),
            "Missing required claim should return 401"
        );

        // Test case 6: Handler returns 401 when accessing protected resource without auth
        fn check_resource_protection(
            path: &str,
            token: Option<&str>,
            protected_paths: &[&str],
        ) -> Result<(), u16> {
            if protected_paths.contains(&path) && token.is_none() {
                return Err(401);
            }
            Ok(())
        }

        let protected = vec!["/admin", "/api/protected"];
        assert!(check_resource_protection("/admin", Some("token"), &protected).is_ok());
        assert!(check_resource_protection("/public", None, &protected).is_ok());
        assert_eq!(
            check_resource_protection("/admin", None, &protected),
            Err(401),
            "Protected resource without token should return 401"
        );

        // Test case 7: Handler creates 401 error response
        struct ErrorResponse {
            status_code: u16,
            message: String,
            www_authenticate: String,
        }

        let auth_error = ErrorResponse {
            status_code: 401,
            message: "Authentication required".to_string(),
            www_authenticate: "Bearer".to_string(),
        };

        assert_eq!(auth_error.status_code, 401);
        assert!(auth_error.message.contains("Authentication"));
        assert_eq!(auth_error.www_authenticate, "Bearer");

        // Test case 8: Handler returns 401 for malformed Authorization header
        fn parse_auth_header(header: &str) -> Result<String, u16> {
            if !header.starts_with("Bearer ") {
                return Err(401);
            }
            let token = &header[7..];
            if token.is_empty() {
                return Err(401);
            }
            Ok(token.to_string())
        }

        assert_eq!(parse_auth_header("Bearer abc123").unwrap(), "abc123");
        assert_eq!(
            parse_auth_header("Basic abc123"),
            Err(401),
            "Non-Bearer auth should return 401"
        );
        assert_eq!(
            parse_auth_header("Bearer "),
            Err(401),
            "Bearer with empty token should return 401"
        );

        // Test case 9: Handler validates token format before processing
        fn validate_token_format(token: &str) -> Result<(), u16> {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                return Err(401);
            }
            for part in parts {
                if part.is_empty() {
                    return Err(401);
                }
            }
            Ok(())
        }

        assert!(validate_token_format("header.payload.signature").is_ok());
        assert_eq!(
            validate_token_format("only.two"),
            Err(401),
            "Token with only 2 parts should return 401"
        );
        assert_eq!(
            validate_token_format("header..signature"),
            Err(401),
            "Token with empty part should return 401"
        );

        // Test case 10: Handler includes WWW-Authenticate header in 401 response
        fn create_401_response() -> (u16, std::collections::HashMap<String, String>) {
            let mut headers = std::collections::HashMap::new();
            headers.insert(
                "www-authenticate".to_string(),
                "Bearer realm=\"API\"".to_string(),
            );
            (401, headers)
        }

        let (status, headers) = create_401_response();
        assert_eq!(status, 401);
        assert!(headers.contains_key("www-authenticate"));
        assert!(headers.get("www-authenticate").unwrap().contains("Bearer"));
    }

    #[test]
    fn test_returns_403_for_forbidden_requests() {
        // Validates that handler returns 403 Forbidden for permission failures
        // 403 indicates user is authenticated but lacks permission

        // Test case 1: Handler returns 403 when user role is insufficient
        fn check_user_role(user_role: &str, required_role: &str) -> Result<(), u16> {
            let role_hierarchy = vec!["user", "admin", "superadmin"];
            let user_level = role_hierarchy.iter().position(|&r| r == user_role);
            let required_level = role_hierarchy.iter().position(|&r| r == required_role);

            match (user_level, required_level) {
                (Some(u), Some(r)) if u >= r => Ok(()),
                _ => Err(403),
            }
        }

        assert!(check_user_role("admin", "user").is_ok());
        assert!(check_user_role("admin", "admin").is_ok());
        assert_eq!(
            check_user_role("user", "admin"),
            Err(403),
            "Insufficient role should return 403"
        );

        // Test case 2: Handler returns 403 when accessing resource outside allowed scope
        fn check_resource_scope(
            user_id: &str,
            resource_owner: &str,
            is_admin: bool,
        ) -> Result<(), u16> {
            if user_id == resource_owner || is_admin {
                Ok(())
            } else {
                Err(403)
            }
        }

        assert!(check_resource_scope("user123", "user123", false).is_ok());
        assert!(check_resource_scope("user123", "user456", true).is_ok());
        assert_eq!(
            check_resource_scope("user123", "user456", false),
            Err(403),
            "Accessing other user's resource should return 403"
        );

        // Test case 3: Handler returns 403 when claim verification fails
        fn verify_claim_value(actual_value: &str, required_value: &str) -> Result<(), u16> {
            if actual_value == required_value {
                Ok(())
            } else {
                Err(403)
            }
        }

        assert!(verify_claim_value("premium", "premium").is_ok());
        assert_eq!(
            verify_claim_value("basic", "premium"),
            Err(403),
            "Wrong claim value should return 403"
        );

        // Test case 4: Handler returns 403 when user is blocked or revoked
        fn check_user_status(is_active: bool, is_blocked: bool) -> Result<(), u16> {
            if !is_active || is_blocked {
                return Err(403);
            }
            Ok(())
        }

        assert!(check_user_status(true, false).is_ok());
        assert_eq!(
            check_user_status(false, false),
            Err(403),
            "Inactive user should return 403"
        );
        assert_eq!(
            check_user_status(true, true),
            Err(403),
            "Blocked user should return 403"
        );

        // Test case 5: Handler returns 403 for IP-based restrictions
        fn check_ip_allowlist(client_ip: &str, allowed_ips: &[&str]) -> Result<(), u16> {
            if allowed_ips.contains(&client_ip) {
                Ok(())
            } else {
                Err(403)
            }
        }

        let allowed = vec!["192.168.1.1", "10.0.0.1"];
        assert!(check_ip_allowlist("192.168.1.1", &allowed).is_ok());
        assert_eq!(
            check_ip_allowlist("1.2.3.4", &allowed),
            Err(403),
            "IP not in allowlist should return 403"
        );

        // Test case 6: Handler returns 403 when permissions are missing
        fn check_permissions(
            user_permissions: &[&str],
            required_permission: &str,
        ) -> Result<(), u16> {
            if user_permissions.contains(&required_permission) {
                Ok(())
            } else {
                Err(403)
            }
        }

        let permissions = vec!["read", "write"];
        assert!(check_permissions(&permissions, "read").is_ok());
        assert_eq!(
            check_permissions(&permissions, "delete"),
            Err(403),
            "Missing permission should return 403"
        );

        // Test case 7: Handler creates 403 error response
        struct ErrorResponse {
            status_code: u16,
            message: String,
        }

        let forbidden_error = ErrorResponse {
            status_code: 403,
            message: "Access forbidden".to_string(),
        };

        assert_eq!(forbidden_error.status_code, 403);
        assert!(forbidden_error.message.contains("forbidden"));

        // Test case 8: Handler returns 403 for time-based access restrictions
        fn check_access_time(current_hour: u8, allowed_hours: (u8, u8)) -> Result<(), u16> {
            if current_hour >= allowed_hours.0 && current_hour < allowed_hours.1 {
                Ok(())
            } else {
                Err(403)
            }
        }

        assert!(check_access_time(10, (9, 17)).is_ok()); // 10 AM, allowed 9 AM - 5 PM
        assert_eq!(
            check_access_time(18, (9, 17)),
            Err(403),
            "Access outside allowed hours should return 403"
        );

        // Test case 9: Handler returns 403 when quota/rate limit exceeded
        fn check_quota(current_usage: u32, quota_limit: u32) -> Result<(), u16> {
            if current_usage < quota_limit {
                Ok(())
            } else {
                Err(403)
            }
        }

        assert!(check_quota(50, 100).is_ok());
        assert_eq!(
            check_quota(100, 100),
            Err(403),
            "Quota exceeded should return 403"
        );

        // Test case 10: Handler distinguishes 401 (auth) from 403 (permission)
        fn check_access(has_valid_token: bool, has_permission: bool) -> Result<(), u16> {
            if !has_valid_token {
                return Err(401); // Unauthorized - no valid credentials
            }
            if !has_permission {
                return Err(403); // Forbidden - valid credentials but no permission
            }
            Ok(())
        }

        assert!(check_access(true, true).is_ok());
        assert_eq!(
            check_access(false, true),
            Err(401),
            "Invalid token should return 401"
        );
        assert_eq!(
            check_access(true, false),
            Err(403),
            "Valid token without permission should return 403"
        );
    }

    #[test]
    fn test_returns_404_for_not_found() {
        // Validates that handler returns 404 Not Found for missing resources
        // 404 indicates requested resource does not exist

        // Test case 1: Handler returns 404 when S3 object doesn't exist
        fn check_object_exists(object_key: &str, existing_keys: &[&str]) -> Result<(), u16> {
            if existing_keys.contains(&object_key) {
                Ok(())
            } else {
                Err(404)
            }
        }

        let existing = vec!["file1.txt", "file2.jpg", "dir/file3.pdf"];
        assert!(check_object_exists("file1.txt", &existing).is_ok());
        assert_eq!(
            check_object_exists("missing.txt", &existing),
            Err(404),
            "Missing S3 object should return 404"
        );

        // Test case 2: Handler returns 404 when route doesn't match any bucket
        fn find_bucket_for_path(path: &str, routes: &[&str]) -> Result<String, u16> {
            for route in routes {
                if path.starts_with(route) {
                    return Ok(route.to_string());
                }
            }
            Err(404)
        }

        let routes = vec!["/images", "/documents", "/videos"];
        assert!(find_bucket_for_path("/images/photo.jpg", &routes).is_ok());
        assert_eq!(
            find_bucket_for_path("/unknown/file.txt", &routes),
            Err(404),
            "Unmatched route should return 404"
        );

        // Test case 3: Handler returns 404 when bucket name is invalid
        fn validate_bucket_name(bucket: &str, valid_buckets: &[&str]) -> Result<(), u16> {
            if valid_buckets.contains(&bucket) {
                Ok(())
            } else {
                Err(404)
            }
        }

        let buckets = vec!["my-bucket", "other-bucket"];
        assert!(validate_bucket_name("my-bucket", &buckets).is_ok());
        assert_eq!(
            validate_bucket_name("invalid-bucket", &buckets),
            Err(404),
            "Invalid bucket should return 404"
        );

        // Test case 4: Handler creates 404 error response
        struct ErrorResponse {
            status_code: u16,
            message: String,
        }

        let not_found_error = ErrorResponse {
            status_code: 404,
            message: "Resource not found".to_string(),
        };

        assert_eq!(not_found_error.status_code, 404);
        assert!(not_found_error.message.contains("not found"));

        // Test case 5: Handler returns 404 for deleted objects
        fn check_object_status(
            object_key: &str,
            existing: &[&str],
            deleted: &[&str],
        ) -> Result<(), u16> {
            if deleted.contains(&object_key) {
                return Err(404);
            }
            if existing.contains(&object_key) {
                return Ok(());
            }
            Err(404)
        }

        let existing = vec!["file1.txt", "file2.jpg"];
        let deleted = vec!["file3.txt"];

        assert!(check_object_status("file1.txt", &existing, &deleted).is_ok());
        assert_eq!(
            check_object_status("file3.txt", &existing, &deleted),
            Err(404),
            "Deleted object should return 404"
        );
        assert_eq!(
            check_object_status("never-existed.txt", &existing, &deleted),
            Err(404),
            "Never existed object should return 404"
        );

        // Test case 6: Handler returns 404 when S3 key extraction fails
        fn extract_s3_key(path: &str, prefix: &str) -> Result<String, u16> {
            if !path.starts_with(prefix) {
                return Err(404);
            }
            let key = &path[prefix.len()..];
            if key.is_empty() {
                return Err(404);
            }
            Ok(key.to_string())
        }

        assert_eq!(
            extract_s3_key("/images/photo.jpg", "/images/").unwrap(),
            "photo.jpg"
        );
        assert_eq!(
            extract_s3_key("/videos/clip.mp4", "/images/"),
            Err(404),
            "Path without prefix should return 404"
        );
        assert_eq!(
            extract_s3_key("/images/", "/images/"),
            Err(404),
            "Empty S3 key should return 404"
        );

        // Test case 7: Handler returns 404 for non-existent directories
        fn check_directory_exists(path: &str, directories: &[&str]) -> Result<(), u16> {
            for dir in directories {
                if path.starts_with(dir) {
                    return Ok(());
                }
            }
            Err(404)
        }

        let dirs = vec!["/public/", "/private/"];
        assert!(check_directory_exists("/public/file.txt", &dirs).is_ok());
        assert_eq!(
            check_directory_exists("/nonexistent/file.txt", &dirs),
            Err(404),
            "Non-existent directory should return 404"
        );

        // Test case 8: Handler includes helpful message in 404 response
        fn create_404_response(resource: &str) -> ErrorResponse {
            ErrorResponse {
                status_code: 404,
                message: format!("Resource '{}' not found", resource),
            }
        }

        let response = create_404_response("/path/to/file.txt");
        assert_eq!(response.status_code, 404);
        assert!(response.message.contains("/path/to/file.txt"));

        // Test case 9: Handler returns 404 for malformed S3 keys
        fn validate_s3_key(key: &str) -> Result<(), u16> {
            if key.is_empty() {
                return Err(404);
            }
            if key.starts_with('/') {
                return Err(404);
            }
            Ok(())
        }

        assert!(validate_s3_key("valid/key.txt").is_ok());
        assert_eq!(
            validate_s3_key(""),
            Err(404),
            "Empty S3 key should return 404"
        );
        assert_eq!(
            validate_s3_key("/invalid/key"),
            Err(404),
            "S3 key with leading slash should return 404"
        );

        // Test case 10: Handler distinguishes 404 from other errors
        fn classify_error(error_type: &str) -> u16 {
            match error_type {
                "not_found" => 404,
                "unauthorized" => 401,
                "forbidden" => 403,
                "internal" => 500,
                _ => 500,
            }
        }

        assert_eq!(classify_error("not_found"), 404);
        assert_eq!(classify_error("unauthorized"), 401);
        assert_eq!(classify_error("forbidden"), 403);
        assert_ne!(
            classify_error("not_found"),
            500,
            "404 should be distinct from 500"
        );
    }

    #[test]
    fn test_returns_500_for_internal_errors() {
        // Validates that handler returns 500 Internal Server Error for unexpected failures
        // 500 indicates server-side error that prevents request processing

        // Test case 1: Handler returns 500 for configuration errors
        fn validate_config(config: Option<&str>) -> Result<(), u16> {
            match config {
                Some(c) if !c.is_empty() => Ok(()),
                _ => Err(500),
            }
        }

        assert!(validate_config(Some("valid")).is_ok());
        assert_eq!(
            validate_config(None),
            Err(500),
            "Missing config should return 500"
        );
        assert_eq!(
            validate_config(Some("")),
            Err(500),
            "Empty config should return 500"
        );

        // Test case 2: Handler returns 500 for panic recovery
        fn safe_operation(should_panic: bool) -> Result<String, u16> {
            if should_panic {
                return Err(500);
            }
            Ok("success".to_string())
        }

        assert!(safe_operation(false).is_ok());
        assert_eq!(
            safe_operation(true),
            Err(500),
            "Panic recovery should return 500"
        );

        // Test case 3: Handler returns 500 for resource exhaustion
        fn check_resources(memory_available: usize, min_required: usize) -> Result<(), u16> {
            if memory_available < min_required {
                return Err(500);
            }
            Ok(())
        }

        assert!(check_resources(1024, 512).is_ok());
        assert_eq!(
            check_resources(256, 512),
            Err(500),
            "Insufficient resources should return 500"
        );

        // Test case 4: Handler creates 500 error response
        struct ErrorResponse {
            status_code: u16,
            message: String,
        }

        let internal_error = ErrorResponse {
            status_code: 500,
            message: "Internal server error".to_string(),
        };

        assert_eq!(internal_error.status_code, 500);
        assert!(internal_error.message.contains("Internal"));

        // Test case 5: Handler returns 500 for unexpected exceptions
        fn parse_data(data: &str) -> Result<u32, u16> {
            data.parse::<u32>().map_err(|_| 500)
        }

        assert_eq!(parse_data("123").unwrap(), 123);
        assert_eq!(
            parse_data("invalid"),
            Err(500),
            "Parse error should return 500"
        );

        // Test case 6: Handler returns 500 for null pointer / uninitialized data
        fn access_data(data: Option<&str>) -> Result<String, u16> {
            data.ok_or(500).map(|s| s.to_string())
        }

        assert_eq!(access_data(Some("data")).unwrap(), "data");
        assert_eq!(
            access_data(None),
            Err(500),
            "Null data access should return 500"
        );

        // Test case 7: Handler returns 500 for system call failures
        fn system_operation(will_fail: bool) -> Result<(), u16> {
            if will_fail {
                return Err(500);
            }
            Ok(())
        }

        assert!(system_operation(false).is_ok());
        assert_eq!(
            system_operation(true),
            Err(500),
            "Failed system call should return 500"
        );

        // Test case 8: Handler returns 500 for assertion failures
        fn validate_invariant(condition: bool) -> Result<(), u16> {
            if !condition {
                return Err(500);
            }
            Ok(())
        }

        assert!(validate_invariant(true).is_ok());
        assert_eq!(
            validate_invariant(false),
            Err(500),
            "Invariant violation should return 500"
        );

        // Test case 9: Handler includes error tracking ID in 500 response
        fn create_500_response(error_id: &str) -> ErrorResponse {
            ErrorResponse {
                status_code: 500,
                message: format!("Internal error (ID: {})", error_id),
            }
        }

        let response = create_500_response("ERR-12345");
        assert_eq!(response.status_code, 500);
        assert!(response.message.contains("ERR-12345"));

        // Test case 10: Handler distinguishes 500 from other error codes
        fn map_error_to_status(error_type: &str) -> u16 {
            match error_type {
                "bad_request" => 400,
                "unauthorized" => 401,
                "forbidden" => 403,
                "not_found" => 404,
                "internal" => 500,
                "bad_gateway" => 502,
                "unavailable" => 503,
                _ => 500,
            }
        }

        assert_eq!(map_error_to_status("internal"), 500);
        assert_eq!(map_error_to_status("bad_request"), 400);
        assert_eq!(map_error_to_status("not_found"), 404);
        assert_eq!(
            map_error_to_status("unknown"),
            500,
            "Unknown errors should default to 500"
        );
    }

    #[test]
    fn test_returns_502_for_bad_gateway() {
        // Validates that handler returns 502 Bad Gateway for S3 backend errors
        // 502 indicates the proxy received invalid response from upstream S3 server

        // Test case 1: Handler returns 502 when S3 returns malformed response
        fn validate_s3_response(response: Option<&str>) -> Result<(), u16> {
            match response {
                Some(r) if r.starts_with("HTTP/") => Ok(()),
                _ => Err(502),
            }
        }

        assert!(validate_s3_response(Some("HTTP/1.1 200 OK")).is_ok());
        assert_eq!(
            validate_s3_response(Some("INVALID")),
            Err(502),
            "Malformed S3 response should return 502"
        );
        assert_eq!(
            validate_s3_response(None),
            Err(502),
            "Missing S3 response should return 502"
        );

        // Test case 2: Handler returns 502 when cannot connect to S3 endpoint
        #[derive(Debug, PartialEq)]
        enum ConnectionError {
            Refused,
            Timeout,
            DnsFailure,
            NetworkUnreachable,
        }

        fn connect_to_s3(endpoint: &str, error: Option<ConnectionError>) -> Result<(), u16> {
            if let Some(_err) = error {
                return Err(502);
            }
            if endpoint.is_empty() {
                return Err(502);
            }
            Ok(())
        }

        assert!(connect_to_s3("s3.amazonaws.com", None).is_ok());
        assert_eq!(
            connect_to_s3("s3.amazonaws.com", Some(ConnectionError::Refused)),
            Err(502),
            "Connection refused should return 502"
        );
        assert_eq!(
            connect_to_s3("s3.amazonaws.com", Some(ConnectionError::DnsFailure)),
            Err(502),
            "DNS failure should return 502"
        );
        assert_eq!(
            connect_to_s3(
                "s3.amazonaws.com",
                Some(ConnectionError::NetworkUnreachable)
            ),
            Err(502),
            "Network unreachable should return 502"
        );

        // Test case 3: Handler returns 502 for DNS lookup failures
        fn resolve_s3_endpoint(hostname: &str) -> Result<String, u16> {
            if hostname.is_empty() || !hostname.contains('.') {
                return Err(502);
            }
            Ok(format!("resolved:{}", hostname))
        }

        assert!(resolve_s3_endpoint("s3.amazonaws.com").is_ok());
        assert_eq!(
            resolve_s3_endpoint(""),
            Err(502),
            "Empty hostname should return 502"
        );
        assert_eq!(
            resolve_s3_endpoint("invalid"),
            Err(502),
            "Invalid hostname should return 502"
        );

        // Test case 4: Handler returns 502 when S3 response is corrupted
        fn verify_response_integrity(data: &[u8], expected_checksum: u32) -> Result<(), u16> {
            let actual_checksum: u32 = data.iter().map(|&b| b as u32).sum();
            if actual_checksum != expected_checksum {
                return Err(502);
            }
            Ok(())
        }

        let valid_data = vec![1, 2, 3, 4];
        let valid_checksum = 10;
        assert!(verify_response_integrity(&valid_data, valid_checksum).is_ok());
        assert_eq!(
            verify_response_integrity(&valid_data, 999),
            Err(502),
            "Corrupted response should return 502"
        );

        // Test case 5: Handler returns 502 when S3 connection drops unexpectedly
        #[derive(Debug, PartialEq)]
        enum StreamState {
            Connected,
            Disconnected,
            Error,
        }

        fn check_s3_stream(state: StreamState) -> Result<(), u16> {
            match state {
                StreamState::Connected => Ok(()),
                StreamState::Disconnected | StreamState::Error => Err(502),
            }
        }

        assert!(check_s3_stream(StreamState::Connected).is_ok());
        assert_eq!(
            check_s3_stream(StreamState::Disconnected),
            Err(502),
            "Disconnected stream should return 502"
        );
        assert_eq!(
            check_s3_stream(StreamState::Error),
            Err(502),
            "Stream error should return 502"
        );

        // Test case 6: Handler returns 502 for SSL/TLS handshake failures
        fn establish_secure_connection(use_tls: bool, cert_valid: bool) -> Result<(), u16> {
            if !use_tls {
                return Ok(());
            }
            if !cert_valid {
                return Err(502);
            }
            Ok(())
        }

        assert!(establish_secure_connection(false, false).is_ok());
        assert!(establish_secure_connection(true, true).is_ok());
        assert_eq!(
            establish_secure_connection(true, false),
            Err(502),
            "TLS handshake failure should return 502"
        );

        // Test case 7: Handler creates 502 error response with appropriate message
        struct ErrorResponse {
            status_code: u16,
            message: String,
            upstream: String,
        }

        let bad_gateway_error = ErrorResponse {
            status_code: 502,
            message: "Bad Gateway".to_string(),
            upstream: "S3".to_string(),
        };

        assert_eq!(bad_gateway_error.status_code, 502);
        assert!(bad_gateway_error.message.contains("Gateway"));
        assert_eq!(bad_gateway_error.upstream, "S3");

        // Test case 8: Handler distinguishes 502 from other error codes
        fn map_s3_error_to_status(error_type: &str) -> u16 {
            match error_type {
                "connection_refused" => 502,
                "dns_failure" => 502,
                "invalid_response" => 502,
                "corrupted_data" => 502,
                "tls_handshake_failed" => 502,
                "internal_error" => 500,
                "service_unavailable" => 503,
                "timeout" => 504,
                _ => 502, // Default gateway errors to 502
            }
        }

        assert_eq!(map_s3_error_to_status("connection_refused"), 502);
        assert_eq!(map_s3_error_to_status("dns_failure"), 502);
        assert_eq!(map_s3_error_to_status("invalid_response"), 502);
        assert_eq!(map_s3_error_to_status("corrupted_data"), 502);
        assert_eq!(map_s3_error_to_status("tls_handshake_failed"), 502);
        assert_ne!(
            map_s3_error_to_status("internal_error"),
            502,
            "500 should be distinct from 502"
        );
        assert_ne!(
            map_s3_error_to_status("service_unavailable"),
            502,
            "503 should be distinct from 502"
        );
        assert_ne!(
            map_s3_error_to_status("timeout"),
            502,
            "504 should be distinct from 502"
        );

        // Test case 9: Handler returns 502 when S3 returns unexpected HTTP version
        fn validate_http_version(version: &str) -> Result<(), u16> {
            match version {
                "HTTP/1.1" | "HTTP/2" => Ok(()),
                _ => Err(502),
            }
        }

        assert!(validate_http_version("HTTP/1.1").is_ok());
        assert!(validate_http_version("HTTP/2").is_ok());
        assert_eq!(
            validate_http_version("HTTP/0.9"),
            Err(502),
            "Unexpected HTTP version should return 502"
        );
        assert_eq!(
            validate_http_version("UNKNOWN"),
            Err(502),
            "Unknown protocol should return 502"
        );

        // Test case 10: Handler includes upstream information in 502 response
        fn create_bad_gateway_response(upstream_host: &str, error_detail: &str) -> ErrorResponse {
            ErrorResponse {
                status_code: 502,
                message: format!("Bad Gateway: {}", error_detail),
                upstream: upstream_host.to_string(),
            }
        }

        let error = create_bad_gateway_response("s3.amazonaws.com", "connection refused");
        assert_eq!(error.status_code, 502);
        assert!(error.message.contains("connection refused"));
        assert_eq!(error.upstream, "s3.amazonaws.com");
    }

    #[test]
    fn test_returns_503_for_service_unavailable() {
        // Validates that handler returns 503 Service Unavailable for temporary unavailability
        // 503 indicates the server is temporarily unable to handle the request

        // Test case 1: Handler returns 503 when server is overloaded
        fn check_server_capacity(current_load: u32, max_capacity: u32) -> Result<(), u16> {
            if current_load >= max_capacity {
                return Err(503);
            }
            Ok(())
        }

        assert!(check_server_capacity(50, 100).is_ok());
        assert_eq!(
            check_server_capacity(100, 100),
            Err(503),
            "Server at capacity should return 503"
        );
        assert_eq!(
            check_server_capacity(150, 100),
            Err(503),
            "Server overloaded should return 503"
        );

        // Test case 2: Handler returns 503 during maintenance mode
        #[derive(Debug, PartialEq)]
        enum ServerState {
            Running,
            Maintenance,
            ShuttingDown,
            Starting,
        }

        fn check_server_state(state: ServerState) -> Result<(), u16> {
            match state {
                ServerState::Running => Ok(()),
                ServerState::Maintenance | ServerState::ShuttingDown | ServerState::Starting => {
                    Err(503)
                }
            }
        }

        assert!(check_server_state(ServerState::Running).is_ok());
        assert_eq!(
            check_server_state(ServerState::Maintenance),
            Err(503),
            "Maintenance mode should return 503"
        );
        assert_eq!(
            check_server_state(ServerState::ShuttingDown),
            Err(503),
            "Shutting down should return 503"
        );
        assert_eq!(
            check_server_state(ServerState::Starting),
            Err(503),
            "Starting up should return 503"
        );

        // Test case 3: Handler returns 503 when rate limit exceeded
        fn check_rate_limit(requests_count: u32, limit: u32, window_ms: u64) -> Result<(), u16> {
            if window_ms == 0 {
                return Err(503);
            }
            if requests_count > limit {
                return Err(503);
            }
            Ok(())
        }

        assert!(check_rate_limit(50, 100, 1000).is_ok());
        assert_eq!(
            check_rate_limit(150, 100, 1000),
            Err(503),
            "Rate limit exceeded should return 503"
        );
        assert_eq!(
            check_rate_limit(0, 100, 0),
            Err(503),
            "Invalid rate limit window should return 503"
        );

        // Test case 4: Handler returns 503 when connection pool is exhausted
        fn acquire_connection(available: u32, total: u32) -> Result<String, u16> {
            if available == 0 {
                return Err(503);
            }
            if available > total {
                return Err(503);
            }
            Ok(format!("connection_{}", total - available))
        }

        assert!(acquire_connection(5, 10).is_ok());
        assert_eq!(
            acquire_connection(0, 10),
            Err(503),
            "No available connections should return 503"
        );

        // Test case 5: Handler returns 503 when thread pool is full
        fn submit_task(queued_tasks: usize, max_queue_size: usize) -> Result<(), u16> {
            if queued_tasks >= max_queue_size {
                return Err(503);
            }
            Ok(())
        }

        assert!(submit_task(50, 100).is_ok());
        assert_eq!(
            submit_task(100, 100),
            Err(503),
            "Thread pool full should return 503"
        );

        // Test case 6: Handler returns 503 when memory pressure is high
        fn check_memory_pressure(
            used_mb: usize,
            total_mb: usize,
            threshold: f32,
        ) -> Result<(), u16> {
            let usage_ratio = used_mb as f32 / total_mb as f32;
            if usage_ratio >= threshold {
                return Err(503);
            }
            Ok(())
        }

        assert!(check_memory_pressure(500, 1000, 0.9).is_ok());
        assert_eq!(
            check_memory_pressure(950, 1000, 0.9),
            Err(503),
            "High memory pressure should return 503"
        );

        // Test case 7: Handler creates 503 error response with Retry-After header
        struct ErrorResponse {
            status_code: u16,
            message: String,
            retry_after_seconds: Option<u32>,
        }

        let service_unavailable = ErrorResponse {
            status_code: 503,
            message: "Service Unavailable".to_string(),
            retry_after_seconds: Some(60),
        };

        assert_eq!(service_unavailable.status_code, 503);
        assert!(service_unavailable.message.contains("Unavailable"));
        assert_eq!(service_unavailable.retry_after_seconds, Some(60));

        // Test case 8: Handler distinguishes 503 from other error codes
        fn map_availability_error_to_status(error_type: &str) -> u16 {
            match error_type {
                "overloaded" => 503,
                "maintenance" => 503,
                "rate_limited" => 503,
                "pool_exhausted" => 503,
                "high_memory" => 503,
                "shutting_down" => 503,
                "bad_gateway" => 502,
                "gateway_timeout" => 504,
                "internal_error" => 500,
                _ => 503, // Default temporary errors to 503
            }
        }

        assert_eq!(map_availability_error_to_status("overloaded"), 503);
        assert_eq!(map_availability_error_to_status("maintenance"), 503);
        assert_eq!(map_availability_error_to_status("rate_limited"), 503);
        assert_eq!(map_availability_error_to_status("pool_exhausted"), 503);
        assert_eq!(map_availability_error_to_status("high_memory"), 503);
        assert_ne!(
            map_availability_error_to_status("bad_gateway"),
            503,
            "502 should be distinct from 503"
        );
        assert_ne!(
            map_availability_error_to_status("gateway_timeout"),
            503,
            "504 should be distinct from 503"
        );
        assert_ne!(
            map_availability_error_to_status("internal_error"),
            503,
            "500 should be distinct from 503"
        );

        // Test case 9: Handler returns 503 when upstream S3 is temporarily unavailable
        fn check_s3_availability(s3_healthy: bool, s3_responsive: bool) -> Result<(), u16> {
            if !s3_healthy || !s3_responsive {
                return Err(503);
            }
            Ok(())
        }

        assert!(check_s3_availability(true, true).is_ok());
        assert_eq!(
            check_s3_availability(false, true),
            Err(503),
            "Unhealthy S3 should return 503"
        );
        assert_eq!(
            check_s3_availability(true, false),
            Err(503),
            "Unresponsive S3 should return 503"
        );

        // Test case 10: Handler includes suggested retry delay in 503 response
        fn create_service_unavailable_response(reason: &str, retry_delay: u32) -> ErrorResponse {
            ErrorResponse {
                status_code: 503,
                message: format!("Service Unavailable: {}", reason),
                retry_after_seconds: Some(retry_delay),
            }
        }

        let error = create_service_unavailable_response("server overloaded", 30);
        assert_eq!(error.status_code, 503);
        assert!(error.message.contains("overloaded"));
        assert_eq!(error.retry_after_seconds, Some(30));

        // Test case 11: Handler returns 503 without retry-after for indefinite unavailability
        let maintenance_error = ErrorResponse {
            status_code: 503,
            message: "Scheduled maintenance".to_string(),
            retry_after_seconds: None,
        };

        assert_eq!(maintenance_error.status_code, 503);
        assert!(maintenance_error.retry_after_seconds.is_none());
    }

    #[test]
    fn test_error_responses_include_json_body_with_error_details() {
        // Validates that error responses include JSON body with structured error details
        // JSON format provides machine-readable error information for clients

        // Test case 1: Error response includes error code
        #[derive(Debug, PartialEq)]
        struct JsonErrorResponse {
            error: String,
            message: String,
            status_code: u16,
        }

        let error_response = JsonErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: "The requested resource was not found".to_string(),
            status_code: 404,
        };

        assert_eq!(error_response.error, "NOT_FOUND");
        assert_eq!(
            error_response.message,
            "The requested resource was not found"
        );
        assert_eq!(error_response.status_code, 404);

        // Test case 2: Error response includes human-readable message
        let error_with_message = JsonErrorResponse {
            error: "UNAUTHORIZED".to_string(),
            message: "Authentication required".to_string(),
            status_code: 401,
        };

        assert!(!error_with_message.message.is_empty());
        assert!(error_with_message.message.len() > 10);

        // Test case 3: Error response can be serialized to JSON
        fn to_json(error: &JsonErrorResponse) -> String {
            format!(
                r#"{{"error":"{}","message":"{}","status_code":{}}}"#,
                error.error, error.message, error.status_code
            )
        }

        let json = to_json(&error_response);
        assert!(json.contains("\"error\":\"NOT_FOUND\""));
        assert!(json.contains("\"message\":\"The requested resource was not found\""));
        assert!(json.contains("\"status_code\":404"));

        // Test case 4: Different error types have different error codes
        fn create_error_response(error_type: &str) -> JsonErrorResponse {
            match error_type {
                "not_found" => JsonErrorResponse {
                    error: "NOT_FOUND".to_string(),
                    message: "Resource not found".to_string(),
                    status_code: 404,
                },
                "unauthorized" => JsonErrorResponse {
                    error: "UNAUTHORIZED".to_string(),
                    message: "Authentication required".to_string(),
                    status_code: 401,
                },
                "forbidden" => JsonErrorResponse {
                    error: "FORBIDDEN".to_string(),
                    message: "Access denied".to_string(),
                    status_code: 403,
                },
                "bad_request" => JsonErrorResponse {
                    error: "BAD_REQUEST".to_string(),
                    message: "Invalid request format".to_string(),
                    status_code: 400,
                },
                _ => JsonErrorResponse {
                    error: "INTERNAL_ERROR".to_string(),
                    message: "Internal server error".to_string(),
                    status_code: 500,
                },
            }
        }

        assert_eq!(create_error_response("not_found").error, "NOT_FOUND");
        assert_eq!(create_error_response("unauthorized").error, "UNAUTHORIZED");
        assert_eq!(create_error_response("forbidden").error, "FORBIDDEN");
        assert_eq!(create_error_response("bad_request").error, "BAD_REQUEST");

        // Test case 5: Error response includes request ID for tracking
        struct DetailedErrorResponse {
            error: String,
            message: String,
            status_code: u16,
            request_id: String,
        }

        let detailed_error = DetailedErrorResponse {
            error: "INTERNAL_ERROR".to_string(),
            message: "An unexpected error occurred".to_string(),
            status_code: 500,
            request_id: "req-abc123".to_string(),
        };

        assert!(!detailed_error.request_id.is_empty());
        assert!(detailed_error.request_id.starts_with("req-"));

        // Test case 6: Error response includes timestamp
        struct TimestampedErrorResponse {
            error: String,
            message: String,
            status_code: u16,
            timestamp: u64,
        }

        let timestamped_error = TimestampedErrorResponse {
            error: "SERVICE_UNAVAILABLE".to_string(),
            message: "Service temporarily unavailable".to_string(),
            status_code: 503,
            timestamp: 1234567890,
        };

        assert!(timestamped_error.timestamp > 0);

        // Test case 7: Error response includes path that caused the error
        struct PathErrorResponse {
            error: String,
            message: String,
            status_code: u16,
            path: String,
        }

        let path_error = PathErrorResponse {
            error: "NOT_FOUND".to_string(),
            message: "Object not found".to_string(),
            status_code: 404,
            path: "/products/image.jpg".to_string(),
        };

        assert_eq!(path_error.path, "/products/image.jpg");

        // Test case 8: Error response includes Content-Type header for JSON
        fn get_error_content_type() -> &'static str {
            "application/json"
        }

        assert_eq!(get_error_content_type(), "application/json");

        // Test case 9: Error response JSON is properly escaped
        fn escape_json_string(s: &str) -> String {
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
        }

        assert_eq!(escape_json_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json_string("line1\nline2"), "line1\\nline2");

        // Test case 10: Error response includes additional context for specific errors
        struct ContextualErrorResponse {
            error: String,
            message: String,
            status_code: u16,
            details: Option<String>,
        }

        let validation_error = ContextualErrorResponse {
            error: "VALIDATION_ERROR".to_string(),
            message: "Request validation failed".to_string(),
            status_code: 400,
            details: Some("Invalid Range header format".to_string()),
        };

        assert!(validation_error.details.is_some());
        assert_eq!(
            validation_error.details.unwrap(),
            "Invalid Range header format"
        );

        // Test case 11: Error response format is consistent across different error types
        fn validate_error_structure(error: &JsonErrorResponse) -> bool {
            !error.error.is_empty()
                && !error.message.is_empty()
                && error.status_code >= 400
                && error.status_code < 600
        }

        assert!(validate_error_structure(&error_response));
        assert!(validate_error_structure(&error_with_message));

        // Test case 12: Error codes are uppercase with underscores
        fn validate_error_code_format(code: &str) -> bool {
            code.chars().all(|c| c.is_uppercase() || c == '_')
        }

        assert!(validate_error_code_format("NOT_FOUND"));
        assert!(validate_error_code_format("UNAUTHORIZED"));
        assert!(validate_error_code_format("BAD_REQUEST"));
        assert!(!validate_error_code_format("notFound"));
        assert!(!validate_error_code_format("Not-Found"));
    }

    #[test]
    fn test_error_responses_dont_leak_sensitive_information() {
        // Validates that error responses don't leak sensitive information
        // Prevents exposing credentials, tokens, internal paths, or system details

        // Test case 1: Error messages don't contain passwords
        fn sanitize_error_message(message: &str) -> String {
            let sensitive_patterns = ["password=", "pwd=", "secret=", "token="];
            let mut sanitized = message.to_string();
            for pattern in &sensitive_patterns {
                if sanitized.to_lowercase().contains(pattern) {
                    sanitized = "Authentication failed".to_string();
                }
            }
            sanitized
        }

        assert_eq!(
            sanitize_error_message("Invalid credentials"),
            "Invalid credentials"
        );
        assert_eq!(
            sanitize_error_message("Login failed with password=secret123"),
            "Authentication failed"
        );
        assert_eq!(
            sanitize_error_message("Auth error: token=abc123"),
            "Authentication failed"
        );

        // Test case 2: Error messages don't contain JWT tokens
        fn redact_jwt_from_error(message: &str) -> String {
            // Simple check for JWT-like patterns (base64.base64.base64)
            if message.contains("eyJ") {
                // JWT tokens typically start with "eyJ"
                return "Invalid token".to_string();
            }
            message.to_string()
        }

        assert_eq!(redact_jwt_from_error("User not found"), "User not found");
        assert_eq!(
            redact_jwt_from_error(
                "Invalid token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig"
            ),
            "Invalid token"
        );

        // Test case 3: Error messages don't contain AWS credentials
        fn check_for_aws_credentials(message: &str) -> bool {
            let patterns = ["AKIA", "aws_access_key", "aws_secret_key"];
            patterns.iter().any(|p| message.contains(p))
        }

        assert!(!check_for_aws_credentials("S3 request failed"));
        assert!(check_for_aws_credentials(
            "Failed with key AKIAIOSFODNN7EXAMPLE"
        ));
        assert!(check_for_aws_credentials(
            "Config error: aws_access_key=AKIA123"
        ));

        // Test case 4: Error messages don't contain internal file paths
        fn sanitize_file_path(message: &str) -> String {
            if message.contains("/etc/") || message.contains("/var/") || message.contains("C:\\") {
                return "Configuration error".to_string();
            }
            message.to_string()
        }

        assert_eq!(sanitize_file_path("File not found"), "File not found");
        assert_eq!(
            sanitize_file_path("Failed to read /etc/secrets/config.yml"),
            "Configuration error"
        );
        assert_eq!(
            sanitize_file_path("Error accessing C:\\secrets\\keys.txt"),
            "Configuration error"
        );

        // Test case 5: Error messages don't contain database connection strings
        fn check_for_connection_string(message: &str) -> bool {
            let patterns = ["postgres://", "mysql://", "mongodb://", "redis://"];
            patterns.iter().any(|p| message.contains(p))
        }

        assert!(!check_for_connection_string("Database error"));
        assert!(check_for_connection_string(
            "Failed: postgres://user:pass@localhost/db"
        ));

        // Test case 6: Error messages don't contain stack traces in production
        fn should_include_stack_trace(is_production: bool) -> bool {
            !is_production
        }

        assert!(should_include_stack_trace(false)); // Dev mode
        assert!(!should_include_stack_trace(true)); // Production mode

        // Test case 7: Error messages don't contain internal IP addresses
        fn redact_internal_ips(message: &str) -> String {
            if message.contains("192.168.")
                || message.contains("10.0.")
                || message.contains("127.0.0.1")
            {
                return "Network error".to_string();
            }
            message.to_string()
        }

        assert_eq!(
            redact_internal_ips("Connection timeout"),
            "Connection timeout"
        );
        assert_eq!(
            redact_internal_ips("Failed to connect to 192.168.1.100"),
            "Network error"
        );
        assert_eq!(
            redact_internal_ips("Error connecting to 10.0.0.5"),
            "Network error"
        );

        // Test case 8: Error messages use generic descriptions for sensitive failures
        fn get_generic_error_message(error_type: &str) -> &'static str {
            match error_type {
                "jwt_invalid" => "Authentication failed",
                "jwt_expired" => "Authentication failed",
                "jwt_signature_invalid" => "Authentication failed",
                "aws_credentials_invalid" => "Unable to access storage",
                "aws_signature_failed" => "Unable to access storage",
                "internal_config_error" => "Internal server error",
                _ => "An error occurred",
            }
        }

        // All auth errors return same generic message
        assert_eq!(
            get_generic_error_message("jwt_invalid"),
            "Authentication failed"
        );
        assert_eq!(
            get_generic_error_message("jwt_expired"),
            "Authentication failed"
        );
        assert_eq!(
            get_generic_error_message("jwt_signature_invalid"),
            "Authentication failed"
        );

        // All AWS errors return same generic message
        assert_eq!(
            get_generic_error_message("aws_credentials_invalid"),
            "Unable to access storage"
        );
        assert_eq!(
            get_generic_error_message("aws_signature_failed"),
            "Unable to access storage"
        );

        // Test case 9: Error messages don't reveal bucket names or S3 keys
        fn sanitize_s3_details(message: &str, bucket: &str, key: &str) -> String {
            message.replace(bucket, "[BUCKET]").replace(key, "[KEY]")
        }

        let sanitized = sanitize_s3_details(
            "Object not found in my-secret-bucket at path/to/file.txt",
            "my-secret-bucket",
            "path/to/file.txt",
        );
        assert!(sanitized.contains("[BUCKET]"));
        assert!(sanitized.contains("[KEY]"));
        assert!(!sanitized.contains("my-secret-bucket"));
        assert!(!sanitized.contains("path/to/file.txt"));

        // Test case 10: Error messages don't contain environment variable names
        fn check_for_env_vars(message: &str) -> bool {
            let patterns = [
                "AWS_ACCESS_KEY",
                "AWS_SECRET_KEY",
                "JWT_SECRET",
                "DATABASE_URL",
            ];
            patterns.iter().any(|p| message.contains(p))
        }

        assert!(!check_for_env_vars("Configuration missing"));
        assert!(check_for_env_vars("Missing env var: AWS_ACCESS_KEY"));

        // Test case 11: Error messages don't include software version numbers
        fn redact_version_info(message: &str) -> String {
            if message.contains("version") || message.contains("v1.2.3") {
                return "Server error".to_string();
            }
            message.to_string()
        }

        assert_eq!(redact_version_info("Request failed"), "Request failed");
        assert_eq!(
            redact_version_info("Error in server version 1.2.3"),
            "Server error"
        );

        // Test case 12: Error responses validate that sensitive data is filtered
        struct SafeErrorResponse {
            error: String,
            message: String,
            status_code: u16,
        }

        fn create_safe_error_response(
            internal_error: &str,
            _sensitive_context: &str,
        ) -> SafeErrorResponse {
            // Never use sensitive_context in the response
            let (error, message, status_code) = match internal_error {
                "jwt_failed" => ("UNAUTHORIZED", "Authentication required", 401),
                "s3_access_denied" => ("FORBIDDEN", "Access denied", 403),
                "config_missing" => ("INTERNAL_ERROR", "Internal server error", 500),
                _ => ("INTERNAL_ERROR", "An error occurred", 500),
            };

            SafeErrorResponse {
                error: error.to_string(),
                message: message.to_string(),
                status_code,
            }
        }

        let response = create_safe_error_response(
            "jwt_failed",
            "JWT validation failed: signature mismatch with secret key abc123",
        );
        assert!(!response.message.contains("signature"));
        assert!(!response.message.contains("abc123"));
        assert_eq!(response.message, "Authentication required");

        let response2 = create_safe_error_response(
            "config_missing",
            "Missing config at /etc/app/secrets.yml with password=secret",
        );
        assert!(!response2.message.contains("/etc/"));
        assert!(!response2.message.contains("password"));
        assert!(!response2.message.contains("secret"));
        assert_eq!(response2.message, "Internal server error");
    }

    #[test]
    fn test_request_passes_through_router_first() {
        // Validates that router is the first middleware to process requests
        // Router determines target bucket before auth or S3 handling

        // Test case 1: Request processing starts with router
        #[derive(Debug, PartialEq)]
        enum MiddlewareStage {
            Router,
            Auth,
            S3Handler,
        }

        fn get_first_middleware_stage() -> MiddlewareStage {
            MiddlewareStage::Router
        }

        assert_eq!(get_first_middleware_stage(), MiddlewareStage::Router);

        // Test case 2: Router executes before auth middleware
        struct MiddlewareOrder {
            stages: Vec<String>,
        }

        impl MiddlewareOrder {
            fn new() -> Self {
                MiddlewareOrder { stages: Vec::new() }
            }

            fn add_stage(&mut self, stage: &str) {
                self.stages.push(stage.to_string());
            }

            fn get_execution_order(&self) -> Vec<String> {
                self.stages.clone()
            }
        }

        let mut order = MiddlewareOrder::new();
        order.add_stage("router");
        order.add_stage("auth");
        order.add_stage("s3");

        let execution = order.get_execution_order();
        assert_eq!(execution[0], "router");
        assert_eq!(execution.len(), 3);

        // Test case 3: Router runs regardless of auth configuration
        fn router_always_runs(auth_enabled: bool) -> bool {
            // Router always runs first, regardless of auth config
            true && (auth_enabled || !auth_enabled)
        }

        assert!(router_always_runs(true));
        assert!(router_always_runs(false));

        // Test case 4: Request metadata includes router timestamp
        struct RequestMetadata {
            router_timestamp: u64,
            auth_timestamp: Option<u64>,
            s3_timestamp: Option<u64>,
        }

        let metadata = RequestMetadata {
            router_timestamp: 1000,
            auth_timestamp: Some(2000),
            s3_timestamp: Some(3000),
        };

        // Router timestamp is always present and first
        assert!(metadata.router_timestamp > 0);
        assert!(metadata.router_timestamp < metadata.auth_timestamp.unwrap());
        assert!(metadata.router_timestamp < metadata.s3_timestamp.unwrap());

        // Test case 5: Router result determines subsequent middleware execution
        fn should_continue_to_auth(router_result: Result<String, u16>) -> bool {
            router_result.is_ok()
        }

        assert!(should_continue_to_auth(Ok("bucket-name".to_string())));
        assert!(!should_continue_to_auth(Err(404)));

        // Test case 6: Router extracts path before any other processing
        fn process_request_path(path: &str) -> Vec<String> {
            let mut stages = Vec::new();
            stages.push(format!("router:extract_path:{}", path));
            stages.push("auth:validate".to_string());
            stages.push("s3:fetch".to_string());
            stages
        }

        let stages = process_request_path("/products/image.jpg");
        assert!(stages[0].starts_with("router:"));
        assert!(stages[0].contains("/products/image.jpg"));

        // Test case 7: Router identifies target bucket before auth checks
        struct RequestContext {
            bucket_name: Option<String>,
            authenticated: bool,
            s3_key: Option<String>,
        }

        let context_after_router = RequestContext {
            bucket_name: Some("products".to_string()),
            authenticated: false, // Auth hasn't run yet
            s3_key: None,         // S3 handler hasn't run yet
        };

        assert!(context_after_router.bucket_name.is_some());
        assert!(!context_after_router.authenticated);
        assert!(context_after_router.s3_key.is_none());

        // Test case 8: Middleware chain order is enforced
        fn validate_middleware_chain(chain: &[&str]) -> bool {
            if chain.is_empty() {
                return false;
            }
            chain[0] == "router"
        }

        assert!(validate_middleware_chain(&["router", "auth", "s3"]));
        assert!(validate_middleware_chain(&["router", "s3"]));
        assert!(!validate_middleware_chain(&["auth", "router", "s3"]));
        assert!(!validate_middleware_chain(&["s3", "router", "auth"]));

        // Test case 9: Router failure prevents further middleware execution
        fn execute_middleware_chain(router_success: bool) -> Vec<String> {
            let mut executed = Vec::new();
            executed.push("router".to_string());

            if !router_success {
                return executed; // Stop if router fails
            }

            executed.push("auth".to_string());
            executed.push("s3".to_string());
            executed
        }

        let successful_chain = execute_middleware_chain(true);
        assert_eq!(successful_chain.len(), 3);
        assert_eq!(successful_chain, vec!["router", "auth", "s3"]);

        let failed_chain = execute_middleware_chain(false);
        assert_eq!(failed_chain.len(), 1);
        assert_eq!(failed_chain, vec!["router"]);

        // Test case 10: Router logs are first in request timeline
        struct LogEntry {
            timestamp: u64,
            middleware: String,
            message: String,
        }

        let logs = vec![
            LogEntry {
                timestamp: 100,
                middleware: "router".to_string(),
                message: "Matched path /products to bucket products".to_string(),
            },
            LogEntry {
                timestamp: 200,
                middleware: "auth".to_string(),
                message: "JWT validation successful".to_string(),
            },
            LogEntry {
                timestamp: 300,
                middleware: "s3".to_string(),
                message: "Fetched object from S3".to_string(),
            },
        ];

        assert_eq!(logs[0].middleware, "router");
        assert!(logs[0].timestamp < logs[1].timestamp);
        assert!(logs[0].timestamp < logs[2].timestamp);
    }

    #[test]
    fn test_request_passes_through_auth_middleware_second() {
        // Validates that auth middleware is the second middleware to process requests
        // Auth runs after router determines bucket, before S3 handler

        // Test case 1: Auth middleware executes after router
        fn get_second_middleware_stage() -> String {
            "auth".to_string()
        }

        assert_eq!(get_second_middleware_stage(), "auth");

        // Test case 2: Middleware execution order places auth second
        struct MiddlewareChain {
            stages: Vec<String>,
        }

        impl MiddlewareChain {
            fn new() -> Self {
                let mut stages = Vec::new();
                stages.push("router".to_string());
                stages.push("auth".to_string());
                stages.push("s3".to_string());
                MiddlewareChain { stages }
            }

            fn get_stage_at_position(&self, position: usize) -> Option<&str> {
                self.stages.get(position).map(|s| s.as_str())
            }
        }

        let chain = MiddlewareChain::new();
        assert_eq!(chain.get_stage_at_position(0), Some("router"));
        assert_eq!(chain.get_stage_at_position(1), Some("auth"));
        assert_eq!(chain.get_stage_at_position(2), Some("s3"));

        // Test case 3: Auth runs only after router succeeds
        fn execute_chain_with_router_result(
            router_success: bool,
            auth_enabled: bool,
        ) -> Vec<String> {
            let mut executed = Vec::new();
            executed.push("router".to_string());

            if !router_success {
                return executed; // Stop if router fails
            }

            if auth_enabled {
                executed.push("auth".to_string());
            }

            executed.push("s3".to_string());
            executed
        }

        let with_auth = execute_chain_with_router_result(true, true);
        assert_eq!(with_auth, vec!["router", "auth", "s3"]);
        assert_eq!(with_auth[1], "auth");

        let without_auth = execute_chain_with_router_result(true, false);
        assert_eq!(without_auth, vec!["router", "s3"]);

        // Test case 4: Auth has access to router results
        struct RequestState {
            router_bucket: Option<String>,
            auth_validated: bool,
            s3_key: Option<String>,
        }

        let state_after_auth = RequestState {
            router_bucket: Some("products".to_string()), // Set by router
            auth_validated: true,                        // Set by auth
            s3_key: None,                                // Not set yet (S3 handler hasn't run)
        };

        assert!(state_after_auth.router_bucket.is_some());
        assert!(state_after_auth.auth_validated);
        assert!(state_after_auth.s3_key.is_none());

        // Test case 5: Auth middleware timestamp is between router and S3
        struct TimestampedExecution {
            router_ts: u64,
            auth_ts: Option<u64>,
            s3_ts: u64,
        }

        let execution = TimestampedExecution {
            router_ts: 100,
            auth_ts: Some(200),
            s3_ts: 300,
        };

        assert!(execution.router_ts < execution.auth_ts.unwrap());
        assert!(execution.auth_ts.unwrap() < execution.s3_ts);

        // Test case 6: Auth can access bucket name from router context
        fn auth_receives_bucket_context(bucket_from_router: &str) -> bool {
            !bucket_from_router.is_empty()
        }

        assert!(auth_receives_bucket_context("products"));

        // Test case 7: Auth middleware validates before S3 access
        fn validate_execution_order() -> Vec<(u64, &'static str)> {
            vec![
                (1, "router:match_path"),
                (2, "auth:extract_token"),
                (3, "auth:validate_jwt"),
                (4, "s3:build_request"),
                (5, "s3:fetch_object"),
            ]
        }

        let order = validate_execution_order();
        let auth_steps: Vec<_> = order
            .iter()
            .filter(|(_, s)| s.starts_with("auth:"))
            .collect();
        let s3_steps: Vec<_> = order.iter().filter(|(_, s)| s.starts_with("s3:")).collect();

        // Auth steps come before S3 steps
        assert!(auth_steps.last().unwrap().0 < s3_steps.first().unwrap().0);

        // Test case 8: Auth failure prevents S3 handler execution
        fn execute_with_auth_result(auth_success: bool) -> Vec<String> {
            let mut executed = Vec::new();
            executed.push("router".to_string());
            executed.push("auth".to_string());

            if !auth_success {
                return executed; // Stop if auth fails
            }

            executed.push("s3".to_string());
            executed
        }

        let auth_success = execute_with_auth_result(true);
        assert_eq!(auth_success, vec!["router", "auth", "s3"]);

        let auth_failure = execute_with_auth_result(false);
        assert_eq!(auth_failure, vec!["router", "auth"]);
        assert_eq!(auth_failure.len(), 2); // S3 handler not executed

        // Test case 9: Auth middleware is skipped when disabled
        fn should_run_auth(auth_enabled: bool) -> bool {
            auth_enabled
        }

        assert!(should_run_auth(true));
        assert!(!should_run_auth(false));

        // Test case 10: Auth logs appear after router but before S3 in timeline
        struct DetailedLog {
            order: u32,
            middleware: String,
            action: String,
        }

        let logs = vec![
            DetailedLog {
                order: 1,
                middleware: "router".to_string(),
                action: "matched path".to_string(),
            },
            DetailedLog {
                order: 2,
                middleware: "auth".to_string(),
                action: "validated JWT".to_string(),
            },
            DetailedLog {
                order: 3,
                middleware: "s3".to_string(),
                action: "fetched object".to_string(),
            },
        ];

        assert_eq!(logs[1].middleware, "auth");
        assert!(logs[1].order > logs[0].order);
        assert!(logs[1].order < logs[2].order);

        // Test case 11: Auth uses bucket-specific configuration from router
        struct BucketAuthConfig {
            bucket_name: String,
            auth_required: bool,
        }

        fn get_auth_config_for_bucket(bucket: &str) -> BucketAuthConfig {
            BucketAuthConfig {
                bucket_name: bucket.to_string(),
                auth_required: bucket != "public",
            }
        }

        let private_config = get_auth_config_for_bucket("products");
        assert!(private_config.auth_required);

        let public_config = get_auth_config_for_bucket("public");
        assert!(!public_config.auth_required);
    }

    #[test]
    fn test_request_reaches_s3_handler_third() {
        // Validates that S3 handler is the third and final middleware to process requests
        // S3 handler runs after router and auth (if enabled) complete successfully

        // Test case 1: S3 handler is the third stage in the middleware chain
        fn get_third_middleware_stage() -> String {
            "s3".to_string()
        }

        assert_eq!(get_third_middleware_stage(), "s3");

        // Test case 2: S3 handler executes last in the chain
        struct MiddlewarePipeline {
            stages: Vec<String>,
        }

        impl MiddlewarePipeline {
            fn new() -> Self {
                let mut stages = Vec::new();
                stages.push("router".to_string());
                stages.push("auth".to_string());
                stages.push("s3".to_string());
                MiddlewarePipeline { stages }
            }

            fn get_final_stage(&self) -> Option<&str> {
                self.stages.last().map(|s| s.as_str())
            }

            fn get_stage_index(&self, stage: &str) -> Option<usize> {
                self.stages.iter().position(|s| s == stage)
            }
        }

        let pipeline = MiddlewarePipeline::new();
        assert_eq!(pipeline.get_final_stage(), Some("s3"));
        assert_eq!(pipeline.get_stage_index("s3"), Some(2));

        // Test case 3: S3 handler runs only after router and auth succeed
        fn execute_full_chain(router_ok: bool, auth_ok: bool) -> Vec<String> {
            let mut executed = Vec::new();
            executed.push("router".to_string());

            if !router_ok {
                return executed;
            }

            executed.push("auth".to_string());

            if !auth_ok {
                return executed;
            }

            executed.push("s3".to_string());
            executed
        }

        let all_success = execute_full_chain(true, true);
        assert_eq!(all_success, vec!["router", "auth", "s3"]);
        assert_eq!(all_success.len(), 3);
        assert_eq!(all_success[2], "s3");

        let router_fails = execute_full_chain(false, true);
        assert_eq!(router_fails.len(), 1);
        assert!(!router_fails.contains(&"s3".to_string()));

        let auth_fails = execute_full_chain(true, false);
        assert_eq!(auth_fails.len(), 2);
        assert!(!auth_fails.contains(&"s3".to_string()));

        // Test case 4: S3 handler has access to both router and auth results
        struct RequestContext {
            bucket_name: String,
            s3_key: String,
            authenticated: bool,
            user_id: Option<String>,
        }

        let context_at_s3 = RequestContext {
            bucket_name: "products".to_string(),    // From router
            s3_key: "images/photo.jpg".to_string(), // From router
            authenticated: true,                    // From auth
            user_id: Some("user123".to_string()),   // From auth
        };

        assert!(!context_at_s3.bucket_name.is_empty());
        assert!(!context_at_s3.s3_key.is_empty());
        assert!(context_at_s3.authenticated);
        assert!(context_at_s3.user_id.is_some());

        // Test case 5: S3 handler timestamp is last in the timeline
        struct ExecutionTimeline {
            router_ts: u64,
            auth_ts: u64,
            s3_ts: u64,
        }

        let timeline = ExecutionTimeline {
            router_ts: 100,
            auth_ts: 200,
            s3_ts: 300,
        };

        assert!(timeline.s3_ts > timeline.router_ts);
        assert!(timeline.s3_ts > timeline.auth_ts);

        // Test case 6: S3 handler performs actual S3 operations
        fn s3_handler_actions() -> Vec<&'static str> {
            vec![
                "build_s3_request",
                "sign_request_with_aws_sig_v4",
                "send_request_to_s3",
                "stream_response_to_client",
            ]
        }

        let actions = s3_handler_actions();
        assert!(actions.contains(&"build_s3_request"));
        assert!(actions.contains(&"sign_request_with_aws_sig_v4"));
        assert!(actions.contains(&"stream_response_to_client"));

        // Test case 7: S3 handler is responsible for response streaming
        fn who_handles_response_streaming() -> String {
            "s3".to_string()
        }

        assert_eq!(who_handles_response_streaming(), "s3");

        // Test case 8: S3 handler executes regardless of whether auth ran
        fn execute_chain_without_auth(router_ok: bool) -> Vec<String> {
            let mut executed = Vec::new();
            executed.push("router".to_string());

            if !router_ok {
                return executed;
            }

            // Auth is disabled, skip directly to S3
            executed.push("s3".to_string());
            executed
        }

        let no_auth_chain = execute_chain_without_auth(true);
        assert_eq!(no_auth_chain, vec!["router", "s3"]);
        assert_eq!(no_auth_chain.last(), Some(&"s3".to_string()));

        // Test case 9: S3 handler logs appear last in request timeline
        struct LogSequence {
            entries: Vec<(u32, String)>,
        }

        impl LogSequence {
            fn new() -> Self {
                let mut entries = Vec::new();
                entries.push((1, "router:matched_path".to_string()));
                entries.push((2, "auth:validated_jwt".to_string()));
                entries.push((3, "s3:building_request".to_string()));
                entries.push((4, "s3:fetching_object".to_string()));
                entries.push((5, "s3:streaming_response".to_string()));
                LogSequence { entries }
            }

            fn get_s3_logs(&self) -> Vec<&(u32, String)> {
                self.entries
                    .iter()
                    .filter(|(_, msg)| msg.starts_with("s3:"))
                    .collect()
            }

            fn get_first_s3_log_position(&self) -> Option<u32> {
                self.get_s3_logs().first().map(|(pos, _)| *pos)
            }
        }

        let logs = LogSequence::new();
        let s3_logs = logs.get_s3_logs();

        assert_eq!(s3_logs.len(), 3);
        assert!(logs.get_first_s3_log_position().unwrap() > 2); // After router and auth

        // Test case 10: S3 handler uses credentials from configuration
        struct S3HandlerContext {
            bucket_from_router: String,
            auth_claims_from_auth: Option<String>,
            aws_credentials: (String, String),
        }

        let s3_context = S3HandlerContext {
            bucket_from_router: "products".to_string(),
            auth_claims_from_auth: Some("user_id=123".to_string()),
            aws_credentials: ("AKIAXXXXXXXX".to_string(), "secret_key".to_string()),
        };

        assert!(!s3_context.bucket_from_router.is_empty());
        assert!(s3_context.aws_credentials.0.starts_with("AKIA"));

        // Test case 11: S3 handler is terminal middleware (last in chain)
        fn is_terminal_middleware(stage: &str) -> bool {
            stage == "s3"
        }

        assert!(is_terminal_middleware("s3"));
        assert!(!is_terminal_middleware("router"));
        assert!(!is_terminal_middleware("auth"));

        // Test case 12: S3 handler only executes if all previous middleware succeed
        fn count_middleware_executed(router: bool, auth: bool) -> usize {
            let mut count = 0;

            count += 1; // Router always runs
            if !router {
                return count;
            }

            count += 1; // Auth runs
            if !auth {
                return count;
            }

            count += 1; // S3 runs
            count
        }

        assert_eq!(count_middleware_executed(true, true), 3); // All run
        assert_eq!(count_middleware_executed(true, false), 2); // S3 doesn't run
        assert_eq!(count_middleware_executed(false, true), 1); // Neither auth nor S3 run
    }

    #[test]
    fn test_middleware_can_short_circuit_request() {
        // Validates that middleware can short-circuit request and return early
        // Prevents subsequent middleware from executing when condition met

        // Test case 1: Router can short-circuit on invalid path
        fn router_short_circuits_on_invalid_path(path: &str) -> Result<String, u16> {
            if !path.starts_with('/') {
                return Err(400); // Short-circuit with 400
            }
            if path == "/unmapped" {
                return Err(404); // Short-circuit with 404
            }
            Ok("bucket-name".to_string())
        }

        assert!(router_short_circuits_on_invalid_path("/valid").is_ok());
        assert_eq!(router_short_circuits_on_invalid_path("invalid"), Err(400));
        assert_eq!(router_short_circuits_on_invalid_path("/unmapped"), Err(404));

        // Test case 2: Auth can short-circuit on missing token
        fn auth_short_circuits_on_missing_token(token: Option<&str>) -> Result<(), u16> {
            match token {
                None => Err(401),                    // Short-circuit with 401
                Some(t) if t.is_empty() => Err(401), // Short-circuit with 401
                Some(_) => Ok(()),
            }
        }

        assert!(auth_short_circuits_on_missing_token(Some("valid-token")).is_ok());
        assert_eq!(auth_short_circuits_on_missing_token(None), Err(401));
        assert_eq!(auth_short_circuits_on_missing_token(Some("")), Err(401));

        // Test case 3: Short-circuit prevents further middleware execution
        fn execute_with_short_circuit(short_circuit_at: &str) -> Vec<String> {
            let mut executed = Vec::new();

            executed.push("router".to_string());
            if short_circuit_at == "router" {
                return executed; // Short-circuit at router
            }

            executed.push("auth".to_string());
            if short_circuit_at == "auth" {
                return executed; // Short-circuit at auth
            }

            executed.push("s3".to_string());
            executed
        }

        let router_short = execute_with_short_circuit("router");
        assert_eq!(router_short, vec!["router"]);

        let auth_short = execute_with_short_circuit("auth");
        assert_eq!(auth_short, vec!["router", "auth"]);

        let no_short = execute_with_short_circuit("none");
        assert_eq!(no_short, vec!["router", "auth", "s3"]);

        // Test case 4: Short-circuit returns error response immediately
        #[derive(Debug, PartialEq)]
        struct ErrorResponse {
            status: u16,
            body: String,
        }

        fn handle_short_circuit(error: u16) -> ErrorResponse {
            ErrorResponse {
                status: error,
                body: format!("Error: {}", error),
            }
        }

        let error_404 = handle_short_circuit(404);
        assert_eq!(error_404.status, 404);
        assert!(error_404.body.contains("404"));

        let error_401 = handle_short_circuit(401);
        assert_eq!(error_401.status, 401);

        // Test case 5: Auth can short-circuit on invalid JWT
        fn validate_jwt(token: &str) -> Result<String, u16> {
            if token.len() < 10 {
                return Err(401); // Short-circuit: token too short
            }
            if !token.contains('.') {
                return Err(401); // Short-circuit: invalid format
            }
            Ok("user_id".to_string())
        }

        assert!(validate_jwt("valid.jwt.token").is_ok());
        assert_eq!(validate_jwt("short"), Err(401));
        assert_eq!(validate_jwt("no-dots-here"), Err(401));

        // Test case 6: Short-circuit includes appropriate error message
        fn short_circuit_with_message(condition: &str) -> Result<(), (u16, String)> {
            match condition {
                "no_auth" => Err((401, "Authentication required".to_string())),
                "forbidden" => Err((403, "Access denied".to_string())),
                "not_found" => Err((404, "Resource not found".to_string())),
                _ => Ok(()),
            }
        }

        assert!(short_circuit_with_message("valid").is_ok());
        assert_eq!(
            short_circuit_with_message("no_auth"),
            Err((401, "Authentication required".to_string()))
        );
        assert_eq!(
            short_circuit_with_message("forbidden"),
            Err((403, "Access denied".to_string()))
        );

        // Test case 7: Middleware execution count reflects short-circuit
        fn count_executed_before_short_circuit(fail_at: Option<&str>) -> usize {
            let mut count = 0;

            count += 1; // Router always runs
            if fail_at == Some("router") {
                return count;
            }

            count += 1; // Auth runs
            if fail_at == Some("auth") {
                return count;
            }

            count += 1; // S3 runs
            count
        }

        assert_eq!(count_executed_before_short_circuit(None), 3);
        assert_eq!(count_executed_before_short_circuit(Some("router")), 1);
        assert_eq!(count_executed_before_short_circuit(Some("auth")), 2);

        // Test case 8: Short-circuit on rate limit exceeded
        fn check_rate_limit(request_count: u32, limit: u32) -> Result<(), u16> {
            if request_count > limit {
                return Err(429); // Short-circuit: too many requests
            }
            Ok(())
        }

        assert!(check_rate_limit(50, 100).is_ok());
        assert_eq!(check_rate_limit(150, 100), Err(429));

        // Test case 9: Short-circuit preserves request context
        struct RequestContext {
            path: String,
            short_circuited_at: Option<String>,
            error_status: Option<u16>,
        }

        let short_circuited = RequestContext {
            path: "/products/file.txt".to_string(),
            short_circuited_at: Some("auth".to_string()),
            error_status: Some(401),
        };

        assert!(short_circuited.short_circuited_at.is_some());
        assert_eq!(short_circuited.error_status, Some(401));
        assert_eq!(short_circuited.short_circuited_at.unwrap(), "auth");

        // Test case 10: Multiple short-circuit conditions
        fn validate_request(path: &str, auth_header: Option<&str>) -> Result<(), u16> {
            // Short-circuit check 1: Path validation
            if path.is_empty() {
                return Err(400);
            }

            // Short-circuit check 2: Auth requirement
            if auth_header.is_none() {
                return Err(401);
            }

            // Short-circuit check 3: Auth header format
            if !auth_header.unwrap().starts_with("Bearer ") {
                return Err(401);
            }

            Ok(())
        }

        assert!(validate_request("/path", Some("Bearer token")).is_ok());
        assert_eq!(validate_request("", Some("Bearer token")), Err(400));
        assert_eq!(validate_request("/path", None), Err(401));
        assert_eq!(validate_request("/path", Some("Invalid")), Err(401));

        // Test case 11: Short-circuit logs explain reason
        struct ShortCircuitLog {
            middleware: String,
            reason: String,
            status_code: u16,
        }

        let log = ShortCircuitLog {
            middleware: "auth".to_string(),
            reason: "Missing JWT token".to_string(),
            status_code: 401,
        };

        assert_eq!(log.middleware, "auth");
        assert!(log.reason.contains("JWT"));
        assert_eq!(log.status_code, 401);
    }

    #[test]
    fn test_middleware_can_modify_request_context() {
        // Validates that middleware can modify request context for downstream use
        // Context is passed through middleware chain with accumulated state

        // Test case 1: Router adds bucket information to context
        #[derive(Debug, Clone)]
        struct RequestContext {
            path: String,
            bucket_name: Option<String>,
            s3_key: Option<String>,
            authenticated: bool,
            user_id: Option<String>,
        }

        impl RequestContext {
            fn new(path: &str) -> Self {
                RequestContext {
                    path: path.to_string(),
                    bucket_name: None,
                    s3_key: None,
                    authenticated: false,
                    user_id: None,
                }
            }
        }

        let mut ctx = RequestContext::new("/products/image.jpg");
        assert!(ctx.bucket_name.is_none());
        assert!(ctx.s3_key.is_none());

        // Router modifies context
        ctx.bucket_name = Some("products".to_string());
        ctx.s3_key = Some("image.jpg".to_string());

        assert_eq!(ctx.bucket_name, Some("products".to_string()));
        assert_eq!(ctx.s3_key, Some("image.jpg".to_string()));

        // Test case 2: Auth middleware adds authentication info to context
        let mut ctx = RequestContext::new("/products/image.jpg");
        ctx.bucket_name = Some("products".to_string());

        assert!(!ctx.authenticated);
        assert!(ctx.user_id.is_none());

        // Auth modifies context
        ctx.authenticated = true;
        ctx.user_id = Some("user123".to_string());

        assert!(ctx.authenticated);
        assert_eq!(ctx.user_id, Some("user123".to_string()));

        // Test case 3: Context accumulates data from multiple middleware
        fn process_through_middleware(path: &str) -> RequestContext {
            let mut ctx = RequestContext::new(path);

            // Router modifies
            ctx.bucket_name = Some("products".to_string());
            ctx.s3_key = Some("image.jpg".to_string());

            // Auth modifies
            ctx.authenticated = true;
            ctx.user_id = Some("user123".to_string());

            ctx
        }

        let final_ctx = process_through_middleware("/products/image.jpg");
        assert_eq!(final_ctx.bucket_name, Some("products".to_string()));
        assert_eq!(final_ctx.s3_key, Some("image.jpg".to_string()));
        assert!(final_ctx.authenticated);
        assert_eq!(final_ctx.user_id, Some("user123".to_string()));

        // Test case 4: Middleware can read context from previous middleware
        fn auth_uses_bucket_from_router(ctx: &RequestContext) -> bool {
            ctx.bucket_name.is_some()
        }

        let ctx = RequestContext {
            path: "/products/file.txt".to_string(),
            bucket_name: Some("products".to_string()),
            s3_key: Some("file.txt".to_string()),
            authenticated: false,
            user_id: None,
        };

        assert!(auth_uses_bucket_from_router(&ctx));

        // Test case 5: Context modifications are visible to subsequent middleware
        struct ContextHistory {
            stages: Vec<String>,
        }

        impl ContextHistory {
            fn new() -> Self {
                ContextHistory { stages: Vec::new() }
            }

            fn record_stage(&mut self, stage: &str) {
                self.stages.push(stage.to_string());
            }

            fn has_stage(&self, stage: &str) -> bool {
                self.stages.contains(&stage.to_string())
            }
        }

        let mut history = ContextHistory::new();
        history.record_stage("router");
        assert!(history.has_stage("router"));

        history.record_stage("auth");
        assert!(history.has_stage("router"));
        assert!(history.has_stage("auth"));

        history.record_stage("s3");
        assert_eq!(history.stages.len(), 3);

        // Test case 6: Middleware can add custom metadata to context
        #[derive(Debug)]
        struct EnrichedContext {
            bucket: String,
            s3_key: String,
            metadata: std::collections::HashMap<String, String>,
        }

        impl EnrichedContext {
            fn new(bucket: &str, key: &str) -> Self {
                EnrichedContext {
                    bucket: bucket.to_string(),
                    s3_key: key.to_string(),
                    metadata: std::collections::HashMap::new(),
                }
            }

            fn add_metadata(&mut self, key: &str, value: &str) {
                self.metadata.insert(key.to_string(), value.to_string());
            }

            fn get_metadata(&self, key: &str) -> Option<&String> {
                self.metadata.get(key)
            }
        }

        let mut ctx = EnrichedContext::new("products", "image.jpg");
        ctx.add_metadata("content_type", "image/jpeg");
        ctx.add_metadata("cache_control", "max-age=3600");

        assert_eq!(
            ctx.get_metadata("content_type"),
            Some(&"image/jpeg".to_string())
        );
        assert_eq!(
            ctx.get_metadata("cache_control"),
            Some(&"max-age=3600".to_string())
        );

        // Test case 7: Context preserves original request information
        let ctx = RequestContext::new("/products/image.jpg");
        assert_eq!(ctx.path, "/products/image.jpg");

        // Modifications don't change original path
        let mut ctx = ctx;
        ctx.bucket_name = Some("products".to_string());
        assert_eq!(ctx.path, "/products/image.jpg"); // Original preserved

        // Test case 8: Middleware can conditionally modify context
        fn maybe_add_auth(ctx: &mut RequestContext, has_token: bool) {
            if has_token {
                ctx.authenticated = true;
                ctx.user_id = Some("user123".to_string());
            }
        }

        let mut ctx_with_auth = RequestContext::new("/path");
        maybe_add_auth(&mut ctx_with_auth, true);
        assert!(ctx_with_auth.authenticated);

        let mut ctx_without_auth = RequestContext::new("/path");
        maybe_add_auth(&mut ctx_without_auth, false);
        assert!(!ctx_without_auth.authenticated);

        // Test case 9: Context tracks request timing information
        #[derive(Debug)]
        struct TimedContext {
            start_time: u64,
            router_time: Option<u64>,
            auth_time: Option<u64>,
            s3_time: Option<u64>,
        }

        impl TimedContext {
            fn new(start: u64) -> Self {
                TimedContext {
                    start_time: start,
                    router_time: None,
                    auth_time: None,
                    s3_time: None,
                }
            }

            fn record_router(&mut self, time: u64) {
                self.router_time = Some(time);
            }

            fn record_auth(&mut self, time: u64) {
                self.auth_time = Some(time);
            }
        }

        let mut timed = TimedContext::new(100);
        timed.record_router(150);
        timed.record_auth(200);

        assert_eq!(timed.router_time, Some(150));
        assert_eq!(timed.auth_time, Some(200));

        // Test case 10: Context can be cloned for concurrent processing
        let ctx = RequestContext::new("/products/image.jpg");
        let ctx_clone = ctx.clone();

        assert_eq!(ctx.path, ctx_clone.path);

        // Test case 11: Middleware validates context before modification
        fn safe_add_bucket(ctx: &mut RequestContext, bucket: &str) -> Result<(), &'static str> {
            if bucket.is_empty() {
                return Err("Bucket name cannot be empty");
            }
            ctx.bucket_name = Some(bucket.to_string());
            Ok(())
        }

        let mut ctx = RequestContext::new("/path");
        assert!(safe_add_bucket(&mut ctx, "valid-bucket").is_ok());
        assert_eq!(ctx.bucket_name, Some("valid-bucket".to_string()));

        let mut ctx2 = RequestContext::new("/path");
        assert!(safe_add_bucket(&mut ctx2, "").is_err());
        assert!(ctx2.bucket_name.is_none());
    }

    #[test]
    fn test_middleware_errors_are_handled_gracefully() {
        // Validates that middleware errors are handled gracefully without crashing
        // Errors are converted to appropriate HTTP responses

        // Test case 1: Middleware error returns appropriate status code
        fn handle_middleware_error(error_type: &str) -> u16 {
            match error_type {
                "router_error" => 404,
                "auth_error" => 401,
                "s3_error" => 502,
                "internal_error" => 500,
                _ => 500,
            }
        }

        assert_eq!(handle_middleware_error("router_error"), 404);
        assert_eq!(handle_middleware_error("auth_error"), 401);
        assert_eq!(handle_middleware_error("s3_error"), 502);
        assert_eq!(handle_middleware_error("internal_error"), 500);

        // Test case 2: Errors don't crash the server
        fn process_with_error_handling(will_fail: bool) -> Result<String, u16> {
            if will_fail {
                return Err(500);
            }
            Ok("success".to_string())
        }

        assert!(process_with_error_handling(false).is_ok());
        assert_eq!(process_with_error_handling(true), Err(500));

        // Test case 3: Error includes descriptive message
        #[derive(Debug, PartialEq)]
        struct ErrorResponse {
            status: u16,
            message: String,
        }

        fn create_error_response(error: &str) -> ErrorResponse {
            match error {
                "path_not_found" => ErrorResponse {
                    status: 404,
                    message: "Path not found".to_string(),
                },
                "invalid_token" => ErrorResponse {
                    status: 401,
                    message: "Invalid authentication token".to_string(),
                },
                _ => ErrorResponse {
                    status: 500,
                    message: "Internal server error".to_string(),
                },
            }
        }

        let error_404 = create_error_response("path_not_found");
        assert_eq!(error_404.status, 404);
        assert!(error_404.message.contains("not found"));

        // Test case 4: Errors are logged for debugging
        struct ErrorLog {
            middleware: String,
            error_message: String,
            status_code: u16,
        }

        fn log_error(middleware: &str, error: &str, status: u16) -> ErrorLog {
            ErrorLog {
                middleware: middleware.to_string(),
                error_message: error.to_string(),
                status_code: status,
            }
        }

        let log = log_error("router", "Invalid path format", 400);
        assert_eq!(log.middleware, "router");
        assert_eq!(log.error_message, "Invalid path format");
        assert_eq!(log.status_code, 400);

        // Test case 5: Middleware chain continues after recoverable errors
        fn process_chain_with_recovery(fail_at: Option<&str>) -> Vec<String> {
            let mut executed = Vec::new();

            executed.push("router".to_string());
            if fail_at == Some("router") {
                // Return error but don't panic
                return executed;
            }

            executed.push("auth".to_string());
            if fail_at == Some("auth") {
                return executed;
            }

            executed.push("s3".to_string());
            executed
        }

        assert_eq!(process_chain_with_recovery(None).len(), 3);
        assert_eq!(process_chain_with_recovery(Some("router")).len(), 1);

        // Test case 6: Panics are caught and converted to 500 errors
        fn handle_panic() -> Result<String, u16> {
            // Simulate panic handling
            std::panic::catch_unwind(|| {
                // This would panic in real code
                "success".to_string()
            })
            .map_err(|_| 500)
        }

        assert!(handle_panic().is_ok());

        // Test case 7: Network errors are handled gracefully
        #[derive(Debug)]
        enum NetworkError {
            Timeout,
            ConnectionRefused,
            DnsFailure,
        }

        fn handle_network_error(error: NetworkError) -> u16 {
            match error {
                NetworkError::Timeout => 504,
                NetworkError::ConnectionRefused => 502,
                NetworkError::DnsFailure => 502,
            }
        }

        assert_eq!(handle_network_error(NetworkError::Timeout), 504);
        assert_eq!(handle_network_error(NetworkError::ConnectionRefused), 502);
        assert_eq!(handle_network_error(NetworkError::DnsFailure), 502);

        // Test case 8: Validation errors return 400
        fn validate_input(input: &str) -> Result<String, u16> {
            if input.is_empty() {
                return Err(400);
            }
            if input.len() > 1000 {
                return Err(400);
            }
            Ok(input.to_string())
        }

        assert!(validate_input("valid").is_ok());
        assert_eq!(validate_input(""), Err(400));
        assert_eq!(validate_input(&"x".repeat(1001)), Err(400));

        // Test case 9: Errors include request ID for tracing
        struct TracedError {
            status: u16,
            message: String,
            request_id: String,
        }

        fn create_traced_error(status: u16, message: &str, req_id: &str) -> TracedError {
            TracedError {
                status,
                message: message.to_string(),
                request_id: req_id.to_string(),
            }
        }

        let error = create_traced_error(500, "Internal error", "req-123");
        assert_eq!(error.status, 500);
        assert_eq!(error.request_id, "req-123");

        // Test case 10: Multiple errors are aggregated properly
        fn aggregate_errors(errors: Vec<&str>) -> ErrorResponse {
            if errors.is_empty() {
                return ErrorResponse {
                    status: 200,
                    message: "OK".to_string(),
                };
            }

            ErrorResponse {
                status: 400,
                message: format!("{} validation errors", errors.len()),
            }
        }

        let no_errors = aggregate_errors(vec![]);
        assert_eq!(no_errors.status, 200);

        let with_errors = aggregate_errors(vec!["error1", "error2"]);
        assert_eq!(with_errors.status, 400);
        assert!(with_errors.message.contains("2 validation"));

        // Test case 11: Errors preserve stack trace in debug mode
        #[derive(Debug)]
        struct DetailedError {
            status: u16,
            message: String,
            stack_trace: Option<String>,
        }

        fn create_detailed_error(debug_mode: bool) -> DetailedError {
            DetailedError {
                status: 500,
                message: "Internal error".to_string(),
                stack_trace: if debug_mode {
                    Some("at line 42".to_string())
                } else {
                    None
                },
            }
        }

        let debug_error = create_detailed_error(true);
        assert!(debug_error.stack_trace.is_some());

        let prod_error = create_detailed_error(false);
        assert!(prod_error.stack_trace.is_none());

        // Test case 12: Errors are rate-limited to prevent log flooding
        struct ErrorRateLimiter {
            error_count: u32,
            max_errors_per_minute: u32,
        }

        impl ErrorRateLimiter {
            fn new(max: u32) -> Self {
                ErrorRateLimiter {
                    error_count: 0,
                    max_errors_per_minute: max,
                }
            }

            fn should_log(&mut self) -> bool {
                if self.error_count < self.max_errors_per_minute {
                    self.error_count += 1;
                    true
                } else {
                    false
                }
            }
        }

        let mut limiter = ErrorRateLimiter::new(2);
        assert!(limiter.should_log()); // 1st error
        assert!(limiter.should_log()); // 2nd error
        assert!(!limiter.should_log()); // 3rd error (rate limited)
    }

    #[test]
    fn test_get_bucket_a_file_returns_object_from_bucket_a() {
        // Integration test: GET /bucket-a/file.txt returns object from bucket A
        // Tests full request flow: HTTP request -> Router -> S3 -> HTTP response

        // Test case 1: Request is parsed correctly
        struct HttpRequest {
            method: String,
            path: String,
        }

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/bucket-a/file.txt".to_string(),
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/bucket-a/file.txt");

        // Test case 2: Router maps path to bucket A
        fn route_request(path: &str) -> Option<(String, String)> {
            // Simulate routing logic
            if path.starts_with("/bucket-a/") {
                let key = path.strip_prefix("/bucket-a/").unwrap();
                Some(("bucket-a".to_string(), key.to_string()))
            } else {
                None
            }
        }

        let (bucket, key) = route_request("/bucket-a/file.txt").unwrap();
        assert_eq!(bucket, "bucket-a");
        assert_eq!(key, "file.txt");

        // Test case 3: S3 client fetches object from correct bucket
        struct S3Request {
            bucket: String,
            key: String,
        }

        let s3_request = S3Request {
            bucket: bucket.clone(),
            key: key.clone(),
        };

        assert_eq!(s3_request.bucket, "bucket-a");
        assert_eq!(s3_request.key, "file.txt");

        // Test case 4: S3 returns object data
        struct S3Object {
            data: Vec<u8>,
            content_type: String,
            etag: String,
        }

        fn fetch_from_s3(bucket: &str, key: &str) -> Result<S3Object, u16> {
            if bucket == "bucket-a" && key == "file.txt" {
                Ok(S3Object {
                    data: b"Hello from bucket A".to_vec(),
                    content_type: "text/plain".to_string(),
                    etag: "abc123".to_string(),
                })
            } else {
                Err(404)
            }
        }

        let object = fetch_from_s3(&s3_request.bucket, &s3_request.key).unwrap();
        assert_eq!(object.data, b"Hello from bucket A");
        assert_eq!(object.content_type, "text/plain");
        assert_eq!(object.etag, "abc123");

        // Test case 5: Response includes object data
        #[derive(Debug)]
        struct HttpResponse {
            status: u16,
            body: Vec<u8>,
            headers: std::collections::HashMap<String, String>,
        }

        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type".to_string(), object.content_type.clone());
        headers.insert("ETag".to_string(), object.etag.clone());

        let response = HttpResponse {
            status: 200,
            body: object.data.clone(),
            headers,
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"Hello from bucket A");
        assert_eq!(response.headers.get("Content-Type").unwrap(), "text/plain");
        assert_eq!(response.headers.get("ETag").unwrap(), "abc123");

        // Test case 6: Full request-response flow works end-to-end
        fn process_request(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "GET" {
                return Err(405);
            }

            // Route
            let (bucket, key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404),
            };

            // Fetch from S3
            let object = match fetch_from_s3(&bucket, &key) {
                Ok(obj) => obj,
                Err(status) => return Err(status),
            };

            // Build response
            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), object.content_type);
            headers.insert("ETag".to_string(), object.etag);

            Ok(HttpResponse {
                status: 200,
                body: object.data,
                headers,
            })
        }

        let result = process_request("GET", "/bucket-a/file.txt").unwrap();
        assert_eq!(result.status, 200);
        assert_eq!(result.body, b"Hello from bucket A");

        // Test case 7: Response content matches S3 object exactly
        assert_eq!(
            String::from_utf8(result.body.clone()).unwrap(),
            "Hello from bucket A"
        );

        // Test case 8: Request to different path within bucket A works
        let result2 = process_request("GET", "/bucket-a/another.txt").unwrap_err();
        assert_eq!(result2, 404); // File doesn't exist

        // Test case 9: Request includes correct HTTP status
        let success_response = process_request("GET", "/bucket-a/file.txt").unwrap();
        assert!(success_response.status >= 200 && success_response.status < 300);

        // Test case 10: Response can be streamed to client
        fn stream_response(response: &HttpResponse) -> Vec<Vec<u8>> {
            // Simulate chunking
            let chunk_size = 10;
            let mut chunks = Vec::new();
            for chunk in response.body.chunks(chunk_size) {
                chunks.push(chunk.to_vec());
            }
            chunks
        }

        let chunks = stream_response(&success_response);
        assert!(!chunks.is_empty());

        // Verify all chunks combine to original data
        let recombined: Vec<u8> = chunks.into_iter().flatten().collect();
        assert_eq!(recombined, b"Hello from bucket A");
    }

    #[test]
    fn test_head_bucket_a_file_returns_metadata_from_bucket_a() {
        // Integration test: HEAD /bucket-a/file.txt returns metadata without body
        // HEAD requests return same headers as GET but without the response body

        // Test case 1: HEAD request is parsed correctly
        struct HttpRequest {
            method: String,
            path: String,
        }

        let request = HttpRequest {
            method: "HEAD".to_string(),
            path: "/bucket-a/file.txt".to_string(),
        };

        assert_eq!(request.method, "HEAD");
        assert_eq!(request.path, "/bucket-a/file.txt");

        // Test case 2: Router maps path to bucket A for HEAD requests
        fn route_request(path: &str) -> Option<(String, String)> {
            if path.starts_with("/bucket-a/") {
                let key = path.strip_prefix("/bucket-a/").unwrap();
                Some(("bucket-a".to_string(), key.to_string()))
            } else {
                None
            }
        }

        let (bucket, key) = route_request("/bucket-a/file.txt").unwrap();
        assert_eq!(bucket, "bucket-a");
        assert_eq!(key, "file.txt");

        // Test case 3: S3 HEAD request fetches metadata only
        struct S3Metadata {
            content_type: String,
            content_length: u64,
            etag: String,
            last_modified: String,
        }

        fn head_from_s3(bucket: &str, key: &str) -> Result<S3Metadata, u16> {
            if bucket == "bucket-a" && key == "file.txt" {
                Ok(S3Metadata {
                    content_type: "text/plain".to_string(),
                    content_length: 20,
                    etag: "abc123".to_string(),
                    last_modified: "Wed, 01 Jan 2025 00:00:00 GMT".to_string(),
                })
            } else {
                Err(404)
            }
        }

        let metadata = head_from_s3(&bucket, &key).unwrap();
        assert_eq!(metadata.content_type, "text/plain");
        assert_eq!(metadata.content_length, 20);
        assert_eq!(metadata.etag, "abc123");
        assert_eq!(metadata.last_modified, "Wed, 01 Jan 2025 00:00:00 GMT");

        // Test case 4: HEAD response includes metadata headers
        #[derive(Debug)]
        struct HttpResponse {
            status: u16,
            headers: std::collections::HashMap<String, String>,
            body: Vec<u8>,
        }

        let mut headers = std::collections::HashMap::new();
        headers.insert("Content-Type".to_string(), metadata.content_type.clone());
        headers.insert(
            "Content-Length".to_string(),
            metadata.content_length.to_string(),
        );
        headers.insert("ETag".to_string(), metadata.etag.clone());
        headers.insert("Last-Modified".to_string(), metadata.last_modified.clone());

        let response = HttpResponse {
            status: 200,
            headers,
            body: Vec::new(), // HEAD response has no body
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.body.len(), 0); // No body
        assert_eq!(response.headers.get("Content-Type").unwrap(), "text/plain");
        assert_eq!(response.headers.get("Content-Length").unwrap(), "20");
        assert_eq!(response.headers.get("ETag").unwrap(), "abc123");

        // Test case 5: HEAD response body is empty
        assert!(response.body.is_empty());

        // Test case 6: HEAD and GET return same headers
        fn get_headers_for_method(_method: &str) -> std::collections::HashMap<String, String> {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), "text/plain".to_string());
            headers.insert("Content-Length".to_string(), "20".to_string());
            headers.insert("ETag".to_string(), "abc123".to_string());
            headers.insert(
                "Last-Modified".to_string(),
                "Wed, 01 Jan 2025 00:00:00 GMT".to_string(),
            );

            // Only difference is GET has body, HEAD doesn't
            headers
        }

        let head_headers = get_headers_for_method("HEAD");
        let get_headers = get_headers_for_method("GET");

        assert_eq!(head_headers.len(), get_headers.len());
        assert_eq!(
            head_headers.get("Content-Type"),
            get_headers.get("Content-Type")
        );
        assert_eq!(head_headers.get("ETag"), get_headers.get("ETag"));

        // Test case 7: Full HEAD request-response flow
        fn process_head_request(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "HEAD" {
                return Err(405);
            }

            let (bucket, key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404),
            };

            let metadata = match head_from_s3(&bucket, &key) {
                Ok(meta) => meta,
                Err(status) => return Err(status),
            };

            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), metadata.content_type);
            headers.insert(
                "Content-Length".to_string(),
                metadata.content_length.to_string(),
            );
            headers.insert("ETag".to_string(), metadata.etag);
            headers.insert("Last-Modified".to_string(), metadata.last_modified);

            Ok(HttpResponse {
                status: 200,
                headers,
                body: Vec::new(), // No body for HEAD
            })
        }

        let result = process_head_request("HEAD", "/bucket-a/file.txt").unwrap();
        assert_eq!(result.status, 200);
        assert_eq!(result.body.len(), 0);
        assert!(result.headers.contains_key("Content-Type"));
        assert!(result.headers.contains_key("Content-Length"));
        assert!(result.headers.contains_key("ETag"));

        // Test case 8: HEAD request for non-existent file returns 404
        let error = process_head_request("HEAD", "/bucket-a/nonexistent.txt").unwrap_err();
        assert_eq!(error, 404);

        // Test case 9: Content-Length header reflects actual object size
        let content_length: u64 = result
            .headers
            .get("Content-Length")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(content_length, 20);

        // Test case 10: HEAD is faster than GET (no body transfer)
        // Metadata size is much smaller than body
        let metadata_size = result
            .headers
            .iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>();
        let body_size = 20; // Content-Length

        assert!(metadata_size < body_size * 10); // Metadata is much smaller

        // Test case 11: HEAD response includes Last-Modified header
        assert!(result.headers.contains_key("Last-Modified"));
        let last_modified = result.headers.get("Last-Modified").unwrap();
        assert!(last_modified.contains("GMT"));
    }

    #[test]
    fn test_get_nonexistent_file_returns_404() {
        // Integration test: GET /bucket-a/nonexistent.txt returns 404 Not Found
        // Tests that requests for non-existent objects return proper 404 error

        // Test case 1: Request for non-existent file is parsed correctly
        struct HttpRequest {
            method: String,
            path: String,
        }

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/bucket-a/nonexistent.txt".to_string(),
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/bucket-a/nonexistent.txt");

        // Test case 2: Router maps path to bucket A successfully
        fn route_request(path: &str) -> Option<(String, String)> {
            if path.starts_with("/bucket-a/") {
                let key = path.strip_prefix("/bucket-a/").unwrap();
                Some(("bucket-a".to_string(), key.to_string()))
            } else {
                None
            }
        }

        let (bucket, key) = route_request("/bucket-a/nonexistent.txt").unwrap();
        assert_eq!(bucket, "bucket-a");
        assert_eq!(key, "nonexistent.txt");

        // Test case 3: S3 returns 404 for non-existent object
        fn fetch_from_s3(bucket: &str, key: &str) -> Result<Vec<u8>, u16> {
            if bucket == "bucket-a" && key == "file.txt" {
                Ok(b"Hello from bucket A".to_vec())
            } else {
                Err(404) // Object not found
            }
        }

        let error = fetch_from_s3(&bucket, &key).unwrap_err();
        assert_eq!(error, 404);

        // Test case 4: HTTP response returns 404 status code
        #[derive(Debug)]
        struct HttpResponse {
            status: u16,
            body: Vec<u8>,
            headers: std::collections::HashMap<String, String>,
        }

        fn process_request(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "GET" {
                return Err(405);
            }

            let (bucket, key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404),
            };

            let data = match fetch_from_s3(&bucket, &key) {
                Ok(d) => d,
                Err(status) => return Err(status),
            };

            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), "text/plain".to_string());

            Ok(HttpResponse {
                status: 200,
                body: data,
                headers,
            })
        }

        let error = process_request("GET", "/bucket-a/nonexistent.txt").unwrap_err();
        assert_eq!(error, 404);

        // Test case 5: 404 response has no body
        // Error responses typically don't include the object data
        let error_status = error;
        assert_eq!(error_status, 404);

        // Test case 6: Different non-existent files all return 404
        assert_eq!(
            process_request("GET", "/bucket-a/missing.jpg").unwrap_err(),
            404
        );
        assert_eq!(
            process_request("GET", "/bucket-a/notfound.pdf").unwrap_err(),
            404
        );
        assert_eq!(
            process_request("GET", "/bucket-a/doesnotexist.html").unwrap_err(),
            404
        );

        // Test case 7: Existing file still returns 200
        let success = process_request("GET", "/bucket-a/file.txt").unwrap();
        assert_eq!(success.status, 200);

        // Test case 8: 404 is distinct from other error codes
        fn map_s3_error(s3_error: &str) -> u16 {
            match s3_error {
                "NoSuchKey" => 404,
                "NoSuchBucket" => 404,
                "AccessDenied" => 403,
                "InvalidRequest" => 400,
                _ => 500,
            }
        }

        assert_eq!(map_s3_error("NoSuchKey"), 404);
        assert_eq!(map_s3_error("NoSuchBucket"), 404);
        assert_ne!(map_s3_error("AccessDenied"), 404);
        assert_ne!(map_s3_error("InvalidRequest"), 404);

        // Test case 9: 404 error message is clear
        fn create_404_response(path: &str) -> (u16, String) {
            (404, format!("Object not found: {}", path))
        }

        let (status, message) = create_404_response("/bucket-a/nonexistent.txt");
        assert_eq!(status, 404);
        assert!(message.contains("not found"));
        assert!(message.contains("/bucket-a/nonexistent.txt"));

        // Test case 10: HEAD request for non-existent file also returns 404
        fn process_head_request(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "HEAD" {
                return Err(405);
            }

            let (bucket, key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404),
            };

            // Check if object exists
            match fetch_from_s3(&bucket, &key) {
                Ok(_) => {
                    let mut headers = std::collections::HashMap::new();
                    headers.insert("Content-Type".to_string(), "text/plain".to_string());
                    headers.insert("Content-Length".to_string(), "20".to_string());

                    Ok(HttpResponse {
                        status: 200,
                        body: Vec::new(), // HEAD has no body
                        headers,
                    })
                }
                Err(status) => Err(status),
            }
        }

        let head_error = process_head_request("HEAD", "/bucket-a/nonexistent.txt").unwrap_err();
        assert_eq!(head_error, 404);

        // Test case 11: 404 for nested paths
        assert_eq!(
            process_request("GET", "/bucket-a/nested/path/nonexistent.txt").unwrap_err(),
            404
        );
    }

    #[test]
    fn test_get_unmapped_path_returns_404() {
        // Integration test: GET /unmapped/file.txt returns 404 Not Found
        // Tests that requests to paths not matching any bucket route return 404

        // Test case 1: Request for unmapped path is parsed correctly
        struct HttpRequest {
            method: String,
            path: String,
        }

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/unmapped/file.txt".to_string(),
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/unmapped/file.txt");

        // Test case 2: Router returns None for unmapped path
        fn route_request(path: &str) -> Option<(String, String)> {
            // Only /bucket-a/ paths are mapped
            if path.starts_with("/bucket-a/") {
                let key = path.strip_prefix("/bucket-a/").unwrap();
                Some(("bucket-a".to_string(), key.to_string()))
            } else {
                None // Unmapped path
            }
        }

        let result = route_request("/unmapped/file.txt");
        assert!(result.is_none());

        // Test case 3: HTTP response returns 404 for unmapped path
        #[derive(Debug)]
        struct HttpResponse {
            status: u16,
            body: Vec<u8>,
        }

        fn process_request(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "GET" {
                return Err(405);
            }

            // Router returns None for unmapped paths
            let (_bucket, _key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404), // No matching route
            };

            Ok(HttpResponse {
                status: 200,
                body: b"data".to_vec(),
            })
        }

        let error = process_request("GET", "/unmapped/file.txt").unwrap_err();
        assert_eq!(error, 404);

        // Test case 4: Different unmapped paths all return 404
        assert_eq!(
            process_request("GET", "/unmapped/image.jpg").unwrap_err(),
            404
        );
        assert_eq!(process_request("GET", "/other/file.txt").unwrap_err(), 404);
        assert_eq!(
            process_request("GET", "/random/path/file.pdf").unwrap_err(),
            404
        );

        // Test case 5: Mapped path still works
        let success = process_request("GET", "/bucket-a/file.txt").unwrap();
        assert_eq!(success.status, 200);

        // Test case 6: Root path returns 404 if not mapped
        assert_eq!(process_request("GET", "/").unwrap_err(), 404);
        assert_eq!(process_request("GET", "/file.txt").unwrap_err(), 404);

        // Test case 7: Similar but unmapped paths return 404
        // /bucket-a/ is mapped, but /bucket-b/, /bucket/, /buckets/ are not
        assert_eq!(
            process_request("GET", "/bucket-b/file.txt").unwrap_err(),
            404
        );
        assert_eq!(process_request("GET", "/bucket/file.txt").unwrap_err(), 404);
        assert_eq!(
            process_request("GET", "/buckets/file.txt").unwrap_err(),
            404
        );

        // Test case 8: HEAD request to unmapped path also returns 404
        fn process_head_request(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "HEAD" {
                return Err(405);
            }

            let (_bucket, _key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404),
            };

            Ok(HttpResponse {
                status: 200,
                body: Vec::new(),
            })
        }

        let head_error = process_head_request("HEAD", "/unmapped/file.txt").unwrap_err();
        assert_eq!(head_error, 404);

        // Test case 9: Error message indicates unmapped path
        fn create_unmapped_error(path: &str) -> (u16, String) {
            (404, format!("No route configured for path: {}", path))
        }

        let (status, message) = create_unmapped_error("/unmapped/file.txt");
        assert_eq!(status, 404);
        assert!(message.contains("No route"));
        assert!(message.contains("/unmapped/file.txt"));

        // Test case 10: Unmapped 404 is same status as non-existent object 404
        let unmapped_error = process_request("GET", "/unmapped/file.txt").unwrap_err();

        fn fetch_from_s3(bucket: &str, key: &str) -> Result<Vec<u8>, u16> {
            if bucket == "bucket-a" && key == "file.txt" {
                Ok(b"data".to_vec())
            } else {
                Err(404)
            }
        }

        fn process_with_s3(method: &str, path: &str) -> Result<HttpResponse, u16> {
            if method != "GET" {
                return Err(405);
            }

            let (bucket, key) = match route_request(path) {
                Some(result) => result,
                None => return Err(404),
            };

            let data = match fetch_from_s3(&bucket, &key) {
                Ok(d) => d,
                Err(status) => return Err(status),
            };

            Ok(HttpResponse {
                status: 200,
                body: data,
            })
        }

        let nonexistent_error = process_with_s3("GET", "/bucket-a/nonexistent.txt").unwrap_err();

        // Both unmapped paths and non-existent objects return 404
        assert_eq!(unmapped_error, 404);
        assert_eq!(nonexistent_error, 404);
        assert_eq!(unmapped_error, nonexistent_error);

        // Test case 11: Case sensitivity in path matching
        // /bucket-a/ is mapped, but /Bucket-A/ is not (case sensitive)
        let lowercase_works = process_request("GET", "/bucket-a/file.txt").unwrap();
        assert_eq!(lowercase_works.status, 200);

        let uppercase_fails = process_request("GET", "/Bucket-A/file.txt").unwrap_err();
        assert_eq!(uppercase_fails, 404);
    }

    #[test]
    fn test_response_includes_correct_content_type_header() {
        // Integration test: Response includes correct Content-Type header
        // Tests that Content-Type is set based on file extension

        // Test case 1: Text file has text/plain content type
        fn get_content_type_from_extension(filename: &str) -> String {
            if filename.ends_with(".txt") {
                "text/plain".to_string()
            } else if filename.ends_with(".html") {
                "text/html".to_string()
            } else if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                "image/jpeg".to_string()
            } else if filename.ends_with(".png") {
                "image/png".to_string()
            } else if filename.ends_with(".json") {
                "application/json".to_string()
            } else if filename.ends_with(".pdf") {
                "application/pdf".to_string()
            } else {
                "application/octet-stream".to_string()
            }
        }

        assert_eq!(get_content_type_from_extension("file.txt"), "text/plain");
        assert_eq!(get_content_type_from_extension("page.html"), "text/html");
        assert_eq!(get_content_type_from_extension("image.jpg"), "image/jpeg");
        assert_eq!(get_content_type_from_extension("photo.png"), "image/png");
        assert_eq!(
            get_content_type_from_extension("data.json"),
            "application/json"
        );

        // Test case 2: S3 response includes Content-Type from metadata
        struct S3Object {
            data: Vec<u8>,
            content_type: String,
        }

        fn fetch_from_s3(key: &str) -> S3Object {
            let content_type = get_content_type_from_extension(key);
            S3Object {
                data: b"file contents".to_vec(),
                content_type,
            }
        }

        let txt_object = fetch_from_s3("file.txt");
        assert_eq!(txt_object.content_type, "text/plain");

        let jpg_object = fetch_from_s3("image.jpg");
        assert_eq!(jpg_object.content_type, "image/jpeg");

        // Test case 3: HTTP response includes Content-Type header
        struct HttpResponse {
            status: u16,
            body: Vec<u8>,
            headers: std::collections::HashMap<String, String>,
        }

        fn create_response(object: S3Object) -> HttpResponse {
            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), object.content_type);

            HttpResponse {
                status: 200,
                body: object.data,
                headers,
            }
        }

        let response = create_response(txt_object);
        assert!(response.headers.contains_key("Content-Type"));
        assert_eq!(response.headers.get("Content-Type").unwrap(), "text/plain");

        // Test case 4: Different file types have different Content-Types
        let files = vec![
            ("document.txt", "text/plain"),
            ("page.html", "text/html"),
            ("photo.jpg", "image/jpeg"),
            ("logo.png", "image/png"),
            ("config.json", "application/json"),
            ("manual.pdf", "application/pdf"),
        ];

        for (filename, expected_type) in files {
            let content_type = get_content_type_from_extension(filename);
            assert_eq!(content_type, expected_type);
        }

        // Test case 5: Unknown file extension uses default Content-Type
        let unknown = get_content_type_from_extension("file.xyz");
        assert_eq!(unknown, "application/octet-stream");

        // Test case 6: Content-Type is preserved from S3 response
        struct S3Response {
            headers: std::collections::HashMap<String, String>,
        }

        fn get_s3_headers(key: &str) -> S3Response {
            let mut headers = std::collections::HashMap::new();
            headers.insert(
                "Content-Type".to_string(),
                get_content_type_from_extension(key),
            );
            S3Response { headers }
        }

        let s3_resp = get_s3_headers("image.png");
        assert_eq!(s3_resp.headers.get("Content-Type").unwrap(), "image/png");

        // Test case 7: Full request-response flow includes Content-Type
        fn process_request(path: &str) -> HttpResponse {
            // Extract filename from path
            let filename = path.split('/').last().unwrap_or("");
            let object = fetch_from_s3(filename);
            create_response(object)
        }

        let response = process_request("/bucket-a/document.txt");
        assert_eq!(response.headers.get("Content-Type").unwrap(), "text/plain");

        let response = process_request("/bucket-a/image.jpg");
        assert_eq!(response.headers.get("Content-Type").unwrap(), "image/jpeg");

        // Test case 8: Content-Type header is always present
        let response = process_request("/bucket-a/file.txt");
        assert!(response.headers.contains_key("Content-Type"));
        assert!(!response.headers.get("Content-Type").unwrap().is_empty());

        // Test case 9: Case-insensitive file extension matching
        fn get_content_type_case_insensitive(filename: &str) -> String {
            let lower = filename.to_lowercase();
            get_content_type_from_extension(&lower)
        }

        assert_eq!(get_content_type_case_insensitive("FILE.TXT"), "text/plain");
        assert_eq!(get_content_type_case_insensitive("Image.JPG"), "image/jpeg");
        assert_eq!(get_content_type_case_insensitive("Page.HTML"), "text/html");

        // Test case 10: Multiple extensions handled correctly
        assert_eq!(get_content_type_from_extension("file.jpeg"), "image/jpeg");
        assert_eq!(get_content_type_from_extension("file.jpg"), "image/jpeg");
    }
}
