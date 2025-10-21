# Quick Reference: Yatagarasu Data Flow

## TL;DR: Streaming vs Caching

**Question**: Does the proxy buffer large files to disk before serving?  
**Answer**: **NO** - Zero-copy streaming architecture. Data flows directly from S3 → Proxy → Client.

**Small files (<10MB)**: May be cached in memory (if cache enabled)  
**Large files (>10MB)**: Always streamed, never buffered or cached  
**Memory usage**: Constant ~64KB per connection regardless of file size

---

## 1. Large File Streaming (No Cache)

```
┌────────┐              ┌──────────┐              ┌─────┐
│ Client │              │  Proxy   │              │ S3  │
└───┬────┘              └────┬─────┘              └──┬──┘
    │                        │                       │
    │ GET /video/movie.mp4   │                       │
    │───────────────────────>│                       │
    │                        │                       │
    │                        │ Build S3 request      │
    │                        │ + Sign with SigV4     │
    │                        │                       │
    │                        │ GET movie.mp4         │
    │                        │──────────────────────>│
    │                        │                       │
    │                        │    200 OK             │
    │                        │    Headers            │
    │                        │<──────────────────────│
    │                        │                       │
    │    200 OK              │                       │
    │    Headers             │                       │
    │<───────────────────────│                       │
    │                        │                       │
    │                        │    Chunk 1 (64KB)     │
    │                        │<──────────────────────│
    │    Chunk 1             │  [FLOWS THROUGH]      │
    │<───────────────────────│                       │
    │                        │                       │
    │                        │    Chunk 2 (64KB)     │
    │                        │<──────────────────────│
    │    Chunk 2             │  [FLOWS THROUGH]      │
    │<───────────────────────│                       │
    │                        │                       │
    │         ...            │         ...           │
    │                        │                       │
    │                        │    Chunk N            │
    │                        │<──────────────────────│
    │    Chunk N             │  [FLOWS THROUGH]      │
    │<───────────────────────│                       │
    │                        │                       │
    │                        │    EOF                │
    │    EOF                 │<──────────────────────│
    │<───────────────────────│                       │
    │                        │                       │

⚡ Latency: <500ms to first byte
💾 Memory: ~64KB constant (NOT file size!)
📊 Scalability: Can stream 1000s of concurrent large files
```

---

## 2. Small File with Cache (First Request - Cache Miss)

```
┌────────┐        ┌──────────┐        ┌───────┐        ┌─────┐
│ Client │        │  Proxy   │        │ Cache │        │ S3  │
└───┬────┘        └────┬─────┘        └───┬───┘        └──┬──┘
    │                  │                  │               │
    │ GET /img/logo.png│                  │               │
    │─────────────────>│                  │               │
    │                  │                  │               │
    │                  │ Check cache      │               │
    │                  │─────────────────>│               │
    │                  │                  │               │
    │                  │    MISS          │               │
    │                  │<─────────────────│               │
    │                  │                  │               │
    │                  │ GET logo.png     │               │
    │                  │───────────────────────────────>  │
    │                  │                  │               │
    │                  │ 200 OK + Body (50KB)             │
    │                  │<─────────────────────────────────│
    │                  │                  │               │
    │                  │ [Async] PUT      │               │
    │                  │─────────────────>│               │
    │   200 OK + Body  │                  │               │
    │<─────────────────│  [Background]    │               │
    │                  │                  │               │
    │                  │        Stored ✓  │               │
    │                  │<─────────────────│               │
    │                  │                  │               │

⚡ Latency: S3 latency + small overhead
💾 Memory: File size + 64KB (temporary)
📝 Note: Cache write is ASYNC, doesn't delay response
```

---

## 3. Small File with Cache (Second Request - Cache Hit)

```
┌────────┐        ┌──────────┐        ┌───────┐        ┌─────┐
│ Client │        │  Proxy   │        │ Cache │        │ S3  │
└───┬────┘        └────┬─────┘        └───┬───┘        └──┬──┘
    │                  │                  │               │
    │ GET /img/logo.png│                  │               │
    │─────────────────>│                  │               │
    │                  │                  │               │
    │                  │ Check cache      │               │
    │                  │─────────────────>│               │
    │                  │                  │               │
    │                  │    HIT! + Body   │               │
    │                  │<─────────────────│               │
    │                  │                  │               │
    │   200 OK + Body  │                  │               │
    │<─────────────────│                  │               │
    │                  │                  │               │
    │                  │  [NO S3 REQUEST] │               │
    │                  │                  │               │

⚡ Latency: <10ms (memory speed!)
💾 Memory: From cache (already in RAM)
🎯 Best case: Fastest possible response
```

---

## 4. Client Disconnects During Large File Stream

```
┌────────┐              ┌──────────┐              ┌─────┐
│ Client │              │  Proxy   │              │ S3  │
└───┬────┘              └────┬─────┘              └──┬──┘
    │                        │                       │
    │ GET /large-file.iso    │                       │
    │───────────────────────>│                       │
    │                        │                       │
    │                        │ GET large-file.iso    │
    │                        │──────────────────────>│
    │                        │                       │
    │                        │    Chunk 1            │
    │    Chunk 1             │<──────────────────────│
    │<───────────────────────│                       │
    │                        │                       │
    │                        │    Chunk 2            │
    │    Chunk 2             │<──────────────────────│
    │<───────────────────────│                       │
    │                        │                       │
    │  [CLIENT DISCONNECTS]  │                       │
    X                        │                       │
                             │                       │
                             │ Detect disconnect     │
                             │ Cancel S3 stream      │
                             │──────────────────────>│
                             │                       │
                             │    Stream cancelled   │
                             │<──────────────────────│
                             │                       │
                             │ Cleanup resources     │
                             │                       │

⚡ Response: Immediate cancellation
💰 Cost savings: Stop S3 data transfer immediately
🧹 Cleanup: No orphaned streams or leaked connections
```

---

## Cache Decision Tree

```
                    Incoming GET Request
                            │
                            ▼
                    ┌───────────────┐
                    │ Cache enabled │
                    │  for bucket?  │
                    └───────┬───────┘
                            │
                ┌───────────┼───────────┐
               NO          YES          │
                │           │           │
                ▼           ▼           │
         Stream from    ┌──────────┐   │
         S3 directly    │File size?│   │
                        └─────┬────┘   │
                              │         │
                    ┌─────────┼─────────┐
                 >10MB      <10MB       │
                    │         │         │
                    ▼         ▼         │
             Stream from  Check cache   │
             S3 (too      └────┬────┘   │
             large to          │         │
             cache)        ┌───┴───┐    │
                          HIT    MISS   │
                           │       │    │
                           ▼       ▼    │
                       Serve    Fetch   │
                       from     from    │
                       cache    S3 +    │
                       (<10ms)  cache   │
                                 async  │
                                        │
                           └────────────┘
                                  │
                                  ▼
                          Client receives
                             response
```

---

## Configuration Examples

### Example 1: Stream Everything (No Cache)
```yaml
buckets:
  - name: "videos"
    path_prefix: "/media"
    cache:
      enabled: false  # All files streamed
```
**Result**: All requests streamed from S3, constant ~64KB memory per request

---

### Example 2: Cache Small Files Only
```yaml
buckets:
  - name: "assets"
    path_prefix: "/static"
    cache:
      enabled: true
      ttl: 3600           # 1 hour
      max_size: "1GB"     # Total cache size
      max_item_size: "5MB" # Only files <5MB cached
```
**Result**: 
- Files <5MB: Cached in memory (fast repeat access)
- Files >5MB: Streamed from S3

---

### Example 3: Aggressive Caching (CDN-like)
```yaml
buckets:
  - name: "cdn"
    path_prefix: "/cdn"
    cache:
      enabled: true
      ttl: 86400          # 24 hours
      max_size: "10GB"    # Large cache
      max_item_size: "10MB"
```
**Result**: Hot files cached for 24h, reducing S3 requests by 90%+

---

## Performance Characteristics

| Scenario | TTFB | Memory/Request | S3 Requests |
|----------|------|----------------|-------------|
| Cache Hit (small file) | <10ms | From cache | 0 |
| Cache Miss (small file) | ~200ms | +File size | 1 |
| Stream (large file) | ~500ms | ~64KB | 1 per request |
| Stream + client disconnect | N/A | ~64KB | Cancelled early |

---

## Memory Usage Examples

### Scenario: 1000 concurrent requests

**All streaming (no cache):**
```
1000 requests × 64KB buffer = 64MB total
```

**Mixed (cache enabled, 100MB cache):**
```
1000 requests × 64KB buffer = 64MB
Cache resident memory        = 100MB
Total                        = 164MB
```

**10 concurrent 1GB file streams:**
```
10 requests × 64KB buffer = 640KB total
(NOT 10GB! Memory usage is constant)
```

---

## Key Takeaways

✅ **Streaming is default**: Large files never buffered to disk
✅ **Constant memory**: ~64KB per connection regardless of file size  
✅ **Smart caching**: Small hot files cached automatically (if enabled)
✅ **Fast disconnect**: Client drop cancels S3 stream immediately
✅ **Scalable**: Can handle thousands of concurrent large file transfers
✅ **Efficient**: No local disk I/O, no cleanup jobs, no disk space issues

⚠️ **Trade-offs**:
- Cache misses add latency for small files (one-time cost)
- Large files not cached (by design for memory efficiency)
- Cache invalidation requires TTL expiry or manual purge
