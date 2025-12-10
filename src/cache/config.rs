//! Cache configuration types
//!
//! This module defines configuration structures for all cache backends:
//! - Memory cache configuration
//! - Disk cache configuration
//! - Redis cache configuration
//! - Per-bucket cache overrides

use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_MAX_CACHE_SIZE_MB, DEFAULT_MAX_ITEM_SIZE_MB, DEFAULT_TTL_SECONDS};

use super::warming::PrewarmConfig;

/// Main cache configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub memory: MemoryCacheConfig,
    #[serde(default)]
    pub disk: DiskCacheConfig,
    #[serde(default)]
    pub redis: RedisCacheConfig,
    #[serde(default)]
    pub warming: Option<PrewarmConfig>,
    #[serde(default = "default_cache_layers")]
    pub cache_layers: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: default_cache_layers(),
        }
    }
}

fn default_cache_layers() -> Vec<String> {
    vec!["memory".to_string()]
}

impl CacheConfig {
    /// Validate cache configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate individual layer configs
        self.memory.validate()?;
        self.disk.validate()?;
        self.redis.validate()?;

        // Validate cache_layers
        if self.enabled && self.cache_layers.is_empty() {
            return Err("cache_layers cannot be empty when caching is enabled".to_string());
        }

        // Check for unknown layer names
        for layer in &self.cache_layers {
            if !matches!(layer.as_str(), "memory" | "disk" | "redis") {
                return Err(format!("Unknown cache layer: '{}'", layer));
            }
        }

        // Check for duplicate layers
        let mut seen = std::collections::HashSet::new();
        for layer in &self.cache_layers {
            if !seen.insert(layer) {
                return Err(format!("Duplicate cache layer: '{}'", layer));
            }
        }

        // Validate layer dependencies
        for layer in &self.cache_layers {
            match layer.as_str() {
                "disk" if !self.disk.enabled => {
                    return Err(
                        "disk layer requires disk.enabled=true in configuration".to_string()
                    );
                }
                "redis" if !self.redis.enabled => {
                    return Err(
                        "redis layer requires redis.enabled=true in configuration".to_string()
                    );
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// Memory cache configuration
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
    DEFAULT_MAX_ITEM_SIZE_MB
}

fn default_max_cache_size_mb() -> u64 {
    DEFAULT_MAX_CACHE_SIZE_MB
}

fn default_ttl_seconds() -> u64 {
    DEFAULT_TTL_SECONDS
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

/// Disk cache configuration
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

/// Redis cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisCacheConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub redis_url: Option<String>,
    #[serde(default)]
    pub redis_password: Option<String>,
    #[serde(default = "default_redis_db")]
    pub redis_db: u32,
    #[serde(default = "default_redis_key_prefix")]
    pub redis_key_prefix: String,
    #[serde(default = "default_redis_ttl_seconds")]
    pub redis_ttl_seconds: u64,
}

impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redis_url: None,
            redis_password: None,
            redis_db: default_redis_db(),
            redis_key_prefix: default_redis_key_prefix(),
            redis_ttl_seconds: default_redis_ttl_seconds(),
        }
    }
}

fn default_redis_db() -> u32 {
    0
}

fn default_redis_key_prefix() -> String {
    "yatagarasu:".to_string()
}

fn default_redis_ttl_seconds() -> u64 {
    DEFAULT_TTL_SECONDS
}

impl RedisCacheConfig {
    /// Validate redis cache configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.redis_url.is_none() {
            return Err("redis_url is required when redis cache is enabled".to_string());
        }
        // Basic URL format validation
        if let Some(url) = &self.redis_url {
            if self.enabled && !url.starts_with("redis://") && !url.starts_with("rediss://") {
                return Err("redis_url must start with redis:// or rediss:// (for TLS)".to_string());
            }
        }
        Ok(())
    }
}

/// Per-bucket cache override configuration
/// This can be included in BucketConfig to override global cache settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BucketCacheOverride {
    /// Override: disable caching for this specific bucket
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Override: custom TTL for this bucket (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    /// Override: custom max item size for this bucket (MB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_item_size_mb: Option<u64>,
}

impl BucketCacheOverride {
    /// Merge override with global cache config to get effective config
    pub fn merge_with_global(&self, global: &CacheConfig) -> CacheConfig {
        let mut result = global.clone();

        // Apply enabled override
        if let Some(enabled) = self.enabled {
            result.enabled = enabled;
        }

        // Apply TTL override
        if let Some(ttl) = self.ttl_seconds {
            result.memory.default_ttl_seconds = ttl;
            result.redis.redis_ttl_seconds = ttl;
        }

        // Apply max_item_size override
        if let Some(max_size) = self.max_item_size_mb {
            result.memory.max_item_size_mb = max_size;
        }

        result
    }

    /// Validate bucket cache override
    pub fn validate(&self) -> Result<(), String> {
        // Validate max_item_size if specified
        if let Some(max_size) = self.max_item_size_mb {
            if max_size == 0 {
                return Err("max_item_size_mb must be greater than 0".to_string());
            }
        }

        // Validate TTL if specified
        if let Some(ttl) = self.ttl_seconds {
            if ttl == 0 {
                return Err(
                    "ttl_seconds must be greater than 0 (use enabled=false to disable caching)"
                        .to_string(),
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_empty_cache_config() {
        let _config = CacheConfig::default();
    }

    #[test]
    fn test_can_deserialize_minimal_cache_config_from_yaml() {
        let yaml = r#"
enabled: false
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn test_cache_config_has_enabled_field() {
        let config = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };
        assert!(config.enabled);

        let config = CacheConfig {
            enabled: false,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };
        assert!(!config.enabled);
    }

    #[test]
    fn test_cache_config_defaults_to_disabled() {
        let config = CacheConfig::default();
        assert!(!config.enabled);

        let yaml = r#"{}"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn test_can_parse_cache_config_with_enabled_true() {
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.enabled);
    }

    #[test]
    fn test_can_parse_cache_config_with_enabled_false() {
        let yaml = r#"
enabled: false
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn test_can_parse_memory_cache_section() {
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
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_item_size_mb, 10);

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
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.max_cache_size_mb, 1024);

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
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.memory.default_ttl_seconds, 3600);

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
        let config = MemoryCacheConfig::default();
        assert_eq!(config.max_item_size_bytes(), 10 * 1024 * 1024);
        assert_eq!(config.max_item_size_bytes(), 10485760);

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
        let config = MemoryCacheConfig::default();
        assert_eq!(config.max_cache_size_bytes(), 1024 * 1024 * 1024);
        assert_eq!(config.max_cache_size_bytes(), 1073741824);

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

        let config = MemoryCacheConfig::default();
        assert!(config.validate().is_ok());

        let config = MemoryCacheConfig {
            max_item_size_mb: 1024,
            max_cache_size_mb: 1024,
            default_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_can_parse_disk_cache_section() {
        let yaml = r#"
enabled: true
disk:
  enabled: true
  cache_dir: /tmp/cache
  max_disk_cache_size_mb: 5120
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.disk.enabled);
        assert_eq!(config.disk.cache_dir, "/tmp/cache");
        assert_eq!(config.disk.max_disk_cache_size_mb, 5120);
    }

    #[test]
    fn test_can_parse_cache_dir_default() {
        let config = DiskCacheConfig::default();
        assert_eq!(config.cache_dir, "/var/cache/yatagarasu");

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
        let config = DiskCacheConfig::default();
        assert_eq!(config.max_disk_cache_size_mb, 10240);

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
        let config = DiskCacheConfig::default();
        assert!(!config.enabled);

        let yaml = r#"
enabled: true
disk:
  enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.disk.enabled);
    }

    #[test]
    fn test_rejects_disk_cache_with_empty_cache_dir() {
        let config = DiskCacheConfig {
            enabled: true,
            cache_dir: String::new(),
            max_disk_cache_size_mb: 10240,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cache_dir cannot be empty"));

        let config = DiskCacheConfig::default();
        assert!(config.validate().is_ok());

        let config = DiskCacheConfig {
            enabled: false,
            cache_dir: String::new(),
            max_disk_cache_size_mb: 10240,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_can_parse_redis_cache_section() {
        let yaml = r#"
enabled: true
redis:
  enabled: true
  redis_url: redis://localhost:6379
  redis_password: secret
  redis_db: 1
  redis_key_prefix: "myapp:"
  redis_ttl_seconds: 7200
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.redis.enabled);
        assert_eq!(
            config.redis.redis_url,
            Some("redis://localhost:6379".to_string())
        );
        assert_eq!(config.redis.redis_password, Some("secret".to_string()));
        assert_eq!(config.redis.redis_db, 1);
        assert_eq!(config.redis.redis_key_prefix, "myapp:");
        assert_eq!(config.redis.redis_ttl_seconds, 7200);
    }

    #[test]
    fn test_can_parse_redis_url() {
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_url, None);

        let yaml = r#"
enabled: true
redis:
  redis_url: redis://localhost:6379
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.redis.redis_url,
            Some("redis://localhost:6379".to_string())
        );
    }

    #[test]
    fn test_can_parse_redis_password_optional() {
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_password, None);

        let yaml = r#"
enabled: true
redis:
  redis_password: mypassword
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.redis.redis_password, Some("mypassword".to_string()));
    }

    #[test]
    fn test_can_parse_redis_db_default_0() {
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_db, 0);

        let yaml = r#"
enabled: true
redis:
  redis_db: 5
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.redis.redis_db, 5);
    }

    #[test]
    fn test_can_parse_redis_key_prefix_default() {
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_key_prefix, "yatagarasu:");

        let yaml = r#"
enabled: true
redis:
  redis_key_prefix: "custom:"
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.redis.redis_key_prefix, "custom:");
    }

    #[test]
    fn test_can_parse_redis_ttl_seconds_default() {
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_ttl_seconds, 3600);

        let yaml = r#"
enabled: true
redis:
  redis_ttl_seconds: 1800
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.redis.redis_ttl_seconds, 1800);
    }

    #[test]
    fn test_redis_enabled_defaults_to_false() {
        let config = RedisCacheConfig::default();
        assert!(!config.enabled);

        let yaml = r#"
enabled: true
redis:
  enabled: true
  redis_url: redis://localhost:6379
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.redis.enabled);
    }

    #[test]
    fn test_rejects_redis_cache_with_invalid_url_format() {
        let config = RedisCacheConfig {
            enabled: true,
            redis_url: Some("http://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 0,
            redis_key_prefix: "yatagarasu:".to_string(),
            redis_ttl_seconds: 3600,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must start with redis:// or rediss://"));

        let config = RedisCacheConfig {
            enabled: true,
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 0,
            redis_key_prefix: "yatagarasu:".to_string(),
            redis_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());

        let config = RedisCacheConfig {
            enabled: true,
            redis_url: Some("rediss://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 0,
            redis_key_prefix: "yatagarasu:".to_string(),
            redis_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());

        let config = RedisCacheConfig {
            enabled: false,
            redis_url: None,
            redis_password: None,
            redis_db: 0,
            redis_key_prefix: "yatagarasu:".to_string(),
            redis_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_can_parse_cache_layers_array_default_memory() {
        let config = CacheConfig::default();
        assert_eq!(config.cache_layers, vec!["memory".to_string()]);

        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.cache_layers, vec!["memory".to_string()]);
    }

    #[test]
    fn test_can_parse_cache_layers_with_multiple_layers() {
        let yaml = r#"
enabled: true
disk:
  enabled: true
cache_layers: ["memory", "disk"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.cache_layers,
            vec!["memory".to_string(), "disk".to_string()]
        );
    }

    #[test]
    fn test_can_parse_cache_layers_with_all_layers() {
        let yaml = r#"
enabled: true
disk:
  enabled: true
redis:
  enabled: true
  redis_url: redis://localhost:6379
cache_layers: ["memory", "disk", "redis"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.cache_layers,
            vec![
                "memory".to_string(),
                "disk".to_string(),
                "redis".to_string()
            ]
        );
    }

    #[test]
    fn test_rejects_cache_layers_with_unknown_layer_name() {
        let yaml = r#"
enabled: true
cache_layers: ["memory", "unknown"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown cache layer"));
    }

    #[test]
    fn test_rejects_cache_layers_with_duplicate_layers() {
        let yaml = r#"
enabled: true
cache_layers: ["memory", "memory"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate cache layer"));
    }

    #[test]
    fn test_rejects_cache_layers_with_empty_array_when_enabled() {
        let yaml = r#"
enabled: true
cache_layers: []
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("cache_layers cannot be empty when caching is enabled"));

        let yaml = r#"
enabled: false
cache_layers: []
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validates_disk_layer_requires_disk_enabled() {
        let yaml = r#"
enabled: true
disk:
  enabled: false
cache_layers: ["memory", "disk"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("disk layer requires disk.enabled=true"));

        let yaml = r#"
enabled: true
disk:
  enabled: true
cache_layers: ["memory", "disk"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validates_redis_layer_requires_redis_enabled() {
        let yaml = r#"
enabled: true
redis:
  enabled: false
cache_layers: ["memory", "redis"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("redis layer requires redis.enabled=true"));

        let yaml = r#"
enabled: true
redis:
  enabled: true
  redis_url: redis://localhost:6379
cache_layers: ["memory", "redis"]
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_can_parse_per_bucket_cache_override() {
        let yaml = r#"
enabled: false
ttl_seconds: 1800
max_item_size_mb: 5
"#;
        let override_config: BucketCacheOverride = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(override_config.enabled, Some(false));
        assert_eq!(override_config.ttl_seconds, Some(1800));
        assert_eq!(override_config.max_item_size_mb, Some(5));
    }

    #[test]
    fn test_per_bucket_cache_override_can_disable_caching() {
        let override_config = BucketCacheOverride {
            enabled: Some(false),
            ttl_seconds: None,
            max_item_size_mb: None,
        };

        let global = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert!(!merged.enabled);
    }

    #[test]
    fn test_per_bucket_cache_override_can_set_custom_ttl() {
        let override_config = BucketCacheOverride {
            enabled: None,
            ttl_seconds: Some(600),
            max_item_size_mb: None,
        };

        let global = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert_eq!(merged.memory.default_ttl_seconds, 600);
        assert_eq!(merged.redis.redis_ttl_seconds, 600);
    }

    #[test]
    fn test_per_bucket_cache_override_can_set_custom_max_item_size() {
        let override_config = BucketCacheOverride {
            enabled: None,
            ttl_seconds: None,
            max_item_size_mb: Some(50),
        };

        let global = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert_eq!(merged.memory.max_item_size_mb, 50);
    }

    #[test]
    fn test_per_bucket_cache_inherits_global_defaults() {
        let override_config = BucketCacheOverride {
            enabled: None,
            ttl_seconds: None,
            max_item_size_mb: None,
        };

        let global = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig {
                max_item_size_mb: 10,
                max_cache_size_mb: 1024,
                default_ttl_seconds: 3600,
            },
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert!(merged.enabled);
        assert_eq!(merged.memory.max_item_size_mb, 10);
        assert_eq!(merged.memory.default_ttl_seconds, 3600);
    }

    #[test]
    fn test_rejects_per_bucket_cache_with_invalid_values() {
        let override_config = BucketCacheOverride {
            enabled: None,
            ttl_seconds: None,
            max_item_size_mb: Some(0),
        };
        let result = override_config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("max_item_size_mb must be greater than 0"));

        let override_config = BucketCacheOverride {
            enabled: None,
            ttl_seconds: Some(0),
            max_item_size_mb: None,
        };
        let result = override_config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("ttl_seconds must be greater than 0"));

        let override_config = BucketCacheOverride {
            enabled: Some(true),
            ttl_seconds: Some(300),
            max_item_size_mb: Some(5),
        };
        assert!(override_config.validate().is_ok());
    }

    #[test]
    fn test_validates_cache_config_when_enabled() {
        let config = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig {
                enabled: true,
                cache_dir: "".to_string(),
                max_disk_cache_size_mb: 10240,
            },
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec!["memory".to_string()],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cache_dir cannot be empty"));
    }

    #[test]
    fn test_skips_validation_when_disabled() {
        let config = CacheConfig {
            enabled: false,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig {
                enabled: true,
                cache_dir: "".to_string(),
                max_disk_cache_size_mb: 10240,
            },
            redis: RedisCacheConfig::default(),
            warming: None,
            cache_layers: vec![],
        };

        let result = config.validate();
        assert!(result.is_err());
    }
}
