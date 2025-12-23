---
title: Reference
layout: default
nav_order: 7
has_children: true
permalink: /reference/
---

# Reference

API endpoints, error codes, and technical specifications.
{: .fs-6 .fw-300 }

---

## Quick Links

| Reference                                                   | Description                       |
| :---------------------------------------------------------- | :-------------------------------- |
| [API Reference](/yatagarasu/reference/api/)                 | HTTP endpoints and methods        |
| [Image Parameters](/yatagarasu/reference/image-parameters/) | Image optimization URL parameters |
| [Error Codes](/yatagarasu/reference/errors/)                | Error responses and handling      |
| [Performance](/yatagarasu/reference/performance/)           | Benchmarks and specifications     |

---

## Supported HTTP Methods

| Method    | Description              |
| :-------- | :----------------------- |
| `GET`     | Retrieve objects from S3 |
| `HEAD`    | Get object metadata      |
| `OPTIONS` | CORS preflight requests  |

{: .note }
Yatagarasu is a **read-only** proxy. PUT, POST, and DELETE are not supported.

---

## Headers

### Request Headers

| Header              | Description                    |
| :------------------ | :----------------------------- |
| `Authorization`     | JWT token (`Bearer <token>`)   |
| `Range`             | Byte range for partial content |
| `If-None-Match`     | Conditional request (ETag)     |
| `If-Modified-Since` | Conditional request (date)     |

### Response Headers

| Header           | Description               |
| :--------------- | :------------------------ |
| `Content-Type`   | Object MIME type          |
| `Content-Length` | Response body size        |
| `ETag`           | Object version identifier |
| `Last-Modified`  | Object modification date  |
| `Cache-Control`  | Caching directives        |
| `X-Cache`        | Cache hit/miss indicator  |
| `X-Request-Id`   | Unique request identifier |

---

## Status Codes

| Code | Description                             |
| :--- | :-------------------------------------- |
| 200  | Success                                 |
| 206  | Partial Content (Range request)         |
| 304  | Not Modified (conditional request)      |
| 400  | Bad Request                             |
| 401  | Unauthorized (missing/invalid token)    |
| 403  | Forbidden (authorization denied)        |
| 404  | Not Found                               |
| 429  | Too Many Requests (rate limited)        |
| 500  | Internal Server Error                   |
| 502  | Bad Gateway (S3 error)                  |
| 503  | Service Unavailable (all backends down) |
| 504  | Gateway Timeout                         |

---

## Quick API Examples

```bash
# Get object
curl http://proxy:8080/bucket/path/to/file.txt

# Get with JWT
curl -H "Authorization: Bearer <token>" http://proxy:8080/private/file.txt

# Head request (metadata only)
curl -I http://proxy:8080/bucket/file.txt

# Range request (partial content)
curl -H "Range: bytes=0-1023" http://proxy:8080/bucket/large-file.bin

# Conditional request
curl -H "If-None-Match: \"abc123\"" http://proxy:8080/bucket/file.txt
```

---

## Performance Specifications

| Metric                | Specification |
| :-------------------- | :------------ |
| Throughput            | 893+ RPS      |
| P95 Latency (cached)  | 807us         |
| P95 TTFB (S3 stream)  | 24.45ms       |
| Memory per connection | ~64KB         |
| JWT validation        | 1.78us        |
| Path routing          | 95.9ns        |

---

## See Also

- [API Reference](/yatagarasu/reference/api/)
- [Error Codes](/yatagarasu/reference/errors/)
- [Performance Reference](/yatagarasu/reference/performance/)
