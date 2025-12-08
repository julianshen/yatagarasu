# PHASE 28: Disk Cache Implementation (REVISED)

**Last Updated**: 2025-11-16
**Status**: Ready for Implementation
**Changes**: Added optional tokio-uring backend (Phase 28.8)

---

## Overview

**Goal**: Implement persistent disk-based cache layer with optional io-uring optimization
**Deliverable**:
- Disk cache stores/retrieves entries, persists across restarts (all platforms)
- Optional high-performance io-uring backend (Linux 5.10+ only)

**Verification**:
- `cargo test` passes on all platforms
- Cache survives process restart
- Optional: io-uring shows 2-3x improvement on Linux

---

## Implementation Strategy

### Two-Phase Approach

**Phase 28.1-28.7**: Core implementation using `tokio::fs`
- ✅ Works on Linux, macOS, Windows
- ✅ Simple, well-tested, maintainable
- ✅ Standard Tokio async APIs
- ✅ **REQUIRED for v1.1.0**

**Phase 28.8**: Optional io-uring backend using `tokio-uring`
- ⚡ 2-3x faster on Linux 5.10+
- 🐧 Linux-only optimization
- 🎯 Feature flag: `--features io-uring`
- 📊 **OPTIONAL for v1.1.0** (nice to have)

**Phase 28.9**: Testing & Validation (renumbered from 28.8)
- ✅ Validates both backends
- 📈 Benchmarks performance comparison

---

# PHASE 28.1-28.7: Core Implementation (tokio::fs)

**NO CHANGES FROM ORIGINAL PLAN** - See plan_v1.1.md lines 650-826

All tests remain the same, using standard `tokio::fs` APIs.

---

# PHASE 28.8: Optional io-uring Backend (NEW)

**Goal**: Add high-performance io-uring backend for Linux production environments
**Deliverable**: Alternative DiskCache implementation using tokio-uring
**Verification**: Same tests pass, 2-3x performance improvement measured

**Note**: This phase is **OPTIONAL** for v1.1.0. Can be deferred to v1.1.1 or v1.2.0.

## 28.8.1: Dependencies & Feature Configuration

### Feature Flag Setup
- [ ] Test: Add `io-uring` feature to Cargo.toml
- [ ] Test: Feature is disabled by default
- [ ] Test: Can build with `--features io-uring`
- [ ] Test: Can build without feature (default)

### Linux-Specific Dependencies
```toml
[target.'cfg(target_os = "linux")'.dependencies]
tokio-uring = { version = "0.4", optional = true }
```

- [ ] Test: Add tokio-uring dependency (Linux-only, optional)
- [ ] Test: Can import tokio_uring::fs when feature enabled
- [ ] Test: Compiles on Linux with feature enabled
- [ ] Test: Compiles on Linux without feature
- [ ] Test: Compiles on macOS (ignores io-uring)

### Version Requirements
- [ ] Test: Document Linux kernel 5.10+ requirement
- [ ] Test: Runtime detection of io-uring support
- [ ] Test: Graceful fallback if kernel too old

---

## 28.8.2: Backend Abstraction Layer

### Module Organization
```
src/cache/disk/
├── mod.rs                 # Public interface
├── tokio_backend.rs       # tokio::fs implementation (default)
├── uring_backend.rs       # tokio-uring implementation (optional)
└── common.rs              # Shared utilities
```

- [ ] Test: Create backend abstraction layer
- [ ] Test: Both backends implement same DiskCacheBackend trait
- [ ] Test: Runtime selects backend based on feature flag
- [ ] Test: Backend selection is compile-time (zero overhead)

### Backend Selection Logic
```rust
#[cfg(all(target_os = "linux", feature = "io-uring"))]
use uring_backend::DiskCacheImpl;

#[cfg(not(all(target_os = "linux", feature = "io-uring")))]
use tokio_backend::DiskCacheImpl;

pub type DiskCache = DiskCacheImpl;
```

- [ ] Test: Correct backend selected based on platform + feature
- [ ] Test: Only one backend compiled into binary
- [ ] Test: No runtime overhead for backend selection

---

## 28.8.3: tokio-uring Backend Implementation

### File Operations with Ownership Transfer

**Key Difference**: tokio-uring requires buffer ownership

```rust
// tokio::fs (borrowed)
let mut buf = vec![0u8; 4096];
file.read(&mut buf).await?;

// tokio-uring (owned)
let buf = vec![0u8; 4096];
let (res, buf) = file.read_at(buf, 0).await;
```

### Read Operations
- [ ] Test: Implement read with ownership-based buffers
- [ ] Test: Pre-allocate buffers before reads
- [ ] Test: Return (Result, buffer) tuples correctly
- [ ] Test: Handle partial reads correctly
- [ ] Test: Reuse buffers when possible (buffer pooling)

### Write Operations
- [ ] Test: Implement write with ownership transfer
- [ ] Test: Atomic writes via temp file + rename
- [ ] Test: Handle write failures without data loss
- [ ] Test: Explicit fsync for durability

### File Management
- [ ] Test: Explicit close() calls for all file handles
- [ ] Test: No reliance on Drop for cleanup
- [ ] Test: Proper error handling on close failures
- [ ] Test: Resource cleanup on panic (via guard)

### Directory Operations
- [ ] Test: Create directories with tokio-uring
- [ ] Test: Scan directories for index recovery
- [ ] Test: Remove files atomically
- [ ] Test: Rename files atomically

---

## 28.8.4: Buffer Management & Pooling

### Buffer Pool Design

**Problem**: tokio-uring consumes buffers, need efficient reuse

```rust
struct BufferPool {
    pool: Vec<Vec<u8>>,
    buffer_size: usize,
}
```

- [ ] Test: Can create BufferPool with configurable size
- [ ] Test: Can acquire buffer from pool
- [ ] Test: Can return buffer to pool
- [ ] Test: Pool has max capacity (prevents unbounded growth)
- [ ] Test: Buffers cleared on return (security)

### Buffer Reuse Strategy
- [ ] Test: Reuse buffers for reads (4KB pool)
- [ ] Test: Reuse buffers for writes (64KB pool)
- [ ] Test: Separate pools for different sizes
- [ ] Test: Pool stats tracked (hits, misses, allocations)

### Memory Safety
- [ ] Test: No buffer aliasing (ownership guarantees)
- [ ] Test: Buffers zeroed before reuse (no data leaks)
- [ ] Test: Pool thread-safe (if needed)

---

## 28.8.5: Cache Trait Implementation (io-uring)

### Implement Cache Trait
- [ ] Test: UringDiskCache implements Cache trait
- [ ] Test: Same trait as TokioDiskCache
- [ ] Test: Can substitute backends without code changes
- [ ] Test: Type system enforces correct usage

### Async Compatibility
- [ ] Test: Compatible with Tokio runtime (via tokio-uring::start)
- [ ] Test: Can spawn io-uring tasks from Tokio context
- [ ] Test: Proper context switching between runtimes

### Error Handling
- [ ] Test: Maps tokio-uring errors to CacheError
- [ ] Test: Handles ENOSYS (io-uring not supported) gracefully
- [ ] Test: Falls back to tokio::fs if io-uring unavailable

---

## 28.8.6: Runtime Integration

### Spawning io-uring Tasks from Tokio

**Challenge**: tokio-uring uses current-thread executor

```rust
// Spawn io-uring task from Tokio context
tokio::task::spawn_blocking(move || {
    tokio_uring::start(async move {
        // io-uring operations here
    })
});
```

- [ ] Test: Can spawn io-uring tasks from Tokio runtime
- [ ] Test: No deadlocks or race conditions
- [ ] Test: Proper error propagation across runtimes
- [ ] Test: Shutdown coordination (both runtimes)

### Concurrency Strategy
- [ ] Test: One io-uring runtime per worker thread
- [ ] Test: Requests distributed across io-uring runtimes
- [ ] Test: Load balancing works correctly
- [ ] Test: No thread starvation

---

## 28.8.7: Performance Optimization

### Zero-Copy Techniques
- [ ] Test: Use registered buffers (io-uring feature)
- [ ] Test: Use fixed files (io-uring optimization)
- [ ] Test: Batch operations when possible
- [ ] Test: Measure zero-copy overhead vs tokio::fs

### I/O Scheduling
- [ ] Test: Optimal queue depth (128-256 entries)
- [ ] Test: Batched submissions reduce syscalls
- [ ] Test: Completions handled efficiently
- [ ] Test: Backpressure under load

### CPU Affinity (Optional)
- [ ] Test: Pin io-uring threads to CPU cores
- [ ] Test: NUMA-aware memory allocation
- [ ] Test: Measure impact on performance

---

## 28.8.8: Configuration

### Runtime Configuration
```yaml
cache:
  enabled: true
  disk:
    enabled: true
    cache_dir: /var/cache/yatagarasu
    max_disk_cache_size_mb: 10240
    # io-uring specific (only when feature enabled)
    io_uring:
      enabled: true  # Auto-detected if not specified
      queue_depth: 256
      use_registered_buffers: true
      use_fixed_files: false
```

- [ ] Test: Can parse io-uring specific config
- [ ] Test: Config ignored if feature disabled
- [ ] Test: Defaults are sensible (queue_depth=128)
- [ ] Test: Validation of io-uring parameters

### Runtime Detection
- [ ] Test: Detect if io-uring available at runtime
- [ ] Test: Fall back to tokio::fs if not available
- [ ] Test: Log which backend is in use
- [ ] Test: Expose backend info via stats API

---

## 28.8.9: Testing & Validation

### Unit Tests
- [ ] Test: All DiskCache tests pass with io-uring backend
- [ ] Test: Ownership-based buffer API works correctly
- [ ] Test: Explicit close() prevents resource leaks
- [ ] Test: No test regressions vs tokio::fs

### Integration Tests
- [ ] Test: Can store and retrieve entries (io-uring)
- [ ] Test: Cache survives restart (io-uring)
- [ ] Test: LRU eviction works (io-uring)
- [ ] Test: Atomic operations maintain consistency

### Platform Tests
- [ ] Test: tokio::fs backend works on macOS
- [ ] Test: tokio::fs backend works on Linux
- [ ] Test: io-uring backend works on Linux 5.10+
- [ ] Test: io-uring disabled on older Linux gracefully

### Error Injection Tests
- [ ] Test: Disk full handling (io-uring)
- [ ] Test: Permission denied handling (io-uring)
- [ ] Test: Kernel doesn't support io-uring (fallback)
- [ ] Test: io-uring queue full (backpressure)

---

# PHASE 28.9: Benchmarking & Performance Validation (RENUMBERED)

**Goal**: Validate performance improvements and correctness of both backends
**Deliverable**: Performance comparison report, regression test suite
**Verification**: io-uring shows measurable improvement, no correctness issues

## 28.9.1: Benchmark Suite

### Small File Operations (4KB)
- [ ] Test: Benchmark tokio::fs read (baseline)
- [ ] Test: Benchmark io-uring read
- [ ] Test: Verify 2-3x throughput improvement
- [ ] Test: Measure latency distribution (P50, P95, P99)

### Large File Operations (10MB)
- [ ] Test: Benchmark tokio::fs read (baseline)
- [ ] Test: Benchmark io-uring read
- [ ] Test: Verify 20-40% throughput improvement
- [ ] Test: Memory usage comparison

### Mixed Workload
- [ ] Test: 70% reads, 30% writes
- [ ] Test: Measure aggregate throughput
- [ ] Test: Compare CPU utilization
- [ ] Test: Compare context switches

### Stress Test
- [ ] Test: 1000+ concurrent operations (tokio::fs)
- [ ] Test: 1000+ concurrent operations (io-uring)
- [ ] Test: No crashes or deadlocks
- [ ] Test: Memory stability under load

---

## 28.9.2: Performance Regression Tests

### Baseline Validation
- [ ] Test: tokio::fs performance not regressed
- [ ] Test: Same throughput as Phase 28.1-28.7
- [ ] Test: No unexpected overhead from abstraction layer

### Feature Flag Overhead
- [ ] Test: Zero overhead when io-uring feature disabled
- [ ] Test: Binary size comparison (with/without feature)
- [ ] Test: Compilation time comparison

---

## 28.9.3: Correctness Validation

### Equivalence Testing
- [ ] Test: Both backends produce identical results
- [ ] Test: Same cache hit/miss behavior
- [ ] Test: Same eviction order (LRU)
- [ ] Test: Same error handling behavior

### Crash Recovery
- [ ] Test: Index recovery works with both backends
- [ ] Test: No data corruption after kill -9
- [ ] Test: Atomic writes properly implemented

### Concurrency Safety
- [ ] Test: No race conditions under concurrent load
- [ ] Test: Stats tracking accurate with both backends
- [ ] Test: No cache inconsistencies

---

## 28.9.4: Documentation & Examples

### User Documentation
- [ ] Document: When to use io-uring feature
- [ ] Document: Performance expectations
- [ ] Document: Platform requirements (Linux 5.10+)
- [ ] Document: Build instructions with feature flag

### Deployment Guide
```bash
# Development (all platforms)
cargo build

# Production (Linux with io-uring)
cargo build --release --features io-uring

# Production (other platforms)
cargo build --release
```

- [ ] Document: Build commands for different scenarios
- [ ] Document: Runtime detection and logging
- [ ] Document: Troubleshooting io-uring issues

### Configuration Examples
- [ ] Example: Basic config (tokio::fs)
- [ ] Example: Optimized config (io-uring)
- [ ] Example: Auto-detection config
- [ ] Example: Fallback configuration

---

## 28.9.5: Performance Report Template

```markdown
# Disk Cache Performance Report

## Test Environment
- **Kernel**: Linux 6.1.0
- **CPU**: AMD EPYC 7763 (64 cores)
- **Disk**: NVMe SSD (Samsung PM9A3)
- **Memory**: 128GB DDR4

## Results

### Small Files (4KB)
| Backend | Throughput | P95 Latency | CPU Usage |
|---------|------------|-------------|-----------|
| tokio::fs | 8,234 ops/s | 450µs | 12% |
| io-uring | 21,567 ops/s | 180µs | 8% |
| **Improvement** | **2.6x** | **2.5x faster** | **33% less** |

### Large Files (10MB)
| Backend | Throughput | P95 Latency | CPU Usage |
|---------|------------|-------------|-----------|
| tokio::fs | 112 files/s | 9.8ms | 18% |
| io-uring | 156 files/s | 7.2ms | 15% |
| **Improvement** | **1.39x** | **1.36x faster** | **17% less** |
```

- [ ] Test: Generate performance report
- [ ] Test: Include all benchmark results
- [ ] Test: Compare resource utilization
- [ ] Test: Include recommendations

---

## Summary

### Phase 28 Deliverables

**REQUIRED** (28.1-28.7):
- ✅ Disk cache implementation (tokio::fs)
- ✅ Works on all platforms
- ✅ All core tests passing
- ✅ Performance: <10ms P95 latency

**OPTIONAL** (28.8):
- ⚡ io-uring backend (Linux 5.10+)
- 📈 2-3x performance improvement
- 🎯 Feature flag enabled
- 🐧 Production optimization

**VALIDATION** (28.9):
- ✅ Both backends tested
- 📊 Performance benchmarks
- 📝 Documentation complete
- ✅ No regressions

### Decision Points

**For v1.1.0 Release**:
1. **Must Have**: Phase 28.1-28.7 (tokio::fs)
2. **Should Have**: Phase 28.8 (io-uring) if time permits
3. **Nice to Have**: Phase 28.9 comprehensive benchmarks

**Recommendation**:
- Implement 28.1-28.7 first (2-3 days)
- Evaluate 28.8 based on schedule (2-3 days)
- Always include 28.9 validation (1 day)

---

**Next Steps**: Begin Phase 28.1 implementation with tokio::fs
**Estimated Time**: 4-7 days (including optional io-uring)
**Priority**: HIGH (Milestone 2: Persistent Cache)
