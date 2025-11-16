//! Utility functions for disk cache

use crate::cache::CacheKey;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Convert a CacheKey to a SHA256 hash for use as a filename
pub fn key_to_hash(key: &CacheKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.bucket.as_bytes());
    hasher.update(b":");
    hasher.update(key.object_key.as_bytes());
    if let Some(etag) = &key.etag {
        hasher.update(b":");
        hasher.update(etag.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Generate file paths for data and metadata files
#[allow(dead_code)] // Will be used in Phase 28.9 (Cache Trait Implementation)
pub fn generate_paths(cache_dir: &Path, hash: &str) -> (PathBuf, PathBuf) {
    let entries_dir = cache_dir.join("entries");
    let data_path = entries_dir.join(format!("{}.data", hash));
    let meta_path = entries_dir.join(format!("{}.meta", hash));
    (data_path, meta_path)
}

/// Generate file path for a cache entry (data or metadata)
#[allow(dead_code)] // Used by Index::validate(), will be called in Phase 28.8
pub fn cache_key_to_file_path(entries_dir: &Path, key: &CacheKey, is_metadata: bool) -> PathBuf {
    let hash = key_to_hash(key);
    let ext = if is_metadata { "meta" } else { "data" };
    entries_dir.join(format!("{}.{}", hash, ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_hash_deterministic() {
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
        assert_eq!(key_to_hash(&key1), key_to_hash(&key2));
    }

    #[test]
    fn test_key_to_hash_different_keys() {
        let key1 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key1".to_string(),
            etag: None,
        };
        let key2 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key2".to_string(),
            etag: None,
        };
        assert_ne!(key_to_hash(&key1), key_to_hash(&key2));
    }
}
