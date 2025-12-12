---
title: Buckets Configuration
layout: default
parent: Configuration
nav_order: 2
---

# Buckets Configuration

Define S3 bucket mappings and routing rules.
{: .fs-6 .fw-300 }

---

## Basic Configuration

```yaml
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

---

## Bucket Options

### name

Identifier for this bucket configuration (used in logs and metrics).

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes |

```yaml
name: "production-assets"
```

---

### path_prefix

URL path prefix that routes to this bucket.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes |

```yaml
path_prefix: "/assets"
# Request: GET /assets/image.png -> S3: GET /image.png
```

Routing uses longest-prefix matching:
- `/api/assets` matches before `/api`
- `/images/thumbnails` matches before `/images`

---

## S3 Options

### s3.bucket

S3 bucket name.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes |

```yaml
s3:
  bucket: "my-bucket-name"
```

---

### s3.region

AWS region for the bucket.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes |

```yaml
s3:
  region: "us-east-1"
```

Common regions:
- `us-east-1`, `us-west-2` (AWS US)
- `eu-west-1`, `eu-central-1` (AWS Europe)
- `ap-northeast-1` (AWS Tokyo)
- `auto` (Cloudflare R2)

---

### s3.endpoint

Custom S3 endpoint for non-AWS providers.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | No |
| **Default** | AWS S3 endpoint |

```yaml
# MinIO
s3:
  endpoint: "http://minio:9000"

# Cloudflare R2
s3:
  endpoint: "https://account-id.r2.cloudflarestorage.com"

# DigitalOcean Spaces
s3:
  endpoint: "https://nyc3.digitaloceanspaces.com"

# Backblaze B2
s3:
  endpoint: "https://s3.us-west-002.backblazeb2.com"
```

---

### s3.access_key / s3.secret_key

S3 credentials.

| | |
|:--|:--|
| **Type** | `string` |
| **Required** | Yes |

```yaml
s3:
  access_key: "${AWS_ACCESS_KEY_ID}"
  secret_key: "${AWS_SECRET_ACCESS_KEY}"
```

{: .warning }
Never commit credentials to version control. Always use environment variables.

---

### s3.path_style

Use path-style URLs instead of virtual-hosted style.

| | |
|:--|:--|
| **Type** | `boolean` |
| **Required** | No |
| **Default** | `false` |

```yaml
s3:
  path_style: true  # http://endpoint/bucket/key
  # vs false:       # http://bucket.endpoint/key
```

Required for MinIO and some S3-compatible services.

---

## Replica Configuration

### s3.replicas

Define multiple S3 backends for high availability.

```yaml
s3:
  bucket: "main-bucket"
  region: "us-west-2"
  access_key: "${AWS_ACCESS_KEY_ID}"
  secret_key: "${AWS_SECRET_ACCESS_KEY}"

  replicas:
    - name: "primary"
      region: "us-west-2"
      priority: 1
      timeout_seconds: 5

    - name: "backup"
      region: "us-east-1"
      priority: 2
      timeout_seconds: 10

    - name: "dr"
      endpoint: "https://r2.example.com"
      access_key: "${R2_ACCESS_KEY}"
      secret_key: "${R2_SECRET_KEY}"
      priority: 3
      timeout_seconds: 15
```

### Replica Options

| Option | Type | Description |
|:-------|:-----|:------------|
| `name` | string | Identifier for metrics/logs |
| `endpoint` | string | Override S3 endpoint |
| `bucket` | string | Override bucket name |
| `region` | string | Override region |
| `access_key` | string | Override access key |
| `secret_key` | string | Override secret key |
| `priority` | integer | Lower = tried first |
| `timeout_seconds` | integer | Request timeout |

---

## Circuit Breaker Configuration

### s3.circuit_breaker

Configure automatic failure detection and recovery.

```yaml
s3:
  circuit_breaker:
    failure_threshold: 5      # Failures before opening
    success_threshold: 2      # Successes to close
    timeout_seconds: 60       # Time before retrying
    half_open_requests: 1     # Test requests when half-open
```

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `failure_threshold` | integer | 5 | Consecutive failures to open |
| `success_threshold` | integer | 2 | Successes to close |
| `timeout_seconds` | integer | 60 | Recovery timeout |
| `half_open_requests` | integer | 1 | Requests to test recovery |

---

## Complete Examples

### Public Bucket

```yaml
buckets:
  - name: "public-assets"
    path_prefix: "/assets"
    s3:
      bucket: "public-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: false
```

### Private Bucket with JWT

```yaml
buckets:
  - name: "private-data"
    path_prefix: "/private"
    s3:
      bucket: "private-bucket"
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
```

### MinIO Bucket

```yaml
buckets:
  - name: "minio-data"
    path_prefix: "/data"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
      path_style: true
    auth:
      enabled: false
```

### Multi-Region HA Bucket

```yaml
buckets:
  - name: "global-assets"
    path_prefix: "/global"
    s3:
      bucket: "assets"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
      replicas:
        - name: "us-west"
          region: "us-west-2"
          priority: 1
          timeout_seconds: 5
        - name: "us-east"
          region: "us-east-1"
          priority: 2
          timeout_seconds: 8
        - name: "eu-west"
          region: "eu-west-1"
          priority: 3
          timeout_seconds: 15
      circuit_breaker:
        failure_threshold: 3
        timeout_seconds: 30
    auth:
      enabled: false
```

---

## Path Matching Rules

Requests are matched to buckets using longest-prefix matching:

| Request | Bucket Config | Matched |
|:--------|:--------------|:--------|
| `/assets/img.png` | `/assets` | Yes |
| `/assets/css/style.css` | `/assets` | Yes |
| `/api/v1/data` | `/api` | Yes |
| `/api/v2/data` | `/api/v2` | Yes (more specific) |
| `/unknown/file.txt` | - | 404 Not Found |

---

## See Also

- [Authentication](/yatagarasu/configuration/authentication/) - JWT configuration
- [Authorization](/yatagarasu/configuration/authorization/) - OPA/OpenFGA
- [High Availability Tutorial](/yatagarasu/tutorials/high-availability/) - HA setup guide
