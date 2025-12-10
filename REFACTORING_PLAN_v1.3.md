# Yatagarasu v1.3 Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan for the Yatagarasu S3 proxy codebase to improve:
- Code readability and simplicity
- Maintainability and modularity
- Efficiency and idiomatic Rust patterns
- Clean code design principles
- Test coverage (>70%, target 90%)

**Current State:**
- 876 unit tests passing (100%)
- Zero clippy warnings
- ~45,000 lines of Rust code across 60+ source files
- Test coverage on library code: ~98.43%

**No Test Cheats Found:** Code review confirmed all tests are legitimate with no hardcoded bypasses.

---

## Phase 1: Structural Refactoring (No Behavioral Changes)

### 1.1 Extract Common Initialization in proxy/mod.rs

**File:** `src/proxy/mod.rs`
**Issue:** `new()` (lines 104-305) and `with_reload()` (lines 308-521) share ~90% duplicated code.

**Refactoring:**
```rust
// Extract common initialization to a builder pattern or helper method
impl YatagarasuProxy {
    fn initialize_from_config(config: Config) -> ProxyComponents {
        // Common initialization logic
    }

    pub fn new(config: Config) -> Self {
        let components = Self::initialize_from_config(config);
        Self::build_from_components(components, None)
    }

    pub fn with_reload(config: Config, config_path: PathBuf) -> Self {
        let components = Self::initialize_from_config(config);
        let reload_manager = Arc::new(ReloadManager::new(config_path));
        Self::build_from_components(components, Some(reload_manager))
    }
}
```

**Lines affected:** ~200 lines of duplication removed
**Commit prefix:** `[STRUCTURAL]`

---

### 1.2 Modularize Large Files

#### 1.2.1 Split cache/mod.rs (7603 lines)

**Current structure:** Single massive file with multiple concerns.

**Proposed structure:**
```
src/cache/
├── mod.rs              # Re-exports, trait definitions (~200 lines)
├── config.rs           # CacheConfig, MemoryCacheConfig, etc. (~300 lines)
├── entry.rs            # CacheEntry, CacheKey, CacheStats (~400 lines)
├── memory.rs           # Memory cache implementation (~600 lines)
├── traits.rs           # Cache trait definition (~100 lines)
├── disk/               # (existing)
├── redis/              # (existing)
├── tiered.rs           # (existing)
└── warming.rs          # (existing)
```

**Commit prefix:** `[STRUCTURAL]`

---

#### 1.2.2 Split config/mod.rs (3497 lines)

**Proposed structure:**
```
src/config/
├── mod.rs              # Re-exports, Config struct (~200 lines)
├── server.rs           # ServerConfig, SecurityLimitsConfig
├── bucket.rs           # BucketConfig, S3Config, S3Replica
├── jwt.rs              # JwtConfig, TokenSource, ClaimRule
├── rate_limit.rs       # RateLimitConfigYaml, etc.
├── circuit_breaker.rs  # CircuitBreakerConfigYaml
├── retry.rs            # RetryConfigYaml
├── audit.rs            # AuditLogConfig, AuditFileConfig, etc.
├── authorization.rs    # AuthorizationConfig (OPA/OpenFGA)
└── validation.rs       # Config::validate() logic
```

**Commit prefix:** `[STRUCTURAL]`

---

#### 1.2.3 Split proxy/mod.rs (4033 lines)

**Proposed structure:**
```
src/proxy/
├── mod.rs              # YatagarasuProxy struct, ProxyHttp impl (~800 lines)
├── init.rs             # Initialization logic (extracted from new/with_reload)
├── handlers.rs         # Request handlers (admin, health, metrics)
├── auth.rs             # Authentication flow integration
├── routing.rs          # Routing helpers
├── metrics_export.rs   # Metrics export logic
└── context.rs          # RequestContext helpers
```

**Commit prefix:** `[STRUCTURAL]`

---

### 1.3 Improve Router Performance

**File:** `src/router/mod.rs`
**Issue:** `get_bucket_by_name()` uses O(n) linear search.

**Refactoring:**
```rust
pub struct Router {
    buckets: Vec<BucketConfig>,
    bucket_by_name: HashMap<String, usize>, // Add index for O(1) lookup
}

impl Router {
    pub fn get_bucket_by_name(&self, name: &str) -> Option<&BucketConfig> {
        self.bucket_by_name.get(name).map(|&idx| &self.buckets[idx])
    }
}
```

**Commit prefix:** `[STRUCTURAL]`

---

## Phase 2: Complete TODOs and Missing Features

### 2.1 Disk Cache Metadata Storage

**File:** `src/cache/disk/disk_cache.rs:89-92`
**Issue:** Metadata (content_type, etag, last_modified) not persisted.

**Current code:**
```rust
content_type: "application/octet-stream".to_string(), // TODO: Store in metadata
etag: "".to_string(), // TODO: Store in metadata
last_modified: None,  // TODO: Store in metadata
```

**Fix:** Update `EntryMetadata` struct to include these fields and serialize them.

**Commit prefix:** `[BEHAVIORAL]`

---

### 2.2 Track Disk Cache Hits/Misses

**File:** `src/cache/disk/disk_cache.rs:216-217`
**Issue:** Statistics not tracked.

**Current code:**
```rust
hits: 0,   // TODO: Track hits
misses: 0, // TODO: Track misses
```

**Fix:** Add atomic counters and increment on get() operations.

**Commit prefix:** `[BEHAVIORAL]`

---

### 2.3 Server Threads Configuration

**File:** `src/server/mod.rs:52`
**Issue:** Hardcoded `threads: 4`.

**Fix:** Add `threads` field to `ServerConfig` with configurable default.

```yaml
server:
  address: "0.0.0.0"
  port: 8080
  threads: 4  # New configurable field
```

**Commit prefix:** `[BEHAVIORAL]`

---

### 2.4 Async Cache Write-Through

**File:** `src/cache/tiered.rs:142`
**Issue:** Write operations may block.

**Current code:**
```rust
// TODO: Make this truly async (tokio::spawn) to avoid blocking
```

**Fix:** Use `tokio::spawn` for background writes to lower-tier caches.

**Commit prefix:** `[BEHAVIORAL]`

---

### 2.5 Error Classification for Failover

**File:** `src/replica_set/mod.rs:1284, 1361`
**Issue:** All errors trigger failover, including 4xx client errors.

**Fix:** Implement error classification:
- 4xx errors: Don't trigger failover (client error)
- 5xx errors: Trigger failover (server error)
- Network errors: Trigger failover

**Commit prefix:** `[BEHAVIORAL]`

---

## Phase 3: Code Quality Improvements

### 3.1 Replace Magic Numbers with Constants

**Create:** `src/constants.rs`

```rust
// Server defaults
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 1000;
pub const DEFAULT_THREADS: usize = 4;

// S3 defaults
pub const DEFAULT_S3_TIMEOUT_SECS: u64 = 20;
pub const DEFAULT_CONNECTION_POOL_SIZE: usize = 50;

// Security defaults
pub const DEFAULT_MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB
pub const DEFAULT_MAX_HEADER_SIZE: usize = 64 * 1024;       // 64 KB
pub const DEFAULT_MAX_URI_LENGTH: usize = 8192;             // 8 KB

// Cache defaults
pub const DEFAULT_MAX_ITEM_SIZE_MB: u64 = 10;
pub const DEFAULT_MAX_CACHE_SIZE_MB: u64 = 1024;
pub const DEFAULT_TTL_SECONDS: u64 = 3600;

// Circuit breaker defaults
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
pub const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;
pub const DEFAULT_CB_TIMEOUT_SECS: u64 = 60;
pub const DEFAULT_HALF_OPEN_MAX_REQUESTS: u32 = 3;

// Retry defaults
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;
pub const DEFAULT_INITIAL_BACKOFF_MS: u64 = 100;
pub const DEFAULT_MAX_BACKOFF_MS: u64 = 1000;

// Audit defaults
pub const DEFAULT_MAX_FILE_SIZE_MB: u64 = 50;
pub const DEFAULT_MAX_BACKUP_FILES: u32 = 5;
pub const DEFAULT_AUDIT_BUFFER_SIZE: usize = 1024 * 1024; // 1 MB
pub const DEFAULT_EXPORT_INTERVAL_SECS: u64 = 60;

// OPA/OpenFGA defaults
pub const DEFAULT_OPA_TIMEOUT_MS: u64 = 100;
pub const DEFAULT_OPA_CACHE_TTL_SECS: u64 = 60;
pub const DEFAULT_OPENFGA_TIMEOUT_MS: u64 = 100;
pub const DEFAULT_OPENFGA_CACHE_TTL_SECS: u64 = 60;
```

**Commit prefix:** `[STRUCTURAL]`

---

### 3.2 Improve Error Handling

**Files:** Various
**Issue:** Some error messages could be more descriptive.

**Example improvement in S3Response:**
```rust
impl S3Response {
    pub fn get_error_code(&self) -> Option<String> {
        // Current: Manual XML parsing
        // Improved: Use quick-xml or roxmltree for robustness
    }
}
```

**Commit prefix:** `[BEHAVIORAL]`

---

### 3.3 Reduce Allocations in Hot Paths

**File:** `src/auth/mod.rs`
**Issue:** Multiple `to_string()` allocations in token extraction.

**Before:**
```rust
.map(|s| s.trim().to_string())
.filter(|s| !s.is_empty())
```

**After:**
```rust
.filter(|s| !s.trim().is_empty())
.map(|s| s.trim().to_string())
```

**Commit prefix:** `[STRUCTURAL]`

---

## Phase 4: Remove Unused Code

### 4.1 Audit for Dead Code

Run `cargo +nightly udeps` to identify unused dependencies.

### 4.2 Remove Commented/Disabled Code

Search for patterns:
- `// TODO Phase 30: Initialize cache from config if enabled`
- `let cache = None; // Temporarily None until...`

Either complete the feature or remove the dead code.

**Commit prefix:** `[STRUCTURAL]`

---

## Phase 5: Documentation and Testing

### 5.1 Add Missing Doc Comments

Files with insufficient documentation:
- `src/resources.rs`
- `src/security/mod.rs`
- `src/observability/slow_query.rs`

### 5.2 Increase Integration Test Coverage

Current: 67 passed, 52 failed (Docker required), 254 ignored.

Focus areas:
- Error handling edge cases
- Cache eviction scenarios
- Rate limiting edge cases

---

## Implementation Order

### Sprint 1: Structural Cleanup (Week 1)
1. [x] Review codebase - DONE
2. [x] 1.1 Extract common initialization in proxy - DONE
3. [x] 1.3 Improve Router performance - DONE
4. [x] 3.1 Replace magic numbers with constants - DONE

### Sprint 2: Module Splitting (Week 2)
1. [x] 1.2.1 Split cache/mod.rs - DONE (7603 lines → 6 focused modules, largest 1061 lines)
2. [ ] 1.2.2 Split config/mod.rs
3. [ ] 1.2.3 Split proxy/mod.rs

### Sprint 3: Complete TODOs (Week 3)
1. [ ] 2.1 Disk cache metadata storage
2. [ ] 2.2 Track disk cache hits/misses
3. [ ] 2.3 Server threads configuration
4. [ ] 2.4 Async cache write-through

### Sprint 4: Quality & Cleanup (Week 4)
1. [ ] 2.5 Error classification for failover
2. [ ] 3.2 Improve error handling
3. [ ] 3.3 Reduce allocations in hot paths
4. [ ] 4.1 Remove dead code
5. [ ] 5.1 Add missing documentation

---

## Quality Gates

Before each commit:
```bash
cargo test --lib        # All unit tests pass
cargo clippy -- -D warnings  # No warnings
cargo fmt --check       # Code formatted
```

Before release:
```bash
cargo tarpaulin --out Html  # Coverage >70%
cargo bench             # No performance regression
```

---

## Risk Assessment

| Refactoring | Risk | Mitigation |
|-------------|------|------------|
| Split large modules | Low | No behavioral changes, comprehensive tests |
| Extract proxy init | Medium | Careful review, integration tests |
| Router HashMap | Low | Existing tests cover lookups |
| Disk cache metadata | Medium | Add new tests, backwards compatible |
| Async write-through | High | Feature flag, load testing |
| Error classification | Medium | Add comprehensive tests |

---

## Metrics

### Before Refactoring
- Total source lines: ~45,000
- Largest file: cache/mod.rs (7,603 lines)
- Unit test count: 876
- Clippy warnings: 0
- Known TODOs: 12

### Target After Refactoring
- Largest file: <1,000 lines
- All TODOs resolved or tracked in issues
- Unit test count: >900
- Code coverage: >70% (target 90%)
- Performance: No regression

---

## Appendix: Files Reviewed

| File | Lines | Status |
|------|-------|--------|
| src/lib.rs | 27 | Clean |
| src/main.rs | 189 | Clean |
| src/error.rs | 96 | Clean |
| src/config/mod.rs | 3,497 | Needs split |
| src/proxy/mod.rs | 4,033 | Needs refactor |
| src/auth/mod.rs | 670 | Minor improvements |
| src/router/mod.rs | 62 | Needs HashMap |
| src/s3/mod.rs | 562 | Clean |
| src/cache/mod.rs | 7,603 | Needs split |
| src/cache/tiered.rs | 1,351 | Has TODO |
| src/cache/disk/disk_cache.rs | - | Has TODOs |
| src/replica_set/mod.rs | 2,103 | Has TODOs |
| src/server/mod.rs | - | Has hardcoded value |
| src/audit/mod.rs | 3,938 | Needs split |

---

*Generated: 2025-12-10*
*Author: Claude Code Review*
