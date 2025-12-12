---
title: Valkey/Redis Integration
layout: default
parent: Tutorials
nav_order: 5
---

# Valkey/Redis Integration

Set up distributed caching with Valkey or Redis.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Configure Valkey/Redis as a distributed cache layer
- Share cache across multiple Yatagarasu instances
- Configure connection pooling and failover
- Monitor cache performance

## Prerequisites

- Completed the [Caching Setup](/yatagarasu/tutorials/caching/) tutorial
- Docker installed

---

## Why Distributed Cache?

When running multiple Yatagarasu instances:

```
Without distributed cache:
Instance 1: Cache MISS -> Fetch from S3
Instance 2: Cache MISS -> Fetch from S3  (duplicate!)
Instance 3: Cache MISS -> Fetch from S3  (duplicate!)

With Valkey/Redis:
Instance 1: Cache MISS -> Fetch from S3 -> Store in Valkey
Instance 2: Cache HIT  -> Get from Valkey (fast!)
Instance 3: Cache HIT  -> Get from Valkey (fast!)
```

---

## Step 1: Setup

Create a directory:

```bash
mkdir valkey-tutorial && cd valkey-tutorial
```

Create `docker-compose.yml`:

```yaml
version: "3.8"

services:
  # Yatagarasu Instance 1
  yatagarasu-1:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    ports:
      - "8081:8080"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://valkey:6379
    depends_on:
      minio:
        condition: service_healthy
      valkey:
        condition: service_healthy

  # Yatagarasu Instance 2
  yatagarasu-2:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    ports:
      - "8082:8080"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://valkey:6379
    depends_on:
      minio:
        condition: service_healthy
      valkey:
        condition: service_healthy

  # Valkey (Redis-compatible)
  valkey:
    image: valkey/valkey:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - valkey-data:/data
    healthcheck:
      test: ["CMD", "valkey-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 3
    command: valkey-server --appendonly yes --maxmemory 256mb --maxmemory-policy allkeys-lru

  # MinIO
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
      echo 'Hello from S3!' | mc pipe local/test-bucket/hello.txt;
      dd if=/dev/urandom bs=1K count=100 2>/dev/null | mc pipe local/test-bucket/data.bin;
      "

volumes:
  valkey-data:
```

---

## Step 2: Configure Yatagarasu with Valkey

Create `config.yaml`:

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
  # L1: Local memory cache (per-instance)
  memory:
    max_capacity: 52428800    # 50MB
    ttl_seconds: 300          # 5 minutes

  # L2: Distributed Valkey cache (shared)
  redis:
    enabled: true
    url: "${REDIS_URL}"
    max_capacity: 268435456   # 256MB
    ttl_seconds: 3600         # 1 hour
    pool_size: 10             # Connection pool size
    timeout_ms: 1000          # Connection timeout

metrics:
  enabled: true
  port: 9090

logging:
  level: "info"
```

---

## Step 3: Start Services

```bash
docker compose up -d

# Wait for all services
sleep 10

# Check status
docker compose ps
```

---

## Step 4: Test Distributed Caching

```bash
# Request through instance 1 - cache miss, fetches from S3
curl http://localhost:8081/files/hello.txt
# Output: Hello from S3!

# Request through instance 2 - cache hit from Valkey!
curl http://localhost:8082/files/hello.txt
# Output: Hello from S3! (served from cache)
```

---

## Step 5: Verify Cache in Valkey

```bash
# Connect to Valkey
docker compose exec valkey valkey-cli

# List cached keys
KEYS *

# Check cache info
INFO memory

# Exit
exit
```

---

## Step 6: Monitor Cache Metrics

```bash
# Instance 1 metrics
curl -s http://localhost:8081/metrics | grep cache

# Instance 2 metrics
curl -s http://localhost:8082/metrics | grep cache
```

You should see cache hits on instance 2 after instance 1 cached the file.

---

## Redis Configuration Options

### Basic Connection

```yaml
cache:
  redis:
    enabled: true
    url: "redis://localhost:6379"
```

### With Authentication

```yaml
cache:
  redis:
    enabled: true
    url: "redis://:password@localhost:6379"
    # Or with username (Redis 6+)
    url: "redis://username:password@localhost:6379"
```

### With Database Selection

```yaml
cache:
  redis:
    enabled: true
    url: "redis://localhost:6379/0"  # Use database 0
```

### With TLS

```yaml
cache:
  redis:
    enabled: true
    url: "rediss://localhost:6379"  # Note: rediss:// (with s)
```

### Connection Pool

```yaml
cache:
  redis:
    enabled: true
    url: "redis://localhost:6379"
    pool_size: 20           # Max connections in pool
    timeout_ms: 2000        # Connection timeout
    retry_attempts: 3       # Retry on connection failure
    retry_delay_ms: 100     # Delay between retries
```

---

## Using Redis Instead of Valkey

Valkey is a Redis fork that's fully compatible. To use Redis instead:

```yaml
# docker-compose.yml
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru
```

The configuration remains the same - just change the service name.

---

## Step 7: High Availability with Redis Sentinel

For production, use Redis Sentinel:

```yaml
version: "3.8"

services:
  redis-master:
    image: redis:7-alpine
    command: redis-server --appendonly yes

  redis-slave:
    image: redis:7-alpine
    command: redis-server --replicaof redis-master 6379

  redis-sentinel:
    image: redis:7-alpine
    command: >
      redis-sentinel /etc/redis/sentinel.conf
    volumes:
      - ./sentinel.conf:/etc/redis/sentinel.conf
```

Sentinel configuration (`sentinel.conf`):

```
sentinel monitor mymaster redis-master 6379 2
sentinel down-after-milliseconds mymaster 5000
sentinel failover-timeout mymaster 60000
sentinel parallel-syncs mymaster 1
```

Yatagarasu configuration:

```yaml
cache:
  redis:
    enabled: true
    sentinel:
      master_name: "mymaster"
      nodes:
        - "redis-sentinel-1:26379"
        - "redis-sentinel-2:26379"
        - "redis-sentinel-3:26379"
```

---

## Step 8: Redis Cluster Support

For very large deployments:

```yaml
cache:
  redis:
    enabled: true
    cluster:
      nodes:
        - "redis-1:6379"
        - "redis-2:6379"
        - "redis-3:6379"
        - "redis-4:6379"
        - "redis-5:6379"
        - "redis-6:6379"
```

---

## Cache Flow with Valkey

```
Request arrives at Instance 2
          |
          v
+--------------------+
| Memory Cache (L1)  |  Check local memory first
+--------------------+
          |
      MISS |
          v
+--------------------+
| Valkey Cache (L2)  |  Check distributed cache
+--------------------+
          |
       HIT | (instance 1 cached it earlier)
          v
+--------------------+
| Return to Client   |
+--------------------+
```

---

## Best Practices

### 1. Size Appropriately

```yaml
cache:
  memory:
    max_capacity: 134217728  # 128MB per instance
  redis:
    max_capacity: 1073741824 # 1GB total shared
```

Memory cache should be smaller (per-instance), Redis larger (shared).

### 2. TTL Strategy

```yaml
cache:
  memory:
    ttl_seconds: 300    # 5 min - short for hot data
  redis:
    ttl_seconds: 3600   # 1 hour - longer for warm data
```

### 3. Handle Redis Failures

Yatagarasu gracefully handles Redis failures:
- Falls back to memory cache + S3
- Reconnects automatically when Redis recovers
- No request failures due to cache unavailability

### 4. Monitor Connection Pool

Watch for these metrics:
- Connection pool exhaustion
- Connection timeouts
- Retry counts

---

## Cleanup

```bash
docker compose down -v
cd .. && rm -rf valkey-tutorial
```

---

## Troubleshooting

### Connection Refused

```bash
# Check Valkey is running
docker compose logs valkey

# Test connectivity
docker compose exec yatagarasu-1 nc -zv valkey 6379
```

### High Latency

- Check network between instances and Valkey
- Increase `pool_size` if connections are exhausted
- Consider Redis Cluster for high throughput

### Memory Issues

- Set `maxmemory` in Valkey configuration
- Use appropriate eviction policy (`allkeys-lru` recommended)
- Monitor memory usage in Valkey

---

## Next Steps

- [High Availability](/yatagarasu/tutorials/high-availability/) - S3 replica failover
- [Configuration Reference](/yatagarasu/configuration/cache/) - All cache options
- [Deployment Guide](/yatagarasu/deployment/) - Production deployment
