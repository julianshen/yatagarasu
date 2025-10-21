# Quick Answer: Cache Management (Purge, Renew, Conditional)

## Your Questions

**Q1: Does it support cache purging (invalidation)?**  
**Q2: Does it support cache renewal (refresh)?**  
**Q3: Does it check Last-Modified / conditional requests?**

---

## Quick Answers

| Feature | v1.0 (Current) | v1.1 (Q4 2025) |
|---------|----------------|----------------|
| **Purging** | ❌ No | ✅ Yes |
| **Renewal** | ⚠️ TTL only | ✅ Yes |
| **Last-Modified** | ⚠️ Forward only | ✅ Yes |
| **304 Not Modified** | ❌ No | ✅ Yes |
| **ETag validation** | ❌ No | ✅ Yes |

---

## 1. Cache Purging (Invalidation)

### v1.0: ❌ NOT Supported

**What you CAN do:**
```yaml
# Option 1: Use TTL-based expiry
cache:
  ttl: 300  # Files expire after 5 minutes
```

```bash
# Option 2: Restart proxy (clears ALL cache)
systemctl restart yatagarasu
```

**What you CANNOT do:**
- ❌ Purge specific file/path
- ❌ Purge by pattern (*.css)
- ❌ Selective invalidation

---

### v1.1: ✅ FULL Support (Planned)

```bash
# Purge specific file
curl -X DELETE http://proxy/admin/cache/purge \
  -H "Authorization: Bearer token" \
  -d '{"bucket": "static", "key": "/css/main.css"}'

# Purge by prefix (recursive)
curl -X DELETE http://proxy/admin/cache/purge \
  -d '{"bucket": "static", "prefix": "/css/", "recursive": true}'

# Purge by pattern
curl -X DELETE http://proxy/admin/cache/purge \
  -d '{"bucket": "static", "pattern": "*.css"}'

# Purge entire bucket
curl -X DELETE http://proxy/admin/cache/purge \
  -d '{"bucket": "static", "all": true}'
```

**Use cases:**
- 🚀 Immediate content updates
- 🔄 Deploy new frontend version
- 🚨 Emergency content removal

---

## 2. Cache Renewal (Refresh)

### v1.0: ⚠️ Partial (TTL-based only)

**What works:**
```yaml
cache:
  ttl: 3600  # Auto-expires after 1 hour
```

**Timeline:**
```
12:00 PM - File cached (TTL=1hr)
12:30 PM - Served from cache ✅
1:01 PM  - Cache expired, refetch from S3 ✅
1:02 PM  - Served from new cache ✅
```

**Limitation:** Cannot force refresh before TTL expires

---

### v1.1: ✅ Manual + Smart Refresh (Planned)

```bash
# Force refresh specific file
curl -X POST http://proxy/admin/cache/refresh \
  -H "Authorization: Bearer token" \
  -d '{"bucket": "static", "key": "/data.json"}'

# Refresh by prefix
curl -X POST http://proxy/admin/cache/refresh \
  -d '{"bucket": "static", "prefix": "/api/", "recursive": true}'

# Conditional refresh (only if changed on S3)
curl -X POST http://proxy/admin/cache/refresh \
  -d '{"bucket": "static", "key": "/data.json", "mode": "conditional"}'
```

**Smart auto-refresh:**
```yaml
cache:
  ttl: 86400  # 24 hours
  auto_refresh:
    enabled: true
    trigger: "ttl_80_percent"  # Check at 19.2 hours
    conditional: true           # Only if S3 changed
```

---

## 3. Last-Modified & Conditional Requests

### v1.0: ⚠️ Partial (Forward only)

**What happens:**
```
S3 → Proxy: Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT
Proxy → Client: Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT ✅

Client → Proxy: If-Modified-Since: Mon, 01 Jan 2024 12:00:00 GMT
Proxy → Client: 200 OK + full body ❌ (ignores conditional header)
```

✅ Headers forwarded  
❌ Not used for validation

---

### v1.1: ✅ Smart Validation (Planned)

**Client-side caching (304 Not Modified):**
```
Client → Proxy: GET /file.txt
                If-Modified-Since: Mon, 01 Jan 2024 12:00:00 GMT

Proxy checks cache:
  - Cached Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT
  - Comparison: SAME ✅

Proxy → Client: 304 Not Modified (no body!)
```

**Benefits:**
- 💾 Saves bandwidth (no body transfer)
- ⚡ Faster response
- 💰 Reduces costs

**Server-side cache revalidation:**
```
Cache entry near expiry (80% TTL elapsed)

Proxy → S3: HEAD /file.txt
            If-None-Match: "abc123"

S3 → Proxy: 304 Not Modified (file unchanged)

Proxy: Extends cache TTL ✅

Client ← Proxy: 200 OK (from fresh cache)
```

**Benefits:**
- 🔄 Cache stays fresh
- 💰 No re-download if unchanged
- 📊 Smart TTL management

---

## Comparison Table

| Feature | v1.0 | v1.1 | Notes |
|---------|------|------|-------|
| **Purging** | | | |
| By key | ❌ | ✅ | Single file |
| By prefix | ❌ | ✅ | Recursive path |
| By pattern | ❌ | ✅ | *.css, *.js |
| Full bucket | ⚠️ Restart | ✅ API | |
| **Renewal** | | | |
| TTL expiry | ✅ | ✅ | Automatic |
| Manual refresh | ❌ | ✅ | Force update |
| Conditional | ❌ | ✅ | If changed only |
| Background | ❌ | ✅ | Don't block requests |
| **Conditional** | | | |
| Last-Modified | ⚠️ Forward | ✅ Validate | |
| ETag | ⚠️ Forward | ✅ Validate | |
| If-Modified-Since | ❌ | ✅ | Client caching |
| If-None-Match | ❌ | ✅ | ETag matching |
| 304 responses | ❌ | ✅ | Bandwidth savings |

---

## Real-World Examples

### Example 1: Deploy New Frontend

**Scenario:** Updated app.js on S3, users stuck with old cached version

**v1.0:**
```bash
# Only option: Restart proxy
systemctl restart yatagarasu

# Or wait for TTL (could be hours!)
```

**v1.1:**
```bash
# Instant purge
curl -X DELETE http://proxy/admin/cache/purge \
  -d '{"bucket": "cdn", "prefix": "/js/", "recursive": true}'

# Users get new version immediately! ⚡
```

---

### Example 2: Content Update

**Scenario:** Updated hero image, want it live now

**v1.0:**
```yaml
# Configure short TTL (compromise)
cache:
  ttl: 300  # 5 min - bad for cache hit rate
```

**v1.1:**
```bash
# Refresh specific file
curl -X POST http://proxy/admin/cache/refresh \
  -d '{"bucket": "images", "key": "/hero.jpg"}'

# New image live in <1 second! 🚀
```

---

### Example 3: Bandwidth Optimization

**Scenario:** Large files repeatedly sent to same clients

**v1.0:**
```
Every request = full file transfer
1000 requests × 1MB = 1GB bandwidth
```

**v1.1:**
```
First request: 200 OK (1MB)
Subsequent: 304 Not Modified (0 bytes!)
1000 requests = 1MB + 999×0 = 1MB bandwidth
Savings: 99.9%! 💰
```

---

## Cost Impact Example

**Scenario:** E-commerce site, 100,000 daily requests

### Without Conditional Requests (v1.0)
```
100,000 requests × 500KB avg = 50GB/day
Bandwidth cost: 50GB × $0.09/GB = $4.50/day
Monthly: $135
```

### With Conditional Requests (v1.1)
```
10,000 cache misses × 500KB = 5GB
90,000 304 responses × 0KB = 0GB
Bandwidth: 5GB/day
Cost: 5GB × $0.09/GB = $0.45/day
Monthly: $13.50

SAVINGS: $121.50/month (90% reduction!) 💰
```

---

## Workarounds for v1.0

### Purge Workaround

```bash
#!/bin/bash
# Manual purge by restarting

echo "Clearing cache..."
systemctl restart yatagarasu

# Wait for startup
sleep 5

# Pre-warm critical files
for file in logo.png main.css app.js; do
  curl -s http://proxy/static/$file > /dev/null
done

echo "Cache cleared and warmed!"
```

### Conditional Request Workaround

```bash
#!/bin/bash
# Client-side conditional logic

URL="http://proxy/file.txt"
CACHE="/tmp/cached-file.txt"
META="/tmp/cached-meta.txt"

if [ -f "$META" ]; then
  ETAG=$(cat "$META")
  RESPONSE=$(curl -sI "$URL" | grep "ETag" | cut -d' ' -f2)
  
  if [ "$ETAG" == "$RESPONSE" ]; then
    echo "Using cached version"
    cat "$CACHE"
  else
    echo "Downloading new version"
    curl -s "$URL" > "$CACHE"
    echo "$RESPONSE" > "$META"
    cat "$CACHE"
  fi
else
  curl -s "$URL" > "$CACHE"
  curl -sI "$URL" | grep "ETag" | cut -d' ' -f2 > "$META"
  cat "$CACHE"
fi
```

---

## Monitoring

### v1.0 Metrics
```prometheus
yatagarasu_cache_hits_total{bucket="static"}
yatagarasu_cache_misses_total{bucket="static"}
yatagarasu_cache_size_bytes{bucket="static"}
```

### v1.1 Additional Metrics (Planned)
```prometheus
yatagarasu_cache_purges_total{bucket="static"}
yatagarasu_cache_refreshes_total{bucket="static"}
yatagarasu_304_responses_total{bucket="static"}
yatagarasu_bytes_saved_304{bucket="static"}
```

---

## Summary

### Current State (v1.0)

| Feature | Status | Workaround |
|---------|--------|------------|
| Purging | ❌ | Restart proxy |
| Renewal | ⚠️ TTL only | Wait or restart |
| Last-Modified | ⚠️ Forward only | Client-side logic |

### Future State (v1.1 - Q4 2025)

| Feature | Status | Benefit |
|---------|--------|---------|
| Purging | ✅ Full API | Instant updates |
| Renewal | ✅ Manual + Auto | Always fresh |
| Conditional | ✅ 304 responses | 90% bandwidth savings |

---

## What to Do Now

### Short Term (v1.0)
1. ✅ Use TTL-based expiry
2. ✅ Restart proxy for full purge
3. ✅ Configure appropriate TTL per content type
4. ✅ Use workaround scripts if needed

### Long Term (v1.1+)
1. ⏳ Wait for v1.1 release (Q4 2025)
2. ⏳ Enable admin API
3. ⏳ Integrate purge into deployment pipeline
4. ⏳ Enable conditional requests for bandwidth savings

---

## Complete Documentation

**[CACHE_MANAGEMENT.md](CACHE_MANAGEMENT.md)** - Full 15KB guide with:
- Detailed API specifications
- Configuration examples
- Use case scenarios
- Cost calculations
- Monitoring setup
- Migration guide v1.0 → v1.1

---

**Bottom Line:**

Cache management in v1.0 is **basic but functional** (TTL-based).  
Full purging, renewal, and conditional requests coming in **v1.1** (Q4 2025).  
Workarounds available for immediate needs.

**ROI when v1.1 releases:** 90% bandwidth savings + instant cache control = 🚀💰
