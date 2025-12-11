---
title: Caching Setup
layout: default
parent: Tutorials
nav_order: 4
---

# Caching Setup

Configure multi-tier caching for optimal performance.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Enable memory caching for fastest access
- Configure disk caching for larger capacity
- Understand cache behavior and TTL
- Monitor cache hit rates

## Prerequisites

- Completed the [Basic Proxy Setup](/yatagarasu/tutorials/basic-proxy/) tutorial
- Docker installed

---

## Understanding the Cache Tiers

Yatagarasu supports three cache tiers:

```
Request
   |
   v
+------------------+
|   Memory Cache   |  L1: ~320ns access, smallest capacity
|   (Moka TinyLFU) |  Best for: frequently accessed files
+------------------+
   |
   v (miss)
+------------------+
|   Redis/Valkey   |  L2: ~1ms access, shared across instances
|   Cache          |  Best for: distributed deployments
+------------------+
   |
   v (miss)
+------------------+
|   Disk Cache     |  L3: ~5ms access, largest capacity
|                  |  Best for: large files, persistence
+------------------+
   |
   v (miss)
+------------------+
|   S3 Backend     |  Origin: ~50-200ms
+------------------+
```

---

## Step 1: Setup

Create a tutorial directory:

```bash
mkdir caching-tutorial && cd caching-tutorial
```

Create `docker-compose.yml`:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
      - cache-data:/var/cache/yatagarasu
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
    depends_on:
      minio:
        condition: service_healthy

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 3

  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set local http://minio:9000 minioadmin minioadmin;
      mc mb local/test-bucket --ignore-existing;
      # Create files of different sizes
      dd if=/dev/urandom bs=1K count=1 2>/dev/null | mc pipe local/test-bucket/small-1kb.bin;
      dd if=/dev/urandom bs=1K count=100 2>/dev/null | mc pipe local/test-bucket/medium-100kb.bin;
      dd if=/dev/urandom bs=1M count=1 2>/dev/null | mc pipe local/test-bucket/large-1mb.bin;
      echo 'Test files created!';
      "

volumes:
  cache-data:
```

---

## Step 2: Enable Memory Caching

Create `config.yaml` with memory caching:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test"
    path_prefix: "/files"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

cache:
  memory:
    max_capacity: 104857600  # 100MB
    ttl_seconds: 3600        # 1 hour

metrics:
  enabled: true
  port: 9090
```

---

## Step 3: Start and Test

```bash
docker compose up -d
sleep 5

# First request - cache miss, fetches from S3
time curl -s http://localhost:8080/files/small-1kb.bin > /dev/null
# ~50-100ms (S3 fetch)

# Second request - cache hit
time curl -s http://localhost:8080/files/small-1kb.bin > /dev/null
# ~1-5ms (from memory cache)
```

---

## Step 4: Monitor Cache Metrics

```bash
# View cache metrics
curl -s http://localhost:9090/metrics | grep cache

# Key metrics:
# yatagarasu_cache_hits_total
# yatagarasu_cache_misses_total
# yatagarasu_cache_size_bytes
```

Calculate hit rate:

```bash
curl -s http://localhost:9090/metrics | grep yatagarasu_cache | head -10
```

---

## Step 5: Add Disk Caching

Update `config.yaml` to add disk cache:

```yaml
cache:
  memory:
    max_capacity: 52428800   # 50MB
    ttl_seconds: 3600

  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 1073741824  # 1GB
    ttl_seconds: 86400        # 24 hours
```

Restart:

```bash
docker compose restart yatagarasu
```

Now files evicted from memory cache are stored on disk before being evicted completely.

---

## Step 6: Test Cache Behavior

```bash
# Fetch large file (may exceed memory cache threshold)
curl -s http://localhost:8080/files/large-1mb.bin > /dev/null

# Fetch many small files to fill memory cache
for i in {1..20}; do
  curl -s "http://localhost:8080/files/small-1kb.bin?v=$i" > /dev/null
done

# Original file may be evicted from memory but still on disk
time curl -s http://localhost:8080/files/large-1mb.bin > /dev/null
# Fast if still in disk cache
```

---

## Step 7: Cache Size Thresholds

Configure which files get cached:

```yaml
cache:
  memory:
    max_capacity: 104857600   # 100MB total
    max_file_size: 10485760   # Only cache files < 10MB
    ttl_seconds: 3600

  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 5368709120  # 5GB total
    max_file_size: 104857600  # Only cache files < 100MB
    ttl_seconds: 86400
```

Files larger than `max_file_size` are streamed directly without caching.

---

## Step 8: Cache TTL Behavior

```yaml
cache:
  memory:
    max_capacity: 104857600
    ttl_seconds: 300  # 5 minutes - entries expire after this time

  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 1073741824
    ttl_seconds: 3600  # 1 hour - longer TTL for disk
```

TTL controls how long cached content is considered valid. After TTL expires:
- Entry is evicted on next access
- Fresh copy is fetched from S3

---

## Cache Configuration Reference

### Memory Cache Options

| Option | Description | Default |
|:-------|:------------|:--------|
| `max_capacity` | Maximum cache size in bytes | 512MB |
| `max_file_size` | Max file size to cache | 10MB |
| `ttl_seconds` | Time-to-live in seconds | 3600 |

### Disk Cache Options

| Option | Description | Default |
|:-------|:------------|:--------|
| `enabled` | Enable disk cache | false |
| `path` | Directory for cache files | /var/cache/yatagarasu |
| `max_capacity` | Maximum cache size in bytes | 1GB |
| `max_file_size` | Max file size to cache | 100MB |
| `ttl_seconds` | Time-to-live in seconds | 86400 |

---

## Performance Tuning

### Small Files (< 1MB)

```yaml
cache:
  memory:
    max_capacity: 536870912   # 512MB - prioritize memory
    max_file_size: 1048576    # 1MB
    ttl_seconds: 7200         # 2 hours
```

### Mixed Workload

```yaml
cache:
  memory:
    max_capacity: 268435456   # 256MB
    max_file_size: 5242880    # 5MB
    ttl_seconds: 3600

  disk:
    enabled: true
    max_capacity: 10737418240 # 10GB
    max_file_size: 104857600  # 100MB
    ttl_seconds: 86400
```

### Large Files with Limited Memory

```yaml
cache:
  memory:
    max_capacity: 134217728   # 128MB - small memory footprint
    max_file_size: 1048576    # 1MB - only cache tiny files
    ttl_seconds: 1800

  disk:
    enabled: true
    max_capacity: 53687091200 # 50GB
    max_file_size: 1073741824 # 1GB
    ttl_seconds: 172800       # 48 hours
```

---

## Cache Eviction

Yatagarasu uses **TinyLFU** (Tiny Least Frequently Used) for memory cache:

- Tracks access frequency efficiently
- Better hit rate than pure LRU
- Low memory overhead

When cache is full, least-frequently-used entries are evicted first.

---

## Cleanup

```bash
docker compose down -v
cd .. && rm -rf caching-tutorial
```

---

## Best Practices

1. **Size memory cache appropriately** - More memory = higher hit rate
2. **Use disk cache for persistence** - Survives restarts
3. **Set appropriate TTL** - Balance freshness vs hit rate
4. **Monitor hit rates** - Target 80%+ for static content
5. **Consider file sizes** - Don't cache files larger than practical

---

## Next Steps

- [Valkey/Redis Integration](/yatagarasu/tutorials/valkey-redis/) - Distributed caching
- [Configuration Reference](/yatagarasu/configuration/cache/) - All cache options
- [Performance Tuning](/yatagarasu/operations/performance/) - Optimization guide
