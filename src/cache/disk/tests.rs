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
}
