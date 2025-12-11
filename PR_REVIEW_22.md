# PR #22 Review: Async Cache Promotion with `tokio::spawn`

## Overall Assessment: ✅ Approve with Suggestions

The implementation correctly achieves non-blocking cache promotion using `tokio::spawn`. The core approach is sound and will improve response latency when cache hits occur in slower layers.

---

## Inline Comments

### `src/cache/tiered.rs:136-160` - Background Promotion Task

```rust
tokio::spawn(async move {
    for faster_layer in layers_to_promote {
        if let Err(e) = faster_layer.set(...).await {
            tracing::debug!(...);
        }
    }
});
```

**Comment:** Consider adding a timeout to prevent task accumulation if a layer hangs:

```rust
tokio::spawn(async move {
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        for faster_layer in layers_to_promote {
            let _ = faster_layer.set(key_clone.clone(), entry_clone.clone()).await;
        }
    }).await;
});
```

---

### `src/cache/tiered.rs:143` - Logging Level

```rust
tracing::debug!(
    error = %e,
    key = %format!("{}/{}", key_clone.bucket, key_clone.object_key),
    "Background cache promotion failed (non-critical)"
);
```

**Comment:** `debug!` level might be too quiet for production troubleshooting. Consider `tracing::warn!` since promotion failures could indicate infrastructure issues worth monitoring.

---

### `src/cache/tiered.rs:179-238` - Asymmetric Async Behavior

**Comment:** The `get()` promotion is now async, but `set()` still writes synchronously to all layers (lines 206-238). This asymmetry should be documented. If intentional (write-through semantics), add a comment explaining why. If not, consider making `set()` async for consistency.

---

### `src/cache/tiered.rs:877-878` - Test Timing Sensitivity

```rust
// Wait a bit for async promotion to complete
sleep(Duration::from_millis(100)).await;
```

**Comment:** Fixed sleep could be flaky on slow CI. Consider a retry loop:

```rust
let start = std::time::Instant::now();
loop {
    if memory_entries.lock().await.contains_key(&cache_key) {
        break;
    }
    if start.elapsed() > Duration::from_secs(2) {
        panic!("Promotion did not complete in time");
    }
    sleep(Duration::from_millis(10)).await;
}
```

---

## Summary of Recommendations

| Priority | Issue | Recommendation |
|----------|-------|----------------|
| Medium | No timeout on background tasks | Add `tokio::time::timeout` |
| Low | Debug logging too quiet | Consider `warn!` level |
| Low | Asymmetric async behavior | Document or align `set()` |
| Low | Flaky test timing | Use retry loop with deadline |

---

## What's Done Well

1. **Correct `Arc` usage** - Proper shared ownership for background tasks
2. **Fire-and-forget pattern** - Promotion errors don't affect main request
3. **Comprehensive test updates** - All 18 tests updated consistently
4. **Clear commit message** - `[BEHAVIORAL]` prefix follows project conventions

---

**Verdict:** Ship it! The suggestions above are improvements but not blockers.
