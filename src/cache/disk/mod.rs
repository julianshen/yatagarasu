//! Disk-based cache implementation with hybrid backends
//!
//! This module provides a persistent disk cache with platform-optimized backends:
//! - **io-uring backend** (Linux 5.10+): High-performance using io-uring
//! - **tokio::fs backend** (all platforms): Portable async file I/O
//!
//! The backend is selected at compile time based on the target platform,
//! providing zero runtime overhead.

use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use bytes::Bytes;

// Re-export main types
pub use self::disk_cache::DiskCache;
pub use self::error::DiskCacheError;

mod backend;
mod disk_cache;
mod error;
mod index;
mod types;
mod utils;

// Platform-specific backends
#[cfg(target_os = "linux")]
mod uring_backend;

// Make tokio_backend available for non-Linux or for tests
#[cfg(any(not(target_os = "linux"), test))]
mod tokio_backend;

// Select backend at compile time
#[cfg(target_os = "linux")]
use uring_backend as platform_backend;

#[cfg(not(target_os = "linux"))]
use tokio_backend as platform_backend;

#[cfg(test)]
mod mock_backend;

#[cfg(test)]
mod tests;
