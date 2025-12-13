---
title: Multi-Bucket Routing
layout: default
parent: Tutorials
nav_order: 2
---

# Multi-Bucket Routing

Route different URL paths to different S3 buckets with isolated credentials.
{: .fs-6 .fw-300 }

---

## What You'll Learn

- Configure multiple buckets with different path prefixes
- Use different credentials per bucket
- Mix public and private buckets
- Understand longest-prefix matching

## Prerequisites

- Docker installed
- Completed the [Basic Proxy Setup](/yatagarasu/tutorials/basic-proxy/) tutorial

---

## Step 1: Setup

Create a directory for this tutorial:

```bash
mkdir multi-bucket-tutorial && cd multi-bucket-tutorial
```

Create `docker-compose.yml`:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
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

      # Create buckets for different purposes
      mc mb local/images-bucket --ignore-existing;
      mc mb local/videos-bucket --ignore-existing;
      mc mb local/documents-bucket --ignore-existing;
      mc mb local/api-assets-bucket --ignore-existing;

      # Add sample files
      echo 'Image content' | mc pipe local/images-bucket/logo.png;
      echo 'Video content' | mc pipe local/videos-bucket/intro.mp4;
      echo 'Document content' | mc pipe local/documents-bucket/readme.pdf;
      echo 'API asset' | mc pipe local/api-assets-bucket/schema.json;
      echo 'Nested API asset' | mc pipe local/api-assets-bucket/v2/schema.json;

      echo 'Buckets initialized!';
      "
```

---

## Step 2: Configure Multiple Buckets

Create `config.yaml`:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  # Images bucket
  - name: "images"
    path_prefix: "/images"
    s3:
      bucket: "images-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

  # Videos bucket
  - name: "videos"
    path_prefix: "/videos"
    s3:
      bucket: "videos-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

  # Documents bucket
  - name: "documents"
    path_prefix: "/docs"
    s3:
      bucket: "documents-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

  # API assets bucket (more specific path)
  - name: "api-assets"
    path_prefix: "/api/assets"
    s3:
      bucket: "api-assets-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

logging:
  level: "info"
```

---

## Step 3: Start Services

```bash
docker compose up -d

# Wait for initialization
sleep 5

# Check services
docker compose ps
```

---

## Step 4: Test Different Buckets

```bash
# Access images bucket
curl http://localhost:8080/images/logo.png
# Output: Image content

# Access videos bucket
curl http://localhost:8080/videos/intro.mp4
# Output: Video content

# Access documents bucket
curl http://localhost:8080/docs/readme.pdf
# Output: Document content

# Access API assets bucket
curl http://localhost:8080/api/assets/schema.json
# Output: API asset
```

---

## Step 5: Understand Longest-Prefix Matching

Yatagarasu uses **longest-prefix matching** to route requests. The most specific path wins.

```yaml
buckets:
  - name: "api"
    path_prefix: "/api"           # Less specific

  - name: "api-assets"
    path_prefix: "/api/assets"    # More specific
```

| Request Path | Matched Bucket |
|:-------------|:---------------|
| `/api/users` | api |
| `/api/assets/image.png` | api-assets |
| `/api/assets/v2/schema.json` | api-assets |

```bash
# This goes to api-assets bucket (longer prefix match)
curl http://localhost:8080/api/assets/v2/schema.json
# Output: Nested API asset
```

---

## Step 6: Mix Public and Private Buckets

Update `config.yaml` to add authentication to some buckets:

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  # Public images - no auth
  - name: "public-images"
    path_prefix: "/public/images"
    s3:
      bucket: "images-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

  # Private documents - requires JWT
  - name: "private-docs"
    path_prefix: "/private/docs"
    s3:
      bucket: "documents-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "my-jwt-secret"
        algorithm: "HS256"
        token_sources:
          - type: "bearer"
```

Restart and test:

```bash
docker compose restart yatagarasu

# Public access works
curl http://localhost:8080/public/images/logo.png
# Output: Image content

# Private access requires token
curl -i http://localhost:8080/private/docs/readme.pdf
# HTTP/1.1 401 Unauthorized
```

---

## Step 7: Different Credentials Per Bucket

In production, you might have different S3 accounts for different buckets:

```yaml
buckets:
  # Production assets (AWS account 1)
  - name: "production-assets"
    path_prefix: "/prod"
    s3:
      bucket: "prod-assets"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY_PROD}"
      secret_key: "${AWS_SECRET_KEY_PROD}"
    auth:
      enabled: false

  # Development assets (AWS account 2)
  - name: "dev-assets"
    path_prefix: "/dev"
    s3:
      bucket: "dev-assets"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_DEV}"
      secret_key: "${AWS_SECRET_KEY_DEV}"
    auth:
      enabled: false

  # Partner assets (different provider - R2)
  - name: "partner-assets"
    path_prefix: "/partner"
    s3:
      bucket: "partner-bucket"
      region: "auto"
      endpoint: "https://xxx.r2.cloudflarestorage.com"
      access_key: "${R2_ACCESS_KEY}"
      secret_key: "${R2_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${PARTNER_JWT_SECRET}"
```

Each bucket is completely isolated - different credentials, different regions, even different S3 providers.

---

## Step 8: Organize by Tenant

Multi-tenancy pattern:

```yaml
buckets:
  # Tenant A
  - name: "tenant-a"
    path_prefix: "/tenants/a"
    s3:
      bucket: "tenant-a-bucket"
      region: "us-east-1"
      access_key: "${TENANT_A_ACCESS_KEY}"
      secret_key: "${TENANT_A_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${TENANT_A_JWT_SECRET}"
        claims_verification:
          - claim: "tenant"
            operator: "equals"
            value: "tenant-a"

  # Tenant B
  - name: "tenant-b"
    path_prefix: "/tenants/b"
    s3:
      bucket: "tenant-b-bucket"
      region: "eu-west-1"
      access_key: "${TENANT_B_ACCESS_KEY}"
      secret_key: "${TENANT_B_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${TENANT_B_JWT_SECRET}"
        claims_verification:
          - claim: "tenant"
            operator: "equals"
            value: "tenant-b"
```

---

## Routing Summary

| Path Pattern | Matched By |
|:-------------|:-----------|
| `/images/*` | First bucket with `path_prefix: "/images"` |
| `/api/assets/*` | Longest match wins over `/api` |
| `/api/v2/*` | Falls back to `/api` if no `/api/v2` defined |
| `/unknown/*` | 404 Not Found |

---

## Cleanup

```bash
docker compose down -v
cd .. && rm -rf multi-bucket-tutorial
```

---

## Best Practices

1. **Use descriptive path prefixes** - `/assets`, `/media`, `/api` are clear
2. **Group by access pattern** - Public vs private, by tenant, by region
3. **Isolate credentials** - Different buckets should have different credentials
4. **Document your routes** - Keep a map of path prefixes to buckets

---

## Next Steps

- [JWT Authentication](/yatagarasu/tutorials/jwt-authentication/) - Add authentication
- [Caching Setup](/yatagarasu/tutorials/caching/) - Enable caching per bucket
- [High Availability](/yatagarasu/tutorials/high-availability/) - Add replica failover
