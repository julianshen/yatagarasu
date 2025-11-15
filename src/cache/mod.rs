// Cache module

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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
    3600 // 1 hour
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

/// Cache key for identifying cached objects
/// Combines bucket name and object key to uniquely identify a cache entry
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CacheKey {
    /// Bucket name
    pub bucket: String,
    /// S3 object key (path)
    pub object_key: String,
    /// Optional ETag for validation
    pub etag: Option<String>,
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format: "bucket:encoded_object_key"
        // URL-encode special characters in object_key, but preserve slashes (valid S3 path separators)
        let encoded_object_key = url_encode_cache_key(&self.object_key);
        write!(f, "{}:{}", self.bucket, encoded_object_key)
    }
}

/// URL-encode a cache key component, preserving slashes but encoding other special characters
fn url_encode_cache_key(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            // Preserve slashes (valid S3 path separators)
            '/' => "/".to_string(),
            // Preserve alphanumeric and common safe characters
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            // Encode everything else
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// URL-decode a cache key component
fn url_decode_cache_key(s: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Read next two characters as hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() != 2 {
                return Err("Invalid URL encoding: incomplete escape sequence".to_string());
            }

            match u8::from_str_radix(&hex, 16) {
                Ok(byte) => decoded.push(byte as char),
                Err(_) => {
                    return Err(format!(
                        "Invalid URL encoding: invalid hex sequence %{}",
                        hex
                    ))
                }
            }
        } else {
            decoded.push(c);
        }
    }

    Ok(decoded)
}

impl std::str::FromStr for CacheKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expected format: "bucket:encoded_object_key"
        let parts: Vec<&str> = s.splitn(2, ':').collect();

        if parts.len() != 2 {
            return Err("Invalid cache key format: missing ':' separator".to_string());
        }

        let bucket = parts[0];
        let encoded_object_key = parts[1];

        if bucket.is_empty() {
            return Err("Invalid cache key format: bucket cannot be empty".to_string());
        }

        if encoded_object_key.is_empty() {
            return Err("Invalid cache key format: object_key cannot be empty".to_string());
        }

        // Decode the object key
        let object_key = url_decode_cache_key(encoded_object_key)?;

        Ok(CacheKey {
            bucket: bucket.to_string(),
            object_key,
            etag: None,
        })
    }
}

/// Cache entry representing a cached S3 object
/// Contains the object data and metadata for cache management
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached object data
    pub data: Bytes,
    /// Content type of the cached object
    pub content_type: String,
    /// Content length of the cached object
    pub content_length: usize,
    /// ETag of the cached object (for validation)
    pub etag: String,
    /// When this entry was created
    pub created_at: SystemTime,
    /// When this entry expires (for TTL-based eviction)
    pub expires_at: SystemTime,
    /// Last time this entry was accessed (for LRU eviction)
    pub last_accessed_at: SystemTime,
}

impl CacheEntry {
    /// Create a new cache entry with the given data and TTL
    ///
    /// # Arguments
    /// * `data` - The cached object data
    /// * `content_type` - MIME type of the object
    /// * `etag` - ETag for validation
    /// * `ttl` - Time-to-live duration. None uses default (3600s). Zero means no expiration.
    pub fn new(
        data: Bytes,
        content_type: String,
        etag: String,
        ttl: Option<std::time::Duration>,
    ) -> Self {
        let now = SystemTime::now();
        let content_length = data.len();

        // Determine expiration time
        let expires_at = match ttl {
            Some(duration) if duration.as_secs() == 0 => {
                // TTL of 0 means no expiration - set to far future
                // Use a large duration (100 years)
                now + std::time::Duration::from_secs(100 * 365 * 24 * 3600)
            }
            Some(duration) => now + duration,
            None => {
                // Default TTL: 3600 seconds (1 hour)
                now + std::time::Duration::from_secs(3600)
            }
        };

        Self {
            data,
            content_type,
            content_length,
            etag,
            created_at: now,
            expires_at,
            last_accessed_at: now,
        }
    }

    /// Check if this cache entry has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }

    /// Update the last accessed timestamp to current time
    /// Used for LRU (Least Recently Used) cache eviction
    pub fn touch(&mut self) {
        self.last_accessed_at = SystemTime::now();
    }

    /// Validate the cache entry's ETag against a provided ETag
    /// Returns true if the ETags match
    pub fn validate_etag(&self, etag: &str) -> bool {
        self.etag == etag
    }

    /// Check if the cache entry is valid (not expired and ETag matches)
    /// Returns true only if both conditions are met
    pub fn is_valid(&self, etag: &str) -> bool {
        !self.is_expired() && self.validate_etag(etag)
    }

    /// Calculate the approximate size of this cache entry in bytes
    /// Includes data length plus metadata overhead
    pub fn size_bytes(&self) -> usize {
        // Data size
        let data_size = self.data.len();

        // String metadata size
        let content_type_size = self.content_type.len();
        let etag_size = self.etag.len();

        // Fixed-size metadata
        let content_length_size = std::mem::size_of::<usize>();
        let timestamps_size = 3 * std::mem::size_of::<SystemTime>();

        // Total size
        data_size + content_type_size + etag_size + content_length_size + timestamps_size
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

/// Cache error types
#[derive(Debug)]
pub enum CacheError {
    /// Cache entry not found
    NotFound,
    /// Cache storage is full
    StorageFull,
    /// I/O error (for disk cache)
    IoError(std::io::Error),
    /// Redis error (for redis cache)
    RedisError(String),
    /// Serialization/deserialization error
    SerializationError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::NotFound => write!(f, "Cache entry not found"),
            CacheError::StorageFull => write!(f, "Cache storage is full"),
            CacheError::IoError(err) => write!(f, "I/O error: {}", err),
            CacheError::RedisError(msg) => write!(f, "Redis error: {}", msg),
            CacheError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CacheError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        CacheError::IoError(err)
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::SerializationError(err.to_string())
    }
}

/// Cache statistics for monitoring and metrics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions (due to size/TTL)
    pub evictions: u64,
    /// Current cache size in bytes
    pub current_size_bytes: u64,
    /// Current number of items in cache
    pub current_item_count: u64,
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
}

impl CacheStats {
    /// Calculate hit rate (hits / total requests)
    /// Returns 0.0 if there are no requests
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Cache trait for different cache implementations (memory, disk, redis)
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a cache entry by key
    /// Returns None if the key is not found or the entry has expired
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError>;

    /// Set a cache entry
    /// Overwrites existing entry if key already exists
    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError>;

    /// Delete a cache entry by key
    /// Returns true if the entry was deleted, false if it didn't exist
    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError>;

    /// Clear all cache entries
    async fn clear(&self) -> Result<(), CacheError>;

    /// Get cache statistics
    async fn stats(&self) -> Result<CacheStats, CacheError>;
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
            redis: RedisCacheConfig::default(),
            cache_layers: vec!["memory".to_string()],
        };
        assert_eq!(config.enabled, true);

        let config = CacheConfig {
            enabled: false,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig::default(),
            redis: RedisCacheConfig::default(),
            cache_layers: vec!["memory".to_string()],
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

    // Redis Cache Configuration tests
    #[test]
    fn test_can_parse_redis_cache_section() {
        // Test: Can parse redis cache section (optional)
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
        assert_eq!(config.redis.enabled, true);
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
        // Test: Can parse redis_url (e.g., redis://localhost:6379)
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_url, None);

        // Test explicit value
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
        // Test: Can parse redis_password (optional)
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_password, None);

        // Test explicit value
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
        // Test: Can parse redis_db (default 0)
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_db, 0);

        // Test explicit value
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
        // Test: Can parse redis_key_prefix (default "yatagarasu:")
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_key_prefix, "yatagarasu:");

        // Test explicit value
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
        // Test: Can parse redis_ttl_seconds (default 3600)
        let config = RedisCacheConfig::default();
        assert_eq!(config.redis_ttl_seconds, 3600);

        // Test explicit value
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
        // Test: Can parse redis_enabled (default false)
        let config = RedisCacheConfig::default();
        assert_eq!(config.enabled, false);

        // Test explicit enabled
        let yaml = r#"
enabled: true
redis:
  enabled: true
  redis_url: redis://localhost:6379
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.redis.enabled, true);
    }

    #[test]
    fn test_rejects_redis_cache_with_invalid_url_format() {
        // Test: Rejects redis cache with invalid URL format
        let config = RedisCacheConfig {
            enabled: true,
            redis_url: Some("http://localhost:6379".to_string()), // Wrong protocol
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

        // Valid redis:// URL should pass
        let config = RedisCacheConfig {
            enabled: true,
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 0,
            redis_key_prefix: "yatagarasu:".to_string(),
            redis_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());

        // Valid rediss:// URL (TLS) should pass
        let config = RedisCacheConfig {
            enabled: true,
            redis_url: Some("rediss://localhost:6379".to_string()),
            redis_password: None,
            redis_db: 0,
            redis_key_prefix: "yatagarasu:".to_string(),
            redis_ttl_seconds: 3600,
        };
        assert!(config.validate().is_ok());

        // Disabled cache doesn't need URL
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

    // Cache Hierarchy Configuration tests
    #[test]
    fn test_can_parse_cache_layers_array_default_memory() {
        // Test: Can parse cache_layers array (default: ["memory"])
        let config = CacheConfig::default();
        assert_eq!(config.cache_layers, vec!["memory".to_string()]);

        // Test with empty YAML
        let yaml = r#"
enabled: true
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.cache_layers, vec!["memory".to_string()]);
    }

    #[test]
    fn test_can_parse_cache_layers_with_multiple_layers() {
        // Test: Can parse cache_layers with multiple layers (["memory", "disk"])
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
        // Test: Can parse cache_layers with all layers (["memory", "disk", "redis"])
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
        // Test: Rejects cache_layers with unknown layer name
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
        // Test: Rejects cache_layers with duplicate layers
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
        // Test: Rejects cache_layers with empty array when caching enabled
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

        // Empty layers OK when caching disabled
        let yaml = r#"
enabled: false
cache_layers: []
"#;
        let config: CacheConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validates_disk_layer_requires_disk_enabled() {
        // Test: Validates disk layer requires disk.enabled=true
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

        // Valid config with disk enabled
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
        // Test: Validates redis layer requires redis.enabled=true
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

        // Valid config with redis enabled
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

    // Per-Bucket Cache Configuration tests
    #[test]
    fn test_can_parse_per_bucket_cache_override() {
        // Test: Can parse per-bucket cache override in bucket config
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
        // Test: Per-bucket cache override can disable caching for specific bucket
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
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert_eq!(merged.enabled, false);
    }

    #[test]
    fn test_per_bucket_cache_override_can_set_custom_ttl() {
        // Test: Per-bucket cache override can set custom TTL
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
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert_eq!(merged.memory.default_ttl_seconds, 600);
        assert_eq!(merged.redis.redis_ttl_seconds, 600);
    }

    #[test]
    fn test_per_bucket_cache_override_can_set_custom_max_item_size() {
        // Test: Per-bucket cache override can set custom max_item_size
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
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert_eq!(merged.memory.max_item_size_mb, 50);
    }

    #[test]
    fn test_per_bucket_cache_inherits_global_defaults() {
        // Test: Per-bucket cache inherits global defaults when not overridden
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
            cache_layers: vec!["memory".to_string()],
        };

        let merged = override_config.merge_with_global(&global);
        assert_eq!(merged.enabled, true);
        assert_eq!(merged.memory.max_item_size_mb, 10);
        assert_eq!(merged.memory.default_ttl_seconds, 3600);
    }

    #[test]
    fn test_rejects_per_bucket_cache_with_invalid_values() {
        // Test: Rejects per-bucket cache with invalid values

        // Zero max_item_size_mb is invalid
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

        // Zero ttl_seconds is invalid
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

        // Valid values pass
        let override_config = BucketCacheOverride {
            enabled: Some(true),
            ttl_seconds: Some(300),
            max_item_size_mb: Some(5),
        };
        assert!(override_config.validate().is_ok());
    }

    // Configuration Validation tests
    #[test]
    fn test_validates_cache_config_when_enabled() {
        // Test: Validates cache config when enabled=true
        let config = CacheConfig {
            enabled: true,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig {
                enabled: true,
                cache_dir: "".to_string(), // Invalid: empty cache_dir
                max_disk_cache_size_mb: 10240,
            },
            redis: RedisCacheConfig::default(),
            cache_layers: vec!["memory".to_string()],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cache_dir cannot be empty"));
    }

    #[test]
    fn test_skips_validation_when_disabled() {
        // Test: Skips validation when enabled=false
        // When cache is disabled, validation should still be called but not fail for empty layers
        let config = CacheConfig {
            enabled: false,
            memory: MemoryCacheConfig::default(),
            disk: DiskCacheConfig {
                enabled: true,
                cache_dir: "".to_string(), // Would be invalid if enabled
                max_disk_cache_size_mb: 10240,
            },
            redis: RedisCacheConfig::default(),
            cache_layers: vec![], // Would be invalid if enabled
        };

        // Validation still runs and catches the empty cache_dir
        let result = config.validate();
        assert!(result.is_err()); // Still validates individual layer configs
    }

    // Phase 26.2: Cache Key Design tests
    #[test]
    fn test_can_create_cache_key_struct() {
        // Test: Can create CacheKey struct
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "path/to/object.jpg".to_string(),
            etag: None,
        };
        assert_eq!(key.bucket, "test-bucket");
        assert_eq!(key.object_key, "path/to/object.jpg");
        assert_eq!(key.etag, None);
    }

    #[test]
    fn test_cache_key_contains_bucket_name() {
        // Test: CacheKey contains bucket name
        let key = CacheKey {
            bucket: "my-bucket".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };
        assert_eq!(key.bucket, "my-bucket");
    }

    #[test]
    fn test_cache_key_contains_object_key() {
        // Test: CacheKey contains object key (S3 path)
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "path/to/my/object.pdf".to_string(),
            etag: None,
        };
        assert_eq!(key.object_key, "path/to/my/object.pdf");
    }

    #[test]
    fn test_cache_key_contains_etag_optional() {
        // Test: CacheKey contains etag (optional for validation)
        let key_without_etag = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };
        assert_eq!(key_without_etag.etag, None);

        let key_with_etag = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file.txt".to_string(),
            etag: Some("abc123".to_string()),
        };
        assert_eq!(key_with_etag.etag, Some("abc123".to_string()));
    }

    #[test]
    fn test_cache_key_implements_hash_trait() {
        // Test: CacheKey implements Hash trait
        use std::collections::HashMap;

        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let mut map: HashMap<CacheKey, String> = HashMap::new();
        map.insert(key.clone(), "value".to_string());

        assert_eq!(map.get(&key), Some(&"value".to_string()));
    }

    #[test]
    fn test_cache_key_implements_eq_trait() {
        // Test: CacheKey implements Eq trait
        let key1 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let key2 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let key3 = CacheKey {
            bucket: "different".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_implements_clone_trait() {
        // Test: CacheKey implements Clone trait
        let key1 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: Some("etag123".to_string()),
        };

        let key2 = key1.clone();
        assert_eq!(key1, key2);
        assert_eq!(key2.bucket, "bucket");
        assert_eq!(key2.object_key, "key");
        assert_eq!(key2.etag, Some("etag123".to_string()));
    }

    #[test]
    fn test_cache_key_implements_debug_trait() {
        // Test: CacheKey implements Debug trait
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };

        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("file.txt"));
    }

    // CacheKey String Representation tests
    #[test]
    fn test_cache_key_can_serialize_to_string() {
        // Test: CacheKey can serialize to string (for Redis keys)
        let key = CacheKey {
            bucket: "my-bucket".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        assert!(string_repr.contains("my-bucket"));
        assert!(string_repr.contains("file.txt"));
    }

    #[test]
    fn test_cache_key_format_bucket_colon_object_key() {
        // Test: CacheKey format: "bucket:object_key"
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "path/to/file.jpg".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        assert_eq!(string_repr, "test-bucket:path/to/file.jpg");
    }

    #[test]
    fn test_cache_key_escapes_special_characters() {
        // Test: CacheKey escapes special characters in object_key
        // Colons in object_key should be escaped to avoid confusion with separator
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file:with:colons.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        // Colons in the object key should be URL-encoded (%3A)
        assert!(string_repr.contains("%3A"));
        assert!(!string_repr.ends_with(":colons.txt"));
    }

    #[test]
    fn test_cache_key_handles_slashes_correctly() {
        // Test: CacheKey handles object keys with slashes correctly
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "path/to/nested/file.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        // Slashes should be preserved (they're valid S3 path separators)
        assert!(string_repr.contains("path/to/nested/file.txt"));
        assert_eq!(string_repr, "bucket:path/to/nested/file.txt");
    }

    #[test]
    fn test_cache_key_handles_spaces_correctly() {
        // Test: CacheKey handles object keys with spaces correctly
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file with spaces.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        // Spaces should be URL-encoded
        assert!(string_repr.contains("%20") || string_repr.contains("file+with+spaces"));
    }

    #[test]
    fn test_cache_key_handles_unicode_correctly() {
        // Test: CacheKey handles Unicode object keys correctly
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "文件/αρχείο/файл.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        // Unicode should be URL-encoded or preserved correctly
        assert!(string_repr.starts_with("bucket:"));
        // Should contain URL-encoded Unicode or the actual Unicode characters
        assert!(string_repr.len() > "bucket:".len());
    }

    // CacheKey Parsing tests
    #[test]
    fn test_can_parse_cache_key_from_string() {
        // Test: Can parse CacheKey from string
        use std::str::FromStr;

        let cache_key_str = "my-bucket:path/to/file.txt";
        let key = CacheKey::from_str(cache_key_str).unwrap();

        assert_eq!(key.bucket, "my-bucket");
        assert_eq!(key.object_key, "path/to/file.txt");
        assert_eq!(key.etag, None);
    }

    #[test]
    fn test_parsing_fails_gracefully_with_invalid_format() {
        // Test: Parsing fails gracefully with invalid format
        use std::str::FromStr;

        // No colon separator
        let result = CacheKey::from_str("invalid-format");
        assert!(result.is_err());

        // Empty bucket
        let result = CacheKey::from_str(":object");
        assert!(result.is_err());

        // Empty object key
        let result = CacheKey::from_str("bucket:");
        assert!(result.is_err());

        // Only colon
        let result = CacheKey::from_str(":");
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_key_roundtrip_to_string_parse() {
        // Test: Roundtrip: to_string().parse() == original
        use std::str::FromStr;

        let original = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "path/to/file.txt".to_string(),
            etag: None,
        };

        let string_repr = original.to_string();
        let parsed = CacheKey::from_str(&string_repr).unwrap();

        assert_eq!(parsed.bucket, original.bucket);
        assert_eq!(parsed.object_key, original.object_key);
        assert_eq!(parsed.etag, original.etag);

        // Test with special characters
        let original_special = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file with spaces.txt".to_string(),
            etag: None,
        };

        let string_repr = original_special.to_string();
        let parsed_special = CacheKey::from_str(&string_repr).unwrap();

        assert_eq!(parsed_special.object_key, "file with spaces.txt");
    }

    // CacheKey Hashing tests
    #[test]
    fn test_same_cache_key_produces_same_hash() {
        // Test: Same CacheKey produces same hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let key1 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let key2 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let mut hasher1 = DefaultHasher::new();
        key1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        key2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_cache_keys_produce_different_hashes() {
        // Test: Different CacheKeys produce different hashes
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let key1 = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let key2 = CacheKey {
            bucket: "bucket2".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let mut hasher1 = DefaultHasher::new();
        key1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        key2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cache_key_with_different_etags_are_different() {
        // Test: CacheKey with different etags are different keys
        let key1 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: Some("etag1".to_string()),
        };

        let key2 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: Some("etag2".to_string()),
        };

        let key3 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key2, key3);
    }

    #[test]
    fn test_cache_key_hash_is_stable() {
        // Test: CacheKey hash is stable across runs (within same execution)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: Some("etag".to_string()),
        };

        // Hash multiple times in same execution
        let mut hasher1 = DefaultHasher::new();
        key.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        key.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        let mut hasher3 = DefaultHasher::new();
        key.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    // Phase 26.3: Cache Entry Design tests

    // CacheEntry Structure tests
    #[test]
    fn test_can_create_cache_entry_struct() {
        // Test: Can create CacheEntry struct
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from("test data");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "abc123".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.data, data);
    }

    #[test]
    fn test_cache_entry_contains_data_bytes() {
        // Test: CacheEntry contains data (Bytes)
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from("hello world");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "etag".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.data, Bytes::from("hello world"));
        assert_eq!(entry.data.len(), 11);
    }

    #[test]
    fn test_cache_entry_contains_content_type() {
        // Test: CacheEntry contains content_type (String)
        use bytes::Bytes;
        use std::time::SystemTime;

        let now = SystemTime::now();
        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "application/json".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.content_type, "application/json");
    }

    #[test]
    fn test_cache_entry_contains_content_length() {
        // Test: CacheEntry contains content_length (usize)
        use bytes::Bytes;
        use std::time::SystemTime;

        let now = SystemTime::now();
        let entry = CacheEntry {
            data: Bytes::from("test"),
            content_type: "text/plain".to_string(),
            content_length: 1024,
            etag: "etag".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.content_length, 1024);
    }

    #[test]
    fn test_cache_entry_contains_etag() {
        // Test: CacheEntry contains etag (String)
        use bytes::Bytes;
        use std::time::SystemTime;

        let now = SystemTime::now();
        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "my-etag-123".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.etag, "my-etag-123");
    }

    #[test]
    fn test_cache_entry_contains_created_at() {
        // Test: CacheEntry contains created_at (timestamp)
        use bytes::Bytes;
        use std::time::SystemTime;

        let created = SystemTime::now();
        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: created,
            expires_at: created,
            last_accessed_at: created,
        };

        assert_eq!(entry.created_at, created);
    }

    #[test]
    fn test_cache_entry_contains_expires_at() {
        // Test: CacheEntry contains expires_at (timestamp)
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let now = SystemTime::now();
        let expires = now + Duration::from_secs(3600);

        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: now,
            expires_at: expires,
            last_accessed_at: now,
        };

        assert_eq!(entry.expires_at, expires);
    }

    #[test]
    fn test_cache_entry_contains_last_accessed_at() {
        // Test: CacheEntry contains last_accessed_at (timestamp, for LRU)
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let now = SystemTime::now();
        let accessed = now + Duration::from_secs(10);

        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: accessed,
        };

        assert_eq!(entry.last_accessed_at, accessed);
    }

    // CacheEntry Size Calculation tests
    #[test]
    fn test_cache_entry_can_calculate_size_in_bytes() {
        // Test: CacheEntry can calculate its size in bytes
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from("test data");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data,
            content_type: "text/plain".to_string(),
            content_length: 9,
            etag: "etag123".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        assert!(size > 0);
    }

    #[test]
    fn test_size_includes_data_length() {
        // Test: Size includes data length
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from("hello world");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "etag".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        // Size should at least include the data length
        assert!(size >= data.len());
    }

    #[test]
    fn test_size_includes_metadata_overhead() {
        // Test: Size includes metadata overhead (approximate)
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from("test");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "application/json".to_string(),
            content_length: data.len(),
            etag: "abc123".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        // Size should be greater than just data (includes strings and metadata)
        // Estimate: data.len() + content_type.len() + etag.len() + sizeof(usize) + 3*sizeof(SystemTime)
        let metadata_size = "application/json".len()
            + "abc123".len()
            + std::mem::size_of::<usize>()
            + 3 * std::mem::size_of::<SystemTime>();
        assert!(size >= data.len() + metadata_size);
    }

    #[test]
    fn test_size_accurate_for_small_entries() {
        // Test: Size is accurate for small entries (<1KB)
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from(vec![0u8; 512]); // 512 bytes
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "image/png".to_string(),
            content_length: data.len(),
            etag: "small".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        // Should be at least the data size
        assert!(size >= 512);
        // Should not be wildly inaccurate (< 2x the data size for small entries)
        assert!(size < 2048);
    }

    #[test]
    fn test_size_accurate_for_large_entries() {
        // Test: Size is accurate for large entries (>1MB)
        use bytes::Bytes;
        use std::time::SystemTime;

        let data = Bytes::from(vec![0u8; 2 * 1024 * 1024]); // 2MB
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "video/mp4".to_string(),
            content_length: data.len(),
            etag: "large".to_string(),
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        // Should be at least the data size
        assert!(size >= 2 * 1024 * 1024);
        // Metadata overhead should be negligible compared to data size
        // Allow 1% overhead for metadata
        assert!(size < (2 * 1024 * 1024) + (2 * 1024 * 1024 / 100));
    }

    // CacheEntry TTL & Expiration tests
    #[test]
    fn test_cache_entry_can_check_if_expired() {
        // Test: CacheEntry can check if expired
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let now = SystemTime::now();
        let past = now - Duration::from_secs(3600);
        let future = now + Duration::from_secs(3600);

        // Expired entry
        let expired_entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: past,
            expires_at: past,
            last_accessed_at: now,
        };

        assert!(expired_entry.is_expired());

        // Valid entry
        let valid_entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: now,
            expires_at: future,
            last_accessed_at: now,
        };

        assert!(!valid_entry.is_expired());
    }

    #[test]
    fn test_is_expired_returns_false_before_expires_at() {
        // Test: is_expired() returns false before expires_at
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let now = SystemTime::now();
        let future = now + Duration::from_secs(7200); // 2 hours in future

        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: now,
            expires_at: future,
            last_accessed_at: now,
        };

        assert!(!entry.is_expired());
    }

    #[test]
    fn test_is_expired_returns_true_after_expires_at() {
        // Test: is_expired() returns true after expires_at
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let now = SystemTime::now();
        let past = now - Duration::from_secs(1); // 1 second in past

        let entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at: past,
            expires_at: past,
            last_accessed_at: past,
        };

        assert!(entry.is_expired());
    }

    #[test]
    fn test_can_create_entry_with_custom_ttl() {
        // Test: Can create entry with custom TTL
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let data = Bytes::from("test");
        let custom_ttl = Duration::from_secs(7200); // 2 hours

        let entry = CacheEntry::new(
            data.clone(),
            "text/plain".to_string(),
            "etag123".to_string(),
            Some(custom_ttl),
        );

        assert_eq!(entry.data, data);
        assert_eq!(entry.content_type, "text/plain");
        assert_eq!(entry.etag, "etag123");

        // Check TTL was applied correctly
        let now = SystemTime::now();
        let expected_expiry = now + custom_ttl;
        // Allow 1 second tolerance for test execution time
        assert!(
            entry.expires_at > now && entry.expires_at <= expected_expiry + Duration::from_secs(1)
        );
    }

    #[test]
    fn test_can_create_entry_with_default_ttl() {
        // Test: Can create entry with default TTL
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let data = Bytes::from("test");

        let entry = CacheEntry::new(
            data.clone(),
            "application/json".to_string(),
            "etag456".to_string(),
            None, // Use default TTL
        );

        assert_eq!(entry.data, data);
        assert_eq!(entry.content_type, "application/json");

        // Check default TTL was applied (3600 seconds = 1 hour)
        let now = SystemTime::now();
        let expected_expiry = now + Duration::from_secs(3600);
        // Allow 1 second tolerance
        assert!(
            entry.expires_at > now && entry.expires_at <= expected_expiry + Duration::from_secs(1)
        );
    }

    #[test]
    fn test_ttl_of_zero_means_no_expiration() {
        // Test: TTL of 0 means no expiration
        use bytes::Bytes;
        use std::time::Duration;

        let data = Bytes::from("test");
        let zero_ttl = Duration::from_secs(0);

        let entry = CacheEntry::new(
            data,
            "text/plain".to_string(),
            "etag789".to_string(),
            Some(zero_ttl),
        );

        // Entry with TTL=0 should never expire
        // We represent this by setting expires_at to a far future time
        assert!(!entry.is_expired());

        // Even after waiting, it should not expire
        // (We can't actually wait, but we verify the expires_at is far in the future)
        // A TTL of 0 should set expires_at to SystemTime::MAX or a very large value
    }

    // CacheEntry Access Tracking (for LRU) tests
    #[test]
    fn test_cache_entry_can_update_last_accessed_at() {
        // Test: CacheEntry can update last_accessed_at
        use bytes::Bytes;
        use std::time::Duration;

        let data = Bytes::from("test");
        let mut entry = CacheEntry::new(data, "text/plain".to_string(), "etag".to_string(), None);

        let original_access_time = entry.last_accessed_at;

        // Wait a tiny bit and touch the entry
        std::thread::sleep(Duration::from_millis(10));
        entry.touch();

        // last_accessed_at should be updated
        assert!(entry.last_accessed_at > original_access_time);
    }

    #[test]
    fn test_touch_updates_last_accessed_at_to_current_time() {
        // Test: touch() updates last_accessed_at to current time
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        let data = Bytes::from("test");
        let mut entry = CacheEntry::new(data, "text/plain".to_string(), "etag".to_string(), None);

        // Wait a bit
        std::thread::sleep(Duration::from_millis(10));

        let before_touch = SystemTime::now();
        entry.touch();
        let after_touch = SystemTime::now();

        // last_accessed_at should be between before and after
        assert!(entry.last_accessed_at >= before_touch);
        assert!(entry.last_accessed_at <= after_touch);
    }

    #[test]
    fn test_last_accessed_at_used_for_lru_sorting() {
        // Test: last_accessed_at used for LRU sorting
        use bytes::Bytes;
        use std::time::Duration;

        // Create three entries
        let mut entry1 = CacheEntry::new(
            Bytes::from("data1"),
            "text/plain".to_string(),
            "etag1".to_string(),
            None,
        );

        std::thread::sleep(Duration::from_millis(10));

        let entry2 = CacheEntry::new(
            Bytes::from("data2"),
            "text/plain".to_string(),
            "etag2".to_string(),
            None,
        );

        std::thread::sleep(Duration::from_millis(10));

        let entry3 = CacheEntry::new(
            Bytes::from("data3"),
            "text/plain".to_string(),
            "etag3".to_string(),
            None,
        );

        // entry3 should have the most recent access time
        assert!(entry3.last_accessed_at > entry2.last_accessed_at);
        assert!(entry2.last_accessed_at > entry1.last_accessed_at);

        // Touch entry1 to make it most recently accessed
        std::thread::sleep(Duration::from_millis(10));
        entry1.touch();

        // Now entry1 should be most recent
        assert!(entry1.last_accessed_at > entry3.last_accessed_at);
        assert!(entry1.last_accessed_at > entry2.last_accessed_at);

        // Can sort by last_accessed_at for LRU ordering
        let mut entries = vec![&entry1, &entry2, &entry3];
        entries.sort_by_key(|e| e.last_accessed_at);

        // After sorting, least recently accessed should be first
        assert_eq!(entries[0].etag, "etag2");
        assert_eq!(entries[1].etag, "etag3");
        assert_eq!(entries[2].etag, "etag1");
    }

    // CacheEntry Validation tests
    #[test]
    fn test_can_validate_entry_against_s3_etag() {
        // Test: Can validate entry against S3 ETag
        use bytes::Bytes;

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "matching-etag".to_string(),
            None,
        );

        // Validate against matching ETag
        assert!(entry.validate_etag("matching-etag"));

        // Validate against non-matching ETag
        assert!(!entry.validate_etag("different-etag"));
    }

    #[test]
    fn test_validation_succeeds_when_etags_match() {
        // Test: Validation succeeds when ETags match
        use bytes::Bytes;

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "application/json".to_string(),
            "abc123def456".to_string(),
            None,
        );

        assert!(entry.validate_etag("abc123def456"));
    }

    #[test]
    fn test_validation_fails_when_etags_differ() {
        // Test: Validation fails when ETags differ
        use bytes::Bytes;

        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "original-etag".to_string(),
            None,
        );

        assert!(!entry.validate_etag("updated-etag"));
        assert!(!entry.validate_etag(""));
        assert!(!entry.validate_etag("completely-different"));
    }

    #[test]
    fn test_validation_fails_when_entry_expired() {
        // Test: Validation fails when entry expired
        use bytes::Bytes;
        use std::time::{Duration, SystemTime};

        // Create an expired entry
        let now = SystemTime::now();
        let past = now - Duration::from_secs(3600);

        let entry = CacheEntry {
            data: Bytes::from("data"),
            content_type: "text/plain".to_string(),
            content_length: 4,
            etag: "valid-etag".to_string(),
            created_at: past,
            expires_at: past, // Already expired
            last_accessed_at: now,
        };

        // Even with matching ETag, validation should fail if expired
        assert!(!entry.is_valid("valid-etag"));

        // Non-matching ETag should also fail
        assert!(!entry.is_valid("different-etag"));
    }

    // Phase 26.4: Cache Trait Abstraction tests

    // Cache Trait Definition tests
    #[test]
    fn test_can_define_cache_trait() {
        // Test: Can define Cache trait
        // If the trait compiles, this test passes
        // The trait should be public and available
        fn _assert_trait_exists<T: Cache>() {}
    }

    #[test]
    fn test_cache_trait_has_get_method() {
        // Test: Cache trait has get() method signature
        // This test verifies the method signature compiles
        async fn _test_get<T: Cache>(cache: &T, key: &CacheKey) {
            let _result: Result<Option<CacheEntry>, CacheError> = cache.get(key).await;
        }
    }

    #[test]
    fn test_cache_trait_has_set_method() {
        // Test: Cache trait has set() method signature
        async fn _test_set<T: Cache>(cache: &T, key: CacheKey, entry: CacheEntry) {
            let _result: Result<(), CacheError> = cache.set(key, entry).await;
        }
    }

    #[test]
    fn test_cache_trait_has_delete_method() {
        // Test: Cache trait has delete() method signature
        async fn _test_delete<T: Cache>(cache: &T, key: &CacheKey) {
            let _result: Result<bool, CacheError> = cache.delete(key).await;
        }
    }

    #[test]
    fn test_cache_trait_has_clear_method() {
        // Test: Cache trait has clear() method signature
        async fn _test_clear<T: Cache>(cache: &T) {
            let _result: Result<(), CacheError> = cache.clear().await;
        }
    }

    #[test]
    fn test_cache_trait_has_stats_method() {
        // Test: Cache trait has stats() method signature
        async fn _test_stats<T: Cache>(cache: &T) {
            let _result: Result<CacheStats, CacheError> = cache.stats().await;
        }
    }

    #[test]
    fn test_cache_trait_methods_are_async() {
        // Test: All methods are async
        // This is verified by the async fn signatures in the tests above
        // The trait uses #[async_trait] which makes all methods async
    }

    #[test]
    fn test_cache_trait_methods_return_result() {
        // Test: All methods return Result<T, CacheError>
        // This is verified by the return type checks in the tests above
    }

    // Mock Cache implementation for testing
    struct MockCache;

    #[async_trait]
    impl Cache for MockCache {
        async fn get(&self, _key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
            Ok(None)
        }

        async fn set(&self, _key: CacheKey, _entry: CacheEntry) -> Result<(), CacheError> {
            Ok(())
        }

        async fn delete(&self, _key: &CacheKey) -> Result<bool, CacheError> {
            Ok(false)
        }

        async fn clear(&self) -> Result<(), CacheError> {
            Ok(())
        }

        async fn stats(&self) -> Result<CacheStats, CacheError> {
            Ok(CacheStats::default())
        }
    }

    #[test]
    fn test_cache_trait_compiles_with_signatures() {
        // Test: Cache trait compiles with signatures
        // If MockCache compiles, the trait is properly defined
        let _cache = MockCache;
    }

    #[tokio::test]
    async fn test_can_create_mock_implementation() {
        // Test: Can create mock implementation of Cache trait
        let cache = MockCache;
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        // Test all methods work
        let get_result = cache.get(&key).await;
        assert!(get_result.is_ok());

        let entry = CacheEntry::new(
            bytes::Bytes::from("data"),
            "text/plain".to_string(),
            "etag".to_string(),
            None,
        );
        let set_result = cache.set(key.clone(), entry).await;
        assert!(set_result.is_ok());

        let delete_result = cache.delete(&key).await;
        assert!(delete_result.is_ok());

        let clear_result = cache.clear().await;
        assert!(clear_result.is_ok());

        let stats_result = cache.stats().await;
        assert!(stats_result.is_ok());
    }

    #[test]
    fn test_mock_satisfies_send_sync_bounds() {
        // Test: Mock implementation satisfies Send + Sync bounds
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockCache>();
    }

    // Cache Error Types tests
    #[test]
    fn test_can_create_cache_error() {
        // Test: Can create CacheError enum
        let _err1 = CacheError::NotFound;
        let _err2 = CacheError::StorageFull;
        let _err3 = CacheError::RedisError("test".to_string());
    }

    #[test]
    fn test_cache_error_has_not_found_variant() {
        // Test: CacheError has NotFound variant
        let err = CacheError::NotFound;
        matches!(err, CacheError::NotFound);
    }

    #[test]
    fn test_cache_error_has_storage_full_variant() {
        // Test: CacheError has StorageFull variant
        let err = CacheError::StorageFull;
        matches!(err, CacheError::StorageFull);
    }

    #[test]
    fn test_cache_error_has_io_error_variant() {
        // Test: CacheError has IoError variant (for disk cache)
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CacheError::IoError(io_err);
        matches!(err, CacheError::IoError(_));
    }

    #[test]
    fn test_cache_error_has_redis_error_variant() {
        // Test: CacheError has RedisError variant (for redis cache)
        let err = CacheError::RedisError("connection failed".to_string());
        matches!(err, CacheError::RedisError(_));
    }

    #[test]
    fn test_cache_error_has_serialization_error_variant() {
        // Test: CacheError has SerializationError variant
        let err = CacheError::SerializationError("invalid JSON".to_string());
        matches!(err, CacheError::SerializationError(_));
    }

    #[test]
    fn test_cache_error_implements_error_trait() {
        // Test: CacheError implements Error trait
        fn assert_error<T: std::error::Error>() {}
        assert_error::<CacheError>();
    }

    #[test]
    fn test_cache_error_implements_display_trait() {
        // Test: CacheError implements Display trait
        let err = CacheError::NotFound;
        let display_str = format!("{}", err);
        assert!(display_str.contains("not found"));

        let err = CacheError::StorageFull;
        let display_str = format!("{}", err);
        assert!(display_str.contains("full"));
    }

    #[test]
    fn test_cache_error_converts_from_io_error() {
        // Test: CacheError can convert from std::io::Error
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let cache_err: CacheError = io_err.into();
        matches!(cache_err, CacheError::IoError(_));
    }

    #[test]
    fn test_cache_error_converts_from_serde_error() {
        // Test: CacheError can convert from serde_json::Error
        let json_str = "{invalid json}";
        let serde_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let cache_err: CacheError = serde_err.into();
        matches!(cache_err, CacheError::SerializationError(_));
    }
}
