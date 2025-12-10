//! Cache key and entry types
//!
//! This module defines the core cache entry structures:
//! - `CacheKey`: Unique identifier for cached objects (bucket + object_key + optional etag)
//! - `CacheEntry`: Cached object data with metadata for TTL and LRU management

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Cache key for identifying cached objects
/// Combines bucket name and object key to uniquely identify a cache entry
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Last-Modified timestamp from S3 (for If-Modified-Since validation)
    pub last_modified: Option<String>,
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
    /// * `last_modified` - Last-Modified timestamp from S3 (for If-Modified-Since)
    /// * `ttl` - Time-to-live duration. None uses default (3600s). Zero means no expiration.
    pub fn new(
        data: Bytes,
        content_type: String,
        etag: String,
        last_modified: Option<String>,
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
            last_modified,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // CacheKey tests
    #[test]
    fn test_can_create_cache_key_struct() {
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
        let key = CacheKey {
            bucket: "my-bucket".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };
        assert_eq!(key.bucket, "my-bucket");
    }

    #[test]
    fn test_cache_key_contains_object_key() {
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "path/to/my/object.pdf".to_string(),
            etag: None,
        };
        assert_eq!(key.object_key, "path/to/my/object.pdf");
    }

    #[test]
    fn test_cache_key_contains_etag_optional() {
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
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "file.txt".to_string(),
            etag: None,
        };

        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("file.txt"));
    }

    #[test]
    fn test_cache_key_format_bucket_colon_object_key() {
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
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file:with:colons.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        assert!(string_repr.contains("%3A"));
        assert!(!string_repr.ends_with(":colons.txt"));
    }

    #[test]
    fn test_cache_key_handles_slashes_correctly() {
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "path/to/nested/file.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        assert!(string_repr.contains("path/to/nested/file.txt"));
        assert_eq!(string_repr, "bucket:path/to/nested/file.txt");
    }

    #[test]
    fn test_cache_key_handles_spaces_correctly() {
        let key = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "file with spaces.txt".to_string(),
            etag: None,
        };

        let string_repr = key.to_string();
        assert!(string_repr.contains("%20") || string_repr.contains("file+with+spaces"));
    }

    #[test]
    fn test_can_parse_cache_key_from_string() {
        use std::str::FromStr;

        let cache_key_str = "my-bucket:path/to/file.txt";
        let key = CacheKey::from_str(cache_key_str).unwrap();

        assert_eq!(key.bucket, "my-bucket");
        assert_eq!(key.object_key, "path/to/file.txt");
        assert_eq!(key.etag, None);
    }

    #[test]
    fn test_parsing_fails_gracefully_with_invalid_format() {
        use std::str::FromStr;

        let result = CacheKey::from_str("invalid-format");
        assert!(result.is_err());

        let result = CacheKey::from_str(":object");
        assert!(result.is_err());

        let result = CacheKey::from_str("bucket:");
        assert!(result.is_err());

        let result = CacheKey::from_str(":");
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_key_roundtrip_to_string_parse() {
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
    }

    #[test]
    fn test_same_cache_key_produces_same_hash() {
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

    // CacheEntry tests
    #[test]
    fn test_can_create_cache_entry_struct() {
        let data = Bytes::from("test data");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "abc123".to_string(),
            last_modified: None,
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.data, data);
    }

    #[test]
    fn test_cache_entry_contains_data_bytes() {
        let data = Bytes::from("hello world");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "etag".to_string(),
            last_modified: None,
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        assert_eq!(entry.data, Bytes::from("hello world"));
        assert_eq!(entry.data.len(), 11);
    }

    #[test]
    fn test_cache_entry_can_calculate_size_in_bytes() {
        let data = Bytes::from("test data");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data,
            content_type: "text/plain".to_string(),
            content_length: 9,
            etag: "etag123".to_string(),
            last_modified: None,
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        assert!(size > 0);
    }

    #[test]
    fn test_size_includes_data_length() {
        let data = Bytes::from("hello world");
        let now = SystemTime::now();

        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "etag".to_string(),
            last_modified: None,
            created_at: now,
            expires_at: now,
            last_accessed_at: now,
        };

        let size = entry.size_bytes();
        assert!(size >= data.len());
    }

    #[test]
    fn test_cache_entry_can_check_if_expired() {
        let now = SystemTime::now();
        let past = now - Duration::from_secs(3600);
        let future = now + Duration::from_secs(3600);

        let expired_entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            last_modified: None,
            created_at: past,
            expires_at: past,
            last_accessed_at: now,
        };

        assert!(expired_entry.is_expired());

        let valid_entry = CacheEntry {
            data: Bytes::new(),
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            last_modified: None,
            created_at: now,
            expires_at: future,
            last_accessed_at: now,
        };

        assert!(!valid_entry.is_expired());
    }

    #[test]
    fn test_can_create_entry_with_custom_ttl() {
        let data = Bytes::from("test");
        let custom_ttl = Duration::from_secs(7200);

        let entry = CacheEntry::new(
            data.clone(),
            "text/plain".to_string(),
            "etag123".to_string(),
            None,
            Some(custom_ttl),
        );

        assert_eq!(entry.data, data);
        assert_eq!(entry.content_type, "text/plain");
        assert_eq!(entry.etag, "etag123");

        let now = SystemTime::now();
        let expected_expiry = now + custom_ttl;
        assert!(
            entry.expires_at > now && entry.expires_at <= expected_expiry + Duration::from_secs(1)
        );
    }

    #[test]
    fn test_can_create_entry_with_default_ttl() {
        let data = Bytes::from("test");

        let entry = CacheEntry::new(
            data.clone(),
            "application/json".to_string(),
            "etag456".to_string(),
            None,
            None,
        );

        assert_eq!(entry.data, data);
        assert_eq!(entry.content_type, "application/json");

        let now = SystemTime::now();
        let expected_expiry = now + Duration::from_secs(3600);
        assert!(
            entry.expires_at > now && entry.expires_at <= expected_expiry + Duration::from_secs(1)
        );
    }

    #[test]
    fn test_ttl_of_zero_means_no_expiration() {
        let data = Bytes::from("test");
        let zero_ttl = Duration::from_secs(0);

        let entry = CacheEntry::new(
            data,
            "text/plain".to_string(),
            "etag789".to_string(),
            None,
            Some(zero_ttl),
        );

        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cache_entry_stores_last_modified() {
        let data = Bytes::from("test data");
        let last_modified = Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string());

        let entry = CacheEntry::new(
            data.clone(),
            "text/plain".to_string(),
            "etag123".to_string(),
            last_modified.clone(),
            None,
        );

        assert_eq!(entry.data, data);
        assert_eq!(entry.etag, "etag123");
        assert_eq!(entry.last_modified, last_modified);
    }

    #[test]
    fn test_cache_entry_last_modified_none_by_default() {
        let data = Bytes::from("test data");

        let entry = CacheEntry::new(
            data,
            "text/plain".to_string(),
            "etag123".to_string(),
            None,
            None,
        );

        assert_eq!(entry.last_modified, None);
    }

    #[test]
    fn test_cache_entry_can_update_last_accessed_at() {
        let data = Bytes::from("test");
        let mut entry = CacheEntry::new(
            data,
            "text/plain".to_string(),
            "etag".to_string(),
            None,
            None,
        );

        let original_access_time = entry.last_accessed_at;

        std::thread::sleep(Duration::from_millis(10));
        entry.touch();

        assert!(entry.last_accessed_at > original_access_time);
    }

    #[test]
    fn test_can_validate_entry_against_s3_etag() {
        let entry = CacheEntry::new(
            Bytes::from("data"),
            "text/plain".to_string(),
            "matching-etag".to_string(),
            None,
            None,
        );

        assert!(entry.validate_etag("matching-etag"));
        assert!(!entry.validate_etag("different-etag"));
    }

    #[test]
    fn test_validation_fails_when_entry_expired() {
        let now = SystemTime::now();
        let past = now - Duration::from_secs(3600);

        let entry = CacheEntry {
            data: Bytes::from("data"),
            content_type: "text/plain".to_string(),
            content_length: 4,
            etag: "valid-etag".to_string(),
            last_modified: None,
            created_at: past,
            expires_at: past,
            last_accessed_at: now,
        };

        assert!(!entry.is_valid("valid-etag"));
        assert!(!entry.is_valid("different-etag"));
    }
}
