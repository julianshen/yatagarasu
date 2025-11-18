//! io-uring based filesystem backend (Linux only, high performance)
//!
//! Uses low-level io-uring crate wrapped with tokio::task::spawn_blocking
//! for Send-compatible async API.
//!
//! Unlike tokio-uring (which has !Send futures due to Rc<T>), the io-uring
//! crate provides Send + Sync types that work with #[async_trait].

#[cfg(target_os = "linux")]
use super::backend::DiskBackend;
#[cfg(target_os = "linux")]
use super::error::DiskCacheError;
#[cfg(target_os = "linux")]
use async_trait::async_trait;
#[cfg(target_os = "linux")]
use bytes::Bytes;
#[cfg(target_os = "linux")]
use std::path::Path;

/// io-uring backend for Linux
///
/// Wraps low-level io-uring operations in spawn_blocking tasks to provide
/// Send futures compatible with async_trait.
#[cfg(target_os = "linux")]
pub struct UringBackend;

#[cfg(target_os = "linux")]
impl UringBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl DiskBackend for UringBackend {
    async fn read_file(&self, _path: &Path) -> Result<Bytes, DiskCacheError> {
        // TODO: Implement with io-uring
        todo!("implement read_file with io-uring")
    }

    async fn write_file_atomic(&self, _path: &Path, _data: Bytes) -> Result<(), DiskCacheError> {
        // TODO: Implement with io-uring
        todo!("implement write_file_atomic with io-uring")
    }

    async fn delete_file(&self, _path: &Path) -> Result<(), DiskCacheError> {
        // TODO: Implement with tokio::fs (simpler for delete)
        todo!("implement delete_file")
    }

    async fn create_dir_all(&self, _path: &Path) -> Result<(), DiskCacheError> {
        // Use tokio::fs for directory operations (io-uring optimizes file I/O)
        tokio::fs::create_dir_all(_path).await?;
        Ok(())
    }

    async fn file_size(&self, _path: &Path) -> Result<u64, DiskCacheError> {
        // Use tokio::fs for metadata queries
        let metadata = tokio::fs::metadata(_path).await?;
        Ok(metadata.len())
    }

    async fn read_dir(&self, _path: &Path) -> Result<Vec<std::path::PathBuf>, DiskCacheError> {
        // Use tokio::fs for directory listing (io-uring optimizes file I/O)
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(_path).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry.path());
        }
        Ok(entries)
    }
}
