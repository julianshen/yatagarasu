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

        // Sort replicas by priority (1 = highest priority)
        for bucket in &mut config.buckets {
            if let Some(replicas) = &mut bucket.s3.replicas {
                replicas.sort_by_key(|r| r.priority);
            }
        }

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

            // Validate replica set if present (Phase 23: HA Bucket Replication)
            if let Some(replicas) = &bucket.s3.replicas {
                // Check that at least one replica is defined
                if replicas.is_empty() {
                    return Err(format!(
                        "Bucket '{}': Replica set cannot be empty. At least one replica is required.",
                        bucket.name
                    ));
                }

                // Check for duplicate priorities within bucket
                let mut seen_priorities = HashSet::new();
                // Check for duplicate names within bucket
                let mut seen_names = HashSet::new();

                for replica in replicas {
                    // Check replica name is not empty
                    if replica.name.trim().is_empty() {
                        return Err(format!(
                            "Bucket '{}': Replica name cannot be empty. Each replica must have a non-empty name.",
                            bucket.name
                        ));
                    }

                    // Check replica bucket is not empty
                    if replica.bucket.trim().is_empty() {
                        return Err(format!(
                            "Bucket '{}': Replica '{}' has empty bucket name. Each replica must have a non-empty bucket name.",
                            bucket.name, replica.name
                        ));
                    }

                    // Check timeout is greater than 0
                    if replica.timeout == 0 {
                        return Err(format!(
                            "Bucket '{}': Replica '{}' has timeout 0. Timeout must be > 0 seconds.",
                            bucket.name, replica.name
                        ));
                    }

                    // Check priority is at least 1
                    if replica.priority < 1 {
                        return Err(format!(
                            "Bucket '{}': Replica '{}' has priority {}. Priority must be >= 1.",
                            bucket.name, replica.name, replica.priority
                        ));
                    }

                    // Check for duplicate priorities
                    if !seen_priorities.insert(replica.priority) {
                        return Err(format!(
                            "Bucket '{}': Duplicate priority {} found in replica set. Each replica must have a unique priority.",
                            bucket.name, replica.priority
                        ));
                    }

                    // Check for duplicate names
                    if !seen_names.insert(&replica.name) {
                        return Err(format!(
                            "Bucket '{}': Duplicate replica name '{}' found. Each replica must have a unique name.",
                            bucket.name, replica.name
                        ));
                    }
                }
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

    /// Normalize the configuration by converting legacy single-bucket format to replica format.
    /// This provides a unified code path where all buckets use the replica-based structure internally.
    pub fn normalize(&self) -> Config {
        let mut normalized = self.clone();

        for bucket in &mut normalized.buckets {
            // If replicas is None, convert legacy fields to single-replica format
            if bucket.s3.replicas.is_none() {
                // Only convert if legacy fields are populated (non-empty)
                if !bucket.s3.bucket.is_empty() {
                    let replica = S3Replica {
                        name: "default".to_string(),
                        bucket: bucket.s3.bucket.clone(),
                        region: bucket.s3.region.clone(),
                        access_key: bucket.s3.access_key.clone(),
                        secret_key: bucket.s3.secret_key.clone(),
                        endpoint: bucket.s3.endpoint.clone(),
                        priority: 1,
                        timeout: bucket.s3.timeout,
                    };

                    bucket.s3.replicas = Some(vec![replica]);
                }
            }
        }

        normalized
    }
}

// Default timeout values
fn default_request_timeout() -> u64 {
    30 // 30 seconds
}

fn default_s3_timeout() -> u64 {
    20 // 20 seconds
}

// Default connection pool values
fn default_max_concurrent_requests() -> usize {
    1000 // 1000 concurrent requests
}

fn default_connection_pool_size() -> usize {
    50 // 50 connections per S3 bucket
}

// Default security limit values
fn default_max_body_size() -> usize {
    10 * 1024 * 1024 // 10 MB
}

fn default_max_header_size() -> usize {
    64 * 1024 // 64 KB
}

fn default_max_uri_length() -> usize {
    8192 // 8 KB
}

/// Security validation limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLimitsConfig {
    /// Maximum request body size in bytes (default: 10 MB)
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    /// Maximum total header size in bytes (default: 64 KB)
    #[serde(default = "default_max_header_size")]
    pub max_header_size: usize,
    /// Maximum URI length in bytes (default: 8 KB)
    #[serde(default = "default_max_uri_length")]
    pub max_uri_length: usize,
}

impl Default for SecurityLimitsConfig {
    fn default() -> Self {
        Self {
            max_body_size: default_max_body_size(),
            max_header_size: default_max_header_size(),
            max_uri_length: default_max_uri_length(),
        }
    }
}

impl SecurityLimitsConfig {
    /// Convert to SecurityLimits from security module
    pub fn to_security_limits(&self) -> crate::security::SecurityLimits {
        crate::security::SecurityLimits {
            max_body_size: self.max_body_size,
            max_header_size: self.max_header_size,
            max_uri_length: self.max_uri_length,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfigYaml>,
    #[serde(default)]
    pub security_limits: SecurityLimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3Config,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

/// S3 Replica configuration (for HA bucket replication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Replica {
    pub name: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub priority: u8,
    #[serde(default = "default_s3_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    // Legacy single-bucket fields (for backward compatibility - kept as non-optional to avoid breaking existing code)
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub secret_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default = "default_s3_timeout")]
    pub timeout: u64,
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerConfigYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<BucketRateLimitConfigYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfigYaml>,

    // New replica set field (for HA - optional, mutually exclusive with legacy fields)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<Vec<S3Replica>>,
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

/// Circuit breaker configuration (YAML format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfigYaml {
    /// Number of consecutive failures to open circuit
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    /// Number of successes in half-open to close circuit
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
    /// How long to wait before trying again (seconds)
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    /// Max concurrent test requests in half-open state
    #[serde(default = "default_half_open_max_requests")]
    pub half_open_max_requests: u32,
}

fn default_failure_threshold() -> u32 {
    5
}

fn default_success_threshold() -> u32 {
    2
}

fn default_timeout_seconds() -> u64 {
    60
}

fn default_half_open_max_requests() -> u32 {
    3
}

impl CircuitBreakerConfigYaml {
    /// Convert to CircuitBreakerConfig from circuit_breaker module
    pub fn to_circuit_breaker_config(&self) -> crate::circuit_breaker::CircuitBreakerConfig {
        crate::circuit_breaker::CircuitBreakerConfig {
            failure_threshold: self.failure_threshold,
            success_threshold: self.success_threshold,
            timeout_duration: std::time::Duration::from_secs(self.timeout_seconds),
            half_open_max_requests: self.half_open_max_requests,
        }
    }
}

/// Rate limiting configuration for server (global and per-IP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfigYaml {
    /// Enable rate limiting
    #[serde(default)]
    pub enabled: bool,
    /// Global rate limit (requests per second across all buckets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global: Option<GlobalRateLimitConfigYaml>,
    /// Per-IP rate limit (requests per second per client IP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_ip: Option<PerIpRateLimitConfigYaml>,
}

/// Global rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRateLimitConfigYaml {
    /// Requests per second (global limit)
    pub requests_per_second: u32,
}

/// Per-IP rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerIpRateLimitConfigYaml {
    /// Requests per second per IP address
    pub requests_per_second: u32,
}

/// Per-bucket rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketRateLimitConfigYaml {
    /// Requests per second for this bucket
    pub requests_per_second: u32,
}

/// Retry configuration (YAML format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfigYaml {
    /// Maximum number of retry attempts (including initial attempt)
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Initial backoff delay in milliseconds
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_initial_backoff_ms() -> u64 {
    100
}

fn default_max_backoff_ms() -> u64 {
    1000
}

impl RetryConfigYaml {
    /// Convert to RetryPolicy from retry module
    pub fn to_retry_policy(&self) -> crate::retry::RetryPolicy {
        crate::retry::RetryPolicy::new(
            self.max_attempts,
            self.initial_backoff_ms,
            self.max_backoff_ms,
        )
    }
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

    #[test]
    fn test_server_config_has_max_concurrent_requests_with_default() {
        // Test 1: Config without max_concurrent_requests should use default (1000)
        let yaml_without_limit = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml_without_limit).unwrap();

        // Should default to 1000 concurrent requests
        assert_eq!(
            config.server.max_concurrent_requests, 1000,
            "ServerConfig should default to 1000 max concurrent requests"
        );

        // Test 2: Config with explicit max_concurrent_requests should use that value
        let yaml_with_limit = r#"
server:
  address: "127.0.0.1"
  port: 8080
  max_concurrent_requests: 5000
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml_with_limit).unwrap();

        assert_eq!(
            config.server.max_concurrent_requests, 5000,
            "ServerConfig should use explicit max_concurrent_requests value"
        );
    }

    #[test]
    fn test_s3_config_has_connection_pool_size_with_default() {
        // Test 1: S3 config without connection_pool_size should use default (50)
        let yaml_without_pool = r#"
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
        let config = Config::from_yaml_with_env(yaml_without_pool).unwrap();

        // Should default to 50 connections
        assert_eq!(
            config.buckets[0].s3.connection_pool_size, 50,
            "S3Config should default to 50 connection pool size"
        );

        // Test 2: S3 config with explicit connection_pool_size should use that value
        let yaml_with_pool = r#"
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
      connection_pool_size: 100
"#;
        let config = Config::from_yaml_with_env(yaml_with_pool).unwrap();

        assert_eq!(
            config.buckets[0].s3.connection_pool_size, 100,
            "S3Config should use explicit connection_pool_size value"
        );
    }

    #[test]
    fn test_circuit_breaker_config_loaded_from_yaml() {
        let yaml = r#"
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
      circuit_breaker:
        failure_threshold: 3
        success_threshold: 1
        timeout_seconds: 30
        half_open_max_requests: 5
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Verify circuit breaker config loaded
        let cb_config = config.buckets[0].s3.circuit_breaker.as_ref().unwrap();
        assert_eq!(cb_config.failure_threshold, 3);
        assert_eq!(cb_config.success_threshold, 1);
        assert_eq!(cb_config.timeout_seconds, 30);
        assert_eq!(cb_config.half_open_max_requests, 5);
    }

    #[test]
    fn test_circuit_breaker_uses_defaults_when_omitted() {
        let yaml = r#"
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
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Verify circuit breaker is None when not specified
        assert!(config.buckets[0].s3.circuit_breaker.is_none());
    }

    #[test]
    fn test_circuit_breaker_config_conversion() {
        let yaml_config = CircuitBreakerConfigYaml {
            failure_threshold: 10,
            success_threshold: 3,
            timeout_seconds: 120,
            half_open_max_requests: 2,
        };

        let cb_config = yaml_config.to_circuit_breaker_config();

        assert_eq!(cb_config.failure_threshold, 10);
        assert_eq!(cb_config.success_threshold, 3);
        assert_eq!(
            cb_config.timeout_duration,
            std::time::Duration::from_secs(120)
        );
        assert_eq!(cb_config.half_open_max_requests, 2);
    }

    // Phase 23: High Availability Bucket Replication Tests
    // ======================================================

    #[test]
    fn test_can_parse_single_bucket_config_backward_compatibility() {
        // Test: The existing single-bucket config format should continue to work
        // This ensures backward compatibility - existing deployments don't break
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
      endpoint: "https://s3.us-west-2.amazonaws.com"
      timeout: 30
"#;

        // Parse config - should succeed without errors
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Verify bucket configuration loaded correctly
        assert_eq!(config.buckets.len(), 1);
        assert_eq!(config.buckets[0].name, "products");
        assert_eq!(config.buckets[0].path_prefix, "/products");

        // Verify S3 config fields (legacy format)
        let s3_config = &config.buckets[0].s3;
        assert_eq!(s3_config.bucket, "my-products-bucket");
        assert_eq!(s3_config.region, "us-west-2");
        assert_eq!(s3_config.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            s3_config.secret_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
        assert_eq!(
            s3_config.endpoint,
            Some("https://s3.us-west-2.amazonaws.com".to_string())
        );
        assert_eq!(s3_config.timeout, 30);

        // Verify replicas field is None for legacy config
        assert!(
            s3_config.replicas.is_none(),
            "Legacy config should not have replicas"
        );

        // Validation should pass
        config.validate().unwrap();
    }

    #[test]
    fn test_can_parse_replica_set_with_multiple_replicas() {
        // Test: New replica set format with multiple S3 buckets
        // This enables HA failover with priority-based replica selection
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          endpoint: "https://s3.us-west-2.amazonaws.com"
          priority: 1
          timeout: 30
        - name: "replica-eu"
          bucket: "products-eu-west-1"
          region: "eu-west-1"
          access_key: "AKIAIOSFODNN7EXAMPLE2"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
          endpoint: "https://s3.eu-west-1.amazonaws.com"
          priority: 2
          timeout: 25
        - name: "replica-minio"
          bucket: "products-backup"
          region: "us-east-1"
          access_key: "minioadmin"
          secret_key: "minioadmin"
          endpoint: "https://minio.example.com"
          priority: 3
          timeout: 20
"#;

        // Parse config - should succeed with replica set
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Verify bucket loaded
        assert_eq!(config.buckets.len(), 1);
        assert_eq!(config.buckets[0].name, "products");

        // Verify replica set structure exists
        let s3_config = &config.buckets[0].s3;
        assert!(
            s3_config.replicas.is_some(),
            "Replicas field should be present"
        );

        let replicas = s3_config.replicas.as_ref().unwrap();
        assert_eq!(replicas.len(), 3, "Should have 3 replicas");

        // Verify first replica (primary)
        let primary = &replicas[0];
        assert_eq!(primary.name, "primary");
        assert_eq!(primary.bucket, "products-us-west-2");
        assert_eq!(primary.region, "us-west-2");
        assert_eq!(primary.access_key, "AKIAIOSFODNN7EXAMPLE1");
        assert_eq!(
            primary.secret_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
        );
        assert_eq!(
            primary.endpoint,
            Some("https://s3.us-west-2.amazonaws.com".to_string())
        );
        assert_eq!(primary.priority, 1);
        assert_eq!(primary.timeout, 30);

        // Verify second replica (EU)
        let replica_eu = &replicas[1];
        assert_eq!(replica_eu.name, "replica-eu");
        assert_eq!(replica_eu.priority, 2);
        assert_eq!(replica_eu.timeout, 25);

        // Verify third replica (MinIO)
        let replica_minio = &replicas[2];
        assert_eq!(replica_minio.name, "replica-minio");
        assert_eq!(replica_minio.priority, 3);
        assert_eq!(replica_minio.timeout, 20);

        // Validation should pass
        config.validate().unwrap();
    }

    #[test]
    fn test_replicas_sorted_by_priority() {
        // Test: Replicas should be automatically sorted by priority (1, 2, 3...)
        // This ensures failover always tries replicas in the correct order
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "replica-minio"
          bucket: "products-backup"
          region: "us-east-1"
          access_key: "minioadmin"
          secret_key: "minioadmin"
          priority: 3
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
        - name: "replica-eu"
          bucket: "products-eu-west-1"
          region: "eu-west-1"
          access_key: "AKIAIOSFODNN7EXAMPLE2"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
          priority: 2
"#;

        // Parse config
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Get replicas
        let replicas = config.buckets[0].s3.replicas.as_ref().unwrap();

        // Verify replicas are sorted by priority (1, 2, 3)
        assert_eq!(replicas.len(), 3);
        assert_eq!(
            replicas[0].priority, 1,
            "First replica should have priority 1"
        );
        assert_eq!(
            replicas[0].name, "primary",
            "First replica should be 'primary'"
        );

        assert_eq!(
            replicas[1].priority, 2,
            "Second replica should have priority 2"
        );
        assert_eq!(
            replicas[1].name, "replica-eu",
            "Second replica should be 'replica-eu'"
        );

        assert_eq!(
            replicas[2].priority, 3,
            "Third replica should have priority 3"
        );
        assert_eq!(
            replicas[2].name, "replica-minio",
            "Third replica should be 'replica-minio'"
        );
    }

    #[test]
    fn test_replica_priority_must_be_unique_within_bucket() {
        // Test: Duplicate priorities within a bucket should be rejected
        // This ensures deterministic failover order - no ambiguity
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
        - name: "replica-eu"
          bucket: "products-eu-west-1"
          region: "eu-west-1"
          access_key: "AKIAIOSFODNN7EXAMPLE2"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
          priority: 1
"#;

        // Parse config should succeed
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Validation should fail due to duplicate priority
        let result = config.validate();
        assert!(
            result.is_err(),
            "Validation should fail with duplicate priorities"
        );

        let error = result.unwrap_err();
        let error_lower = error.to_lowercase();
        assert!(
            error_lower.contains("priority") && error_lower.contains("duplicate"),
            "Error should mention duplicate priority, got: {}",
            error
        );
    }

    #[test]
    fn test_replica_priority_must_be_at_least_one() {
        // Test: Priority 0 should be rejected - priorities start at 1
        // This ensures clean, human-friendly priority numbering (1, 2, 3...)
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 0
"#;

        // Parse config should succeed
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Validation should fail due to priority 0
        let result = config.validate();
        assert!(result.is_err(), "Validation should fail with priority 0");

        let error = result.unwrap_err();
        assert!(
            error.contains("priority") && (error.contains(">= 1") || error.contains("at least 1")),
            "Error should mention priority must be >= 1, got: {}",
            error
        );
    }

    #[test]
    fn test_replica_name_must_be_unique_within_bucket() {
        // Test: Duplicate replica names within a bucket should be rejected
        // This ensures clear replica identification in logs and metrics
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
        - name: "primary"
          bucket: "products-eu-west-1"
          region: "eu-west-1"
          access_key: "AKIAIOSFODNN7EXAMPLE2"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
          priority: 2
"#;

        // Parse config should succeed
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Validation should fail due to duplicate name
        let result = config.validate();
        assert!(
            result.is_err(),
            "Validation should fail with duplicate replica names"
        );

        let error = result.unwrap_err();
        let error_lower = error.to_lowercase();
        assert!(
            error_lower.contains("name") && error_lower.contains("duplicate"),
            "Error should mention duplicate name, got: {}",
            error
        );
    }

    #[test]
    fn test_at_least_one_replica_required() {
        // Test: Empty replicas array should be rejected
        // At least one replica is required for HA configuration
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas: []
"#;

        // Parse config should succeed
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Validation should fail due to empty replicas
        let result = config.validate();
        assert!(
            result.is_err(),
            "Validation should fail with empty replicas array"
        );

        let error = result.unwrap_err();
        let error_lower = error.to_lowercase();
        assert!(
            error_lower.contains("at least one") || error_lower.contains("empty"),
            "Error should mention at least one replica required, got: {}",
            error
        );
    }

    #[test]
    fn test_replica_required_fields_enforced() {
        // Test: Required fields (bucket, region, access_key, secret_key, priority, name)
        // must be present in each replica. This is enforced by serde during parsing.

        // Test 1: Missing bucket field
        let yaml_missing_bucket = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
"#;
        let result = Config::from_yaml_with_env(yaml_missing_bucket);
        assert!(
            result.is_err(),
            "Config with missing bucket field should fail to parse"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("bucket") || error.contains("missing field"),
            "Error should mention missing bucket field, got: {}",
            error
        );

        // Test 2: Missing priority field
        let yaml_missing_priority = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
"#;
        let result = Config::from_yaml_with_env(yaml_missing_priority);
        assert!(
            result.is_err(),
            "Config with missing priority field should fail to parse"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("priority") || error.contains("missing field"),
            "Error should mention missing priority field, got: {}",
            error
        );

        // Test 3: Missing access_key field
        let yaml_missing_access_key = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
"#;
        let result = Config::from_yaml_with_env(yaml_missing_access_key);
        assert!(
            result.is_err(),
            "Config with missing access_key field should fail to parse"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("access_key") || error.contains("missing field"),
            "Error should mention missing access_key field, got: {}",
            error
        );
    }

    #[test]
    fn test_invalid_replica_config_rejected() {
        // Test 1: Zero timeout should be rejected (timeout must be > 0)
        let yaml_zero_timeout = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
          timeout: 0
"#;
        let config =
            Config::from_yaml_with_env(yaml_zero_timeout).expect("Valid YAML should parse");

        let result = config.validate();
        assert!(result.is_err(), "Replica with timeout=0 should be rejected");
        let error = result.unwrap_err();
        assert!(
            error.contains("timeout") && error.contains("0"),
            "Error should mention timeout=0, got: {}",
            error
        );

        // Test 2: Empty replica name should be rejected
        let yaml_empty_name = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: ""
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
"#;
        let config = Config::from_yaml_with_env(yaml_empty_name).expect("Valid YAML should parse");

        let result = config.validate();
        assert!(
            result.is_err(),
            "Replica with empty name should be rejected"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("name") && error.contains("empty"),
            "Error should mention empty name, got: {}",
            error
        );

        // Test 3: Empty bucket name should be rejected
        let yaml_empty_bucket = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: ""
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
"#;
        let config =
            Config::from_yaml_with_env(yaml_empty_bucket).expect("Valid YAML should parse");

        let result = config.validate();
        assert!(
            result.is_err(),
            "Replica with empty bucket name should be rejected"
        );
        let error = result.unwrap_err();
        assert!(
            error.contains("bucket") && error.contains("empty"),
            "Error should mention empty bucket, got: {}",
            error
        );
    }

    #[test]
    fn test_replica_timeout_defaults_and_overrides() {
        // Test: timeout field is optional with default, and can be overridden per-replica
        // This allows flexibility: fast primary (lower timeout) vs. slow backup (higher timeout)
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "AKIAIOSFODNN7EXAMPLE1"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1"
          priority: 1
          # timeout not specified - should default to 20 seconds
        - name: "backup"
          bucket: "products-backup"
          region: "us-east-1"
          access_key: "AKIAIOSFODNN7EXAMPLE2"
          secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2"
          priority: 2
          timeout: 45  # custom timeout - should override default
"#;

        // Parse config - should succeed
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Get replicas
        let replicas = config.buckets[0].s3.replicas.as_ref().unwrap();
        assert_eq!(replicas.len(), 2);

        // Verify first replica uses default timeout (20s)
        let primary = &replicas[0];
        assert_eq!(primary.name, "primary");
        assert_eq!(
            primary.timeout, 20,
            "Replica without timeout field should default to 20 seconds"
        );

        // Verify second replica uses custom timeout (45s)
        let backup = &replicas[1];
        assert_eq!(backup.name, "backup");
        assert_eq!(
            backup.timeout, 45,
            "Replica with timeout field should use specified value"
        );

        // Validation should pass
        config.validate().unwrap();
    }

    #[test]
    fn test_single_bucket_config_converted_to_replica_format() {
        // Test: Legacy single-bucket config should be converted to single-replica format internally
        // This provides a unified code path: everything is handled as replicas internally
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
      endpoint: "https://s3.us-west-2.amazonaws.com"
      timeout: 30
"#;

        // Parse config
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // Verify the config parsed successfully
        assert_eq!(config.buckets.len(), 1);
        let s3_config = &config.buckets[0].s3;

        // Legacy format: replicas should be None initially
        assert!(
            s3_config.replicas.is_none(),
            "Legacy config should have replicas=None before normalization"
        );

        // After normalization, the config should be treated as a single replica
        // This is tested by calling a method that normalizes the config
        let normalized_config = config.normalize();

        // Now the replicas field should be populated with a single replica
        let normalized_s3 = &normalized_config.buckets[0].s3;
        assert!(
            normalized_s3.replicas.is_some(),
            "After normalization, legacy config should have replicas populated"
        );

        let replicas = normalized_s3.replicas.as_ref().unwrap();
        assert_eq!(replicas.len(), 1, "Should have exactly 1 replica");

        // Verify replica fields match legacy config
        let replica = &replicas[0];
        assert_eq!(
            replica.name, "default",
            "Converted replica should be named 'default'"
        );
        assert_eq!(replica.bucket, "my-products-bucket");
        assert_eq!(replica.region, "us-west-2");
        assert_eq!(replica.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            replica.secret_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
        assert_eq!(
            replica.endpoint,
            Some("https://s3.us-west-2.amazonaws.com".to_string())
        );
        assert_eq!(replica.timeout, 30);
        assert_eq!(
            replica.priority, 1,
            "Default replica should have priority 1"
        );

        // Validation should pass
        normalized_config.validate().unwrap();
    }
}
