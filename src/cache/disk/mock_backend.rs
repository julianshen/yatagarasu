//! Mock disk backend for testing (in-memory HashMap storage)

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use super::backend::DiskBackend;
use super::error::DiskCacheError;

/// Mock backend that stores files in memory for testing
#[derive(Clone)]
pub struct MockDiskBackend {
    files: Arc<RwLock<HashMap<PathBuf, Bytes>>>,
    directories: Arc<RwLock<Vec<PathBuf>>>,
    /// Simulate errors if true
    simulate_storage_full: Arc<RwLock<bool>>,
    simulate_permission_denied: Arc<RwLock<bool>>,
}

impl MockDiskBackend {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(Vec::new())),
            simulate_storage_full: Arc::new(RwLock::new(false)),
            simulate_permission_denied: Arc::new(RwLock::new(false)),
        }
    }

    /// Enable storage full simulation for testing
    pub fn set_storage_full(&self, enabled: bool) {
        *self.simulate_storage_full.write() = enabled;
    }

    /// Enable permission denied simulation for testing
    pub fn set_permission_denied(&self, enabled: bool) {
        *self.simulate_permission_denied.write() = enabled;
    }

    /// Get number of stored files
    pub fn file_count(&self) -> usize {
        self.files.read().len()
    }

    /// Clear all stored files and directories
    pub fn clear(&self) {
        self.files.write().clear();
        self.directories.write().clear();
    }
}

#[async_trait]
impl DiskBackend for MockDiskBackend {
    async fn read_file(&self, path: &Path) -> Result<Bytes, DiskCacheError> {
        if *self.simulate_permission_denied.read() {
            return Err(DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated permission denied"
            )));
        }

        self.files
            .read()
            .get(path)
            .cloned()
            .ok_or_else(|| {
                DiskCacheError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "File not found"
                ))
            })
    }

    async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError> {
        if *self.simulate_storage_full.read() {
            return Err(DiskCacheError::StorageFull);
        }

        if *self.simulate_permission_denied.read() {
            return Err(DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated permission denied"
            )));
        }

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent).await?;
        }

        self.files.write().insert(path.to_path_buf(), data);
        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<(), DiskCacheError> {
        if *self.simulate_permission_denied.read() {
            return Err(DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated permission denied"
            )));
        }

        self.files.write().remove(path);
        Ok(())
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), DiskCacheError> {
        if *self.simulate_permission_denied.read() {
            return Err(DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated permission denied"
            )));
        }

        let mut dirs = self.directories.write();
        if !dirs.contains(&path.to_path_buf()) {
            dirs.push(path.to_path_buf());
        }
        Ok(())
    }

    async fn file_size(&self, path: &Path) -> Result<u64, DiskCacheError> {
        if *self.simulate_permission_denied.read() {
            return Err(DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated permission denied"
            )));
        }

        self.files
            .read()
            .get(path)
            .map(|data| data.len() as u64)
            .ok_or_else(|| {
                DiskCacheError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "File not found"
                ))
            })
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, DiskCacheError> {
        if *self.simulate_permission_denied.read() {
            return Err(DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Simulated permission denied"
            )));
        }

        let files = self.files.read();
        let entries: Vec<PathBuf> = files
            .keys()
            .filter(|p| p.parent() == Some(path))
            .cloned()
            .collect();

        Ok(entries)
    }
}
