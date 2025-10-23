// Configuration module

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

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

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let yaml = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        Self::from_yaml_with_env(&yaml)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen_prefixes = HashSet::new();

        // Validate each bucket configuration
        for bucket in &self.buckets {
            // Check that bucket name is not empty
            if bucket.name.is_empty() {
                return Err("Bucket name cannot be empty".to_string());
            }

            if bucket.path_prefix.is_empty() {
                return Err(format!("Bucket '{}' has empty path_prefix", bucket.name));
            }

            // Check that path_prefix starts with /
            if !bucket.path_prefix.starts_with('/') {
                return Err(format!(
                    "Bucket '{}' has path_prefix '{}' that does not start with /",
                    bucket.name, bucket.path_prefix
                ));
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
            // Validate that secret is not empty when JWT is enabled
            if jwt.enabled && jwt.secret.is_empty() {
                return Err("JWT secret cannot be empty when authentication is enabled".to_string());
            }

            // Validate algorithm
            const VALID_ALGORITHMS: &[&str] = &["HS256", "HS384", "HS512"];
            if !VALID_ALGORITHMS.contains(&jwt.algorithm.as_str()) {
                return Err(format!(
                    "Invalid JWT algorithm '{}'. Supported algorithms: {}",
                    jwt.algorithm,
                    VALID_ALGORITHMS.join(", ")
                ));
            }

            // Validate that at least one token source exists when JWT is enabled
            if jwt.enabled && jwt.token_sources.is_empty() {
                return Err(
                    "At least one token source must be configured when JWT authentication is enabled"
                        .to_string(),
                );
            }

            // Validate claim operators
            const VALID_OPERATORS: &[&str] =
                &["equals", "in", "contains", "gt", "lt", "gte", "lte"];
            for claim_rule in &jwt.claims {
                if !VALID_OPERATORS.contains(&claim_rule.operator.as_str()) {
                    return Err(format!(
                        "Invalid claim operator '{}'. Supported operators: {}",
                        claim_rule.operator,
                        VALID_OPERATORS.join(", ")
                    ));
                }
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
    #[serde(default)]
    pub claims: Vec<ClaimRule>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRule {
    pub claim: String,
    pub operator: String,
    pub value: serde_json::Value,
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
        let config: Config =
            serde_yaml::from_str(yaml).expect("Failed to deserialize equals operator");
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
            err_msg.contains("/api/v1")
                || err_msg.contains("duplicate")
                || err_msg.contains("path"),
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
        let config: Config = serde_yaml::from_str(yaml)
            .expect("Failed to deserialize config with invalid path_prefix");
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
        let config: Config = serde_yaml::from_str(yaml)
            .expect("Failed to deserialize config with empty bucket name");
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
        let config: Config =
            serde_yaml::from_str(yaml).expect("Failed to deserialize valid config");
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
}
