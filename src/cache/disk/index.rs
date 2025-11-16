//! Cache index management

use crate::cache::CacheKey;
use super::types::EntryMetadata;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Thread-safe in-memory index of cached entries
pub struct CacheIndex {
    entries: Arc<RwLock<HashMap<CacheKey, EntryMetadata>>>,
    total_size: Arc<AtomicU64>,
}

impl CacheIndex {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            total_size: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<EntryMetadata> {
        self.entries.read().get(key).cloned()
    }

    pub fn insert(&self, key: CacheKey, metadata: EntryMetadata) {
        let size = metadata.size_bytes;
        self.entries.write().insert(key, metadata);
        self.total_size.fetch_add(size, Ordering::SeqCst);
    }

    pub fn remove(&self, key: &CacheKey) -> Option<EntryMetadata> {
        let removed = self.entries.write().remove(key);
        if let Some(ref metadata) = removed {
            self.total_size.fetch_sub(metadata.size_bytes, Ordering::SeqCst);
        }
        removed
    }

    pub fn total_size(&self) -> u64 {
        self.total_size.load(Ordering::SeqCst)
    }

    pub fn entry_count(&self) -> usize {
        self.entries.read().len()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
        self.total_size.store(0, Ordering::SeqCst);
    }
}
