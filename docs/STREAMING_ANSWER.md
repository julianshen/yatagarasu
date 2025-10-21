# Answer: Yatagarasu Streaming Architecture

## Your Questions Answered

### Q1: If a file is too large (no cache), does the proxy stream the data or write local first?

**Answer: The proxy STREAMS the data directly.** 

Yatagarasu uses a **zero-copy streaming architecture**:
- Data flows: S3 → Proxy → Client (directly)
- **No local disk buffering** at any point
- **Constant memory usage** (~64KB per connection)
- Works for files of ANY size (MB to GB+)

### Q2: How about the caching flow?

**Answer: Caching is selective and asynchronous.**

**Small files (<10MB):**
- Cache MISS: Fetch from S3 → Send to client → Cache async (background)
- Cache HIT: Serve directly from memory cache (<10ms)

**Large files (>10MB):**
- Always streamed from S3
- Never cached (memory efficiency)

## Visual Summary

### Large File (Streaming - No Cache)
```
S3 → Proxy → Client
     ↓
   64KB buffer (constant memory!)
```

### Small File (Cache Miss)
```
S3 → Proxy → Client
     ↓  └→ Cache (async, background)
```

### Small File (Cache Hit)
```
Cache → Proxy → Client
```

## Key Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Large files** | Stream only | Memory efficiency, low latency |
| **Small files** | Cache in memory | Performance, reduced S3 costs |
| **Local disk** | Never used | Simplicity, no cleanup needed |
| **Memory per request** | ~64KB constant | Scalability (1000s of concurrent streams) |
| **Cache writes** | Async/background | Don't block client response |

## Performance Implications

**Streaming (Large Files):**
- ✅ First byte to client: ~500ms (low TTFB)
- ✅ Memory: Constant regardless of file size
- ✅ Can stream 1000+ concurrent GB files
- ⚠️ Every request hits S3 (no cache)

**Caching (Small Files):**
- ✅ Cache hit: <10ms response time
- ✅ Reduced S3 costs (90%+ cache hit rate possible)
- ⚠️ Cache miss adds small latency (~50-200ms)
- ⚠️ Uses RAM for cache storage

## Detailed Documentation

For complete sequence diagrams and implementation details:
- **[STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md)** - Full technical details
- **[QUICK_REFERENCE_STREAMING.md](QUICK_REFERENCE_STREAMING.md)** - Quick ASCII diagrams
- **[spec.md](spec.md)** - Feature specifications with acceptance criteria

## Configuration Example

```yaml
buckets:
  # Videos: Stream everything (too large to cache)
  - name: "videos"
    path_prefix: "/media"
    cache:
      enabled: false
  
  # Assets: Cache small files only
  - name: "assets"
    path_prefix: "/static"
    cache:
      enabled: true
      max_item_size: "10MB"  # Files >10MB streamed
      max_size: "1GB"         # Total cache size
      ttl: 3600               # 1 hour
```

## Why This Architecture?

**Inspired by:** Your previous S3 proxy research emphasizing mmap caching and efficient streaming

**Benefits:**
1. **Memory efficient**: No risk of OOM with large files
2. **Low latency**: Start serving immediately (no buffering delay)
3. **Scalable**: Horizontal scaling (stateless, no shared disk)
4. **Simple**: No disk cleanup, no partial file handling
5. **Cost effective**: Cache reduces S3 GET costs by 80-90%

**Trade-offs:**
- Large files always hit S3 (by design)
- Cache invalidation requires TTL or manual purge
- In-memory cache limited by available RAM

---

This architecture ensures Yatagarasu can handle both high-throughput video streaming AND fast static asset delivery efficiently!
