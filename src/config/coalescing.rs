//! Coalescing configuration types.
//!
//! This module defines the request coalescing configuration:
//! - Enable/disable coalescing
//! - Strategy selection (wait_for_complete vs streaming)
//!
//! Default: enabled with wait_for_complete strategy for backward compatibility.

use serde::{Deserialize, Serialize};

/// Default enabled state
fn default_enabled() -> bool {
    true
}

/// Coalescing strategy selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoalescingStrategy {
    /// Followers wait for leader to fully complete download before proceeding.
    /// Simple and works well for small files.
    #[default]
    WaitForComplete,
    /// Followers receive bytes in real-time as leader downloads.
    /// Better for large files - no head-of-line blocking.
    Streaming,
}

/// Request coalescing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoalescingConfig {
    /// Enable request coalescing (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Coalescing strategy (default: wait_for_complete)
    #[serde(default)]
    pub strategy: CoalescingStrategy,
}

impl Default for CoalescingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            strategy: CoalescingStrategy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coalescing_config_default() {
        let config = CoalescingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.strategy, CoalescingStrategy::WaitForComplete);
    }

    #[test]
    fn test_coalescing_config_deserialize_defaults() {
        let yaml = "{}";
        let config: CoalescingConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.strategy, CoalescingStrategy::WaitForComplete);
    }

    #[test]
    fn test_coalescing_config_deserialize_streaming() {
        let yaml = r#"
enabled: true
strategy: streaming
"#;
        let config: CoalescingConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
        assert_eq!(config.strategy, CoalescingStrategy::Streaming);
    }

    #[test]
    fn test_coalescing_config_deserialize_disabled() {
        let yaml = r#"
enabled: false
"#;
        let config: CoalescingConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn test_coalescing_strategy_serialization() {
        // Test wait_for_complete serialization
        let config = CoalescingConfig {
            enabled: true,
            strategy: CoalescingStrategy::WaitForComplete,
        };
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("wait_for_complete"));

        // Test streaming serialization
        let config = CoalescingConfig {
            enabled: true,
            strategy: CoalescingStrategy::Streaming,
        };
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("streaming"));
    }
}
