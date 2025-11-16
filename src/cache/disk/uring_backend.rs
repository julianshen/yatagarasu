//! io-uring based filesystem backend (Linux only, high performance)

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
    async fn read_file(&self, path: &Path) -> Result<Bytes, DiskCacheError> {
        // Use tokio-uring for high-performance I/O on Linux
        let file = tokio_uring::fs::File::open(path).await?;
        let stat = file.statx().await?;
        let size = stat.stx_size as usize;

        let buf = vec![0u8; size];
        let (res, buf) = file.read_at(buf, 0).await;
        res?;

        Ok(Bytes::from(buf))
    }

    async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError> {
        // Create parent directory if needed (using tokio::fs for directory ops)
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to temp file using io-uring
        let temp_path = path.with_extension("tmp");
        let file = tokio_uring::fs::File::create(&temp_path).await?;

        let (res, _) = file.write_at(data.to_vec(), 0).await;
        res?;

        // Atomically rename (using tokio::fs as io-uring doesn't support rename)
        tokio::fs::rename(&temp_path, path).await?;

        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<(), DiskCacheError> {
        // Ignore error if file doesn't exist (idempotent)
        // Use tokio::fs as tokio-uring 0.4 doesn't have remove_file
        let _ = tokio::fs::remove_file(path).await;
        Ok(())
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), DiskCacheError> {
        // Use tokio::fs for directory operations (io-uring focus is on file I/O)
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }

    async fn file_size(&self, path: &Path) -> Result<u64, DiskCacheError> {
        let file = tokio_uring::fs::File::open(path).await?;
        let stat = file.statx().await?;
        Ok(stat.stx_size)
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>, DiskCacheError> {
        // Use tokio::fs for directory listing (io-uring optimization focuses on file I/O)
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry.path());
        }
        Ok(entries)
    }
}
