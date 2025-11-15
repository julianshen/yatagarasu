// Cache module

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub memory: MemoryCacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    #[serde(default = "default_max_item_size_mb")]
    pub max_item_size_mb: u64,
    #[serde(default = "default_max_cache_size_mb")]
    pub max_cache_size_mb: u64,
    #[serde(default = "default_ttl_seconds")]
    pub default_ttl_seconds: u64,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            max_item_size_mb: default_max_item_size_mb(),
            max_cache_size_mb: default_max_cache_size_mb(),
            default_ttl_seconds: default_ttl_seconds(),
        }
    }
}

fn default_max_item_size_mb() -> u64 {
    10 // 10MB
}

fn default_max_cache_size_mb() -> u64 {
    1024 // 1GB
}

fn default_ttl_seconds() -> u64 {
    3600 // 1 hour
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
        let config = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
        };
        assert_eq!(config.enabled, true);

        let config = CacheConfig {
            enabled: false,
            memory: MemoryCacheConfig::default(),
        };
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

    // Memory Cache Configuration tests
    #[test]
    fn test_can_parse_memory_cache_section() {
        // Test: Can parse memory cache section
        let yaml = r#"
enabled: true
memory:
  max_item_size_mb: 10
  max_cache_size_mb: 1024
  default_ttl_seconds: 3600
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_item_size_mb, 10);
        assert_eq!(config.memory.max_cache_size_mb, 1024);
        assert_eq!(config.memory.default_ttl_seconds, 3600);
    }

    #[test]
    fn test_can_parse_max_item_size_mb_default_10mb() {
        // Test: Can parse max_item_size_mb (default 10MB)
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_item_size_mb, 10);

        // Test explicit value
        let yaml = r#"
enabled: true
memory:
  max_item_size_mb: 20
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_item_size_mb, 20);
    }

    #[test]
    fn test_can_parse_max_cache_size_mb_default_1gb() {
        // Test: Can parse max_cache_size_mb (default 1024MB = 1GB)
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_cache_size_mb, 1024);

        // Test explicit value
        let yaml = r#"
enabled: true
memory:
  max_cache_size_mb: 2048
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_cache_size_mb, 2048);
    }

    #[test]
    fn test_can_parse_default_ttl_seconds() {
        // Test: Can parse default_ttl_seconds (default 3600 = 1 hour)
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.default_ttl_seconds, 3600);

        // Test explicit value
        let yaml = r#"
enabled: true
memory:
  default_ttl_seconds: 7200
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.default_ttl_seconds, 7200);
    }
}
