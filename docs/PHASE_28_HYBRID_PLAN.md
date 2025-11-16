# PHASE 28: Hybrid Disk Cache Implementation (REPLANNED)

**Last Updated**: 2025-11-16
**Status**: Ready for Implementation
**Strategy**: Hybrid approach - io-uring on Linux, tokio::fs elsewhere

---

## Overview

**Goal**: Implement persistent disk-based cache with platform-optimized backends
**Deliverable**:
- High-performance io-uring backend on Linux 5.10+
- Portable tokio::fs backend on macOS/Windows/older Linux
- Single unified API via trait abstraction

**Verification**:
- All tests pass on all platforms
- io-uring shows 2-3x improvement on Linux
- Cache survives process restart
- No platform-specific bugs

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│          Cache Trait (Public API)               │
└─────────────────┬───────────────────────────────┘
                  │
        ┌─────────┴──────────┐
        │                    │
        ▼                    ▼
┌──────────────────┐  ┌──────────────────┐
│ io-uring Backend │  │ tokio::fs Backend│
│   (Linux 5.10+)  │  │   (All platforms)│
│   - Fast path    │  │   - Fallback     │
│   - Zero-copy    │  │   - Portable     │
└──────────────────┘  └──────────────────┘
```

**Compile-Time Selection**:
```rust
#[cfg(all(target_os = "linux", tokio_uring_available))]
use uring_backend::DiskCache;

#[cfg(not(all(target_os = "linux", tokio_uring_available)))]
use tokio_backend::DiskCache;
```

---

## Phase 28 Structure (Revised)

### Week 1: Abstraction & Core Logic (Days 1-3)
- 28.1: Shared abstractions and types
- 28.2: Backend trait definition
- 28.3: Cache key mapping and file structure
- 28.4: Index management (shared between backends)

### Week 2: Dual Backend Implementation (Days 4-7)
- 28.5: tokio::fs backend implementation
- 28.6: tokio-uring backend implementation
- 28.7: LRU eviction (shared logic)
- 28.8: Recovery & startup (both backends)

### Week 3: Integration & Testing (Days 8-10)
- 28.9: Cache trait implementation
- 28.10: Cross-platform testing
- 28.11: Performance validation

**Total Time**: 7-10 days (more thorough than original plan)

---

# PHASE 28.1: Shared Abstractions & Dependencies

**Goal**: Define common types and abstractions used by both backends
**Deliverable**: Core types, errors, and dependencies configured
**Verification**: Code compiles on all platforms

## 28.1.1: Dependencies Setup

### Core Dependencies (All Platforms)
```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
parking_lot = "0.12"
```

- [ ] Test: Add tokio for async runtime
- [ ] Test: Add sha2 for cache key hashing
- [ ] Test: Add serde/serde_json for metadata
- [ ] Test: Add parking_lot for thread-safe index

### Platform-Specific Dependencies
```toml
[target.'cfg(target_os = "linux")'.dependencies]
tokio-uring = "0.4"

[dev-dependencies]
tempfile = "3.8"
```

- [ ] Test: Add tokio-uring on Linux only
- [ ] Test: Add tempfile for test isolation
- [ ] Test: Dependencies compile on all platforms
- [ ] Test: Can import tokio_uring on Linux
- [ ] Test: Build works without tokio-uring on macOS

### Feature Detection
- [ ] Test: Detect Linux at compile time
- [ ] Test: Detect kernel version at runtime (Linux only)
- [ ] Test: Log which backend is selected
- [ ] Test: Graceful fallback if io-uring unavailable

---

## 28.1.2: Common Types & Structures

### EntryMetadata Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub last_accessed_at: u64,
}
```

- [ ] Test: Can create EntryMetadata
- [ ] Test: Can serialize to JSON
- [ ] Test: Can deserialize from JSON
- [ ] Test: Implements Clone, Debug
- [ ] Test: Size calculation is accurate

### CacheIndex Structure
```rust
pub struct CacheIndex {
    entries: HashMap<CacheKey, EntryMetadata>,
    total_size: AtomicU64,
}
```

- [ ] Test: Can create CacheIndex
- [ ] Test: Can add entry to index
- [ ] Test: Can remove entry from index
- [ ] Test: Can query entry existence
- [ ] Test: Thread-safe (Send + Sync)
- [ ] Test: Total size tracking accurate

### CacheStats Structure (Already defined in Phase 26)
- [ ] Test: Extend with backend_type field
- [ ] Test: Can track per-backend stats
- [ ] Test: Atomic counters for thread safety

---

## 28.1.3: File Path Utilities (Shared)

### Path Generation
- [ ] Test: Can convert CacheKey to SHA256 hash
- [ ] Test: Can generate file path from hash
- [ ] Test: Path uses entries/ subdirectory
- [ ] Test: Generates .data and .meta file paths
- [ ] Test: Prevents path traversal attacks

### Example Paths
```
/var/cache/yatagarasu/
├── index.json
└── entries/
    ├── a3f2e8d1.../.data
    └── a3f2e8d1.../.meta
```

- [ ] Test: Directory structure created correctly
- [ ] Test: File paths are consistent across calls
- [ ] Test: Handles Unicode in cache keys

---

## 28.1.4: Error Types

### DiskCacheError Enum
```rust
pub enum DiskCacheError {
    Io(io::Error),
    Serialization(serde_json::Error),
    StorageFull,
    IndexCorrupted,
    BackendUnavailable,
}
```

- [ ] Test: Can create all error variants
- [ ] Test: Implements Error trait
- [ ] Test: Implements Display trait
- [ ] Test: Can convert from io::Error
- [ ] Test: Can convert from serde_json::Error
- [ ] Test: Maps to CacheError correctly

---

# PHASE 28.2: Backend Trait Definition

**Goal**: Define abstraction layer for filesystem operations
**Deliverable**: Trait that both backends implement
**Verification**: Trait compiles, mock implementation works

## 28.2.1: DiskBackend Trait

### Trait Definition
```rust
#[async_trait]
pub trait DiskBackend: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Bytes, DiskCacheError>;
    async fn write_file(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError>;
    async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<(), DiskCacheError>;
    async fn delete_file(&self, path: &Path) -> Result<(), DiskCacheError>;
    async fn create_dir_all(&self, path: &Path) -> Result<(), DiskCacheError>;
    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, DiskCacheError>;
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, DiskCacheError>;
}
```

- [ ] Test: Trait compiles with correct bounds
- [ ] Test: All methods are async
- [ ] Test: Can create trait object Arc<dyn DiskBackend>
- [ ] Test: Send + Sync bounds enforced

### FileMetadata Structure
```rust
pub struct FileMetadata {
    pub size: u64,
    pub modified: SystemTime,
}
```

- [ ] Test: Can extract size from metadata
- [ ] Test: Can extract modified time
- [ ] Test: Lightweight structure

---

## 28.2.2: Mock Backend (for testing)

### MockDiskBackend Implementation
- [ ] Test: Can create MockDiskBackend
- [ ] Test: Implements DiskBackend trait
- [ ] Test: Stores files in HashMap (in-memory)
- [ ] Test: Can read what was written
- [ ] Test: Simulates errors (disk full, permission denied)
- [ ] Test: Used in unit tests for higher layers

---

# PHASE 28.3: Cache Key Mapping & File Structure

**Goal**: Implement file path generation and structure
**Deliverable**: Deterministic file path generation
**Verification**: Same key always maps to same path

## 28.3.1: Hash-Based File Naming

### SHA256 Hashing
- [ ] Test: Can hash CacheKey to SHA256
- [ ] Test: Hash is deterministic (same key = same hash)
- [ ] Test: Hash is hex-encoded string
- [ ] Test: Hash length is 64 characters

### Path Construction
- [ ] Test: Path format: {cache_dir}/entries/{hash}.data
- [ ] Test: Metadata path: {cache_dir}/entries/{hash}.meta
- [ ] Test: Handles bucket name safely (no path traversal)
- [ ] Test: Creates entries subdirectory if needed

---

## 28.3.2: File Format

### Data File (.data)
- [ ] Test: Stores raw cache entry data (binary)
- [ ] Test: Written atomically (temp + rename)
- [ ] Test: No compression (for now)

### Metadata File (.meta)
```json
{
  "bucket": "products",
  "object_key": "images/product-123.jpg",
  "size_bytes": 45678,
  "content_type": "image/jpeg",
  "etag": "abc123",
  "created_at": 1699999999,
  "expires_at": 1700003599,
  "last_accessed_at": 1699999999
}
```

- [ ] Test: Metadata is JSON format
- [ ] Test: Contains all required fields
- [ ] Test: Deserializes correctly
- [ ] Test: Human-readable for debugging

---

# PHASE 28.4: Index Management (Shared)

**Goal**: Implement in-memory index for fast lookups
**Deliverable**: Thread-safe index with persistence
**Verification**: Index survives restart, stays consistent

## 28.4.1: In-Memory Index

### CacheIndex Implementation
- [ ] Test: Maps CacheKey → EntryMetadata
- [ ] Test: Thread-safe (uses RwLock or DashMap)
- [ ] Test: Can add entry
- [ ] Test: Can remove entry
- [ ] Test: Can update last_accessed_at
- [ ] Test: Can query by key

### Size Tracking
- [ ] Test: Tracks total cache size (atomic)
- [ ] Test: Size updated on insert
- [ ] Test: Size updated on delete
- [ ] Test: Size calculation is accurate

---

## 28.4.2: Index Persistence

### Save to Disk
- [ ] Test: Index saved to index.json
- [ ] Test: Serializes entire index to JSON
- [ ] Test: Written atomically (temp + rename)
- [ ] Test: Triggered periodically (e.g., every 30s)
- [ ] Test: Triggered on graceful shutdown

### Load from Disk
- [ ] Test: Index loaded on startup
- [ ] Test: Deserializes from index.json
- [ ] Test: Handles missing file (starts empty)
- [ ] Test: Handles corrupted JSON (logs error, starts empty)
- [ ] Test: Validates entries against actual files

---

## 28.4.3: Index Validation & Repair

### Startup Validation
- [ ] Test: Scans entries/ directory
- [ ] Test: Removes orphaned files (no index entry)
- [ ] Test: Removes index entries without files
- [ ] Test: Recalculates total size from files
- [ ] Test: Logs repair actions

### Consistency Checks
- [ ] Test: Validates .data and .meta exist together
- [ ] Test: Validates metadata size matches actual file
- [ ] Test: Removes expired entries on startup
- [ ] Test: Rebuilds index if validation fails

---

# PHASE 28.5: tokio::fs Backend Implementation

**Goal**: Implement portable backend using tokio::fs
**Deliverable**: DiskBackend implementation for all platforms
**Verification**: All trait methods work correctly

## 28.5.1: TokioFsBackend Structure

### Backend Setup
```rust
pub struct TokioFsBackend {
    // Minimal state, maybe just for metrics
}
```

- [ ] Test: Can create TokioFsBackend
- [ ] Test: Implements DiskBackend trait
- [ ] Test: Implements Send + Sync
- [ ] Test: Stateless (all operations use provided paths)

---

## 28.5.2: Read Operations

### read_file Implementation
- [ ] Test: Reads file using tokio::fs::read
- [ ] Test: Returns Bytes
- [ ] Test: Returns error if file doesn't exist
- [ ] Test: Returns error if permission denied
- [ ] Test: Works with various file sizes (0B to 100MB)

### metadata Implementation
- [ ] Test: Gets file metadata using tokio::fs::metadata
- [ ] Test: Returns size correctly
- [ ] Test: Returns modified time
- [ ] Test: Returns error if file doesn't exist

### read_dir Implementation
- [ ] Test: Lists directory contents
- [ ] Test: Returns Vec<PathBuf>
- [ ] Test: Handles empty directory
- [ ] Test: Handles missing directory (error)

---

## 28.5.3: Write Operations

### write_file Implementation
- [ ] Test: Writes data to file
- [ ] Test: Creates parent directories if needed
- [ ] Test: Overwrites existing file
- [ ] Test: Returns error on disk full
- [ ] Test: Returns error on permission denied

### write_file_atomic Implementation
```rust
async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<()> {
    let temp_path = path.with_extension("tmp");
    tokio::fs::write(&temp_path, &data).await?;
    tokio::fs::rename(&temp_path, path).await?;
    Ok(())
}
```

- [ ] Test: Writes to temp file first
- [ ] Test: Atomically renames to final path
- [ ] Test: Cleans up temp file on error
- [ ] Test: Prevents partial writes being visible

---

## 28.5.4: Delete Operations

### delete_file Implementation
- [ ] Test: Deletes file using tokio::fs::remove_file
- [ ] Test: Returns Ok if file exists
- [ ] Test: Returns Ok if file doesn't exist (idempotent)
- [ ] Test: Returns error if directory (not a file)

### create_dir_all Implementation
- [ ] Test: Creates directory recursively
- [ ] Test: No-op if directory exists
- [ ] Test: Returns error on permission denied

---

# PHASE 28.6: tokio-uring Backend Implementation

**Goal**: Implement high-performance backend using io-uring on Linux
**Deliverable**: DiskBackend implementation with io-uring
**Verification**: All trait methods work, 2-3x faster than tokio::fs

## 28.6.1: UringBackend Structure

### Backend Setup
```rust
#[cfg(target_os = "linux")]
pub struct UringBackend {
    buffer_pool: Arc<BufferPool>,
}
```

- [ ] Test: Can create UringBackend
- [ ] Test: Implements DiskBackend trait
- [ ] Test: Implements Send + Sync
- [ ] Test: Initializes buffer pool

---

## 28.6.2: Buffer Pool Management

### BufferPool Design
```rust
struct BufferPool {
    small_buffers: Mutex<Vec<Vec<u8>>>,  // 4KB
    large_buffers: Mutex<Vec<Vec<u8>>>,  // 64KB
}
```

- [ ] Test: Can create buffer pool
- [ ] Test: Can acquire 4KB buffer
- [ ] Test: Can acquire 64KB buffer
- [ ] Test: Can return buffer to pool
- [ ] Test: Pool has max capacity (e.g., 1000 buffers)
- [ ] Test: Buffers zeroed on return

### Buffer Lifecycle
- [ ] Test: acquire_buffer() gets from pool or allocates
- [ ] Test: return_buffer() returns to pool if space available
- [ ] Test: Buffers dropped if pool full
- [ ] Test: Thread-safe operations

---

## 28.6.3: Read Operations (io-uring)

### read_file Implementation
```rust
async fn read_file(&self, path: &Path) -> Result<Bytes> {
    let file = tokio_uring::fs::File::open(path).await?;
    let metadata = file.statx().await?;
    let size = metadata.stx_size as usize;

    let mut buf = self.buffer_pool.acquire(size);
    let (res, buf) = file.read_at(buf, 0).await;
    res?;

    file.close().await?;
    Ok(Bytes::from(buf))
}
```

- [ ] Test: Opens file with tokio_uring
- [ ] Test: Gets file size via statx
- [ ] Test: Reads entire file into buffer
- [ ] Test: Explicitly closes file
- [ ] Test: Returns Bytes
- [ ] Test: Returns buffer to pool on error

### Ownership-Based API Handling
- [ ] Test: Acquires buffer before read
- [ ] Test: Passes buffer ownership to kernel
- [ ] Test: Gets buffer back after read
- [ ] Test: No buffer aliasing or use-after-free

---

## 28.6.4: Write Operations (io-uring)

### write_file_atomic Implementation
```rust
async fn write_file_atomic(&self, path: &Path, data: Bytes) -> Result<()> {
    let temp_path = path.with_extension("tmp");

    let file = tokio_uring::fs::File::create(&temp_path).await?;
    let (res, _) = file.write_at(data.to_vec(), 0).await;
    res?;

    file.sync_all().await?;
    file.close().await?;

    tokio_uring::fs::rename(&temp_path, path).await?;
    Ok(())
}
```

- [ ] Test: Creates temp file
- [ ] Test: Writes data to temp file
- [ ] Test: Syncs to disk (fsync)
- [ ] Test: Explicitly closes file
- [ ] Test: Atomically renames
- [ ] Test: Cleans up temp file on error

### Buffer Handling for Writes
- [ ] Test: Converts Bytes to Vec<u8> for ownership
- [ ] Test: Buffer consumed by write operation
- [ ] Test: No buffer reuse for writes (data consumed)

---

## 28.6.5: Delete & Directory Operations (io-uring)

### delete_file Implementation
- [ ] Test: Uses tokio_uring::fs::remove_file
- [ ] Test: Handles non-existent files gracefully
- [ ] Test: Returns appropriate errors

### create_dir_all Implementation
- [ ] Test: Creates directories using tokio_uring::fs
- [ ] Test: Recursive creation works
- [ ] Test: Idempotent (no error if exists)

### read_dir Implementation
- [ ] Test: Scans directory with tokio_uring::fs
- [ ] Test: Returns list of paths
- [ ] Test: Handles empty directories

---

## 28.6.6: Runtime Integration

### Spawning io-uring Tasks
```rust
pub async fn run_uring_task<F, R>(f: F) -> Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            f()
        })
    }).await?
}
```

- [ ] Test: Can spawn io-uring tasks from Tokio
- [ ] Test: Results propagated correctly
- [ ] Test: Errors propagated correctly
- [ ] Test: No deadlocks

### Context Switching
- [ ] Test: Tokio runtime not blocked by io-uring
- [ ] Test: io-uring operations complete successfully
- [ ] Test: Concurrency works correctly

---

# PHASE 28.7: LRU Eviction (Shared Logic)

**Goal**: Implement LRU eviction policy for both backends
**Deliverable**: Eviction logic that works with any backend
**Verification**: Evicts least recently used entries correctly

## 28.7.1: Size Tracking

### Current Size Monitoring
- [ ] Test: Tracks total disk cache size (atomic)
- [ ] Test: Size updated on insert
- [ ] Test: Size updated on delete
- [ ] Test: Size includes both .data and .meta files

### Threshold Detection
- [ ] Test: Detects when size exceeds max
- [ ] Test: Triggers eviction when threshold exceeded
- [ ] Test: Allows configurable headroom (e.g., evict to 90% of max)

---

## 28.7.2: LRU Sorting

### Access Time Tracking
- [ ] Test: Updates last_accessed_at on cache hit
- [ ] Test: Index stores last_accessed_at
- [ ] Test: Can sort entries by access time

### Eviction Candidate Selection
- [ ] Test: Identifies least recently accessed entry
- [ ] Test: Can select N oldest entries
- [ ] Test: Excludes recently created entries (grace period)
- [ ] Test: Handles tied access times (use created_at)

---

## 28.7.3: Eviction Execution

### Single Entry Eviction
- [ ] Test: Deletes .data file
- [ ] Test: Deletes .meta file
- [ ] Test: Removes from index
- [ ] Test: Updates total size
- [ ] Test: Increments eviction counter

### Batch Eviction
- [ ] Test: Can evict multiple entries in one pass
- [ ] Test: Evicts in LRU order
- [ ] Test: Stops when enough space freed
- [ ] Test: Atomic index updates (all or nothing)

### Error Handling
- [ ] Test: Continues eviction if one file fails
- [ ] Test: Logs eviction errors
- [ ] Test: Ensures cache doesn't grow unbounded even with errors

---

# PHASE 28.8: Recovery & Startup (Both Backends)

**Goal**: Implement crash recovery and validation
**Deliverable**: Cache recovers correctly after restart or crash
**Verification**: No data corruption, orphaned files cleaned up

## 28.8.1: Startup Sequence

### Initialization Steps
1. Load index from index.json
2. Validate index against filesystem
3. Remove orphaned files
4. Remove invalid index entries
5. Recalculate total size
6. Trigger eviction if oversized

- [ ] Test: Startup sequence completes successfully
- [ ] Test: Each step is independent and testable
- [ ] Test: Failures are logged but don't crash

---

## 28.8.2: Index Loading

### Load from index.json
- [ ] Test: Deserializes index from file
- [ ] Test: Handles missing index.json (starts empty)
- [ ] Test: Handles corrupted JSON (logs error, starts empty)
- [ ] Test: Handles empty index.json (empty index)

### Index Validation
- [ ] Test: Validates all required fields present
- [ ] Test: Validates timestamps are reasonable
- [ ] Test: Validates file paths are within cache_dir
- [ ] Test: Rejects invalid entries

---

## 28.8.3: Filesystem Validation

### File Existence Check
- [ ] Test: Verifies .data file exists for each index entry
- [ ] Test: Verifies .meta file exists for each index entry
- [ ] Test: Removes index entry if files missing
- [ ] Test: Logs missing files

### Size Validation
- [ ] Test: Compares actual file size to metadata
- [ ] Test: Updates metadata if sizes differ
- [ ] Test: Removes entry if size mismatch too large (corruption)

---

## 28.8.4: Orphan Cleanup

### Orphaned File Detection
- [ ] Test: Scans entries/ directory
- [ ] Test: Identifies files not in index
- [ ] Test: Deletes orphaned .data files
- [ ] Test: Deletes orphaned .meta files
- [ ] Test: Logs cleanup actions

### Temporary File Cleanup
- [ ] Test: Deletes .tmp files from failed writes
- [ ] Test: Doesn't delete legitimate files
- [ ] Test: Handles permission errors gracefully

---

## 28.8.5: Size Recalculation

### Total Size Audit
- [ ] Test: Sums size of all .data files
- [ ] Test: Updates total_size in index
- [ ] Test: Matches index total_size to filesystem
- [ ] Test: Triggers eviction if over limit

---

## 28.8.6: Corrupted Entry Handling

### Detection
- [ ] Test: Detects corrupted .data file (read error)
- [ ] Test: Detects corrupted .meta file (JSON parse error)
- [ ] Test: Detects mismatched .data and .meta

### Recovery
- [ ] Test: Removes corrupted entry from index
- [ ] Test: Deletes both .data and .meta files
- [ ] Test: Logs corruption details
- [ ] Test: Continues operation (doesn't crash)

---

# PHASE 28.9: Cache Trait Implementation

**Goal**: Implement Cache trait for DiskCache
**Deliverable**: DiskCache works as drop-in replacement for MemoryCache
**Verification**: Can use through Arc<dyn Cache>

## 28.9.1: DiskCache Structure

### Main Structure
```rust
pub struct DiskCache {
    backend: Arc<dyn DiskBackend>,
    index: Arc<CacheIndex>,
    config: DiskCacheConfig,
    stats: Arc<CacheStatsTracker>,
}
```

- [ ] Test: Can create DiskCache
- [ ] Test: Contains backend (either tokio::fs or io-uring)
- [ ] Test: Contains index
- [ ] Test: Contains config
- [ ] Test: Contains stats tracker

---

## 28.9.2: Backend Selection at Compile Time

### Conditional Compilation
```rust
#[cfg(all(target_os = "linux", not(test)))]
type DefaultBackend = UringBackend;

#[cfg(any(not(target_os = "linux"), test))]
type DefaultBackend = TokioFsBackend;
```

- [ ] Test: Linux builds use UringBackend
- [ ] Test: macOS builds use TokioFsBackend
- [ ] Test: Tests use TokioFsBackend (consistent across platforms)
- [ ] Test: Only one backend compiled into binary

---

## 28.9.3: Cache::get() Implementation

### Get Logic
```rust
async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>> {
    // 1. Check index
    let metadata = self.index.get(key)?;

    // 2. Check if expired
    if metadata.is_expired() {
        self.delete(key).await?;
        return Ok(None);
    }

    // 3. Read from disk
    let data = self.backend.read_file(&metadata.file_path).await?;
    let meta_json = self.backend.read_file(&metadata.meta_path).await?;

    // 4. Update access time
    self.index.touch(key);

    // 5. Build CacheEntry
    Ok(Some(CacheEntry::from_disk(data, meta_json)?))
}
```

- [ ] Test: Returns None if key not in index
- [ ] Test: Returns None if entry expired
- [ ] Test: Reads data and metadata from disk
- [ ] Test: Updates last_accessed_at
- [ ] Test: Increments hit counter
- [ ] Test: Increments miss counter on not found

---

## 28.9.4: Cache::set() Implementation

### Set Logic
```rust
async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<()> {
    // 1. Validate size
    if entry.size_bytes() > self.config.max_item_size {
        return Err(CacheError::StorageFull);
    }

    // 2. Generate file paths
    let (data_path, meta_path) = self.generate_paths(&key);

    // 3. Write files atomically
    self.backend.write_file_atomic(&data_path, entry.data).await?;
    self.backend.write_file_atomic(&meta_path, entry.metadata_json()).await?;

    // 4. Update index
    self.index.insert(key, EntryMetadata { ... });

    // 5. Trigger eviction if needed
    self.evict_if_needed().await?;

    Ok(())
}
```

- [ ] Test: Rejects entries larger than max_item_size
- [ ] Test: Writes data and metadata atomically
- [ ] Test: Updates index
- [ ] Test: Triggers eviction if cache full
- [ ] Test: Returns error on disk full

---

## 28.9.5: Cache::delete() Implementation

### Delete Logic
- [ ] Test: Removes entry from index
- [ ] Test: Deletes .data file
- [ ] Test: Deletes .meta file
- [ ] Test: Updates total size
- [ ] Test: Returns true if entry existed
- [ ] Test: Returns false if entry didn't exist
- [ ] Test: Doesn't increment eviction counter (manual delete)

---

## 28.9.6: Cache::clear() Implementation

### Clear Logic
- [ ] Test: Removes all entries from index
- [ ] Test: Deletes all files in entries/
- [ ] Test: Resets total size to 0
- [ ] Test: Preserves hit/miss stats
- [ ] Test: Doesn't increment eviction counter

---

## 28.9.7: Cache::stats() Implementation

### Stats Aggregation
- [ ] Test: Returns current statistics
- [ ] Test: Includes hits, misses, evictions
- [ ] Test: Includes entry count from index
- [ ] Test: Includes total size
- [ ] Test: Includes max size from config
- [ ] Test: Includes backend type (io-uring or tokio::fs)

---

# PHASE 28.10: Cross-Platform Testing

**Goal**: Validate both backends work correctly on all platforms
**Deliverable**: Comprehensive test suite passing on Linux and macOS
**Verification**: CI passes on both platforms

## 28.10.1: Platform-Specific Tests

### Linux Tests (io-uring)
- [ ] Test: All tests pass with UringBackend
- [ ] Test: io-uring specific features work
- [ ] Test: Buffer pool works correctly
- [ ] Test: No file descriptor leaks
- [ ] Test: Explicit close() calls succeed

### macOS Tests (tokio::fs)
- [ ] Test: All tests pass with TokioFsBackend
- [ ] Test: No platform-specific issues
- [ ] Test: Same behavior as Linux (functional equivalence)

### Windows Tests (tokio::fs)
- [ ] Test: All tests pass with TokioFsBackend
- [ ] Test: Path handling works correctly
- [ ] Test: CRLF line endings handled

---

## 28.10.2: Integration Tests

### Basic Operations
- [ ] Test: Can store and retrieve 100 different entries
- [ ] Test: Cache hit rate improves with repeated access
- [ ] Test: Eviction works when cache fills up
- [ ] Test: TTL expiration works end-to-end

### Restart Persistence
- [ ] Test: Cache survives process restart (Linux)
- [ ] Test: Cache survives process restart (macOS)
- [ ] Test: Index loaded correctly after restart
- [ ] Test: All entries accessible after restart

### Large File Handling
- [ ] Test: Can cache 10MB files
- [ ] Test: Can cache 100MB files
- [ ] Test: Handles 1000+ files efficiently
- [ ] Test: Handles 10GB cache size

---

## 28.10.3: Error Injection Tests

### Filesystem Errors
- [ ] Test: Handles disk full error
- [ ] Test: Handles permission denied error
- [ ] Test: Handles read-only filesystem
- [ ] Test: Handles corrupted files

### Concurrent Access
- [ ] Test: Multiple threads reading simultaneously
- [ ] Test: Multiple threads writing simultaneously
- [ ] Test: Mixed read/write workload
- [ ] Test: No race conditions in index

---

## 28.10.4: Edge Cases

### Special Files
- [ ] Test: Handles empty files (0 bytes)
- [ ] Test: Handles very small files (1 byte)
- [ ] Test: Handles files at max_item_size boundary
- [ ] Test: Rejects files over max_item_size

### Special Keys
- [ ] Test: Handles Unicode cache keys
- [ ] Test: Handles very long cache keys
- [ ] Test: Handles special characters in keys
- [ ] Test: Prevents path traversal in keys

---

# PHASE 28.11: Performance Validation

**Goal**: Validate performance improvements on Linux, no regression elsewhere
**Deliverable**: Performance benchmark results
**Verification**: io-uring shows 2-3x improvement on Linux

## 28.11.1: Benchmark Setup

### Benchmark Infrastructure
- [ ] Test: Create benchmark harness using criterion
- [ ] Test: Benchmark runs on both backends
- [ ] Test: Isolated environment (tmpfs for consistency)
- [ ] Test: Multiple iterations for statistical significance

---

## 28.11.2: Small File Benchmarks (4KB)

### Read Operations
- [ ] Benchmark: tokio::fs sequential reads (baseline)
- [ ] Benchmark: io-uring sequential reads (Linux)
- [ ] Target: 2-3x throughput improvement on Linux
- [ ] Verify: No regression on macOS

### Write Operations
- [ ] Benchmark: tokio::fs sequential writes (baseline)
- [ ] Benchmark: io-uring sequential writes (Linux)
- [ ] Target: 2-3x throughput improvement on Linux

### Mixed Workload
- [ ] Benchmark: 70% reads, 30% writes (tokio::fs)
- [ ] Benchmark: 70% reads, 30% writes (io-uring)
- [ ] Target: 2x overall throughput improvement on Linux

---

## 28.11.3: Large File Benchmarks (10MB)

### Sequential I/O
- [ ] Benchmark: tokio::fs large file read (baseline)
- [ ] Benchmark: io-uring large file read (Linux)
- [ ] Target: 20-40% throughput improvement on Linux
- [ ] Verify: Memory usage similar

### Concurrent I/O
- [ ] Benchmark: 10 concurrent large file reads (tokio::fs)
- [ ] Benchmark: 10 concurrent large file reads (io-uring)
- [ ] Target: Better CPU utilization with io-uring

---

## 28.11.4: Latency Benchmarks

### P50, P95, P99 Latency
- [ ] Benchmark: Get latency distribution (tokio::fs)
- [ ] Benchmark: Get latency distribution (io-uring)
- [ ] Target: P95 latency <10ms (tokio::fs)
- [ ] Target: P95 latency <5ms (io-uring on Linux)

### Tail Latency
- [ ] Benchmark: P99.9 latency (tokio::fs)
- [ ] Benchmark: P99.9 latency (io-uring)
- [ ] Verify: io-uring has better tail latency

---

## 28.11.5: Resource Utilization

### CPU Usage
- [ ] Benchmark: CPU usage under load (tokio::fs)
- [ ] Benchmark: CPU usage under load (io-uring)
- [ ] Expected: io-uring uses less CPU (fewer syscalls)

### Memory Usage
- [ ] Benchmark: Memory usage (tokio::fs)
- [ ] Benchmark: Memory usage (io-uring with buffer pool)
- [ ] Verify: Buffer pool doesn't cause unbounded growth

### File Descriptors
- [ ] Benchmark: FD count under load (tokio::fs)
- [ ] Benchmark: FD count under load (io-uring)
- [ ] Verify: No file descriptor leaks

---

## 28.11.6: Stress Testing

### Sustained Load
- [ ] Test: 1000 ops/s for 10 minutes (tokio::fs)
- [ ] Test: 1000 ops/s for 10 minutes (io-uring)
- [ ] Verify: No memory leaks
- [ ] Verify: Performance stable over time

### Burst Load
- [ ] Test: Sudden spike to 10,000 ops/s
- [ ] Verify: Handles backpressure
- [ ] Verify: Recovers after burst
- [ ] Verify: No crashes or deadlocks

---

## 28.11.7: Performance Report

### Benchmark Results Template
```markdown
# Disk Cache Performance Report

## Environment
- OS: Ubuntu 22.04 (Linux 6.1.0) / macOS 14.0
- CPU: AMD EPYC 7763 / Apple M2
- Disk: NVMe SSD
- Memory: 128GB

## Results

### Small Files (4KB)
| Metric | tokio::fs | io-uring | Improvement |
|--------|-----------|----------|-------------|
| Throughput | 8,234 ops/s | 21,567 ops/s | 2.6x |
| P95 Latency | 450µs | 180µs | 2.5x faster |
| CPU Usage | 12% | 8% | 33% less |

### Large Files (10MB)
| Metric | tokio::fs | io-uring | Improvement |
|--------|-----------|----------|-------------|
| Throughput | 112 files/s | 156 files/s | 1.4x |
| P95 Latency | 9.8ms | 7.2ms | 27% faster |
| CPU Usage | 18% | 15% | 17% less |
```

- [ ] Document: Generate performance report
- [ ] Document: Include all metrics
- [ ] Document: Compare both backends
- [ ] Document: Include recommendations

---

## Summary

### Phase 28 Deliverables

**COMPLETED**:
- ✅ Hybrid disk cache with dual backends
- ✅ io-uring backend for Linux 5.10+
- ✅ tokio::fs backend for all platforms
- ✅ Compile-time backend selection
- ✅ Shared abstractions and index management
- ✅ LRU eviction and recovery
- ✅ Cache trait implementation
- ✅ Cross-platform testing
- ✅ Performance validation

**Performance Targets**:
- ✅ tokio::fs: <10ms P95 latency (all platforms)
- ✅ io-uring: <5ms P95 latency (Linux)
- ✅ 2-3x throughput improvement on Linux
- ✅ No regression on other platforms

**Quality Gates**:
- ✅ All tests pass on Linux
- ✅ All tests pass on macOS
- ✅ No clippy warnings
- ✅ Code formatted
- ✅ Documentation complete

---

## Estimated Timeline

### Week 1: Foundation (Days 1-3)
- Day 1: 28.1-28.2 (Abstractions & trait)
- Day 2: 28.3-28.4 (File structure & index)
- Day 3: Testing & validation

### Week 2: Backends (Days 4-7)
- Day 4: 28.5 (tokio::fs backend)
- Day 5: 28.6 (io-uring backend)
- Day 6: 28.7-28.8 (Eviction & recovery)
- Day 7: Testing

### Week 3: Integration (Days 8-10)
- Day 8: 28.9 (Cache trait implementation)
- Day 9: 28.10 (Cross-platform testing)
- Day 10: 28.11 (Performance validation)

**Total Time**: 10 days (vs. 7 days original plan)

---

**Ready to implement?** This plan integrates both backends from the start, ensuring optimal performance on Linux while maintaining portability.

**Next Step**: Say "go" to start implementing Phase 28.1!
