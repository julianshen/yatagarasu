//! Bucket and S3 configuration types.
//!
//! This module defines per-bucket configuration including:
//! - Bucket routing (name, path prefix)
//! - S3 backend settings (credentials, endpoint, timeouts)
//! - HA replica configuration for multi-region failover
//! - Per-bucket auth, cache, authorization, and IP filtering
//!
//! Default values for timeouts and pool sizes are sourced from `crate::constants`.
//!
//! # Legacy vs Replica Configuration
//!
//! [`S3Config`] supports two mutually exclusive modes:
//!
//! - **Legacy mode**: Uses top-level `bucket`, `region`, `access_key`, `secret_key` fields
//!   for simple single-backend configuration.
//! - **Replica mode**: Uses `replicas` array for HA multi-region deployments with
//!   automatic failover.
//!
//! These modes cannot be mixed. Use [`S3Config::validate()`] to check for errors
//! before using the configuration.

use serde::{Deserialize, Serialize};

use crate::cache::BucketCacheOverride;
use crate::constants::{DEFAULT_CONNECTION_POOL_SIZE, DEFAULT_S3_TIMEOUT_SECS};

// Re-export IpFilterConfig from security module.
// This allows tests and external code to access it via `config::IpFilterConfig`
// while the canonical definition remains in the security module.
pub use crate::security::IpFilterConfig;

use super::authorization::AuthorizationConfig;
use super::circuit_breaker::CircuitBreakerConfigYaml;
use super::rate_limit::BucketRateLimitConfigYaml;
use super::retry::RetryConfigYaml;

fn default_s3_timeout() -> u64 {
    DEFAULT_S3_TIMEOUT_SECS
}

fn default_connection_pool_size() -> usize {
    DEFAULT_CONNECTION_POOL_SIZE
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3Config,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<BucketCacheOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<AuthorizationConfig>,
    /// IP filtering configuration (allowlist/blocklist with CIDR support)
    #[serde(default)]
    pub ip_filter: IpFilterConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

impl S3Config {
    /// Returns true if any legacy S3 fields are populated (non-empty).
    ///
    /// Legacy fields are: `bucket`, `region`, `access_key`, `secret_key`.
    /// These are mutually exclusive with the `replicas` configuration.
    pub fn has_legacy_config(&self) -> bool {
        !self.bucket.is_empty()
            || !self.region.is_empty()
            || !self.access_key.is_empty()
            || !self.secret_key.is_empty()
    }

    /// Validates S3 configuration, ensuring legacy fields and replicas are not both set.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Both legacy fields and `replicas` are configured (mutually exclusive)
    /// - Neither legacy fields nor `replicas` are configured (at least one required)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let s3_config: S3Config = serde_yaml::from_str(yaml)?;
    /// s3_config.validate("my-bucket")?;
    /// ```
    pub fn validate(&self, bucket_name: &str) -> Result<(), String> {
        let has_legacy = self.has_legacy_config();
        let has_replicas = self.replicas.is_some();

        if has_legacy && has_replicas {
            return Err(format!(
                "Bucket '{}': Cannot use both legacy S3 fields (bucket, region, access_key, secret_key) \
                and 'replicas' configuration. Choose one approach:\n  \
                - Legacy: Set bucket, region, access_key, secret_key directly\n  \
                - Replicas: Use 'replicas' array for HA multi-region setup",
                bucket_name
            ));
        }

        if !has_legacy && !has_replicas {
            return Err(format!(
                "Bucket '{}': S3 configuration is empty. Either set legacy fields \
                (bucket, region, access_key, secret_key) or configure 'replicas' array.",
                bucket_name
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_deserialize() {
        let yaml = r#"
enabled: true
"#;
        let config: AuthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);

        let yaml = r#"
enabled: false
"#;
        let config: AuthConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn test_s3_config_defaults() {
        let yaml = "{}";
        let config: S3Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.bucket, "");
        assert_eq!(config.region, "");
        assert_eq!(config.timeout, DEFAULT_S3_TIMEOUT_SECS);
        assert_eq!(config.connection_pool_size, DEFAULT_CONNECTION_POOL_SIZE);
        assert!(config.endpoint.is_none());
        assert!(config.circuit_breaker.is_none());
        assert!(config.rate_limit.is_none());
        assert!(config.retry.is_none());
        assert!(config.replicas.is_none());
    }

    #[test]
    fn test_s3_config_legacy_format() {
        let yaml = r#"
bucket: "my-bucket"
region: "us-west-2"
access_key: "AKIAIOSFODNN7EXAMPLE"
secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
endpoint: "https://s3.us-west-2.amazonaws.com"
timeout: 30
"#;
        let config: S3Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, "us-west-2");
        assert_eq!(config.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(config.timeout, 30);
        assert!(config.replicas.is_none());
    }

    #[test]
    fn test_s3_config_with_circuit_breaker() {
        let yaml = r#"
bucket: "my-bucket"
region: "us-west-2"
access_key: "test"
secret_key: "test"
circuit_breaker:
  failure_threshold: 5
  timeout_seconds: 60
"#;
        let config: S3Config = serde_yaml::from_str(yaml).unwrap();

        let cb = config.circuit_breaker.unwrap();
        assert_eq!(cb.failure_threshold, 5);
        assert_eq!(cb.timeout_seconds, 60);
    }

    #[test]
    fn test_s3_config_with_rate_limit() {
        let yaml = r#"
bucket: "my-bucket"
region: "us-west-2"
access_key: "test"
secret_key: "test"
rate_limit:
  requests_per_second: 100
"#;
        let config: S3Config = serde_yaml::from_str(yaml).unwrap();

        let rl = config.rate_limit.unwrap();
        assert_eq!(rl.requests_per_second, 100);
    }

    #[test]
    fn test_s3_config_with_retry() {
        let yaml = r#"
bucket: "my-bucket"
region: "us-west-2"
access_key: "test"
secret_key: "test"
retry:
  max_attempts: 5
  initial_backoff_ms: 200
"#;
        let config: S3Config = serde_yaml::from_str(yaml).unwrap();

        let retry = config.retry.unwrap();
        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.initial_backoff_ms, 200);
    }

    #[test]
    fn test_s3_replica_deserialize() {
        let yaml = r#"
name: "primary"
bucket: "products-us-west-2"
region: "us-west-2"
access_key: "AKIAIOSFODNN7EXAMPLE"
secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
endpoint: "https://s3.us-west-2.amazonaws.com"
priority: 1
timeout: 30
"#;
        let replica: S3Replica = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(replica.name, "primary");
        assert_eq!(replica.bucket, "products-us-west-2");
        assert_eq!(replica.region, "us-west-2");
        assert_eq!(replica.priority, 1);
        assert_eq!(replica.timeout, 30);
    }

    #[test]
    fn test_s3_replica_timeout_default() {
        let yaml = r#"
name: "primary"
bucket: "products-us-west-2"
region: "us-west-2"
access_key: "test"
secret_key: "test"
priority: 1
"#;
        let replica: S3Replica = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(replica.timeout, DEFAULT_S3_TIMEOUT_SECS);
    }

    #[test]
    fn test_s3_config_with_replicas() {
        let yaml = r#"
replicas:
  - name: "primary"
    bucket: "products-us-west-2"
    region: "us-west-2"
    access_key: "key1"
    secret_key: "secret1"
    priority: 1
  - name: "replica-eu"
    bucket: "products-eu-west-1"
    region: "eu-west-1"
    access_key: "key2"
    secret_key: "secret2"
    priority: 2
"#;
        let config: S3Config = serde_yaml::from_str(yaml).unwrap();

        let replicas = config.replicas.unwrap();
        assert_eq!(replicas.len(), 2);
        assert_eq!(replicas[0].name, "primary");
        assert_eq!(replicas[0].priority, 1);
        assert_eq!(replicas[1].name, "replica-eu");
        assert_eq!(replicas[1].priority, 2);
    }

    #[test]
    fn test_bucket_config_minimal() {
        let yaml = r#"
name: "test-bucket"
path_prefix: "/test"
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  access_key: "test-key"
  secret_key: "test-secret"
"#;
        let config: BucketConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.name, "test-bucket");
        assert_eq!(config.path_prefix, "/test");
        assert_eq!(config.s3.bucket, "my-bucket");
        assert!(config.auth.is_none());
        assert!(config.cache.is_none());
        assert!(config.authorization.is_none());
        assert!(config.ip_filter.allowlist.is_empty());
        assert!(config.ip_filter.blocklist.is_empty());
    }

    #[test]
    fn test_bucket_config_with_auth() {
        let yaml = r#"
name: "protected-bucket"
path_prefix: "/protected"
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  access_key: "test"
  secret_key: "test"
auth:
  enabled: true
"#;
        let config: BucketConfig = serde_yaml::from_str(yaml).unwrap();

        let auth = config.auth.unwrap();
        assert!(auth.enabled);
    }

    #[test]
    fn test_bucket_config_with_authorization() {
        let yaml = r#"
name: "protected-bucket"
path_prefix: "/protected"
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  access_key: "test"
  secret_key: "test"
authorization:
  type: opa
  opa_url: "http://localhost:8181"
  opa_policy_path: "yatagarasu/authz/allow"
"#;
        let config: BucketConfig = serde_yaml::from_str(yaml).unwrap();

        let authz = config.authorization.unwrap();
        assert_eq!(authz.auth_type, "opa");
    }

    #[test]
    fn test_bucket_config_with_ip_filter() {
        let yaml = r#"
name: "internal-bucket"
path_prefix: "/internal"
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  access_key: "test"
  secret_key: "test"
ip_filter:
  allowlist:
    - "10.0.0.0/8"
    - "192.168.1.0/24"
  blocklist:
    - "10.0.0.50"
"#;
        let config: BucketConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.ip_filter.allowlist.len(), 2);
        assert_eq!(config.ip_filter.blocklist.len(), 1);
        assert!(config
            .ip_filter
            .allowlist
            .contains(&"10.0.0.0/8".to_string()));
    }

    #[test]
    fn test_bucket_config_with_cache_override() {
        let yaml = r#"
name: "cached-bucket"
path_prefix: "/cached"
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  access_key: "test"
  secret_key: "test"
cache:
  enabled: true
  ttl_seconds: 7200
"#;
        let config: BucketConfig = serde_yaml::from_str(yaml).unwrap();

        let cache = config.cache.unwrap();
        assert!(cache.enabled.unwrap());
        assert_eq!(cache.ttl_seconds, Some(7200));
    }

    #[test]
    fn test_bucket_config_full() {
        let yaml = r#"
name: "full-bucket"
path_prefix: "/full"
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  access_key: "test"
  secret_key: "test"
  timeout: 45
  connection_pool_size: 100
  circuit_breaker:
    failure_threshold: 3
  rate_limit:
    requests_per_second: 500
  retry:
    max_attempts: 3
auth:
  enabled: true
cache:
  enabled: true
authorization:
  type: opa
  opa_url: "http://localhost:8181"
  opa_policy_path: "test/allow"
ip_filter:
  allowlist:
    - "10.0.0.0/8"
"#;
        let config: BucketConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.s3.timeout, 45);
        assert_eq!(config.s3.connection_pool_size, 100);
        assert!(config.s3.circuit_breaker.is_some());
        assert!(config.s3.rate_limit.is_some());
        assert!(config.s3.retry.is_some());
        assert!(config.auth.is_some());
        assert!(config.cache.is_some());
        assert!(config.authorization.is_some());
        assert_eq!(config.ip_filter.allowlist.len(), 1);
    }

    // S3Config::has_legacy_config() tests
    #[test]
    fn test_s3_config_has_legacy_config_with_bucket() {
        let config = S3Config {
            bucket: "test".to_string(),
            ..Default::default()
        };
        assert!(config.has_legacy_config());
    }

    #[test]
    fn test_s3_config_has_legacy_config_with_region() {
        let config = S3Config {
            region: "us-west-2".to_string(),
            ..Default::default()
        };
        assert!(config.has_legacy_config());
    }

    #[test]
    fn test_s3_config_has_legacy_config_with_access_key() {
        let config = S3Config {
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            ..Default::default()
        };
        assert!(config.has_legacy_config());
    }

    #[test]
    fn test_s3_config_has_legacy_config_with_secret_key() {
        let config = S3Config {
            secret_key: "wJalrXUtnFEMI".to_string(),
            ..Default::default()
        };
        assert!(config.has_legacy_config());
    }

    #[test]
    fn test_s3_config_has_legacy_config_empty() {
        let config = S3Config::default();
        assert!(!config.has_legacy_config());
    }

    // S3Config::validate() tests
    #[test]
    fn test_s3_config_validate_legacy_only_ok() {
        let config = S3Config {
            bucket: "my-bucket".to_string(),
            region: "us-west-2".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            ..Default::default()
        };
        assert!(config.validate("test-bucket").is_ok());
    }

    #[test]
    fn test_s3_config_validate_replicas_only_ok() {
        let config = S3Config {
            replicas: Some(vec![S3Replica {
                name: "primary".to_string(),
                bucket: "my-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "test".to_string(),
                secret_key: "test".to_string(),
                endpoint: None,
                priority: 1,
                timeout: 30,
            }]),
            ..Default::default()
        };
        assert!(config.validate("test-bucket").is_ok());
    }

    #[test]
    fn test_s3_config_validate_rejects_both_legacy_and_replicas() {
        let config = S3Config {
            bucket: "legacy-bucket".to_string(),
            region: "us-west-2".to_string(),
            access_key: "legacy-key".to_string(),
            secret_key: "legacy-secret".to_string(),
            replicas: Some(vec![S3Replica {
                name: "primary".to_string(),
                bucket: "replica-bucket".to_string(),
                region: "us-east-1".to_string(),
                access_key: "replica-key".to_string(),
                secret_key: "replica-secret".to_string(),
                endpoint: None,
                priority: 1,
                timeout: 30,
            }]),
            ..Default::default()
        };

        let result = config.validate("conflicting-bucket");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Cannot use both legacy S3 fields"));
        assert!(err.contains("conflicting-bucket"));
    }

    #[test]
    fn test_s3_config_validate_rejects_empty_config() {
        let config = S3Config::default();

        let result = config.validate("empty-bucket");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("S3 configuration is empty"));
        assert!(err.contains("empty-bucket"));
    }
}
