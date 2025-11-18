# Phase 28.11 Linux Testing - Findings

## Date: 2025-11-18

## Objective
Test io-uring backend performance on Linux and validate 2-3x throughput improvements for small files and 20-40% improvements for large files.

## Status: ❌ BLOCKED

## Blocking Issues Discovered

###  1. UringBackend Implementation Has Fundamental Send Trait Issues

**Problem**: The `UringBackend` implementation uses `tokio-uring` which relies on `Rc<T>` internally, making its futures `!Send`. However, the `DiskBackend` trait uses `#[async_trait]` which requires `Send` futures by default.

**Error Messages**:
```
error[E0277]: `Rc<tokio_uring::driver::shared_fd::Inner>` cannot be shared between threads safely
error[E0277]: `*mut ()` cannot be sent between threads safely
error: future cannot be sent between threads safely
```

**Impact**:
- UringBackend cannot compile
- All tests for Phase 28.6 are marked complete `[x]` but never actually ran on Linux
- Cannot run Phase 28.11 benchmarks to validate io-uring performance

### 2. API Incompatibility: statx() Method Missing

**Problem**: The UringBackend implementation calls `file.statx().await` but `tokio-uring` 0.4 doesn't provide this method.

**Available Methods** in tokio-uring 0.4 File:
- `open()`, `create()`, `read_at()`, `write_at()`, `sync_all()`, `sync_data()`, `close()`
- No `statx()` method available

**Workaround**: Use fixed buffer sizes or `tokio::fs::metadata()` for file size.

## Work Completed

### ✅ Created Backend Comparison Benchmark

**File**: `benches/backend_comparison.rs`

**Features**:
- Benchmarks tokio::fs (baseline) for 4KB and 10MB files
- Benchmarks tokio-uring (Linux only) for same file sizes
- Uses proper `tokio_uring::start()` runtime pattern
- Conditionally compiled for Linux vs other platforms

**Status**: Compiles successfully but doesn't execute due to Criterion issue (0 benchmarks registered)

### ⚠️ Temporary Workarounds Applied

To allow the codebase to compile:

1. **Disabled UringBackend** in `src/cache/disk/mod.rs`:
   ```rust
   // TEMP: Disabled uring_backend due to compilation issues
   //#[cfg(target_os = "linux")]
   //mod uring_backend;
   ```

2. **Using TokioFsBackend on all platforms** (including Linux):
   ```rust
   // TEMP: Using tokio_backend on all platforms until uring_backend fixed
   mod tokio_backend;
   use tokio_backend as platform_backend;
   ```

3. **Updated DiskCache** to use TokioFsBackend everywhere:
   ```rust
   // TEMP: Using TokioFsBackend on all platforms until uring_backend Send issues resolved
   let backend: Arc<dyn DiskBackend> = Arc::new(super::tokio_backend::TokioFsBackend::new());
   ```

## Root Cause Analysis

The fundamental issue is an architectural mismatch:

1. **tokio-uring design**: Single-threaded runtime with `!Send` types (uses `Rc` internally)
2. **async_trait requirement**: Generates `Send` futures by default
3. **Cache trait cascade**: If DiskBackend is `?Send`, then Cache must also be `?Send`, cascading through the entire system

## Possible Solutions

### Option 1: Remove Send Requirement (Major Refactor)
- Add `#[async_trait(?Send)]` to `DiskBackend`, `Cache`, and all dependent traits
- **Pros**: Allows tokio-uring to work as-is
- **Cons**: Breaks `Send` requirement throughout the system, may impact proxy usage

### Option 2: Spawn Tasks in Dedicated tokio-uring Runtime
- Create a dedicated thread pool running tokio-uring runtime
- Spawn each I/O operation into this pool, return Send-able futures
- **Pros**: Maintains Send traits, isolates io-uring complexity
- **Cons**: Added complexity, potential performance overhead

### Option 3: Use Different io-uring Library
- Consider `io-uring` crate instead of `tokio-uring`
- Or use `glommio` which has different Send semantics
- **Pros**: May have better trait compatibility
- **Cons**: Different APIs, may have other tradeoffs

### Option 4: Benchmark Without Trait Integration (Current)
- Keep benchmarks as standalone code testing raw tokio-uring
- Don't integrate UringBackend into DiskCache/Cache system
- **Pros**: Can measure performance without fixing architecture
- **Cons**: Can't use io-uring in production without trait integration

## Recommendations

1. **Short term**: Complete Phase 28.11 benchmarks using standalone code (Option 4)
   - Benchmark raw tokio-uring vs tokio::fs file operations
   - Document performance characteristics
   - Decide if io-uring is worth the architectural changes

2. **Medium term**: If benchmarks show significant improvements, implement Option 2
   - Most pragmatic solution that maintains Send traits
   - Isolates complexity to backend layer

3. **Long term**: Consider Option 1 if io-uring becomes critical
   - Requires careful analysis of Send requirements in proxy layer
   - May be unnecessary if Option 2 performance is acceptable

## Next Steps

- [ ] Fix Criterion benchmark registration issue (0 benchmarks shown)
- [ ] Run benchmarks on Linux to measure actual io-uring performance
- [ ] Document benchmark results (even if integration is broken)
- [ ] Decide on architecture fix based on performance data
- [ ] Update plan_v1.1.md with findings
- [ ] Create follow-up phase for UringBackend architectural fix if warranted

## Files Modified

- `benches/backend_comparison.rs` - Created new benchmark (✅ compiles)
- `src/cache/disk/mod.rs` - Temporarily disabled uring_backend (⚠️ workaround)
- `src/cache/disk/disk_cache.rs` - Using TokioFsBackend on Linux (⚠️ workaround)

## Conclusion

Phase 28.11 revealed that **Phase 28.6 (UringBackend) was never actually functional on Linux**. The tests were marked complete but the code has fundamental Send trait incompatibilities that prevent compilation.

Before proceeding with io-uring integration, we need actual performance data to justify the architectural changes required. The benchmark infrastructure is in place but needs debugging to execute.

---

Generated: 2025-11-18
Platform: Linux (Fedora 41, kernel 6.13.9)
Rust: stable
tokio-uring: 0.4.0
