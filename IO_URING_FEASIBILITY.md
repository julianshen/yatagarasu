# io-uring Crate Feasibility Analysis

## Date: 2025-11-18

## Executive Summary

✅ **FEASIBLE**: The low-level `io-uring` crate (v0.7.11) **CAN solve our Send trait issues** that block `tokio-uring` integration.

## Problem Recap

- `tokio-uring` uses `Rc<T>` internally → `!Send` futures
- Our `DiskBackend` trait uses `#[async_trait]` → requires `Send` futures
- Incompatibility blocks UringBackend implementation

## Solution: Use `io-uring` Instead of `tokio-uring`

### Key Differences

| Feature | tokio-uring | io-uring |
|---------|-------------|-----------|
| **Send/Sync** | ❌ !Send (uses Rc) | ✅ Send + Sync |
| **async/await** | ✅ Native async | ❌ Manual (requires wrapper) |
| **API Level** | High-level | Low-level |
| **Runtime** | Integrated tokio-uring runtime | Manual submission/completion queues |
| **Complexity** | Simple | Moderate (needs async wrapper) |

### Proof-of-Concept Results

**Test**: Created async wrapper using `tokio::task::spawn_blocking`

```rust
async fn read_file_uring(path: impl AsRef<Path>) -> io::Result<Bytes> {
    tokio::task::spawn_blocking(move || {
        let mut ring = io_uring::IoUring::new(8)?;
        // ... submit read operation ...
        // ... wait for completion ...
        Ok(Bytes::from(buf))
    }).await?
}
```

**Results**:
- ✅ Compiles successfully
- ✅ Returns `Send` future
- ✅ Works with `#[async_trait]`
- ✅ Successfully reads files
- ✅ No trait system changes needed

### Performance Characteristics

#### spawn_blocking Approach
- **Pros**:
  - Simple to implement
  - Maintains Send traits
  - No architectural changes
  - Works with existing code

- **Cons**:
  - Uses thread pool (context switching overhead)
  - Each operation spawns a blocking task
  - May not achieve full io_uring performance potential

#### Alternative: Dedicated Runtime Thread
- **Pros**:
  - Better performance (persistent thread)
  - Shared IoUring instance
  - Batch submissions possible

- **Cons**:
  - More complex implementation
  - Requires channels for communication
  - Custom async runtime integration

## Implementation Options

### Option A: spawn_blocking (Simple)

```rust
pub struct UringBackend;

impl UringBackend {
    async fn read_file(&self, path: &Path) -> Result<Bytes, Error> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            // io-uring operations here
        }).await?
    }
}
```

**Effort**: Low (1-2 days)
**Performance**: Good (thread pool overhead ~5-10%)
**Complexity**: Low

### Option B: Dedicated Runtime Thread (Optimal)

```rust
pub struct UringBackend {
    tx: mpsc::Sender<UringRequest>,
}

// Background thread running:
loop {
    let request = rx.recv().await;
    // Submit to shared IoUring instance
    // Send result back via oneshot channel
}
```

**Effort**: Medium (3-5 days)
**Performance**: Excellent (minimal overhead)
**Complexity**: Medium

## Recommendation

### Phase 1: Implement Option A (spawn_blocking)

**Why**:
1. Proves io-uring works in our architecture
2. Unblocks Phase 28.6 tests
3. Enables Phase 28.11 benchmarks
4. Simple, low-risk implementation

**Success Criteria**:
- UringBackend compiles on Linux
- All Phase 28.6 tests pass
- Benchmarks show performance improvement vs tokio::fs

### Phase 2: Optimize if Warranted (Option B)

**Decision Point**: After benchmarks
- If spawn_blocking overhead acceptable → keep it
- If performance critical → implement Option B
- Benchmark data drives architectural decision

## Next Steps

1. ✅ Proof-of-concept validates feasibility
2. [ ] Implement UringBackend using Option A (spawn_blocking)
3. [ ] Re-enable all Phase 28.6 tests
4. [ ] Run Phase 28.11 benchmarks on Linux
5. [ ] Measure spawn_blocking overhead
6. [ ] Decide if Option B optimization needed

## Technical Details

### Dependencies

```toml
[target.'cfg(target_os = "linux")'.dependencies]
tokio-uring = "0.5"  # REMOVE - causes !Send issues
io-uring = "0.7"     # ADD - has Send + Sync types
```

### Example Implementation

See `/tmp/test-io-uring/src/main.rs` for working proof-of-concept.

Key points:
- Use `io_uring::IoUring::new(entries)` to create ring
- Use `io_uring::opcode::Read::new()` for read operations
- Submit with `ring.submission().push()` (unsafe)
- Wait with `ring.submit_and_wait(count)`
- Retrieve with `ring.completion().next()`

### Safety Considerations

io-uring operations are `unsafe` because:
- Must ensure file descriptors remain valid
- Must ensure buffers aren't moved/dropped during operation
- Must manually manage operation lifecycle

**Mitigation in spawn_blocking**:
- Entire operation in single blocking task
- No lifetimes crossing await points
- File/buffer owned by blocking task

## Comparison: tokio-uring vs io-uring

### tokio-uring (Current - Blocked)
```rust
// !Send - doesn't work with #[async_trait]
tokio_uring::start(async {
    let file = tokio_uring::fs::File::open(path).await?;
    let (res, buf) = file.read_at(buf, 0).await;
    // ...
})
```

### io-uring + spawn_blocking (Proposed - Works!)
```rust
// Send ✅ - works with #[async_trait]
tokio::task::spawn_blocking(move || {
    let mut ring = io_uring::IoUring::new(8)?;
    let read_op = io_uring::opcode::Read::new(...);
    ring.submission().push(&read_op)?;
    ring.submit_and_wait(1)?;
    // ...
}).await?
```

## Conclusion

**The `io-uring` crate is a viable solution** that:
- ✅ Solves the Send trait blocker
- ✅ Requires no architectural changes
- ✅ Can be implemented quickly (Option A)
- ✅ Can be optimized later if needed (Option B)
- ✅ Proven with working proof-of-concept

This unblocks Phase 28.6 and Phase 28.11 on Linux!

---

Generated: 2025-11-18
Proof-of-concept: `/tmp/test-io-uring/`
Status: ✅ FEASIBLE - Ready for implementation
