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

// Make backend trait public for benchmarks
pub mod backend;
mod disk_cache;
mod error;
mod index;
mod types;
mod utils;

// Platform-specific backends
// Using io-uring crate (not tokio-uring) for Linux
// io-uring has Send + Sync types, wrapped with spawn_blocking
#[cfg(target_os = "linux")]
pub mod uring_backend;

// Make tokio_backend always available for testing and benchmarking
// On Linux: both backends compiled, uring_backend used in production
// On other platforms: only tokio_backend available
pub mod tokio_backend;

// Select backend at compile time
// Use uring_backend on Linux (production), but tokio_backend for tests (until Phase 28.11)
#[cfg(all(target_os = "linux", not(test)))]
use uring_backend as platform_backend;

#[cfg(any(not(target_os = "linux"), test))]
use tokio_backend as platform_backend;

#[cfg(test)]
mod mock_backend;

#[cfg(test)]
mod tests;
