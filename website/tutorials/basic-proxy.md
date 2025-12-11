---
title: Basic Proxy Setup
layout: default
parent: Tutorials
nav_order: 1
---

# Basic Proxy Setup

Set up a simple S3 proxy in 10 minutes.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Configure Yatagarasu to proxy requests to S3
- Test the proxy with curl commands
- Understand the basic request flow

## Prerequisites

- Docker installed
- S3 bucket with some files (or we'll use MinIO)

---

## Step 1: Set Up S3 Storage

If you already have an S3 bucket, skip to Step 2. Otherwise, let's set up MinIO:

```bash
# Create a directory for this tutorial
mkdir yatagarasu-tutorial && cd yatagarasu-tutorial

# Start MinIO
docker run -d \
  --name minio \
  -p 9000:9000 \
  -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"
```

Wait for MinIO to start, then create a bucket and upload a test file:

```bash
# Install MinIO client (mc) or use the web console at http://localhost:9001
docker exec minio mc alias set local http://localhost:9000 minioadmin minioadmin

# Create a bucket
docker exec minio mc mb local/my-bucket

# Add a test file
docker exec minio sh -c 'echo "Hello from Yatagarasu!" > /tmp/hello.txt'
docker exec minio mc cp /tmp/hello.txt local/my-bucket/hello.txt

# Verify the file exists
docker exec minio mc ls local/my-bucket/
```

---

## Step 2: Create Configuration

Create a `config.yaml` file:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "my-bucket"
    path_prefix: "/files"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      endpoint: "http://host.docker.internal:9000"  # MinIO endpoint
      access_key: "minioadmin"
      secret_key: "minioadmin"
    auth:
      enabled: false  # No authentication for this tutorial

metrics:
  enabled: true
  port: 9090

logging:
  level: "info"
```

{: .note }
We're using `host.docker.internal` because Yatagarasu runs in Docker and needs to reach MinIO on the host. On Linux, you may need to use `--network host` or the actual IP.

---

## Step 3: Start Yatagarasu

```bash
# Start Yatagarasu
docker run -d \
  --name yatagarasu \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  --add-host=host.docker.internal:host-gateway \
  ghcr.io/julianshen/yatagarasu:1.2.0

# Check it started successfully
docker logs yatagarasu
```

---

## Step 4: Test the Proxy

```bash
# Health check
curl http://localhost:8080/health
# {"status":"ok"}

# Fetch the file through the proxy
curl http://localhost:8080/files/hello.txt
# Hello from Yatagarasu!

# Get file metadata (HEAD request)
curl -I http://localhost:8080/files/hello.txt
# HTTP/1.1 200 OK
# Content-Type: text/plain
# Content-Length: 23
# ...
```

---

## Understanding the Request Flow

When you request `http://localhost:8080/files/hello.txt`:

```
1. Client sends GET /files/hello.txt
            |
            v
2. Yatagarasu matches path prefix "/files"
   -> Routes to "my-bucket" configuration
            |
            v
3. Auth check (disabled in this tutorial)
            |
            v
4. Yatagarasu builds S3 request:
   GET http://minio:9000/my-bucket/hello.txt
   (with AWS SigV4 signature)
            |
            v
5. S3/MinIO returns the file
            |
            v
6. Yatagarasu streams response to client
```

---

## Step 5: View Metrics

Yatagarasu exposes Prometheus metrics:

```bash
curl http://localhost:9090/metrics
```

Key metrics to look for:

```
# Total requests
yatagarasu_requests_total{...}

# Request duration
yatagarasu_request_duration_seconds{...}

# S3 backend requests
yatagarasu_s3_requests_total{...}
```

---

## Step 6: Add More Files

Let's add more files to test with:

```bash
# Create some test files
docker exec minio sh -c 'echo "Image content" > /tmp/logo.png'
docker exec minio mc cp /tmp/logo.png local/my-bucket/images/logo.png

docker exec minio sh -c 'echo "CSS content" > /tmp/style.css'
docker exec minio mc cp /tmp/style.css local/my-bucket/css/style.css

# Fetch through proxy
curl http://localhost:8080/files/images/logo.png
curl http://localhost:8080/files/css/style.css
```

---

## Step 7: Test Error Handling

```bash
# Request non-existent file
curl -i http://localhost:8080/files/does-not-exist.txt
# HTTP/1.1 404 Not Found

# Request non-existent path prefix
curl -i http://localhost:8080/unknown/file.txt
# HTTP/1.1 404 Not Found
```

---

## Cleanup

```bash
# Stop and remove containers
docker stop yatagarasu minio
docker rm yatagarasu minio

# Remove tutorial directory
cd .. && rm -rf yatagarasu-tutorial
```

---

## Configuration Explained

Let's break down the configuration:

```yaml
server:
  address: "0.0.0.0:8080"  # Listen on all interfaces, port 8080
```

**Server section**: Defines where Yatagarasu listens for incoming requests.

```yaml
buckets:
  - name: "my-bucket"           # Identifier (used in logs/metrics)
    path_prefix: "/files"       # URL path that routes to this bucket
```

**Buckets section**: Maps URL paths to S3 buckets. Any request starting with `/files/` is routed to this bucket.

```yaml
    s3:
      bucket: "my-bucket"       # Actual S3 bucket name
      region: "us-east-1"       # AWS region (required even for MinIO)
      endpoint: "http://..."    # Custom endpoint (for non-AWS S3)
      access_key: "..."         # S3 credentials
      secret_key: "..."
```

**S3 section**: Configures the connection to S3. The `endpoint` is optional for AWS S3 but required for S3-compatible storage.

```yaml
    auth:
      enabled: false            # No authentication required
```

**Auth section**: Controls whether JWT authentication is required. We'll cover this in the next tutorial.

---

## Next Steps

Now that you have a basic proxy working:

1. **[Add JWT Authentication](/yatagarasu/tutorials/jwt-authentication/)** - Protect your bucket with tokens
2. **[Multi-Bucket Routing](/yatagarasu/tutorials/multi-bucket/)** - Route to multiple buckets
3. **[Enable Caching](/yatagarasu/tutorials/caching/)** - Improve performance

---

## Troubleshooting

### "Connection refused" error

Make sure MinIO is running:
```bash
docker ps | grep minio
```

### "Access Denied" from S3

Check your credentials:
```bash
docker exec minio mc admin info local
```

### Yatagarasu won't start

Check the logs:
```bash
docker logs yatagarasu
```

Common issues:
- Invalid YAML syntax
- Missing required fields
- Port already in use
