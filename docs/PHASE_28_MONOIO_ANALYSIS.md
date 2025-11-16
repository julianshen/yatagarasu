# Phase 28: Monoio Research & Disk Cache Strategy Analysis

**Date**: 2025-11-16
**Status**: Research Complete - Recommendations Ready
**Decision Required**: Choose disk cache implementation approach

---

## Executive Summary

**TL;DR**: Monoio is NOT compatible with Yatagarasu's Pingora/Tokio architecture. However, we can achieve similar performance benefits using **tokio-uring** as an optional enhancement to the standard tokio::fs implementation.

**Recommendation**: Implement a hybrid approach with:
1. **Base implementation**: tokio::fs (portable, simple, works everywhere)
2. **Optional enhancement**: tokio-uring (io-uring performance on Linux 5.10+)
3. **Feature flag**: `--features io-uring` to enable io-uring backend

---

## Monoio Deep Dive

### What is Monoio?

Monoio is a **thread-per-core async runtime** built by ByteDance that provides:
- Direct io-uring integration on Linux 5.6+
- Zero-copy I/O operations
- Thread-local task execution (no Send/Sync requirements)
- Fallback to epoll on older Linux, kqueue on macOS

### Key Advantages

1. **Performance**: Direct io-uring access, avoiding Tokio's overhead
2. **Zero-copy**: Buffer ownership model enables true zero-copy I/O
3. **Predictability**: Thread-per-core eliminates work-stealing unpredictability
4. **Simplicity**: No need for Send/Sync bounds on application data

### Critical Limitations

1. **🚨 INCOMPATIBLE WITH TOKIO**: Monoio is a separate runtime, cannot mix with Tokio
2. **🚨 INCOMPATIBLE WITH PINGORA**: Pingora is built on Tokio, requires Tokio runtime
3. **Linux-focused**: Best performance only on Linux 5.6+
4. **Unbalanced workloads**: May underutilize CPU vs. work-stealing runtimes
5. **Ecosystem isolation**: Cannot use Tokio-based libraries

---

## Compatibility Analysis

### Current Yatagarasu Architecture

```
┌─────────────────────────────────────────────┐
│ Pingora HTTP Server (Tokio-based)          │
│  ├─ pingora-core (requires Tokio)          │
│  ├─ pingora-proxy (requires Tokio)         │
│  └─ pingora-http (requires Tokio)          │
└─────────────────────────────────────────────┘
         ↓
┌─────────────────────────────────────────────┐
│ Tokio Runtime (v1.35)                       │
│  ├─ Used by AWS SDK                         │
│  ├─ Used by moka cache                      │
│  └─ Used throughout codebase                │
└─────────────────────────────────────────────┘
```

**Verdict**: Cannot replace Tokio without rewriting the entire application.

### Why Monoio Won't Work

```rust
// Current architecture (Pingora on Tokio)
#[tokio::main]
async fn main() {
    let server = pingora::Server::new(...);  // Requires Tokio runtime
    server.bootstrap();  // Spawns Tokio tasks
}

// Monoio architecture (incompatible)
#[monoio::main]  // ❌ Different runtime, can't coexist
async fn main() {
    let server = pingora::Server::new(...);  // ❌ Won't work without Tokio
}
```

**The Problem**:
- Pingora spawns Tokio tasks internally
- Monoio tasks are `!Send` (thread-local only)
- Cannot bridge between Monoio and Tokio contexts without complex IPC

---

## Alternative: tokio-uring

### What is tokio-uring?

A **bridge between Tokio and io-uring** that provides:
- io-uring performance within Tokio ecosystem
- Ownership-based buffer API (like Monoio)
- Linux 5.10+ requirement
- Integration with existing Tokio code

### Key Differences from Standard Tokio

| Feature | tokio::fs | tokio-uring | Monoio |
|---------|-----------|-------------|---------|
| **Runtime** | Tokio (multi-threaded) | Tokio (current-thread) | Monoio (thread-per-core) |
| **I/O Backend** | epoll | io-uring | io-uring |
| **API Style** | Borrowed buffers | Owned buffers | Owned buffers |
| **Pingora Compatible** | ✅ Yes | ✅ Yes | ❌ No |
| **Cross-platform** | ✅ Yes | ❌ Linux 5.10+ only | ❌ Linux-focused |
| **Performance** | Good | Excellent | Excellent |
| **Complexity** | Low | Medium | High |

### tokio-uring API Example

```rust
use tokio_uring::fs::File;

// Ownership-based API (buffer passed to kernel)
let file = File::create("test.txt").await?;
let buf = vec![0u8; 4096];

// Returns (result, buffer) tuple
let (res, buf) = file.write_at(buf, 0).await;

// Must explicitly close (no async Drop)
file.close().await?;
```

### tokio-uring Limitations

1. **Explicit close required**: No async Drop, must call `close()`
2. **Current-thread executor**: Single-threaded, need multiple runtimes for concurrency
3. **Ownership transfer**: Different API from standard Tokio
4. **Linux-only**: No fallback for other platforms (need conditional compilation)

---

## Recommended Approach: Hybrid Implementation

### Strategy: Portable Base + Optional io-uring

Implement **two backend implementations** with feature flags:

```toml
# Cargo.toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }

[target.'cfg(target_os = "linux")'.dependencies]
tokio-uring = { version = "0.4", optional = true }

[features]
default = []
io-uring = ["tokio-uring"]
```

### Architecture

```rust
// src/cache/disk/mod.rs

#[cfg(feature = "io-uring")]
mod uring_backend;

#[cfg(not(feature = "io-uring"))]
mod tokio_backend;

// Runtime selects implementation
pub use self::backend::DiskCache;

#[cfg(feature = "io-uring")]
use uring_backend as backend;

#[cfg(not(feature = "io-uring"))]
use tokio_backend as backend;
```

### Implementation Plan

**Phase 28.1-28.7: Standard tokio::fs Implementation** (All platforms)
- Use `tokio::fs` for file operations
- Async file I/O with standard Tokio APIs
- Works on Linux, macOS, Windows
- Simple, well-tested, broadly compatible

**Phase 28.8: Optional tokio-uring Backend** (Linux 5.10+ only)
- Alternative implementation using tokio-uring
- Same trait interface (`Cache` trait)
- Enabled with `--features io-uring`
- Better performance on modern Linux

**Phase 28.9: Benchmarking & Validation**
- Compare tokio::fs vs tokio-uring performance
- Measure: latency, throughput, CPU usage
- Validate: same correctness guarantees

---

## Performance Expectations

### tokio::fs (Baseline)

| Operation | Latency | Throughput |
|-----------|---------|------------|
| **Small file read (4KB)** | ~500µs | ~8K ops/sec |
| **Large file read (10MB)** | ~10ms | ~100 files/sec |
| **Write + fsync** | ~5ms | ~200 ops/sec |

### tokio-uring (io-uring optimized)

| Operation | Latency | Throughput |
|-----------|---------|------------|
| **Small file read (4KB)** | ~200µs | ~20K ops/sec |
| **Large file read (10MB)** | ~8ms | ~125 files/sec |
| **Write + fsync** | ~3ms | ~333 ops/sec |

**Expected improvement**: 2-3x throughput for small files, 20-40% for large files

---

## Revised Phase 28 Plan

### Phase 28.1-28.7: Core Implementation (tokio::fs)

**NO CHANGES** - Implement as planned with tokio::fs

✅ All tests use standard tokio::fs
✅ Works on all platforms
✅ Simple, maintainable, well-documented

### Phase 28.8: Optional io-uring Backend (NEW)

**Add optional tokio-uring implementation**

#### 28.8.1: Dependencies & Feature Flags
- [ ] Test: Add tokio-uring to Linux-only dependencies
- [ ] Test: Add io-uring feature flag
- [ ] Test: Conditional compilation works correctly

#### 28.8.2: io-uring Backend Structure
- [ ] Test: Create `uring_backend` module
- [ ] Test: Implement Cache trait with tokio-uring
- [ ] Test: Handle ownership-based buffer API
- [ ] Test: Explicit close() on all file handles

#### 28.8.3: Performance Comparison Tests
- [ ] Test: Benchmark tokio::fs vs tokio-uring (small files)
- [ ] Test: Benchmark tokio::fs vs tokio-uring (large files)
- [ ] Test: Verify 2-3x improvement on io-uring
- [ ] Test: CPU usage comparison

#### 28.8.4: Runtime Selection
- [ ] Test: Runtime selects correct backend based on feature
- [ ] Test: Can compile without io-uring feature
- [ ] Test: Can compile with io-uring feature
- [ ] Test: Runtime behavior identical (trait interface)

#### 28.8.5: Documentation
- [ ] Document: When to use io-uring feature
- [ ] Document: Performance tradeoffs
- [ ] Document: Linux kernel version requirements
- [ ] Document: Build/deployment with io-uring

### Phase 28.9: Validation (UPDATED)

- [ ] Test: Both backends pass all Phase 28.1-28.7 tests
- [ ] Test: Feature flag switching works correctly
- [ ] Test: No performance regression on tokio::fs path
- [ ] Test: io-uring shows measurable improvement
- [ ] Test: Production config examples for both backends

---

## Decision Matrix

| Approach | Complexity | Performance | Portability | Pingora Compatible | Recommendation |
|----------|------------|-------------|-------------|-------------------|----------------|
| **Monoio** | Very High | Excellent | Poor | ❌ No | ❌ Reject |
| **tokio-uring only** | Medium | Excellent | Poor | ✅ Yes | ⚠️ Too limiting |
| **tokio::fs only** | Low | Good | Excellent | ✅ Yes | ✅ Safe default |
| **Hybrid (tokio::fs + tokio-uring)** | Medium | Excellent* | Excellent | ✅ Yes | ✅✅ **BEST** |

\* Excellent performance when io-uring enabled, good performance on all platforms

---

## FAQ

### Q: Why not just use Monoio everywhere?
**A**: Monoio is incompatible with Pingora. Switching would require rewriting the entire proxy layer, AWS SDK integration, and all async code. Estimated effort: 6-12 months. Not feasible.

### Q: Can we run Monoio in a separate process?
**A**: Possible but adds massive complexity (IPC, serialization, separate process management). The performance benefit doesn't justify the operational overhead.

### Q: Why not use glommio instead of Monoio?
**A**: Same problem - glommio is also a thread-per-core runtime incompatible with Tokio.

### Q: What about async-io or smol?
**A**: These are also separate runtimes. Pingora requires Tokio specifically.

### Q: Is tokio-uring production-ready?
**A**: Yes, used in production by companies like Materialize. Version 0.4+ is stable. However, it's Linux-only.

### Q: What if we want to support macOS for development?
**A**: Use the default tokio::fs backend. Only enable io-uring for Linux production deployments.

---

## Recommendations

### For v1.1.0 Release

**REQUIRED**:
- ✅ Implement Phase 28.1-28.7 with tokio::fs (portable, simple)

**OPTIONAL** (Nice to have):
- 🎯 Implement Phase 28.8 with tokio-uring backend (Linux optimization)

**REJECTED**:
- ❌ Do NOT attempt Monoio integration (incompatible with Pingora)

### For Future Versions (v2.0+)

**If performance is critical**, consider:
1. Evaluate io-uring integration more deeply
2. Contribute to Pingora for native io-uring support
3. Monitor Pingora roadmap for runtime improvements

**Do NOT**:
- Attempt to replace Pingora with custom Monoio-based proxy
- Fork Pingora to retrofit Monoio support
- Maintain two separate runtime branches

---

## Conclusion

**Monoio is an excellent runtime**, but it's fundamentally incompatible with Yatagarasu's architecture. The pragmatic approach is:

1. **Implement Phase 28 as planned** using tokio::fs
2. **Add optional tokio-uring** support for Linux production environments
3. **Use feature flags** to maintain portability

This gives us:
- ✅ Portability (works everywhere)
- ✅ Performance (io-uring when available)
- ✅ Simplicity (standard Tokio APIs as default)
- ✅ Future-proofing (can optimize further as needed)

**Next Steps**: Proceed with Phase 28.1 using tokio::fs. Evaluate adding Phase 28.8 (tokio-uring) after core implementation is complete and tested.

---

**Prepared by**: Claude Code
**Reviewed**: Pending
**Approved**: Pending
**Implementation Start**: TBD
