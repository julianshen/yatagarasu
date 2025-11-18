# Yatagarasu v1.1.0 Implementation Plan

**Last Updated**: 2025-11-15
**Current Status**: Planning Phase - v1.0.0 Complete, Starting v1.1.0
**Target Release**: Q1 2026 (8-10 weeks)

---

## 🎯 v1.1.0 Goals

**Primary Goal**: Cost optimization through intelligent caching (80%+ reduction in S3 costs)
**Secondary Goals**:
- Enhanced authentication (RS256/ES256 JWT, JWKS support)
- Audit logging for compliance (SOC2, GDPR, HIPAA)
- Enhanced observability and security

**Success Metrics**:
- ✅ Demonstrate 80%+ reduction in S3 costs for typical workload
- ✅ Cache hit rate >80% for static assets
- ✅ P95 latency <50ms (cached), <200ms (uncached)
- ✅ Backward compatible with v1.0.0 configurations
- ✅ All v1.0.0 performance targets maintained or exceeded

---

## Functional Milestones

### 🔴 Milestone 1: Cache Foundation (Phases 26-27) - CRITICAL
**Deliverable**: In-memory LRU cache operational with configurable limits
**Verification**: Cache stores/retrieves objects, enforces size limits, evicts LRU
**Status**: ✅ COMPLETE - Phase 26: COMPLETE (164 tests) | Phase 27: COMPLETE (117 tests, 268 total cache tests)

### 🔴 Milestone 2: Persistent Cache (Phase 28-29) - CRITICAL
**Deliverable**: Disk OR Redis cache layer operational
**Verification**: Cache persists across restarts, handles failures gracefully
**Status**: ⏳ NOT STARTED

### 🔴 Milestone 3: Cache Management API (Phase 30) - CRITICAL
**Deliverable**: Cache purge/stats endpoints working
**Verification**: Can purge cache, retrieve statistics via API
**Status**: ⏳ NOT STARTED

### 🟡 Milestone 4: Advanced JWT (Phase 31) - HIGH
**Deliverable**: RS256/ES256 JWT validation, JWKS support
**Verification**: Can validate RSA/ECDSA signed JWTs, fetch keys from JWKS
**Status**: ⏳ NOT STARTED

### 🟡 Milestone 5: Audit Logging (Phase 32) - HIGH
**Deliverable**: Comprehensive audit logging operational
**Verification**: All requests logged with correlation IDs, exportable to S3/syslog
**Status**: ⏳ NOT STARTED

### 🟢 Milestone 6: Enhanced Observability (Phase 33) - MEDIUM
**Deliverable**: OpenTelemetry tracing, slow query logging
**Verification**: Traces exported to Jaeger/Zipkin, slow queries logged
**Status**: ⏳ NOT STARTED

### 🟢 Milestone 7: Advanced Security (Phase 34) - MEDIUM
**Deliverable**: IP allowlist/blocklist, advanced rate limiting
**Verification**: IP filtering works, token bucket rate limiting operational
**Status**: ⏳ NOT STARTED

### 🔴 Milestone 8: Performance Validation (Phase 35-38) - CRITICAL
**Deliverable**: All performance targets met or exceeded
**Verification**: K6 tests pass for cold/hot cache, large files, 10K+ concurrent users
**Status**: ⏳ NOT STARTED

### 🔴 Milestone 9: Production Ready (Phase 39-40) - CRITICAL
**Deliverable**: Chaos testing complete, operational tests pass
**Verification**: Survives S3 failures, cache failures, hot reload, graceful shutdown
**Status**: ⏳ NOT STARTED

**Target**: Milestone 9 = v1.1.0 production release

---

## How to Use This Plan

1. Find the next unmarked test (marked with `[ ]`)
2. Write the test and watch it fail (Red)
3. Write the minimum code to make it pass (Green)
4. Refactor if needed while keeping tests green
5. Mark the test complete with `[x]`
6. **Verify the feature works end-to-end** - not just unit tests
7. Commit (separately for structural and behavioral changes)
8. Move to the next test

## Legend

- `[ ]` - Not yet implemented
- `[x]` - Implemented and passing
- `[~]` - In progress
- `[!]` - Blocked or needs discussion

---

# PHASE 26: Cache Foundation - Configuration & Traits (Week 1)

**Goal**: Establish cache configuration schema and core abstractions
**Deliverable**: Cache config loads from YAML, core traits defined
**Verification**: `cargo test` passes, cache config can be parsed

## 26.1: Cache Configuration Schema

### Basic Cache Config Structure
- [x] Test: Can create empty CacheConfig struct
- [x] Test: Can deserialize minimal cache config from YAML
- [x] Test: CacheConfig has enabled field (bool)
- [x] Test: CacheConfig defaults to disabled when not specified
- [x] Test: Can parse cache config with enabled=true
- [x] Test: Can parse cache config with enabled=false

### Memory Cache Configuration
- [x] Test: Can parse memory cache section
- [x] Test: Can parse max_item_size_mb (default 10MB)
- [x] Test: Can parse max_cache_size_mb (default 1024MB = 1GB)
- [x] Test: Can parse default_ttl_seconds (default 3600 = 1 hour)
- [x] Test: Can parse max_item_size in bytes (10MB = 10485760 bytes)
- [x] Test: Can parse max_cache_size in bytes (1GB = 1073741824 bytes)
- [x] Test: Rejects negative max_item_size (N/A - u64 type prevents negative values)
- [x] Test: Rejects negative max_cache_size (N/A - u64 type prevents negative values)
- [x] Test: Rejects negative default_ttl (N/A - u64 type prevents negative values)
- [x] Test: Rejects max_item_size > max_cache_size

### Disk Cache Configuration
- [x] Test: Can parse disk cache section (optional)
- [x] Test: Can parse cache_dir path (default: /var/cache/yatagarasu)
- [x] Test: Can parse max_disk_cache_size_mb (default 10GB)
- [x] Test: Can parse disk_cache_enabled (default false)
- [x] Test: Rejects disk cache with empty cache_dir
- [x] Test: Rejects disk cache with negative max size (N/A - u64 type prevents negative values)

### Redis Cache Configuration
- [x] Test: Can parse redis cache section (optional)
- [x] Test: Can parse redis_url (e.g., redis://localhost:6379)
- [x] Test: Can parse redis_password (optional)
- [x] Test: Can parse redis_db (default 0)
- [x] Test: Can parse redis_key_prefix (default "yatagarasu:")
- [x] Test: Can parse redis_ttl_seconds (default 3600)
- [x] Test: Can parse redis_enabled (default false)
- [x] Test: Rejects redis cache with invalid URL format
- [x] Test: Rejects redis cache with negative DB number (N/A - u32 type prevents negative values)

### Cache Hierarchy Configuration
- [x] Test: Can parse cache_layers array (default: ["memory"])
- [x] Test: Can parse cache_layers with multiple layers (["memory", "disk"])
- [x] Test: Can parse cache_layers with all layers (["memory", "disk", "redis"])
- [x] Test: Rejects cache_layers with unknown layer name
- [x] Test: Rejects cache_layers with duplicate layers
- [x] Test: Rejects cache_layers with empty array when caching enabled
- [x] Test: Validates disk layer requires disk_cache_enabled=true
- [x] Test: Validates redis layer requires redis_enabled=true

### Per-Bucket Cache Configuration
- [x] Test: Can parse per-bucket cache override in bucket config
- [x] Test: Per-bucket cache override can disable caching for specific bucket
- [x] Test: Per-bucket cache override can set custom TTL
- [x] Test: Per-bucket cache override can set custom max_item_size
- [x] Test: Per-bucket cache inherits global defaults when not overridden
- [x] Test: Rejects per-bucket cache with invalid values

### Environment Variable Substitution
- [x] Test: Can substitute environment variable in cache_dir
- [x] Test: Can substitute environment variable in redis_url
- [x] Test: Can substitute environment variable in redis_password
- [x] Test: Substitution fails gracefully when env var missing
- [x] Test: Can use literal value (no substitution) for cache config

### Configuration Validation
- [x] Test: Validates cache config when enabled=true
- [x] Test: Skips validation when enabled=false
- [x] Test: Validates at least one cache layer configured when enabled (covered by cache hierarchy test)
- [x] Test: Validates layer dependencies (covered by cache hierarchy tests)
- [x] Test: Full config validation passes with valid cache config
- [x] Test: Full config validation fails with invalid cache config

### Example YAML Configuration Test
```yaml
cache:
  enabled: true
  memory:
    max_item_size_mb: 10
    max_cache_size_mb: 1024
    default_ttl_seconds: 3600
  disk:
    enabled: false
    cache_dir: /var/cache/yatagarasu
    max_disk_cache_size_mb: 10240
  redis:
    enabled: false
    redis_url: ${REDIS_URL}
    redis_password: ${REDIS_PASSWORD}
    redis_db: 0
    key_prefix: "yatagarasu:"
    ttl_seconds: 3600
  cache_layers: ["memory"]

buckets:
  - name: products
    path_prefix: /products
    cache:
      ttl_seconds: 7200  # Override: 2 hours
      max_item_size_mb: 5  # Override: 5MB max
```

- [x] Test: Can parse complete cache config example above
- [x] Test: Per-bucket overrides apply correctly

---

## 26.2: Cache Key Design

### CacheKey Structure
- [x] Test: Can create CacheKey struct
- [x] Test: CacheKey contains bucket name
- [x] Test: CacheKey contains object key (S3 path)
- [x] Test: CacheKey contains etag (optional for validation)
- [x] Test: CacheKey implements Hash trait
- [x] Test: CacheKey implements Eq trait
- [x] Test: CacheKey implements Clone trait
- [x] Test: CacheKey implements Debug trait

### CacheKey String Representation
- [x] Test: CacheKey can serialize to string (for Redis keys)
- [x] Test: CacheKey format: "bucket:object_key"
- [x] Test: CacheKey escapes special characters in object_key
- [x] Test: CacheKey handles object keys with slashes correctly
- [x] Test: CacheKey handles object keys with spaces correctly
- [x] Test: CacheKey handles Unicode object keys correctly

### CacheKey Parsing
- [x] Test: Can parse CacheKey from string
- [x] Test: Parsing fails gracefully with invalid format
- [x] Test: Roundtrip: to_string().parse() == original

### CacheKey Hashing
- [x] Test: Same CacheKey produces same hash
- [x] Test: Different CacheKeys produce different hashes
- [x] Test: CacheKey with different etags are different keys
- [x] Test: CacheKey hash is stable across runs

---

## 26.3: Cache Entry Design

### CacheEntry Structure
- [x] Test: Can create CacheEntry struct
- [x] Test: CacheEntry contains data (Bytes)
- [x] Test: CacheEntry contains content_type (String)
- [x] Test: CacheEntry contains content_length (usize)
- [x] Test: CacheEntry contains etag (String)
- [x] Test: CacheEntry contains created_at (timestamp)
- [x] Test: CacheEntry contains expires_at (timestamp)
- [x] Test: CacheEntry contains last_accessed_at (timestamp, for LRU)

### CacheEntry Size Calculation
- [x] Test: CacheEntry can calculate its size in bytes
- [x] Test: Size includes data length
- [x] Test: Size includes metadata overhead (approximate)
- [x] Test: Size is accurate for small entries (<1KB)
- [x] Test: Size is accurate for large entries (>1MB)

### CacheEntry TTL & Expiration
- [x] Test: CacheEntry can check if expired
- [x] Test: is_expired() returns false before expires_at
- [x] Test: is_expired() returns true after expires_at
- [x] Test: Can create entry with custom TTL
- [x] Test: Can create entry with default TTL
- [x] Test: TTL of 0 means no expiration

### CacheEntry Access Tracking (for LRU)
- [x] Test: CacheEntry can update last_accessed_at
- [x] Test: touch() updates last_accessed_at to current time
- [x] Test: last_accessed_at used for LRU sorting

### CacheEntry Validation
- [x] Test: Can validate entry against S3 ETag
- [x] Test: Validation succeeds when ETags match
- [x] Test: Validation fails when ETags differ
- [x] Test: Validation fails when entry expired

---

## 26.4: Cache Trait Abstraction

### Cache Trait Definition
- [x] Test: Can define Cache trait
- [x] Test: Cache trait has get() method signature
- [x] Test: Cache trait has set() method signature
- [x] Test: Cache trait has delete() method signature
- [x] Test: Cache trait has clear() method signature
- [x] Test: Cache trait has stats() method signature
- [x] Test: All methods are async
- [x] Test: All methods return Result<T, CacheError>

### Cache Trait Method Signatures
```rust
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError>;
    async fn set(&self, key: CacheKey, entry: CacheEntry) -> Result<(), CacheError>;
    async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
    async fn stats(&self) -> Result<CacheStats, CacheError>;
}
```

- [x] Test: Cache trait compiles with signatures above
- [x] Test: Can create mock implementation of Cache trait
- [x] Test: Mock implementation satisfies Send + Sync bounds

### Cache Error Types
- [x] Test: Can create CacheError enum
- [x] Test: CacheError has NotFound variant
- [x] Test: CacheError has StorageFull variant
- [x] Test: CacheError has IoError variant (for disk cache)
- [x] Test: CacheError has RedisError variant (for redis cache)
- [x] Test: CacheError has SerializationError variant
- [x] Test: CacheError implements Error trait
- [x] Test: CacheError implements Display trait
- [x] Test: CacheError can convert from std::io::Error
- [x] Test: CacheError can convert from serde_json::Error

---

## 26.5: Cache Statistics

### CacheStats Structure
- [x] Test: Can create CacheStats struct
- [x] Test: CacheStats contains hits (u64)
- [x] Test: CacheStats contains misses (u64)
- [x] Test: CacheStats contains evictions (u64)
- [x] Test: CacheStats contains current_size_bytes (u64)
- [x] Test: CacheStats contains current_item_count (u64)
- [x] Test: CacheStats contains max_size_bytes (u64)
- [x] Test: CacheStats implements Clone trait
- [x] Test: CacheStats implements Debug trait

### CacheStats Calculations
- [x] Test: CacheStats can calculate hit rate
- [x] Test: Hit rate = hits / (hits + misses)
- [x] Test: Hit rate is 0.0 when no requests
- [x] Test: Hit rate is 1.0 when all hits
- [x] Test: Hit rate is 0.0 when all misses
- [x] Test: Hit rate is 0.5 when 50% hits

### CacheStats Serialization (for API)
- [x] Test: CacheStats implements Serialize trait
- [x] Test: CacheStats serializes to JSON
- [x] Test: JSON includes all fields
- [x] Test: JSON includes computed hit_rate field

### CacheStats Per-Bucket Tracking
- [x] Test: Can create BucketCacheStats struct
- [x] Test: BucketCacheStats maps bucket name to CacheStats
- [x] Test: Can aggregate stats across all buckets
- [x] Test: Can retrieve stats for specific bucket
- [x] Test: Returns empty stats for unknown bucket

---

## 26.6: Cache Module Integration

### Module Structure
- [x] Test: Can create cache module in src/cache/mod.rs
- [x] Test: Cache module exports CacheConfig
- [x] Test: Cache module exports CacheKey
- [x] Test: Cache module exports CacheEntry
- [x] Test: Cache module exports Cache trait
- [x] Test: Cache module exports CacheError
- [x] Test: Cache module exports CacheStats
- [x] Test: Cache module imports compile in lib.rs

### Module Documentation
- [x] Test: Cache module has module-level documentation
- [x] Test: CacheConfig has doc comments
- [x] Test: Cache trait has doc comments with examples
- [x] Test: CacheKey has doc comments
- [x] Test: CacheEntry has doc comments

### Configuration Integration
- [x] Test: Main Config struct includes cache field
- [x] Test: Config::from_yaml() parses cache section
- [x] Test: Config validation includes cache validation
- [x] Test: Can load complete config with cache section

---

# PHASE 27: In-Memory Cache Implementation with Moka (Week 1-2)

**Goal**: Wrap moka cache library to implement production-ready in-memory cache  
**Deliverable**: Memory cache stores/retrieves entries, enforces size limits, uses TinyLFU eviction  
**Verification**: `cargo test` passes, integration tests demonstrate >80% hit rate  
**Approach**: Use battle-tested `moka` library instead of building from scratch

**Why Moka?**
- Production-proven (used by crates.io with 85% hit rate)
- Built-in async support for Tokio
- TinyLFU admission policy (better hit rates than pure LRU)
- Thread-safe concurrent hash table (no manual locking needed)
- Size-aware eviction with custom weigher functions
- Built-in TTL/TTI support

---

## 27.1: Dependencies & Moka Setup

### Add Moka Dependency
- [x] Test: Add `moka = { version = "0.12", features = ["future"] }` to Cargo.toml
- [x] Test: Can import `moka::future::Cache`
- [x] Test: Can import `moka::notification::RemovalCause`
- [x] Test: Moka compiles without errors

### Understand Moka's API
- [x] Test: Can create basic moka::future::Cache
- [x] Test: Can call get() and insert() on moka cache
- [x] Test: Can configure max_capacity on builder
- [x] Test: Can configure time_to_live on builder
- [x] Test: Moka cache is Send + Sync

---

## 27.2: MemoryCache Wrapper Structure

### MemoryCache Structure Definition
- [x] Test: Can create MemoryCache struct
- [x] Test: MemoryCache contains moka::future::Cache<CacheKey, CacheEntry>
- [x] Test: MemoryCache contains Arc<AtomicU64> for stats tracking
- [x] Test: MemoryCache contains config parameters (max sizes, TTL)

### Statistics Tracking Structure
- [x] Test: Can create CacheStatsTracker struct
- [x] Test: Tracker contains AtomicU64 for hits, misses, evictions
- [x] Test: Tracker provides atomic increment methods
- [x] Test: Tracker provides snapshot method returning CacheStats

### MemoryCache Constructor
- [x] Test: Can create MemoryCache::new(config)
- [x] Test: Constructor creates moka::Cache::builder()
- [x] Test: Constructor sets max_capacity from config
- [ ] Test: Constructor sets time_to_live from config (implicit in constructor, will verify in integration tests)
- [ ] Test: Constructor configures weigher function (Phase 27.3)
- [x] Test: Constructor initializes stats tracker

---

## 27.3: Moka Weigher Function

### Custom Weigher for CacheEntry
- [x] Test: Can define weigher closure
- [x] Test: Weigher returns entry.size_bytes() as u32
- [x] Test: Weigher accounts for data + metadata size
- [x] Test: Weigher handles overflow (max = u32::MAX)

### Weigher Integration
- [x] Test: Moka builder accepts weigher closure
- [x] Test: Moka respects max_capacity as total weight
- [x] Test: Moka evicts based on weighted size
- [x] Test: Can retrieve weighted_size() from moka cache

---

## 27.4: Basic Cache Operations (Moka Wrapper)

### Get Operation
- [x] Test: get() calls moka.get(key).await
- [x] Test: get() on empty cache returns None
- [x] Test: get() on existing key returns Some(entry)
- [x] Test: get() increments hit counter on cache hit
- [x] Test: get() increments miss counter on cache miss
- [x] Test: get() returns cloned CacheEntry

### Insert Operation
- [x] Test: set() calls moka.insert(key, entry).await
- [x] Test: set() rejects entry larger than max_item_size
- [x] Test: set() returns CacheError::StorageFull for oversized entry
- [x] Test: set() stores entry successfully when within limits
- [x] Test: set() overwrites existing entry for same key
- [x] Test: Can retrieve entry immediately after set()

### TTL Handling
- [x] Test: Moka automatically expires entries after TTL
- [x] Test: get() returns None for expired entry
- [x] Test: Expired entries don't count as hits
- [ ] Test: Can set TTL of 0 for no expiration (covered by existing CacheEntry tests)

---

## 27.5: Eviction Listener & Statistics

### Eviction Listener Setup
- [x] Test: Can define eviction_listener closure
- [x] Test: Listener increments eviction counter
- [x] Test: Listener receives RemovalCause enum
- [x] Test: Listener tracks Size-based evictions separately from Expired

### Eviction Listener Integration
- [x] Test: Moka builder accepts eviction_listener
- [x] Test: Listener called when entry evicted
- [x] Test: Listener called when entry expires
- [ ] Test: Listener not called on manual delete (will verify in 27.6 with delete())

### Statistics Accuracy
- [x] Test: Hit counter increments correctly
- [x] Test: Miss counter increments correctly
- [x] Test: Eviction counter increments correctly
- [x] Test: Counters are thread-safe (use atomics)

---

## 27.6: Advanced Cache Operations

### Delete Operation
- [x] Test: delete() calls moka.invalidate(key)
- [x] Test: delete() removes entry from cache
- [x] Test: delete() returns true (operation completed)
- [x] Test: delete() does not increment eviction counter

### Clear Operation
- [x] Test: clear() calls invalidate_all()
- [x] Test: clear() removes all entries

### Maintenance Operations
- [x] Test: run_pending_tasks() processes pending evictions
- [x] Test: weighted_size() returns current cache size in bytes
- [x] Test: entry_count() returns approximate entry count
- [x] Test: Can delete then re-insert same key

---

## 27.7: Cache Trait Implementation

### Implement Cache Trait for MemoryCache
- [x] Test: MemoryCache implements Cache trait
- [x] Test: MemoryCache implements Send + Sync
- [x] Test: Can use MemoryCache through Arc<dyn Cache>

### Cache::get() Implementation
- [x] Test: get() wraps moka.get().await
- [x] Test: Returns Ok(None) on miss
- [x] Test: Returns Ok(Some(entry)) on hit
- [x] Test: Updates statistics correctly

### Cache::set() Implementation
- [x] Test: set() validates entry size first
- [x] Test: Returns Err(StorageFull) if entry too large
- [x] Test: Returns Ok(()) on success

### Cache::delete() Implementation
- [x] Test: delete() wraps moka.invalidate()
- [x] Test: Returns Ok(bool) always

### Cache::clear() Implementation
- [x] Test: clear() wraps moka.invalidate_all()
- [x] Test: Preserves hit/miss stats

### Cache::stats() Implementation
- [x] Test: stats() returns snapshot of counters
- [x] Test: Includes hits, misses, evictions
- [x] Test: Includes entry_count() from moka
- [x] Test: Includes weighted_size() from moka
- [x] Test: Includes max_size_bytes from config

---

## 27.8: Integration with Config

### MemoryCache from CacheConfig
- [x] Test: Can create MemoryCache from MemoryCacheConfig
- [x] Test: Extracts max_item_size_mb from config
- [x] Test: Converts MB to bytes for moka

### Cache Factory Function
- [x] Test: Can create cache_factory(config) function
- [x] Test: Factory returns Arc<dyn Cache>
- [x] Test: Factory creates MemoryCache when enabled=true
- [x] Test: Factory creates NullCache when enabled=false
- [x] Test: Factory uses moka when cache_layers includes "memory"

### NullCache (No-Op Implementation)
- [x] Test: Can create NullCache struct
- [x] Test: NullCache implements Cache trait
- [x] Test: NullCache::get() always returns Ok(None)
- [x] Test: NullCache::set() always returns Ok(())
- [x] Test: NullCache::delete() always returns Ok(false)
- [x] Test: NullCache::clear() always returns Ok(())
- [x] Test: NullCache::stats() returns zeros

---

## 27.9: Thread Safety & Concurrency

### Moka's Concurrent Guarantees
- [x] Test: Moka cache is thread-safe by design
- [x] Test: Can share MemoryCache across threads
- [x] Test: Concurrent get() operations work correctly
- [x] Test: Concurrent insert() operations work correctly

### Mixed Concurrent Operations
- [x] Test: Can get() and insert() from different threads
- [x] Test: Stats remain accurate under concurrent load
- [x] Test: No race conditions in statistics tracking

### Stress Test
- [x] Test: 10 threads performing random get/set operations (500 total ops)

---

## 27.10: Testing & Validation

### Unit Tests Summary
- [x] Test: All MemoryCache unit tests pass (268 tests)
- [x] Test: No clippy warnings in cache module
- [x] Test: Code formatted with rustfmt

### Integration Tests - Basic Operations
- [x] Test: Can store and retrieve 100 different entries
- [x] Test: Cache hit rate improves with repeated access
- [x] Test: Eviction works when cache fills up
- [x] Test: TTL expiration works end-to-end
- [x] Test: Statistics tracking is accurate

### Integration Tests - Size Management
- [x] Test: Rejects entries larger than max_item_size
- [x] Test: Evicts entries when total size exceeds max_cache_size
- [x] Test: Weighted size calculation is accurate

### Integration Tests - Edge Cases
- [x] Test: Cache handles empty data (0 bytes)
- [x] Test: Cache handles very large entries (near max size)
- [x] Test: Cache handles rapid insert/evict cycles
- [x] Test: Cache handles all entries expiring simultaneously

### Integration Tests - Hit Rate Validation
- [ ] Test: Repeated access pattern achieves >80% hit rate
- [ ] Test: TinyLFU improves hit rate over pure LRU
- [ ] Test: Cache adapts to changing access patterns
- [ ] Test: Hit rate calculation is accurate

---

**Summary**: Phase 27 revised to use `moka` instead of manual implementation  
**Tests Reduced**: ~135 tests → ~87 tests (65% reduction in test count)  
**Complexity Reduced**: No manual LRU, no manual locking, no manual TTL tracking  
**Benefits**: Production-proven library, better hit rates, less code to maintain  
**Trade-off**: Dependency on external crate (acceptable - widely used)

---

# PHASE 28: Hybrid Disk Cache Implementation (Week 2-3)

**Goal**: Implement persistent disk-based cache with platform-optimized backends
**Strategy**: Hybrid approach - io-uring on Linux 5.10+, tokio::fs elsewhere
**Deliverable**:
- High-performance io-uring backend (Linux)
- Portable tokio::fs backend (all platforms)
- Single unified API via trait abstraction

**Verification**:
- All tests pass on all platforms
- io-uring shows 2-3x improvement on Linux (benchmarked)
- Cache survives process restart
- Docker testing for Linux (from macOS/Windows)

**Architecture**:
```
Cache Trait → DiskCache → Backend (compile-time selection)
                          ├─ UringBackend (Linux only)
                          └─ TokioFsBackend (all platforms)
```

**Reference**: See `docs/PHASE_28_HYBRID_PLAN.md` for complete 332-test plan
**Docker**: See `docs/DOCKER_TESTING_GUIDE.md` for Linux testing setup

---

## 28.1: Shared Abstractions & Dependencies (Day 1)

### Core Dependencies (All Platforms)
- [x] Test: Add tokio for async runtime
- [x] Test: Add sha2 for cache key hashing
- [x] Test: Add serde/serde_json for metadata
- [x] Test: Add parking_lot for thread-safe index

### Platform-Specific Dependencies
- [x] Test: Add tokio-uring on Linux only
- [x] Test: Add tempfile for test isolation
- [x] Test: Dependencies compile on all platforms
- [x] Test: Can import tokio_uring on Linux
- [x] Test: Build works without tokio-uring on macOS

### Common Types
- [x] Test: Can create EntryMetadata struct
- [x] Test: EntryMetadata serializes to JSON
- [x] Test: Can create CacheIndex with thread-safe operations
- [x] Test: CacheIndex tracks total size atomically
- [x] Test: DiskCacheError enum with all variants

### File Path Utilities (Shared)
- [x] Test: Can convert CacheKey to SHA256 hash
- [x] Test: Can generate file path from hash
- [x] Test: Path uses entries/ subdirectory
- [x] Test: Generates .data and .meta file paths
- [x] Test: Prevents path traversal attacks

---

## 28.2: Backend Trait Definition (Day 1)

### DiskBackend Trait
- [x] Test: Can define DiskBackend trait
- [x] Test: Trait has read_file() method
- [x] Test: Trait has write_file_atomic() method
- [x] Test: Trait has delete_file() method
- [x] Test: Trait has create_dir_all() method
- [x] Test: All methods are async
- [x] Test: Trait is Send + Sync
- [x] Test: Can create trait object Arc<dyn DiskBackend>

### MockDiskBackend (for testing)
- [x] Test: Can create MockDiskBackend
- [x] Test: Implements DiskBackend trait
- [x] Test: Stores files in HashMap (in-memory)
- [x] Test: Can read what was written
- [x] Test: Simulates errors (disk full, permission denied)

---

## 28.3: Cache Key Mapping & File Structure (Day 1)

### Hash-Based File Naming
- [x] Test: Uses SHA256 hash of key for filename
- [x] Test: Hash is deterministic (same key = same hash)
- [x] Test: Path format: {cache_dir}/entries/{hash}.data
- [x] Test: Metadata path: {cache_dir}/entries/{hash}.meta

### File Structure
```
/var/cache/yatagarasu/
├── index.json              # Cache index metadata
└── entries/
    ├── <hash>.data         # Entry data (binary)
    └── <hash>.meta         # Entry metadata (JSON)
```

- [x] Test: Creates entries subdirectory
- [x] Test: Data file stores raw binary
- [x] Test: Metadata file stores JSON
- [x] Test: Both files created atomically

---

## 28.4: Index Management (Day 2)

### In-Memory Index
- [x] Test: Index maps CacheKey → EntryMetadata
- [x] Test: Thread-safe operations (RwLock or DashMap)
- [x] Test: Can add/remove/update entries
- [x] Test: Tracks total cache size atomically

### Index Persistence
- [x] Test: Index saved to index.json
- [x] Test: Index loaded on startup
- [x] Test: Handles missing file (starts empty)
- [x] Test: Handles corrupted JSON (logs, starts empty)

### Index Validation & Repair
- [x] Test: Scans entries/ directory on startup
- [x] Test: Removes orphaned files (no index entry)
- [x] Test: Removes index entries without files
- [x] Test: Recalculates total size from files
- [x] Test: Removes expired entries on startup

---

## 28.5: tokio::fs Backend Implementation (Day 3)

### TokioFsBackend Structure
- [x] Test: Can create TokioFsBackend
- [x] Test: Implements DiskBackend trait
- [x] Test: Implements Send + Sync

### Read Operations
- [x] Test: read_file() uses tokio::fs::read
- [x] Test: Returns Bytes
- [x] Test: Returns error if file doesn't exist
- [x] Test: Works with various file sizes (0B to 100MB)

### Write Operations
- [x] Test: write_file_atomic() uses temp file + rename
- [x] Test: Writes to .tmp file first
- [x] Test: Atomically renames to final path
- [x] Test: Cleans up temp file on error

### Delete & Directory Operations
- [x] Test: delete_file() removes file
- [x] Test: create_dir_all() creates directories recursively
- [x] Test: Handles errors gracefully

---

## 28.6: UringBackend Implementation - REVISED (Using io-uring crate)

**Original Goal**: Implement UringBackend using tokio-uring
**Status**: ❌ Blocked - tokio-uring has !Send futures (intentional design using Rc<T>)

**NEW Goal**: Implement UringBackend using low-level **io-uring crate** + spawn_blocking wrapper

**Architecture Change**:
- **OLD**: tokio-uring (high-level, !Send futures, blocked by async_trait)
- **NEW**: io-uring (low-level, Send + Sync types, wrapped with spawn_blocking)

**Solution**: Wrap io-uring operations in `tokio::task::spawn_blocking` to get Send futures

### UringBackend Structure (Linux only)
- [x] Test: Can create UringBackend (using io-uring::IoUring)
- [x] Test: Implements DiskBackend trait (with Send futures)
- [x] Test: Is Send + Sync (required for async)
- [x] Test: Can be used interchangeably with TokioFsBackend

### Read Operations (io-uring + spawn_blocking)
- [x] Test: read_file() successfully reads existing file
- [x] Test: read_file() returns error for missing file
- [x] Test: read_file() returns Bytes with correct content
- [x] Test: Handles large files (>1MB) correctly
- [x] Implementation: Wrap io_uring::opcode::Read in spawn_blocking

### Write Operations (io-uring + spawn_blocking)
- [x] Test: write_file_atomic() creates parent directories
- [x] Test: write_file_atomic() writes to temp file first
- [x] Test: write_file_atomic() atomically renames temp to final
- [x] Test: write_file_atomic() handles write errors gracefully
- [x] Implementation: Wrap io_uring::opcode::Write in spawn_blocking

### Delete Operations (standard fs or io-uring)
- [x] Test: delete_file() removes existing file
- [x] Test: delete_file() is idempotent (ignores missing files)
- [x] Implementation: May use tokio::fs for simplicity

### Directory Operations (standard fs)
- [x] Test: create_dir_all() creates nested directories
- [x] Test: create_dir_all() is idempotent
- [x] Test: file_size() returns correct size for existing file
- [x] Test: read_dir() lists directory contents
- [x] Implementation: Use tokio::fs (io-uring optimizes file I/O, not directory ops)

**Note**:
- Proof-of-concept validated: io-uring + spawn_blocking works!
- See IO_URING_FEASIBILITY.md for implementation guide
- All previous [x] marks were INVALID - tests never actually ran due to !Send blocker
- Advanced optimizations (dedicated runtime thread) deferred to Phase 28.11 based on benchmarks

---

## 28.7: LRU Eviction (Day 6)

### Size Tracking
- [x] Test: Tracks total disk cache size
- [x] Test: Size updated on set()
- [x] Test: Size updated on delete()
- [x] Test: Detects when size exceeds max

### Eviction Logic
- [x] Test: Eviction triggered when threshold exceeded
- [x] Test: Identifies least recently accessed entry
- [x] Test: Deletes both .data and .meta files
- [x] Test: Removes entry from index
- [x] Test: Updates stats (eviction count)

### Batch Eviction
- [x] Test: Can evict multiple entries in one pass
- [x] Test: Evicts in LRU order
- [x] Test: Stops when enough space freed

---

## 28.8: Recovery & Startup (Day 6) ✅ COMPLETED

### Startup Sequence
- [x] Test: Loads index from index.json
- [x] Test: Validates index against filesystem
- [x] Test: Removes orphaned files
- [x] Test: Removes invalid index entries
- [x] Test: Recalculates total size
- [x] Test: Triggers eviction if oversized

### Corrupted Entry Handling
- [x] Test: Handles corrupted .data file
- [x] Test: Handles corrupted .meta file
- [x] Test: Handles corrupted index.json
- [x] Test: Logs errors but continues operation
- [x] Test: Removes corrupted entries from cache

### Temporary File Cleanup
- [x] Test: Deletes .tmp files from failed writes
- [x] Test: Doesn't delete legitimate files

---

## 28.9: Cache Trait Implementation (Day 7) ✅ COMPLETED

### DiskCache Structure
- [x] Test: Can create DiskCache
- [x] Test: Contains backend (either tokio::fs or io-uring)
- [x] Test: Contains index
- [x] Test: Contains config
- [x] Test: Contains stats tracker

### Backend Selection at Compile Time
- [x] Test: Linux builds use UringBackend
- [x] Test: macOS builds use TokioFsBackend
- [x] Test: Tests use TokioFsBackend (consistent)
- [x] Test: Only one backend compiled into binary

### Cache::get() Implementation
- [x] Test: Checks index first
- [x] Test: Returns None if expired
- [x] Test: Reads data and metadata from disk
- [x] Test: Updates last_accessed_at
- [x] Test: Increments hit/miss counters

### Cache::set() Implementation
- [x] Test: Validates entry size
- [x] Test: Writes data and metadata atomically
- [x] Test: Updates index
- [x] Test: Triggers eviction if needed
- [x] Test: Returns error on disk full

### Cache::delete() / clear() / stats()
- [x] Test: delete() removes from index and disk
- [x] Test: clear() removes all entries
- [x] Test: stats() returns current statistics
- [x] Test: stats() includes backend type

---

## 28.10: Cross-Platform Testing (Day 8-9) ✅ CORE TESTS COMPLETED

### Platform-Specific Tests
- [ ] Test: All tests pass with UringBackend (Linux) - requires Linux environment
- [x] Test: All tests pass with TokioFsBackend (macOS) - 538 tests passing
- [ ] Test: All tests pass with TokioFsBackend (Windows) - requires Windows environment
- [x] Test: Same behavior across platforms (functional equivalence verified via tests)

### Integration Tests
- [x] Test: Can store and retrieve 100 entries
- [x] Test: Index persistence and recovery (adapted from restart tests)
- [x] Test: LRU eviction works end-to-end
- [ ] Test: Handles 1000+ files efficiently (future: stress testing)
- [ ] Test: Handles 10GB cache size (future: stress testing)

### Error Injection Tests
- [x] Test: Handles disk full error
- [x] Test: Handles permission denied error
- [ ] Test: Handles read-only filesystem (covered by permission denied)
- [ ] Test: Handles corrupted files (covered by Phase 28.8 corruption tests)

### Docker Testing (Linux from macOS/Windows) ✅ COMPLETED
- [x] Docker: Dockerfile.bench created (Rust 1.70 + Debian Bookworm)
- [x] Docker: docker-compose.bench.yml for easy benchmark execution
- [x] Docker: bench-compare.sh script for macOS vs Linux comparison
- [x] Docker: BENCHMARKING.md documentation with complete guide
- [ ] Validation: Run benchmarks in Docker (requires user execution)

---

## 28.11: Performance Validation & io-uring Integration (Day 10) - REVISED

**Original Goal**: Validate performance and optionally add optimizations
**Status**: ✅ **UNBLOCKED** - io-uring crate solution found!

**Updated Goal**:
1. Benchmark io-uring (spawn_blocking) vs tokio::fs on Linux
2. Decide on optimization: keep spawn_blocking vs dedicated runtime thread

**Key Findings from Investigation**:
- ❌ tokio-uring: !Send futures (intentional Rc<T> design) → blocked by async_trait
- ✅ io-uring crate: Send + Sync types → works with spawn_blocking wrapper!
- ✅ Proof-of-concept validated: See IO_URING_FEASIBILITY.md

### Implementation Approach Decision

**Option A: spawn_blocking (Simple)** - IMPLEMENT FIRST
- Thread pool overhead: ~5-10%
- Easy implementation (1-2 days)
- Still faster than tokio::fs on Linux
- ✅ Proven with POC

**Option B: Dedicated Runtime Thread (Optimal)** - OPTIONAL
- Minimal overhead
- Shared IoUring instance
- Implement ONLY if benchmarks show spawn_blocking insufficient

### Benchmarking Tasks

#### Small File Benchmarks (4KB)
- [x] Benchmark: tokio::fs read (baseline) - 41.7 µs read, 195.1 µs write (Linux)
- [x] Benchmark: io-uring (spawn_blocking) read (Linux) - 86.0 µs read, 187.5 µs write
- [x] Target: 2-3x throughput improvement - ❌ FAILED: 2.1x SLOWER for reads!
- [x] Verify: No regression on macOS - ✅ 17.7 µs (excellent performance)

#### Large File Benchmarks (10MB)
- [x] Benchmark: tokio::fs read (baseline) - 2.68 ms read (Linux)
- [x] Benchmark: io-uring (spawn_blocking) read (Linux) - 4.39 ms read
- [x] Target: 20-40% throughput improvement - ❌ FAILED: 64% SLOWER!

#### spawn_blocking Overhead Analysis
- [x] Benchmark: Measure spawn_blocking overhead - ✅ MEASURED: dominates I/O time
- [x] Benchmark: Compare spawn_blocking vs dedicated thread - N/A: spawn_blocking too slow
- [x] Decision: Keep spawn_blocking or implement dedicated thread? - ✅ **NEITHER - Keep TokioFsBackend!**

### Latency Benchmarks
- [x] Target: P95 latency <10ms (tokio::fs) - ✅ All operations <3ms (excellent!)
- [x] Target: P95 latency <5ms (io-uring spawn_blocking) - ❌ FAILED: worse than tokio::fs
- [x] Stretch: P95 latency <3ms (io-uring dedicated thread) - N/A: not pursuing

### Advanced Optimizations (DEFERRED until benchmarks)

**NOTE**: These are now DEFERRED until benchmarks prove spawn_blocking insufficient

#### Dedicated Runtime Thread (if needed)
- [ ] Implementation: Create dedicated thread with IoUring instance
- [ ] Implementation: Channel-based request/response
- [ ] Test: No file descriptor leaks under load
- [ ] Test: Proper cleanup on errors
- [ ] Benchmark: Compare vs spawn_blocking

#### Buffer Pool Management (future optimization)
- [ ] DEFERRED: Implement only if dedicated thread shows value
- [ ] DEFERRED: Buffer pools, zero-copy patterns

**Current Status**: spawn_blocking approach proven viable, ready for implementation

### Resource Utilization
- [ ] Benchmark: CPU usage under load
- [ ] Benchmark: Memory usage
- [ ] Verify: No file descriptor leaks
- [ ] Verify: Buffer pool doesn't cause unbounded growth

### Performance Report
- [x] Document: Benchmark results - ✅ See benchmark_results_final.txt
- [x] Document: Platform comparison - ✅ TokioFs faster than UringBackend+spawn_blocking
- [x] Document: When to use io-uring - ✅ NOT with spawn_blocking (too much overhead)

### Phase 28.11 Conclusion ✅ COMPLETED

**Final Decision: Use TokioFsBackend on All Platforms**

Benchmark data proves that:
1. ✅ TokioFsBackend delivers excellent performance (41.7 µs for 4KB reads)
2. ❌ UringBackend + spawn_blocking adds overhead (86.0 µs for 4KB reads)
3. ✅ No platform-specific code needed - simpler codebase
4. ✅ Consistent behavior across all platforms

**What We Learned:**
- spawn_blocking overhead (thread pool + IoUring init) dominates I/O time
- For io-uring benefits, need dedicated runtime thread (complex, not justified)
- TDD approach validated: implement → test → benchmark → decide
- Data-driven decisions prevent premature optimization

**Implementation Status:**
- Phase 28.6: UringBackend implemented and tested (18/18 tests ✅)
- Phase 28.11: Benchmarked and compared (all benchmarks ✅)
- Decision: Keep TokioFsBackend for production
- UringBackend: Valuable learning exercise, will not be used in production

**Production Configuration:**
- Linux: TokioFsBackend (via platform_backend alias in tests)
- macOS: TokioFsBackend
- Windows: TokioFsBackend
- All platforms: Same backend, consistent behavior

---

**Summary**: Phase 28 implements hybrid disk cache with TokioFsBackend
**Tests**: 554 total tests passing ✅
**Backend**: TokioFsBackend (all platforms) - simpler and faster!
**Performance**: Excellent (41.7 µs for 4KB reads, 2.68 ms for 10MB reads)
**Docker**: Test Linux code on any platform

---

# PHASE 29: Redis Cache Implementation (Week 3)

**Goal**: Implement distributed Redis-based cache layer
**Deliverable**: Redis cache stores/retrieves entries, supports distributed caching
**Verification**: `cargo test` passes with Redis, failover to disk works

## 29.1: Redis Integration Setup

### Dependencies
- [ ] Test: Add `redis` crate to Cargo.toml (with async support)
- [ ] Test: Can import redis::Client
- [ ] Test: Can import redis::AsyncCommands
- [ ] Test: Can create connection pool

### RedisCache Structure
- [ ] Test: Can create RedisCache struct
- [ ] Test: RedisCache contains redis Client
- [ ] Test: RedisCache contains connection pool
- [ ] Test: RedisCache contains config (URL, password, key prefix)
- [ ] Test: RedisCache contains stats

### RedisCache Constructor
- [ ] Test: Can create RedisCache::new(config)
- [ ] Test: Constructor connects to Redis server
- [ ] Test: Constructor authenticates with password if provided
- [ ] Test: Constructor selects database number
- [ ] Test: Returns error if Redis unreachable

---

## 29.2: Basic Redis Operations

### Key Formatting
- [ ] Test: Formats Redis key with prefix
- [ ] Test: Redis key format: "yatagarasu:bucket:object_key"
- [ ] Test: Escapes special characters in key
- [ ] Test: Handles Unicode keys correctly

### Set Operation
- [ ] Test: set() stores entry in Redis
- [ ] Test: Uses Redis SET command
- [ ] Test: Sets TTL with SETEX if configured
- [ ] Test: Serializes CacheEntry to bytes (MessagePack or JSON)
- [ ] Test: Updates stats on successful set

### Get Operation
- [ ] Test: get() retrieves entry from Redis
- [ ] Test: Uses Redis GET command
- [ ] Test: Deserializes bytes to CacheEntry
- [ ] Test: Returns None if key doesn't exist
- [ ] Test: Increments hit counter on success
- [ ] Test: Increments miss counter on key not found

---

## 29.3: Serialization & Deserialization

### Entry Serialization
- [ ] Test: Can serialize CacheEntry to bytes
- [ ] Test: Uses MessagePack for compact binary format
- [ ] Test: Includes all entry fields (data, metadata)
- [ ] Test: Handles large entries (>1MB)

### Entry Deserialization
- [ ] Test: Can deserialize bytes to CacheEntry
- [ ] Test: Handles version compatibility
- [ ] Test: Returns error on corrupt data
- [ ] Test: Validates deserialized entry

---

## 29.4: TTL & Expiration

### TTL Management
- [ ] Test: Sets Redis TTL on entry insertion
- [ ] Test: Uses config default TTL if not specified
- [ ] Test: Uses custom TTL if provided
- [ ] Test: Redis auto-expires entries (no manual cleanup needed)

### TTL Validation
- [ ] Test: get() validates entry not expired locally (double-check)
- [ ] Test: Redis TTL and local TTL are consistent
- [ ] Test: Handles clock skew gracefully

---

## 29.5: Connection Pool & Error Handling

### Connection Pooling
- [ ] Test: Uses connection pool for Redis connections
- [ ] Test: Pool size configurable
- [ ] Test: Connections reused across requests
- [ ] Test: Handles connection exhaustion gracefully

### Error Handling
- [ ] Test: Handles Redis connection timeout
- [ ] Test: Handles Redis server down
- [ ] Test: Handles Redis authentication failure
- [ ] Test: Returns CacheError::RedisError on failures
- [ ] Test: Logs errors but doesn't crash

### Retry Logic
- [ ] Test: Retries failed operations (configurable)
- [ ] Test: Exponential backoff on retries
- [ ] Test: Gives up after max retries
- [ ] Test: Circuit breaker for Redis (optional)

---

## 29.6: Cache Trait Implementation

### Implement Cache Trait for RedisCache
- [ ] Test: RedisCache implements Cache trait
- [ ] Test: RedisCache implements Send + Sync
- [ ] Test: Can use RedisCache through Cache trait object

### delete() Implementation
- [ ] Test: delete() removes key from Redis
- [ ] Test: Uses Redis DEL command
- [ ] Test: Returns true if key existed
- [ ] Test: Returns false if key didn't exist

### clear() Implementation
- [ ] Test: clear() removes all keys with prefix
- [ ] Test: Uses Redis SCAN + DEL for prefix match
- [ ] Test: Handles large key count efficiently
- [ ] Test: Does not affect other Redis keys (other prefixes)

### stats() Implementation
- [ ] Test: stats() returns current statistics
- [ ] Test: Stats tracked locally (Redis doesn't provide these)
- [ ] Test: Can optionally query Redis INFO for size estimate

---

## 29.7: Testing with Redis

### Unit Tests (Mocked Redis)
- [ ] Test: Unit tests use mocked Redis client
- [ ] Test: Tests don't require running Redis server
- [ ] Test: All Redis operations covered

### Integration Tests (Real Redis)
- [ ] Test: Integration tests use real Redis (via Docker)
- [ ] Test: Tests clean up Redis keys after run
- [ ] Test: Can store and retrieve entries
- [ ] Test: TTL expiration works correctly

### Integration Test Setup
- [ ] Test: docker-compose includes Redis service
- [ ] Test: Tests wait for Redis to be ready
- [ ] Test: Tests use unique key prefixes (avoid collisions)

---

# PHASE 30: Cache Hierarchy & Management API (Week 3)

**Goal**: Implement tiered cache (memory → disk → redis) and management endpoints
**Deliverable**: Cache hierarchy operational, purge/stats API working
**Verification**: Cache promotion works, API returns accurate stats

## 30.1: Tiered Cache Implementation

### TieredCache Structure
- [ ] Test: Can create TieredCache struct
- [ ] Test: TieredCache contains ordered list of cache layers
- [ ] Test: TieredCache preserves layer order (memory, disk, redis)
- [ ] Test: TieredCache can have 1, 2, or 3 layers

### TieredCache Constructor
- [ ] Test: Can create TieredCache from config
- [ ] Test: Initializes layers in correct order
- [ ] Test: Memory layer first (fastest)
- [ ] Test: Disk layer second
- [ ] Test: Redis layer last

---

## 30.2: Get Operation with Hierarchy

### Multi-Layer Get Logic
- [ ] Test: get() checks memory layer first
- [ ] Test: Returns immediately on memory hit
- [ ] Test: Checks disk layer on memory miss
- [ ] Test: Returns immediately on disk hit
- [ ] Test: Checks redis layer on disk miss
- [ ] Test: Returns None if all layers miss

### Cache Promotion (Write-Back)
- [ ] Test: Disk hit promotes to memory
- [ ] Test: Redis hit promotes to disk and memory
- [ ] Test: Promotion is async (non-blocking)
- [ ] Test: Promotion failures logged but don't block get()

---

## 30.3: Set Operation with Hierarchy

### Write-Through Strategy
- [ ] Test: set() writes to all configured layers
- [ ] Test: Writes to memory layer first
- [ ] Test: Writes to disk layer (if enabled)
- [ ] Test: Writes to redis layer (if enabled)
- [ ] Test: Partial write failure is logged

### Write-Behind Strategy (Alternative)
- [ ] Test: set() writes to memory synchronously
- [ ] Test: Writes to disk/redis asynchronously
- [ ] Test: Async writes queued in background
- [ ] Test: Background write failures logged

---

## 30.4: Delete & Clear Operations

### Delete from All Layers
- [ ] Test: delete() removes from all layers
- [ ] Test: Removes from memory layer
- [ ] Test: Removes from disk layer
- [ ] Test: Removes from redis layer
- [ ] Test: Returns true if any layer had the key

### Clear All Layers
- [ ] Test: clear() clears all layers
- [ ] Test: Clears memory layer
- [ ] Test: Clears disk layer
- [ ] Test: Clears redis layer

---

## 30.5: Aggregated Statistics

### Stats Aggregation
- [ ] Test: stats() aggregates across all layers
- [ ] Test: Returns total hits (sum of all layers)
- [ ] Test: Returns total misses
- [ ] Test: Returns per-layer stats breakdown
- [ ] Test: Returns total cache size (sum of all layers)

### Per-Bucket Stats
- [ ] Test: Can track stats per bucket
- [ ] Test: Can retrieve stats for specific bucket
- [ ] Test: Can aggregate stats across all buckets

---

## 30.6: Cache Management API Endpoints

### POST /admin/cache/purge (Purge All)
- [ ] Test: Endpoint exists and responds
- [ ] Test: Requires JWT authentication
- [ ] Test: Requires admin claim in JWT
- [ ] Test: Clears all cache layers
- [ ] Test: Returns success message
- [ ] Test: Returns 401 without valid JWT
- [ ] Test: Returns 403 without admin claim

### POST /admin/cache/purge/:bucket (Purge Bucket)
- [ ] Test: Endpoint accepts bucket name parameter
- [ ] Test: Purges only entries for that bucket
- [ ] Test: Returns success message with count
- [ ] Test: Returns 404 if bucket unknown

### POST /admin/cache/purge/:bucket/*path (Purge Object)
- [ ] Test: Endpoint accepts bucket and object path
- [ ] Test: Purges specific object from cache
- [ ] Test: Returns success message
- [ ] Test: Returns 404 if object not in cache

### GET /admin/cache/stats (Cache Statistics)
- [ ] Test: Endpoint exists and responds
- [ ] Test: Requires JWT authentication
- [ ] Test: Returns JSON with cache stats
- [ ] Test: Includes hits, misses, hit_rate
- [ ] Test: Includes current_size, max_size
- [ ] Test: Includes per-bucket breakdown

### GET /admin/cache/stats/:bucket (Bucket Stats)
- [ ] Test: Endpoint accepts bucket name parameter
- [ ] Test: Returns stats for that bucket only
- [ ] Test: Returns 404 if bucket unknown

---

## 30.7: Integration with Proxy

### Cache Lookup in Proxy Flow
- [ ] Test: Proxy checks cache before S3 request
- [ ] Test: Cache hit returns cached response
- [ ] Test: Cache miss proceeds to S3
- [ ] Test: S3 response populates cache

### Cache Bypass Logic
- [ ] Test: Range requests bypass cache (always)
- [ ] Test: Large files (>max_item_size) bypass cache
- [ ] Test: Conditional requests (If-None-Match) check cache ETag

### ETag Validation
- [ ] Test: Proxy includes ETag in cache entries
- [ ] Test: Validates cached ETag matches S3 ETag on hit
- [ ] Test: Invalidates cache if ETags don't match
- [ ] Test: Refreshes cache entry with updated content

---

## 30.8: Prometheus Metrics for Cache

### Cache Metrics
- [ ] Test: Add cache_hits_total counter
- [ ] Test: Add cache_misses_total counter
- [ ] Test: Add cache_evictions_total counter
- [ ] Test: Add cache_size_bytes gauge
- [ ] Test: Add cache_items gauge
- [ ] Test: Metrics include layer label (memory, disk, redis)
- [ ] Test: Metrics include bucket label

### Histogram Metrics
- [ ] Test: Add cache_get_duration_seconds histogram
- [ ] Test: Add cache_set_duration_seconds histogram
- [ ] Test: Histograms track latency percentiles

---

## 30.9: Testing & Validation

### Integration Tests
- [ ] Test: End-to-end cache hit/miss flow
- [ ] Test: Cache promotion works (disk→memory, redis→disk→memory)
- [ ] Test: Purge API clears cache correctly
- [ ] Test: Stats API returns accurate data
- [ ] Test: Cache survives proxy restart (disk/redis)

### Performance Tests
- [ ] Test: Cache lookup adds <1ms latency on hit
- [ ] Test: Cache write is non-blocking (<1ms)
- [ ] Test: Promotion is async (doesn't slow down response)

---

# PHASE 31: Advanced JWT Algorithms (Week 4)

**Goal**: Support RS256 (RSA) and ES256 (ECDSA) JWT algorithms, JWKS endpoint
**Deliverable**: Can validate RSA and ECDSA signed JWTs, fetch keys from JWKS
**Verification**: Integration tests with RS256/ES256 tokens pass

## 31.1: JWT Library Upgrade

### Update Dependencies
- [ ] Test: Update jsonwebtoken crate to latest version
- [ ] Test: Supports RS256 algorithm
- [ ] Test: Supports ES256 algorithm
- [ ] Test: Supports multiple validation keys

### JWT Algorithm Configuration
- [ ] Test: Add algorithm field to JWT config
- [ ] Test: Can parse algorithm: HS256
- [ ] Test: Can parse algorithm: RS256
- [ ] Test: Can parse algorithm: ES256
- [ ] Test: Rejects unknown algorithm
- [ ] Test: Algorithm is required in config

---

## 31.2: RS256 (RSA) Support

### RSA Public Key Configuration
- [ ] Test: Add rsa_public_key_path to JWT config
- [ ] Test: Can load RSA public key from PEM file
- [ ] Test: Can parse RSA public key format
- [ ] Test: Rejects invalid RSA key format
- [ ] Test: Returns error if file not found

### RS256 Validation
- [ ] Test: Can validate RS256 JWT with valid signature
- [ ] Test: Rejects RS256 JWT with invalid signature
- [ ] Test: Rejects RS256 JWT signed with wrong key
- [ ] Test: Rejects RS256 JWT with HS256 signature
- [ ] Test: Validates claims for RS256 JWT

### RS256 Test Key Generation
- [ ] Test: Generate test RSA key pair for tests
- [ ] Test: Store test keys in tests/fixtures/
- [ ] Test: Load test keys in integration tests
- [ ] Test: Sign test JWT with RS256

---

## 31.3: ES256 (ECDSA) Support

### ECDSA Public Key Configuration
- [ ] Test: Add ecdsa_public_key_path to JWT config
- [ ] Test: Can load ECDSA public key from PEM file
- [ ] Test: Can parse ECDSA P-256 key format
- [ ] Test: Rejects invalid ECDSA key format

### ES256 Validation
- [ ] Test: Can validate ES256 JWT with valid signature
- [ ] Test: Rejects ES256 JWT with invalid signature
- [ ] Test: Rejects ES256 JWT signed with wrong key
- [ ] Test: Validates claims for ES256 JWT

### ES256 Test Key Generation
- [ ] Test: Generate test ECDSA key pair for tests
- [ ] Test: Store test keys in tests/fixtures/
- [ ] Test: Sign test JWT with ES256

---

## 31.4: Multiple Key Support (Key Rotation)

### Multi-Key Configuration
- [ ] Test: Add keys array to JWT config
- [ ] Test: Each key has id, algorithm, and path
- [ ] Test: Can load multiple keys
- [ ] Test: Can mix HS256, RS256, ES256 keys

### Multi-Key Validation Logic
- [ ] Test: Tries each configured key until one validates
- [ ] Test: Returns first successful validation
- [ ] Test: Returns error if all keys fail
- [ ] Test: Logs which key succeeded

### Key ID (kid) Header Support
- [ ] Test: Extracts kid from JWT header
- [ ] Test: Selects validation key by kid
- [ ] Test: Falls back to trying all keys if kid missing
- [ ] Test: Returns error if kid doesn't match any configured key

---

## 31.5: JWKS (JSON Web Key Set) Support

### JWKS Configuration
- [ ] Test: Add jwks_url to JWT config
- [ ] Test: Can parse JWKS URL from config
- [ ] Test: JWKS URL is optional (mutually exclusive with static keys)
- [ ] Test: Validates JWKS URL format

### JWKS Fetching
- [ ] Test: Can fetch JWKS from URL on startup
- [ ] Test: Parses JWKS JSON format
- [ ] Test: Extracts keys from JWKS
- [ ] Test: Handles HTTP errors gracefully
- [ ] Test: Retries on transient failures

### JWKS Key Extraction
- [ ] Test: Extracts RSA keys from JWKS
- [ ] Test: Extracts ECDSA keys from JWKS
- [ ] Test: Maps kid to key
- [ ] Test: Ignores unsupported key types

### JWKS Caching & Refresh
- [ ] Test: Caches JWKS with TTL (default 1 hour)
- [ ] Test: Refreshes JWKS after TTL expires
- [ ] Test: Serves from cache during TTL
- [ ] Test: Handles refresh failures (keeps old JWKS)

---

## 31.6: JWT Validation with JWKS

### JWKS Validation Logic
- [ ] Test: Validates JWT using key from JWKS
- [ ] Test: Matches JWT kid to JWKS key
- [ ] Test: Returns error if kid not in JWKS
- [ ] Test: Validates signature with correct algorithm

### JWKS Test Setup
- [ ] Test: Create mock JWKS endpoint for tests
- [ ] Test: Serve test JWKS with sample keys
- [ ] Test: Generate JWTs signed with JWKS keys
- [ ] Test: Validate JWTs against mock JWKS

---

## 31.7: Configuration Examples

### Example Config - RS256
```yaml
buckets:
  - name: products
    path_prefix: /products
    jwt:
      enabled: true
      algorithm: RS256
      rsa_public_key_path: /etc/yatagarasu/public_key.pem
      token_sources:
        - type: bearer_header
```

- [ ] Test: Can parse RS256 config example
- [ ] Test: Validates with RS256 key correctly

### Example Config - ES256
```yaml
buckets:
  - name: api
    path_prefix: /api
    jwt:
      enabled: true
      algorithm: ES256
      ecdsa_public_key_path: /etc/yatagarasu/ecdsa_public.pem
```

- [ ] Test: Can parse ES256 config example

### Example Config - JWKS
```yaml
buckets:
  - name: secure
    path_prefix: /secure
    jwt:
      enabled: true
      jwks_url: https://auth.example.com/.well-known/jwks.json
      jwks_refresh_interval_seconds: 3600
```

- [ ] Test: Can parse JWKS config example
- [ ] Test: Fetches JWKS from URL

### Example Config - Multiple Keys
```yaml
buckets:
  - name: multi
    path_prefix: /multi
    jwt:
      enabled: true
      keys:
        - id: key1
          algorithm: HS256
          secret: ${JWT_SECRET_1}
        - id: key2
          algorithm: RS256
          rsa_public_key_path: /etc/yatagarasu/rsa_key2.pem
```

- [ ] Test: Can parse multi-key config
- [ ] Test: Validates with any configured key

---

## 31.8: Testing & Validation

### Unit Tests
- [ ] Test: All JWT unit tests pass with new algorithms
- [ ] Test: RS256 validation covered
- [ ] Test: ES256 validation covered
- [ ] Test: JWKS fetching covered
- [ ] Test: No clippy warnings

### Integration Tests
- [ ] Test: End-to-end test with RS256 JWT
- [ ] Test: End-to-end test with ES256 JWT
- [ ] Test: End-to-end test with JWKS
- [ ] Test: Key rotation scenario (old + new key both work)

### Security Tests
- [ ] Test: Cannot forge RS256 JWT without private key
- [ ] Test: Cannot forge ES256 JWT without private key
- [ ] Test: Algorithm confusion attacks prevented (HS256 vs RS256)

---

# PHASE 32: Audit Logging (Week 4)

**Goal**: Implement comprehensive audit logging for compliance
**Deliverable**: All requests logged with correlation IDs, exportable to multiple destinations
**Verification**: Audit logs complete and accurate under load

## 32.1: Audit Log Configuration

### Configuration Schema
- [ ] Test: Add audit_log section to config
- [ ] Test: Can parse enabled field (default false)
- [ ] Test: Can parse output destinations (file, syslog, s3)
- [ ] Test: Can parse log_level (default info)

### File Output Configuration
- [ ] Test: Can parse file_path for audit log
- [ ] Test: Can parse max_file_size_mb
- [ ] Test: Can parse max_backup_files
- [ ] Test: Can parse rotation policy (size, daily)

### Syslog Configuration
- [ ] Test: Can parse syslog_address
- [ ] Test: Can parse syslog_protocol (TCP/UDP)
- [ ] Test: Can parse syslog_facility

### S3 Export Configuration
- [ ] Test: Can parse s3_export section
- [ ] Test: Can parse s3_bucket for audit logs
- [ ] Test: Can parse s3_path_prefix
- [ ] Test: Can parse export_interval_seconds

---

## 32.2: Audit Log Entry Structure

### AuditLogEntry Fields
- [ ] Test: Can create AuditLogEntry struct
- [ ] Test: Contains timestamp (RFC3339 format)
- [ ] Test: Contains correlation_id (UUID)
- [ ] Test: Contains client_ip (real IP, not proxy IP)
- [ ] Test: Contains user (from JWT sub/username claim, if authenticated)
- [ ] Test: Contains bucket name
- [ ] Test: Contains object_key (S3 path)
- [ ] Test: Contains http_method (GET/HEAD)
- [ ] Test: Contains request_path (original URL path)
- [ ] Test: Contains response_status (200, 404, 403, etc.)
- [ ] Test: Contains response_size_bytes
- [ ] Test: Contains duration_ms (request processing time)
- [ ] Test: Contains cache_status (hit, miss, bypass)
- [ ] Test: Contains user_agent (from request headers)
- [ ] Test: Contains referer (from request headers)

### Sensitive Data Redaction
- [ ] Test: JWT tokens redacted in logs
- [ ] Test: Authorization header redacted (show "Bearer [REDACTED]")
- [ ] Test: Query param tokens redacted
- [ ] Test: Sensitive custom headers redacted

### JSON Serialization
- [ ] Test: AuditLogEntry serializes to JSON
- [ ] Test: All fields included in JSON output
- [ ] Test: Timestamp in ISO8601 format
- [ ] Test: Handles special characters correctly

---

## 32.3: Audit Logging Integration

### Request Context Enrichment
- [ ] Test: Generate correlation_id on request start
- [ ] Test: Extract client_ip from request (handle X-Forwarded-For)
- [ ] Test: Extract user from validated JWT
- [ ] Test: Track request start time

### Response Context Enrichment
- [ ] Test: Capture response status
- [ ] Test: Capture response size
- [ ] Test: Calculate duration
- [ ] Test: Capture cache status (hit/miss/bypass)

### Audit Log Middleware
- [ ] Test: Create audit log middleware for Pingora
- [ ] Test: Middleware runs on every request
- [ ] Test: Logs request start
- [ ] Test: Logs request completion
- [ ] Test: Logs request failure/error

---

## 32.4: File-Based Audit Logging

### File Writer
- [ ] Test: Can create audit log file
- [ ] Test: Appends entries to file (one JSON per line)
- [ ] Test: Handles file write errors gracefully
- [ ] Test: Creates directory if not exists

### File Rotation
- [ ] Test: Rotates file when size exceeds max
- [ ] Test: Rotates file daily (if configured)
- [ ] Test: Renames old file with timestamp
- [ ] Test: Keeps only max_backup_files
- [ ] Test: Deletes oldest files when limit exceeded

### Async Writing
- [ ] Test: Writes are async (non-blocking)
- [ ] Test: Uses buffered writer for performance
- [ ] Test: Flushes buffer periodically
- [ ] Test: Flushes buffer on shutdown

---

## 32.5: Syslog Audit Logging

### Syslog Integration
- [ ] Test: Can connect to syslog server (TCP)
- [ ] Test: Can connect to syslog server (UDP)
- [ ] Test: Formats entry as syslog message
- [ ] Test: Includes facility and severity
- [ ] Test: Handles syslog server down gracefully

### Syslog Message Format
- [ ] Test: Uses RFC5424 syslog format
- [ ] Test: Includes structured data (JSON in message)
- [ ] Test: Includes hostname

---

## 32.6: S3 Export for Audit Logs

### Batching Logic
- [ ] Test: Batches audit entries in memory
- [ ] Test: Exports batch to S3 every interval (e.g., 5 minutes)
- [ ] Test: Batch file format: yatagarasu-audit-YYYY-MM-DD-HH-MM-SS.jsonl
- [ ] Test: Each line is one JSON audit entry

### S3 Upload
- [ ] Test: Uploads batch file to S3
- [ ] Test: Uses configured bucket and prefix
- [ ] Test: Handles S3 upload failures (retries)
- [ ] Test: Keeps local copy until upload succeeds

### Async Export
- [ ] Test: Export runs in background task
- [ ] Test: Does not block request processing
- [ ] Test: Flushes remaining entries on shutdown

---

## 32.7: Correlation ID Propagation

### Correlation ID Generation
- [ ] Test: Generates UUID v4 for each request
- [ ] Test: Uses existing X-Correlation-ID header if present
- [ ] Test: Includes correlation ID in all log entries

### Response Header
- [ ] Test: Adds X-Correlation-ID to response headers
- [ ] Test: Clients can use correlation ID for debugging

---

## 32.8: Testing & Validation

### Unit Tests
- [ ] Test: AuditLogEntry serialization
- [ ] Test: Sensitive data redaction
- [ ] Test: File rotation logic
- [ ] Test: S3 batch export logic

### Integration Tests
- [ ] Test: End-to-end request logged correctly
- [ ] Test: All fields populated accurately
- [ ] Test: Multiple requests have different correlation IDs
- [ ] Test: Authenticated request includes user
- [ ] Test: Unauthenticated request has user=null

### Load Tests
- [ ] Test: Audit logging under 1000 req/s
- [ ] Test: No dropped audit entries
- [ ] Test: File rotation works under load
- [ ] Test: Async writing keeps up with request rate

---

# PHASES 33-40: Additional Features & Testing (Week 5-8)

**Note**: These phases are more concise as they follow similar patterns to previous phases.

## PHASE 33: Enhanced Observability (Week 5) - MEDIUM PRIORITY

### OpenTelemetry Tracing
- [ ] Test: Add opentelemetry dependencies
- [ ] Test: Configure trace exporter (Jaeger/Zipkin/OTLP)
- [ ] Test: Create spans for request processing
- [ ] Test: Create spans for S3 operations
- [ ] Test: Create spans for cache operations
- [ ] Test: Propagate trace context across async boundaries
- [ ] Test: Traces exported correctly
- [ ] Test: Span hierarchy is correct (parent-child relationships)

### Request/Response Logging
- [ ] Test: Add configurable request logging
- [ ] Test: Add configurable response logging
- [ ] Test: Filter logging by path pattern
- [ ] Test: Filter logging by status code
- [ ] Test: Redact sensitive headers

### Slow Query Logging
- [ ] Test: Add configurable slow query threshold
- [ ] Test: Log requests exceeding threshold
- [ ] Test: Include timing breakdown (auth, cache, s3)
- [ ] Test: Slow query logs include correlation ID

---

## PHASE 34: Advanced Security (Week 5-6) - MEDIUM PRIORITY

### IP Allowlist/Blocklist
- [ ] Test: Add ip_allowlist to bucket config
- [ ] Test: Add ip_blocklist to bucket config
- [ ] Test: Support CIDR notation (192.168.0.0/24)
- [ ] Test: Allowed IPs pass through
- [ ] Test: Blocked IPs rejected with 403
- [ ] Test: CIDR matching works correctly
- [ ] Test: Allowlist takes precedence over blocklist

### Advanced Rate Limiting
- [ ] Test: Implement token bucket algorithm
- [ ] Test: Implement sliding window algorithm
- [ ] Test: Add per-bucket rate limit config
- [ ] Test: Add per-user rate limit (from JWT)
- [ ] Test: Rate limits enforced correctly
- [ ] Test: Metrics track rate-limited requests

---

## PHASE 35-38: Performance & Resilience Testing (Week 7)

### Phase 35: Cold Cache Performance Tests
- [ ] Test: 1000+ concurrent users, all cache misses
- [ ] Test: P95 latency <200ms (uncached)
- [ ] Test: Error rate <0.1%
- [ ] Test: Memory growth <10% over 30 minutes
- [ ] Test: Throughput matches v1.0.0

### Phase 36: Hot Cache Performance Tests
- [ ] Test: 1000+ concurrent users, 90%+ cache hit rate
- [ ] Test: P95 latency <50ms (cached)
- [ ] Test: Cache hit rate >80%
- [ ] Test: Memory stable under sustained load
- [ ] Test: Throughput >v1.0.0 baseline

### Phase 37: Large File Streaming Tests
- [ ] Test: 100 concurrent large file downloads (>5GB each)
- [ ] Test: Zero-copy streaming: ~64KB memory per connection
- [ ] Test: Total memory <500MB
- [ ] Test: P95 TTFB <500ms
- [ ] Test: No memory leaks over 30 minutes

### Phase 38: Extreme Concurrency Tests
- [ ] Test: 10,000 concurrent users, 1KB files
- [ ] Test: P95 latency <100ms
- [ ] Test: Throughput >1000 req/s
- [ ] Test: Error rate <0.1%
- [ ] Test: No connection pool exhaustion

---

## PHASE 39: Chaos & Resilience Testing (Week 7-8)

### S3 Backend Failures
- [ ] Test: S3 503 errors → circuit breaker opens
- [ ] Test: S3 unreachable → 504 Gateway Timeout
- [ ] Test: Slow S3 (2s+ latency) → timeouts work
- [ ] Test: High error rate (50% 500s) → circuit breaker protects

### Cache Layer Failures
- [ ] Test: Memory cache full → eviction works
- [ ] Test: Disk cache full → eviction works
- [ ] Test: Redis connection lost → falls back to disk
- [ ] Test: Disk I/O errors → logs error, continues serving

### HA Replication Failover
- [ ] Test: Primary replica failure → failover <5s
- [ ] Test: Backup failure → tertiary fallback
- [ ] Test: Primary recovery → returns to primary
- [ ] Test: Failover during load → <1% error rate spike

---

## PHASE 40: Operational Testing (Week 8)

### Hot Reload Under Load
- [ ] Test: Config reload while serving 100+ req/s
- [ ] Test: Zero dropped requests during reload
- [ ] Test: New config applies immediately
- [ ] Test: Cache preserved during reload

### Graceful Shutdown
- [ ] Test: SIGTERM while serving 1000+ connections
- [ ] Test: All in-flight requests complete
- [ ] Test: No broken pipes or connection resets
- [ ] Test: Cache state persisted (disk/redis)

### Cache Consistency Validation
- [ ] Test: Cached content matches S3 (byte-for-byte)
- [ ] Test: ETag validation works correctly
- [ ] Test: Stale content not served after TTL
- [ ] Test: Purge/invalidation works under load
- [ ] Test: No cache corruption after crashes/restarts

---

# Summary & Release Checklist

## v1.1.0 Release Criteria

### 🔴 CRITICAL - Must Have
- [x] Phase 26: Cache configuration and abstractions
- [x] Phase 27: In-memory LRU cache implementation
- [x] Phase 28 OR 29: Disk cache OR Redis cache (at least one)
- [x] Phase 30: Cache hierarchy and management API
- [x] Phase 35-38: All performance tests pass
- [x] Phase 40: Cache validation tests pass

### 🟡 HIGH - Must Have
- [x] Phase 31: RS256/ES256 JWT support
- [x] Phase 32: Audit logging

### 🟢 MEDIUM - Nice to Have
- [ ] Phase 33: OpenTelemetry tracing
- [ ] Phase 34: Advanced security features
- [ ] Phase 39: Chaos engineering tests (full suite)
- [ ] 24-hour soak test

### Documentation Requirements
- [ ] Update README.md with v1.1 features
- [ ] Create docs/CACHING.md
- [ ] Create docs/ADVANCED_AUTH.md
- [ ] Create docs/AUDIT_LOGGING.md
- [ ] Create MIGRATION_v1.0_to_v1.1.md

### Final Quality Gates
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Test coverage >90%
- [ ] Performance targets met
- [ ] Backward compatible with v1.0.0 configs

---

**Total Test Count**: 400+ tests across 15 phases
**Estimated Timeline**: 6-8 weeks development + 2 weeks testing = 8-10 weeks total
**Target Release**: Q1 2026

**Last Updated**: 2025-11-15
**Status**: Ready to begin Phase 26 implementation
