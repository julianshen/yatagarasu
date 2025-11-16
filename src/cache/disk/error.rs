//! Error types for disk cache operations

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiskCacheError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage full: cannot store entry")]
    StorageFull,

    #[error("Index corrupted")]
    IndexCorrupted,

    #[error("Backend unavailable")]
    BackendUnavailable,
}

// Conversion to CacheError
impl From<DiskCacheError> for crate::cache::CacheError {
    fn from(err: DiskCacheError) -> Self {
        match err {
            DiskCacheError::Io(e) => crate::cache::CacheError::IoError(e),
            DiskCacheError::Serialization(e) => {
                crate::cache::CacheError::SerializationError(format!("{}", e))
            }
            DiskCacheError::StorageFull => crate::cache::CacheError::StorageFull,
            DiskCacheError::IndexCorrupted => crate::cache::CacheError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, "Index corrupted"),
            ),
            DiskCacheError::BackendUnavailable => crate::cache::CacheError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, "Backend unavailable"),
            ),
        }
    }
}
