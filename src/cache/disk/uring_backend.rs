//! io-uring based filesystem backend (Linux only, high performance)

#[cfg(target_os = "linux")]
use async_trait::async_trait;
#[cfg(target_os = "linux")]
use bytes::Bytes;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use super::backend::DiskBackend;
#[cfg(target_os = "linux")]
use super::error::DiskCacheError;

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
        // TODO: Implement with tokio-uring
        todo!("io-uring backend not yet implemented")
    }

    async fn write_file_atomic(&self, _path: &Path, _data: Bytes) -> Result<(), DiskCacheError> {
        // TODO: Implement with tokio-uring
        todo!("io-uring backend not yet implemented")
    }

    async fn delete_file(&self, _path: &Path) -> Result<(), DiskCacheError> {
        // TODO: Implement with tokio-uring
        todo!("io-uring backend not yet implemented")
    }

    async fn create_dir_all(&self, _path: &Path) -> Result<(), DiskCacheError> {
        // TODO: Implement with tokio-uring
        todo!("io-uring backend not yet implemented")
    }

    async fn file_size(&self, _path: &Path) -> Result<u64, DiskCacheError> {
        // TODO: Implement with tokio-uring
        todo!("io-uring backend not yet implemented")
    }

    async fn read_dir(&self, _path: &Path) -> Result<Vec<std::path::PathBuf>, DiskCacheError> {
        // TODO: Implement with tokio-uring
        todo!("io-uring backend not yet implemented")
    }
}
