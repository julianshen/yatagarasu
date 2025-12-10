//! Circuit breaker configuration for S3 backend resilience.
//!
//! This module defines the YAML configuration format for circuit breakers,
//! which protect against cascading failures when S3 backends become unavailable.
//!
//! Default values for thresholds and timeouts are sourced from `crate::constants`.

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_CB_TIMEOUT_SECS, DEFAULT_FAILURE_THRESHOLD, DEFAULT_HALF_OPEN_MAX_REQUESTS,
    DEFAULT_SUCCESS_THRESHOLD,
};

fn default_failure_threshold() -> u32 {
    DEFAULT_FAILURE_THRESHOLD
}

fn default_success_threshold() -> u32 {
    DEFAULT_SUCCESS_THRESHOLD
}

fn default_timeout_seconds() -> u64 {
    DEFAULT_CB_TIMEOUT_SECS
}

fn default_half_open_max_requests() -> u32 {
    DEFAULT_HALF_OPEN_MAX_REQUESTS
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_config_defaults() {
        let yaml = "{}";
        let config: CircuitBreakerConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.failure_threshold, DEFAULT_FAILURE_THRESHOLD);
        assert_eq!(config.success_threshold, DEFAULT_SUCCESS_THRESHOLD);
        assert_eq!(config.timeout_seconds, DEFAULT_CB_TIMEOUT_SECS);
        assert_eq!(
            config.half_open_max_requests,
            DEFAULT_HALF_OPEN_MAX_REQUESTS
        );
    }

    #[test]
    fn test_circuit_breaker_config_custom_values() {
        let yaml = r#"
failure_threshold: 3
success_threshold: 1
timeout_seconds: 30
half_open_max_requests: 5
"#;
        let config: CircuitBreakerConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.half_open_max_requests, 5);
    }

    #[test]
    fn test_circuit_breaker_config_partial_values() {
        let yaml = r#"
failure_threshold: 10
timeout_seconds: 120
"#;
        let config: CircuitBreakerConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.success_threshold, DEFAULT_SUCCESS_THRESHOLD);
        assert_eq!(config.timeout_seconds, 120);
        assert_eq!(
            config.half_open_max_requests,
            DEFAULT_HALF_OPEN_MAX_REQUESTS
        );
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
}
