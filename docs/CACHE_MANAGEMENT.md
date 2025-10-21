# Cache Management: Purging, Renewal, and Conditional Requests

## Quick Answers

### Q1: Does it support cache purging (invalidation)?
**v1.0:** ❌ **NO - Not implemented**  
**v1.1:** ✅ **YES - Planned feature**

### Q2: Does it support cache renewal (refresh)?
**v1.0:** ⚠️ **Partial - TTL-based expiry only**  
**v1.1:** ✅ **YES - Manual refresh API**

### Q3: Does it check Last-Modified / conditional requests?
**v1.0:** ⚠️ **Partial - Forwards headers but doesn't validate cache**  
**v1.1:** ✅ **YES - Smart cache revalidation**

---

## Detailed Breakdown

## 1. Cache Purging (Invalidation)

### v1.0 Status: ❌ Not Supported

**What works:**
- ✅ TTL-based expiry (cache entries expire automatically)
- ✅ LRU eviction (when cache is full, oldest entries removed)
- ✅ Restart proxy to clear all cache (nuclear option)

**What doesn't work:**
- ❌ Selective purge by path/pattern
- ❌ API to invalidate specific files
- ❌ Purge by tag or metadata
- ❌ Bulk purge operations

**Workarounds:**
```bash
# Option 1: Restart proxy (clears all cache)
systemctl restart yatagarasu

# Option 2: Wait for TTL expiry
# Configure short TTL for frequently changing content
cache:
  ttl: 300  # 5 minutes - cache expires quickly

# Option 3: Deploy new version to different path
# /v1/assets/logo.png → /v2/assets/logo.png
```

---

### v1.1 Planned: ✅ Full Purge Support

#### Configuration

```yaml
cache:
  enabled: true
  admin_api:
    enabled: true
    auth_token: "${ADMIN_TOKEN}"  # Secure admin operations
```

#### API Endpoints

```bash
# 1. Purge specific file
DELETE /admin/cache/purge
Content-Type: application/json
Authorization: Bearer admin-token

{
  "bucket": "static-assets",
  "key": "/css/main.css"
}

# Response:
{
  "status": "purged",
  "key": "/css/main.css",
  "bucket": "static-assets"
}

# 2. Purge by prefix (recursive)
DELETE /admin/cache/purge
{
  "bucket": "static-assets",
  "prefix": "/css/",
  "recursive": true
}

# Response:
{
  "status": "purged",
  "prefix": "/css/",
  "files_purged": 45,
  "bytes_freed": 5242880
}

# 3. Purge by pattern
DELETE /admin/cache/purge
{
  "bucket": "static-assets",
  "pattern": "*.css",
  "recursive": true
}

# Response:
{
  "status": "purged",
  "pattern": "*.css",
  "files_purged": 23
}

# 4. Purge entire bucket cache
DELETE /admin/cache/purge
{
  "bucket": "static-assets",
  "all": true
}

# Response:
{
  "status": "purged",
  "bucket": "static-assets",
  "files_purged": 10000,
  "bytes_freed": 1073741824
}

# 5. Get cache stats before purging
GET /admin/cache/stats?bucket=static-assets

# Response:
{
  "bucket": "static-assets",
  "entries": 10000,
  "size_bytes": 1073741824,
  "hit_rate": 0.95,
  "hits": 1000000,
  "misses": 50000
}
```

#### Use Cases

**1. Content Update**
```bash
# Updated logo.png on S3 - purge old version
curl -X DELETE http://proxy/admin/cache/purge \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"bucket": "cdn", "key": "/images/logo.png"}'
```

**2. Deploy New CSS Version**
```bash
# Purge all CSS files
curl -X DELETE http://proxy/admin/cache/purge \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"bucket": "static", "prefix": "/css/", "recursive": true}'
```

**3. Emergency Content Removal**
```bash
# Immediately remove cached content
curl -X DELETE http://proxy/admin/cache/purge \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"bucket": "cdn", "key": "/sensitive/data.pdf"}'
```

---

## 2. Cache Renewal (Refresh)

### v1.0 Status: ⚠️ Partial (TTL-based only)

**Automatic Renewal:**
```yaml
cache:
  ttl: 3600  # Files expire after 1 hour
```

**How it works:**
```
1. File cached at 12:00 PM with TTL=3600
2. Requests at 12:30 PM → Served from cache
3. Requests at 1:01 PM (>1hr) → Cache expired
4. Fetch from S3 → Cache renewed
5. Subsequent requests → Served from new cache
```

**Limitations:**
- ❌ Cannot manually refresh specific file
- ❌ Cannot force refresh while TTL valid
- ❌ All files use same TTL

**Workaround:**
```bash
# Use purge + request to force refresh
# 1. Clear cache (restart proxy)
systemctl restart yatagarasu

# 2. Request file (will fetch from S3 and re-cache)
curl http://proxy/static/main.css
```

---

### v1.1 Planned: ✅ Manual Refresh

#### API

```bash
# 1. Refresh specific file
POST /admin/cache/refresh
Authorization: Bearer admin-token
{
  "bucket": "static",
  "key": "/css/main.css"
}

# Response:
{
  "status": "refreshed",
  "key": "/css/main.css",
  "size_bytes": 52428,
  "cached_at": "2025-10-21T10:30:00Z"
}

# 2. Refresh by prefix
POST /admin/cache/refresh
{
  "bucket": "static",
  "prefix": "/css/",
  "recursive": true
}

# Response:
{
  "status": "refreshing",
  "task_id": "refresh-123",
  "files_queued": 45
}

# 3. Soft vs Hard Refresh
POST /admin/cache/refresh
{
  "bucket": "static",
  "key": "/data.json",
  "mode": "conditional"  # Only refresh if S3 version changed
}

# Response:
{
  "status": "not_modified",
  "key": "/data.json",
  "reason": "S3 ETag matches cached version"
}
```

#### Configuration

```yaml
cache:
  refresh:
    enabled: true
    
    # Auto-refresh strategy
    auto_refresh:
      enabled: true
      trigger: "ttl_80_percent"  # Refresh when 80% of TTL elapsed
      background: true            # Don't block client requests
      
    # Conditional refresh (check If-Modified-Since)
    conditional:
      enabled: true
      check_etag: true
      check_last_modified: true
```

---

## 3. Last-Modified and Conditional Requests

### v1.0 Status: ⚠️ Partial (Header Forwarding)

**What works:**
```
Client → Proxy: GET /file.txt
Proxy → S3: GET /file.txt
S3 → Proxy: 200 OK
            Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT
            ETag: "abc123"
Proxy → Client: 200 OK
                Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT
                ETag: "abc123"
```

✅ Proxy **forwards** Last-Modified and ETag headers  
❌ Proxy **doesn't use them** for cache validation

**Current behavior:**
- Client sends `If-Modified-Since` → Proxy ignores it
- Client sends `If-None-Match` → Proxy ignores it
- Cache validation based on TTL only

**Example:**
```bash
# First request
curl -v http://proxy/file.txt
# < Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT
# < ETag: "abc123"

# Second request with conditional header
curl -v http://proxy/file.txt \
  -H "If-Modified-Since: Mon, 01 Jan 2024 12:00:00 GMT"

# Current v1.0 behavior: Returns 200 + full file (from cache)
# Expected behavior: Should return 304 Not Modified
```

---

### v1.1 Planned: ✅ Smart Conditional Requests

#### Client-Side Caching (304 Not Modified)

```
Client → Proxy: GET /file.txt
                If-Modified-Since: Mon, 01 Jan 2024 12:00:00 GMT
                
Proxy checks cache:
  - File in cache: YES
  - Cached Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT
  - Comparison: SAME
  
Proxy → Client: 304 Not Modified
                (No body sent, saves bandwidth!)
```

**Benefits:**
- Saves bandwidth (no body transfer)
- Faster response (just headers)
- Client can use local cache

#### Server-Side Cache Validation

```
Proxy cache entry:
  - Key: /file.txt
  - Cached at: 12:00 PM
  - TTL: 1 hour (expires 1:00 PM)
  - ETag: "abc123"
  - Last-Modified: Mon, 01 Jan 2024 12:00:00 GMT

Request at 12:50 PM (TTL 80% expired):
  
Proxy → S3: HEAD /file.txt
            If-None-Match: "abc123"
            
S3 → Proxy: 304 Not Modified
            (File unchanged)
            
Proxy: Extends cache TTL, serves from cache

Client ← Proxy: 200 OK (from cache)
```

**Benefits:**
- Cache stays fresh without re-downloading
- Saves S3 data transfer costs
- Smart TTL extension

#### Configuration

```yaml
cache:
  conditional_requests:
    enabled: true
    
    # Client-side conditional requests
    client_304:
      enabled: true
      check_etag: true
      check_last_modified: true
      
    # Server-side cache revalidation
    server_revalidation:
      enabled: true
      trigger: "ttl_80_percent"  # Revalidate at 80% TTL
      use_head_request: true      # Use HEAD (not GET)
      extend_ttl_on_match: true   # Extend TTL if not modified
```

#### API for Cache Metadata

```bash
# Get cache entry metadata
GET /admin/cache/info?bucket=static&key=/file.txt

# Response:
{
  "key": "/file.txt",
  "bucket": "static",
  "cached_at": "2025-10-21T12:00:00Z",
  "expires_at": "2025-10-21T13:00:00Z",
  "ttl_remaining": 600,
  "size_bytes": 102400,
  "last_modified": "2025-10-20T10:00:00Z",
  "etag": "abc123def456",
  "hit_count": 1500,
  "last_accessed": "2025-10-21T12:45:00Z"
}
```

---

## Comparison: v1.0 vs v1.1

| Feature | v1.0 | v1.1 |
|---------|------|------|
| **Cache Purging** |
| Purge by key | ❌ | ✅ |
| Purge by prefix | ❌ | ✅ |
| Purge by pattern | ❌ | ✅ |
| Purge entire bucket | ⚠️ Restart only | ✅ API |
| **Cache Renewal** |
| TTL-based expiry | ✅ | ✅ |
| Manual refresh | ❌ | ✅ |
| Conditional refresh | ❌ | ✅ |
| Background refresh | ❌ | ✅ |
| **Conditional Requests** |
| Forward headers | ✅ | ✅ |
| 304 Not Modified | ❌ | ✅ |
| ETag validation | ❌ | ✅ |
| If-Modified-Since | ❌ | ✅ |
| Cache revalidation | ❌ | ✅ |

---

## Use Cases and Examples

### Use Case 1: Deploy New Frontend

**Problem:** Updated JavaScript/CSS but users see old cached version

**v1.0 Solution:**
```bash
# 1. Update config with short TTL
cache:
  ttl: 300  # 5 minutes

# 2. Wait 5 minutes
sleep 300

# 3. Users automatically get new version
```

**v1.1 Solution:**
```bash
# Immediate purge after deployment
curl -X DELETE http://proxy/admin/cache/purge \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "bucket": "static",
    "prefix": "/assets/",
    "recursive": true
  }'

# Users get new version immediately
```

---

### Use Case 2: Content Update

**Problem:** Updated image on S3, want users to see it now

**v1.0 Solution:**
```bash
# Only option: Restart proxy
systemctl restart yatagarasu

# Or wait for TTL to expire
```

**v1.1 Solution:**
```bash
# Option 1: Purge old version
curl -X DELETE http://proxy/admin/cache/purge \
  -d '{"bucket": "images", "key": "/hero.jpg"}'

# Option 2: Refresh (re-fetch from S3)
curl -X POST http://proxy/admin/cache/refresh \
  -d '{"bucket": "images", "key": "/hero.jpg"}'
```

---

### Use Case 3: Reduce Bandwidth

**Problem:** Large files repeatedly transferred to clients

**v1.0 Solution:**
```bash
# Enable caching with long TTL
cache:
  enabled: true
  ttl: 86400  # 24 hours
  max_item_size: "10MB"
```

**v1.1 Solution:**
```yaml
# Enable conditional requests
cache:
  conditional_requests:
    enabled: true
    client_304: true

# Clients can now use If-Modified-Since
# Proxy responds with 304 (no body)
# Saves bandwidth!
```

---

### Use Case 4: Keep Cache Fresh

**Problem:** Long TTL = stale content, Short TTL = poor cache hit rate

**v1.0 Solution:**
```yaml
# Compromise with medium TTL
cache:
  ttl: 3600  # 1 hour - not ideal
```

**v1.1 Solution:**
```yaml
# Smart revalidation: Long TTL + background refresh
cache:
  ttl: 86400  # 24 hours (long)
  conditional_requests:
    server_revalidation:
      enabled: true
      trigger: "ttl_80_percent"  # Check at 19.2 hours
      extend_ttl_on_match: true  # Extend if unchanged

# Result: 24hr cache + always fresh!
```

---

## Monitoring and Metrics

### v1.0 Metrics (Available)

```
yatagarasu_cache_hits_total{bucket}
yatagarasu_cache_misses_total{bucket}
yatagarasu_cache_size_bytes{bucket}
yatagarasu_cache_items{bucket}
yatagarasu_cache_evictions_total{bucket}
```

### v1.1 Additional Metrics (Planned)

```
# Purge operations
yatagarasu_cache_purges_total{bucket, type}
yatagarasu_cache_purge_duration_seconds{bucket}

# Refresh operations
yatagarasu_cache_refreshes_total{bucket, type}
yatagarasu_cache_refresh_duration_seconds{bucket}

# Conditional requests
yatagarasu_conditional_requests_total{bucket, result}
yatagarasu_304_responses_total{bucket}
yatagarasu_revalidations_total{bucket, result}

# Bandwidth savings
yatagarasu_bytes_saved_304{bucket}
yatagarasu_bytes_saved_revalidation{bucket}
```

---

## Workarounds for v1.0

### Manual Purge Script

```bash
#!/bin/bash
# purge-and-refresh.sh

PROXY="http://localhost:8080"
FILE="/static/main.css"

# 1. Restart proxy to clear cache
systemctl restart yatagarasu

# 2. Wait for startup
sleep 5

# 3. Request file to re-cache
curl -s "$PROXY$FILE" > /dev/null

echo "Cache refreshed for $FILE"
```

### Conditional Request Polyfill

```bash
#!/bin/bash
# conditional-request.sh

URL="$1"
CACHE_FILE="/tmp/cache/$(echo $URL | md5sum | cut -d' ' -f1)"

if [ -f "$CACHE_FILE" ]; then
  # Get cached Last-Modified
  LAST_MODIFIED=$(head -1 "$CACHE_FILE")
  
  # Request with If-Modified-Since
  RESPONSE=$(curl -s -D - "$URL" \
    -H "If-Modified-Since: $LAST_MODIFIED")
  
  if echo "$RESPONSE" | grep -q "304 Not Modified"; then
    # Use cached version
    tail -n +2 "$CACHE_FILE"
  else
    # Save new version
    echo "$RESPONSE" > "$CACHE_FILE"
    tail -n +2 "$CACHE_FILE"
  fi
else
  # First request
  curl -s -D - "$URL" > "$CACHE_FILE"
  tail -n +2 "$CACHE_FILE"
fi
```

---

## Summary

| Question | v1.0 Answer | v1.1 Answer |
|----------|-------------|-------------|
| **Cache Purging?** | ❌ No (restart only) | ✅ Yes (full API) |
| **Cache Renewal?** | ⚠️ TTL only | ✅ Yes (manual + auto) |
| **Check Last-Modified?** | ⚠️ Forward only | ✅ Yes (validate) |
| **304 Not Modified?** | ❌ No | ✅ Yes |
| **ETag validation?** | ❌ No | ✅ Yes |
| **Workarounds available?** | ✅ Yes | N/A |

---

## Complete Documentation

See detailed guides:
- **Current file** - Cache management overview
- **[CACHE_PREWARMING.md](CACHE_PREWARMING.md)** - Pre-warming strategies
- **[STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md)** - Cache architecture
- **[spec.md](spec.md)** - Feature specifications

---

**Bottom Line:**

| Feature | Status |
|---------|--------|
| **Purging** | ❌ v1.0: No → ✅ v1.1: Full support |
| **Renewal** | ⚠️ v1.0: TTL only → ✅ v1.1: Manual + Auto |
| **Last-Modified** | ⚠️ v1.0: Forward → ✅ v1.1: Validate |

For v1.0, use workarounds (restart proxy, short TTL). Full cache management coming in v1.1 (Q4 2025)! 🚀
