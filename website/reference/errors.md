---
title: Error Codes
layout: default
parent: Reference
nav_order: 2
---

# Error Codes

HTTP status codes and error handling.
{: .fs-6 .fw-300 }

---

## Error Response Format

All errors return JSON:

```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE",
  "request_id": "req-abc123"
}
```

---

## Client Errors (4xx)

### 400 Bad Request

Invalid request format or parameters.

| Code | Message | Cause |
|:-----|:--------|:------|
| `INVALID_REQUEST` | Invalid request format | Malformed HTTP request |
| `INVALID_PATH` | Invalid path format | Path doesn't match expected pattern |
| `INVALID_RANGE` | Invalid Range header | Malformed byte range |

**Example:**
```json
{
  "error": "Invalid Range header format",
  "code": "INVALID_RANGE",
  "request_id": "req-abc123"
}
```

---

### 401 Unauthorized

Authentication failed or missing.

| Code | Message | Cause |
|:-----|:--------|:------|
| `MISSING_TOKEN` | Missing authentication token | No token in request |
| `INVALID_TOKEN` | Invalid token format | Token is malformed |
| `INVALID_SIGNATURE` | Invalid JWT signature | Signature verification failed |
| `TOKEN_EXPIRED` | Token expired | `exp` claim is past |
| `INVALID_ALGORITHM` | Unsupported algorithm | Algorithm mismatch |
| `INVALID_ISSUER` | Invalid token issuer | `iss` claim mismatch |

**Example:**
```json
{
  "error": "JWT token has expired",
  "code": "TOKEN_EXPIRED",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Ensure token is included in request
2. Verify token signature with correct secret/key
3. Generate new token if expired
4. Check algorithm matches configuration

---

### 403 Forbidden

Authorization denied.

| Code | Message | Cause |
|:-----|:--------|:------|
| `ACCESS_DENIED` | Access denied | Authorization check failed |
| `CLAIM_MISMATCH` | Required claim not present | JWT claim verification failed |
| `POLICY_DENIED` | Policy denied access | OPA/OpenFGA denied |
| `IP_BLOCKED` | IP address blocked | IP in blocklist |

**Example:**
```json
{
  "error": "Claim 'role' does not match required value 'admin'",
  "code": "CLAIM_MISMATCH",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Ensure JWT has required claims
2. Verify user has necessary permissions
3. Check OPA/OpenFGA policies
4. Verify IP is not blocked

---

### 404 Not Found

Resource not found.

| Code | Message | Cause |
|:-----|:--------|:------|
| `NOT_FOUND` | Object not found | S3 object doesn't exist |
| `BUCKET_NOT_FOUND` | Bucket not found | S3 bucket doesn't exist |
| `NO_ROUTE` | No matching route | Path doesn't match any bucket |

**Example:**
```json
{
  "error": "Object not found: /assets/missing.png",
  "code": "NOT_FOUND",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Verify object exists in S3
2. Check path matches configured `path_prefix`
3. Verify bucket name is correct

---

### 405 Method Not Allowed

HTTP method not supported.

**Example:**
```json
{
  "error": "Method POST not allowed. Supported: GET, HEAD, OPTIONS",
  "code": "METHOD_NOT_ALLOWED",
  "request_id": "req-abc123"
}
```

**Supported methods:** GET, HEAD, OPTIONS

---

### 416 Range Not Satisfiable

Invalid byte range.

**Example:**
```json
{
  "error": "Range not satisfiable: requested 0-1000000, file size 1000",
  "code": "RANGE_NOT_SATISFIABLE",
  "request_id": "req-abc123"
}
```

**Resolution:**
Use a valid byte range within file size.

---

### 429 Too Many Requests

Rate limit exceeded.

**Example:**
```json
{
  "error": "Rate limit exceeded",
  "code": "RATE_LIMITED",
  "retry_after": 5,
  "request_id": "req-abc123"
}
```

**Headers:**
```
Retry-After: 5
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1705312800
```

**Resolution:**
1. Wait for `Retry-After` seconds
2. Implement exponential backoff
3. Request higher rate limits

---

## Server Errors (5xx)

### 500 Internal Server Error

Unexpected server error.

| Code | Message | Cause |
|:-----|:--------|:------|
| `INTERNAL_ERROR` | Internal server error | Unexpected error |
| `CONFIG_ERROR` | Configuration error | Invalid configuration |

**Example:**
```json
{
  "error": "Internal server error",
  "code": "INTERNAL_ERROR",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Check server logs for details
2. Report issue with `request_id`

---

### 502 Bad Gateway

S3 backend error.

| Code | Message | Cause |
|:-----|:--------|:------|
| `S3_ERROR` | S3 backend error | S3 returned an error |
| `INVALID_RESPONSE` | Invalid backend response | S3 returned invalid data |

**Example:**
```json
{
  "error": "S3 returned error: AccessDenied",
  "code": "S3_ERROR",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Verify S3 credentials
2. Check bucket permissions
3. Verify S3 service status

---

### 503 Service Unavailable

Service temporarily unavailable.

| Code | Message | Cause |
|:-----|:--------|:------|
| `ALL_BACKENDS_DOWN` | All backends unavailable | All S3 replicas failed |
| `CIRCUIT_OPEN` | Circuit breaker open | All circuits are open |
| `OVERLOADED` | Service overloaded | Too many concurrent requests |

**Example:**
```json
{
  "error": "All S3 backends are unavailable",
  "code": "ALL_BACKENDS_DOWN",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Check S3 backend health
2. Wait for circuit breaker recovery
3. Verify network connectivity

---

### 504 Gateway Timeout

Backend request timeout.

| Code | Message | Cause |
|:-----|:--------|:------|
| `TIMEOUT` | Request timeout | S3 request exceeded timeout |
| `CONNECTION_TIMEOUT` | Connection timeout | Failed to connect to S3 |

**Example:**
```json
{
  "error": "Request timed out after 30 seconds",
  "code": "TIMEOUT",
  "request_id": "req-abc123"
}
```

**Resolution:**
1. Retry the request
2. Check S3 latency
3. Increase timeout configuration

---

## Error Handling Best Practices

### Client-Side

```javascript
async function fetchFromProxy(path) {
  const response = await fetch(`https://proxy.example.com${path}`, {
    headers: {
      'Authorization': `Bearer ${token}`
    }
  });

  if (!response.ok) {
    const error = await response.json();

    switch (response.status) {
      case 401:
        // Refresh token and retry
        await refreshToken();
        return fetchFromProxy(path);

      case 404:
        // Handle not found
        return null;

      case 429:
        // Wait and retry
        const retryAfter = response.headers.get('Retry-After');
        await sleep(retryAfter * 1000);
        return fetchFromProxy(path);

      case 503:
      case 504:
        // Retry with backoff
        await sleep(1000);
        return fetchFromProxy(path);

      default:
        throw new Error(error.message);
    }
  }

  return response;
}
```

### Retry Strategy

```
Retry on: 429, 502, 503, 504
Max retries: 3
Backoff: exponential (1s, 2s, 4s)
Jitter: random 0-500ms
```

---

## Debugging Errors

### Include Request ID

Always log the `request_id` from error responses:

```json
{
  "request_id": "req-abc123"
}
```

### Check Server Logs

Search for the request ID:

```bash
docker logs yatagarasu 2>&1 | jq 'select(.request_id == "req-abc123")'
```

### Enable Debug Logging

```yaml
logging:
  level: "debug"
```

---

## See Also

- [API Reference](/yatagarasu/reference/api/)
- [Troubleshooting](/yatagarasu/operations/troubleshooting/)
- [Authentication](/yatagarasu/configuration/authentication/)
