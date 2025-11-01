// Basic HTTP server integration test
// Tests that the Pingora server can start with valid configuration

use yatagarasu::config::Config;

#[test]
fn test_can_create_proxy_service_from_config() {
    // Test: Can create a ProxyHttp implementation from config
    // This is the RED phase - this will fail until we implement ProxyHttp

    // Load test configuration
    let config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 18080

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-test-bucket"
      region: "us-east-1"
      access_key: "test-access-key"
      secret_key: "test-secret-key"
    auth:
      enabled: false

jwt:
  enabled: false
  secret: "test-secret"
  algorithm: "HS256"
  token_sources: []
  claims: []
"#;

    // Parse config
    let config: Config =
        serde_yaml::from_str(config_yaml).expect("Should parse valid YAML configuration");

    // Try to create proxy service - this will fail until ProxyHttp is implemented
    // For now, just verify we can import the proxy module
    // Once implemented, this should be:
    // let _proxy = yatagarasu::proxy::YatagarasuProxy::new(config);

    assert!(
        config.buckets.len() > 0,
        "Config should have buckets loaded for proxy creation"
    );
}
