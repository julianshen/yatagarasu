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
