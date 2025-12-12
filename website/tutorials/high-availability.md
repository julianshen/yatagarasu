---
title: High Availability
layout: default
parent: Tutorials
nav_order: 7
---

# High Availability Setup

Configure automatic S3 failover with replica buckets.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Configure multiple S3 replicas per bucket
- Set up automatic failover with health checking
- Configure circuit breakers for fast failure detection
- Test failover scenarios

## Prerequisites

- Docker installed
- Understanding of basic Yatagarasu configuration

---

## Architecture Overview

```
                    +------------------+
                    |   Primary S3     | Priority: 1
                    |   (us-west-2)    |
                    +------------------+
                           ^
                           | Active
                           |
Client --> Yatagarasu -----+
                           |
                           | Standby (failover)
                           v
                    +------------------+
                    |   Backup S3      | Priority: 2
                    |   (us-east-1)    |
                    +------------------+
                           |
                           | Standby (last resort)
                           v
                    +------------------+
                    |   DR S3          | Priority: 3
                    |   (eu-west-1)    |
                    +------------------+
```

---

## Step 1: Setup

Create a tutorial directory:

```bash
mkdir ha-tutorial && cd ha-tutorial
```

Create `docker-compose.yml` with multiple MinIO instances:

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
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
    depends_on:
      minio-primary:
        condition: service_healthy
      minio-backup:
        condition: service_healthy

  # Primary MinIO instance
  minio-primary:
    image: minio/minio:latest
    ports:
      - "9001:9000"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 3

  # Backup MinIO instance
  minio-backup:
    image: minio/minio:latest
    ports:
      - "9002:9000"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 3

  # Initialize both buckets
  minio-init:
    image: minio/mc:latest
    depends_on:
      minio-primary:
        condition: service_healthy
      minio-backup:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      # Setup primary
      mc alias set primary http://minio-primary:9000 minioadmin minioadmin;
      mc mb primary/data-bucket --ignore-existing;
      echo 'Primary data!' | mc pipe primary/data-bucket/test.txt;

      # Setup backup
      mc alias set backup http://minio-backup:9000 minioadmin minioadmin;
      mc mb backup/data-bucket --ignore-existing;
      echo 'Backup data!' | mc pipe backup/data-bucket/test.txt;

      echo 'HA buckets initialized!';
      "
```

---

## Step 2: Configure HA Buckets

Create `config.yaml`:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "ha-data"
    path_prefix: "/data"
    s3:
      bucket: "data-bucket"
      region: "us-east-1"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"

      # HA Replicas - tried in priority order
      replicas:
        - name: "primary"
          endpoint: "http://minio-primary:9000"
          priority: 1          # Highest priority (tried first)
          timeout_seconds: 5   # Per-request timeout

        - name: "backup"
          endpoint: "http://minio-backup:9000"
          priority: 2          # Fallback
          timeout_seconds: 10  # Longer timeout for backup

      # Circuit breaker settings
      circuit_breaker:
        failure_threshold: 3     # Open after 3 failures
        success_threshold: 2     # Close after 2 successes
        timeout_seconds: 30      # Reset after 30s
        half_open_requests: 1    # Allow 1 request when half-open

    auth:
      enabled: false

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
sleep 10
docker compose ps
```

---

## Step 4: Test Normal Operation

```bash
# Request goes to primary (healthy)
curl http://localhost:8080/data/test.txt
# Output: Primary data!
```

---

## Step 5: Simulate Primary Failure

```bash
# Stop the primary MinIO
docker compose stop minio-primary

# Wait for circuit breaker to open (after failures)
sleep 5

# Request now goes to backup
curl http://localhost:8080/data/test.txt
# Output: Backup data!
```

Check the logs to see failover:

```bash
docker compose logs yatagarasu --tail 20
# You should see: "Replica 'primary' failed, trying 'backup'"
```

---

## Step 6: Test Recovery

```bash
# Restart primary
docker compose start minio-primary
sleep 10

# Circuit breaker recovers, primary becomes active again
curl http://localhost:8080/data/test.txt
# Output: Primary data!
```

---

## Step 7: Monitor Replica Health

```bash
# Check metrics
curl -s http://localhost:9090/metrics | grep replica

# Key metrics:
# yatagarasu_replica_health{replica="primary"} 1  (healthy)
# yatagarasu_replica_health{replica="backup"} 1   (healthy)
# yatagarasu_replica_requests_total{replica="primary"} 10
# yatagarasu_replica_failures_total{replica="primary"} 3
```

---

## Circuit Breaker States

```
         +----------------+
         |     CLOSED     |  Normal operation
         | (all requests  |  Requests go to replica
         |   allowed)     |
         +-------+--------+
                 |
                 | failure_threshold failures
                 v
         +-------+--------+
         |      OPEN      |  Replica is marked down
         | (all requests  |  Skip this replica
         |   rejected)    |
         +-------+--------+
                 |
                 | timeout_seconds elapsed
                 v
         +-------+--------+
         |   HALF-OPEN    |  Testing recovery
         | (limited       |  Allow half_open_requests
         |  requests)     |
         +-------+--------+
                 |
        success  |  failure
           +-----+-----+
           |           |
           v           v
       CLOSED        OPEN
```

---

## Configuration Reference

### Replica Configuration

```yaml
replicas:
  - name: "primary"           # Identifier for logs/metrics
    endpoint: "http://..."    # S3 endpoint URL
    bucket: "bucket-name"     # Optional: override bucket name
    region: "us-west-2"       # Optional: override region
    priority: 1               # Lower = higher priority
    timeout_seconds: 5        # Request timeout
    weight: 100               # For weighted load balancing (future)
```

### Circuit Breaker Configuration

```yaml
circuit_breaker:
  failure_threshold: 5        # Failures before opening
  success_threshold: 2        # Successes to close
  timeout_seconds: 60         # Time before trying again
  half_open_requests: 1       # Test requests in half-open
```

---

## Health Checking

Yatagarasu performs active health checking:

```yaml
health_check:
  enabled: true
  interval_seconds: 10       # Check every 10 seconds
  timeout_seconds: 5         # Health check timeout
  path: "/"                  # Path to check (HEAD request)
```

---

## Multi-Region Setup

Example with real AWS regions:

```yaml
buckets:
  - name: "global-assets"
    path_prefix: "/assets"
    s3:
      bucket: "global-assets"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"

      replicas:
        # Primary in US West
        - name: "us-west"
          region: "us-west-2"
          priority: 1
          timeout_seconds: 5

        # Backup in US East
        - name: "us-east"
          region: "us-east-1"
          priority: 2
          timeout_seconds: 8

        # DR in Europe
        - name: "eu-west"
          region: "eu-west-1"
          priority: 3
          timeout_seconds: 15

      circuit_breaker:
        failure_threshold: 3
        timeout_seconds: 60
```

---

## Mixed Providers

Use different S3 providers as replicas:

```yaml
replicas:
  # Primary: AWS S3
  - name: "aws-primary"
    region: "us-east-1"
    priority: 1

  # Backup: Cloudflare R2
  - name: "r2-backup"
    endpoint: "https://xxx.r2.cloudflarestorage.com"
    access_key: "${R2_ACCESS_KEY}"
    secret_key: "${R2_SECRET_KEY}"
    priority: 2

  # DR: MinIO on-premises
  - name: "onprem-dr"
    endpoint: "https://minio.internal:9000"
    access_key: "${MINIO_ACCESS_KEY}"
    secret_key: "${MINIO_SECRET_KEY}"
    priority: 3
```

---

## Best Practices

### 1. Geographic Distribution

Place replicas in different regions/zones for true HA.

### 2. Priority Assignment

- Priority 1: Closest/fastest replica
- Priority 2-3: Geographic fallbacks
- Highest priority: DR/cold storage

### 3. Timeout Configuration

```yaml
replicas:
  - name: "local"
    timeout_seconds: 5      # Fast timeout for local

  - name: "cross-region"
    timeout_seconds: 15     # Longer for cross-region
```

### 4. Circuit Breaker Tuning

- Production: Higher thresholds to avoid flapping
- Development: Lower thresholds for faster feedback

```yaml
# Production settings
circuit_breaker:
  failure_threshold: 10
  success_threshold: 5
  timeout_seconds: 120

# Development settings
circuit_breaker:
  failure_threshold: 3
  success_threshold: 1
  timeout_seconds: 30
```

---

## Cleanup

```bash
docker compose down -v
cd .. && rm -rf ha-tutorial
```

---

## Troubleshooting

### All Replicas Failing

Check:
- Network connectivity to all endpoints
- Credentials are correct for all replicas
- Bucket names exist on all replicas

### Slow Failover

- Reduce `timeout_seconds` on replicas
- Lower `failure_threshold` in circuit breaker

### Flapping Between Replicas

- Increase `failure_threshold`
- Increase `timeout_seconds` in circuit breaker
- Add health checking with `interval_seconds`

---

## Next Steps

- [Production Monitoring](/yatagarasu/tutorials/monitoring/) - Set up Prometheus and Grafana
- [Deployment Guide](/yatagarasu/deployment/high-availability/) - Full HA deployment
- [Configuration Reference](/yatagarasu/configuration/) - All options
