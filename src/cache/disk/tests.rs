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
}
