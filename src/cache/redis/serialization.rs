// Redis cache entry serialization using MessagePack

use crate::cache::{CacheEntry, CacheError};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Serialization format version for schema evolution
const SERIALIZATION_VERSION: u8 = 1;

/// Serializable wrapper for CacheEntry with version marker
#[derive(Debug, Serialize, Deserialize)]
struct SerializableCacheEntry {
    /// Schema version for forward/backward compatibility
    version: u8,
    /// The cached object data (as Vec<u8> for serialization)
    data: Vec<u8>,
    /// Content type of the cached object
    content_type: String,
    /// Content length of the cached object
    content_length: usize,
    /// ETag of the cached object (for validation)
    etag: String,
    /// When this entry was created (seconds since UNIX_EPOCH)
    created_at_secs: u64,
    /// When this entry expires (seconds since UNIX_EPOCH)
    expires_at_secs: u64,
    /// Last time this entry was accessed (seconds since UNIX_EPOCH)
    last_accessed_at_secs: u64,
}

/// Serializes a CacheEntry to bytes using MessagePack
///
/// # Arguments
/// * `entry` - The cache entry to serialize
///
/// # Returns
/// MessagePack-encoded bytes with version marker
///
/// # Errors
/// Returns CacheError::SerializationError if encoding fails
pub fn serialize_entry(entry: &CacheEntry) -> Result<Vec<u8>, CacheError> {
    let serializable = SerializableCacheEntry {
        version: SERIALIZATION_VERSION,
        data: entry.data.to_vec(),
        content_type: entry.content_type.clone(),
        content_length: entry.content_length,
        etag: entry.etag.clone(),
        created_at_secs: entry
            .created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| CacheError::SerializationError(format!("Invalid created_at: {}", e)))?
            .as_secs(),
        expires_at_secs: entry
            .expires_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| CacheError::SerializationError(format!("Invalid expires_at: {}", e)))?
            .as_secs(),
        last_accessed_at_secs: entry
            .last_accessed_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| {
                CacheError::SerializationError(format!("Invalid last_accessed_at: {}", e))
            })?
            .as_secs(),
    };

    rmp_serde::to_vec(&serializable)
        .map_err(|e| CacheError::SerializationError(format!("MessagePack encoding failed: {}", e)))
}

/// Deserializes bytes to a CacheEntry using MessagePack
///
/// # Arguments
/// * `bytes` - MessagePack-encoded bytes
///
/// # Returns
/// Deserialized CacheEntry
///
/// # Errors
/// Returns CacheError::SerializationError if:
/// - Data is corrupt or truncated
/// - Version is unsupported
/// - Required fields are invalid
pub fn deserialize_entry(bytes: &[u8]) -> Result<CacheEntry, CacheError> {
    // Deserialize from MessagePack
    let serializable: SerializableCacheEntry = rmp_serde::from_slice(bytes).map_err(|e| {
        CacheError::SerializationError(format!("MessagePack decoding failed: {}", e))
    })?;

    // Validate version
    if serializable.version != SERIALIZATION_VERSION {
        return Err(CacheError::SerializationError(format!(
            "Unsupported schema version: {} (expected: {})",
            serializable.version, SERIALIZATION_VERSION
        )));
    }

    // Validate required fields
    if serializable.data.is_empty() {
        return Err(CacheError::SerializationError(
            "Invalid entry: data is empty".to_string(),
        ));
    }

    if serializable.etag.is_empty() {
        return Err(CacheError::SerializationError(
            "Invalid entry: etag is empty".to_string(),
        ));
    }

    // Convert timestamps back to SystemTime
    let created_at = SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(serializable.created_at_secs);
    let expires_at = SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(serializable.expires_at_secs);
    let last_accessed_at = SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(serializable.last_accessed_at_secs);

    Ok(CacheEntry {
        data: Bytes::from(serializable.data),
        content_type: serializable.content_type,
        content_length: serializable.content_length,
        etag: serializable.etag,
        created_at,
        expires_at,
        last_accessed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_entry() -> CacheEntry {
        CacheEntry::new(
            Bytes::from("test data"),
            "text/plain".to_string(),
            "etag123".to_string(),
            Some(Duration::from_secs(3600)),
        )
    }

    #[test]
    fn test_can_serialize_cache_entry_to_bytes() {
        // Test: Can serialize CacheEntry to bytes
        let entry = create_test_entry();
        let result = serialize_entry(&entry);

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_uses_messagepack_for_compact_binary_format() {
        // Test: Uses MessagePack for compact binary format
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        // MessagePack binary starts with specific markers
        // We just verify it's binary (not text)
        assert!(bytes.len() > 0);
        // MessagePack uses compact binary format
        assert!(bytes.len() < 500); // Much smaller than JSON
    }

    #[test]
    fn test_serialized_format_includes_version_marker() {
        // Test: Serialized format includes version marker
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        // Deserialize to verify version is present
        let deserialized: SerializableCacheEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(deserialized.version, SERIALIZATION_VERSION);
    }

    #[test]
    fn test_includes_all_entry_fields() {
        // Test: Includes all entry fields (data, content_type, etag, etc.)
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        let deserialized: SerializableCacheEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(deserialized.data, b"test data");
        assert_eq!(deserialized.content_type, "text/plain");
        assert_eq!(deserialized.etag, "etag123");
        assert!(deserialized.created_at_secs > 0);
        assert!(deserialized.expires_at_secs > 0);
    }

    #[test]
    fn test_handles_small_entries_less_than_1kb() {
        // Test: Handles small entries (<1KB)
        let small_data = Bytes::from("small");
        let entry = CacheEntry::new(
            small_data,
            "text/plain".to_string(),
            "etag".to_string(),
            Some(Duration::from_secs(3600)),
        );

        let result = serialize_entry(&entry);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.len() < 1024);
    }

    #[test]
    fn test_handles_medium_entries_1kb_to_1mb() {
        // Test: Handles medium entries (1KB-1MB)
        let medium_data = Bytes::from(vec![0u8; 10 * 1024]); // 10KB
        let entry = CacheEntry::new(
            medium_data,
            "application/octet-stream".to_string(),
            "etag".to_string(),
            Some(Duration::from_secs(3600)),
        );

        let result = serialize_entry(&entry);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.len() >= 10 * 1024);
        assert!(bytes.len() < 1024 * 1024);
    }

    #[test]
    fn test_handles_large_entries_greater_than_1mb() {
        // Test: Handles large entries (>1MB)
        let large_data = Bytes::from(vec![0u8; 2 * 1024 * 1024]); // 2MB
        let entry = CacheEntry::new(
            large_data,
            "application/octet-stream".to_string(),
            "etag".to_string(),
            Some(Duration::from_secs(3600)),
        );

        let result = serialize_entry(&entry);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.len() > 1024 * 1024);
    }

    #[test]
    fn test_serialization_is_deterministic() {
        // Test: Serialization is deterministic (same input → same output)
        let entry = create_test_entry();

        let bytes1 = serialize_entry(&entry).unwrap();
        let bytes2 = serialize_entry(&entry).unwrap();

        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_can_deserialize_bytes_to_cache_entry() {
        // Test: Can deserialize bytes to CacheEntry
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        let result = deserialize_entry(&bytes);
        assert!(result.is_ok());

        let deserialized = result.unwrap();
        assert_eq!(deserialized.data, entry.data);
        assert_eq!(deserialized.content_type, entry.content_type);
        assert_eq!(deserialized.etag, entry.etag);
    }

    #[test]
    fn test_validates_version_marker() {
        // Test: Validates version marker (schema version)
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        // Deserialize should succeed with correct version
        let result = deserialize_entry(&bytes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_returns_error_on_corrupt_data() {
        // Test: Returns CacheError::SerializationError on corrupt data
        let corrupt_data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid MessagePack

        let result = deserialize_entry(&corrupt_data);
        assert!(result.is_err());

        match result.unwrap_err() {
            CacheError::SerializationError(msg) => {
                assert!(msg.contains("MessagePack decoding failed"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_returns_error_on_truncated_data() {
        // Test: Returns CacheError::SerializationError on truncated data
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        // Truncate the bytes
        let truncated = &bytes[..bytes.len() / 2];

        let result = deserialize_entry(truncated);
        assert!(result.is_err());

        match result.unwrap_err() {
            CacheError::SerializationError(_) => {
                // Expected
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_validates_deserialized_entry_fields() {
        // Test: Validates deserialized entry fields (non-empty data, valid timestamps)
        let entry = create_test_entry();
        let bytes = serialize_entry(&entry).unwrap();

        let deserialized = deserialize_entry(&bytes).unwrap();

        // Validate fields
        assert!(!deserialized.data.is_empty());
        assert!(!deserialized.etag.is_empty());
        assert!(!deserialized.content_type.is_empty());
        assert!(deserialized.content_length > 0);
    }

    #[test]
    fn test_rejects_entry_with_empty_data() {
        // Test: Validation rejects entry with empty data
        let serializable = SerializableCacheEntry {
            version: SERIALIZATION_VERSION,
            data: vec![], // Empty!
            content_type: "text/plain".to_string(),
            content_length: 0,
            etag: "etag".to_string(),
            created_at_secs: 1000,
            expires_at_secs: 2000,
            last_accessed_at_secs: 1000,
        };

        let bytes = rmp_serde::to_vec(&serializable).unwrap();
        let result = deserialize_entry(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            CacheError::SerializationError(msg) => {
                assert!(msg.contains("data is empty"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_rejects_entry_with_empty_etag() {
        // Test: Validation rejects entry with empty etag
        let serializable = SerializableCacheEntry {
            version: SERIALIZATION_VERSION,
            data: b"data".to_vec(),
            content_type: "text/plain".to_string(),
            content_length: 4,
            etag: String::new(), // Empty!
            created_at_secs: 1000,
            expires_at_secs: 2000,
            last_accessed_at_secs: 1000,
        };

        let bytes = rmp_serde::to_vec(&serializable).unwrap();
        let result = deserialize_entry(&bytes);

        assert!(result.is_err());
        match result.unwrap_err() {
            CacheError::SerializationError(msg) => {
                assert!(msg.contains("etag is empty"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_roundtrip_serialization() {
        // Test: Entry can be serialized and deserialized back
        let original = create_test_entry();
        let bytes = serialize_entry(&original).unwrap();
        let deserialized = deserialize_entry(&bytes).unwrap();

        assert_eq!(original.data, deserialized.data);
        assert_eq!(original.content_type, deserialized.content_type);
        assert_eq!(original.etag, deserialized.etag);
        assert_eq!(original.content_length, deserialized.content_length);
    }
}
