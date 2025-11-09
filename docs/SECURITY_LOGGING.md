# Security Logging Guidelines

## Overview

Yatagarasu follows strict security practices to prevent sensitive data exposure in logs. This document outlines what MUST NOT be logged and what is safe to log.

## ❌ NEVER Log These

### 1. Authentication Credentials
- **JWT tokens** (full tokens) - Log only token length
- **Authorization header values** - Never log the full header
- **Bearer tokens** - Only log presence/absence, not value
- **S3 access keys** - NEVER log (not even partially)
- **S3 secret keys** - NEVER log (not even partially)
- **API keys** - NEVER log
- **Passwords** - NEVER log

### 2. Personally Identifiable Information (PII)
- **Full request/response bodies** - May contain sensitive user data
- **Query parameters** (as a whole) - May contain tokens or sensitive data
- **Request headers** (as a whole) - May contain Authorization headers

### 3. Internal Secrets
- **JWT signing secrets** - NEVER log
- **Encryption keys** - NEVER log
- **Database credentials** - NEVER log

## ✅ Safe to Log

### Request Metadata
- **Request ID** (UUID) - Safe, used for correlation
- **Client IP** (X-Forwarded-For aware) - Safe for diagnostics
- **HTTP method** (GET, POST, etc.) - Safe
- **Request path** (URI path) - Safe (but don't log query strings with tokens)
- **Status code** (200, 404, 500, etc.) - Safe
- **Request duration** (milliseconds) - Safe
- **Bucket name** - Safe
- **Timestamp** - Safe

### Authentication Events
- **Token source type** ("bearer", "header", "query") - Safe
- **Token length** (number of characters) - Safe
- **Authentication success/failure** - Safe
- **Claims verification result** - Safe (don't log claim values if sensitive)

### Error Information
- **Error type** ("InvalidToken", "MissingToken", etc.) - Safe
- **Error message** (generic, no sensitive details) - Safe
- **Stack traces** (in debug mode) - Safe if they don't contain secrets

## Current Implementation

### JWT Authentication Logging

**SECURE** - We log token length, not the token itself:

```rust
// src/auth/mod.rs:103-107
tracing::debug!(
    "Successfully extracted JWT token from source type '{}' (length: {} chars)",
    source.source_type,
    token_value.len()  // ✅ Safe - only logs length
);
```

### Request Logging

**SECURE** - We log request metadata, not headers/body:

```rust
// src/proxy/mod.rs:1433-1441
tracing::info!(
    request_id = %ctx.request_id(),    // ✅ Safe
    client_ip = %client_ip,             // ✅ Safe
    method = %ctx.method(),             // ✅ Safe
    path = %ctx.path(),                 // ✅ Safe
    status_code = status_code,          // ✅ Safe
    duration_ms = duration_ms,          // ✅ Safe
    "Request completed"
);
```

### S3 Error Logging

**SECURE** - We log S3 error codes and messages from response headers (safe diagnostic info):

```rust
// src/proxy/mod.rs:1379-1430
// Extract S3 error information from upstream response headers (if error status)
let (s3_error_code, s3_error_message) = if status_code >= 400 {
    if let Some(resp) = session.response_written() {
        let error_code = resp
            .headers
            .get("x-amz-error-code")           // ✅ Safe - AWS error code
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let error_message = resp
            .headers
            .get("x-amz-error-message")        // ✅ Safe - AWS error message
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        (error_code, error_message)
    } else {
        (None, None)
    }
} else {
    (None, None)
};

// Log S3 errors with error code and message
if let (Some(code), Some(message)) = (&s3_error_code, &s3_error_message) {
    tracing::warn!(
        request_id = %ctx.request_id(),
        client_ip = %client_ip,
        method = %ctx.method(),
        path = %ctx.path(),
        status_code = status_code,
        s3_error_code = %code,             // ✅ Safe - e.g., "NoSuchKey", "AccessDenied"
        s3_error_message = %message,       // ✅ Safe - AWS error description
        bucket = bucket_name,              // ✅ Safe
        duration_ms = duration_ms,
        "S3 error response with error details"
    );
}
```

**AWS S3 Error Headers**:
- `x-amz-error-code`: Error type (e.g., "NoSuchKey", "AccessDenied", "InvalidBucketName")
- `x-amz-error-message`: Human-readable error description
- These headers contain diagnostic information only, never credentials or sensitive data

### Configuration Loading

**SECURE** - We log config file path and summary, not credentials:

```rust
// src/main.rs:69-76
tracing::info!(
    config_file = %config_path_display,  // ✅ Safe
    server_address = %config.server.address,  // ✅ Safe
    server_port = config.server.port,    // ✅ Safe
    buckets = config.buckets.len(),      // ✅ Safe
    jwt_enabled = config.jwt.is_some(),  // ✅ Safe
    "Configuration loaded and validated successfully"
);
// ❌ We do NOT log: access_key, secret_key, jwt.secret
```

## Security Review Checklist

Before adding any new logging statement, verify:

1. [ ] No JWT tokens or bearer tokens logged
2. [ ] No Authorization header values logged
3. [ ] No S3 credentials (access_key, secret_key) logged
4. [ ] No JWT signing secrets logged
5. [ ] No request/response bodies logged
6. [ ] No query parameters with potential tokens logged
7. [ ] If logging error details, ensure no sensitive data in error message

## Verification

To verify no sensitive data is logged:

```bash
# Search for potential sensitive logging
grep -rn "tracing::" src/ | grep -i "token\|secret\|password\|credential\|authorization"

# Audit auth module specifically
grep -n "tracing::" src/auth/mod.rs

# Check main.rs for config logging
grep -n "tracing::" src/main.rs
```

## Production Recommendations

### Log Aggregation

When sending logs to aggregation systems (ELK, Splunk, CloudWatch):

1. **Enable TLS** - Logs in transit must be encrypted
2. **Restrict access** - Only authorized personnel can view logs
3. **Retention policy** - Delete logs after retention period (90 days typical)
4. **Audit access** - Track who accesses logs and when

### Monitoring Alerts

Set up alerts for:

- Unusual authentication failure rates (possible credential stuffing)
- SQL injection attempts (blocked by our validation)
- Path traversal attempts (blocked by our validation)
- Rate limit exceeded events (possible DoS)
- Circuit breaker state changes (backend health issues)

## Compliance

This logging approach supports compliance with:

- **GDPR** - No PII logged without consent
- **PCI DSS** - No credit card data in logs
- **SOC 2** - Secure credential handling
- **HIPAA** - No PHI in logs (if handling healthcare data)

## Incident Response

If sensitive data is accidentally logged:

1. **Stop logging immediately** - Fix the code
2. **Rotate credentials** - Assume compromised
3. **Delete logs** - Purge affected log entries
4. **Audit access** - Check who accessed the logs
5. **Notify stakeholders** - If compliance-relevant
6. **Post-mortem** - Update guidelines to prevent recurrence

## References

- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
- [NIST SP 800-92: Guide to Computer Security Log Management](https://csrc.nist.gov/publications/detail/sp/800-92/final)
- [CWE-532: Insertion of Sensitive Information into Log File](https://cwe.mitre.org/data/definitions/532.html)

## See Also

- [GRACEFUL_SHUTDOWN.md](GRACEFUL_SHUTDOWN.md) - Operational lifecycle
- [RETRY_INTEGRATION.md](RETRY_INTEGRATION.md) - Pingora retry handling
- [Phase 22: Graceful Shutdown & Observability](../plan.md) - Implementation plan
