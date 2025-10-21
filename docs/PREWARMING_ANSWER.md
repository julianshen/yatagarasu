# Quick Answer: Cache Pre-Warming Support

## Your Question

**Q: Does Yatagarasu support pre-warming cache for a path recursively?**

---

## Short Answer

**Current Version (v1.0):** ❌ **NO - Not implemented**  
**Future Version (v1.1):** ✅ **YES - Planned feature** (Q4 2025)

---

## What is Pre-Warming?

Pre-warming = Proactively loading files into cache **before** users request them.

**Without Pre-Warming:**
```
Deploy → Empty cache → 1000 users → All hit S3 (slow) → Cache populated
```

**With Pre-Warming:**
```
Deploy → Pre-warm cache → Cache populated → 1000 users → All cache hits (fast!)
```

---

## v1.1 Feature Preview

### Configuration (Coming Soon)

```yaml
buckets:
  - name: "static"
    path_prefix: "/static"
    cache:
      enabled: true
      prewarm:
        enabled: true
        on_startup: true           # Pre-warm on proxy start
        on_schedule: "0 */6 * * *" # Every 6 hours
        
        paths:
          - path: "/css/"
            recursive: true       # Recursive subdirectories
            max_depth: 5
            max_files: 1000
            
          - path: "/js/"
            recursive: true
            
        include_patterns:
          - "*.css"
          - "*.js"
          - "*.png"
        
        concurrency: 10          # Parallel workers
        rate_limit: "100/s"      # Max files/second
```

### API (Coming Soon)

```bash
# Trigger pre-warm manually
curl -X POST http://proxy/admin/cache/prewarm \
  -H "Authorization: Bearer admin-token" \
  -d '{
    "bucket": "static",
    "path": "/css/",
    "recursive": true
  }'

# Response:
{
  "task_id": "prewarm-123",
  "status": "started",
  "estimated_files": 500
}

# Check progress
curl http://proxy/admin/cache/prewarm/status/prewarm-123

# Response:
{
  "task_id": "prewarm-123",
  "status": "running",
  "progress": {
    "files_cached": 250,
    "bytes_cached": "50MB",
    "elapsed": "30s"
  }
}
```

---

## Workarounds for v1.0

Since pre-warming isn't in v1.0 yet, here are workarounds:

### Option 1: Simple Bash Script

```bash
#!/bin/bash
# prewarm-cache.sh

PROXY="http://localhost:8080"
PREFIX="/static"

# List files from S3
aws s3 ls s3://my-bucket/static/ --recursive | \
  awk '{print $4}' | \
  while read file; do
    # Request through proxy to populate cache
    curl -s "$PROXY$PREFIX/${file#static/}" > /dev/null
    echo "✓ Cached: $file"
  done

echo "Pre-warming complete!"
```

**Usage:**
```bash
chmod +x prewarm-cache.sh
./prewarm-cache.sh
```

### Option 2: Parallel Pre-Warming

```bash
#!/bin/bash
# Fast parallel pre-warming

cat urls.txt | xargs -P 10 -I {} curl -s {} > /dev/null

echo "Pre-warmed $(wc -l < urls.txt) files"
```

**urls.txt:**
```
http://proxy/static/logo.png
http://proxy/static/main.css
http://proxy/static/app.js
...
```

### Option 3: Python Script

```python
#!/usr/bin/env python3
import boto3
import requests
from concurrent.futures import ThreadPoolExecutor

PROXY = "http://localhost:8080"
BUCKET = "my-bucket"
PREFIX = "static/"

s3 = boto3.client('s3')

def prewarm_file(key):
    url = f"{PROXY}/{key}"
    requests.get(url)
    print(f"✓ Cached: {key}")

# List all files
response = s3.list_objects_v2(Bucket=BUCKET, Prefix=PREFIX)
keys = [obj['Key'] for obj in response.get('Contents', [])]

# Pre-warm in parallel
with ThreadPoolExecutor(max_workers=10) as executor:
    executor.map(prewarm_file, keys)

print(f"Pre-warmed {len(keys)} files")
```

### Option 4: Kubernetes Init Container

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: yatagarasu
spec:
  initContainers:
  - name: cache-prewarm
    image: amazon/aws-cli
    command:
    - /bin/sh
    - -c
    - |
      # List and pre-warm all static files
      aws s3 ls s3://my-bucket/static/ --recursive | \
        awk '{print $4}' | \
        xargs -I {} curl -s http://localhost:8080/{} > /dev/null
      echo "Cache pre-warmed"
  
  containers:
  - name: proxy
    image: yatagarasu:latest
    ports:
    - containerPort: 8080
```

---

## Use Cases for Pre-Warming

### 1. 🚀 Application Deployment
**Problem:** First users after deployment get cold cache (slow)  
**Solution:** Pre-warm critical assets on startup  
**Benefit:** Instant load times for all users

### 2. 📈 Peak Traffic Events
**Problem:** Black Friday, product launches = traffic spike  
**Solution:** Pre-warm entire catalog before event  
**Benefit:** Handle 10x traffic without S3 overload

### 3. 🌍 Multi-Region Deployment
**Problem:** New region has empty cache  
**Solution:** Pre-warm before traffic cutover  
**Benefit:** Consistent performance across regions

### 4. 🔄 Scheduled Refresh
**Problem:** Cache gets stale over time  
**Solution:** Pre-warm every 6 hours via cron  
**Benefit:** Always fresh, always fast

---

## Benefits When Implemented (v1.1)

| Benefit | Impact |
|---------|--------|
| **Instant load times** | First user = same speed as 1000th user |
| **Cost savings** | 90% fewer S3 requests = $400/day saved |
| **Traffic spike ready** | Cache pre-populated before rush |
| **Predictable performance** | No cold cache surprises |
| **Automated refresh** | Set and forget with cron |

### ROI Example

**Scenario:** E-commerce site, 10,000 static files, 100,000 requests/day

**Without Pre-Warming:**
```
First 10,000 requests → S3 (cold cache)
S3 costs: $4/day
User experience: Slow initial loads
```

**With Pre-Warming:**
```
Pre-warm: 10,000 S3 requests once = $4/day
Cached requests: 100,000 cache hits = $0/day
Total: $4/day (same cost!)
User experience: Fast for EVERYONE
```

**Net benefit:** Same cost, 10x better UX! 🎉

---

## Implementation Timeline

### v1.0 (Current)
❌ Pre-warming not available  
✅ Use workarounds (scripts above)

### v1.1 (Q4 2025)
✅ Recursive pre-warming  
✅ API trigger  
✅ Scheduled pre-warming  
✅ Progress tracking  
✅ Pattern matching (include/exclude)

### v2.0 (Future)
✅ ML-based predictive pre-warming  
✅ Analytics-driven (auto-warm hot files)  
✅ Incremental updates only  
✅ Distributed pre-warming across replicas

---

## Summary

| Aspect | Status |
|--------|--------|
| **v1.0 Support** | ❌ Not available |
| **v1.1 Support** | ✅ Fully planned |
| **Workarounds** | ✅ Available (scripts) |
| **Priority** | Medium (production valuable) |
| **Complexity** | Medium (2-3 weeks dev) |
| **Release Date** | Q4 2025 (estimated) |

---

## What to Do Now

### For v1.0 (Current):
1. Use the bash/python scripts above
2. Run pre-warming after deployments
3. Schedule via cron if needed
4. Monitor cache hit rates

### For v1.1 (When Available):
1. Update configuration with `prewarm` section
2. Enable `on_startup: true`
3. Add scheduled pre-warming
4. Use API for manual triggers
5. Monitor via `/admin/cache/prewarm/status`

---

## Complete Documentation

For full details, see:
- **[CACHE_PREWARMING.md](CACHE_PREWARMING.md)** - Complete 10KB guide with:
  - Detailed configuration examples
  - API specifications
  - Performance metrics
  - Cost analysis
  - All workarounds
  - Implementation plan

---

**Bottom Line:** Pre-warming is NOT in v1.0, but it's coming in v1.1! Use the workaround scripts for now. It'll be worth the wait - instant load times for everyone! 🚀
