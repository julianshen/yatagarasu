// Configuration module

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub buckets: Vec<BucketConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt: Option<JwtConfig>,
}

impl Config {
    pub fn from_yaml_with_env(yaml: &str) -> Result<Self, String> {
        // Replace ${VAR_NAME} with environment variable values
        let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").map_err(|e| e.to_string())?;

        // First, check that all referenced environment variables exist
        for caps in re.captures_iter(yaml) {
            let var_name = &caps[1];
            std::env::var(var_name).map_err(|_| {
                format!(
                    "Environment variable '{}' is referenced but not set",
                    var_name
                )
            })?;
        }

        // Now perform the substitution (we know all vars exist)
        let substituted = re.replace_all(yaml, |caps: &regex::Captures| {
            let var_name = &caps[1];
            std::env::var(var_name).unwrap() // Safe because we checked above
        });

        serde_yaml::from_str(&substituted).map_err(|e| e.to_string())
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen_prefixes = HashSet::new();

        // Validate each bucket configuration
        for bucket in &self.buckets {
            if bucket.path_prefix.is_empty() {
                return Err(format!("Bucket '{}' has empty path_prefix", bucket.name));
            }

            // Check for duplicate path_prefix
            if !seen_prefixes.insert(&bucket.path_prefix) {
                return Err(format!(
                    "Duplicate path_prefix '{}' found in bucket '{}'",
                    bucket.path_prefix, bucket.name
                ));
            }
        }

        // Validate JWT configuration if present
        if let Some(jwt) = &self.jwt {
            // Validate algorithm
            const VALID_ALGORITHMS: &[&str] = &["HS256", "HS384", "HS512"];
            if !VALID_ALGORITHMS.contains(&jwt.algorithm.as_str()) {
                return Err(format!(
                    "Invalid JWT algorithm '{}'. Supported algorithms: {}",
                    jwt.algorithm,
                    VALID_ALGORITHMS.join(", ")
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub enabled: bool,
    pub secret: String,
    pub algorithm: String,
    #[serde(default)]
    pub token_sources: Vec<TokenSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_empty_config_struct() {
        let _config = Config {
            server: ServerConfig {
                address: String::from("127.0.0.1"),
                port: 8080,
            },
            buckets: vec![],
            jwt: None,
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
        let config: Config = serde_yaml::from_str(yaml)
            .expect("Failed to deserialize JWT config with enabled=false");
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
        let config: Config =
            serde_yaml::from_str(yaml).expect("Failed to deserialize JWT algorithm");
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
        let config: Config = serde_yaml::from_str(yaml)
            .expect("Failed to deserialize config with invalid algorithm");
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
}
