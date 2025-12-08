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
impl Default for UringBackend {
    fn default() -> Self {
        Self
    }
}

#[cfg(target_os = "linux")]
impl UringBackend {
    pub fn new() -> Self {
        Self
    }
}

/// Factory function for platform_backend module
#[cfg(target_os = "linux")]
pub fn create_backend() -> UringBackend {
    UringBackend::new()
}

/// Blocking helper to write file using io-uring
#[cfg(target_os = "linux")]
fn write_file_blocking(path: &Path, data: &[u8]) -> Result<(), DiskCacheError> {
    use io_uring::{opcode, types, IoUring};
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    // Create/truncate file for writing
    let file = File::create(path)?;
    let fd = file.as_raw_fd();

    // Create io_uring instance with queue depth of 8
    let mut ring = IoUring::new(8)?;

    // Submit write operation
    let write_op = opcode::Write::new(types::Fd(fd), data.as_ptr(), data.len() as u32)
        .offset(0)
        .build()
        .user_data(1);

    unsafe {
        ring.submission().push(&write_op).map_err(|_| {
            DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to push write operation",
            ))
        })?;
    }

    // Submit and wait for completion
    ring.submit_and_wait(1)?;

    // Retrieve completion event
    let cqe = ring.completion().next().ok_or_else(|| {
        DiskCacheError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No completion event",
        ))
    })?;

    // Check result
    let result = cqe.result();
    if result < 0 {
        return Err(DiskCacheError::Io(std::io::Error::from_raw_os_error(
            -result,
        )));
    }

    // Verify all bytes were written
    if result as usize != data.len() {
        return Err(DiskCacheError::Io(std::io::Error::new(
            std::io::ErrorKind::WriteZero,
            format!("Partial write: {} of {} bytes", result, data.len()),
        )));
    }

    Ok(())
}

/// Blocking helper to read file using io-uring
#[cfg(target_os = "linux")]
fn read_file_blocking(path: &Path) -> Result<Bytes, DiskCacheError> {
    use io_uring::{opcode, types, IoUring};
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    // Open file for reading
    let file = File::open(path)?;
    let fd = file.as_raw_fd();

    // Get file size
    let metadata = file.metadata()?;
    let file_size = metadata.len() as usize;

    // Allocate buffer for file contents
    let mut buffer = vec![0u8; file_size];

    // Create io_uring instance with queue depth of 8
    let mut ring = IoUring::new(8)?;

    // Submit read operation
    let read_op = opcode::Read::new(types::Fd(fd), buffer.as_mut_ptr(), file_size as u32)
        .offset(0)
        .build()
        .user_data(1);

    unsafe {
        ring.submission().push(&read_op).map_err(|_| {
            DiskCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to push read operation",
            ))
        })?;
    }

    // Submit and wait for completion
    ring.submit_and_wait(1)?;

    // Retrieve completion event
    let cqe = ring.completion().next().ok_or_else(|| {
        DiskCacheError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No completion event",
        ))
    })?;

    // Check result
    let result = cqe.result();
    if result < 0 {
        return Err(DiskCacheError::Io(std::io::Error::from_raw_os_error(
            -result,
        )));
    }

    // Return bytes
    Ok(Bytes::from(buffer))
}

#[cfg(target_os = "linux")]
#[async_trait]
impl DiskBackend for UringBackend {
    async fn read_file(&self, path: &Path) -> Result<Bytes, DiskCacheError> {
        // Wrap io-uring operations in spawn_blocking for Send-compatible async
        let path_buf = path.to_path_buf();

        tokio::task::spawn_blocking(move || read_file_blocking(&path_buf))
            .await
            .map_err(|e| DiskCacheError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
    }

    async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError> {
        // Create parent directory if needed (use tokio::fs for directory operations)
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to temp file using io-uring
        let temp_path = path.with_extension("tmp");
        let temp_path_clone = temp_path.clone();
        let data_vec = data.to_vec(); // Convert Bytes to Vec for moving into closure

        tokio::task::spawn_blocking(move || write_file_blocking(&temp_path_clone, &data_vec))
            .await
            .map_err(|e| DiskCacheError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

        // Atomically rename temp to final (use tokio::fs for rename)
        tokio::fs::rename(&temp_path, path).await?;

        Ok(())
    }

    async fn delete_file(&self, path: &Path) -> Result<(), DiskCacheError> {
        // Use tokio::fs for delete (metadata operation, doesn't benefit from io-uring)
        // Ignore error if file doesn't exist (idempotent)
        let _ = tokio::fs::remove_file(path).await;
        Ok(())
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
