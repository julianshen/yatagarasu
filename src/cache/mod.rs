// Cache module

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_empty_cache_config() {
        // Test: Can create empty CacheConfig struct
        let _config = CacheConfig::default();
        // If this compiles, the test passes
    }

    #[test]
    fn test_can_deserialize_minimal_cache_config_from_yaml() {
        // Test: Can deserialize minimal cache config from YAML
        let yaml = r#"
enabled: false
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        // If this deserializes without error, the test passes
        assert_eq!(config.enabled, false);
    }

    #[test]
    fn test_cache_config_has_enabled_field() {
        // Test: CacheConfig has enabled field (bool)
        let config = CacheConfig { enabled: true };
        assert_eq!(config.enabled, true);

        let config = CacheConfig { enabled: false };
        assert_eq!(config.enabled, false);
    }

    #[test]
    fn test_cache_config_defaults_to_disabled() {
        // Test: CacheConfig defaults to disabled when not specified
        let config = CacheConfig::default();
        assert_eq!(config.enabled, false);

        // Also test with empty YAML
        let yaml = r#"{}"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.enabled, false);
    }

    #[test]
    fn test_can_parse_cache_config_with_enabled_true() {
        // Test: Can parse cache config with enabled=true
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_can_parse_cache_config_with_enabled_false() {
        // Test: Can parse cache config with enabled=false
        let yaml = r#"
enabled: false
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.enabled, false);
    }
}
