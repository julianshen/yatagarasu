//! Type definitions for disk cache

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::cache::CacheKey;

/// Metadata for a cached entry on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub cache_key: CacheKey,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub last_accessed_at: u64,
    /// HTTP Content-Type header value
    #[serde(default = "default_content_type")]
    pub content_type: String,
    /// HTTP ETag header value
    #[serde(default)]
    pub etag: String,
    /// HTTP Last-Modified header value (RFC 2822 format)
    #[serde(default)]
    pub last_modified: Option<String>,
}

fn default_content_type() -> String {
    "application/octet-stream".to_string()
}

impl EntryMetadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cache_key: CacheKey,
        file_path: PathBuf,
        size_bytes: u64,
        created_at: u64,
        expires_at: u64,
        content_type: String,
        etag: String,
        last_modified: Option<String>,
    ) -> Self {
        Self {
            cache_key,
            file_path,
            size_bytes,
            created_at,
            expires_at,
            last_accessed_at: created_at,
            content_type,
            etag,
            last_modified,
        }
    }

    #[allow(dead_code)] // Will be used in Phase 28.9 (Cache Trait Implementation)
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at > 0 && now >= self.expires_at
    }
}
