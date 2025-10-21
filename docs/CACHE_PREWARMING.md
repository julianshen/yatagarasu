# Cache Pre-Warming (Recursive Path Prefetching)

## Quick Answer

**Q: Does Yatagarasu support pre-warming cache for a path recursively?**

**Current Status (v1.0):** ❌ **NOT implemented**  
**Future Version (v1.1):** ✅ **Planned feature**  
**Priority:** Medium (valuable for production deployments)

---

## What is Cache Pre-Warming?

Cache pre-warming (also called cache priming or prefetching) is the process of **proactively loading frequently-accessed objects into cache** before users request them.

### Example Scenario

**Without Pre-Warming:**
```
1. Proxy starts → Cache is empty
2. First 1000 users request /static/logo.png
3. All 1000 requests hit S3 (slow, expensive)
4. Logo.png finally cached
5. Subsequent requests served from cache (fast)
```

**With Pre-Warming:**
```
1. Proxy starts → Pre-warm /static/* recursively
2. Logo.png already in cache
3. First 1000 users get instant response (fast, cheap)
4. No S3 requests needed
```

---

## Proposed Feature Design (v1.1)

### Configuration

```yaml
buckets:
  - name: "static-assets"
    path_prefix: "/static"
    s3:
      bucket: "my-static-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    cache:
      enabled: true
      ttl: 86400
      max_size: "5GB"
      max_item_size: "10MB"
      
      # Pre-warming configuration
      prewarm:
        enabled: true
        on_startup: true          # Pre-warm when proxy starts
        on_schedule: "0 */6 * * *" # Cron: every 6 hours
        
        # Paths to pre-warm
        paths:
          - path: "/css/"
            recursive: true
            max_depth: 5
            max_files: 1000
            
          - path: "/js/"
            recursive: true
            max_depth: 5
            max_files: 1000
            
          - path: "/images/common/"
            recursive: true
            max_depth: 2
            max_files: 500
            
          - path: "/fonts/"
            recursive: false  # Only top-level files
            
        # Filters
        include_patterns:
          - "*.css"
          - "*.js"
          - "*.png"
          - "*.jpg"
          - "*.woff2"
        
        exclude_patterns:
          - "*.tmp"
          - "*.bak"
          - "*-dev.*"
        
        # Performance controls
        concurrency: 10          # Parallel pre-warm requests
        rate_limit: "100/s"      # Max 100 files/second
        timeout: "30m"           # Max pre-warm duration
```

### API Endpoints

```bash
# Manual trigger pre-warm
POST /admin/cache/prewarm
Content-Type: application/json
{
  "bucket": "static-assets",
  "path": "/css/",
  "recursive": true,
  "max_depth": 5
}

# Response:
{
  "task_id": "prewarm-123",
  "status": "started",
  "estimated_files": 500
}

# Check pre-warm status
GET /admin/cache/prewarm/status/prewarm-123

# Response:
{
  "task_id": "prewarm-123",
  "status": "running",
  "progress": {
    "files_scanned": 300,
    "files_cached": 250,
    "bytes_cached": "50MB",
    "elapsed": "30s",
    "estimated_remaining": "20s"
  }
}

# Cancel pre-warm
DELETE /admin/cache/prewarm/prewarm-123
```

---

## How It Works

### 1. Discovery Phase (LIST Objects)

```
Client (Admin)          Yatagarasu               S3 Backend
     |                       |                        |
     | POST /admin/          |                        |
     |  cache/prewarm        |                        |
     |---------------------->|                        |
     |                       |                        |
     |                       | LIST /css/ recursive   |
     |                       | (with prefix)          |
     |                       |----------------------->|
     |                       |                        |
     |                       |  Returns list of keys: |
     |                       |  - css/main.css        |
     |                       |  - css/theme.css       |
     |                       |  - css/vendor/lib.css  |
     |                       |<-----------------------|
     |                       |                        |
     |   Task created        |                        |
     |   task_id: "123"      |                        |
     |<----------------------|                        |
     |                       |                        |
```

### 2. Prefetch Phase (GET Objects)

```
     Yatagarasu                    S3 Backend
          |                             |
          | [Background worker pool]    |
          |                             |
    ┌─────┴─────┐                       |
    │ Worker 1  │ GET /css/main.css     |
    │           │---------------------->|
    │           │<----------------------|
    │           │ Cache: main.css       |
    └───────────┘                       |
          |                             |
    ┌─────┴─────┐                       |
    │ Worker 2  │ GET /css/theme.css    |
    │           │---------------------->|
    │           │<----------------------|
    │           │ Cache: theme.css      |
    └───────────┘                       |
          |                             |
    ┌─────┴─────┐                       |
    │ Worker 3  │ GET /css/vendor/lib.css
    │           │---------------------->|
    │           │<----------------------|
    │           │ Cache: lib.css        |
    └───────────┘                       |
          |                             |
     [Continues until all files cached] |
```

### 3. Progress Tracking

```
Time    Files   Bytes     Status
----    -----   -----     ------
0:00    0/500   0MB       Starting...
0:10    50/500  10MB      Running (10%)
0:20    150/500 30MB      Running (30%)
0:30    300/500 60MB      Running (60%)
0:40    450/500 90MB      Running (90%)
0:45    500/500 100MB     Completed
```

---

## Use Cases

### 1. Application Deployment
```yaml
# Pre-warm static assets after deployment
prewarm:
  on_startup: true
  paths:
    - path: "/v2.0/static/"
      recursive: true
```

**Benefit**: First users get instant load times, not cold cache

### 2. Scheduled Refresh
```yaml
# Refresh cache every 6 hours
prewarm:
  on_schedule: "0 */6 * * *"
  paths:
    - path: "/catalog/"
      recursive: true
```

**Benefit**: Cache stays fresh without manual intervention

### 3. Peak Traffic Preparation
```bash
# Before Black Friday sale
curl -X POST http://proxy/admin/cache/prewarm \
  -H "Authorization: Bearer admin-token" \
  -d '{"path": "/products/", "recursive": true}'
```

**Benefit**: Handle traffic spike with warm cache

### 4. Multi-Region Deployment
```yaml
# Warm cache in new region before traffic cutover
prewarm:
  on_startup: true
  paths:
    - path: "/"
      recursive: true
      max_files: 10000
```

**Benefit**: New region ready to serve immediately

---

## Performance Characteristics

### Metrics

| Scenario | Files | Size | Time | S3 Requests |
|----------|-------|------|------|-------------|
| Small site | 100 | 10MB | 10s | 100 |
| Medium site | 1,000 | 100MB | 2m | 1,000 |
| Large site | 10,000 | 1GB | 20m | 10,000 |
| Huge site | 100,000 | 10GB | 3h+ | 100,000 |

### Resource Usage

**During Pre-Warming:**
```
CPU: 10-30% (parallel fetching)
Memory: Cache size + worker buffers
Network: Full bandwidth utilized
S3 Requests: High (one per file)
```

**After Pre-Warming:**
```
CPU: Normal (~5%)
Memory: Cache size (stable)
Network: Minimal (cache hits)
S3 Requests: Near zero (cache serving)
```

### Cost Implications

**Example: Pre-warm 10,000 files daily**

```
S3 LIST requests: 10 requests/day × $0.005/1000 = $0.00005/day
S3 GET requests: 10,000 requests/day × $0.0004/1000 = $4/day
S3 Data transfer: 1GB/day × $0.09/GB = $0.09/day
Total: ~$4.09/day = ~$123/month

Savings from cache hits:
100,000 requests/day cached → $400/day saved
Net savings: $400 - $4 = $396/day = ~$11,880/month
```

**ROI**: Pre-warming pays for itself immediately with high traffic!

---

## Implementation Strategy (v1.1)

### Phase 1: Basic Pre-Warming
- [ ] S3 LIST operation support
- [ ] Sequential prefetching (one file at a time)
- [ ] Manual trigger API
- [ ] Progress tracking
- [ ] Basic filtering (include/exclude patterns)

### Phase 2: Advanced Features
- [ ] Parallel prefetching (worker pool)
- [ ] Scheduled pre-warming (cron)
- [ ] On-startup pre-warming
- [ ] Rate limiting
- [ ] Recursive depth control
- [ ] Pause/resume capability

### Phase 3: Smart Pre-Warming
- [ ] Analytics-based (warm most-accessed files)
- [ ] Predictive pre-warming (ML-based)
- [ ] Partial path warming
- [ ] Incremental updates only

---

## Alternatives (Workarounds for v1.0)

Since pre-warming is not in v1.0, here are workarounds:

### Option 1: External Script
```bash
#!/bin/bash
# prewarm.sh - Pre-warm cache by requesting all files

BASE_URL="http://proxy/static"
S3_BUCKET="my-bucket"
S3_PREFIX="static/"

# List all files from S3
aws s3 ls "s3://$S3_BUCKET/$S3_PREFIX" --recursive | \
  awk '{print $4}' | \
  while read file; do
    # Request each file through proxy
    curl -s "$BASE_URL/${file#$S3_PREFIX}" > /dev/null
    echo "Cached: $file"
  done

echo "Pre-warming complete!"
```

**Run on deployment:**
```bash
./prewarm.sh
```

### Option 2: Kubernetes Init Container
```yaml
apiVersion: v1
kind: Pod
metadata:
  name: yatagarasu-proxy
spec:
  initContainers:
  - name: cache-prewarm
    image: appropriate/curl
    command:
    - /bin/sh
    - -c
    - |
      # Pre-warm critical files
      for file in logo.png main.css app.js; do
        curl -s http://localhost:8080/static/$file > /dev/null
      done
  containers:
  - name: proxy
    image: yatagarasu:latest
```

### Option 3: Load Test Tool
```bash
# Use hey or wrk to warm cache
cat urls.txt | xargs -P 10 -I {} curl -s {}
```

**urls.txt:**
```
http://proxy/static/logo.png
http://proxy/static/main.css
http://proxy/static/app.js
...
```

---

## Configuration Examples (v1.1 Preview)

### Minimal Configuration
```yaml
cache:
  prewarm:
    enabled: true
    on_startup: true
    paths:
      - path: "/static/"
        recursive: true
```

### Production Configuration
```yaml
cache:
  enabled: true
  max_size: "10GB"
  prewarm:
    enabled: true
    on_startup: true
    on_schedule: "0 2 * * *"  # 2 AM daily
    
    paths:
      - path: "/static/critical/"
        recursive: true
        max_depth: 10
        priority: high
        
      - path: "/static/assets/"
        recursive: true
        max_depth: 5
        priority: normal
        
    include_patterns:
      - "*.css"
      - "*.js"
      - "*.png"
      - "*.jpg"
      - "*.woff2"
      
    exclude_patterns:
      - "*.map"
      - "*.bak"
      
    concurrency: 20
    rate_limit: "200/s"
    timeout: "1h"
```

### Selective Pre-Warming
```yaml
cache:
  prewarm:
    enabled: true
    paths:
      # Only top 100 most accessed files
      - path: "/hot/"
        recursive: false
        max_files: 100
        
      # All CSS/JS but limit size
      - path: "/assets/"
        recursive: true
        include_patterns: ["*.css", "*.js"]
        max_size: "100MB"
```

---

## Monitoring and Metrics

### Prometheus Metrics (v1.1)

```
# Pre-warming metrics
yatagarasu_prewarm_tasks_total{bucket, status}
yatagarasu_prewarm_files_total{bucket}
yatagarasu_prewarm_bytes_total{bucket}
yatagarasu_prewarm_duration_seconds{bucket}
yatagarasu_prewarm_errors_total{bucket, error_type}

# Cache effectiveness after pre-warming
yatagarasu_cache_hit_rate{bucket}
yatagarasu_cache_size_bytes{bucket}
yatagarasu_cache_items{bucket}
```

### Logs

```json
{
  "level": "info",
  "message": "Pre-warming started",
  "bucket": "static-assets",
  "path": "/static/",
  "recursive": true,
  "task_id": "prewarm-123"
}

{
  "level": "info",
  "message": "Pre-warming progress",
  "task_id": "prewarm-123",
  "files_cached": 500,
  "bytes_cached": 100000000,
  "elapsed_seconds": 30
}

{
  "level": "info",
  "message": "Pre-warming completed",
  "task_id": "prewarm-123",
  "files_cached": 1000,
  "bytes_cached": 200000000,
  "duration_seconds": 60,
  "errors": 0
}
```

---

## Comparison with Other Solutions

| Feature | Yatagarasu (v1.1) | Varnish | Nginx | CloudFront |
|---------|-------------------|---------|-------|------------|
| Recursive pre-warm | ✅ Yes | ❌ No | ⚠️ Manual | ✅ Yes (Invalidate) |
| API trigger | ✅ Yes | ❌ No | ❌ No | ✅ Yes |
| Scheduled pre-warm | ✅ Yes | ❌ No | ❌ No | ❌ No |
| S3 integration | ✅ Native | ⚠️ Via backend | ⚠️ Via proxy | ✅ Native |
| Progress tracking | ✅ Yes | ❌ No | ❌ No | ⚠️ Limited |

---

## Summary

### Current Status (v1.0)
❌ Pre-warming **NOT supported**  
⚠️ Use workarounds (external scripts, load tests)

### Future Version (v1.1)
✅ Full recursive pre-warming planned  
✅ API-driven and scheduled  
✅ Parallel fetching with rate limiting  
✅ Progress tracking and monitoring  

### Benefits When Implemented
- 🚀 **Instant load times** for first users after deployment
- 💰 **Cost savings** by serving from cache immediately
- 📊 **Predictable performance** during traffic spikes
- 🔄 **Automated refresh** via scheduled pre-warming
- 🎯 **Selective warming** with pattern matching

### Estimated Implementation
- **Complexity**: Medium
- **Development time**: 2-3 weeks
- **Testing time**: 1 week
- **Target release**: v1.1 (Q4 2025)

---

**For now (v1.0)**, use the workaround scripts provided above. Pre-warming will be a first-class feature in v1.1! 🚀
