//! Tests for disk cache

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_module_compiles() {
        // Initial test to verify module structure compiles
        assert!(true);
    }

    // Phase 28.1.1: Dependencies Setup

    #[tokio::test]
    async fn test_tokio_async_runtime_available() {
        // Verify tokio async runtime is available and working
        let result = tokio::spawn(async {
            42
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_sha2_cache_key_hashing_available() {
        // Verify sha2 crate is available for cache key hashing
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(b"test data");
        let result = hasher.finalize();

        // SHA256 always produces 32 bytes
        assert_eq!(result.len(), 32);

        // Verify hashing is deterministic
        let mut hasher2 = Sha256::new();
        hasher2.update(b"test data");
        let result2 = hasher2.finalize();

        assert_eq!(result, result2);
    }

    #[test]
    fn test_serde_json_metadata_serialization_available() {
        // Verify serde/serde_json is available for metadata serialization
        use serde::{Serialize, Deserialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestMetadata {
            size: u64,
            timestamp: u64,
        }

        let metadata = TestMetadata {
            size: 1024,
            timestamp: 1234567890,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("1024"));
        assert!(json.contains("1234567890"));

        // Deserialize from JSON
        let deserialized: TestMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, metadata);
    }

    #[test]
    fn test_parking_lot_rwlock_available() {
        // Verify parking_lot is available for efficient thread-safe locking
        use parking_lot::RwLock;
        use std::sync::Arc;

        let lock = Arc::new(RwLock::new(42));

        // Test read lock
        {
            let read_guard = lock.read();
            assert_eq!(*read_guard, 42);
        }

        // Test write lock
        {
            let mut write_guard = lock.write();
            *write_guard = 100;
        }

        // Verify write succeeded
        {
            let read_guard = lock.read();
            assert_eq!(*read_guard, 100);
        }
    }

    // Phase 28.1.1: Platform-Specific Dependencies

    #[test]
    #[cfg(target_os = "linux")]
    fn test_tokio_uring_available_on_linux() {
        // Verify tokio-uring is available on Linux
        // This is a compile-time test - if it compiles, the dependency is available
        use tokio_uring;

        // Simply verify the module can be imported
        // Runtime tests will be added later when we implement the uring backend
        assert!(true, "tokio-uring is available on Linux");
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_tokio_uring_not_required_on_non_linux() {
        // Verify build works without tokio-uring on non-Linux platforms
        // This test simply verifies the module compiles without tokio-uring
        assert!(true, "Build succeeds without tokio-uring on non-Linux platforms");
    }

    #[test]
    fn test_tempfile_available_for_isolation() {
        // Verify tempfile is available for test isolation
        use tempfile::TempDir;

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Verify directory exists
        assert!(temp_path.exists());
        assert!(temp_path.is_dir());

        // Directory will be automatically cleaned up when temp_dir is dropped
    }

    #[test]
    fn test_all_imports_compile() {
        // Verify all core imports compile together on all platforms
        use sha2::{Sha256, Digest};
        use parking_lot::RwLock;
        use bytes::Bytes;
        use std::sync::Arc;

        // Create instances to verify they work together
        let _hasher = Sha256::new();
        let _lock = Arc::new(RwLock::new(0u64));
        let _data = Bytes::from("test");

        assert!(true, "All core imports compile successfully");
    }

    // Phase 28.1.1: Common Types

    #[test]
    fn test_entry_metadata_creation() {
        // Verify EntryMetadata struct can be created
        use std::path::PathBuf;
        use super::super::types::EntryMetadata;

        let metadata = EntryMetadata::new(
            PathBuf::from("/cache/entries/abc123.data"),
            1024,
            1000000,
            2000000,
        );

        assert_eq!(metadata.file_path, PathBuf::from("/cache/entries/abc123.data"));
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.created_at, 1000000);
        assert_eq!(metadata.expires_at, 2000000);
        assert_eq!(metadata.last_accessed_at, 1000000); // Should equal created_at initially
    }

    #[test]
    fn test_entry_metadata_serialization() {
        // Verify EntryMetadata serializes to JSON
        use std::path::PathBuf;
        use super::super::types::EntryMetadata;

        let metadata = EntryMetadata::new(
            PathBuf::from("/cache/test.data"),
            2048,
            1111111,
            3333333,
        );

        // Serialize to JSON
        let json = serde_json::to_string(&metadata).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("file_path"));
        assert!(json.contains("2048"));
        assert!(json.contains("1111111"));
        assert!(json.contains("3333333"));

        // Deserialize back
        let deserialized: EntryMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.size_bytes, 2048);
        assert_eq!(deserialized.created_at, 1111111);
    }

    #[test]
    fn test_entry_metadata_expiration() {
        // Verify EntryMetadata expiration check works
        use std::path::PathBuf;
        use super::super::types::EntryMetadata;

        let metadata = EntryMetadata::new(
            PathBuf::from("/cache/test.data"),
            1024,
            1000,
            2000, // Expires at 2000
        );

        // Not expired before expiration time
        assert!(!metadata.is_expired(1500));

        // Expired at expiration time
        assert!(metadata.is_expired(2000));

        // Expired after expiration time
        assert!(metadata.is_expired(3000));
    }

    #[test]
    fn test_cache_index_thread_safe_operations() {
        // Verify CacheIndex supports thread-safe operations
        use std::path::PathBuf;
        use super::super::index::CacheIndex;
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;

        let index = CacheIndex::new();

        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: None,
        };

        let metadata = EntryMetadata::new(
            PathBuf::from("/cache/test.data"),
            1024,
            1000,
            2000,
        );

        // Insert entry
        index.insert(key.clone(), metadata.clone());

        // Retrieve entry
        let retrieved = index.get(&key);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().size_bytes, 1024);

        // Remove entry
        let removed = index.remove(&key);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().size_bytes, 1024);

        // Entry should be gone
        assert!(index.get(&key).is_none());
    }

    #[test]
    fn test_cache_index_atomic_size_tracking() {
        // Verify CacheIndex tracks total size atomically
        use std::path::PathBuf;
        use super::super::index::CacheIndex;
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;

        let index = CacheIndex::new();

        // Initial size should be 0
        assert_eq!(index.total_size(), 0);

        // Insert first entry
        let key1 = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: "key1".to_string(),
            etag: None,
        };
        let metadata1 = EntryMetadata::new(
            PathBuf::from("/cache/1.data"),
            1024,
            1000,
            2000,
        );
        index.insert(key1.clone(), metadata1);

        assert_eq!(index.total_size(), 1024);
        assert_eq!(index.entry_count(), 1);

        // Insert second entry
        let key2 = CacheKey {
            bucket: "bucket2".to_string(),
            object_key: "key2".to_string(),
            etag: None,
        };
        let metadata2 = EntryMetadata::new(
            PathBuf::from("/cache/2.data"),
            2048,
            1000,
            2000,
        );
        index.insert(key2.clone(), metadata2);

        assert_eq!(index.total_size(), 3072); // 1024 + 2048
        assert_eq!(index.entry_count(), 2);

        // Remove first entry
        index.remove(&key1);

        assert_eq!(index.total_size(), 2048); // Only second entry remains
        assert_eq!(index.entry_count(), 1);

        // Clear all
        index.clear();

        assert_eq!(index.total_size(), 0);
        assert_eq!(index.entry_count(), 0);
    }

    #[test]
    fn test_disk_cache_error_variants() {
        // Verify DiskCacheError enum has all expected variants
        use super::super::error::DiskCacheError;
        use crate::cache::CacheError;
        use std::io;

        // Test Io variant
        let io_error = DiskCacheError::Io(io::Error::new(io::ErrorKind::NotFound, "test"));
        let cache_error: CacheError = io_error.into();
        assert!(matches!(cache_error, CacheError::IoError(_)));

        // Test Serialization variant
        let json_error = serde_json::from_str::<u64>("not a number").unwrap_err();
        let ser_error = DiskCacheError::Serialization(json_error);
        let cache_error: CacheError = ser_error.into();
        assert!(matches!(cache_error, CacheError::SerializationError(_)));

        // Test StorageFull variant
        let storage_error = DiskCacheError::StorageFull;
        let cache_error: CacheError = storage_error.into();
        assert!(matches!(cache_error, CacheError::StorageFull));

        // Test IndexCorrupted variant
        let index_error = DiskCacheError::IndexCorrupted;
        let cache_error: CacheError = index_error.into();
        assert!(matches!(cache_error, CacheError::IoError(_)));

        // Test BackendUnavailable variant
        let backend_error = DiskCacheError::BackendUnavailable;
        let cache_error: CacheError = backend_error.into();
        assert!(matches!(cache_error, CacheError::IoError(_)));
    }

    // Phase 28.1.1: File Path Utilities

    #[test]
    fn test_cache_key_to_sha256_hash() {
        // Verify CacheKey converts to SHA256 hash
        use super::super::utils::key_to_hash;
        use crate::cache::CacheKey;

        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "path/to/file.jpg".to_string(),
            etag: Some("abc123".to_string()),
        };

        let hash = key_to_hash(&key);

        // SHA256 hash should be 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Hash should be deterministic
        assert_eq!(hash, key_to_hash(&key));
    }

    #[test]
    fn test_generate_file_paths_with_subdirectory() {
        // Verify file paths use entries/ subdirectory
        use super::super::utils::generate_paths;
        use std::path::Path;

        let cache_dir = Path::new("/cache");
        let hash = "abc123def456";

        let (data_path, meta_path) = generate_paths(cache_dir, hash);

        // Both paths should be in entries/ subdirectory
        assert!(data_path.to_str().unwrap().contains("/entries/"));
        assert!(meta_path.to_str().unwrap().contains("/entries/"));

        // Verify .data and .meta extensions
        assert!(data_path.to_str().unwrap().ends_with(".data"));
        assert!(meta_path.to_str().unwrap().ends_with(".meta"));

        // Verify both contain the hash
        assert!(data_path.to_str().unwrap().contains(hash));
        assert!(meta_path.to_str().unwrap().contains(hash));
    }

    #[test]
    fn test_file_paths_different_for_different_keys() {
        // Verify different cache keys produce different file paths
        use super::super::utils::{key_to_hash, generate_paths};
        use crate::cache::CacheKey;
        use std::path::Path;

        let key1 = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: "key1".to_string(),
            etag: None,
        };

        let key2 = CacheKey {
            bucket: "bucket2".to_string(),
            object_key: "key2".to_string(),
            etag: None,
        };

        let hash1 = key_to_hash(&key1);
        let hash2 = key_to_hash(&key2);

        // Different keys should produce different hashes
        assert_ne!(hash1, hash2);

        let cache_dir = Path::new("/cache");
        let (data_path1, _) = generate_paths(cache_dir, &hash1);
        let (data_path2, _) = generate_paths(cache_dir, &hash2);

        // Different hashes should produce different paths
        assert_ne!(data_path1, data_path2);
    }

    #[test]
    fn test_hash_includes_bucket_and_key() {
        // Verify hash changes when bucket or key changes
        use super::super::utils::key_to_hash;
        use crate::cache::CacheKey;

        let key1 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let key2 = CacheKey {
            bucket: "different-bucket".to_string(),
            object_key: "key".to_string(),
            etag: None,
        };

        let key3 = CacheKey {
            bucket: "bucket".to_string(),
            object_key: "different-key".to_string(),
            etag: None,
        };

        let hash1 = key_to_hash(&key1);
        let hash2 = key_to_hash(&key2);
        let hash3 = key_to_hash(&key3);

        // All should be different
        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }
}
