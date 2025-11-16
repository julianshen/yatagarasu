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
        let result = tokio::spawn(async { 42 }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_sha2_cache_key_hashing_available() {
        // Verify sha2 crate is available for cache key hashing
        use sha2::{Digest, Sha256};

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
        use serde::{Deserialize, Serialize};

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
        assert!(
            true,
            "Build succeeds without tokio-uring on non-Linux platforms"
        );
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
        use bytes::Bytes;
        use parking_lot::RwLock;
        use sha2::{Digest, Sha256};
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
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "test".to_string(),
            etag: None,
        };

        let metadata = EntryMetadata::new(
            key,
            PathBuf::from("/cache/entries/abc123.data"),
            1024,
            1000000,
            2000000,
        );

        assert_eq!(
            metadata.file_path,
            PathBuf::from("/cache/entries/abc123.data")
        );
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.created_at, 1000000);
        assert_eq!(metadata.expires_at, 2000000);
        assert_eq!(metadata.last_accessed_at, 1000000); // Should equal created_at initially
    }

    #[test]
    fn test_entry_metadata_serialization() {
        // Verify EntryMetadata serializes to JSON
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "test".to_string(),
            etag: None,
        };

        let metadata = EntryMetadata::new(
            key,
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
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "test".to_string(),
            etag: None,
        };

        let metadata = EntryMetadata::new(
            key,
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
        use super::super::index::CacheIndex;
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;

        let index = CacheIndex::new();

        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: None,
        };

        let metadata = EntryMetadata::new(
            key.clone(),
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
        use super::super::index::CacheIndex;
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;

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
            key1.clone(),
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
            key2.clone(),
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
        use super::super::utils::{generate_paths, key_to_hash};
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
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use bytes::Bytes;
        use std::path::Path;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test.dat");

        // Test write_file_atomic (verifies method exists and is async)
        let data = Bytes::from("test data");
        backend
            .write_file_atomic(&test_path, data.clone())
            .await
            .unwrap();

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
        use super::super::backend::DiskBackend;
        use super::super::mock_backend::MockDiskBackend;
        use bytes::Bytes;
        use std::path::Path;

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
        use super::super::backend::DiskBackend;
        use super::super::mock_backend::MockDiskBackend;
        use bytes::Bytes;
        use std::path::Path;

        let backend = MockDiskBackend::new();

        // Write multiple files
        backend
            .write_file_atomic(Path::new("/file1.dat"), Bytes::from("data1"))
            .await
            .unwrap();
        backend
            .write_file_atomic(Path::new("/file2.dat"), Bytes::from("data2"))
            .await
            .unwrap();
        backend
            .write_file_atomic(Path::new("/file3.dat"), Bytes::from("data3"))
            .await
            .unwrap();

        // Verify count
        assert_eq!(backend.file_count(), 3);

        // Clear and verify
        backend.clear();
        assert_eq!(backend.file_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_backend_can_read_what_was_written() {
        // Verify MockDiskBackend can read what was written
        use super::super::backend::DiskBackend;
        use super::super::mock_backend::MockDiskBackend;
        use bytes::Bytes;
        use std::path::Path;

        let backend = MockDiskBackend::new();
        let path = Path::new("/test/data.bin");
        let original_data = Bytes::from("Hello, World! 🦀");

        // Write data
        backend
            .write_file_atomic(path, original_data.clone())
            .await
            .unwrap();

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
        use super::super::backend::DiskBackend;
        use super::super::error::DiskCacheError;
        use super::super::mock_backend::MockDiskBackend;
        use bytes::Bytes;
        use std::path::Path;

        let backend = MockDiskBackend::new();

        // Enable storage full simulation
        backend.set_storage_full(true);

        // Try to write - should fail with StorageFull
        let result = backend
            .write_file_atomic(Path::new("/test.dat"), Bytes::from("data"))
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DiskCacheError::StorageFull));
    }

    #[tokio::test]
    async fn test_mock_backend_simulates_permission_denied() {
        // Verify MockDiskBackend can simulate permission denied errors
        use super::super::backend::DiskBackend;
        use super::super::error::DiskCacheError;
        use super::super::mock_backend::MockDiskBackend;
        use bytes::Bytes;
        use std::path::Path;

        let backend = MockDiskBackend::new();
        let path = Path::new("/test.dat");

        // First write some data (permission OK)
        backend
            .write_file_atomic(path, Bytes::from("data"))
            .await
            .unwrap();

        // Enable permission denied simulation
        backend.set_permission_denied(true);

        // Try to read - should fail with permission denied
        let read_result = backend.read_file(path).await;
        assert!(read_result.is_err());
        assert!(matches!(read_result.unwrap_err(), DiskCacheError::Io(_)));

        // Try to write - should fail with permission denied
        let write_result = backend
            .write_file_atomic(path, Bytes::from("new data"))
            .await;
        assert!(write_result.is_err());
        assert!(matches!(write_result.unwrap_err(), DiskCacheError::Io(_)));
    }

    // Phase 28.3: Cache Key Mapping & File Structure

    #[tokio::test]
    async fn test_creates_entries_subdirectory() {
        // Verify backend creates entries/ subdirectory
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::utils::generate_paths;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let hash = "abc123def456";

        let (data_path, _) = generate_paths(temp_dir.path(), hash);

        // Write file - should create entries/ subdirectory
        backend
            .write_file_atomic(&data_path, Bytes::from("test"))
            .await
            .unwrap();

        // Verify entries/ subdirectory was created
        let entries_dir = temp_dir.path().join("entries");
        assert!(entries_dir.exists());
        assert!(entries_dir.is_dir());

        // Verify file exists in subdirectory
        assert!(data_path.exists());
    }

    #[tokio::test]
    async fn test_data_file_stores_raw_binary() {
        // Verify .data file stores raw binary data
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::utils::generate_paths;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let hash = "test_binary_hash";

        let (data_path, _) = generate_paths(temp_dir.path(), hash);

        // Create binary data (including non-UTF8 bytes)
        let binary_data = Bytes::from(vec![0x00, 0xFF, 0x42, 0xCA, 0xFE, 0xBA, 0xBE]);

        // Write binary data
        backend
            .write_file_atomic(&data_path, binary_data.clone())
            .await
            .unwrap();

        // Read back and verify exact binary match
        let read_data = backend.read_file(&data_path).await.unwrap();
        assert_eq!(read_data, binary_data);

        // Verify file size matches
        let size = backend.file_size(&data_path).await.unwrap();
        assert_eq!(size, binary_data.len() as u64);
    }

    #[tokio::test]
    async fn test_metadata_file_stores_json() {
        // Verify .meta file stores JSON metadata
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::generate_paths;
        use bytes::Bytes;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let hash = "test_meta_hash";

        let (_, meta_path) = generate_paths(temp_dir.path(), hash);

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "test".to_string(),
            etag: None,
        };

        // Create metadata
        let metadata = EntryMetadata::new(
            key,
            PathBuf::from("/cache/entries/test.data"),
            4096,
            1234567890,
            9999999999,
        );

        // Serialize to JSON and write
        let json = serde_json::to_string(&metadata).unwrap();
        backend
            .write_file_atomic(&meta_path, Bytes::from(json.clone()))
            .await
            .unwrap();

        // Read back and verify
        let read_json = backend.read_file(&meta_path).await.unwrap();
        let read_str = String::from_utf8(read_json.to_vec()).unwrap();

        // Parse JSON to verify it's valid
        let parsed: EntryMetadata = serde_json::from_str(&read_str).unwrap();
        assert_eq!(parsed.size_bytes, 4096);
        assert_eq!(parsed.created_at, 1234567890);
        assert_eq!(parsed.expires_at, 9999999999);
    }

    #[tokio::test]
    async fn test_files_created_atomically() {
        // Verify write_file_atomic creates files atomically
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("entries").join("atomic.data");

        // Write data atomically
        let data = Bytes::from("atomic write test");
        backend
            .write_file_atomic(&file_path, data.clone())
            .await
            .unwrap();

        // File should exist after atomic write
        assert!(file_path.exists());

        // Verify no .tmp file remains (atomic write cleaned up)
        let tmp_path = file_path.with_extension("tmp");
        assert!(!tmp_path.exists());

        // Read back to verify data integrity
        let read_data = backend.read_file(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_both_data_and_meta_files_created() {
        // Verify both .data and .meta files can be created for same entry
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::generate_paths;
        use bytes::Bytes;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let hash = "complete_entry_hash";

        let (data_path, meta_path) = generate_paths(temp_dir.path(), hash);

        // Write data file
        let data = Bytes::from("entry data content");
        backend
            .write_file_atomic(&data_path, data.clone())
            .await
            .unwrap();

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: "test".to_string(),
            etag: None,
        };

        // Write metadata file
        let metadata =
            EntryMetadata::new(key, data_path.clone(), data.len() as u64, 1000000, 2000000);
        let meta_json = serde_json::to_string(&metadata).unwrap();
        backend
            .write_file_atomic(&meta_path, Bytes::from(meta_json))
            .await
            .unwrap();

        // Verify both files exist
        assert!(data_path.exists());
        assert!(meta_path.exists());

        // Verify both are in entries/ subdirectory
        assert_eq!(data_path.parent().unwrap().file_name().unwrap(), "entries");
        assert_eq!(meta_path.parent().unwrap().file_name().unwrap(), "entries");

        // Verify extensions
        assert_eq!(data_path.extension().unwrap(), "data");
        assert_eq!(meta_path.extension().unwrap(), "meta");
    }

    // Phase 28.4: Index Persistence
    // Note: In-Memory Index tests already completed in Phase 28.1

    #[tokio::test]
    async fn test_index_can_save_to_json() {
        // Verify index can be saved to index.json
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let index = CacheIndex::new();
        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("index.json");

        // Add some entries to the index
        let key1 = CacheKey {
            bucket: "bucket1".to_string(),
            object_key: "key1".to_string(),
            etag: None,
        };
        let metadata1 = EntryMetadata::new(
            key1.clone(),
            PathBuf::from("/cache/1.data"),
            1024,
            1000,
            2000,
        );
        index.insert(key1, metadata1);

        // Save index to JSON
        index.save_to_file(&index_path, &backend).await.unwrap();

        // Verify file was created
        assert!(index_path.exists());

        // Verify file contains valid JSON
        let content = backend.read_file(&index_path).await.unwrap();
        let json_str = String::from_utf8(content.to_vec()).unwrap();
        assert!(json_str.contains("bucket1"));
        assert!(json_str.contains("key1"));
    }

    #[tokio::test]
    async fn test_index_can_load_from_json() {
        // Verify index can be loaded from index.json
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use crate::cache::CacheKey;
        use std::path::PathBuf;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("index.json");

        // Create and save index
        let index1 = CacheIndex::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: Some("etag123".to_string()),
        };
        let metadata = EntryMetadata::new(
            key.clone(),
            PathBuf::from("/cache/test.data"),
            2048,
            5000,
            10000,
        );
        index1.insert(key.clone(), metadata);
        index1.save_to_file(&index_path, &backend).await.unwrap();

        // Load into new index
        let index2 = CacheIndex::load_from_file(&index_path, &backend)
            .await
            .unwrap();

        // Verify loaded data matches
        let loaded_metadata = index2.get(&key).unwrap();
        assert_eq!(loaded_metadata.size_bytes, 2048);
        assert_eq!(loaded_metadata.created_at, 5000);
        assert_eq!(loaded_metadata.expires_at, 10000);
        assert_eq!(index2.total_size(), 2048);
        assert_eq!(index2.entry_count(), 1);
    }

    #[tokio::test]
    async fn test_index_handles_missing_file() {
        // Verify index starts empty when file doesn't exist
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let missing_path = temp_dir.path().join("does_not_exist.json");

        // Load from missing file - should return empty index
        let index = CacheIndex::load_from_file(&missing_path, &backend)
            .await
            .unwrap();

        assert_eq!(index.entry_count(), 0);
        assert_eq!(index.total_size(), 0);
    }

    #[tokio::test]
    async fn test_index_handles_corrupted_json() {
        // Verify index handles corrupted JSON gracefully
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let corrupt_path = temp_dir.path().join("corrupt.json");

        // Write invalid JSON
        backend
            .write_file_atomic(&corrupt_path, Bytes::from("{invalid json}"))
            .await
            .unwrap();

        // Load from corrupted file - should return empty index and log error
        let index = CacheIndex::load_from_file(&corrupt_path, &backend)
            .await
            .unwrap();

        // Should start with empty index despite corruption
        assert_eq!(index.entry_count(), 0);
        assert_eq!(index.total_size(), 0);
    }

    // Phase 28.4: Index Validation & Repair
    #[tokio::test]
    async fn test_index_scans_entries_directory_on_startup() {
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::cache_key_to_file_path;
        use crate::cache::CacheKey;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let entries_dir = temp_dir.path().join("entries");
        std::fs::create_dir_all(&entries_dir).unwrap();

        // Create some cache files without index
        let key1 = CacheKey {
            bucket: "test".to_string(),
            object_key: "file1.txt".to_string(),
            etag: Some("etag1".to_string()),
        };
        let key2 = CacheKey {
            bucket: "test".to_string(),
            object_key: "file2.txt".to_string(),
            etag: Some("etag2".to_string()),
        };

        let data_path1 = cache_key_to_file_path(&entries_dir, &key1, false);
        let meta_path1 = cache_key_to_file_path(&entries_dir, &key1, true);
        let data_path2 = cache_key_to_file_path(&entries_dir, &key2, false);
        let meta_path2 = cache_key_to_file_path(&entries_dir, &key2, true);

        // Create data files
        backend
            .write_file_atomic(&data_path1, Bytes::from("data1"))
            .await
            .unwrap();
        backend
            .write_file_atomic(&data_path2, Bytes::from("data2"))
            .await
            .unwrap();

        // Create metadata files
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let metadata1 = EntryMetadata::new(key1.clone(), data_path1.clone(), 5, now, 0);
        let metadata2 = EntryMetadata::new(key2.clone(), data_path2.clone(), 5, now, 0);

        let meta_json1 = serde_json::to_string(&metadata1).unwrap();
        let meta_json2 = serde_json::to_string(&metadata2).unwrap();
        backend
            .write_file_atomic(&meta_path1, Bytes::from(meta_json1))
            .await
            .unwrap();
        backend
            .write_file_atomic(&meta_path2, Bytes::from(meta_json2))
            .await
            .unwrap();

        // Validate and repair - should discover these files
        let index = CacheIndex::new();
        index
            .validate_and_repair(&entries_dir, &backend)
            .await
            .unwrap();

        // Should have found both entries
        assert_eq!(index.entry_count(), 2);
        assert!(index.get(&key1).is_some());
        assert!(index.get(&key2).is_some());
    }

    #[tokio::test]
    async fn test_index_removes_orphaned_files() {
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::cache_key_to_file_path;
        use crate::cache::CacheKey;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let entries_dir = temp_dir.path().join("entries");
        std::fs::create_dir_all(&entries_dir).unwrap();

        let key1 = CacheKey {
            bucket: "test".to_string(),
            object_key: "valid.txt".to_string(),
            etag: Some("etag1".to_string()),
        };
        let key2 = CacheKey {
            bucket: "test".to_string(),
            object_key: "orphaned.txt".to_string(),
            etag: Some("etag2".to_string()),
        };

        // Create files for both keys
        let data_path1 = cache_key_to_file_path(&entries_dir, &key1, false);
        let meta_path1 = cache_key_to_file_path(&entries_dir, &key1, true);
        let data_path2 = cache_key_to_file_path(&entries_dir, &key2, false);
        let meta_path2 = cache_key_to_file_path(&entries_dir, &key2, true);

        backend
            .write_file_atomic(&data_path1, Bytes::from("data1"))
            .await
            .unwrap();
        backend
            .write_file_atomic(&data_path2, Bytes::from("data2"))
            .await
            .unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let metadata1 = EntryMetadata::new(key1.clone(), data_path1.clone(), 5, now, 0);
        let metadata2 = EntryMetadata::new(key2.clone(), data_path2.clone(), 5, now, 0);

        let meta_json1 = serde_json::to_string(&metadata1).unwrap();
        let meta_json2 = serde_json::to_string(&metadata2).unwrap();
        backend
            .write_file_atomic(&meta_path1, Bytes::from(meta_json1))
            .await
            .unwrap();
        backend
            .write_file_atomic(&meta_path2, Bytes::from(meta_json2))
            .await
            .unwrap();

        // Create index with only key1
        let index = CacheIndex::new();
        index.insert(key1.clone(), metadata1.clone());

        // Validate and repair - should remove orphaned key2 files
        index
            .validate_and_repair(&entries_dir, &backend)
            .await
            .unwrap();

        // key2 files should be deleted
        assert!(!data_path2.exists());
        assert!(!meta_path2.exists());

        // key1 should still exist
        assert_eq!(index.entry_count(), 1);
        assert!(index.get(&key1).is_some());
    }

    #[tokio::test]
    async fn test_index_removes_entries_without_files() {
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::cache_key_to_file_path;
        use crate::cache::CacheKey;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let entries_dir = temp_dir.path().join("entries");
        std::fs::create_dir_all(&entries_dir).unwrap();

        let key1 = CacheKey {
            bucket: "test".to_string(),
            object_key: "has_files.txt".to_string(),
            etag: Some("etag1".to_string()),
        };
        let key2 = CacheKey {
            bucket: "test".to_string(),
            object_key: "missing_files.txt".to_string(),
            etag: Some("etag2".to_string()),
        };

        // Create files only for key1
        let data_path1 = cache_key_to_file_path(&entries_dir, &key1, false);
        let meta_path1 = cache_key_to_file_path(&entries_dir, &key1, true);

        backend
            .write_file_atomic(&data_path1, Bytes::from("data1"))
            .await
            .unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let metadata = EntryMetadata::new(key1.clone(), data_path1.clone(), 5, now, 0);
        let meta_json = serde_json::to_string(&metadata).unwrap();
        backend
            .write_file_atomic(&meta_path1, Bytes::from(meta_json))
            .await
            .unwrap();

        // Create index with both keys (but key2 has no files)
        let index = CacheIndex::new();
        let metadata2 = EntryMetadata::new(key2.clone(), PathBuf::from("/nonexistent"), 5, now, 0);
        index.insert(key1.clone(), metadata.clone());
        index.insert(key2.clone(), metadata2.clone());

        assert_eq!(index.entry_count(), 2);

        // Validate and repair - should remove key2 from index
        index
            .validate_and_repair(&entries_dir, &backend)
            .await
            .unwrap();

        // key2 should be removed from index
        assert_eq!(index.entry_count(), 1);
        assert!(index.get(&key1).is_some());
        assert!(index.get(&key2).is_none());
    }

    #[tokio::test]
    async fn test_index_recalculates_total_size_from_files() {
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::cache_key_to_file_path;
        use crate::cache::CacheKey;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let entries_dir = temp_dir.path().join("entries");
        std::fs::create_dir_all(&entries_dir).unwrap();

        let key1 = CacheKey {
            bucket: "test".to_string(),
            object_key: "file1.txt".to_string(),
            etag: Some("etag1".to_string()),
        };

        // Create file with specific size
        let data = vec![0u8; 12345]; // 12345 bytes
        let data_path = cache_key_to_file_path(&entries_dir, &key1, false);
        let meta_path = cache_key_to_file_path(&entries_dir, &key1, true);

        backend
            .write_file_atomic(&data_path, Bytes::from(data))
            .await
            .unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create metadata with WRONG size
        let wrong_metadata = EntryMetadata::new(key1.clone(), data_path.clone(), 999, now, 0); // Wrong size!
        let meta_json = serde_json::to_string(&wrong_metadata).unwrap();
        backend
            .write_file_atomic(&meta_path, Bytes::from(meta_json))
            .await
            .unwrap();

        // Create index with wrong size
        let index = CacheIndex::new();
        index.insert(key1.clone(), wrong_metadata.clone());

        assert_eq!(index.total_size(), 999);

        // Validate and repair - should fix the size
        index
            .validate_and_repair(&entries_dir, &backend)
            .await
            .unwrap();

        // Size should be corrected
        assert_eq!(index.total_size(), 12345);

        // Metadata should be updated
        let updated = index.get(&key1).unwrap();
        assert_eq!(updated.size_bytes, 12345);
    }

    #[tokio::test]
    async fn test_index_removes_expired_entries() {
        use super::super::backend::DiskBackend;
        use super::super::index::CacheIndex;
        use super::super::tokio_backend::TokioFsBackend;
        use super::super::types::EntryMetadata;
        use super::super::utils::cache_key_to_file_path;
        use crate::cache::CacheKey;
        use std::time::{Duration, SystemTime};
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let entries_dir = temp_dir.path().join("entries");
        std::fs::create_dir_all(&entries_dir).unwrap();

        let key1 = CacheKey {
            bucket: "test".to_string(),
            object_key: "expired.txt".to_string(),
            etag: Some("etag1".to_string()),
        };
        let key2 = CacheKey {
            bucket: "test".to_string(),
            object_key: "valid.txt".to_string(),
            etag: Some("etag2".to_string()),
        };

        // Create files for both keys
        let data_path1 = cache_key_to_file_path(&entries_dir, &key1, false);
        let meta_path1 = cache_key_to_file_path(&entries_dir, &key1, true);
        let data_path2 = cache_key_to_file_path(&entries_dir, &key2, false);
        let meta_path2 = cache_key_to_file_path(&entries_dir, &key2, true);

        backend
            .write_file_atomic(&data_path1, Bytes::from("data1"))
            .await
            .unwrap();
        backend
            .write_file_atomic(&data_path2, Bytes::from("data2"))
            .await
            .unwrap();

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // key1 expired 1 hour ago
        let expired_time = now - 3600;
        let expired_metadata =
            EntryMetadata::new(key1.clone(), data_path1.clone(), 5, now, expired_time);

        // key2 expires in 1 hour (still valid)
        let future_time = now + 3600;
        let valid_metadata =
            EntryMetadata::new(key2.clone(), data_path2.clone(), 5, now, future_time);

        let meta_json1 = serde_json::to_string(&expired_metadata).unwrap();
        let meta_json2 = serde_json::to_string(&valid_metadata).unwrap();
        backend
            .write_file_atomic(&meta_path1, Bytes::from(meta_json1))
            .await
            .unwrap();
        backend
            .write_file_atomic(&meta_path2, Bytes::from(meta_json2))
            .await
            .unwrap();

        // Create index with both entries
        let index = CacheIndex::new();
        index.insert(key1.clone(), expired_metadata);
        index.insert(key2.clone(), valid_metadata);

        assert_eq!(index.entry_count(), 2);

        // Validate and repair - should remove expired entry
        index
            .validate_and_repair(&entries_dir, &backend)
            .await
            .unwrap();

        // key1 (expired) should be removed
        assert_eq!(index.entry_count(), 1);
        assert!(index.get(&key1).is_none());
        assert!(index.get(&key2).is_some());

        // Expired files should be deleted
        assert!(!data_path1.exists());
        assert!(!meta_path1.exists());
    }

    // Phase 28.5: tokio::fs Backend Implementation

    #[test]
    fn test_can_create_tokio_fs_backend() {
        // Verify TokioFsBackend can be created
        use super::super::tokio_backend::TokioFsBackend;

        let backend = TokioFsBackend::new();

        // Backend should be created successfully
        // This is a simple smoke test to ensure the struct can be instantiated
        drop(backend);
    }

    #[test]
    fn test_tokio_fs_backend_is_send_sync() {
        // Verify TokioFsBackend implements Send + Sync (required for async)
        use super::super::tokio_backend::TokioFsBackend;

        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<TokioFsBackend>();
        assert_sync::<TokioFsBackend>();
    }

    #[tokio::test]
    async fn test_read_file_returns_error_if_not_exists() {
        // Verify read_file() returns error when file doesn't exist
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use std::path::Path;

        let backend = TokioFsBackend::new();
        let nonexistent_path = Path::new("/nonexistent/file/that/does/not/exist.dat");

        // Should return error for nonexistent file
        let result = backend.read_file(nonexistent_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_works_with_various_file_sizes() {
        // Verify TokioFsBackend works with files from 0B to 100MB
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();

        // Test various file sizes
        let test_sizes = vec![
            (0, "0B"),
            (1024, "1KB"),
            (1024 * 1024, "1MB"),
            (10 * 1024 * 1024, "10MB"),
            (100 * 1024 * 1024, "100MB"),
        ];

        for (size, label) in test_sizes {
            let file_path = temp_dir.path().join(format!("test_{}.dat", label));

            // Create data of specified size
            let data = Bytes::from(vec![0x42; size]);

            // Write file
            backend
                .write_file_atomic(&file_path, data.clone())
                .await
                .unwrap();

            // Verify file size
            let file_size = backend.file_size(&file_path).await.unwrap();
            assert_eq!(file_size, size as u64, "File size mismatch for {}", label);

            // Read back and verify
            let read_data = backend.read_file(&file_path).await.unwrap();
            assert_eq!(
                read_data.len(),
                size,
                "Read data length mismatch for {}",
                label
            );
            assert_eq!(read_data, data, "Data content mismatch for {}", label);

            // Clean up
            backend.delete_file(&file_path).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_write_file_atomic_uses_tmp_file() {
        // Verify write_file_atomic() creates .tmp file during write
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use bytes::Bytes;
        use tempfile::TempDir;

        let backend = TokioFsBackend::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.dat");

        // Write file
        let data = Bytes::from("test data");
        backend
            .write_file_atomic(&file_path, data.clone())
            .await
            .unwrap();

        // Final file should exist
        assert!(file_path.exists());

        // Temp file should NOT exist after successful write
        let tmp_path = file_path.with_extension("tmp");
        assert!(
            !tmp_path.exists(),
            "Temp file should be cleaned up after write"
        );

        // Read back to verify data
        let read_data = backend.read_file(&file_path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_cleans_up_temp_file_on_error() {
        // Verify temp file cleanup happens even when write fails
        // This test simulates a failure scenario by trying to write to a read-only location
        use super::super::backend::DiskBackend;
        use super::super::tokio_backend::TokioFsBackend;
        use bytes::Bytes;
        use std::path::Path;

        let backend = TokioFsBackend::new();

        // Try to write to a path that will fail (e.g., root directory without permissions)
        // Note: This test behavior depends on OS permissions
        // On most systems, regular users can't write to /
        let invalid_path = Path::new("/test_file_that_cannot_be_created.dat");
        let data = Bytes::from("test data");

        // This should fail
        let result = backend.write_file_atomic(invalid_path, data).await;
        assert!(result.is_err(), "Write to invalid path should fail");

        // Verify temp file doesn't exist (cleaned up on error)
        let tmp_path = invalid_path.with_extension("tmp");
        assert!(
            !tmp_path.exists(),
            "Temp file should be cleaned up on error"
        );
    }
}
