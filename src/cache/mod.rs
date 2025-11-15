// Cache module

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub memory: MemoryCacheConfig,
    #[serde(default)]
    pub disk: DiskCacheConfig,
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

impl MemoryCacheConfig {
    /// Convert max_item_size_mb to bytes
    pub fn max_item_size_bytes(&self) -> u64 {
        self.max_item_size_mb * 1024 * 1024
    }

    /// Convert max_cache_size_mb to bytes
    pub fn max_cache_size_bytes(&self) -> u64 {
        self.max_cache_size_mb * 1024 * 1024
    }

    /// Validate memory cache configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_item_size_mb > self.max_cache_size_mb {
            return Err(format!(
                "max_item_size_mb ({}) cannot be greater than max_cache_size_mb ({})",
                self.max_item_size_mb, self.max_cache_size_mb
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskCacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    #[serde(default = "default_max_disk_cache_size_mb")]
    pub max_disk_cache_size_mb: u64,
}

impl Default for DiskCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cache_dir: default_cache_dir(),
            max_disk_cache_size_mb: default_max_disk_cache_size_mb(),
        }
    }
}

fn default_cache_dir() -> String {
    "/var/cache/yatagarasu".to_string()
}

fn default_max_disk_cache_size_mb() -> u64 {
    10240 // 10GB
}

impl DiskCacheConfig {
    /// Validate disk cache configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.cache_dir.is_empty() {
            return Err("cache_dir cannot be empty when disk cache is enabled".to_string());
        }
        Ok(())
    }
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
            disk: DiskCacheConfig::default(),
        };
        assert_eq!(config.enabled, true);

        let config = CacheConfig {
            enabled: false,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
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

    #[test]
    fn test_can_parse_max_item_size_in_bytes() {
        // Test: Can parse max_item_size in bytes (10MB = 10485760 bytes)
        let config = MemoryCacheConfig::default();
        assert_eq!(config.max_item_size_bytes(), 10 * 1024 * 1024);
        assert_eq!(config.max_item_size_bytes(), 10485760);

        // Test custom value
        let yaml = r#"
enabled: true
memory:
  max_item_size_mb: 20
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_item_size_bytes(), 20 * 1024 * 1024);
        assert_eq!(config.memory.max_item_size_bytes(), 20971520);
    }

    #[test]
    fn test_can_parse_max_cache_size_in_bytes() {
        // Test: Can parse max_cache_size in bytes (1GB = 1073741824 bytes)
        let config = MemoryCacheConfig::default();
        assert_eq!(config.max_cache_size_bytes(), 1024 * 1024 * 1024);
        assert_eq!(config.max_cache_size_bytes(), 1073741824);

        // Test custom value
        let yaml = r#"
enabled: true
memory:
  max_cache_size_mb: 2048
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_cache_size_bytes(), 2048 * 1024 * 1024);
        assert_eq!(config.memory.max_cache_size_bytes(), 2147483648);
    }

    #[test]
    fn test_rejects_max_item_size_greater_than_max_cache_size() {
        // Test: Rejects max_item_size > max_cache_size
        let config = MemoryCacheConfig {
            max_item_size_mb: 2048,
            max_cache_size_mb: 1024,
            default_ttl_seconds: 3600,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("cannot be greater than max_cache_size_mb"));

        // Valid config should pass
        let config = MemoryCacheConfig::default();
        assert!(config.validate().is_ok());

        // Equal sizes should be valid
        let config = MemoryCacheConfig {
            max_item_size_mb: 1024,
            max_cache_size_mb: 1024,
            default_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());
    }

    // Disk Cache Configuration tests
    #[test]
    fn test_can_parse_disk_cache_section() {
        // Test: Can parse disk cache section (optional)
        let yaml = r#"
enabled: true
disk:
  enabled: true
  cache_dir: /tmp/cache
  max_disk_cache_size_mb: 5120
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.disk.enabled, true);
        assert_eq!(config.disk.cache_dir, "/tmp/cache");
        assert_eq!(config.disk.max_disk_cache_size_mb, 5120);
    }

    #[test]
    fn test_can_parse_cache_dir_default() {
        // Test: Can parse cache_dir path (default: /var/cache/yatagarasu)
        let config = DiskCacheConfig::default();
        assert_eq!(config.cache_dir, "/var/cache/yatagarasu");

        // Test explicit value
        let yaml = r#"
enabled: true
disk:
  cache_dir: /custom/path
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.disk.cache_dir, "/custom/path");
    }

    #[test]
    fn test_can_parse_max_disk_cache_size_mb_default_10gb() {
        // Test: Can parse max_disk_cache_size_mb (default 10GB)
        let config = DiskCacheConfig::default();
        assert_eq!(config.max_disk_cache_size_mb, 10240);

        // Test explicit value
        let yaml = r#"
enabled: true
disk:
  max_disk_cache_size_mb: 20480
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.disk.max_disk_cache_size_mb, 20480);
    }

    #[test]
    fn test_disk_cache_enabled_defaults_to_false() {
        // Test: Can parse disk_cache_enabled (default false)
        let config = DiskCacheConfig::default();
        assert_eq!(config.enabled, false);

        // Test explicit enabled
        let yaml = r#"
enabled: true
disk:
  enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.disk.enabled, true);
    }

    #[test]
    fn test_rejects_disk_cache_with_empty_cache_dir() {
        // Test: Rejects disk cache with empty cache_dir
        let config = DiskCacheConfig {
            enabled: true,
            cache_dir: String::new(),
            max_disk_cache_size_mb: 10240,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cache_dir cannot be empty"));

        // Valid config should pass
        let config = DiskCacheConfig::default();
        assert!(config.validate().is_ok());

        // Disabled cache with empty dir should pass (not checked when disabled)
        let config = DiskCacheConfig {
            enabled: false,
            cache_dir: String::new(),
            max_disk_cache_size_mb: 10240,
        };
        assert!(config.validate().is_ok());
    }
}
