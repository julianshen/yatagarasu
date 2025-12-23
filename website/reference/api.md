---
title: API Reference
layout: default
parent: Reference
nav_order: 1
---

# API Reference

Complete HTTP API documentation.
{: .fs-6 .fw-300 }

---

## Object Endpoints

### GET /{path_prefix}/{key}

Retrieve an object from S3.

**Request:**

```http
GET /assets/images/logo.png HTTP/1.1
Host: proxy.example.com
Authorization: Bearer <jwt>  (if auth enabled)
```

**Response:**

```http
HTTP/1.1 200 OK
Content-Type: image/png
Content-Length: 45678
ETag: "abc123def456"
Last-Modified: Mon, 15 Jan 2024 10:30:00 GMT
Cache-Control: max-age=3600
X-Cache: HIT
X-Request-Id: req-abc123

<binary data>
```

**Example:**

```bash
curl http://proxy:8080/assets/images/logo.png -o logo.png
```

---

### HEAD /{path_prefix}/{key}

Get object metadata without body.

**Request:**

```http
HEAD /assets/images/logo.png HTTP/1.1
Host: proxy.example.com
```

**Response:**

```http
HTTP/1.1 200 OK
Content-Type: image/png
Content-Length: 45678
ETag: "abc123def456"
Last-Modified: Mon, 15 Jan 2024 10:30:00 GMT
```

**Example:**

```bash
curl -I http://proxy:8080/assets/images/logo.png
```

---

### OPTIONS /{path_prefix}/{key}

CORS preflight request.

**Request:**

```http
OPTIONS /assets/images/logo.png HTTP/1.1
Host: proxy.example.com
Origin: https://example.com
Access-Control-Request-Method: GET
```

**Response:**

```http
HTTP/1.1 200 OK
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, HEAD, OPTIONS
Access-Control-Allow-Headers: Authorization, Content-Type
Access-Control-Max-Age: 86400
```

---

## Image Optimization

### GET /{path_prefix}/{image}?{params}

Process and optimize images on-the-fly.

**Request:**

```http
GET /img/photo.jpg?w=800&h=600&fmt=webp HTTP/1.1
Host: proxy.example.com
Accept: image/webp
```

**Response:**

```http
HTTP/1.1 200 OK
Content-Type: image/webp
Content-Length: 45678
X-Cache: MISS
X-Image-Original-Size: 125000
X-Image-Format: webp

<binary data>
```

**Parameters:**

| Parameter | Description           | Example     |
| :-------- | :-------------------- | :---------- |
| `w`       | Width in pixels       | `w=800`     |
| `h`       | Height in pixels      | `h=600`     |
| `q`       | Quality (1-100)       | `q=80`      |
| `fmt`     | Output format         | `fmt=webp`  |
| `fit`     | Fit mode              | `fit=cover` |
| `g`       | Gravity/crop position | `g=center`  |
| `rot`     | Rotation (90,180,270) | `rot=90`    |
| `flip`    | Flip (h,v,hv)         | `flip=h`    |
| `dpr`     | Device pixel ratio    | `dpr=2`     |

**Examples:**

```bash
# Resize to 800px width
curl "http://proxy:8080/img/photo.jpg?w=800" -o resized.jpg

# Convert to WebP at 80% quality
curl "http://proxy:8080/img/photo.jpg?w=800&fmt=webp&q=80" -o photo.webp

# Square thumbnail with cover crop
curl "http://proxy:8080/img/photo.jpg?w=200&h=200&fit=cover" -o thumb.jpg

# Auto-format based on Accept header
curl -H "Accept: image/avif" "http://proxy:8080/img/photo.jpg?w=800&fmt=auto"
```

See [Image Parameters Reference](/yatagarasu/reference/image-parameters/) for all options.

---

## Range Requests

### Partial Content

Request a byte range:

**Request:**

```http
GET /assets/video.mp4 HTTP/1.1
Host: proxy.example.com
Range: bytes=0-1023
```

**Response:**

```http
HTTP/1.1 206 Partial Content
Content-Type: video/mp4
Content-Length: 1024
Content-Range: bytes 0-1023/1048576
Accept-Ranges: bytes

<binary data>
```

**Examples:**

```bash
# First 1KB
curl -H "Range: bytes=0-1023" http://proxy:8080/assets/video.mp4

# Last 1KB
curl -H "Range: bytes=-1024" http://proxy:8080/assets/video.mp4

# From offset to end
curl -H "Range: bytes=1000-" http://proxy:8080/assets/video.mp4
```

---

## Conditional Requests

### If-None-Match (ETag)

**Request:**

```http
GET /assets/data.json HTTP/1.1
Host: proxy.example.com
If-None-Match: "abc123def456"
```

**Response (Not Modified):**

```http
HTTP/1.1 304 Not Modified
ETag: "abc123def456"
```

**Response (Modified):**

```http
HTTP/1.1 200 OK
ETag: "xyz789new123"
Content-Type: application/json

<new content>
```

### If-Modified-Since

**Request:**

```http
GET /assets/data.json HTTP/1.1
Host: proxy.example.com
If-Modified-Since: Mon, 15 Jan 2024 10:30:00 GMT
```

---

## Health Endpoints

### GET /health

Liveness check - is the process running?

**Response:**

```json
{
  "status": "ok"
}
```

**Usage:**

```bash
curl http://proxy:8080/health
```

### GET /ready

Readiness check - is the proxy ready to serve traffic?

**Response:**

```json
{
  "status": "ok",
  "backends": [
    {
      "name": "primary",
      "healthy": true,
      "latency_ms": 15
    },
    {
      "name": "backup",
      "healthy": true,
      "latency_ms": 45
    }
  ]
}
```

**Usage:**

```bash
curl http://proxy:8080/ready
```

---

## Metrics Endpoint

### GET /metrics (port 9090)

Prometheus metrics.

**Response:**

```prometheus
# HELP yatagarasu_requests_total Total requests
# TYPE yatagarasu_requests_total counter
yatagarasu_requests_total{bucket="assets",status="200"} 12345

# HELP yatagarasu_request_duration_seconds Request duration
# TYPE yatagarasu_request_duration_seconds histogram
yatagarasu_request_duration_seconds_bucket{bucket="assets",le="0.01"} 10000
yatagarasu_request_duration_seconds_bucket{bucket="assets",le="0.1"} 12000
...
```

**Usage:**

```bash
curl http://proxy:9090/metrics
```

---

## Authentication

### Bearer Token

```http
GET /private/data.json HTTP/1.1
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
```

### Query Parameter

```http
GET /private/data.json?token=eyJhbGciOiJIUzI1NiIs... HTTP/1.1
```

### Custom Header

```http
GET /private/data.json HTTP/1.1
X-Auth-Token: eyJhbGciOiJIUzI1NiIs...
```

---

## Request Headers

| Header              | Required    | Description           |
| :------------------ | :---------- | :-------------------- |
| `Host`              | Yes         | Target host           |
| `Authorization`     | Conditional | Bearer token for auth |
| `Range`             | No          | Byte range request    |
| `If-None-Match`     | No          | Conditional ETag      |
| `If-Modified-Since` | No          | Conditional date      |
| `Accept-Encoding`   | No          | Accepted encodings    |

---

## Response Headers

| Header           | Always        | Description        |
| :--------------- | :------------ | :----------------- |
| `Content-Type`   | Yes           | MIME type          |
| `Content-Length` | Yes           | Body size          |
| `Date`           | Yes           | Response timestamp |
| `X-Request-Id`   | Yes           | Unique request ID  |
| `ETag`           | If available  | Object version     |
| `Last-Modified`  | If available  | Modification date  |
| `Cache-Control`  | If configured | Caching directive  |
| `X-Cache`        | If cached     | HIT or MISS        |
| `Content-Range`  | If partial    | Range info         |
| `Accept-Ranges`  | If supported  | bytes              |

---

## CORS Headers

When CORS is enabled:

| Header                          | Value                         |
| :------------------------------ | :---------------------------- |
| `Access-Control-Allow-Origin`   | `*` or configured origin      |
| `Access-Control-Allow-Methods`  | `GET, HEAD, OPTIONS`          |
| `Access-Control-Allow-Headers`  | `Authorization, Content-Type` |
| `Access-Control-Max-Age`        | `86400`                       |
| `Access-Control-Expose-Headers` | `ETag, Content-Length`        |

---

## URL Encoding

Object keys must be URL-encoded:

| Character | Encoded             |
| :-------- | :------------------ |
| Space     | `%20` or `+`        |
| `/`       | `%2F` (in key name) |
| `?`       | `%3F`               |
| `#`       | `%23`               |
| `&`       | `%26`               |

**Example:**

```bash
# File: "reports/2024 Q1/summary.pdf"
curl "http://proxy:8080/docs/reports%2F2024%20Q1%2Fsummary.pdf"
```

---

## Rate Limiting

When rate limited:

**Response:**

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 5
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1705312800

{
  "error": "Rate limit exceeded",
  "retry_after": 5
}
```

---

## See Also

- [Error Codes](/yatagarasu/reference/errors/)
- [Authentication](/yatagarasu/configuration/authentication/)
- [Configuration](/yatagarasu/configuration/)
