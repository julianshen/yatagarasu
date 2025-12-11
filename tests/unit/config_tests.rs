// Configuration module unit tests
// Extracted from src/config/mod.rs for improved readability

use yatagarasu::config::*;
use yatagarasu::observability::ObservabilityConfig;

fn test_can_create_empty_config_struct() {
    let _config = Config {
        server: ServerConfig {
            address: String::from("127.0.0.1"),
            port: 8080,
            threads: 4,
            request_timeout: 30,
            max_concurrent_requests: 1000,
            rate_limit: None,
            security_limits: SecurityLimitsConfig::default(),
        },
        buckets: vec![],
        jwt: None,
        cache: None,
        audit_log: None,
        observability: ObservabilityConfig::default(),
        generation: 0,
    };
}

#[test]
fn test_can_deserialize_minimal_valid_yaml_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    // If we got here, deserialization succeeded
    let _ = config;
}

#[test]
fn test_can_access_server_address_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.server.address, "127.0.0.1");
}

#[test]
fn test_can_access_server_port_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.server.port, 8080);
}

#[test]
fn test_config_deserialization_fails_with_empty_file() {
    let yaml = "";
    let result: Result<Config, _> = serde_yaml::from_str(yaml);
    assert!(
        result.is_err(),
        "Expected deserialization to fail with empty file"
    );
}

#[test]
fn test_config_deserialization_fails_with_invalid_yaml() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: [invalid syntax here}
"#;
    let result: Result<Config, _> = serde_yaml::from_str(yaml);
    assert!(
        result.is_err(),
        "Expected deserialization to fail with invalid YAML"
    );
}

#[test]
fn test_can_parse_single_bucket_configuration() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets.len(), 1);
    assert_eq!(config.buckets[0].name, "products");
    assert_eq!(config.buckets[0].path_prefix, "/products");
    assert_eq!(config.buckets[0].s3.bucket, "my-products-bucket");
    assert_eq!(config.buckets[0].s3.region, "us-west-2");
}

#[test]
fn test_can_parse_multiple_bucket_configurations() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  - name: "images"
    path_prefix: "/images"
    s3:
      bucket: "my-images-bucket"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE2"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets.len(), 2);
    assert_eq!(config.buckets[0].name, "products");
    assert_eq!(config.buckets[1].name, "images");
    assert_eq!(config.buckets[0].path_prefix, "/products");
    assert_eq!(config.buckets[1].path_prefix, "/images");
}

#[test]
fn test_can_access_bucket_name_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets[0].name, "products");
}

#[test]
fn test_can_access_bucket_path_prefix_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets[0].path_prefix, "/products");
}

#[test]
fn test_can_access_s3_bucket_name_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets[0].s3.bucket, "my-products-bucket");
}

#[test]
fn test_can_access_s3_region_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets[0].s3.region, "us-west-2");
}

#[test]
fn test_can_access_s3_access_key_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(config.buckets[0].s3.access_key, "AKIAIOSFODNN7EXAMPLE");
}

#[test]
fn test_can_access_s3_secret_key_from_config() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    assert_eq!(
        config.buckets[0].s3.secret_key,
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
    );
}

#[test]
fn test_rejects_bucket_config_with_missing_required_fields() {
    // Missing 'name' field
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let result: Result<Config, _> = serde_yaml::from_str(yaml);
    assert!(
        result.is_err(),
        "Expected deserialization to fail with missing 'name' field"
    );
}

#[test]
fn test_rejects_bucket_config_with_empty_path_prefix() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: ""
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with empty path_prefix"
    );
}

#[test]
fn test_rejects_bucket_config_with_duplicate_path_prefix() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/api"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  - name: "images"
    path_prefix: "/api"
    s3:
      bucket: "my-images-bucket"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE2"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize YAML");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with duplicate path_prefix"
    );
}

#[test]
fn test_can_substitute_environment_variable_in_access_key() {
    std::env::set_var("TEST_ACCESS_KEY", "AKIAIOSFODNN7EXAMPLE");

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "${TEST_ACCESS_KEY}"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config =
        Config::from_yaml_with_env(yaml).expect("Failed to load config with env substitution");
    assert_eq!(config.buckets[0].s3.access_key, "AKIAIOSFODNN7EXAMPLE");

    std::env::remove_var("TEST_ACCESS_KEY");
}

#[test]
fn test_can_substitute_environment_variable_in_secret_key() {
    std::env::set_var(
        "TEST_SECRET_KEY",
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    );

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "${TEST_SECRET_KEY}"
"#;
    let config: Config =
        Config::from_yaml_with_env(yaml).expect("Failed to load config with env substitution");
    assert_eq!(
        config.buckets[0].s3.secret_key,
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
    );

    std::env::remove_var("TEST_SECRET_KEY");
}

#[test]
fn test_can_substitute_environment_variable_in_jwt_secret() {
    std::env::set_var("TEST_JWT_SECRET", "my-super-secret-jwt-key");

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "${TEST_JWT_SECRET}"
  algorithm: "HS256"
"#;
    let config: Config =
        Config::from_yaml_with_env(yaml).expect("Failed to load config with env substitution");
    assert_eq!(
        config.jwt.as_ref().unwrap().secret,
        "my-super-secret-jwt-key"
    );

    std::env::remove_var("TEST_JWT_SECRET");
}

#[test]
fn test_substitution_fails_gracefully_when_env_var_missing() {
    // Ensure the env var doesn't exist
    std::env::remove_var("MISSING_VAR");

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "${MISSING_VAR}"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let result = Config::from_yaml_with_env(yaml);
    assert!(
        result.is_err(),
        "Expected error when environment variable is missing"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("MISSING_VAR") || err_msg.contains("environment variable"),
        "Error message should mention the missing variable or environment variable"
    );
}

#[test]
fn test_can_use_literal_value_without_substitution() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
jwt:
  enabled: true
  secret: "my-jwt-secret-key"
  algorithm: "HS256"
"#;
    let config: Config =
        Config::from_yaml_with_env(yaml).expect("Failed to load config with literal values");

    // Verify all literal values are preserved
    assert_eq!(config.server.address, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.buckets[0].name, "products");
    assert_eq!(config.buckets[0].path_prefix, "/products");
    assert_eq!(config.buckets[0].s3.bucket, "my-products-bucket");
    assert_eq!(config.buckets[0].s3.region, "us-west-2");
    assert_eq!(config.buckets[0].s3.access_key, "AKIAIOSFODNN7EXAMPLE");
    assert_eq!(
        config.buckets[0].s3.secret_key,
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
    );
    assert_eq!(config.jwt.as_ref().unwrap().secret, "my-jwt-secret-key");
}

#[test]
fn test_can_parse_jwt_config_with_enabled_true() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with enabled=true");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.enabled, true);
    assert_eq!(jwt.secret, "my-jwt-secret");
}

#[test]
fn test_can_parse_jwt_config_with_enabled_false() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: false
  secret: "my-jwt-secret"
  algorithm: "HS256"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with enabled=false");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.enabled, false);
    assert_eq!(jwt.secret, "my-jwt-secret");
}

#[test]
fn test_can_parse_multiple_token_sources() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  token_sources:
    - type: "header"
    - type: "query"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize multiple token sources");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.token_sources.len(), 2);
}

#[test]
fn test_can_parse_header_token_source_with_prefix() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  token_sources:
    - type: "header"
      name: "Authorization"
      prefix: "Bearer "
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize header token source");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.token_sources.len(), 1);

    let token_source = &jwt.token_sources[0];
    assert_eq!(token_source.source_type, "header");
    assert_eq!(token_source.name.as_ref().unwrap(), "Authorization");
    assert_eq!(token_source.prefix.as_ref().unwrap(), "Bearer ");
}

#[test]
fn test_can_parse_query_parameter_token_source() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  token_sources:
    - type: "query"
      name: "token"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize query token source");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.token_sources.len(), 1);

    let token_source = &jwt.token_sources[0];
    assert_eq!(token_source.source_type, "query");
    assert_eq!(token_source.name.as_ref().unwrap(), "token");
}

#[test]
fn test_can_parse_custom_header_token_source() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  token_sources:
    - type: "header"
      name: "X-API-Token"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize custom header token source");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.token_sources.len(), 1);

    let token_source = &jwt.token_sources[0];
    assert_eq!(token_source.source_type, "header");
    assert_eq!(token_source.name.as_ref().unwrap(), "X-API-Token");
}

#[test]
fn test_can_parse_jwt_algorithm_hs256() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize JWT algorithm");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.algorithm, "HS256");
}

#[test]
fn test_can_parse_jwt_secret() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-super-secret-key-12345"
  algorithm: "HS256"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize JWT secret");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.secret, "my-super-secret-key-12345");
}

#[test]
fn test_rejects_jwt_config_with_invalid_algorithm() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "INVALID"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with invalid algorithm");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with invalid algorithm"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("INVALID") || err_msg.contains("algorithm"),
        "Error message should mention the invalid algorithm or algorithm field"
    );
}

#[test]
fn test_rejects_auth_config_missing_jwt_secret_when_enabled() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: ""
  algorithm: "HS256"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with empty secret");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with empty JWT secret when enabled"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("secret") || err_msg.contains("empty"),
        "Error message should mention secret or empty"
    );
}

#[test]
fn test_can_parse_single_claim_verification_rule() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "role"
      operator: "equals"
      value: "admin"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize claim verification rule");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.claims.len(), 1);

    let claim_rule = &jwt.claims[0];
    assert_eq!(claim_rule.claim, "role");
    assert_eq!(claim_rule.operator, "equals");
    assert_eq!(claim_rule.value.as_str().unwrap(), "admin");
}

#[test]
fn test_can_parse_multiple_claim_verification_rules() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "role"
      operator: "equals"
      value: "admin"
    - claim: "department"
      operator: "equals"
      value: "engineering"
"#;
    let config: Config = serde_yaml::from_str(yaml)
        .expect("Failed to deserialize multiple claim verification rules");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.claims.len(), 2);

    let first_rule = &jwt.claims[0];
    assert_eq!(first_rule.claim, "role");
    assert_eq!(first_rule.operator, "equals");
    assert_eq!(first_rule.value.as_str().unwrap(), "admin");

    let second_rule = &jwt.claims[1];
    assert_eq!(second_rule.claim, "department");
    assert_eq!(second_rule.operator, "equals");
    assert_eq!(second_rule.value.as_str().unwrap(), "engineering");
}

#[test]
fn test_can_parse_equals_operator() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "role"
      operator: "equals"
      value: "admin"
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize equals operator");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    let claim_rule = &jwt.claims[0];
    assert_eq!(claim_rule.operator, "equals");
}

#[test]
fn test_can_parse_string_claim_value() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "username"
      operator: "equals"
      value: "alice@example.com"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize string claim value");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    let claim_rule = &jwt.claims[0];
    assert_eq!(claim_rule.value.as_str().unwrap(), "alice@example.com");
}

#[test]
fn test_can_parse_numeric_claim_value() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "user_level"
      operator: "equals"
      value: 5
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize numeric claim value");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    let claim_rule = &jwt.claims[0];
    assert_eq!(claim_rule.value.as_i64().unwrap(), 5);
}

#[test]
fn test_can_parse_boolean_claim_value() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "is_admin"
      operator: "equals"
      value: true
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize boolean claim value");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    let claim_rule = &jwt.claims[0];
    assert_eq!(claim_rule.value.as_bool().unwrap(), true);
}

#[test]
fn test_can_parse_array_claim_value() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  claims:
    - claim: "role"
      operator: "in"
      value: ["admin", "moderator", "owner"]
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize array claim value");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    let claim_rule = &jwt.claims[0];
    let array = claim_rule.value.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str().unwrap(), "admin");
    assert_eq!(array[1].as_str().unwrap(), "moderator");
    assert_eq!(array[2].as_str().unwrap(), "owner");
}

#[test]
fn test_rejects_claim_verification_with_unknown_operator() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  token_sources:
    - type: "header"
      name: "Authorization"
      prefix: "Bearer "
  claims:
    - claim: "role"
      operator: "invalid_operator"
      value: "admin"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with unknown operator");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with unknown operator"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("invalid_operator") || err_msg.contains("operator"),
        "Error message should mention the invalid operator or operator field"
    );
}

#[test]
fn test_validates_that_all_path_prefixes_are_unique() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "bucket1"
    path_prefix: "/api/v1"
    s3:
      bucket: "my-bucket-1"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  - name: "bucket2"
    path_prefix: "/api/v1"
    s3:
      bucket: "my-bucket-2"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE2"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with duplicate paths");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with duplicate path_prefix"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("/api/v1") || err_msg.contains("duplicate") || err_msg.contains("path"),
        "Error message should mention the duplicate path_prefix"
    );
}

#[test]
fn test_validates_that_all_path_prefixes_start_with_slash() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "api/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with invalid path_prefix");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with path_prefix not starting with /"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("api/products") || err_msg.contains("/") || err_msg.contains("start"),
        "Error message should mention the path_prefix or / requirement"
    );
}

#[test]
fn test_validates_that_bucket_names_are_not_empty() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: ""
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with empty bucket name");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with empty bucket name"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("name") || err_msg.contains("empty") || err_msg.contains("bucket"),
        "Error message should mention name or empty bucket"
    );
}

#[test]
fn test_validates_that_jwt_secret_exists_when_auth_is_enabled() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: ""
  algorithm: "HS256"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with empty JWT secret");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail when JWT is enabled but secret is empty"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("secret") || err_msg.contains("JWT") || err_msg.contains("empty"),
        "Error message should mention JWT secret requirement"
    );
}

#[test]
fn test_validates_that_at_least_one_token_source_exists_when_jwt_enabled() {
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
  algorithm: "HS256"
  token_sources: []
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with no token sources");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail when JWT is enabled but no token sources are defined"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("token") || err_msg.contains("source") || err_msg.contains("JWT"),
        "Error message should mention token source requirement"
    );
}

#[test]
fn test_full_config_validation_passes_with_valid_config() {
    let yaml = r#"
server:
  address: "0.0.0.0"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/api/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  - name: "images"
    path_prefix: "/api/images"
    s3:
      bucket: "my-images-bucket"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE2"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
jwt:
  enabled: true
  secret: "my-super-secret-jwt-key"
  algorithm: "HS256"
  token_sources:
    - type: "header"
      name: "Authorization"
      prefix: "Bearer "
    - type: "query"
      name: "token"
  claims:
    - claim: "role"
      operator: "equals"
      value: "admin"
    - claim: "department"
      operator: "in"
      value: ["engineering", "operations"]
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to deserialize valid config");
    let validation_result = config.validate();
    assert!(
        validation_result.is_ok(),
        "Expected validation to pass with valid config, but got error: {:?}",
        validation_result.err()
    );
}

#[test]
fn test_full_config_validation_fails_with_invalid_config() {
    // Config with duplicate path_prefix
    let yaml = r#"
server:
  address: "0.0.0.0"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/api/data"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  - name: "images"
    path_prefix: "/api/data"
    s3:
      bucket: "my-images-bucket"
      region: "us-east-1"
      access_key: "AKIAIOSFODNN7EXAMPLE2"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
jwt:
  enabled: true
  secret: "my-super-secret-jwt-key"
  algorithm: "HS256"
  token_sources:
    - type: "header"
      name: "Authorization"
      prefix: "Bearer "
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize config with duplicate paths");
    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Expected validation to fail with duplicate path_prefix"
    );
    let err_msg = validation_result.unwrap_err();
    assert!(
        err_msg.contains("Duplicate") || err_msg.contains("path"),
        "Error message should mention duplicate path_prefix"
    );
}

#[test]
fn test_can_load_config_from_yaml_file_path() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-test-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file
        .write_all(yaml.as_bytes())
        .expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");

    let config = Config::from_file(temp_file.path()).expect("Failed to load config from file");

    assert_eq!(config.server.address, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.buckets.len(), 1);
    assert_eq!(config.buckets[0].name, "test-bucket");
}

#[test]
fn test_returns_error_for_non_existent_file() {
    let non_existent_path = "/tmp/this_file_definitely_does_not_exist_12345.yaml";
    let result = Config::from_file(non_existent_path);

    assert!(result.is_err(), "Expected error for non-existent file");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("Failed to read config file"),
        "Error message should mention failed to read config file, got: {}",
        err_msg
    );
}

#[test]
#[cfg(unix)]
fn test_returns_error_for_unreadable_file() {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-test-bucket"
      region: "us-west-2"
      access_key: "AKIAIOSFODNN7EXAMPLE"
      secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
"#;
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file
        .write_all(yaml.as_bytes())
        .expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");

    // Remove read permissions (mode 000)
    let permissions = fs::Permissions::from_mode(0o000);
    fs::set_permissions(temp_file.path(), permissions).expect("Failed to set permissions");

    let result = Config::from_file(temp_file.path());

    // Restore read permissions before assertions (cleanup)
    let permissions = fs::Permissions::from_mode(0o644);
    let _ = fs::set_permissions(temp_file.path(), permissions);

    assert!(result.is_err(), "Expected error for unreadable file");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("Failed to read config file"),
        "Error message should mention failed to read config file, got: {}",
        err_msg
    );
}

#[test]
fn test_returns_error_for_malformed_yaml() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Invalid YAML with missing colon and bad indentation
    let malformed_yaml = r#"
server
  address "127.0.0.1"
    port: 8080
buckets
  - name: "test-bucket"
"#;
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file
        .write_all(malformed_yaml.as_bytes())
        .expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");

    let result = Config::from_file(temp_file.path());

    assert!(result.is_err(), "Expected error for malformed YAML");
    let err_msg = result.unwrap_err();
    // Error message should indicate parsing/deserialization failure
    assert!(
        !err_msg.is_empty(),
        "Error message should not be empty for malformed YAML"
    );
}

// ============================================================================
// Phase 31.1: JWT Algorithm Configuration Tests (RS256/ES256)
// ============================================================================

#[test]
fn test_can_parse_jwt_algorithm_rs256() {
    // Test that RS256 algorithm is recognized as a valid algorithm
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "placeholder-for-rsa-key-path"
  algorithm: "RS256"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT algorithm RS256");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.algorithm, "RS256");
}

#[test]
fn test_can_parse_jwt_algorithm_es256() {
    // Test that ES256 algorithm is recognized as a valid algorithm
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "placeholder-for-ecdsa-key-path"
  algorithm: "ES256"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT algorithm ES256");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.algorithm, "ES256");
}

#[test]
fn test_jwt_algorithm_is_required_when_jwt_enabled() {
    // Test that algorithm field is required when JWT is enabled
    // Note: Currently algorithm has no default value and will fail to parse if missing
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  secret: "my-jwt-secret"
"#;
    // Without algorithm field, deserialization should fail
    let result: Result<Config, _> = serde_yaml::from_str(yaml);
    assert!(
        result.is_err(),
        "Expected deserialization to fail when algorithm is missing"
    );
}

#[test]
fn test_can_parse_jwt_rsa_public_key_path() {
    // Test that rsa_public_key_path can be configured for RS256
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "RS256"
  rsa_public_key_path: "/etc/yatagarasu/rsa_public.pem"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT RS256 config");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.algorithm, "RS256");
    assert_eq!(
        jwt.rsa_public_key_path,
        Some("/etc/yatagarasu/rsa_public.pem".to_string())
    );
}

#[test]
fn test_can_parse_jwt_ecdsa_public_key_path() {
    // Test that ecdsa_public_key_path can be configured for ES256
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "ES256"
  ecdsa_public_key_path: "/etc/yatagarasu/ecdsa_public.pem"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT ES256 config");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.algorithm, "ES256");
    assert_eq!(
        jwt.ecdsa_public_key_path,
        Some("/etc/yatagarasu/ecdsa_public.pem".to_string())
    );
}

// =============================================================================
// Phase 31.4: Multiple Key Support (Key Rotation)
// =============================================================================

#[test]
fn test_jwt_config_can_have_keys_array() {
    // Test that JWT config supports a keys array for multiple key rotation
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  secret: "default-secret"
  keys: []
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with keys array");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert!(jwt.keys.is_empty());
}

#[test]
fn test_jwt_key_has_id_algorithm_and_path() {
    // Test that each key entry has id, algorithm, and path fields
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  keys:
    - id: "key-1"
      algorithm: "RS256"
      path: "/etc/keys/rsa_public.pem"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with key entry");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.keys.len(), 1);
    let key = &jwt.keys[0];
    assert_eq!(key.id, "key-1");
    assert_eq!(key.algorithm, "RS256");
    assert_eq!(key.path, Some("/etc/keys/rsa_public.pem".to_string()));
}

#[test]
fn test_can_load_multiple_keys_in_config() {
    // Test that multiple keys can be configured
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  keys:
    - id: "key-rsa-1"
      algorithm: "RS256"
      path: "/etc/keys/rsa1_public.pem"
    - id: "key-rsa-2"
      algorithm: "RS256"
      path: "/etc/keys/rsa2_public.pem"
    - id: "key-ecdsa-1"
      algorithm: "ES256"
      path: "/etc/keys/ecdsa_public.pem"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with multiple keys");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.keys.len(), 3);
    assert_eq!(jwt.keys[0].id, "key-rsa-1");
    assert_eq!(jwt.keys[1].id, "key-rsa-2");
    assert_eq!(jwt.keys[2].id, "key-ecdsa-1");
}

#[test]
fn test_can_mix_hs256_rs256_es256_keys() {
    // Test that different algorithm types can be mixed in keys array
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  keys:
    - id: "hmac-key"
      algorithm: "HS256"
      secret: "shared-secret-for-hs256"
    - id: "rsa-key"
      algorithm: "RS256"
      path: "/etc/keys/rsa_public.pem"
    - id: "ecdsa-key"
      algorithm: "ES256"
      path: "/etc/keys/ecdsa_public.pem"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with mixed algorithms");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.keys.len(), 3);

    // HS256 key has secret
    assert_eq!(jwt.keys[0].algorithm, "HS256");
    assert_eq!(
        jwt.keys[0].secret,
        Some("shared-secret-for-hs256".to_string())
    );

    // RS256 key has path
    assert_eq!(jwt.keys[1].algorithm, "RS256");
    assert_eq!(
        jwt.keys[1].path,
        Some("/etc/keys/rsa_public.pem".to_string())
    );

    // ES256 key has path
    assert_eq!(jwt.keys[2].algorithm, "ES256");
    assert_eq!(
        jwt.keys[2].path,
        Some("/etc/keys/ecdsa_public.pem".to_string())
    );
}

// =============================================================================
// Phase 31.5: JWKS (JSON Web Key Set) Configuration Tests
// =============================================================================

#[test]
fn test_jwt_config_can_have_jwks_url() {
    // Test that JWT config supports jwks_url for dynamic key fetching
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "RS256"
  jwks_url: "https://example.com/.well-known/jwks.json"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with jwks_url");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(
        jwt.jwks_url,
        Some("https://example.com/.well-known/jwks.json".to_string())
    );
}

#[test]
fn test_jwks_url_is_optional() {
    // Test that jwks_url is optional (defaults to None)
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  secret: "test-secret"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config without jwks_url");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert!(jwt.jwks_url.is_none());
}

#[test]
fn test_jwks_url_with_refresh_interval() {
    // Test that jwks_refresh_interval can be configured
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "RS256"
  jwks_url: "https://auth.example.com/.well-known/jwks.json"
  jwks_refresh_interval_secs: 3600
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to deserialize JWT config with refresh interval");
    let jwt = config.jwt.as_ref().expect("JWT config should be present");
    assert_eq!(jwt.jwks_refresh_interval_secs, Some(3600));
}

// ============================================================================
// Phase 32: OPA Integration - Configuration Tests
// ============================================================================

#[test]
fn test_bucket_can_have_authorization_section() {
    // Test: Add `authorization` section to bucket config
    use yatagarasu::config::AuthorizationConfig;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to parse config with authorization section");
    let bucket = &config.buckets[0];
    assert!(
        bucket.authorization.is_some(),
        "Bucket should have authorization section"
    );
}

#[test]
fn test_authorization_type_opa() {
    // Test: Can parse `type: opa` authorization type
    use yatagarasu::config::AuthorizationConfig;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse authorization type");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(authz.auth_type, "opa");
}

#[test]
fn test_authorization_opa_url() {
    // Test: Can parse `opa_url` (OPA REST API endpoint)
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://opa.example.com:8181
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse opa_url");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(
        authz.opa_url.as_deref(),
        Some("http://opa.example.com:8181")
    );
}

#[test]
fn test_authorization_opa_policy_path() {
    // Test: Can parse `opa_policy_path` (e.g., "yatagarasu/authz/allow")
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
      opa_policy_path: yatagarasu/authz/allow
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse opa_policy_path");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(
        authz.opa_policy_path.as_deref(),
        Some("yatagarasu/authz/allow")
    );
}

#[test]
fn test_authorization_opa_timeout_ms() {
    // Test: Can parse `opa_timeout_ms` (default: 100ms)
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
      opa_timeout_ms: 200
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse opa_timeout_ms");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(authz.opa_timeout_ms, 200);
}

#[test]
fn test_authorization_opa_timeout_default() {
    // Test: Default opa_timeout_ms is 100ms
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse config");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(authz.opa_timeout_ms, 100, "Default timeout should be 100ms");
}

#[test]
fn test_authorization_opa_cache_ttl() {
    // Test: Can parse `opa_cache_ttl_seconds` (default: 60)
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
      opa_cache_ttl_seconds: 120
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse opa_cache_ttl_seconds");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(authz.opa_cache_ttl_seconds, 120);
}

#[test]
fn test_authorization_opa_cache_ttl_default() {
    // Test: Default opa_cache_ttl_seconds is 60
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse config");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(
        authz.opa_cache_ttl_seconds, 60,
        "Default cache TTL should be 60 seconds"
    );
}

#[test]
fn test_authorization_opa_fail_mode() {
    // Test: Can parse `opa_fail_mode` (open or closed)
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
      opa_fail_mode: open
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse opa_fail_mode");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");
    assert_eq!(authz.opa_fail_mode.as_deref(), Some("open"));
}

#[test]
fn test_authorization_opa_complete_config() {
    // Test: Can parse complete OPA config example
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
    authorization:
      type: opa
      opa_url: http://localhost:8181
      opa_policy_path: yatagarasu/authz/allow
      opa_timeout_ms: 100
      opa_cache_ttl_seconds: 60
      opa_fail_mode: closed
"#;
    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse complete OPA config");
    let bucket = &config.buckets[0];
    let authz = bucket.authorization.as_ref().expect("Should have authz");

    assert_eq!(authz.auth_type, "opa");
    assert_eq!(authz.opa_url.as_deref(), Some("http://localhost:8181"));
    assert_eq!(
        authz.opa_policy_path.as_deref(),
        Some("yatagarasu/authz/allow")
    );
    assert_eq!(authz.opa_timeout_ms, 100);
    assert_eq!(authz.opa_cache_ttl_seconds, 60);
    assert_eq!(authz.opa_fail_mode.as_deref(), Some("closed"));
}

#[test]
fn test_authorization_section_is_optional() {
    // Test: Authorization section is optional (existing buckets without it should work)
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: products
    path_prefix: /products
    s3:
      bucket: products-bucket
      region: us-east-1
      access_key: test
      secret_key: test
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to parse config without authorization");
    let bucket = &config.buckets[0];
    assert!(
        bucket.authorization.is_none(),
        "Authorization should be None when not configured"
    );
}

// =============================================================================
// Phase 48.1: OpenFGA Configuration Tests
// =============================================================================

#[test]
fn test_can_parse_openfga_authorization_config() {
    // Test: Parse OpenFGA config from bucket auth section
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "shared-files"
    path_prefix: "/shared"
    s3:
      bucket: "shared-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
    authorization:
      type: "openfga"
      openfga_endpoint: "http://localhost:8080"
      openfga_store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to parse OpenFGA authorization config");
    let bucket = &config.buckets[0];
    let authz = bucket
        .authorization
        .as_ref()
        .expect("Should have authorization");

    assert_eq!(authz.auth_type, "openfga");
    assert_eq!(
        authz.openfga_endpoint.as_deref(),
        Some("http://localhost:8080")
    );
    assert_eq!(
        authz.openfga_store_id.as_deref(),
        Some("01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
    // Verify defaults for timeout and cache TTL
    assert_eq!(authz.openfga_timeout_ms, 100); // default value
    assert_eq!(authz.openfga_cache_ttl_seconds, 60); // default value
}

#[test]
fn test_can_parse_complete_openfga_config_with_all_optional_fields() {
    // Test: Parse OpenFGA config with all optional fields populated
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "secure-documents"
    path_prefix: "/docs"
    s3:
      bucket: "secure-docs-bucket"
      region: "us-west-2"
      access_key: "test"
      secret_key: "test"
    authorization:
      type: "openfga"
      openfga_endpoint: "https://api.fga.example.com"
      openfga_store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"
      openfga_authorization_model_id: "01GXSA8YR785C4FYS3C0RTG7B1"
      openfga_api_token: "fga_live_sk_test123"
      openfga_timeout_ms: 200
      openfga_cache_ttl_seconds: 120
      openfga_fail_mode: "open"
"#;
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to parse complete OpenFGA config");
    let bucket = &config.buckets[0];
    let authz = bucket
        .authorization
        .as_ref()
        .expect("Should have authorization");

    assert_eq!(authz.auth_type, "openfga");
    assert_eq!(
        authz.openfga_endpoint.as_deref(),
        Some("https://api.fga.example.com")
    );
    assert_eq!(
        authz.openfga_store_id.as_deref(),
        Some("01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
    assert_eq!(
        authz.openfga_authorization_model_id.as_deref(),
        Some("01GXSA8YR785C4FYS3C0RTG7B1")
    );
    assert_eq!(
        authz.openfga_api_token.as_deref(),
        Some("fga_live_sk_test123")
    );
    assert_eq!(authz.openfga_timeout_ms, 200);
    assert_eq!(authz.openfga_cache_ttl_seconds, 120);
    assert_eq!(authz.openfga_fail_mode.as_deref(), Some("open"));
}

#[test]
fn test_openfga_config_allows_env_var_substitution() {
    // Test: OpenFGA API token supports environment variable substitution
    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "cloud-files"
    path_prefix: "/cloud"
    s3:
      bucket: "cloud-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
    authorization:
      type: "openfga"
      openfga_endpoint: "https://api.fga.example.com"
      openfga_store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"
      openfga_api_token: "${OPENFGA_API_TOKEN}"
"#;
    // Just verify it parses - actual substitution happens in resolve_env_vars
    let config: Config =
        serde_yaml::from_str(yaml).expect("Failed to parse OpenFGA config with env var");
    let bucket = &config.buckets[0];
    let authz = bucket
        .authorization
        .as_ref()
        .expect("Should have authorization");

    assert_eq!(authz.auth_type, "openfga");
    assert_eq!(
        authz.openfga_api_token.as_deref(),
        Some("${OPENFGA_API_TOKEN}")
    );
}
