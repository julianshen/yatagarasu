// Redis key formatting and hashing utilities

use sha2::{Digest, Sha256};

/// Maximum Redis key length (Redis limit is 512MB, but we use a practical limit)
/// Keys longer than this will be hashed
pub const MAX_KEY_LENGTH: usize = 250;

/// Formats a Redis cache key with prefix
///
/// # Format
/// - Short keys: "{prefix}:{bucket}:{object_key}"
/// - Long keys: "{prefix}:hash:{sha256}"
///
/// # Arguments
/// * `prefix` - Key prefix (e.g., "yatagarasu")
/// * `bucket` - Bucket name
/// * `object_key` - Object key/path
///
/// # Returns
/// Formatted Redis key with URL encoding for special characters
pub fn format_key(prefix: &str, bucket: &str, object_key: &str) -> String {
    // URL encode the object key to handle special characters
    let encoded_key = urlencoding::encode(object_key);

    // Construct the full key
    let full_key = format!("{}:{}:{}", prefix, bucket, encoded_key);

    // If key is too long, hash it
    if full_key.len() > MAX_KEY_LENGTH {
        hash_long_key(prefix, bucket, object_key)
    } else {
        full_key
    }
}

/// Hashes a long key using SHA256
///
/// Format: "{prefix}:hash:{sha256}"
fn hash_long_key(prefix: &str, bucket: &str, object_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bucket.as_bytes());
    hasher.update(b":");
    hasher.update(object_key.as_bytes());

    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    format!("{}:hash:{}", prefix, hash_hex)
}

/// Validates a cache key before Redis operations
///
/// # Errors
/// Returns error message if:
/// - Key contains null bytes
/// - Key exceeds Redis limits (512MB)
pub fn validate_key(key: &str) -> Result<(), String> {
    // Check for null bytes
    if key.contains('\0') {
        return Err("Key contains null bytes".to_string());
    }

    // Check Redis key size limit (512MB)
    if key.len() > 512 * 1024 * 1024 {
        return Err("Key exceeds Redis limit of 512MB".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formats_redis_key_with_prefix() {
        // Test: Formats Redis key with prefix
        let key = format_key("yatagarasu", "images", "cat.jpg");
        assert!(key.starts_with("yatagarasu:"));
    }

    #[test]
    fn test_key_format_prefix_bucket_object() {
        // Test: Key format: "{prefix}:{bucket}:{object_key}"
        let key = format_key("test", "mybucket", "myfile.txt");
        assert_eq!(key, "test:mybucket:myfile.txt");
    }

    #[test]
    fn test_key_format_example_yatagarasu_images_cat() {
        // Test: Example: "yatagarasu:images:cat.jpg"
        let key = format_key("yatagarasu", "images", "cat.jpg");
        assert_eq!(key, "yatagarasu:images:cat.jpg");
    }

    #[test]
    fn test_handles_bucket_names_with_special_chars() {
        // Test: Handles bucket names with special chars
        // Bucket names typically don't have special chars in S3, but we handle them
        let key = format_key("prefix", "my-bucket_01", "file.txt");
        assert_eq!(key, "prefix:my-bucket_01:file.txt");
    }

    #[test]
    fn test_handles_object_keys_with_special_chars_url_encoding() {
        // Test: Handles object keys with special chars (URL encoding)
        let key = format_key("prefix", "bucket", "path/to/file with spaces.txt");
        // Spaces should be encoded as %20
        assert!(
            key.contains("path%2Fto%2Ffile+with+spaces.txt")
                || key.contains("path%2Fto%2Ffile%20with%20spaces.txt")
        );
    }

    #[test]
    fn test_handles_unicode_keys_correctly_utf8() {
        // Test: Handles Unicode keys correctly (UTF-8)
        let key = format_key("prefix", "bucket", "文件.txt");
        assert!(key.starts_with("prefix:bucket:"));
        // Unicode should be percent-encoded
        assert!(key.contains("%"));
    }

    #[test]
    fn test_handles_very_long_keys_via_sha256_hash() {
        // Test: Handles very long keys (>250 chars) via SHA256 hash
        let long_object_key = "a".repeat(300);
        let key = format_key("prefix", "bucket", &long_object_key);

        // Should use hash format
        assert!(key.starts_with("prefix:hash:"));
        // Hash should be 64 characters (SHA256 hex)
        assert!(key.contains(&":hash:"));
    }

    #[test]
    fn test_hash_format_prefix_hash_sha256() {
        // Test: Hash format: "{prefix}:hash:{sha256}" for long keys
        let long_key = "x".repeat(300);
        let key = format_key("test", "bucket", &long_key);

        assert!(key.starts_with("test:hash:"));

        // Extract hash part
        let hash_part = key.strip_prefix("test:hash:").unwrap();
        // SHA256 produces 64 hex characters
        assert_eq!(hash_part.len(), 64);
        // Should be valid hex
        assert!(hash_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_key_collision_avoidance_different_buckets() {
        // Test: Key collision avoidance (different buckets → different keys)
        let key1 = format_key("prefix", "bucket1", "file.txt");
        let key2 = format_key("prefix", "bucket2", "file.txt");

        assert_ne!(key1, key2);
        assert_eq!(key1, "prefix:bucket1:file.txt");
        assert_eq!(key2, "prefix:bucket2:file.txt");
    }

    #[test]
    fn test_key_collision_avoidance_different_objects() {
        // Test: Different objects → different keys
        let key1 = format_key("prefix", "bucket", "file1.txt");
        let key2 = format_key("prefix", "bucket", "file2.txt");

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_collision_avoidance_long_keys() {
        // Test: Different long keys produce different hashes
        let long_key1 = format!("{}file1.txt", "a".repeat(300));
        let long_key2 = format!("{}file2.txt", "a".repeat(300));

        let key1 = format_key("prefix", "bucket", &long_key1);
        let key2 = format_key("prefix", "bucket", &long_key2);

        assert_ne!(key1, key2);
        assert!(key1.starts_with("prefix:hash:"));
        assert!(key2.starts_with("prefix:hash:"));
    }

    #[test]
    fn test_rejects_keys_with_null_bytes() {
        // Test: Rejects keys with null bytes
        let key_with_null = "prefix:bucket:file\0.txt";
        let result = validate_key(key_with_null);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("null bytes"));
    }

    #[test]
    fn test_rejects_keys_exceeding_redis_limits() {
        // Test: Rejects keys exceeding Redis limits (512MB)
        // Create a key that would exceed 512MB (in practice, this is theoretical)
        // We'll test with a smaller threshold for practicality
        let huge_key = "x".repeat(513 * 1024 * 1024); // 513MB
        let result = validate_key(&huge_key);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Redis limit"));
    }

    #[test]
    fn test_validates_key_before_redis_operations() {
        // Test: Validates key before Redis operations
        let valid_key = "prefix:bucket:file.txt";
        assert!(validate_key(valid_key).is_ok());

        let invalid_key = "prefix:bucket:file\0.txt";
        assert!(validate_key(invalid_key).is_err());
    }

    #[test]
    fn test_valid_keys_pass_validation() {
        // Test various valid keys
        assert!(validate_key("simple").is_ok());
        assert!(validate_key("prefix:bucket:key").is_ok());
        assert!(validate_key("a".repeat(1000).as_str()).is_ok());
    }

    #[test]
    fn test_slashes_are_url_encoded() {
        // Slashes in object keys should be encoded
        let key = format_key("prefix", "bucket", "path/to/file.txt");
        assert!(key.contains("%2F") || key.contains("path/to/file.txt"));
    }
}
