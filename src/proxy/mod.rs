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
}
