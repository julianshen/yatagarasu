---
title: Cache Configuration
layout: default
parent: Configuration
nav_order: 5
---

# Cache Configuration

Configure multi-tier caching for optimal performance.
{: .fs-6 .fw-300 }

---

## Overview

Yatagarasu supports three cache tiers:

```
L1: Memory Cache  (fastest, smallest)
         |
         v
L2: Redis/Valkey  (fast, shared across instances)
         |
         v
L3: Disk Cache    (slower, largest capacity)
         |
         v
S3 Origin         (slowest)
```

---

## Memory Cache

In-memory cache using Moka with TinyLFU eviction.

```yaml
cache:
  memory:
    max_capacity: 536870912    # 512MB
    max_file_size: 10485760    # 10MB
    ttl_seconds: 3600          # 1 hour
```

### Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `max_capacity` | integer | 536870912 (512MB) | Total cache size in bytes |
| `max_file_size` | integer | 10485760 (10MB) | Max file size to cache |
| `ttl_seconds` | integer | 3600 | Time-to-live |

### Performance

- Access time: ~320ns
- Best for: frequently accessed small files
- Memory: allocated from heap

---

## Redis/Valkey Cache

Distributed cache shared across instances.

```yaml
cache:
  redis:
    enabled: true
    url: "redis://localhost:6379"
    max_capacity: 1073741824   # 1GB
    ttl_seconds: 7200          # 2 hours
    pool_size: 10
    timeout_ms: 1000
```

### Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `enabled` | boolean | false | Enable Redis cache |
| `url` | string | - | Redis connection URL |
| `max_capacity` | integer | 1073741824 (1GB) | Max cache size |
| `max_file_size` | integer | 104857600 (100MB) | Max file size to cache |
| `ttl_seconds` | integer | 7200 | Time-to-live |
| `pool_size` | integer | 10 | Connection pool size |
| `timeout_ms` | integer | 1000 | Connection timeout |
| `retry_attempts` | integer | 3 | Retry count on failure |
| `retry_delay_ms` | integer | 100 | Delay between retries |

### Connection URLs

```yaml
# Basic
url: "redis://localhost:6379"

# With password
url: "redis://:password@localhost:6379"

# With username (Redis 6+)
url: "redis://username:password@localhost:6379"

# With database
url: "redis://localhost:6379/1"

# With TLS
url: "rediss://localhost:6379"
```

### Sentinel Configuration

```yaml
cache:
  redis:
    enabled: true
    sentinel:
      master_name: "mymaster"
      nodes:
        - "sentinel-1:26379"
        - "sentinel-2:26379"
        - "sentinel-3:26379"
```

### Cluster Configuration

```yaml
cache:
  redis:
    enabled: true
    cluster:
      nodes:
        - "redis-1:6379"
        - "redis-2:6379"
        - "redis-3:6379"
```

---

## Disk Cache

Persistent disk-based cache.

```yaml
cache:
  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 10737418240  # 10GB
    max_file_size: 104857600   # 100MB
    ttl_seconds: 86400         # 24 hours
```

### Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `enabled` | boolean | false | Enable disk cache |
| `path` | string | /var/cache/yatagarasu | Cache directory |
| `max_capacity` | integer | 10737418240 (10GB) | Total cache size |
| `max_file_size` | integer | 104857600 (100MB) | Max file size to cache |
| `ttl_seconds` | integer | 86400 | Time-to-live |

### Performance

- Access time: ~5-10ms
- Best for: large files, persistence across restarts
- Storage: requires fast SSD for best performance

---

## Complete Examples

### Memory Only (Development)

```yaml
cache:
  memory:
    max_capacity: 268435456    # 256MB
    ttl_seconds: 1800          # 30 minutes
```

### Memory + Disk (Single Instance)

```yaml
cache:
  memory:
    max_capacity: 536870912    # 512MB
    max_file_size: 10485760    # 10MB
    ttl_seconds: 3600

  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 10737418240  # 10GB
    max_file_size: 104857600   # 100MB
    ttl_seconds: 86400
```

### Full Stack (Production)

```yaml
cache:
  memory:
    max_capacity: 268435456    # 256MB per instance
    max_file_size: 5242880     # 5MB
    ttl_seconds: 1800          # 30 min

  redis:
    enabled: true
    url: "redis://redis-cluster:6379"
    max_capacity: 2147483648   # 2GB shared
    max_file_size: 52428800    # 50MB
    ttl_seconds: 7200          # 2 hours
    pool_size: 20

  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 53687091200  # 50GB
    max_file_size: 524288000   # 500MB
    ttl_seconds: 172800        # 48 hours
```

---

## Cache Behavior

### Cache Key

Cache keys are based on:
- Bucket name
- Full request path
- Query string (if present)

```
bucket:my-bucket:path:/images/logo.png
bucket:my-bucket:path:/data.json?v=2
```

### Cache Flow

```
Request arrives
      |
      v
+---------------+
| Memory (L1)   |-----> HIT: Return cached
+---------------+
      |
    MISS
      v
+---------------+
| Redis (L2)    |-----> HIT: Return + populate L1
+---------------+
      |
    MISS
      v
+---------------+
| Disk (L3)     |-----> HIT: Return + populate L1, L2
+---------------+
      |
    MISS
      v
+---------------+
| S3 Origin     |-----> Fetch + populate all tiers
+---------------+
```

### TTL Behavior

- Each tier has independent TTL
- Expired entries are lazily removed on access
- Background cleanup runs periodically

---

## Size Guidelines

### Memory Cache Sizing

| Use Case | Recommended Size |
|:---------|:-----------------|
| Development | 128-256MB |
| Small deployment | 256-512MB |
| Medium deployment | 512MB-1GB |
| Large deployment | 1-2GB |

### Redis Cache Sizing

| Instances | Recommended Size |
|:----------|:-----------------|
| 2-3 | 512MB-1GB |
| 5-10 | 1-2GB |
| 10+ | 2-4GB |

### Disk Cache Sizing

| Traffic | Recommended Size |
|:--------|:-----------------|
| Light | 5-10GB |
| Medium | 10-50GB |
| Heavy | 50-200GB |

---

## Eviction Policy

Memory cache uses **TinyLFU** (Tiny Least Frequently Used):
- Tracks frequency with low memory overhead
- Better hit rate than LRU for most workloads
- Window admission policy to handle scan resistance

Redis uses its configured `maxmemory-policy`:
- Recommended: `allkeys-lru` or `volatile-lru`

---

## Cache Warming

Pre-populate cache on startup:

```yaml
cache:
  warming:
    enabled: true
    paths:
      - "/assets/critical.css"
      - "/assets/logo.png"
    concurrency: 4
    rate_limit_requests_per_second: 10
```

---

## Metrics

Cache exposes these Prometheus metrics:

```
# Hit/miss rates
yatagarasu_cache_hits_total{tier="memory|redis|disk"}
yatagarasu_cache_misses_total{tier="memory|redis|disk"}

# Cache size
yatagarasu_cache_size_bytes{tier="memory|redis|disk"}
yatagarasu_cache_entries{tier="memory|redis|disk"}

# Timing
yatagarasu_cache_get_duration_seconds{tier="memory|redis|disk"}
yatagarasu_cache_set_duration_seconds{tier="memory|redis|disk"}
```

---

## Best Practices

1. **Size memory cache for hot data** - Top 20% of files
2. **Use Redis for multi-instance** - Shared cache reduces origin load
3. **Enable disk for large files** - Avoid re-fetching GBs
4. **Set appropriate TTL** - Balance freshness vs hit rate
5. **Monitor hit rates** - Target 80%+ for static content

---

## Troubleshooting

### Low Hit Rate

- Increase cache capacity
- Extend TTL
- Check if content is cacheable (not Range requests)

### High Memory Usage

- Reduce `max_capacity`
- Lower `max_file_size`
- Use disk cache for larger files

### Redis Connection Issues

- Check network connectivity
- Increase `pool_size` under load
- Monitor connection pool metrics

---

## See Also

- [Caching Tutorial](/yatagarasu/tutorials/caching/)
- [Valkey/Redis Tutorial](/yatagarasu/tutorials/valkey-redis/)
- [Performance Guide](/yatagarasu/operations/performance/)
