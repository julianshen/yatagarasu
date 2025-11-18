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

/// Factory function for platform_backend module
#[cfg(target_os = "linux")]
pub fn create_backend() -> UringBackend {
    UringBackend::new()
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
