//! Backend trait for filesystem operations

use super::error::DiskCacheError;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::Path;

/// Abstraction over filesystem operations to support multiple backends
#[async_trait]
#[allow(dead_code)] // Methods will be used starting Phase 28.7 (LRU Eviction)
pub trait DiskBackend: Send + Sync {
    /// Read entire file contents
    async fn read_file(&self, path: &Path) -> Result<Bytes, DiskCacheError>;

    /// Write file contents atomically (using temp file + rename)
    async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError>;

    /// Delete a file
    async fn delete_file(&self, path: &Path) -> Result<(), DiskCacheError>;

    /// Create directory and all parent directories
    async fn create_dir_all(&self, path: &Path) -> Result<(), DiskCacheError>;

    /// Get file metadata (size, modified time)
    async fn file_size(&self, path: &Path) -> Result<u64, DiskCacheError>;

    /// List all files in a directory
    async fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>, DiskCacheError>;
}
