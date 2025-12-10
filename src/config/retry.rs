//! Retry configuration for S3 request resilience.
//!
//! This module defines the YAML configuration format for retry policies,
//! which handle transient S3 failures with exponential backoff.
//!
//! Default values for max attempts and backoff delays are sourced from `crate::constants`.

use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_INITIAL_BACKOFF_MS, DEFAULT_MAX_ATTEMPTS, DEFAULT_MAX_BACKOFF_MS};

fn default_max_attempts() -> u32 {
    DEFAULT_MAX_ATTEMPTS
}

fn default_initial_backoff_ms() -> u64 {
    DEFAULT_INITIAL_BACKOFF_MS
}

fn default_max_backoff_ms() -> u64 {
    DEFAULT_MAX_BACKOFF_MS
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

    #[test]
    fn test_retry_config_defaults() {
        let yaml = "{}";
        let config: RetryConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(config.initial_backoff_ms, DEFAULT_INITIAL_BACKOFF_MS);
        assert_eq!(config.max_backoff_ms, DEFAULT_MAX_BACKOFF_MS);
    }

    #[test]
    fn test_retry_config_custom_values() {
        let yaml = r#"
max_attempts: 5
initial_backoff_ms: 200
max_backoff_ms: 10000
"#;
        let config: RetryConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_backoff_ms, 200);
        assert_eq!(config.max_backoff_ms, 10000);
    }

    #[test]
    fn test_retry_config_partial_values() {
        let yaml = r#"
max_attempts: 10
"#;
        let config: RetryConfigYaml = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_backoff_ms, DEFAULT_INITIAL_BACKOFF_MS);
        assert_eq!(config.max_backoff_ms, DEFAULT_MAX_BACKOFF_MS);
    }

    #[test]
    fn test_retry_config_conversion() {
        let yaml_config = RetryConfigYaml {
            max_attempts: 5,
            initial_backoff_ms: 200,
            max_backoff_ms: 5000,
        };

        let policy = yaml_config.to_retry_policy();

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_backoff_ms, 200);
        assert_eq!(policy.max_backoff_ms, 5000);
    }
}
