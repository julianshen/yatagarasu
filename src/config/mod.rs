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
    #[serde(skip)]
    pub generation: u64, // Config version, increments on reload
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

        let mut config: Config = serde_yaml::from_str(&substituted).map_err(|e| e.to_string())?;
        config.generation = 0; // Initialize generation to 0
        Ok(config)
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

            // Validate token source types and required fields
            const VALID_SOURCE_TYPES: &[&str] = &["bearer", "header", "query"];
            for (idx, source) in jwt.token_sources.iter().enumerate() {
                // Validate source type
                if !VALID_SOURCE_TYPES.contains(&source.source_type.as_str()) {
                    return Err(format!(
                        "Invalid token source type '{}' at index {}. Supported types: {}",
                        source.source_type,
                        idx,
                        VALID_SOURCE_TYPES.join(", ")
                    ));
                }

                // Validate that 'header' and 'query' types have 'name' field
                if matches!(source.source_type.as_str(), "header" | "query")
                    && source.name.is_none()
                {
                    return Err(format!(
                        "Token source type '{}' at index {} requires 'name' field",
                        source.source_type, idx
                    ));
                }
            }
        }

        Ok(())
    }
}

// Default timeout values
fn default_request_timeout() -> u64 {
    30 // 30 seconds
}

fn default_s3_timeout() -> u64 {
    20 // 20 seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3Config,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default = "default_s3_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_can_be_loaded_from_file_path() {
        // Create temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
"#;
        temp_file.write_all(config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Load config from file
        let config = Config::from_file(temp_file.path()).unwrap();

        // Verify config was loaded correctly
        assert_eq!(config.server.address, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.buckets.len(), 1);
        assert_eq!(config.buckets[0].name, "test-bucket");
        assert_eq!(config.buckets[0].path_prefix, "/test");
        assert_eq!(config.generation, 0); // Initial generation is 0
    }

    #[test]
    fn test_config_validation_catches_errors_before_applying() {
        // Load invalid config (duplicate path_prefix)
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_config = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "my-bucket-1"
      region: "us-east-1"
      access_key: "test-key-1"
      secret_key: "test-secret-1"
  - name: "bucket2"
    path_prefix: "/api"
    s3:
      bucket: "my-bucket-2"
      region: "us-east-1"
      access_key: "test-key-2"
      secret_key: "test-secret-2"
"#;
        temp_file.write_all(invalid_config.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Load config (succeeds) but validation should fail
        let config = Config::from_file(temp_file.path()).unwrap();
        let validation_result = config.validate();

        assert!(
            validation_result.is_err(),
            "Validation should catch duplicate path_prefix"
        );
        assert!(validation_result
            .unwrap_err()
            .contains("Duplicate path_prefix"));
    }

    #[test]
    fn test_invalid_config_rejected_without_affecting_running_service() {
        // Simulate existing valid config
        let mut valid_file = NamedTempFile::new().unwrap();
        let valid_config = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
"#;
        valid_file.write_all(valid_config.as_bytes()).unwrap();
        valid_file.flush().unwrap();

        // Load valid config
        let current_config = Config::from_file(valid_file.path()).unwrap();
        current_config.validate().unwrap();
        let current_generation = current_config.generation;

        // Attempt to load invalid config (empty bucket name)
        let mut invalid_file = NamedTempFile::new().unwrap();
        let invalid_config = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: ""
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
"#;
        invalid_file.write_all(invalid_config.as_bytes()).unwrap();
        invalid_file.flush().unwrap();

        // Try to load new config
        let new_config_result = Config::from_file(invalid_file.path());
        assert!(new_config_result.is_ok(), "Config should load");

        let new_config = new_config_result.unwrap();
        let validation_result = new_config.validate();

        // Validation should fail
        assert!(
            validation_result.is_err(),
            "Invalid config should fail validation"
        );

        // Current config should remain unchanged (simulated by checking generation)
        assert_eq!(current_config.generation, current_generation);
        assert_eq!(current_config.buckets.len(), 1);
        assert_eq!(current_config.buckets[0].name, "test-bucket");
    }

    #[test]
    fn test_config_has_generation_number() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config: Config = Config::from_yaml_with_env(yaml).unwrap();

        // Initial config should have generation 0
        assert_eq!(config.generation, 0);
    }

    #[test]
    fn test_server_config_has_request_timeout_with_default() {
        // Test 1: Config without request_timeout should use default (30s)
        let yaml_without_timeout = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml_without_timeout).unwrap();

        // Should default to 30 seconds
        assert_eq!(
            config.server.request_timeout, 30,
            "ServerConfig should default to 30 second request timeout"
        );

        // Test 2: Config with explicit request_timeout should use that value
        let yaml_with_timeout = r#"
server:
  address: "127.0.0.1"
  port: 8080
  request_timeout: 60
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml_with_timeout).unwrap();

        assert_eq!(
            config.server.request_timeout, 60,
            "ServerConfig should use explicit request_timeout value"
        );
    }

    #[test]
    fn test_s3_config_has_timeout_with_default() {
        // Test 1: S3 config without timeout should use default (20s)
        let yaml_without_timeout = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
"#;
        let config = Config::from_yaml_with_env(yaml_without_timeout).unwrap();

        // Should default to 20 seconds
        assert_eq!(
            config.buckets[0].s3.timeout, 20,
            "S3Config should default to 20 second timeout"
        );

        // Test 2: S3 config with explicit timeout should use that value
        let yaml_with_timeout = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
      timeout: 45
"#;
        let config = Config::from_yaml_with_env(yaml_with_timeout).unwrap();

        assert_eq!(
            config.buckets[0].s3.timeout, 45,
            "S3Config should use explicit timeout value"
        );
    }
}
