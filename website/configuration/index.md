---
title: Configuration
layout: default
nav_order: 4
has_children: true
permalink: /configuration/
---

# Configuration Reference

Complete reference for all Yatagarasu configuration options.
{: .fs-6 .fw-300 }

---

## Configuration File

Yatagarasu uses YAML configuration files. The configuration file location can be specified via:

1. `--config` command line flag
2. `/etc/yatagarasu/config.yaml` (default)
3. `./config.yaml` (current directory)

## Environment Variable Substitution

Configuration values can reference environment variables:

```yaml
s3:
  access_key: "${AWS_ACCESS_KEY_ID}"
  secret_key: "${AWS_SECRET_ACCESS_KEY}"
```

Variables are substituted at startup. Missing variables cause a startup error.

## Configuration Structure

```yaml
# Server configuration
server:
  address: "0.0.0.0:8080"
  threads: 4

# Bucket definitions (array)
buckets:
  - name: "bucket-name"
    path_prefix: "/path"
    s3: { ... }
    auth: { ... }
    authorization: { ... }

# Cache configuration
cache:
  memory: { ... }
  disk: { ... }
  redis: { ... }

# Metrics configuration
metrics:
  enabled: true
  port: 9090

# Logging configuration
logging:
  level: "info"
  format: "json"

# Rate limiting
rate_limiting:
  enabled: true
  requests_per_second: 1000

# Observability
observability:
  tracing: { ... }
```

---

## Quick Reference

| Section | Purpose |
|:--------|:--------|
| [Server](/yatagarasu/configuration/server/) | Listen address, threads |
| [Buckets](/yatagarasu/configuration/buckets/) | S3 bucket mappings |
| [Authentication](/yatagarasu/configuration/authentication/) | JWT configuration |
| [Authorization](/yatagarasu/configuration/authorization/) | OPA/OpenFGA |
| [Cache](/yatagarasu/configuration/cache/) | Memory, disk, Redis cache |
| [Metrics](/yatagarasu/configuration/metrics/) | Prometheus metrics |
| [Logging](/yatagarasu/configuration/logging/) | Log level and format |
| [Rate Limiting](/yatagarasu/configuration/rate-limiting/) | Request throttling |

---

## Complete Example

```yaml
server:
  address: "0.0.0.0:8080"
  threads: 4

buckets:
  # Public assets - no authentication
  - name: "public-assets"
    path_prefix: "/assets"
    s3:
      bucket: "public-assets-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: false

  # Private API - JWT required
  - name: "private-api"
    path_prefix: "/api"
    s3:
      bucket: "api-data-bucket"
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
          - type: "query"
            name: "token"
        claims_verification:
          - claim: "role"
            operator: "in"
            value: ["admin", "api-user"]

  # HA bucket with replicas
  - name: "ha-media"
    path_prefix: "/media"
    s3:
      bucket: "media-bucket"
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
      circuit_breaker:
        failure_threshold: 5
        timeout_seconds: 60
    auth:
      enabled: false

cache:
  memory:
    max_capacity: 536870912      # 512MB
    max_file_size: 10485760      # 10MB
    ttl_seconds: 3600

  disk:
    enabled: true
    path: "/var/cache/yatagarasu"
    max_capacity: 10737418240    # 10GB
    ttl_seconds: 86400

  redis:
    enabled: true
    url: "redis://redis:6379"
    max_capacity: 1073741824     # 1GB
    ttl_seconds: 7200

metrics:
  enabled: true
  port: 9090

logging:
  level: "info"
  format: "json"

rate_limiting:
  enabled: true
  requests_per_second: 10000
  burst_size: 1000

observability:
  tracing:
    enabled: true
    endpoint: "http://jaeger:4317"
    service_name: "yatagarasu"
```

---

## Validation

Yatagarasu validates configuration at startup and reports errors:

```bash
# Test configuration without starting
yatagarasu --config config.yaml --validate

# Common validation errors:
# - Missing required fields (bucket name, region)
# - Invalid YAML syntax
# - Unknown configuration keys
# - Invalid enum values (algorithm, log level)
```

---

## Hot Reload

Configuration can be reloaded without restart:

```bash
# Send SIGHUP to reload
kill -HUP $(pgrep yatagarasu)

# Or in Docker
docker kill --signal=HUP yatagarasu
```

Reloadable settings:
- Bucket configurations
- Cache settings
- Rate limiting
- Logging level

Non-reloadable settings (require restart):
- Server address/port
- Number of threads

---

## Default Values

| Setting | Default |
|:--------|:--------|
| `server.address` | `0.0.0.0:8080` |
| `server.threads` | Number of CPU cores |
| `cache.memory.max_capacity` | `536870912` (512MB) |
| `cache.memory.ttl_seconds` | `3600` (1 hour) |
| `metrics.enabled` | `false` |
| `metrics.port` | `9090` |
| `logging.level` | `info` |
| `logging.format` | `json` |
