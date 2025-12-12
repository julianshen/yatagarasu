---
title: Docker Quickstart
layout: default
parent: Getting Started
nav_order: 2
---

# Docker Quickstart

Get Yatagarasu running with Docker in under 5 minutes.
{: .fs-6 .fw-300 }

---

## Prerequisites

- Docker 20.10+ installed
- Access to an S3-compatible storage (AWS S3, MinIO, R2, etc.)
- S3 credentials (access key and secret key)

---

## Step 1: Create Configuration File

Create a `config.yaml` file with your S3 settings:

```yaml
server:
  address: "0.0.0.0:8080"
  threads: 4

buckets:
  - name: "my-assets"
    path_prefix: "/assets"
    s3:
      bucket: "your-bucket-name"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: false

metrics:
  enabled: true
  port: 9090

logging:
  level: "info"
  format: "json"
```

{: .note }
Environment variables like `${AWS_ACCESS_KEY_ID}` are substituted at startup. You can also hardcode values directly.

---

## Step 2: Run the Container

```bash
# Pull the latest image
docker pull ghcr.io/julianshen/yatagarasu:1.2.0

# Run with environment variables
docker run -d \
  --name yatagarasu \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -e AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE \
  -e AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

---

## Step 3: Verify It's Working

```bash
# Check container is running
docker ps

# Check health endpoint
curl http://localhost:8080/health
# {"status":"ok"}

# Check readiness (includes backend health)
curl http://localhost:8080/ready

# Try accessing a file from your S3 bucket
curl http://localhost:8080/assets/your-file.txt

# View metrics
curl http://localhost:9090/metrics
```

---

## Docker Run Options

### Port Mapping

| Port | Purpose |
|:-----|:--------|
| 8080 | Main proxy port |
| 9090 | Prometheus metrics |

### Volume Mounts

| Mount | Purpose |
|:------|:--------|
| `/etc/yatagarasu/config.yaml` | Configuration file |
| `/var/cache/yatagarasu` | Disk cache (optional) |
| `/var/log/yatagarasu` | Log files (optional) |

### Environment Variables

| Variable | Description |
|:---------|:------------|
| `AWS_ACCESS_KEY_ID` | S3 access key |
| `AWS_SECRET_ACCESS_KEY` | S3 secret key |
| `JWT_SECRET` | JWT signing secret |
| `RUST_LOG` | Log level (trace, debug, info, warn, error) |

---

## Production Configuration

For production deployments, add these recommended settings:

```yaml
server:
  address: "0.0.0.0:8080"
  threads: 4  # Match CPU cores

buckets:
  - name: "production-assets"
    path_prefix: "/assets"
    s3:
      bucket: "production-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"

cache:
  memory:
    max_capacity: 536870912  # 512MB
    ttl_seconds: 3600

metrics:
  enabled: true
  port: 9090

logging:
  level: "info"
  format: "json"
```

Run with production settings:

```bash
docker run -d \
  --name yatagarasu \
  --restart unless-stopped \
  --memory 1g \
  --cpus 2 \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
  -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
  -e JWT_SECRET=${JWT_SECRET} \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

---

## Using with Custom S3 Endpoints

For S3-compatible storage like MinIO, R2, or DigitalOcean Spaces:

```yaml
buckets:
  - name: "minio-bucket"
    path_prefix: "/files"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"  # Custom endpoint
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false
```

### MinIO Example

```bash
# Start MinIO
docker run -d \
  --name minio \
  -p 9000:9000 \
  -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# Create bucket and add a test file
docker exec minio mc alias set local http://localhost:9000 minioadmin minioadmin
docker exec minio mc mb local/test-bucket
docker exec minio sh -c 'echo "Hello!" | mc pipe local/test-bucket/hello.txt'

# Start Yatagarasu pointing to MinIO
docker run -d \
  --name yatagarasu \
  --link minio \
  -p 8080:8080 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -e MINIO_ACCESS_KEY=minioadmin \
  -e MINIO_SECRET_KEY=minioadmin \
  ghcr.io/julianshen/yatagarasu:1.2.0

# Test
curl http://localhost:8080/files/hello.txt
```

---

## Enabling Disk Cache

For persistent caching across container restarts:

```yaml
cache:
  memory:
    max_capacity: 268435456  # 256MB
    ttl_seconds: 3600
  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 5368709120  # 5GB
    ttl_seconds: 86400
```

```bash
# Create cache volume
docker volume create yatagarasu-cache

# Run with disk cache
docker run -d \
  --name yatagarasu \
  -p 8080:8080 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -v yatagarasu-cache:/var/cache/yatagarasu \
  -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
  -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

---

## Hot Reload Configuration

Reload configuration without restarting the container:

```bash
# Modify config.yaml, then send SIGHUP
docker kill --signal=HUP yatagarasu

# Verify reload in logs
docker logs yatagarasu --tail 20
```

---

## Graceful Shutdown

Yatagarasu handles graceful shutdown automatically:

```bash
# Stop with graceful shutdown (SIGTERM)
docker stop yatagarasu

# Or explicitly send SIGTERM
docker kill --signal=TERM yatagarasu
```

The proxy will:
1. Stop accepting new connections
2. Complete all in-flight requests
3. Exit cleanly

---

## View Logs

```bash
# Follow logs
docker logs -f yatagarasu

# Last 100 lines
docker logs --tail 100 yatagarasu

# With timestamps
docker logs -t yatagarasu
```

---

## Troubleshooting

### Container Exits Immediately

Check logs for errors:

```bash
docker logs yatagarasu
```

Common causes:
- Invalid YAML syntax in config
- Missing required environment variables
- Port already in use

### Connection Refused

```bash
# Check container is running
docker ps -a

# Check port binding
docker port yatagarasu

# Test from inside container
docker exec yatagarasu curl localhost:8080/health
```

### S3 Access Denied

```bash
# Verify environment variables are set
docker exec yatagarasu env | grep AWS

# Check S3 configuration in logs
docker logs yatagarasu | grep -i s3
```

---

## Next Steps

- [Docker Compose](/yatagarasu/getting-started/docker-compose/) - Full development environment
- [Kubernetes](/yatagarasu/deployment/kubernetes/) - Production Kubernetes deployment
- [Configuration](/yatagarasu/configuration/) - All configuration options
