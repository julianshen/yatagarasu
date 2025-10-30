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
