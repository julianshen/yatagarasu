---
title: Getting Started
layout: default
nav_order: 2
has_children: true
permalink: /getting-started/
---

# Getting Started

Get Yatagarasu up and running in minutes with this quick start guide.
{: .fs-6 .fw-300 }

---

## Prerequisites

Before you begin, ensure you have:

- Access to an S3-compatible storage service (AWS S3, MinIO, R2, etc.)
- Docker installed (for the quickest setup)
- OR Rust 1.70+ (for building from source)

## Quick Start Options

Choose your preferred installation method:

| Method | Time | Best For |
|:-------|:-----|:---------|
| [Docker](/yatagarasu/getting-started/docker-quickstart/) | 2 min | Quick testing, production |
| [Docker Compose](/yatagarasu/getting-started/docker-compose/) | 5 min | Development with MinIO |
| [From Source](/yatagarasu/getting-started/installation/) | 10 min | Development, customization |

## Fastest Path: Docker with MinIO

The quickest way to try Yatagarasu is using Docker Compose with MinIO (S3-compatible storage):

```bash
# Clone the repository
git clone https://github.com/julianshen/yatagarasu.git
cd yatagarasu

# Start Yatagarasu + MinIO
docker compose up -d

# Test it works
curl http://localhost:8080/public/hello.txt
# Output: Hello, World!

# Check health
curl http://localhost:8080/health
# Output: {"status":"ok"}
```

That's it! You now have a working S3 proxy. The default setup includes:
- Yatagarasu proxy on port 8080
- MinIO (S3) on port 9000 (console on 9001)
- A pre-created `public` bucket with a test file

## Next Steps

Once you have Yatagarasu running:

1. **[Configure Your Buckets](/yatagarasu/configuration/buckets/)** - Map your S3 buckets to URL paths
2. **[Add Authentication](/yatagarasu/tutorials/jwt-authentication/)** - Protect buckets with JWT
3. **[Enable Caching](/yatagarasu/configuration/cache/)** - Improve performance with multi-tier caching
4. **[Deploy to Production](/yatagarasu/deployment/)** - Kubernetes, HA, and monitoring

## Architecture Overview

Here's what happens when a request hits Yatagarasu:

```
Client Request (GET /assets/image.png)
         |
         v
  +------+-------+
  | Path Router  |  Match "/assets" -> bucket config
  +------+-------+
         |
         v
  +------+-------+
  | Auth Check   |  Validate JWT if enabled
  +------+-------+
         |
         v
  +------+-------+
  | Cache Lookup |  Check memory -> Redis -> disk
  +------+-------+
         |
    +----+----+
    |         |
  HIT       MISS
    |         |
    v         v
  Return   +--+---+
  Cached   | S3   |  Fetch from S3
           +--+---+
              |
              v
         +----+----+
         | Stream  |  Zero-copy response
         | to      |
         | Client  |
         +---------+
```

## Feature Comparison

| Feature | Basic Setup | Production Setup |
|:--------|:------------|:-----------------|
| S3 Proxy | Yes | Yes |
| Multi-bucket routing | Yes | Yes |
| JWT Authentication | Optional | Recommended |
| Memory Cache | Optional | Recommended |
| Redis/Valkey Cache | No | Recommended |
| HA with Replicas | No | Recommended |
| Prometheus Metrics | Optional | Recommended |
| OpenTelemetry Tracing | No | Optional |

## Configuration File

Yatagarasu uses YAML configuration. Here's a minimal example:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "my-bucket"
    path_prefix: "/files"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: false
```

Environment variables (like `${AWS_ACCESS_KEY_ID}`) are automatically substituted at startup.

## Common Issues

### Connection Refused

If you get "connection refused" when accessing the proxy:

1. Check that Yatagarasu is running: `docker ps` or `ps aux | grep yatagarasu`
2. Verify the port binding: default is 8080
3. Check the logs: `docker logs yatagarasu` or check stdout

### S3 Access Denied

If you see "Access Denied" errors:

1. Verify your S3 credentials are correct
2. Check the bucket name and region match your S3 setup
3. Ensure the credentials have read access to the bucket

### JWT Authentication Failed

If JWT validation fails:

1. Verify the JWT secret matches
2. Check the token hasn't expired
3. Ensure the token is in the expected location (header/query)

See the [Troubleshooting](/yatagarasu/operations/troubleshooting/) guide for more solutions.
