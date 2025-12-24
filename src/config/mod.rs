//! Configuration module for Yatagarasu S3 proxy.
//!
//! This module provides YAML-based configuration loading with:
//! - Environment variable substitution (`${VAR_NAME}`)
//! - Comprehensive validation
//! - Hot reload support (via generation tracking)
//!
//! # Module Organization
//!
//! Configuration is split into focused submodules:
//! - [`audit`] - Audit logging (file, syslog, S3 export)
//! - [`authorization`] - OPA/OpenFGA integration
//! - [`bucket`] - Per-bucket S3 and routing config
//! - [`circuit_breaker`] - Backend resilience
//! - [`jwt`] - Token authentication
//! - [`rate_limit`] - Request throttling
//! - [`retry`] - Transient failure handling
//! - [`server`] - Server bindings and limits
//!
//! # Default Values
//!
//! Most default values are centralized in `crate::constants` to ensure
//! consistency and easy modification. Each submodule documents which
//! constants it uses.

pub mod audit;
pub mod authorization;
pub mod bucket;
pub mod circuit_breaker;
pub mod jwt;
pub mod rate_limit;
pub mod retry;
pub mod server;

// Re-export all types for backward compatibility
pub use audit::{
    AuditFileConfig, AuditLogConfig, AuditLogLevel, AuditOutput, AuditS3ExportConfig,
    AuditSyslogConfig, RotationPolicy, SyslogFacility, SyslogProtocol,
};
pub use authorization::AuthorizationConfig;
pub use bucket::{AuthConfig, BucketConfig, IpFilterConfig, S3Config, S3Replica};
pub use circuit_breaker::CircuitBreakerConfigYaml;
pub use jwt::{ClaimRule, JwtConfig, JwtKey, TokenSource};
pub use rate_limit::{
    BucketRateLimitConfigYaml, GlobalRateLimitConfigYaml, PerIpRateLimitConfigYaml,
    RateLimitConfigYaml,
};
pub use retry::RetryConfigYaml;
pub use server::{SecurityLimitsConfig, ServerConfig};

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::cache::CacheConfig;
use crate::image_optimizer::ImageConfig;
use crate::observability::ObservabilityConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub buckets: Vec<BucketConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt: Option<JwtConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheConfig>,
    #[serde(default)]
    pub image_optimization: ImageConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_log: Option<AuditLogConfig>,
    /// Observability configuration (tracing, request logging, slow queries)
    #[serde(default)]
    pub observability: ObservabilityConfig,
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

            // Validate S3 configuration (legacy vs replicas mutual exclusivity)
            bucket.s3.validate(&bucket.name)?;

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

            // Validate authorization configuration if present (Phase 32: OPA Integration)
            if let Some(auth_config) = &bucket.authorization {
                // Validate authorization type
                const VALID_AUTH_TYPES: &[&str] = &["opa"];
                if !VALID_AUTH_TYPES.contains(&auth_config.auth_type.as_str()) {
                    return Err(format!(
                        "Bucket '{}': Invalid authorization type '{}'. Supported types: {}",
                        bucket.name,
                        auth_config.auth_type,
                        VALID_AUTH_TYPES.join(", ")
                    ));
                }

                // Validate OPA-specific configuration when type is "opa"
                if auth_config.auth_type == "opa" {
                    // opa_url is required
                    if auth_config.opa_url.is_none() {
                        return Err(format!(
                            "Bucket '{}': opa_url is required when authorization type is 'opa'",
                            bucket.name
                        ));
                    }

                    // opa_policy_path is required
                    if auth_config.opa_policy_path.is_none() {
                        return Err(format!(
                            "Bucket '{}': opa_policy_path is required when authorization type is 'opa'",
                            bucket.name
                        ));
                    }

                    // Validate URL format
                    if let Some(url) = &auth_config.opa_url {
                        if !url.starts_with("http://") && !url.starts_with("https://") {
                            return Err(format!(
                                "Bucket '{}': opa_url '{}' must start with http:// or https://",
                                bucket.name, url
                            ));
                        }
                    }
                }
            }

            // Validate watermark configuration if present
            if let Some(watermark_config) = &bucket.watermark {
                watermark_config.validate(&bucket.name)?;
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

        // Validate cache configuration if present
        if let Some(cache) = &self.cache {
            cache.validate()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper function to create minimal valid config YAML
    fn minimal_config_yaml() -> &'static str {
        r#"
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
"#
    }

    #[test]
    fn test_config_can_be_loaded_from_file_path() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file
            .write_all(minimal_config_yaml().as_bytes())
            .unwrap();
        temp_file.flush().unwrap();

        let config = Config::from_file(temp_file.path()).unwrap();

        assert_eq!(config.server.address, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.buckets.len(), 1);
        assert_eq!(config.buckets[0].name, "test-bucket");
        assert_eq!(config.buckets[0].path_prefix, "/test");
        assert_eq!(config.generation, 0);
    }

    #[test]
    fn test_config_validation_catches_duplicate_path_prefix() {
        let yaml = r#"
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
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate path_prefix"));
    }

    #[test]
    fn test_config_validation_catches_empty_bucket_name() {
        let yaml = r#"
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
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_config_has_generation_number() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        assert_eq!(config.generation, 0);
    }

    #[test]
    fn test_config_env_var_substitution() {
        std::env::set_var("TEST_CONFIG_VAR", "substituted-value");

        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "test"
    path_prefix: "/test"
    s3:
      bucket: ${TEST_CONFIG_VAR}
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        assert_eq!(config.buckets[0].s3.bucket, "substituted-value");

        std::env::remove_var("TEST_CONFIG_VAR");
    }

    #[test]
    fn test_config_env_var_missing_returns_error() {
        std::env::remove_var("MISSING_VAR_FOR_TEST");

        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "test"
    path_prefix: "/test"
    s3:
      bucket: ${MISSING_VAR_FOR_TEST}
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
"#;
        let result = Config::from_yaml_with_env(yaml);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("MISSING_VAR_FOR_TEST"));
    }

    #[test]
    fn test_config_normalize_legacy_to_replica() {
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
      access_key: "AKIAEXAMPLE"
      secret_key: "secretkey"
      timeout: 30
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let normalized = config.normalize();

        let replicas = normalized.buckets[0].s3.replicas.as_ref().unwrap();
        assert_eq!(replicas.len(), 1);
        assert_eq!(replicas[0].name, "default");
        assert_eq!(replicas[0].bucket, "my-products-bucket");
        assert_eq!(replicas[0].region, "us-west-2");
        assert_eq!(replicas[0].priority, 1);
        assert_eq!(replicas[0].timeout, 30);
    }

    #[test]
    fn test_config_replicas_sorted_by_priority() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "replica-3"
          bucket: "bucket-3"
          region: "us-east-1"
          access_key: "key3"
          secret_key: "secret3"
          priority: 3
        - name: "replica-1"
          bucket: "bucket-1"
          region: "us-west-2"
          access_key: "key1"
          secret_key: "secret1"
          priority: 1
        - name: "replica-2"
          bucket: "bucket-2"
          region: "eu-west-1"
          access_key: "key2"
          secret_key: "secret2"
          priority: 2
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        let replicas = config.buckets[0].s3.replicas.as_ref().unwrap();
        assert_eq!(replicas[0].priority, 1);
        assert_eq!(replicas[0].name, "replica-1");
        assert_eq!(replicas[1].priority, 2);
        assert_eq!(replicas[2].priority, 3);
    }

    #[test]
    fn test_config_validation_replica_duplicate_priority() {
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
          bucket: "bucket-1"
          region: "us-west-2"
          access_key: "key1"
          secret_key: "secret1"
          priority: 1
        - name: "secondary"
          bucket: "bucket-2"
          region: "eu-west-1"
          access_key: "key2"
          secret_key: "secret2"
          priority: 1
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate priority"));
    }

    #[test]
    fn test_config_validation_replica_empty_name() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: ""
          bucket: "bucket-1"
          region: "us-west-2"
          access_key: "key1"
          secret_key: "secret1"
          priority: 1
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Replica name cannot be empty"));
    }

    #[test]
    fn test_config_validation_opa_requires_url() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "protected"
    path_prefix: "/protected"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
    authorization:
      type: opa
      opa_policy_path: "yatagarasu/authz/allow"
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("opa_url"));
    }

    #[test]
    fn test_config_validation_opa_url_format() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: "protected"
    path_prefix: "/protected"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      access_key: "test"
      secret_key: "test"
    authorization:
      type: opa
      opa_url: "invalid-url"
      opa_policy_path: "yatagarasu/authz/allow"
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("http://"));
    }

    #[test]
    fn test_config_with_observability() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
observability:
  tracing:
    enabled: true
    exporter: otlp
    service_name: my-proxy
  request_logging:
    log_requests: true
  slow_query:
    enabled: true
    threshold_ms: 500
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        assert!(config.observability.tracing.enabled);
        assert_eq!(config.observability.tracing.exporter, "otlp");
        assert!(config.observability.request_logging.log_requests);
        assert!(config.observability.slow_query.enabled);
        assert_eq!(config.observability.slow_query.threshold_ms, 500);
    }

    #[test]
    fn test_config_observability_defaults() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        assert!(!config.observability.tracing.enabled);
        assert!(!config.observability.request_logging.log_requests);
        assert!(!config.observability.slow_query.enabled);
    }

    #[test]
    fn test_config_with_cache() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
cache:
  layers:
    - type: memory
      max_size_mb: 512
      max_item_size_mb: 5
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        assert!(config.cache.is_some());
    }

    #[test]
    fn test_config_with_audit_log() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
audit_log:
  enabled: true
  outputs:
    - file
  file:
    path: /var/log/audit.log
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        let audit = config.audit_log.unwrap();
        assert!(audit.enabled);
        assert!(audit.outputs.contains(&AuditOutput::File));
    }

    #[test]
    fn test_config_with_jwt() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  secret: "my-secret-key"
  token_sources:
    - type: bearer
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        config.validate().unwrap();

        let jwt = config.jwt.unwrap();
        assert!(jwt.enabled);
        assert_eq!(jwt.algorithm, "HS256");
    }

    #[test]
    fn test_config_validation_jwt_invalid_algorithm() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "INVALID"
  secret: "my-secret"
  token_sources:
    - type: bearer
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JWT algorithm"));
    }

    #[test]
    fn test_config_validation_jwt_empty_secret() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  secret: ""
  token_sources:
    - type: bearer
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JWT secret cannot be empty"));
    }

    #[test]
    fn test_config_validation_jwt_no_token_sources() {
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
jwt:
  enabled: true
  algorithm: "HS256"
  secret: "my-secret"
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("token source"));
    }
}
