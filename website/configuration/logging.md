---
title: Logging Configuration
layout: default
parent: Configuration
nav_order: 7
---

# Logging Configuration

Configure logging output and verbosity.
{: .fs-6 .fw-300 }

---

## Basic Configuration

```yaml
logging:
  level: "info"
  format: "json"
```

---

## Options

### level

Log verbosity level.

| | |
|:--|:--|
| **Type** | `string` |
| **Default** | `"info"` |
| **Values** | `trace`, `debug`, `info`, `warn`, `error` |

```yaml
logging:
  level: "debug"  # More verbose
  level: "warn"   # Less verbose
```

| Level | Description |
|:------|:------------|
| `trace` | Very detailed debugging |
| `debug` | Debugging information |
| `info` | Normal operational messages |
| `warn` | Warning conditions |
| `error` | Error conditions |

---

### format

Log output format.

| | |
|:--|:--|
| **Type** | `string` |
| **Default** | `"json"` |
| **Values** | `json`, `text` |

#### JSON Format

```yaml
logging:
  format: "json"
```

Output:
```json
{"timestamp":"2024-01-15T10:30:00Z","level":"info","message":"Request completed","bucket":"assets","path":"/image.png","status":200,"duration_ms":15}
```

#### Text Format

```yaml
logging:
  format: "text"
```

Output:
```
2024-01-15T10:30:00Z INFO Request completed bucket=assets path=/image.png status=200 duration_ms=15
```

---

## Environment Variable Override

```bash
# Override log level at runtime
RUST_LOG=debug yatagarasu --config config.yaml

# Module-specific logging
RUST_LOG=yatagarasu=debug,hyper=warn yatagarasu --config config.yaml
```

---

## Request Logging

Enable detailed request logging:

```yaml
logging:
  level: "info"
  format: "json"
  request_logging:
    enabled: true
    include_headers: false      # Include request headers
    include_query_params: true  # Include query parameters
    include_response_time: true # Include response timing
```

---

## Log Fields

### Standard Fields

| Field | Description |
|:------|:------------|
| `timestamp` | ISO 8601 timestamp |
| `level` | Log level |
| `message` | Log message |
| `target` | Module/component name |

### Request Fields

| Field | Description |
|:------|:------------|
| `request_id` | Unique request ID |
| `method` | HTTP method |
| `path` | Request path |
| `bucket` | Matched bucket name |
| `status` | Response status code |
| `duration_ms` | Request duration |
| `bytes_sent` | Response body size |
| `client_ip` | Client IP address |

### Error Fields

| Field | Description |
|:------|:------------|
| `error` | Error message |
| `error_type` | Error classification |
| `backtrace` | Stack trace (debug only) |

---

## Structured Logging Examples

### Successful Request

```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "info",
  "message": "Request completed",
  "request_id": "req-abc123",
  "method": "GET",
  "path": "/assets/logo.png",
  "bucket": "public-assets",
  "status": 200,
  "duration_ms": 15,
  "bytes_sent": 45678,
  "cache_hit": true,
  "cache_tier": "memory"
}
```

### Authentication Failure

```json
{
  "timestamp": "2024-01-15T10:30:01.456Z",
  "level": "warn",
  "message": "JWT validation failed",
  "request_id": "req-def456",
  "path": "/private/data.json",
  "bucket": "private-data",
  "error": "Token expired",
  "error_type": "auth_error"
}
```

### S3 Backend Error

```json
{
  "timestamp": "2024-01-15T10:30:02.789Z",
  "level": "error",
  "message": "S3 request failed",
  "request_id": "req-ghi789",
  "bucket": "ha-data",
  "replica": "primary",
  "error": "Connection timeout",
  "error_type": "backend_error"
}
```

---

## Log Aggregation

### Docker Logging

```bash
# View logs
docker logs yatagarasu

# Follow logs
docker logs -f yatagarasu

# With timestamps
docker logs -t yatagarasu
```

### Kubernetes Logging

```bash
# View logs
kubectl logs deployment/yatagarasu

# Follow logs
kubectl logs -f deployment/yatagarasu

# All pods
kubectl logs -l app=yatagarasu
```

### Log Shipping

JSON format is compatible with:
- **Elasticsearch/OpenSearch** - Direct ingest
- **Loki** - Promtail/Grafana Agent
- **Splunk** - HTTP Event Collector
- **Datadog** - Log agent

---

## Best Practices

1. **Use JSON in production** - Easier to parse and query
2. **Use info level by default** - Debug only when troubleshooting
3. **Enable request logging** - Essential for debugging
4. **Don't log sensitive data** - Avoid logging JWT tokens or credentials
5. **Correlate with request ID** - Use `request_id` for tracing

---

## See Also

- [Operations Guide](/yatagarasu/operations/)
- [Troubleshooting](/yatagarasu/operations/troubleshooting/)
