//! Type definitions for disk cache

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata for a cached entry on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub last_accessed_at: u64,
}

impl EntryMetadata {
    pub fn new(file_path: PathBuf, size_bytes: u64, created_at: u64, expires_at: u64) -> Self {
        Self {
            file_path,
            size_bytes,
            created_at,
            expires_at,
            last_accessed_at: created_at,
        }
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at > 0 && now >= self.expires_at
    }
}
