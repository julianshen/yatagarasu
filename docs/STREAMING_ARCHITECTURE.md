# Yatagarasu Data Flow Diagrams

This document provides detailed sequence diagrams for how Yatagarasu handles data streaming and caching.

## Architecture Decision: Streaming vs Buffering

**Key Decision**: Yatagarasu uses **zero-copy streaming** whenever possible, never buffering entire files to disk before serving.

**Rationale**:
- **Memory Efficiency**: Large files (GB+) don't consume proxy memory
- **Low Latency**: First bytes reach client immediately (TTFB optimization)
- **Scalability**: Can handle many concurrent large file requests
- **Simplicity**: No local disk management, cleanup, or failure modes

## Scenario 1: Large File Without Cache (Pure Streaming)

```
Client                 Yatagarasu Proxy              S3 Backend
  |                           |                            |
  |  GET /bucket/large.mp4    |                            |
  |-------------------------->|                            |
  |                           |                            |
  |                           | 1. Route to bucket         |
  |                           | 2. Validate JWT (if needed)|
  |                           | 3. Build S3 request        |
  |                           |                            |
  |                           | GET large.mp4 (SigV4)      |
  |                           |--------------------------->|
  |                           |                            |
  |                           |     200 OK                 |
  |                           |     Headers (size, type)   |
  |                           |<---------------------------|
  |                           |                            |
  |     200 OK                |                            |
  |     Headers (forwarded)   |                            |
  |<--------------------------|                            |
  |                           |                            |
  |                           |     Body Chunk 1 (64KB)    |
  |     Body Chunk 1          |<---------------------------|
  |<--------------------------|                            |
  |                           |                            |
  |                           |     Body Chunk 2 (64KB)    |
  |     Body Chunk 2          |<---------------------------|
  |<--------------------------|                            |
  |                           |                            |
  |                           |     Body Chunk N           |
  |     Body Chunk N          |<---------------------------|
  |<--------------------------|                            |
  |                           |                            |
  |                           |     EOF                    |
  |     EOF                   |<---------------------------|
  |<--------------------------|                            |
  |                           |                            |

Key Points:
- No local storage used
- Chunks flow through proxy immediately (pipelining)
- Memory usage: ~64KB buffer per connection
- Client starts receiving data within ~100-500ms
- Proxy memory stays constant regardless of file size
```

## Scenario 2: Small File With Cache Enabled (Cache Miss)

```
Client          Yatagarasu Proxy          Cache Layer         S3 Backend
  |                    |                       |                    |
  |  GET /static/     |                       |                    |
  |   logo.png        |                       |                    |
  |------------------>|                       |                    |
  |                   |                       |                    |
  |                   | 1. Route to bucket    |                    |
  |                   | 2. Validate JWT       |                    |
  |                   | 3. Check cache        |                    |
  |                   |                       |                    |
  |                   | GET logo.png          |                    |
  |                   |---------------------->|                    |
  |                   |                       |                    |
  |                   |     MISS              |                    |
  |                   |<----------------------|                    |
  |                   |                       |                    |
  |                   | GET logo.png (SigV4)  |                    |
  |                   |-------------------------------------->|
  |                   |                       |                    |
  |                   |     200 OK + Body     |                    |
  |                   |<--------------------------------------|
  |                   |                       |                    |
  |                   | PUT logo.png + Body   |                    |
  |                   |---------------------->|                    |
  |                   |                       | Store in cache     |
  |                   |                       | (if size < limit)  |
  |                   |       OK              |                    |
  |                   |<----------------------|                    |
  |                   |                       |                    |
  |     200 OK        |                       |                    |
  |     + Body        |                       |                    |
  |<------------------|                       |                    |
  |                   |                       |                    |

Key Points:
- Small files (<10MB typically) buffered for caching
- Client receives response once S3 responds
- Background cache write (async, doesn't block response)
- Cache key: bucket + S3 key
- Respects cache TTL and max_size per bucket
```

## Scenario 3: Cached File (Cache Hit)

```
Client          Yatagarasu Proxy          Cache Layer         S3 Backend
  |                    |                       |                    |
  |  GET /static/     |                       |                    |
  |   logo.png        |                       |                    |
  |------------------>|                       |                    |
  |                   |                       |                    |
  |                   | 1. Route to bucket    |                    |
  |                   | 2. Validate JWT       |                    |
  |                   | 3. Check cache        |                    |
  |                   |                       |                    |
  |                   | GET logo.png          |                    |
  |                   |---------------------->|                    |
  |                   |                       |                    |
  |                   |       HIT             |                    |
  |                   |       + Body          |                    |
  |                   |       + Metadata      |                    |
  |                   |<----------------------|                    |
  |                   |                       |                    |
  |     200 OK        |                       |                    |
  |     + Body        |                       |                    |
  |     (from cache)  |                       |                    |
  |<------------------|                       |                    |
  |                   |                       |                    |
  |                   |    (No S3 request)    |                    |

Key Points:
- Fastest path: <10ms response time
- No S3 request made
- Validates cache TTL before serving
- Headers preserved from original S3 response
```

## Scenario 4: Large File With Partial Caching (Range Request)

```
Client          Yatagarasu Proxy          Cache Layer         S3 Backend
  |                    |                       |                    |
  |  GET /video.mp4   |                       |                    |
  |  Range: 0-1MB     |                       |                    |
  |------------------>|                       |                    |
  |                   |                       |                    |
  |                   | Check range cache     |                    |
  |                   |---------------------->|                    |
  |                   |                       |                    |
  |                   |     MISS              |                    |
  |                   |<----------------------|                    |
  |                   |                       |                    |
  |                   | GET (Range: 0-1MB)    |                    |
  |                   |-------------------------------------->|
  |                   |                       |                    |
  |                   |     206 Partial       |                    |
  |                   |<--------------------------------------|
  |                   |                       |                    |
  |     206 Partial   |                       |                    |
  |     (streamed)    |                       |                    |
  |<------------------|                       |                    |
  |                   |                       |                    |
  |  GET /video.mp4   |                       |                    |
  |  Range: 1MB-2MB   |                       |                    |
  |------------------>|                       |                    |
  |                   |                       |                    |
  |     [Same flow, streamed from S3]        |                    |

Key Points:
- Range requests always streamed (never fully cached)
- Useful for video seeking
- S3 Range header preserved
- Each range is an independent S3 request
```

## Scenario 5: Client Disconnect During Streaming

```
Client          Yatagarasu Proxy              S3 Backend
  |                    |                            |
  |  GET /large.iso   |                            |
  |------------------>|                            |
  |                   |                            |
  |                   | GET large.iso              |
  |                   |--------------------------->|
  |                   |                            |
  |                   |     Chunk 1                |
  |     Chunk 1       |<---------------------------|
  |<------------------|                            |
  |                   |                            |
  |                   |     Chunk 2                |
  |     Chunk 2       |<---------------------------|
  |<------------------|                            |
  |                   |                            |
  | [CLIENT           |                            |
  |  DISCONNECTS]     |                            |
  X                   |                            |
                      |                            |
                      | Detect disconnect         |
                      | Cancel S3 stream          |
                      |--------------------------->|
                      |                            |
                      |     Stream closed          |
                      |<---------------------------|
                      |                            |
                      | Cleanup connection         |
                      |                            |

Key Points:
- Proxy detects client disconnect immediately
- Cancels S3 request (stops paying for transfer)
- No orphaned streams
- Resources freed immediately
```

## Cache Decision Logic

```
┌─────────────────────────────────────────────┐
│  Incoming Request                           │
└──────────────┬──────────────────────────────┘
               │
               ▼
        ┌──────────────┐
        │ Cache enabled│
        │ for bucket?  │
        └──┬───────┬───┘
           │       │
          No      Yes
           │       │
           │       ▼
           │  ┌──────────────┐
           │  │ Check cache  │
           │  └──┬───────┬───┘
           │     │       │
           │    Hit    Miss
           │     │       │
           │     ▼       ▼
           │  ┌─────┐ ┌──────────────┐
           │  │Serve│ │Request from  │
           │  │from │ │S3 + cache    │
           │  │cache│ │(if cacheable)│
           │  └─────┘ └──────────────┘
           │                │
           ▼                ▼
    ┌──────────────────────────┐
    │ Stream directly from S3  │
    │ (no caching)             │
    └──────────────────────────┘
```

## Cacheable vs Non-Cacheable

**Cacheable** (automatically cached when cache enabled):
- Small files (<10MB configurable threshold)
- Static content (images, CSS, JS)
- Content with low change frequency
- GET requests with 200 responses

**Not Cacheable** (always streamed):
- Large files (>10MB)
- **Range requests (partial content)** - Always fetched from S3, never cached
- HEAD requests (no body to cache)
- Error responses (4xx, 5xx)
- Requests with no-cache headers
- Files exceeding cache max_size

**Why Range Requests Aren't Cached:**
- Partial content difficult to cache efficiently
- Many unique range combinations = poor cache hit rate
- Simple pass-through to S3 is more efficient
- Video seeking patterns are rarely repeated exactly

## Memory Usage Patterns

### Scenario: 1000 concurrent requests, mixed sizes

```
Without Caching (Pure Streaming):
├── 1000 clients × 64KB buffer = 64MB
├── S3 connections: ~10MB
└── Total: ~75MB constant

With Caching (10 hot files, 100MB total):
├── 1000 clients × 64KB buffer = 64MB
├── Cache: 100MB (resident in memory)
├── S3 connections: ~10MB
└── Total: ~175MB constant

Large Files (10 concurrent 1GB streams):
├── 10 clients × 64KB buffer = 640KB
├── S3 connections: ~1MB
└── Total: ~2MB (NOT 10GB!)
```

## Performance Characteristics

| Operation | Latency | Memory | Notes |
|-----------|---------|--------|-------|
| Cache Hit | <10ms | Constant | Fastest path |
| Cache Miss (small) | 50-200ms | +File Size | One-time penalty |
| Large File Stream | <500ms TTFB | Constant (~64KB) | Scales infinitely |
| Range Request | <500ms | Constant (~64KB) | Efficient seeking |

## Configuration Examples

### Aggressive Caching (Static Assets)
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

### No Caching (Large Media Files)
```yaml
buckets:
  - name: "videos"
    path_prefix: "/media"
    cache:
      enabled: false      # Pure streaming
```

### Smart Caching (Mixed Content)
```yaml
buckets:
  - name: "content"
    path_prefix: "/content"
    cache:
      enabled: true
      ttl: 3600           # 1 hour
      max_size: "1GB"     # Moderate cache
      max_item_size: "5MB" # Only cache small files
```

## Implementation Notes

### Streaming Implementation (Rust/Pingora)
```rust
// Pseudo-code for streaming
async fn stream_s3_to_client(s3_response: S3Response, client: ClientStream) {
    // Get S3 response stream
    let mut s3_stream = s3_response.body_stream();
    
    // Stream chunks directly to client
    while let Some(chunk) = s3_stream.next().await {
        client.send_chunk(chunk).await?;
        
        // Check if client disconnected
        if client.is_disconnected() {
            s3_stream.cancel();
            break;
        }
    }
}
```

### Cache-Through Implementation
```rust
async fn cache_through(key: &str, fetch_fn: impl Future<Output = Bytes>) -> Bytes {
    // Check cache first
    if let Some(cached) = cache.get(key).await {
        return cached;
    }
    
    // Fetch from S3
    let data = fetch_fn.await;
    
    // Cache in background (don't block response)
    tokio::spawn(async move {
        cache.put(key, data.clone()).await;
    });
    
    data
}
```

---

## Summary

**Yatagarasu's streaming architecture provides:**

✅ **Constant Memory**: O(1) memory per connection regardless of file size
✅ **Low Latency**: First byte to client in <500ms
✅ **High Throughput**: Can stream GB files to thousands of concurrent clients
✅ **Smart Caching**: Automatic caching of small hot files
✅ **Resource Efficiency**: Cancels S3 streams on client disconnect
✅ **Scalability**: Horizontally scalable (stateless design)

**Trade-offs:**
- ⚠️ Cache misses add latency for small files (one-time cost)
- ⚠️ Large files never cached (by design, memory efficiency)
- ⚠️ Cache invalidation requires manual purge or TTL expiry
