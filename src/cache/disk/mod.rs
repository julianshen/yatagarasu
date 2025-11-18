//! Disk-based cache implementation with hybrid backends
//!
//! This module provides a persistent disk cache with platform-optimized backends:
//! - **io-uring backend** (Linux 5.10+): High-performance using io-uring
//! - **tokio::fs backend** (all platforms): Portable async file I/O
//!
//! The backend is selected at compile time based on the target platform,
//! providing zero runtime overhead.

#[allow(unused_imports)] // Will be used in Phase 28.7+ (DiskCache implementation)
use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
#[allow(unused_imports)]
use async_trait::async_trait;
#[allow(unused_imports)]
use bytes::Bytes;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

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
// Note: UringBackend disabled - tokio-uring 0.5.0 adds statx() but still !Send
// Send trait incompatibility is fundamental to tokio-uring's single-threaded design
//#[cfg(target_os = "linux")]
//mod uring_backend;

// Make tokio_backend available on all platforms
mod tokio_backend;

// Select backend at compile time
// Using tokio_backend on all platforms until architectural decision on !Send traits
#[allow(unused_imports)] // Will be used in Phase 28.9 (Backend Selection)
use tokio_backend as platform_backend;

#[cfg(test)]
mod mock_backend;

#[cfg(test)]
mod tests;
