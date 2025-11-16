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

    // Phase 28.2: Backend Trait Definition

    #[test]
    fn test_disk_backend_trait_defined() {
        // Verify DiskBackend trait exists and has expected structure
        // This is a compile-time test - the trait must exist for this to compile
        use super::super::backend::DiskBackend;
        use std::sync::Arc;

        // Verify we can reference the trait
        fn _accept_backend(_backend: Arc<dyn DiskBackend>) {
            // This function verifies DiskBackend is object-safe
        }

        assert!(true, "DiskBackend trait is defined and object-safe");
    }

    #[test]
    fn test_disk_backend_is_send_sync() {
        // Verify DiskBackend trait is Send + Sync
        use super::super::backend::DiskBackend;

        fn _assert_send<T: Send>() {}
        fn _assert_sync<T: Sync>() {}

        // These will only compile if DiskBackend requires Send + Sync
        _assert_send::<Box<dyn DiskBackend>>();
        _assert_sync::<Box<dyn DiskBackend>>();

        assert!(true, "DiskBackend is Send + Sync");
    }

    #[tokio::test]
    async fn test_disk_backend_trait_has_required_methods() {
        // Verify DiskBackend trait has all required async methods
        // We'll use the TokioFsBackend to verify the trait methods exist
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::backend::DiskBackend;
        use std::path::Path;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test.dat");

        // Test write_file_atomic (verifies method exists and is async)
        let data = Bytes::from("test data");
        backend.write_file_atomic(&test_path, data.clone()).await.unwrap();

        // Test file_size (verifies method exists and is async)
        let size = backend.file_size(&test_path).await.unwrap();
        assert_eq!(size, 9); // "test data" is 9 bytes

        // Test read_file (verifies method exists and is async)
        let read_data = backend.read_file(&test_path).await.unwrap();
        assert_eq!(read_data, data);

        // Test read_dir (verifies method exists and is async)
        let entries = backend.read_dir(temp_dir.path()).await.unwrap();
        assert_eq!(entries.len(), 1);

        // Test create_dir_all (verifies method exists and is async)
        let subdir = temp_dir.path().join("subdir");
        backend.create_dir_all(&subdir).await.unwrap();
        assert!(subdir.exists());

        // Test delete_file (verifies method exists and is async)
        backend.delete_file(&test_path).await.unwrap();
        assert!(!test_path.exists());
    }

    #[test]
    fn test_can_create_trait_object() {
        // Verify we can create Arc<dyn DiskBackend> trait objects
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use std::sync::Arc;

        let backend = TokioFsBackend::new();
        let trait_object: Arc<dyn DiskBackend> = Arc::new(backend);

        // Verify the trait object can be cloned (Arc)
        let _cloned = trait_object.clone();

        assert!(true, "Can create Arc<dyn DiskBackend> trait objects");
    }

    // Phase 28.2: MockDiskBackend (for testing)

    #[test]
    fn test_can_create_mock_backend() {
        // Verify MockDiskBackend can be created
        use super::super::mock_backend::MockDiskBackend;

        let backend = MockDiskBackend::new();

        // Verify initial state
        assert_eq!(backend.file_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_backend_implements_disk_backend() {
        // Verify MockDiskBackend implements DiskBackend trait
        use super::super::mock_backend::MockDiskBackend;
        use super::super::backend::DiskBackend;
        use std::path::Path;
        use bytes::Bytes;

        let backend = MockDiskBackend::new();

        // Use backend through trait interface
        let path = Path::new("/test/file.dat");
        let data = Bytes::from("test data");

        // Write through trait
        backend.write_file_atomic(path, data.clone()).await.unwrap();

        // Read through trait
        let read_data = backend.read_file(path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_mock_backend_stores_in_memory() {
        // Verify MockDiskBackend stores files in HashMap (in-memory)
        use super::super::mock_backend::MockDiskBackend;
        use super::super::backend::DiskBackend;
        use std::path::Path;
        use bytes::Bytes;

        let backend = MockDiskBackend::new();

        // Write multiple files
        backend.write_file_atomic(Path::new("/file1.dat"), Bytes::from("data1")).await.unwrap();
        backend.write_file_atomic(Path::new("/file2.dat"), Bytes::from("data2")).await.unwrap();
        backend.write_file_atomic(Path::new("/file3.dat"), Bytes::from("data3")).await.unwrap();

        // Verify count
        assert_eq!(backend.file_count(), 3);

        // Clear and verify
        backend.clear();
        assert_eq!(backend.file_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_backend_can_read_what_was_written() {
        // Verify MockDiskBackend can read what was written
        use super::super::mock_backend::MockDiskBackend;
        use super::super::backend::DiskBackend;
        use std::path::Path;
        use bytes::Bytes;

        let backend = MockDiskBackend::new();
        let path = Path::new("/test/data.bin");
        let original_data = Bytes::from("Hello, World! 🦀");

        // Write data
        backend.write_file_atomic(path, original_data.clone()).await.unwrap();

        // Read back and verify
        let read_data = backend.read_file(path).await.unwrap();
        assert_eq!(read_data, original_data);

        // Verify file size
        let size = backend.file_size(path).await.unwrap();
        assert_eq!(size, original_data.len() as u64);
    }

    #[tokio::test]
    async fn test_mock_backend_simulates_storage_full() {
        // Verify MockDiskBackend can simulate storage full errors
        use super::super::mock_backend::MockDiskBackend;
        use super::super::backend::DiskBackend;
        use super::super::error::DiskCacheError;
        use std::path::Path;
        use bytes::Bytes;

        let backend = MockDiskBackend::new();

        // Enable storage full simulation
        backend.set_storage_full(true);

        // Try to write - should fail with StorageFull
        let result = backend.write_file_atomic(
            Path::new("/test.dat"),
            Bytes::from("data")
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DiskCacheError::StorageFull));
    }

    #[tokio::test]
    async fn test_mock_backend_simulates_permission_denied() {
        // Verify MockDiskBackend can simulate permission denied errors
        use super::super::mock_backend::MockDiskBackend;
        use super::super::backend::DiskBackend;
        use super::super::error::DiskCacheError;
        use std::path::Path;
        use bytes::Bytes;

        let backend = MockDiskBackend::new();
        let path = Path::new("/test.dat");

        // First write some data (permission OK)
        backend.write_file_atomic(path, Bytes::from("data")).await.unwrap();

        // Enable permission denied simulation
        backend.set_permission_denied(true);

        // Try to read - should fail with permission denied
        let read_result = backend.read_file(path).await;
        assert!(read_result.is_err());
        assert!(matches!(read_result.unwrap_err(), DiskCacheError::Io(_)));

        // Try to write - should fail with permission denied
        let write_result = backend.write_file_atomic(path, Bytes::from("new data")).await;
        assert!(write_result.is_err());
        assert!(matches!(write_result.unwrap_err(), DiskCacheError::Io(_)));
    }
}
