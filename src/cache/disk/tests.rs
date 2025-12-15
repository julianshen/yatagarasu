//! Tests for disk cache

use super::*;

/// Helper to create EntryMetadata with default content_type, etag, last_modified.
/// Used by tests that don't care about these new fields.
#[cfg(test)]
fn test_entry_metadata(
    cache_key: crate::cache::CacheKey,
    file_path: std::path::PathBuf,
    size_bytes: u64,
    created_at: u64,
    expires_at: u64,
) -> types::EntryMetadata {
    types::EntryMetadata::new(
        cache_key,
        file_path,
        size_bytes,
        created_at,
        expires_at,
        "application/octet-stream".to_string(),
        "".to_string(),
        None,
    )
}

#[test]
fn test_module_compiles() {
    // Initial test to verify module structure compiles
    // Test passes if it compiles
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
#[cfg(not(target_os = "linux"))]
fn test_tokio_uring_not_required_on_non_linux() {
    // Verify build works without tokio-uring on non-Linux platforms
    // This test simply verifies the module compiles without tokio-uring
    // Test passes if it compiles - build succeeds without tokio-uring on non-Linux platforms
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

    // Test passes if it compiles - all core imports work together
}

// Phase 28.1.1: Common Types

#[test]
fn test_entry_metadata_creation() {
    // Verify EntryMetadata struct can be created
    use crate::cache::CacheKey;
    use std::path::PathBuf;

    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "test".to_string(),
        etag: None,
    };

    let metadata = test_entry_metadata(
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
    use super::types::EntryMetadata;
    use crate::cache::CacheKey;
    use std::path::PathBuf;

    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "test".to_string(),
        etag: None,
    };

    let metadata = test_entry_metadata(
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
    use crate::cache::CacheKey;
    use std::path::PathBuf;

    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "test".to_string(),
        etag: None,
    };

    let metadata = test_entry_metadata(
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
    use super::index::CacheIndex;
    use crate::cache::CacheKey;
    use std::path::PathBuf;

    let index = CacheIndex::new();

    let key = CacheKey {
        bucket: "test-bucket".to_string(),
        object_key: "test-key".to_string(),
        etag: None,
    };

    let metadata = test_entry_metadata(
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
    use super::index::CacheIndex;
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
    let metadata1 = test_entry_metadata(
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
    let metadata2 = test_entry_metadata(
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
    use super::error::DiskCacheError;
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
    use super::utils::key_to_hash;
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
    use super::utils::generate_paths;
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
    use super::utils::{generate_paths, key_to_hash};
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
    use super::utils::key_to_hash;
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
    use super::backend::DiskBackend;
    use std::sync::Arc;

    // Verify we can reference the trait
    fn _accept_backend(_backend: Arc<dyn DiskBackend>) {
        // This function verifies DiskBackend is object-safe
    }

    // Test passes if it compiles - DiskBackend trait is defined and object-safe
}

#[test]
fn test_disk_backend_is_send_sync() {
    // Verify DiskBackend trait is Send + Sync
    use super::backend::DiskBackend;

    fn _assert_send<T: Send>() {}
    fn _assert_sync<T: Sync>() {}

    // These will only compile if DiskBackend requires Send + Sync
    _assert_send::<Box<dyn DiskBackend>>();
    _assert_sync::<Box<dyn DiskBackend>>();

    // Test passes if it compiles - DiskBackend is Send + Sync
}

#[tokio::test]
async fn test_disk_backend_trait_has_required_methods() {
    // Verify DiskBackend trait has all required async methods
    // We'll use the TokioFsBackend to verify the trait methods exist
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use bytes::Bytes;

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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use std::sync::Arc;

    let backend = TokioFsBackend::new();
    let trait_object: Arc<dyn DiskBackend> = Arc::new(backend);

    // Verify the trait object can be cloned (Arc)
    let _cloned = trait_object.clone();

    // Test passes if it compiles - Can create Arc<dyn DiskBackend> trait objects
}

// Phase 28.2: MockDiskBackend (for testing)

#[test]
fn test_can_create_mock_backend() {
    // Verify MockDiskBackend can be created
    use super::mock_backend::MockDiskBackend;

    let backend = MockDiskBackend::new();

    // Verify initial state
    assert_eq!(backend.file_count(), 0);
}

#[tokio::test]
async fn test_mock_backend_implements_disk_backend() {
    // Verify MockDiskBackend implements DiskBackend trait
    use super::backend::DiskBackend;
    use super::mock_backend::MockDiskBackend;
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
    use super::backend::DiskBackend;
    use super::mock_backend::MockDiskBackend;
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
    use super::backend::DiskBackend;
    use super::mock_backend::MockDiskBackend;
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
    use super::backend::DiskBackend;
    use super::error::DiskCacheError;
    use super::mock_backend::MockDiskBackend;
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
    use super::backend::DiskBackend;
    use super::error::DiskCacheError;
    use super::mock_backend::MockDiskBackend;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::generate_paths;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::generate_paths;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use super::types::EntryMetadata;
    use super::utils::generate_paths;
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
    let metadata = test_entry_metadata(
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::generate_paths;
    use bytes::Bytes;

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
    let metadata = test_entry_metadata(key, data_path.clone(), data.len() as u64, 1000000, 2000000);
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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
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
    let metadata1 = test_entry_metadata(
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

    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
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
    let metadata = test_entry_metadata(
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
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::cache_key_to_file_path;
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

    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 5, now, 0);
    let metadata2 = test_entry_metadata(key2.clone(), data_path2.clone(), 5, now, 0);

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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::cache_key_to_file_path;
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

    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 5, now, 0);
    let metadata2 = test_entry_metadata(key2.clone(), data_path2.clone(), 5, now, 0);

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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::cache_key_to_file_path;
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

    let metadata = test_entry_metadata(key1.clone(), data_path1.clone(), 5, now, 0);
    let meta_json = serde_json::to_string(&metadata).unwrap();
    backend
        .write_file_atomic(&meta_path1, Bytes::from(meta_json))
        .await
        .unwrap();

    // Create index with both keys (but key2 has no files)
    let index = CacheIndex::new();
    let metadata2 = test_entry_metadata(key2.clone(), PathBuf::from("/nonexistent"), 5, now, 0);
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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::cache_key_to_file_path;
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
    let wrong_metadata = test_entry_metadata(key1.clone(), data_path.clone(), 999, now, 0); // Wrong size!
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
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
    use super::utils::cache_key_to_file_path;
    use crate::cache::CacheKey;
    use std::time::SystemTime;
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
        test_entry_metadata(key1.clone(), data_path1.clone(), 5, now, expired_time);

    // key2 expires in 1 hour (still valid)
    let future_time = now + 3600;
    let valid_metadata = test_entry_metadata(key2.clone(), data_path2.clone(), 5, now, future_time);

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
    use super::tokio_backend::TokioFsBackend;

    let backend = TokioFsBackend::new();

    // Backend should be created successfully
    // This is a simple smoke test to ensure the struct can be instantiated
    let _ = backend;
}

#[test]
fn test_tokio_fs_backend_is_send_sync() {
    // Verify TokioFsBackend implements Send + Sync (required for async)
    use super::tokio_backend::TokioFsBackend;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<TokioFsBackend>();
    assert_sync::<TokioFsBackend>();
}

#[tokio::test]
async fn test_read_file_returns_error_if_not_exists() {
    // Verify read_file() returns error when file doesn't exist
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
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
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use bytes::Bytes;
    use std::path::Path;

    // Skip this test if running as root (uid 0) since root can write anywhere
    #[cfg(unix)]
    {
        let is_root = unsafe { libc::getuid() == 0 };
        if is_root {
            eprintln!("Skipping test: running as root, permission test not applicable");
            return;
        }
    }

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

// Phase 28.6: UringBackend Implementation (Linux only)
// Using io-uring crate (not tokio-uring) with spawn_blocking wrapper
// io-uring has Send + Sync types, compatible with async_trait
// See IO_URING_FEASIBILITY.md for implementation approach

#[test]
#[cfg(target_os = "linux")]
fn test_can_create_uring_backend() {
    // Verify UringBackend can be created on Linux
    use super::uring_backend::UringBackend;

    let _backend = UringBackend::new();

    // Backend should be created successfully
    // This is a compile-time test - if it compiles, the struct exists
}

#[test]
#[cfg(target_os = "linux")]
fn test_uring_backend_implements_disk_backend() {
    // Verify UringBackend implements DiskBackend trait (compile-time check)
    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use std::sync::Arc;

    let backend = UringBackend::new();
    let _trait_object: Arc<dyn DiskBackend> = Arc::new(backend);

    // If this compiles, UringBackend implements DiskBackend trait with Send futures
    // Compile-time assertion - if we get here, the trait is implemented
}

#[test]
#[cfg(target_os = "linux")]
fn test_uring_backend_is_send_sync() {
    // Verify UringBackend implements Send + Sync (required for async)
    use super::uring_backend::UringBackend;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<UringBackend>();
    assert_sync::<UringBackend>();
}

#[test]
#[cfg(target_os = "linux")]
fn test_uring_backend_interchangeable_with_tokio_backend() {
    // Verify UringBackend and TokioFsBackend can be used interchangeably through DiskBackend trait
    use super::backend::DiskBackend;
    use super::tokio_backend::TokioFsBackend;
    use super::uring_backend::UringBackend;
    use std::sync::Arc;

    // Both backends can be stored in same type (Arc<dyn DiskBackend>)
    let backends: Vec<Arc<dyn DiskBackend>> = vec![
        Arc::new(UringBackend::new()),
        Arc::new(TokioFsBackend::new()),
    ];

    // Verify we have both backends
    assert_eq!(
        backends.len(),
        2,
        "Both backends can be used as trait objects"
    );
}

#[test]
#[cfg(not(target_os = "linux"))]
fn test_uring_backend_not_available_on_non_linux() {
    // Verify UringBackend is not compiled on non-Linux platforms
    // This test simply verifies the module compiles without uring_backend on macOS/Windows
    // Test passes if it compiles - build succeeds without UringBackend on non-Linux platforms
}

// Functional tests for UringBackend (Linux only)
// These tests verify basic file operations work correctly

/// Helper function to check if io_uring is available on this kernel
/// Returns true if io_uring can be initialized, false otherwise (e.g., kernel < 5.1)
#[cfg(target_os = "linux")]
fn is_io_uring_available() -> bool {
    // Try to create an io_uring instance - will fail on kernels < 5.1
    io_uring::IoUring::new(8).is_ok()
}

/// Macro to skip test if io_uring is not available on this kernel
/// Use at the beginning of io_uring tests to gracefully skip on older kernels (< 5.1)
#[cfg(target_os = "linux")]
macro_rules! skip_if_no_io_uring {
    () => {
        if !is_io_uring_available() {
            eprintln!("Skipping test: io_uring not available on this kernel");
            return;
        }
    };
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_read_file_successfully() {
    // Test: read_file() successfully reads existing file
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_read.txt");

    // Create file using tokio::fs (not the backend under test)
    let content = "Hello from io-uring!";
    tokio::fs::write(&file_path, content).await.unwrap();

    // Read file using UringBackend
    let backend = UringBackend::new();
    let read_data = backend.read_file(&file_path).await.unwrap();

    // Verify content matches
    assert_eq!(read_data, Bytes::from(content));
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_read_missing_file_returns_error() {
    // Test: read_file() returns error for missing file
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let missing_file = temp_dir.path().join("does_not_exist.txt");

    // Verify file doesn't exist
    assert!(!missing_file.exists());

    // Try to read missing file using UringBackend
    let backend = UringBackend::new();
    let result = backend.read_file(&missing_file).await;

    // Should return error (not panic)
    assert!(result.is_err(), "Reading missing file should return error");

    // Verify it's an IO error
    match result {
        Err(e) => {
            let error_string = e.to_string().to_lowercase();
            assert!(
                error_string.contains("no such file") || error_string.contains("not found"),
                "Error should indicate file not found, got: {}",
                e
            );
        }
        Ok(_) => panic!("Expected error but got Ok"),
    }
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_read_file_binary_data_integrity() {
    // Test: read_file() returns Bytes with correct content (binary data)
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary_data.bin");

    // Create binary data with various byte values including null bytes
    let binary_data: Vec<u8> = vec![
        0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD, // Various byte values
        0x48, 0x65, 0x6C, 0x6C, 0x6F, // "Hello"
        0x00, 0x00, // Null bytes
        0xDE, 0xAD, 0xBE, 0xEF, // Common hex pattern
    ];

    // Write binary file using tokio::fs
    tokio::fs::write(&file_path, &binary_data).await.unwrap();

    // Read file using UringBackend
    let backend = UringBackend::new();
    let read_data = backend.read_file(&file_path).await.unwrap();

    // Verify exact byte-for-byte match
    assert_eq!(
        read_data,
        Bytes::from(binary_data.clone()),
        "Binary data should match exactly"
    );
    assert_eq!(
        read_data.len(),
        binary_data.len(),
        "Binary data length should match"
    );

    // Verify specific byte values to ensure no corruption
    assert_eq!(read_data[0], 0x00, "First byte should be null");
    assert_eq!(read_data[3], 0xFF, "High-value byte should be preserved");
    assert_eq!(
        &read_data[6..11],
        b"Hello",
        "ASCII portion should be readable"
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_read_large_file() {
    // Test: Handles large files (>1MB) correctly
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_file.bin");

    // Create 2MB file with repeating pattern for verification
    let pattern: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
    let mut large_data = Vec::new();
    let target_size = 2 * 1024 * 1024; // 2MB

    while large_data.len() < target_size {
        large_data.extend_from_slice(&pattern);
    }
    large_data.truncate(target_size); // Exactly 2MB

    // Write large file using tokio::fs
    tokio::fs::write(&file_path, &large_data).await.unwrap();

    // Read file using UringBackend
    let backend = UringBackend::new();
    let read_data = backend.read_file(&file_path).await.unwrap();

    // Verify size matches
    assert_eq!(
        read_data.len(),
        target_size,
        "Large file size should match (2MB)"
    );

    // Verify exact content match
    assert_eq!(
        read_data,
        Bytes::from(large_data.clone()),
        "Large file content should match exactly"
    );

    // Verify pattern at different positions to ensure no corruption
    assert_eq!(&read_data[0..8], &pattern[..], "Pattern at start");
    assert_eq!(
        &read_data[1024 * 1024..1024 * 1024 + 8],
        &pattern[..],
        "Pattern at 1MB offset"
    );
    assert_eq!(
        &read_data[target_size - 8..],
        &pattern[..],
        "Pattern at end"
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_write_creates_parent_dirs() {
    // Test: write_file_atomic() creates parent directories
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir
        .path()
        .join("subdir1")
        .join("subdir2")
        .join("test_file.txt");

    // Verify parent directories don't exist yet
    assert!(!nested_path.parent().unwrap().exists());

    // Write file using UringBackend (should create parent dirs)
    let backend = UringBackend::new();
    let data = Bytes::from("test data");
    backend
        .write_file_atomic(&nested_path, data.clone())
        .await
        .unwrap();

    // Verify parent directories were created
    assert!(
        nested_path.parent().unwrap().exists(),
        "Parent directories should be created"
    );

    // Verify file was written correctly
    assert!(nested_path.exists(), "File should exist");
    let read_data = tokio::fs::read(&nested_path).await.unwrap();
    assert_eq!(read_data, data.as_ref(), "File content should match");
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_write_uses_temp_file() {
    // Test: write_file_atomic() writes to temp file first
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_file.txt");
    let temp_path = file_path.with_extension("tmp");

    // Verify neither file exists initially
    assert!(!file_path.exists(), "Final file should not exist initially");
    assert!(!temp_path.exists(), "Temp file should not exist initially");

    // Write file using UringBackend
    let backend = UringBackend::new();
    let data = Bytes::from("test data for temp file");
    backend
        .write_file_atomic(&file_path, data.clone())
        .await
        .unwrap();

    // After successful write:
    // - Final file should exist
    // - Temp file should NOT exist (renamed to final)
    assert!(file_path.exists(), "Final file should exist after write");
    assert!(
        !temp_path.exists(),
        "Temp file should be cleaned up (renamed to final)"
    );

    // Verify final file has correct content
    let read_data = tokio::fs::read(&file_path).await.unwrap();
    assert_eq!(read_data, data.as_ref(), "Final file content should match");
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_write_atomic_rename() {
    // Test: write_file_atomic() atomically renames temp to final
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("atomic_test.txt");

    let backend = UringBackend::new();

    // Write initial data
    let data1 = Bytes::from("original content");
    backend
        .write_file_atomic(&file_path, data1.clone())
        .await
        .unwrap();

    // Verify initial write
    let read_data1 = tokio::fs::read(&file_path).await.unwrap();
    assert_eq!(read_data1, data1.as_ref());

    // Overwrite with new data (tests atomic replace)
    let data2 = Bytes::from("new content that replaces original");
    backend
        .write_file_atomic(&file_path, data2.clone())
        .await
        .unwrap();

    // Verify new content completely replaced old content
    let read_data2 = tokio::fs::read(&file_path).await.unwrap();
    assert_eq!(
        read_data2,
        data2.as_ref(),
        "New content should completely replace old"
    );
    assert_ne!(read_data2.len(), data1.len(), "Content length changed");

    // Verify no temp file remains
    let temp_path = file_path.with_extension("tmp");
    assert!(!temp_path.exists(), "No temp file should remain");
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_write_handles_errors() {
    // Test: write_file_atomic() handles write errors gracefully
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let backend = UringBackend::new();

    // Test 1: Try to write where parent is a file (not a directory)
    let blocking_file = temp_dir.path().join("blocking.txt");
    tokio::fs::write(&blocking_file, "I'm a file")
        .await
        .unwrap();

    let impossible_path = blocking_file.join("cant_create_this.txt");
    let data = Bytes::from("test data");
    let result = backend
        .write_file_atomic(&impossible_path, data.clone())
        .await;

    // Should return error, not panic
    assert!(
        result.is_err(),
        "Writing with file as parent should return error"
    );

    // Test 2: Verify error message is informative (file system error)
    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();
        // Error should be a file system error (not a directory, file exists, etc.)
        assert!(
            error_msg.contains("not a directory")
                || error_msg.contains("is a file")
                || error_msg.contains("notdir")
                || error_msg.contains("file exists")
                || error_msg.contains("i/o error"),
            "Error should indicate file system problem, got: {}",
            e
        );
    }
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_delete_removes_file() {
    // Test: delete_file() removes existing file
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file_to_delete.txt");

    // Create a file first
    let backend = UringBackend::new();
    let data = Bytes::from("delete me");
    backend.write_file_atomic(&file_path, data).await.unwrap();

    // Verify file exists
    assert!(file_path.exists(), "File should exist before delete");

    // Delete the file
    backend.delete_file(&file_path).await.unwrap();

    // Verify file no longer exists
    assert!(!file_path.exists(), "File should not exist after delete");
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_delete_is_idempotent() {
    // Test: delete_file() is idempotent (ignores missing files)
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nonexistent_file.txt");

    // Verify file doesn't exist
    assert!(!file_path.exists(), "File should not exist initially");

    let backend = UringBackend::new();

    // Delete non-existent file - should succeed (idempotent)
    let result = backend.delete_file(&file_path).await;
    assert!(
        result.is_ok(),
        "delete_file() should succeed for non-existent file"
    );

    // Delete again - should still succeed (idempotent)
    let result2 = backend.delete_file(&file_path).await;
    assert!(
        result2.is_ok(),
        "delete_file() should succeed when called multiple times"
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_create_dir_all_nested() {
    // Test: create_dir_all() creates nested directories
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("level1").join("level2").join("level3");

    // Verify path doesn't exist
    assert!(
        !nested_path.exists(),
        "Nested path should not exist initially"
    );

    let backend = UringBackend::new();

    // Create nested directories
    backend.create_dir_all(&nested_path).await.unwrap();

    // Verify all levels exist
    assert!(
        nested_path.exists(),
        "Nested path should exist after create"
    );
    assert!(nested_path.is_dir(), "Nested path should be a directory");
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_create_dir_all_idempotent() {
    // Test: create_dir_all() is idempotent
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_dir");

    let backend = UringBackend::new();

    // Create directory first time
    backend.create_dir_all(&dir_path).await.unwrap();
    assert!(
        dir_path.exists(),
        "Directory should exist after first create"
    );

    // Create again - should succeed (idempotent)
    let result = backend.create_dir_all(&dir_path).await;
    assert!(
        result.is_ok(),
        "create_dir_all() should succeed when directory already exists"
    );

    // Verify directory still exists
    assert!(dir_path.exists(), "Directory should still exist");
    assert!(dir_path.is_dir(), "Path should still be a directory");
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_file_size() {
    // Test: file_size() returns correct size for existing file
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_file.txt");

    let backend = UringBackend::new();

    // Create a file with known size
    let data = Bytes::from("Hello, io-uring! This is a test file.");
    let expected_size = data.len() as u64;

    backend.write_file_atomic(&file_path, data).await.unwrap();

    // Get file size
    let size = backend.file_size(&file_path).await.unwrap();

    // Verify size is correct
    assert_eq!(
        size, expected_size,
        "file_size() should return correct file size"
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_uring_backend_read_dir() {
    // Test: read_dir() lists directory contents
    skip_if_no_io_uring!();

    use super::backend::DiskBackend;
    use super::uring_backend::UringBackend;
    use bytes::Bytes;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");

    let backend = UringBackend::new();

    // Create directory
    backend.create_dir_all(&test_dir).await.unwrap();

    // Create some files in the directory
    let file1 = test_dir.join("file1.txt");
    let file2 = test_dir.join("file2.txt");
    let file3 = test_dir.join("file3.txt");

    backend
        .write_file_atomic(&file1, Bytes::from("content1"))
        .await
        .unwrap();
    backend
        .write_file_atomic(&file2, Bytes::from("content2"))
        .await
        .unwrap();
    backend
        .write_file_atomic(&file3, Bytes::from("content3"))
        .await
        .unwrap();

    // Read directory contents
    let mut entries = backend.read_dir(&test_dir).await.unwrap();

    // Sort entries for consistent comparison (filesystem order can vary)
    entries.sort();

    // Verify all files are listed
    assert_eq!(entries.len(), 3, "Should list exactly 3 files");
    assert!(entries.contains(&file1), "Should contain file1.txt");
    assert!(entries.contains(&file2), "Should contain file2.txt");
    assert!(entries.contains(&file3), "Should contain file3.txt");
}

// Phase 28.7: LRU Eviction
// Size Tracking tests

#[tokio::test]
async fn test_tracks_total_disk_cache_size() {
    // Verify DiskCache tracks total size of all cached entries
    use super::disk_cache::DiskCache;
    use crate::cache::Cache;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // Create DiskCache with max size limit
    let max_size_bytes = 10 * 1024 * 1024; // 10MB
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    // Get initial stats
    let stats = cache.stats().await.unwrap();

    // Initial size should be 0
    assert_eq!(stats.current_size_bytes, 0);
    assert_eq!(stats.max_size_bytes, max_size_bytes);
    assert_eq!(stats.current_item_count, 0);
}

#[tokio::test]
async fn test_size_updated_on_set() {
    // Verify DiskCache updates total size when entries are added
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 10 * 1024 * 1024);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600); // Expires in 1 hour

    // Add first entry (1KB)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    // Verify size increased
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 1024);
    assert_eq!(stats.current_item_count, 1);

    // Add second entry (2KB)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![0u8; 2048]); // 2KB
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Verify size increased again
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 3072); // 1KB + 2KB
    assert_eq!(stats.current_item_count, 2);
}

#[tokio::test]
async fn test_size_updated_on_delete() {
    // Verify DiskCache updates total size when entries are deleted
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 10 * 1024 * 1024);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add two entries
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![0u8; 2048]); // 2KB
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Verify initial size (3KB total)
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 3072);
    assert_eq!(stats.current_item_count, 2);

    // Delete first entry (1KB)
    let deleted = cache.delete(&key1).await.unwrap();
    assert!(deleted, "Entry should have been deleted");

    // Verify size decreased
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 2048); // Only 2KB remaining
    assert_eq!(stats.current_item_count, 1);

    // Delete second entry (2KB)
    let deleted = cache.delete(&key2).await.unwrap();
    assert!(deleted, "Entry should have been deleted");

    // Verify size is now zero
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 0);
    assert_eq!(stats.current_item_count, 0);

    // Try to delete non-existent entry
    let deleted = cache.delete(&key1).await.unwrap();
    assert!(!deleted, "Entry should not exist");
}

#[tokio::test]
async fn test_detects_when_size_exceeds_max() {
    // Verify DiskCache can detect when total size exceeds max_size_bytes
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // Create cache with small max size (2KB)
    let max_size_bytes = 2048;
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Initially, size should not exceed max
    let stats = cache.stats().await.unwrap();
    assert!(stats.current_size_bytes <= stats.max_size_bytes);
    assert_eq!(stats.max_size_bytes, max_size_bytes);

    // Add entry that fits (1KB)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    // Still within limit
    let stats = cache.stats().await.unwrap();
    assert!(stats.current_size_bytes <= stats.max_size_bytes);
    assert_eq!(stats.current_size_bytes, 1024);

    // Add another entry that fits (1KB more = 2KB total, exactly at max)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Exactly at max size
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 2048);
    assert_eq!(stats.current_size_bytes, stats.max_size_bytes);

    // Add third entry that exceeds max (1KB more = 3KB total, exceeds 2KB max)
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file3.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Eviction should have kept size at or under max
    let stats = cache.stats().await.unwrap();
    assert!(stats.current_size_bytes <= stats.max_size_bytes);
    assert_eq!(stats.current_size_bytes, 2048); // 2KB (after eviction)
    assert_eq!(stats.max_size_bytes, 2048); // 2KB max
    assert_eq!(stats.current_item_count, 2); // One entry evicted
}

// ============================================================================
// Phase 28.7: Eviction Logic Tests
// ============================================================================

#[tokio::test]
async fn test_eviction_triggered_when_threshold_exceeded() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 2048; // 2KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add first entry (1KB) at time T0
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "old_file.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    // Sleep to ensure different timestamps (longer sleep for parallel test execution)
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add second entry (1KB) at time T1
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "middle_file.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![1u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Size should be 2KB (at max)
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_size_bytes, 2048);
    assert_eq!(stats.current_item_count, 2);

    // Sleep to ensure different timestamps (longer sleep for parallel test execution)
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add third entry (1KB) at time T2 - this should trigger eviction
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "new_file.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![2u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3,
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Eviction should have occurred
    let stats = cache.stats().await.unwrap();

    // Size should be back under or at max (evicted oldest entry)
    assert!(stats.current_size_bytes <= max_size_bytes);

    // Should have 2 entries (oldest evicted)
    assert_eq!(stats.current_item_count, 2);

    // Oldest entry (key1) should be gone
    let result1 = cache.get(&key1).await.unwrap();
    assert!(result1.is_none(), "Oldest entry should have been evicted");

    // Newer entries should still exist
    let result2 = cache.get(&key2).await.unwrap();
    assert!(result2.is_some(), "Second entry should still exist");

    let result3 = cache.get(&key3).await.unwrap();
    assert!(result3.is_some(), "Newest entry should still exist");
}

#[tokio::test]
async fn test_identifies_least_recently_accessed_entry() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 3072; // 3KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add three entries, each 1KB, with clear temporal ordering

    // Entry 1: Oldest (created first)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "oldest.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![1u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    // Sleep to ensure distinct timestamp
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Entry 2: Middle (created second)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "middle.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![2u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Sleep to ensure distinct timestamp
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Entry 3: Newest (created third)
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "newest.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![3u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3,
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Verify all three entries are in cache
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, 3);
    assert_eq!(stats.current_size_bytes, 3072); // Exactly at max

    // Sleep to ensure distinct timestamp
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add a fourth entry to trigger eviction
    let key4 = CacheKey {
        bucket: "test".to_string(),
        object_key: "trigger_eviction.txt".to_string(),
        etag: None,
    };
    let data4 = Bytes::from(vec![4u8; 1024]);
    let now4 = SystemTime::now();
    let entry4 = CacheEntry {
        data: data4.clone(),
        content_type: "text/plain".to_string(),
        content_length: data4.len(),
        etag: "etag4".to_string(),
        last_modified: None,
        created_at: now4,
        expires_at: future,
        last_accessed_at: now4,
    };
    cache.set(key4.clone(), entry4).await.unwrap();

    // Verify eviction occurred
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, 3); // One was evicted
    assert_eq!(stats.current_size_bytes, 3072); // Back to max size

    // The OLDEST entry (key1) should have been evicted
    let result1 = cache.get(&key1).await.unwrap();
    assert!(
        result1.is_none(),
        "Oldest entry (key1) should have been evicted as LRU"
    );

    // All other entries should still exist
    let result2 = cache.get(&key2).await.unwrap();
    assert!(result2.is_some(), "Entry key2 should still exist");

    let result3 = cache.get(&key3).await.unwrap();
    assert!(result3.is_some(), "Entry key3 should still exist");

    let result4 = cache.get(&key4).await.unwrap();
    assert!(result4.is_some(), "Newly added entry key4 should exist");
}

#[tokio::test]
async fn test_deletes_both_data_and_meta_files() {
    use super::disk_cache::DiskCache;
    use crate::cache::disk::utils::{generate_paths, key_to_hash};
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    // Create temp directory for cache
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    // Create cache with 2KB max size
    let cache = DiskCache::with_config(cache_dir.clone(), 2048);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Create first entry (1KB)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![1u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };

    cache.set(key1.clone(), entry1).await.unwrap();

    // Verify files exist
    let hash1 = key_to_hash(&key1);
    let (data_path1, meta_path1) = generate_paths(&cache_dir, &hash1);

    assert!(
        data_path1.exists(),
        "Data file should exist after insertion"
    );
    assert!(
        meta_path1.exists(),
        "Meta file should exist after insertion"
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Create second entry (1KB)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![2u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2,
    };

    cache.set(key2.clone(), entry2).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Create third entry (1KB) - should trigger eviction of key1
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file3.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![3u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3,
    };

    cache.set(key3.clone(), entry3).await.unwrap();

    // Verify that key1's files are deleted
    assert!(
        !data_path1.exists(),
        "Data file should be deleted after eviction"
    );
    assert!(
        !meta_path1.exists(),
        "Meta file should be deleted after eviction"
    );

    // Verify that key2 and key3 files still exist
    let hash2 = key_to_hash(&key2);
    let (data_path2, meta_path2) = generate_paths(&cache_dir, &hash2);

    assert!(data_path2.exists(), "Data file for key2 should still exist");
    assert!(meta_path2.exists(), "Meta file for key2 should still exist");

    let hash3 = key_to_hash(&key3);
    let (data_path3, meta_path3) = generate_paths(&cache_dir, &hash3);

    assert!(data_path3.exists(), "Data file for key3 should still exist");
    assert!(meta_path3.exists(), "Meta file for key3 should still exist");
}

#[tokio::test]
async fn test_removes_entry_from_index() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 2048; // 2KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add first entry (1KB) at time T0
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "old_file.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    // Verify entry is in cache
    let result1 = cache.get(&key1).await.unwrap();
    assert!(result1.is_some(), "Entry key1 should exist after insertion");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add second entry (1KB) at time T1
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "middle_file.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![1u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add third entry (1KB) at time T2 - should trigger eviction of key1
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "new_file.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![2u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3,
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Verify key1 was evicted and is no longer in the index
    let result1_after = cache.get(&key1).await.unwrap();
    assert!(
        result1_after.is_none(),
        "Entry key1 should be evicted and not in index"
    );

    // Verify key2 and key3 are still in the index
    let result2 = cache.get(&key2).await.unwrap();
    assert!(result2.is_some(), "Entry key2 should still exist in index");

    let result3 = cache.get(&key3).await.unwrap();
    assert!(
        result3.is_some(),
        "Newly added entry key3 should exist in index"
    );
}

#[tokio::test]
async fn test_updates_stats_eviction_count() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 2048; // 2KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    // Check initial stats
    let initial_stats = cache.stats().await.unwrap();
    assert_eq!(
        initial_stats.evictions, 0,
        "Initial eviction count should be 0"
    );

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add first entry (1KB)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add second entry (1KB)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![1u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Stats should still show 0 evictions
    let stats_before = cache.stats().await.unwrap();
    assert_eq!(stats_before.evictions, 0, "No evictions yet");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add third entry (1KB) - should trigger eviction of key1
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file3.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![2u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3,
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Stats should now show 1 eviction
    let stats_after = cache.stats().await.unwrap();
    assert_eq!(
        stats_after.evictions, 1,
        "Should have 1 eviction after exceeding capacity"
    );

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add fourth entry (1KB) - should trigger another eviction (key2)
    let key4 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file4.txt".to_string(),
        etag: None,
    };
    let data4 = Bytes::from(vec![3u8; 1024]);
    let now4 = SystemTime::now();
    let entry4 = CacheEntry {
        data: data4.clone(),
        content_type: "text/plain".to_string(),
        content_length: data4.len(),
        etag: "etag4".to_string(),
        last_modified: None,
        created_at: now4,
        expires_at: future,
        last_accessed_at: now4,
    };
    cache.set(key4.clone(), entry4).await.unwrap();

    // Stats should now show 2 evictions
    let stats_final = cache.stats().await.unwrap();
    assert_eq!(stats_final.evictions, 2, "Should have 2 evictions total");
}

#[tokio::test]
async fn test_can_evict_multiple_entries_in_one_pass() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 3072; // 3KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add three 1KB entries to fill cache to capacity
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now,
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![1u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2,
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file3.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![2u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3,
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Verify all three entries are cached
    assert_eq!(cache.stats().await.unwrap().current_item_count, 3);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add a 2KB entry - should evict TWO entries (key1 and key2) in one pass
    let key4 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file4.txt".to_string(),
        etag: None,
    };
    let data4 = Bytes::from(vec![3u8; 2048]); // 2KB
    let now4 = SystemTime::now();
    let entry4 = CacheEntry {
        data: data4.clone(),
        content_type: "text/plain".to_string(),
        content_length: data4.len(),
        etag: "etag4".to_string(),
        last_modified: None,
        created_at: now4,
        expires_at: future,
        last_accessed_at: now4,
    };
    cache.set(key4.clone(), entry4).await.unwrap();

    // Verify that 2 entries were evicted
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.evictions, 2, "Should have evicted 2 entries");

    // Verify key1 and key2 are gone
    assert!(
        cache.get(&key1).await.unwrap().is_none(),
        "key1 should be evicted"
    );
    assert!(
        cache.get(&key2).await.unwrap().is_none(),
        "key2 should be evicted"
    );

    // Verify key3 and key4 remain
    assert!(
        cache.get(&key3).await.unwrap().is_some(),
        "key3 should still exist"
    );
    assert!(
        cache.get(&key4).await.unwrap().is_some(),
        "key4 should exist"
    );

    // Verify final count is 2 entries
    assert_eq!(cache.stats().await.unwrap().current_item_count, 2);
}

#[tokio::test]
async fn test_evicts_in_lru_order() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 4096; // 4KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add four 1KB entries with staggered timestamps
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "oldest.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "text/plain".to_string(),
        content_length: data1.len(),
        etag: "etag1".to_string(),
        last_modified: None,
        created_at: now,
        expires_at: future,
        last_accessed_at: now, // T0 - oldest
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "second_oldest.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![1u8; 1024]);
    let now2 = SystemTime::now();
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "text/plain".to_string(),
        content_length: data2.len(),
        etag: "etag2".to_string(),
        last_modified: None,
        created_at: now2,
        expires_at: future,
        last_accessed_at: now2, // T1 - second oldest
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "newer.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![2u8; 1024]);
    let now3 = SystemTime::now();
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "text/plain".to_string(),
        content_length: data3.len(),
        etag: "etag3".to_string(),
        last_modified: None,
        created_at: now3,
        expires_at: future,
        last_accessed_at: now3, // T2 - second newest
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let key4 = CacheKey {
        bucket: "test".to_string(),
        object_key: "newest.txt".to_string(),
        etag: None,
    };
    let data4 = Bytes::from(vec![3u8; 1024]);
    let now4 = SystemTime::now();
    let entry4 = CacheEntry {
        data: data4.clone(),
        content_type: "text/plain".to_string(),
        content_length: data4.len(),
        etag: "etag4".to_string(),
        last_modified: None,
        created_at: now4,
        expires_at: future,
        last_accessed_at: now4, // T3 - newest
    };
    cache.set(key4.clone(), entry4).await.unwrap();

    // Verify all four entries are cached
    assert_eq!(cache.stats().await.unwrap().current_item_count, 4);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add a 2KB entry - should evict TWO oldest entries (key1 and key2)
    let key5 = CacheKey {
        bucket: "test".to_string(),
        object_key: "large_new.txt".to_string(),
        etag: None,
    };
    let data5 = Bytes::from(vec![4u8; 2048]); // 2KB
    let now5 = SystemTime::now();
    let entry5 = CacheEntry {
        data: data5.clone(),
        content_type: "text/plain".to_string(),
        content_length: data5.len(),
        etag: "etag5".to_string(),
        last_modified: None,
        created_at: now5,
        expires_at: future,
        last_accessed_at: now5,
    };
    cache.set(key5.clone(), entry5).await.unwrap();

    // Verify the TWO OLDEST entries (key1 and key2) were evicted
    assert!(
        cache.get(&key1).await.unwrap().is_none(),
        "key1 (oldest) should be evicted"
    );
    assert!(
        cache.get(&key2).await.unwrap().is_none(),
        "key2 (second oldest) should be evicted"
    );

    // Verify the NEWER entries (key3 and key4) and new entry (key5) remain
    assert!(
        cache.get(&key3).await.unwrap().is_some(),
        "key3 (second newest) should still exist"
    );
    assert!(
        cache.get(&key4).await.unwrap().is_some(),
        "key4 (newest) should still exist"
    );
    assert!(
        cache.get(&key5).await.unwrap().is_some(),
        "key5 (new entry) should exist"
    );

    // Verify stats
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.evictions, 2, "Should have evicted 2 entries");
    assert_eq!(
        stats.current_item_count, 3,
        "Should have 3 entries remaining"
    );
}

#[tokio::test]
async fn test_stops_when_enough_space_freed() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let max_size_bytes = 5120; // 5KB max
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), max_size_bytes);

    let now = SystemTime::now();
    let future = now + std::time::Duration::from_secs(3600);

    // Add five 1KB entries to fill cache to capacity
    let mut keys = Vec::new();
    for i in 0..5 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: format!("file{}.txt", i + 1),
            etag: None,
        };
        let data = Bytes::from(vec![i as u8; 1024]);
        let time = SystemTime::now();
        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: format!("etag{}", i + 1),
            last_modified: None,
            created_at: time,
            expires_at: future,
            last_accessed_at: time,
        };
        cache.set(key.clone(), entry).await.unwrap();
        keys.push(key);
    }

    // Verify all five entries are cached
    assert_eq!(cache.stats().await.unwrap().current_item_count, 5);
    assert_eq!(cache.stats().await.unwrap().current_size_bytes, 5120);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Add a 2KB entry - should evict EXACTLY 2 entries (just enough for 2KB)
    // Cache state: 5KB used
    // New entry needs: 2KB
    // After evicting 1 entry: 4KB used + 2KB new = 6KB > 5KB max (not enough)
    // After evicting 2 entries: 3KB used + 2KB new = 5KB <= 5KB max (just enough!)
    let new_key = CacheKey {
        bucket: "test".to_string(),
        object_key: "large_file.txt".to_string(),
        etag: None,
    };
    let data_new = Bytes::from(vec![99u8; 2048]); // 2KB
    let time_new = SystemTime::now();
    let entry_new = CacheEntry {
        data: data_new.clone(),
        content_type: "text/plain".to_string(),
        content_length: data_new.len(),
        etag: "etag_new".to_string(),
        last_modified: None,
        created_at: time_new,
        expires_at: future,
        last_accessed_at: time_new,
    };
    cache.set(new_key.clone(), entry_new).await.unwrap();

    // Verify EXACTLY 2 entries were evicted (not more)
    let stats = cache.stats().await.unwrap();
    assert_eq!(
        stats.evictions, 2,
        "Should have evicted exactly 2 entries (just enough to fit 2KB)"
    );

    // Verify the two oldest entries (keys[0] and keys[1]) were evicted
    assert!(
        cache.get(&keys[0]).await.unwrap().is_none(),
        "First entry should be evicted"
    );
    assert!(
        cache.get(&keys[1]).await.unwrap().is_none(),
        "Second entry should be evicted"
    );

    // Verify the remaining 3 old entries and the new entry exist
    assert!(
        cache.get(&keys[2]).await.unwrap().is_some(),
        "Third entry should remain"
    );
    assert!(
        cache.get(&keys[3]).await.unwrap().is_some(),
        "Fourth entry should remain"
    );
    assert!(
        cache.get(&keys[4]).await.unwrap().is_some(),
        "Fifth entry should remain"
    );
    assert!(
        cache.get(&new_key).await.unwrap().is_some(),
        "New entry should exist"
    );

    // Verify final count is 4 entries (3 old + 1 new)
    assert_eq!(
        stats.current_item_count, 4,
        "Should have exactly 4 entries remaining"
    );

    // Verify total size is exactly at or under max (3KB old + 2KB new = 5KB)
    assert!(
        stats.current_size_bytes <= max_size_bytes,
        "Total size should be at or under max"
    );
    assert_eq!(
        stats.current_size_bytes, 5120,
        "Total size should be exactly 5KB (3 x 1KB + 1 x 2KB)"
    );
}

// Phase 28.8: Recovery & Startup Tests
#[tokio::test]
async fn test_loads_index_from_json() {
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use crate::cache::CacheKey;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let index_path = PathBuf::from("/cache/index.json");

    // Create an index with some entries
    let index1 = CacheIndex::new();

    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let metadata1 = test_entry_metadata(
        key1.clone(),
        PathBuf::from("/cache/entries/hash1.data"),
        1024,
        1000,
        2000,
    );
    index1.insert(key1.clone(), metadata1);

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let metadata2 = test_entry_metadata(
        key2.clone(),
        PathBuf::from("/cache/entries/hash2.data"),
        2048,
        1100,
        2100,
    );
    index1.insert(key2.clone(), metadata2);

    // Save index to file
    index1.save_to_file(&index_path, &backend).await.unwrap();

    // Load index from file
    let index2 = CacheIndex::load_from_file(&index_path, &backend)
        .await
        .unwrap();

    // Verify loaded index has same entries
    let loaded_meta1 = index2.get(&key1).expect("key1 should exist");
    assert_eq!(loaded_meta1.cache_key, key1);
    assert_eq!(loaded_meta1.size_bytes, 1024);
    assert_eq!(loaded_meta1.created_at, 1000);
    assert_eq!(loaded_meta1.expires_at, 2000);

    let loaded_meta2 = index2.get(&key2).expect("key2 should exist");
    assert_eq!(loaded_meta2.cache_key, key2);
    assert_eq!(loaded_meta2.size_bytes, 2048);
    assert_eq!(loaded_meta2.created_at, 1100);
    assert_eq!(loaded_meta2.expires_at, 2100);

    // Verify total size matches
    assert_eq!(index2.total_size(), 3072);
    assert_eq!(index2.entry_count(), 2);
}

#[tokio::test]
async fn test_validates_index_against_filesystem() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with two entries
    let index = CacheIndex::new();

    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));

    let metadata1 = test_entry_metadata(
        key1.clone(),
        data_path1.clone(),
        1024,
        1000,
        9999999999, // Not expired
    );
    index.insert(key1.clone(), metadata1.clone());

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));
    let meta_path2 = entries_dir.join(format!("{}.meta", hash2));

    let metadata2 = test_entry_metadata(
        key2.clone(),
        data_path2.clone(),
        2048,
        1100,
        9999999999, // Not expired
    );
    index.insert(key2.clone(), metadata2.clone());

    // Create corresponding files in backend
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    backend
        .write_file_atomic(&data_path2, Bytes::from(vec![1u8; 2048]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path2,
            Bytes::from(serde_json::to_string(&metadata2).unwrap()),
        )
        .await
        .unwrap();

    // Validate and repair
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify both entries still exist
    assert!(index.get(&key1).is_some(), "key1 should still exist");
    assert!(index.get(&key2).is_some(), "key2 should still exist");

    // Verify total size is correct
    assert_eq!(index.total_size(), 3072);
    assert_eq!(index.entry_count(), 2);
}

#[tokio::test]
async fn test_removes_orphaned_files() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with one entry
    let index = CacheIndex::new();

    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));

    let metadata1 = test_entry_metadata(
        key1.clone(),
        data_path1.clone(),
        1024,
        1000,
        9999999999, // Not expired
    );
    index.insert(key1.clone(), metadata1.clone());

    // Create files for the valid entry
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Create orphaned files (not in index)
    let orphan_data = entries_dir.join("orphan_hash.data");
    let orphan_meta = entries_dir.join("orphan_hash.meta");

    backend
        .write_file_atomic(&orphan_data, Bytes::from(vec![0u8; 512]))
        .await
        .unwrap();
    backend
        .write_file_atomic(&orphan_meta, Bytes::from("orphaned metadata"))
        .await
        .unwrap();

    // Verify orphaned files exist before validation
    assert!(
        backend.read_file(&orphan_data).await.is_ok(),
        "Orphan data should exist before validation"
    );
    assert!(
        backend.read_file(&orphan_meta).await.is_ok(),
        "Orphan meta should exist before validation"
    );

    // Validate and repair
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify orphaned files are deleted
    assert!(
        backend.read_file(&orphan_data).await.is_err(),
        "Orphan data should be deleted after validation"
    );
    assert!(
        backend.read_file(&orphan_meta).await.is_err(),
        "Orphan meta should be deleted after validation"
    );

    // Verify valid entry still exists
    assert!(index.get(&key1).is_some(), "key1 should still exist");
    assert_eq!(index.total_size(), 1024);
    assert_eq!(index.entry_count(), 1);

    // Verify valid files still exist
    assert!(
        backend.read_file(&data_path1).await.is_ok(),
        "Valid data should still exist"
    );
    assert!(
        backend.read_file(&meta_path1).await.is_ok(),
        "Valid meta should still exist"
    );
}

#[tokio::test]
async fn test_removes_invalid_index_entries() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with two entries
    let index = CacheIndex::new();

    // Entry 1: Has both data and meta files (valid)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));

    let metadata1 = test_entry_metadata(
        key1.clone(),
        data_path1.clone(),
        1024,
        1000,
        9999999999, // Not expired
    );
    index.insert(key1.clone(), metadata1.clone());

    // Create files for entry 1
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Entry 2: Missing files (invalid - will be removed)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));

    let metadata2 = test_entry_metadata(
        key2.clone(),
        data_path2.clone(),
        2048,
        1100,
        9999999999, // Not expired
    );
    index.insert(key2.clone(), metadata2.clone());

    // Don't create files for entry 2 (simulating missing files)

    // Verify both entries exist before validation
    assert!(index.get(&key1).is_some(), "key1 should exist initially");
    assert!(index.get(&key2).is_some(), "key2 should exist initially");
    assert_eq!(index.total_size(), 3072);
    assert_eq!(index.entry_count(), 2);

    // Validate and repair
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify entry with missing files is removed
    assert!(
        index.get(&key2).is_none(),
        "key2 should be removed (files missing)"
    );

    // Verify valid entry still exists
    assert!(index.get(&key1).is_some(), "key1 should still exist");

    // Verify total size is updated (only key1 remains)
    assert_eq!(index.total_size(), 1024);
    assert_eq!(index.entry_count(), 1);
}

#[tokio::test]
async fn test_recalculates_total_size() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with entries that have incorrect size metadata
    let index = CacheIndex::new();

    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));

    // Metadata says 1024 bytes, but actual file will be 2048 bytes
    let metadata1 = test_entry_metadata(
        key1.clone(),
        data_path1.clone(),
        1024, // Incorrect size
        1000,
        9999999999,
    );
    index.insert(key1.clone(), metadata1.clone());

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));
    let meta_path2 = entries_dir.join(format!("{}.meta", hash2));

    // Metadata says 512 bytes, but actual file will be 1024 bytes
    let metadata2 = test_entry_metadata(
        key2.clone(),
        data_path2.clone(),
        512, // Incorrect size
        1100,
        9999999999,
    );
    index.insert(key2.clone(), metadata2.clone());

    // Create files with actual sizes different from metadata
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 2048])) // Actual: 2048
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    backend
        .write_file_atomic(&data_path2, Bytes::from(vec![1u8; 1024])) // Actual: 1024
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path2,
            Bytes::from(serde_json::to_string(&metadata2).unwrap()),
        )
        .await
        .unwrap();

    // Verify initial (incorrect) total size
    assert_eq!(
        index.total_size(),
        1536,
        "Initial total size should be 1024 + 512"
    );

    // Validate and repair
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify total size is recalculated based on actual file sizes
    assert_eq!(
        index.total_size(),
        3072,
        "Total size should be recalculated to 2048 + 1024"
    );

    // Verify metadata is updated with correct sizes
    let updated_meta1 = index.get(&key1).unwrap();
    assert_eq!(
        updated_meta1.size_bytes, 2048,
        "key1 size should be updated to actual size"
    );

    let updated_meta2 = index.get(&key2).unwrap();
    assert_eq!(
        updated_meta2.size_bytes, 1024,
        "key2 size should be updated to actual size"
    );

    assert_eq!(index.entry_count(), 2);
}

#[tokio::test]
async fn test_triggers_eviction_if_oversized_after_recovery() {
    use super::disk_cache::DiskCache;
    use super::utils::key_to_hash;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    // Create DiskCache with small max_size
    let cache_dir = PathBuf::from("/tmp/test_recovery_eviction");
    let max_size = 3000u64;
    let cache = DiskCache::with_config(cache_dir.clone(), max_size);

    // Simulate recovery by manually populating index with entries
    // Entry 1: 1000 bytes
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "old1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = cache_dir.join("entries").join(format!("{}.data", hash1));

    let metadata1 = test_entry_metadata(
        key1.clone(),
        data_path1.clone(),
        1000,
        1000, // Old access time
        9999999999,
    );
    cache.index.insert(key1.clone(), metadata1);

    // Entry 2: 1500 bytes (total now 2500)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "old2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = cache_dir.join("entries").join(format!("{}.data", hash2));

    let metadata2 = test_entry_metadata(
        key2.clone(),
        data_path2.clone(),
        1500,
        1100, // Newer access time than key1
        9999999999,
    );
    cache.index.insert(key2.clone(), metadata2);

    // Verify initial state
    assert_eq!(cache.index.total_size(), 2500);
    assert_eq!(cache.index.entry_count(), 2);

    // Try to add a new entry of 1000 bytes (would exceed 3000 total)
    let new_key = CacheKey {
        bucket: "test".to_string(),
        object_key: "new.txt".to_string(),
        etag: None,
    };

    let new_entry = CacheEntry {
        data: Bytes::from(vec![0u8; 1000]),
        content_type: "text/plain".to_string(),
        content_length: 1000,
        etag: "new_etag".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };

    // Add the new entry - this should trigger eviction of key1 (LRU)
    cache.set(new_key.clone(), new_entry).await.unwrap();

    // Verify key1 (oldest) was evicted
    assert!(
        cache.index.get(&key1).is_none(),
        "key1 should be evicted (oldest)"
    );

    // Verify key2 still exists
    assert!(cache.index.get(&key2).is_some(), "key2 should still exist");

    // Verify new_key was added
    assert!(
        cache.index.get(&new_key).is_some(),
        "new_key should be added"
    );

    // Verify total size is under limit (1500 + 1000 = 2500)
    assert_eq!(cache.index.total_size(), 2500);
    assert!(
        cache.index.total_size() <= max_size,
        "Total size should be under max_size"
    );

    // Verify eviction count was incremented
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.evictions, 1, "Should have 1 eviction");
}

// Phase 28.8: Corrupted Entry Handling

#[tokio::test]
async fn test_handles_corrupted_data_file() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with two entries
    let index = CacheIndex::new();

    // Entry 1: Has valid files
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));

    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 1024, 1000, 9999999999);
    index.insert(key1.clone(), metadata1.clone());

    // Create valid files for entry 1
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Entry 2: Has meta file but corrupted/missing data file
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));
    let meta_path2 = entries_dir.join(format!("{}.meta", hash2));

    let metadata2 = test_entry_metadata(key2.clone(), data_path2.clone(), 2048, 1100, 9999999999);
    index.insert(key2.clone(), metadata2.clone());

    // Create only meta file for entry 2 (data file missing/corrupted)
    backend
        .write_file_atomic(
            &meta_path2,
            Bytes::from(serde_json::to_string(&metadata2).unwrap()),
        )
        .await
        .unwrap();
    // Don't create data_path2 - simulating corruption/missing

    // Verify both entries exist before validation
    assert_eq!(index.entry_count(), 2);

    // Validate and repair - should handle corrupted entry gracefully
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify entry with corrupted data file is removed
    assert!(
        index.get(&key2).is_none(),
        "key2 should be removed (corrupted data)"
    );

    // Verify valid entry still exists
    assert!(index.get(&key1).is_some(), "key1 should still exist");

    // Verify total size and count updated
    assert_eq!(index.total_size(), 1024);
    assert_eq!(index.entry_count(), 1);
}

#[tokio::test]
async fn test_handles_corrupted_meta_file() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with two entries
    let index = CacheIndex::new();

    // Entry 1: Has valid files
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));

    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 1024, 1000, 9999999999);
    index.insert(key1.clone(), metadata1.clone());

    // Create valid files for entry 1
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Entry 2: Has data file but corrupted/missing meta file
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));
    let _meta_path2 = entries_dir.join(format!("{}.meta", hash2)); // Intentionally unused - simulating missing file

    let metadata2 = test_entry_metadata(key2.clone(), data_path2.clone(), 2048, 1100, 9999999999);
    index.insert(key2.clone(), metadata2.clone());

    // Create only data file for entry 2 (meta file missing/corrupted)
    backend
        .write_file_atomic(&data_path2, Bytes::from(vec![1u8; 2048]))
        .await
        .unwrap();
    // Don't create meta_path2 - simulating corruption/missing

    // Verify both entries exist before validation
    assert_eq!(index.entry_count(), 2);

    // Validate and repair - should handle corrupted entry gracefully
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify entry with corrupted meta file is removed
    assert!(
        index.get(&key2).is_none(),
        "key2 should be removed (corrupted meta)"
    );

    // Verify valid entry still exists
    assert!(index.get(&key1).is_some(), "key1 should still exist");

    // Verify total size and count updated
    assert_eq!(index.total_size(), 1024);
    assert_eq!(index.entry_count(), 1);
}

#[tokio::test]
async fn test_handles_corrupted_index_json() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let index_path = PathBuf::from("/cache/index.json");

    // Write corrupted JSON to index file
    let corrupted_json = "{ invalid json with missing quotes and braces";
    backend
        .write_file_atomic(&index_path, Bytes::from(corrupted_json))
        .await
        .unwrap();

    // Load index from corrupted file - should handle gracefully
    let result = CacheIndex::load_from_file(&index_path, &backend).await;

    // Should not return error - should create empty index instead
    assert!(result.is_ok(), "Should handle corrupted JSON gracefully");

    let index = result.unwrap();

    // Should return empty index when JSON is corrupted
    assert_eq!(
        index.entry_count(),
        0,
        "Corrupted index should result in empty index"
    );
    assert_eq!(index.total_size(), 0, "Total size should be 0");
}

#[tokio::test]
async fn test_logs_errors_but_continues_operation() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    // Create index with multiple entries, some corrupted
    let index = CacheIndex::new();

    // Entry 1: Valid
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "valid.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));
    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 1024, 1000, 9999999999);
    index.insert(key1.clone(), metadata1.clone());
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Entry 2: Missing data file
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "missing_data.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let meta_path2 = entries_dir.join(format!("{}.meta", hash2));
    let metadata2 = test_entry_metadata(
        key2.clone(),
        entries_dir.join(format!("{}.data", hash2)),
        512,
        1100,
        9999999999,
    );
    index.insert(key2.clone(), metadata2.clone());
    backend
        .write_file_atomic(
            &meta_path2,
            Bytes::from(serde_json::to_string(&metadata2).unwrap()),
        )
        .await
        .unwrap();

    // Entry 3: Missing meta file
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "missing_meta.txt".to_string(),
        etag: None,
    };
    let hash3 = key_to_hash(&key3);
    let data_path3 = entries_dir.join(format!("{}.data", hash3));
    let metadata3 = test_entry_metadata(key3.clone(), data_path3.clone(), 256, 1200, 9999999999);
    index.insert(key3.clone(), metadata3);
    backend
        .write_file_atomic(&data_path3, Bytes::from(vec![2u8; 256]))
        .await
        .unwrap();

    // Verify initial state
    assert_eq!(index.entry_count(), 3);

    // Validate and repair - should continue despite multiple errors
    let result = index.validate_and_repair(&entries_dir, &backend).await;

    // Should complete successfully (not fail despite errors)
    assert!(
        result.is_ok(),
        "Should continue operation despite corrupted entries"
    );

    // Verify only valid entry remains
    assert!(index.get(&key1).is_some(), "Valid entry should remain");
    assert!(
        index.get(&key2).is_none(),
        "Entry with missing data should be removed"
    );
    assert!(
        index.get(&key3).is_none(),
        "Entry with missing meta should be removed"
    );

    assert_eq!(index.entry_count(), 1, "Only 1 valid entry should remain");
    assert_eq!(index.total_size(), 1024);
}

#[tokio::test]
async fn test_removes_corrupted_entries_from_cache() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    let index = CacheIndex::new();

    // Entry 1: Valid (not expired)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "valid.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));
    let metadata1 = test_entry_metadata(
        key1.clone(),
        data_path1.clone(),
        1024,
        1000,
        9999999999, // Far future expiry
    );
    index.insert(key1.clone(), metadata1.clone());
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Entry 2: Expired (corrupted by time)
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "expired.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));
    let meta_path2 = entries_dir.join(format!("{}.meta", hash2));
    let metadata2 = test_entry_metadata(
        key2.clone(),
        data_path2.clone(),
        512,
        1100,
        1000, // Expired (in the past)
    );
    index.insert(key2.clone(), metadata2.clone());
    backend
        .write_file_atomic(&data_path2, Bytes::from(vec![1u8; 512]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path2,
            Bytes::from(serde_json::to_string(&metadata2).unwrap()),
        )
        .await
        .unwrap();

    // Verify initial state
    assert_eq!(index.entry_count(), 2);

    // Verify files exist before validation
    assert!(
        backend.read_file(&data_path2).await.is_ok(),
        "Expired entry files should exist before validation"
    );

    // Validate and repair - should remove expired entry
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify expired entry is removed from index
    assert!(
        index.get(&key2).is_none(),
        "Expired entry should be removed from index"
    );

    // Verify expired entry files are deleted
    assert!(
        backend.read_file(&data_path2).await.is_err(),
        "Expired entry data file should be deleted"
    );
    assert!(
        backend.read_file(&meta_path2).await.is_err(),
        "Expired entry meta file should be deleted"
    );

    // Verify valid entry still exists
    assert!(index.get(&key1).is_some(), "Valid entry should remain");
    assert!(
        backend.read_file(&data_path1).await.is_ok(),
        "Valid entry files should still exist"
    );

    assert_eq!(index.entry_count(), 1);
    assert_eq!(index.total_size(), 1024);
}

// Phase 28.8: Temporary File Cleanup

#[tokio::test]
async fn test_deletes_tmp_files_from_failed_writes() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    let index = CacheIndex::new();

    // Create a valid entry
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));
    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 1024, 1000, 9999999999);
    index.insert(key1.clone(), metadata1.clone());
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    // Create orphaned .tmp files (from failed writes)
    let tmp_file1 = entries_dir.join("somehash.data.tmp");
    let tmp_file2 = entries_dir.join("anotherhash.meta.tmp");

    backend
        .write_file_atomic(&tmp_file1, Bytes::from(vec![0u8; 100]))
        .await
        .unwrap();
    backend
        .write_file_atomic(&tmp_file2, Bytes::from("temp metadata"))
        .await
        .unwrap();

    // Verify .tmp files exist before cleanup
    assert!(
        backend.read_file(&tmp_file1).await.is_ok(),
        "Temp file 1 should exist before cleanup"
    );
    assert!(
        backend.read_file(&tmp_file2).await.is_ok(),
        "Temp file 2 should exist before cleanup"
    );

    // Validate and repair - should clean up .tmp files
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify .tmp files are deleted
    assert!(
        backend.read_file(&tmp_file1).await.is_err(),
        "Temp file 1 should be deleted after cleanup"
    );
    assert!(
        backend.read_file(&tmp_file2).await.is_err(),
        "Temp file 2 should be deleted after cleanup"
    );

    // Verify valid entry still exists
    assert!(index.get(&key1).is_some(), "Valid entry should remain");
    assert!(
        backend.read_file(&data_path1).await.is_ok(),
        "Valid data file should still exist"
    );
}

#[tokio::test]
async fn test_doesnt_delete_legitimate_files() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::mock_backend::MockDiskBackend;
    use super::utils::key_to_hash;
    use crate::cache::CacheKey;
    use bytes::Bytes;
    use std::path::PathBuf;

    let backend = MockDiskBackend::new();
    let entries_dir = PathBuf::from("/cache/entries");

    let index = CacheIndex::new();

    // Create valid entries with .data and .meta files
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let hash1 = key_to_hash(&key1);
    let data_path1 = entries_dir.join(format!("{}.data", hash1));
    let meta_path1 = entries_dir.join(format!("{}.meta", hash1));
    let metadata1 = test_entry_metadata(key1.clone(), data_path1.clone(), 1024, 1000, 9999999999);
    index.insert(key1.clone(), metadata1.clone());
    backend
        .write_file_atomic(&data_path1, Bytes::from(vec![0u8; 1024]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path1,
            Bytes::from(serde_json::to_string(&metadata1).unwrap()),
        )
        .await
        .unwrap();

    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let hash2 = key_to_hash(&key2);
    let data_path2 = entries_dir.join(format!("{}.data", hash2));
    let meta_path2 = entries_dir.join(format!("{}.meta", hash2));
    let metadata2 = test_entry_metadata(key2.clone(), data_path2.clone(), 512, 1100, 9999999999);
    index.insert(key2.clone(), metadata2.clone());
    backend
        .write_file_atomic(&data_path2, Bytes::from(vec![1u8; 512]))
        .await
        .unwrap();
    backend
        .write_file_atomic(
            &meta_path2,
            Bytes::from(serde_json::to_string(&metadata2).unwrap()),
        )
        .await
        .unwrap();

    // Create a .tmp file (should be deleted)
    let tmp_file = entries_dir.join("orphan.tmp");
    backend
        .write_file_atomic(&tmp_file, Bytes::from("temp"))
        .await
        .unwrap();

    // Verify all files exist before cleanup
    assert!(backend.read_file(&data_path1).await.is_ok());
    assert!(backend.read_file(&meta_path1).await.is_ok());
    assert!(backend.read_file(&data_path2).await.is_ok());
    assert!(backend.read_file(&meta_path2).await.is_ok());
    assert!(backend.read_file(&tmp_file).await.is_ok());

    // Validate and repair
    index
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // Verify .data and .meta files are NOT deleted (legitimate files preserved)
    assert!(
        backend.read_file(&data_path1).await.is_ok(),
        "Legitimate .data file should not be deleted"
    );
    assert!(
        backend.read_file(&meta_path1).await.is_ok(),
        "Legitimate .meta file should not be deleted"
    );
    assert!(
        backend.read_file(&data_path2).await.is_ok(),
        "Legitimate .data file should not be deleted"
    );
    assert!(
        backend.read_file(&meta_path2).await.is_ok(),
        "Legitimate .meta file should not be deleted"
    );

    // Verify .tmp file IS deleted
    assert!(
        backend.read_file(&tmp_file).await.is_err(),
        "Temp file should be deleted"
    );

    // Verify index entries are intact
    assert_eq!(index.entry_count(), 2);
    assert!(index.get(&key1).is_some());
    assert!(index.get(&key2).is_some());
}

// Phase 28.9: Cache Trait Implementation - DiskCache Structure

#[test]
fn test_can_create_diskcache() {
    use super::disk_cache::DiskCache;

    // Test default constructor
    let cache1 = DiskCache::new();
    assert_eq!(cache1.index.entry_count(), 0);

    // Test with_config constructor
    let cache_dir = std::path::PathBuf::from("/tmp/test_cache");
    let max_size = 1024 * 1024 * 10; // 10MB
    let cache2 = DiskCache::with_config(cache_dir, max_size);
    assert_eq!(cache2.index.entry_count(), 0);
}

// Phase 28.9: Cache Trait Implementation - Cache::get()

#[tokio::test]
async fn test_cache_get_returns_none_if_not_found() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheKey};

    let cache = DiskCache::new();
    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "nonexistent.txt".to_string(),
        etag: None,
    };

    let result = cache.get(&key).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_cache_set_and_get_roundtrip() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::new();
    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "test.txt".to_string(),
        etag: Some("etag123".to_string()),
    };

    let data = Bytes::from("Hello, World!");
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "text/plain".to_string(),
        content_length: data.len(),
        etag: "etag123".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };

    // Set entry
    cache.set(key.clone(), entry).await.unwrap();

    // Get entry back
    let retrieved = cache.get(&key).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_entry = retrieved.unwrap();
    assert_eq!(retrieved_entry.data, data);
    assert_eq!(retrieved_entry.content_length, data.len());
}

#[tokio::test]
async fn test_cache_get_returns_none_if_expired() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::new();
    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "expired.txt".to_string(),
        etag: None,
    };

    let data = Bytes::from("Expired content");
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "text/plain".to_string(),
        content_length: data.len(),
        etag: "".to_string(),
        last_modified: None,
        created_at: SystemTime::now() - Duration::from_secs(7200),
        expires_at: SystemTime::now() - Duration::from_secs(3600), // Expired 1 hour ago
        last_accessed_at: SystemTime::now() - Duration::from_secs(7200),
    };

    // Set entry
    cache.set(key.clone(), entry).await.unwrap();

    // Try to get expired entry - should return None
    let retrieved = cache.get(&key).await.unwrap();
    assert!(retrieved.is_none(), "Expired entry should return None");

    // Verify it was removed from index
    assert!(cache.index.get(&key).is_none());
}

#[tokio::test]
async fn test_cache_delete_removes_entry() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::new();
    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "delete_me.txt".to_string(),
        etag: None,
    };

    let data = Bytes::from("To be deleted");
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "text/plain".to_string(),
        content_length: data.len(),
        etag: "".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };

    // Set entry
    cache.set(key.clone(), entry).await.unwrap();
    assert!(cache.index.get(&key).is_some());

    // Delete entry
    let deleted = cache.delete(&key).await.unwrap();
    assert!(deleted, "Should return true when entry exists");

    // Verify it's gone
    assert!(cache.index.get(&key).is_none());
    let retrieved = cache.get(&key).await.unwrap();
    assert!(retrieved.is_none());

    // Delete again - should return false
    let deleted_again = cache.delete(&key).await.unwrap();
    assert!(
        !deleted_again,
        "Should return false when entry doesn't exist"
    );
}

#[tokio::test]
async fn test_cache_stats() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::with_config(
        std::path::PathBuf::from("/tmp/test_cache_stats"),
        1024 * 10, // 10KB max
    );

    // Check initial stats
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, 0);
    assert_eq!(stats.current_size_bytes, 0);
    assert_eq!(stats.evictions, 0);

    // Add an entry
    let key = CacheKey {
        bucket: "test".to_string(),
        object_key: "stats_test.txt".to_string(),
        etag: None,
    };

    let data = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: "".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };

    cache.set(key.clone(), entry).await.unwrap();

    // Check updated stats
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, 1);
    assert_eq!(stats.current_size_bytes, 1024);
    assert_eq!(stats.max_size_bytes, 1024 * 10);
}

#[tokio::test]
async fn test_cache_clear_removes_all_entries() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::new();

    // Add multiple entries
    for i in 0..5 {
        let key = CacheKey {
            bucket: "test".to_string(),
            object_key: format!("file{}.txt", i),
            etag: None,
        };

        let data = Bytes::from(format!("Content {}", i));
        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: "".to_string(),
            last_modified: None,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            last_accessed_at: SystemTime::now(),
        };

        cache.set(key.clone(), entry).await.unwrap();
    }

    // Verify entries exist
    assert_eq!(cache.index.entry_count(), 5);

    // Clear cache
    cache.clear().await.unwrap();

    // Verify all entries are gone
    assert_eq!(cache.index.entry_count(), 0);
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, 0);
    assert_eq!(stats.current_size_bytes, 0);
}

#[tokio::test]
async fn test_cache_set_triggers_eviction_when_full() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::with_config(
        std::path::PathBuf::from("/tmp/test_eviction"),
        2500, // Small cache: 2.5KB
    );

    // Add first entry (1KB)
    let key1 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file1.txt".to_string(),
        etag: None,
    };
    let data1 = Bytes::from(vec![0u8; 1024]);
    let entry1 = CacheEntry {
        data: data1.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data1.len(),
        etag: "".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key1.clone(), entry1).await.unwrap();

    // Add second entry (1KB)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await; // Ensure different timestamp
    let key2 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file2.txt".to_string(),
        etag: None,
    };
    let data2 = Bytes::from(vec![1u8; 1024]);
    let entry2 = CacheEntry {
        data: data2.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data2.len(),
        etag: "".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key2.clone(), entry2).await.unwrap();

    // Add third entry (1KB) - should trigger eviction of key1 (oldest)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let key3 = CacheKey {
        bucket: "test".to_string(),
        object_key: "file3.txt".to_string(),
        etag: None,
    };
    let data3 = Bytes::from(vec![2u8; 1024]);
    let entry3 = CacheEntry {
        data: data3.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data3.len(),
        etag: "".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key3.clone(), entry3).await.unwrap();

    // Verify key1 was evicted (LRU)
    assert!(cache.index.get(&key1).is_none(), "key1 should be evicted");
    assert!(cache.index.get(&key2).is_some(), "key2 should remain");
    assert!(cache.index.get(&key3).is_some(), "key3 should remain");

    // Verify stats
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, 2);
    assert_eq!(stats.evictions, 1);
}

#[tokio::test]
async fn test_cache_stores_multiple_entries() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let cache = DiskCache::new();

    let entries_count = 10;
    let mut keys = Vec::new();

    // Add multiple entries
    for i in 0..entries_count {
        let key = CacheKey {
            bucket: format!("bucket{}", i % 3), // 3 different buckets
            object_key: format!("file{}.txt", i),
            etag: Some(format!("etag{}", i)),
        };

        let data = Bytes::from(format!("Content for file {}", i));
        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: format!("etag{}", i),
            last_modified: None,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            last_accessed_at: SystemTime::now(),
        };

        cache.set(key.clone(), entry).await.unwrap();
        keys.push(key);
    }

    // Verify all entries can be retrieved
    for (i, key) in keys.iter().enumerate() {
        let retrieved = cache.get(key).await.unwrap();
        assert!(retrieved.is_some(), "Entry {} should exist", i);

        let entry = retrieved.unwrap();
        assert_eq!(entry.data, Bytes::from(format!("Content for file {}", i)));
    }

    // Verify stats
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.current_item_count, entries_count as u64);
}

// ========================================
// Phase 28.10: Cross-Platform Integration Tests
// ========================================

#[tokio::test]
async fn test_integration_store_and_retrieve_100_entries() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 100 * 1024 * 1024); // 100MB

    // Store 100 entries
    let mut keys = Vec::new();
    for i in 0..100 {
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: format!("object-{:03}.txt", i),
            etag: None,
        };

        let data = Bytes::from(format!("Test data for entry {}", i));
        let entry = CacheEntry {
            data: data.clone(),
            content_type: "text/plain".to_string(),
            content_length: data.len(),
            etag: format!("etag-{}", i),
            last_modified: None,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            last_accessed_at: SystemTime::now(),
        };

        cache.set(key.clone(), entry).await.unwrap();
        keys.push(key);
    }

    // Verify all 100 entries can be retrieved
    for (i, key) in keys.iter().enumerate() {
        let retrieved = cache.get(key).await.unwrap();
        assert!(retrieved.is_some(), "Entry {} should exist in cache", i);

        let entry = retrieved.unwrap();
        assert_eq!(
            entry.data,
            Bytes::from(format!("Test data for entry {}", i)),
            "Entry {} data should match",
            i
        );
    }

    // Verify stats
    let stats = cache.stats().await.unwrap();
    assert_eq!(
        stats.current_item_count, 100,
        "Cache should contain 100 entries"
    );
}

#[tokio::test]
async fn test_integration_index_persistence_and_recovery() {
    use super::backend::DiskBackend;
    use super::index::CacheIndex;
    use super::tokio_backend::TokioFsBackend;
    use crate::cache::CacheKey;
    use bytes::Bytes;

    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    let index_path = cache_dir.join("index.json");
    let entries_dir = cache_dir.join("entries");
    let backend = TokioFsBackend::new();

    // Create entries directory
    backend.create_dir_all(&entries_dir).await.unwrap();

    // Phase 1: Create index, add entries, save to disk
    let index1 = CacheIndex::new();
    for i in 0..10 {
        let key = CacheKey {
            bucket: "persistent".to_string(),
            object_key: format!("file-{}.dat", i),
            etag: None,
        };

        let hash = super::utils::key_to_hash(&key);
        let data_path = entries_dir.join(format!("{}.data", hash));
        let meta_path = entries_dir.join(format!("{}.meta", hash));

        // Write actual data file
        let data = Bytes::from(vec![i as u8; 1024]);
        backend.write_file_atomic(&data_path, data).await.unwrap();

        // Create metadata
        let metadata = test_entry_metadata(
            key.clone(),
            data_path.clone(),
            1024,
            1000,
            9999999999, // Far future expiration
        );

        // Write metadata file
        let meta_json = serde_json::to_string(&metadata).unwrap();
        backend
            .write_file_atomic(&meta_path, Bytes::from(meta_json))
            .await
            .unwrap();

        index1.insert(key, metadata);
    }

    // Save index to disk
    index1.save_to_file(&index_path, &backend).await.unwrap();

    // Verify index was saved
    assert_eq!(index1.entry_count(), 10);
    assert_eq!(index1.total_size(), 10 * 1024);

    // Phase 2: Load index from disk and validate
    let index2 = CacheIndex::load_from_file(&index_path, &backend)
        .await
        .unwrap();

    assert_eq!(
        index2.entry_count(),
        10,
        "Loaded index should contain all 10 entries"
    );
    assert_eq!(
        index2.total_size(),
        10 * 1024,
        "Loaded index should have correct total size"
    );

    // Phase 3: Validate and repair
    index2
        .validate_and_repair(&entries_dir, &backend)
        .await
        .unwrap();

    // After validation, all entries should still be present (nothing expired or missing)
    assert_eq!(
        index2.entry_count(),
        10,
        "Index should still have 10 entries after validation"
    );

    // Verify we can look up entries in the loaded index
    for i in 0..10 {
        let key = CacheKey {
            bucket: "persistent".to_string(),
            object_key: format!("file-{}.dat", i),
            etag: None,
        };

        let metadata = index2.get(&key);
        assert!(metadata.is_some(), "Entry {} should be in loaded index", i);

        let metadata = metadata.unwrap();
        assert_eq!(metadata.size_bytes, 1024);
    }
}

#[tokio::test]
async fn test_integration_lru_eviction_end_to_end() {
    use super::disk_cache::DiskCache;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    // Create cache with 5KB limit
    let cache = DiskCache::with_config(temp_dir.path().to_path_buf(), 5 * 1024);

    // Add 3 entries, 2KB each (total 6KB - will trigger eviction)
    for i in 0..3 {
        let key = CacheKey {
            bucket: "eviction-test".to_string(),
            object_key: format!("large-file-{}.bin", i),
            etag: None,
        };

        let data = Bytes::from(vec![i as u8; 2 * 1024]);
        let entry = CacheEntry {
            data: data.clone(),
            content_type: "application/octet-stream".to_string(),
            content_length: data.len(),
            etag: "".to_string(),
            last_modified: None,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            last_accessed_at: SystemTime::now(),
        };

        cache.set(key, entry).await.unwrap();

        // Sleep to ensure distinct timestamps for LRU ordering
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // First entry should have been evicted (LRU)
    let key0 = CacheKey {
        bucket: "eviction-test".to_string(),
        object_key: "large-file-0.bin".to_string(),
        etag: None,
    };
    assert!(
        cache.get(&key0).await.unwrap().is_none(),
        "Oldest entry should be evicted"
    );

    // Second and third entries should still exist
    let key1 = CacheKey {
        bucket: "eviction-test".to_string(),
        object_key: "large-file-1.bin".to_string(),
        etag: None,
    };
    let key2 = CacheKey {
        bucket: "eviction-test".to_string(),
        object_key: "large-file-2.bin".to_string(),
        etag: None,
    };

    assert!(
        cache.get(&key1).await.unwrap().is_some(),
        "Second entry should remain"
    );
    assert!(
        cache.get(&key2).await.unwrap().is_some(),
        "Third entry should remain"
    );

    // Verify stats show eviction
    let stats = cache.stats().await.unwrap();
    assert_eq!(stats.evictions, 1, "Stats should show 1 eviction");
}

// ========================================
// Phase 28.10: Error Injection Tests
// ========================================

#[tokio::test]
async fn test_error_injection_disk_full() {
    use super::backend::DiskBackend;
    use super::mock_backend::MockDiskBackend;

    use bytes::Bytes;

    let backend = MockDiskBackend::new();
    let cache_dir = std::path::PathBuf::from("/tmp/test_disk_full");

    // Simulate disk full condition
    backend.set_storage_full(true);

    // Try to write a file - should fail with storage full error
    let data = Bytes::from(vec![0u8; 1024]);
    let result = backend
        .write_file_atomic(&cache_dir.join("test.data"), data)
        .await;

    assert!(result.is_err(), "Write should fail when disk is full");

    // Verify we can still read (doesn't require disk space)
    backend.set_storage_full(false);
    backend
        .write_file_atomic(&cache_dir.join("test.data"), Bytes::from(vec![1u8; 512]))
        .await
        .unwrap();

    backend.set_storage_full(true);
    let read_result = backend.read_file(&cache_dir.join("test.data")).await;
    assert!(
        read_result.is_ok(),
        "Read should work even when disk is full"
    );
}

#[tokio::test]
async fn test_error_injection_permission_denied() {
    use super::backend::DiskBackend;
    use super::mock_backend::MockDiskBackend;
    use bytes::Bytes;

    let backend = MockDiskBackend::new();
    let cache_dir = std::path::PathBuf::from("/tmp/test_permission");

    // First write a file successfully
    backend
        .write_file_atomic(&cache_dir.join("test.data"), Bytes::from(vec![0u8; 512]))
        .await
        .unwrap();

    // Simulate permission denied
    backend.set_permission_denied(true);

    // Try to read - should fail
    let result = backend.read_file(&cache_dir.join("test.data")).await;
    assert!(result.is_err(), "Read should fail with permission denied");

    // Try to write - should fail
    let result = backend
        .write_file_atomic(&cache_dir.join("test2.data"), Bytes::from(vec![1u8; 512]))
        .await;
    assert!(result.is_err(), "Write should fail with permission denied");

    // Try to delete - should fail
    let result = backend.delete_file(&cache_dir.join("test.data")).await;
    assert!(result.is_err(), "Delete should fail with permission denied");
}

// ========================================
// Phase v1.4: sendfile Tests
// ========================================

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_get_sendfile_returns_path_for_large_files() {
    use super::disk_cache::DiskCache;
    use crate::cache::sendfile::SendfileConfig;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    // Create cache with sendfile enabled and 1KB threshold
    let sendfile_config = SendfileConfig {
        enabled: true,
        threshold_bytes: 1024, // 1KB threshold for testing
    };
    let cache = DiskCache::with_sendfile_config(
        temp_dir.path().to_path_buf(),
        100 * 1024 * 1024,
        sendfile_config,
    );

    // Create a large file (2KB - above threshold)
    let key = CacheKey {
        bucket: "sendfile-test".to_string(),
        object_key: "large-file.bin".to_string(),
        etag: None,
    };
    let data = Bytes::from(vec![0u8; 2 * 1024]); // 2KB
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: "test-etag".to_string(),
        last_modified: Some("Tue, 15 Jan 2024 12:00:00 GMT".to_string()),
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key.clone(), entry).await.unwrap();

    // get_sendfile should return a response for large files
    let result = cache.get_sendfile(&key).await.unwrap();
    assert!(
        result.is_some(),
        "Should return sendfile response for large files"
    );

    let response = result.unwrap();
    assert_eq!(response.length, 2 * 1024);
    assert_eq!(response.content_type, "application/octet-stream");
    assert_eq!(response.etag, Some("test-etag".to_string()));
    assert!(response.file_path.exists(), "File path should exist");
}

#[tokio::test]
async fn test_get_sendfile_returns_none_for_small_files() {
    use super::disk_cache::DiskCache;
    use crate::cache::sendfile::SendfileConfig;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    // Create cache with sendfile enabled and 64KB threshold
    let sendfile_config = SendfileConfig {
        enabled: true,
        threshold_bytes: 64 * 1024, // 64KB threshold
    };
    let cache = DiskCache::with_sendfile_config(
        temp_dir.path().to_path_buf(),
        100 * 1024 * 1024,
        sendfile_config,
    );

    // Create a small file (1KB - below threshold)
    let key = CacheKey {
        bucket: "sendfile-test".to_string(),
        object_key: "small-file.bin".to_string(),
        etag: None,
    };
    let data = Bytes::from(vec![0u8; 1024]); // 1KB
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "text/plain".to_string(),
        content_length: data.len(),
        etag: "small-etag".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key.clone(), entry).await.unwrap();

    // get_sendfile should return None for small files
    let result = cache.get_sendfile(&key).await.unwrap();
    assert!(
        result.is_none(),
        "Should return None for files below threshold"
    );

    // Regular get should still work
    let entry = cache.get(&key).await.unwrap();
    assert!(entry.is_some(), "Regular get should work for small files");
}

#[tokio::test]
async fn test_get_sendfile_returns_none_when_disabled() {
    use super::disk_cache::DiskCache;
    use crate::cache::sendfile::SendfileConfig;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    // Create cache with sendfile disabled
    let sendfile_config = SendfileConfig {
        enabled: false,
        threshold_bytes: 1024,
    };
    let cache = DiskCache::with_sendfile_config(
        temp_dir.path().to_path_buf(),
        100 * 1024 * 1024,
        sendfile_config,
    );

    // Create a large file
    let key = CacheKey {
        bucket: "sendfile-test".to_string(),
        object_key: "large-file-disabled.bin".to_string(),
        etag: None,
    };
    let data = Bytes::from(vec![0u8; 2 * 1024]); // 2KB
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: "test-etag".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key.clone(), entry).await.unwrap();

    // get_sendfile should return None when disabled
    let result = cache.get_sendfile(&key).await.unwrap();
    assert!(
        result.is_none(),
        "Should return None when sendfile is disabled"
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_get_sendfile_tracks_cache_hits() {
    use super::disk_cache::DiskCache;
    use crate::cache::sendfile::SendfileConfig;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    let sendfile_config = SendfileConfig {
        enabled: true,
        threshold_bytes: 1024,
    };
    let cache = DiskCache::with_sendfile_config(
        temp_dir.path().to_path_buf(),
        100 * 1024 * 1024,
        sendfile_config,
    );

    let key = CacheKey {
        bucket: "sendfile-test".to_string(),
        object_key: "hit-tracking.bin".to_string(),
        etag: None,
    };
    let data = Bytes::from(vec![0u8; 2 * 1024]); // 2KB
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: "test-etag".to_string(),
        last_modified: None,
        created_at: SystemTime::now(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        last_accessed_at: SystemTime::now(),
    };
    cache.set(key.clone(), entry).await.unwrap();

    // Initial stats
    let stats = cache.stats().await.unwrap();
    let initial_hits = stats.hits;

    // Call get_sendfile - should NOT increment hit count
    // (the proxy calls cache.get() first, which already tracks the hit)
    let sendfile_result = cache.get_sendfile(&key).await.unwrap();
    assert!(sendfile_result.is_some(), "Should return sendfile response");

    // Check stats did NOT increase (hit tracking is done by cache.get(), not get_sendfile())
    let stats = cache.stats().await.unwrap();
    assert_eq!(
        stats.hits, initial_hits,
        "get_sendfile should NOT increment hit count (tracked by get() instead)"
    );
}

#[tokio::test]
async fn test_get_sendfile_returns_none_for_nonexistent_key() {
    use super::disk_cache::DiskCache;
    use crate::cache::sendfile::SendfileConfig;
    use crate::cache::{Cache, CacheKey};

    let temp_dir = tempfile::tempdir().unwrap();
    let sendfile_config = SendfileConfig {
        enabled: true,
        threshold_bytes: 1024,
    };
    let cache = DiskCache::with_sendfile_config(
        temp_dir.path().to_path_buf(),
        100 * 1024 * 1024,
        sendfile_config,
    );

    let key = CacheKey {
        bucket: "sendfile-test".to_string(),
        object_key: "nonexistent.bin".to_string(),
        etag: None,
    };

    // get_sendfile should return None for nonexistent keys
    let result = cache.get_sendfile(&key).await.unwrap();
    assert!(result.is_none(), "Should return None for nonexistent key");
}

#[tokio::test]
async fn test_get_sendfile_returns_none_for_expired_entry() {
    use super::disk_cache::DiskCache;
    use crate::cache::sendfile::SendfileConfig;
    use crate::cache::{Cache, CacheEntry, CacheKey};
    use bytes::Bytes;
    use std::time::{Duration, SystemTime};

    let temp_dir = tempfile::tempdir().unwrap();
    let sendfile_config = SendfileConfig {
        enabled: true,
        threshold_bytes: 1024,
    };
    let cache = DiskCache::with_sendfile_config(
        temp_dir.path().to_path_buf(),
        100 * 1024 * 1024,
        sendfile_config,
    );

    let key = CacheKey {
        bucket: "sendfile-test".to_string(),
        object_key: "expired.bin".to_string(),
        etag: None,
    };
    let data = Bytes::from(vec![0u8; 2 * 1024]); // 2KB
    let entry = CacheEntry {
        data: data.clone(),
        content_type: "application/octet-stream".to_string(),
        content_length: data.len(),
        etag: "test-etag".to_string(),
        last_modified: None,
        created_at: SystemTime::now() - Duration::from_secs(10),
        expires_at: SystemTime::now() - Duration::from_secs(1), // Already expired
        last_accessed_at: SystemTime::now() - Duration::from_secs(10),
    };
    cache.set(key.clone(), entry).await.unwrap();

    // get_sendfile should return None for expired entries
    let result = cache.get_sendfile(&key).await.unwrap();
    assert!(result.is_none(), "Should return None for expired entry");
}
