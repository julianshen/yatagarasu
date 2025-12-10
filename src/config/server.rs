//! Server configuration types.
//!
//! This module defines the server-level configuration including:
//! - Address and port bindings
//! - Request timeouts and concurrency limits
//! - Security validation limits (body size, header size, URI length)
//! - Global rate limiting settings
//!
//! Default values are sourced from `crate::constants`.

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_MAX_BODY_SIZE, DEFAULT_MAX_CONCURRENT_REQUESTS, DEFAULT_MAX_HEADER_SIZE,
    DEFAULT_MAX_URI_LENGTH, DEFAULT_REQUEST_TIMEOUT_SECS, DEFAULT_THREADS,
};

use super::rate_limit::RateLimitConfigYaml;

// Default timeout values
fn default_request_timeout() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_SECS
}

// Default connection pool values
fn default_max_concurrent_requests() -> usize {
    DEFAULT_MAX_CONCURRENT_REQUESTS
}

// Default worker thread count
fn default_threads() -> usize {
    DEFAULT_THREADS
}

// Default security limit values
fn default_max_body_size() -> usize {
    DEFAULT_MAX_BODY_SIZE
}

fn default_max_header_size() -> usize {
    DEFAULT_MAX_HEADER_SIZE
}

fn default_max_uri_length() -> usize {
    DEFAULT_MAX_URI_LENGTH
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
    /// Number of worker threads (default: 4)
    #[serde(default = "default_threads")]
    pub threads: usize,
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfigYaml>,
    #[serde(default)]
    pub security_limits: SecurityLimitsConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_limits_config_default() {
        let config = SecurityLimitsConfig::default();

        assert_eq!(config.max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert_eq!(config.max_header_size, DEFAULT_MAX_HEADER_SIZE);
        assert_eq!(config.max_uri_length, DEFAULT_MAX_URI_LENGTH);
    }

    #[test]
    fn test_security_limits_config_deserialize_defaults() {
        let yaml = "{}";
        let config: SecurityLimitsConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert_eq!(config.max_header_size, DEFAULT_MAX_HEADER_SIZE);
        assert_eq!(config.max_uri_length, DEFAULT_MAX_URI_LENGTH);
    }

    #[test]
    fn test_security_limits_config_deserialize_custom() {
        let yaml = r#"
max_body_size: 20971520
max_header_size: 131072
max_uri_length: 16384
"#;
        let config: SecurityLimitsConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.max_body_size, 20971520);
        assert_eq!(config.max_header_size, 131072);
        assert_eq!(config.max_uri_length, 16384);
    }

    #[test]
    fn test_server_config_deserialize_defaults() {
        let yaml = r#"
address: "127.0.0.1"
port: 8080
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.address, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.threads, DEFAULT_THREADS);
        assert_eq!(config.request_timeout, DEFAULT_REQUEST_TIMEOUT_SECS);
        assert_eq!(
            config.max_concurrent_requests,
            DEFAULT_MAX_CONCURRENT_REQUESTS
        );
        assert!(config.rate_limit.is_none());
    }

    #[test]
    fn test_server_config_deserialize_custom() {
        let yaml = r#"
address: "0.0.0.0"
port: 9090
threads: 8
request_timeout: 60
max_concurrent_requests: 5000
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.address, "0.0.0.0");
        assert_eq!(config.port, 9090);
        assert_eq!(config.threads, 8);
        assert_eq!(config.request_timeout, 60);
        assert_eq!(config.max_concurrent_requests, 5000);
    }

    #[test]
    fn test_server_config_threads_custom_value() {
        let yaml = r#"
address: "127.0.0.1"
port: 8080
threads: 16
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.threads, 16);
    }

    #[test]
    fn test_server_config_with_rate_limit() {
        let yaml = r#"
address: "127.0.0.1"
port: 8080
rate_limit:
  enabled: true
  global:
    requests_per_second: 1000
  per_ip:
    requests_per_second: 100
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();

        let rate_limit = config.rate_limit.unwrap();
        assert!(rate_limit.enabled);
        assert!(rate_limit.global.is_some());
        assert!(rate_limit.per_ip.is_some());
    }

    #[test]
    fn test_server_config_with_security_limits() {
        let yaml = r#"
address: "127.0.0.1"
port: 8080
security_limits:
  max_body_size: 52428800
  max_header_size: 65536
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.security_limits.max_body_size, 52428800);
        assert_eq!(config.security_limits.max_header_size, 65536);
        // max_uri_length should use default
        assert_eq!(
            config.security_limits.max_uri_length,
            DEFAULT_MAX_URI_LENGTH
        );
    }
}
