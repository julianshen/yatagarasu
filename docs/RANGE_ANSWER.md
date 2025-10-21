# Quick Answer: Range Request Support

## Your Question

**Q: Does Yatagarasu support "range"? Under what conditions?**

---

## Answer

**YES! ✅ Full HTTP Range request support**

### Supported Range Types

✅ **Single range**: `Range: bytes=0-1023`  
✅ **Open-ended**: `Range: bytes=1000-` (from byte 1000 to end)  
✅ **Suffix range**: `Range: bytes=-1024` (last 1024 bytes)  
✅ **Multiple ranges**: `Range: bytes=0-1023,2048-3071`

### Conditions

**Always Enabled:**
- ✅ No configuration needed
- ✅ Works on all buckets (public and private)
- ✅ JWT authentication still enforced if configured
- ✅ Always streamed from S3 (never cached)

**Behavior:**
- 📡 Proxy forwards Range header to S3
- 📦 S3 returns 206 Partial Content
- 🚀 Only requested bytes transferred (bandwidth savings!)
- 💾 Memory: Constant ~64KB buffer (not file size)

---

## Use Cases

### 1. Video Seeking 🎬
```bash
# Jump to middle of 1GB movie
curl -H "Range: bytes=500000000-600000000" \
  http://proxy/videos/movie.mp4

# Result: Only 100MB transferred, not 1GB!
```

### 2. Resume Downloads 📥
```bash
# Resume from byte 50MB
curl -H "Range: bytes=52428800-" \
  http://proxy/downloads/linux.iso

# Result: Only downloads remaining bytes
```

### 3. PDF Preview 📄
```bash
# Get first page only
curl -H "Range: bytes=0-102400" \
  http://proxy/docs/manual.pdf

# Result: Quick preview without full download
```

---

## Quick Example

```
Client Request:
  GET /media/video.mp4
  Range: bytes=1000000-2000000

Yatagarasu:
  1. Authenticate (if required)
  2. Forward to S3 with signature
  3. Stream response to client
  4. Skip cache (range = no cache)

S3 Response:
  206 Partial Content
  Content-Range: bytes 1000000-2000000/1048576000
  (Only 1MB transferred, not 1GB!)

Client Receives:
  206 Partial Content
  Exactly 1MB of data
```

---

## Performance Impact

**Bandwidth Savings Example:**

| Scenario | Without Range | With Range | Savings |
|----------|---------------|------------|---------|
| Video seeking (1000 users) | 1TB | 50GB | **95%** |
| PDF preview | 10MB | 100KB | **99%** |
| Resume download | Full file | Remaining | **Variable** |

**Cost Savings:**
- S3 transfer: Only pay for bytes actually transferred
- Bandwidth: 95% reduction in video streaming scenarios
- User experience: Instant seeking, no buffering

---

## Key Points

✅ **Full Range support** - All range types (single, multiple, suffix, open-ended)  
✅ **Always enabled** - No configuration required  
✅ **Bandwidth efficient** - Only transfer requested bytes  
✅ **Memory efficient** - Constant ~64KB buffer per request  
✅ **Cache aware** - Range requests always bypass cache  
✅ **Auth compatible** - Works with JWT authentication  
✅ **Error handling** - 416 Range Not Satisfiable for invalid ranges  

❌ **Not cached** - Range requests never cached (by design)  
❌ **Per-request to S3** - Each range request hits S3  

---

## Documentation

For complete details, see:
- **[RANGE_REQUESTS.md](RANGE_REQUESTS.md)** - Complete guide with examples, sequence diagrams, edge cases
- **[STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md)** - How ranges fit into streaming architecture
- **[spec.md](spec.md)** - Acceptance criteria (8 test cases for range support)
- **[plan.md](plan.md)** - Implementation tests (30+ range-related tests)

---

## Example with Authentication

```bash
# Private bucket with JWT
curl -H "Authorization: Bearer eyJhbGc..." \
     -H "Range: bytes=0-1048576" \
  http://proxy/private/video.mp4

# Response:
# 206 Partial Content (if auth succeeds)
# 401 Unauthorized (if auth fails, range not processed)
```

---

## Summary

Range requests are **fully supported**, **always enabled**, and work **exactly as expected** with HTTP standards. Perfect for video streaming, large file downloads, and any scenario where you need partial content delivery!

**Bandwidth Savings**: Up to 95% in video streaming scenarios  
**Memory Usage**: Constant ~64KB regardless of range size  
**User Experience**: Instant seeking, fast previews, resume downloads
