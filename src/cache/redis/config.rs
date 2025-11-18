// Redis cache configuration module

use serde::{Deserialize, Serialize};

/// Redis-specific cache configuration
///
/// This configuration is part of the broader cache configuration and provides
/// Redis-specific settings for distributed caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL (e.g., "redis://localhost:6379")
    #[serde(default)]
    pub redis_url: Option<String>,

    /// Optional password for Redis authentication
    #[serde(default)]
    pub redis_password: Option<String>,

    /// Redis database number (default: 0)
    #[serde(default = "default_redis_db")]
    pub redis_db: u32,

    /// Key prefix for cache entries (default: "yatagarasu")
    #[serde(default = "default_redis_key_prefix")]
    pub redis_key_prefix: String,

    /// Default TTL for cache entries in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_redis_ttl_seconds")]
    pub redis_ttl_seconds: u64,

    /// Connection timeout in milliseconds (default: 5000 = 5 seconds)
    #[serde(default = "default_connection_timeout_ms")]
    pub connection_timeout_ms: u64,

    /// Operation timeout in milliseconds (default: 2000 = 2 seconds)
    #[serde(default = "default_operation_timeout_ms")]
    pub operation_timeout_ms: u64,

    /// Minimum connection pool size (default: 1)
    #[serde(default = "default_min_pool_size")]
    pub min_pool_size: usize,

    /// Maximum connection pool size (default: 10)
    #[serde(default = "default_max_pool_size")]
    pub max_pool_size: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            redis_url: None,
            redis_password: None,
            redis_db: default_redis_db(),
            redis_key_prefix: default_redis_key_prefix(),
            redis_ttl_seconds: default_redis_ttl_seconds(),
            connection_timeout_ms: default_connection_timeout_ms(),
            operation_timeout_ms: default_operation_timeout_ms(),
            min_pool_size: default_min_pool_size(),
            max_pool_size: default_max_pool_size(),
        }
    }
}

fn default_redis_db() -> u32 {
    0
}

fn default_redis_key_prefix() -> String {
    "yatagarasu".to_string()
}

fn default_redis_ttl_seconds() -> u64 {
    3600 // 1 hour
}

fn default_connection_timeout_ms() -> u64 {
    5000 // 5 seconds
}

fn default_operation_timeout_ms() -> u64 {
    2000 // 2 seconds
}

fn default_min_pool_size() -> usize {
    1
}

fn default_max_pool_size() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_redis_config_from_yaml() {
        let yaml = r#"
redis_url: "redis://localhost:6379"
redis_password: "secret"
redis_db: 1
redis_key_prefix: "test"
redis_ttl_seconds: 7200
connection_timeout_ms: 3000
operation_timeout_ms: 1000
min_pool_size: 2
max_pool_size: 20
"#;

        let config: RedisConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.redis_url, Some("redis://localhost:6379".to_string()));
        assert_eq!(config.redis_password, Some("secret".to_string()));
        assert_eq!(config.redis_db, 1);
        assert_eq!(config.redis_key_prefix, "test");
        assert_eq!(config.redis_ttl_seconds, 7200);
        assert_eq!(config.connection_timeout_ms, 3000);
        assert_eq!(config.operation_timeout_ms, 1000);
        assert_eq!(config.min_pool_size, 2);
        assert_eq!(config.max_pool_size, 20);
    }

    #[test]
    fn test_config_has_redis_url_field() {
        let config = RedisConfig::default();
        assert!(config.redis_url.is_none());

        let config_with_url = RedisConfig {
            redis_url: Some("redis://localhost:6379".to_string()),
            ..Default::default()
        };
        assert_eq!(
            config_with_url.redis_url,
            Some("redis://localhost:6379".to_string())
        );
    }

    #[test]
    fn test_config_has_optional_password_field() {
        let config = RedisConfig::default();
        assert!(config.redis_password.is_none());

        let config_with_password = RedisConfig {
            redis_password: Some("mysecret".to_string()),
            ..Default::default()
        };
        assert_eq!(
            config_with_password.redis_password,
            Some("mysecret".to_string())
        );
    }

    #[test]
    fn test_config_has_database_number_default_zero() {
        let config = RedisConfig::default();
        assert_eq!(config.redis_db, 0);

        let yaml_minimal = "{}";
        let config: RedisConfig = serde_yaml::from_str(yaml_minimal).unwrap();
        assert_eq!(config.redis_db, 0);
    }

    #[test]
    fn test_config_has_connection_pool_settings() {
        let config = RedisConfig::default();
        assert_eq!(config.min_pool_size, 1);
        assert_eq!(config.max_pool_size, 10);

        let yaml = r#"
min_pool_size: 5
max_pool_size: 50
"#;
        let config: RedisConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.min_pool_size, 5);
        assert_eq!(config.max_pool_size, 50);
    }

    #[test]
    fn test_config_has_key_prefix_default_yatagarasu() {
        let config = RedisConfig::default();
        assert_eq!(config.redis_key_prefix, "yatagarasu");

        let yaml_minimal = "{}";
        let config: RedisConfig = serde_yaml::from_str(yaml_minimal).unwrap();
        assert_eq!(config.redis_key_prefix, "yatagarasu");
    }

    #[test]
    fn test_config_has_default_ttl_seconds_3600() {
        let config = RedisConfig::default();
        assert_eq!(config.redis_ttl_seconds, 3600);

        let yaml_minimal = "{}";
        let config: RedisConfig = serde_yaml::from_str(yaml_minimal).unwrap();
        assert_eq!(config.redis_ttl_seconds, 3600);
    }

    #[test]
    fn test_config_has_connection_timeout_ms_default_5000() {
        let config = RedisConfig::default();
        assert_eq!(config.connection_timeout_ms, 5000);

        let yaml_minimal = "{}";
        let config: RedisConfig = serde_yaml::from_str(yaml_minimal).unwrap();
        assert_eq!(config.connection_timeout_ms, 5000);
    }

    #[test]
    fn test_config_has_operation_timeout_ms_default_2000() {
        let config = RedisConfig::default();
        assert_eq!(config.operation_timeout_ms, 2000);

        let yaml_minimal = "{}";
        let config: RedisConfig = serde_yaml::from_str(yaml_minimal).unwrap();
        assert_eq!(config.operation_timeout_ms, 2000);
    }
}
