//! Cache error types
//!
//! This module defines error types for cache operations.

/// Cache error types
#[derive(Debug)]
pub enum CacheError {
    /// Cache entry not found
    NotFound,
    /// Cache storage is full
    StorageFull,
    /// I/O error (for disk cache)
    IoError(std::io::Error),
    /// Redis connection failed
    RedisConnectionFailed(String),
    /// Redis operation error
    RedisError(String),
    /// Configuration error
    ConfigurationError(String),
    /// Serialization/deserialization error
    SerializationError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::NotFound => write!(f, "Cache entry not found"),
            CacheError::StorageFull => write!(f, "Cache storage is full"),
            CacheError::IoError(err) => write!(f, "I/O error: {}", err),
            CacheError::RedisConnectionFailed(msg) => write!(f, "Redis connection failed: {}", msg),
            CacheError::RedisError(msg) => write!(f, "Redis error: {}", msg),
            CacheError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            CacheError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CacheError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        CacheError::IoError(err)
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_cache_error() {
        let _err1 = CacheError::NotFound;
        let _err2 = CacheError::StorageFull;
        let _err3 = CacheError::RedisError("test".to_string());
    }

    #[test]
    fn test_cache_error_has_not_found_variant() {
        let err = CacheError::NotFound;
        matches!(err, CacheError::NotFound);
    }

    #[test]
    fn test_cache_error_has_storage_full_variant() {
        let err = CacheError::StorageFull;
        matches!(err, CacheError::StorageFull);
    }

    #[test]
    fn test_cache_error_has_io_error_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CacheError::IoError(io_err);
        matches!(err, CacheError::IoError(_));
    }

    #[test]
    fn test_cache_error_has_redis_error_variant() {
        let err = CacheError::RedisError("connection failed".to_string());
        matches!(err, CacheError::RedisError(_));
    }

    #[test]
    fn test_cache_error_has_serialization_error_variant() {
        let err = CacheError::SerializationError("invalid JSON".to_string());
        matches!(err, CacheError::SerializationError(_));
    }

    #[test]
    fn test_cache_error_implements_error_trait() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<CacheError>();
    }

    #[test]
    fn test_cache_error_implements_display_trait() {
        let err = CacheError::NotFound;
        let display_str = format!("{}", err);
        assert!(display_str.contains("not found"));

        let err = CacheError::StorageFull;
        let display_str = format!("{}", err);
        assert!(display_str.contains("full"));
    }

    #[test]
    fn test_cache_error_converts_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let cache_err: CacheError = io_err.into();
        matches!(cache_err, CacheError::IoError(_));
    }

    #[test]
    fn test_cache_error_converts_from_serde_error() {
        let json_str = "{invalid json}";
        let serde_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let cache_err: CacheError = serde_err.into();
        matches!(cache_err, CacheError::SerializationError(_));
    }
}
