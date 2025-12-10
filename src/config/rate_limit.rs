//! Rate limiting configuration types.
//!
//! This module defines rate limiting configuration at multiple levels:
//! - Global rate limits (server-wide)
//! - Per-IP rate limits (client throttling)
//! - Per-bucket rate limits (S3 backend protection)

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_deserialize_minimal() {
        let yaml = r#"
enabled: false
"#;
        let config: RateLimitConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert!(!config.enabled);
        assert!(config.global.is_none());
        assert!(config.per_ip.is_none());
    }

    #[test]
    fn test_rate_limit_config_deserialize_enabled_default() {
        let yaml = "{}";
        let config: RateLimitConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert!(!config.enabled);
    }

    #[test]
    fn test_rate_limit_config_deserialize_with_global() {
        let yaml = r#"
enabled: true
global:
  requests_per_second: 1000
"#;
        let config: RateLimitConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        let global = config.global.unwrap();
        assert_eq!(global.requests_per_second, 1000);
        assert!(config.per_ip.is_none());
    }

    #[test]
    fn test_rate_limit_config_deserialize_with_per_ip() {
        let yaml = r#"
enabled: true
per_ip:
  requests_per_second: 100
"#;
        let config: RateLimitConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert!(config.global.is_none());
        let per_ip = config.per_ip.unwrap();
        assert_eq!(per_ip.requests_per_second, 100);
    }

    #[test]
    fn test_rate_limit_config_deserialize_full() {
        let yaml = r#"
enabled: true
global:
  requests_per_second: 5000
per_ip:
  requests_per_second: 500
"#;
        let config: RateLimitConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert_eq!(config.global.unwrap().requests_per_second, 5000);
        assert_eq!(config.per_ip.unwrap().requests_per_second, 500);
    }

    #[test]
    fn test_bucket_rate_limit_config_deserialize() {
        let yaml = r#"
requests_per_second: 200
"#;
        let config: BucketRateLimitConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.requests_per_second, 200);
    }
}
