//! Tokio-based filesystem backend (portable, works on all platforms)

use super::backend::DiskBackend;
use super::error::DiskCacheError;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::Path;

/// Portable filesystem backend using tokio::fs
#[derive(Default)]
pub struct TokioFsBackend;

impl TokioFsBackend {
    pub fn new() -> Self {
        Self
    }
}

/// Factory function for platform_backend module
pub fn create_backend() -> TokioFsBackend {
    TokioFsBackend::new()
}

#[async_trait]
impl DiskBackend for TokioFsBackend {
    async fn read_file(&self, path: &Path) -> Result<Bytes, DiskCacheError> {
        let data = tokio::fs::read(path).await?;
        Ok(Bytes::from(data))
    }

    async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to temp file
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, &data).await?;

        // Atomically rename
        tokio::fs::rename(&temp_path, path).await?;

        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<(), DiskCacheError> {
        // Ignore error if file doesn't exist (idempotent)
        let _ = tokio::fs::remove_file(path).await;
        Ok(())
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), DiskCacheError> {
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }

    async fn file_size(&self, path: &Path) -> Result<u64, DiskCacheError> {
        let metadata = tokio::fs::metadata(path).await?;
        Ok(metadata.len())
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>, DiskCacheError> {
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry.path());
        }
        Ok(entries)
    }
}
