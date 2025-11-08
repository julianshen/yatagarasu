# Retry Logic Integration with Pingora

**Date**: 2025-11-08
**Status**: Implementation Guide
**Related**: Phase 21 - Production Hardening & Resilience

## Executive Summary

**Good News**: Pingora has BUILT-IN retry logic that works out of the box!

We do NOT need to implement custom retry loops. Pingora automatically retries requests based on error retry-ability flags that we can control through error handling hooks.

## How Pingora's Retry System Works

### 1. Built-in Retry Loop

Pingora's HTTP proxy has an automatic retry loop ([pingora-proxy-0.6.0/src/lib.rs](https://github.com/cloudflare/pingora/blob/main/pingora-proxy/src/lib.rs)):

```rust
let mut retries: usize = 0;
while retries < self.max_retries {
    retries += 1;
    let (reuse, e) = self.proxy_to_upstream(&mut session, &mut ctx).await;

    match e {
        Some(error) => {
            let retry = error.retry();  // Check if error is retry-able
            proxy_error = Some(error);
            if !retry {
                break;  // Stop retrying if error is not retry-able
            }
            warn!("Fail to proxy: {}, tries: {}, retry: {}", ...);
        }
        None => {
            // Success - exit retry loop
            proxy_error = None;
            break;
        }
    };
}
```

### 2. Configuration

Retry behavior is controlled via `ServerConf::max_retries`:

**Location**: `pingora_core::server::configuration::ServerConf`

```rust
pub struct ServerConf {
    // ... other fields ...

    /// Maximum number of retry attempts for failed requests
    pub max_retries: usize,

    // ... other fields ...
}
```

**How we currently set it** ([src/main.rs:68](../src/main.rs#L68)):
```rust
let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
```

The `server.configuration` from `pingora_core::server::Server` contains default retry settings.

### 3. Error Retry Control

We control WHICH errors are retry-able through the `ProxyHttp` trait error handling hooks:

#### fail_to_connect Hook

Called when upstream connection establishment fails:

```rust
fn fail_to_connect(
    &self,
    _session: &mut Session,
    _peer: &HttpPeer,
    _ctx: &mut Self::CTX,
    mut e: Box<Error>,
) -> Box<Error> {
    // Decide if this error should be retried
    if is_transient_error(&e) {
        e.retry = true.into();  // RETRY this error
    } else {
        e.retry = false.into(); // DON'T retry this error
    }
    e
}
```

#### error_while_proxy Hook

Called when an error occurs AFTER connection is established:

```rust
fn error_while_proxy(
    &self,
    peer: &HttpPeer,
    session: &mut Session,
    mut e: Box<Error>,
    _ctx: &mut Self::CTX,
    client_reused: bool,
) -> Box<Error> {
    // Default Pingora behavior (already implemented):
    e.retry.decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());

    // We can override to add custom logic:
    if should_retry_based_on_error_type(&e) {
        e.retry = true.into();
    }

    e
}
```

### 4. Retry Types

Pingora has three retry modes ([pingora-error-0.6.0/src/lib.rs](https://github.com/cloudflare/pingora/blob/main/pingora-error/src/lib.rs)):

```rust
pub enum RetryType {
    Decided(bool),      // Explicitly retry (true) or don't (false)
    ReusedOnly,         // Only retry if connection was reused
}
```

**Key Insight**: `ReusedOnly` is the safe default - only retry if the connection was reused (meaning the request might not have been sent yet).

## Integration Strategy

### Option A: Use Pingora Defaults (Recommended for MVP)

**What we get**:
- Automatic retries for connection failures
- Automatic retries for reused connection errors
- No additional code needed

**Default behavior**:
- `max_retries`: Set by Pingora server configuration (likely 3)
- Connection failures: Automatically retried
- Reused connection errors: Automatically retried
- New connection errors after data sent: NOT retried (safe default)

**Action required**: NONE - already working!

### Option B: Customize Retry Logic (Future Enhancement)

**When to implement**:
- Need to retry specific S3 error codes (500, 503)
- Need exponential backoff between retries
- Need per-bucket retry policies
- Need retry metrics

**Implementation approach**:

#### Step 1: Implement fail_to_connect Hook

```rust
// In src/proxy/mod.rs

impl ProxyHttp for ProxyHandler {
    // ... existing methods ...

    fn fail_to_connect(
        &self,
        _session: &mut Session,
        peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut e: Box<Error>,
    ) -> Box<Error> {
        // Get bucket-specific retry policy
        let bucket_config = ctx.bucket_config();
        let retry_policy = self.retry_policies.get(&bucket_config.name);

        // Decide if error is retry-able based on error type
        let should_retry = match e.etype {
            ErrorType::ConnectTimedout => true,      // Retry timeouts
            ErrorType::ConnectRefused => true,       // Retry refused connections
            ErrorType::ConnectNoRoute => false,      // Don't retry no route
            ErrorType::TLSHandshakeFailure => false, // Don't retry TLS failures
            _ => false,
        };

        if should_retry {
            e.retry = true.into();
            tracing::warn!(
                peer = %peer,
                error = %e,
                "Connection failed, will retry"
            );
        } else {
            e.retry = false.into();
            tracing::error!(
                peer = %peer,
                error = %e,
                "Connection failed, non-retriable"
            );
        }

        e
    }
}
```

#### Step 2: Implement error_while_proxy Hook for S3 Errors

```rust
fn error_while_proxy(
    &self,
    peer: &HttpPeer,
    session: &mut Session,
    mut e: Box<Error>,
    ctx: &mut Self::CTX,
    client_reused: bool,
) -> Box<Error> {
    // Default Pingora behavior for reused connections
    e.retry.decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());

    // Additional logic for S3 server errors
    if let Some(status) = e.status {
        match status.as_u16() {
            500 | 503 => {
                // S3 server errors - retry even on new connections
                e.retry = true.into();
                tracing::warn!(
                    status = status.as_u16(),
                    peer = %peer,
                    "S3 server error, will retry"
                );
            }
            502 | 504 => {
                // Gateway errors - retry
                e.retry = true.into();
            }
            404 | 403 | 400 => {
                // Client errors - DON'T retry
                e.retry = false.into();
            }
            _ => {
                // Keep default Pingora behavior
            }
        }
    }

    e
}
```

#### Step 3: Configure max_retries

**Challenge**: Pingora's `ServerConf` is created by the server framework. We cannot directly set `max_retries` without modifying server creation.

**Solutions**:

**Option 1**: Use Pingora's default max_retries (simplest)
- No code changes needed
- Default is likely 3 retries

**Option 2**: Create custom ServerConf (requires refactoring)
```rust
// Would require changing server creation in main.rs
let mut server_conf = ServerConf::default();
server_conf.max_retries = config.server.max_retries.unwrap_or(3);
// Need to pass this to Server::new() - requires API changes
```

**Option 3**: Environment variable override (if Pingora supports it)
```bash
PINGORA_MAX_RETRIES=3 ./yatagarasu --config config.yaml
```

## What About Exponential Backoff?

**Current Status**: Pingora's built-in retry loop does NOT implement exponential backoff.

**Why**: Pingora is designed for high-performance proxying where retry delays would hurt latency. The assumption is that if a backend is down, rapid retries to alternative backends are better than delayed retries to the same backend.

**Our Implementation**: We have exponential backoff logic in [src/retry.rs](../src/retry.rs), but it's not currently integrated because:
1. Pingora controls the retry loop timing
2. No hooks for adding delays between retries
3. Would require significant architectural changes

**Alternatives**:
1. **Circuit Breaker** (already implemented): Prevents rapid retries to failing backends
2. **Rate Limiting** (already implemented): Prevents overwhelming S3
3. **Health Checks**: Could add periodic health checks to failing backends
4. **Backend Selection**: Implement smart peer selection to avoid known-bad backends

## Current Implementation Status

### What's Already Working

✅ **Automatic Retries**: Pingora retries connection failures automatically
✅ **Reused Connection Retries**: Pingora retries errors on reused connections
✅ **Circuit Breaker**: Prevents retries to failing backends ([src/proxy/mod.rs:844-860](../src/proxy/mod.rs#L844-L860))
✅ **Retry Policy Config**: Configuration parsing complete ([src/config/mod.rs](../src/config/mod.rs))
✅ **Retry Module**: Exponential backoff logic exists ([src/retry.rs](../src/retry.rs))

### What's NOT Integrated

❌ **Custom Retry Decision Logic**: Not using fail_to_connect/error_while_proxy hooks
❌ **S3-Specific Error Retries**: Not retrying S3 500/503 errors
❌ **Exponential Backoff**: No delays between retries
❌ **Per-Bucket Retry Policies**: Not using bucket-specific retry configs
❌ **Retry Metrics**: Not tracking retry attempts/successes/failures

## Recommendation

### For v1.0 (Current)

**Use Pingora's default retry behavior** with no custom code:
- Connection failures are retried automatically
- Works well for transient network issues
- Zero implementation cost
- Safe defaults (won't retry after data sent)

### For v1.1 (Future Enhancement)

**Implement custom retry hooks** to add:
1. **S3-specific error handling** (retry 500/503)
2. **Non-retriable error detection** (don't retry 404/403)
3. **Retry metrics** (track attempts per request)
4. **Logging** (detailed retry context for debugging)

**NOT recommended**: Exponential backoff (conflicts with Pingora's design)
**Better alternative**: Use circuit breaker + rate limiting (already implemented)

## Testing Retry Logic

### Unit Tests

Test error retry decisions:

```rust
#[test]
fn test_connection_timeout_is_retriable() {
    let mut error = Error::new(ErrorType::ConnectTimedout);
    error.retry = false.into(); // Start as non-retriable

    // Call our fail_to_connect hook
    let error = proxy.fail_to_connect(&mut session, &peer, &mut ctx, Box::new(error));

    assert!(error.retry.retry(), "Connection timeout should be retriable");
}
```

### Integration Tests

Test actual retry behavior:

```rust
#[tokio::test]
#[ignore]
async fn test_retries_connection_failures() {
    // Start MinIO
    // Configure bucket with retry policy
    // Simulate connection failure (stop MinIO mid-request)
    // Verify request is retried
    // Verify final error after max_retries exceeded
}
```

## References

- **Pingora ProxyHttp Trait**: [pingora-proxy/src/proxy_trait.rs](https://github.com/cloudflare/pingora/blob/main/pingora-proxy/src/proxy_trait.rs)
- **Pingora Error Types**: [pingora-error/src/lib.rs](https://github.com/cloudflare/pingora/blob/main/pingora-error/src/lib.rs)
- **Pingora Retry Loop**: [pingora-proxy/src/lib.rs](https://github.com/cloudflare/pingora/blob/main/pingora-proxy/src/lib.rs) (search for `max_retries`)
- **Our Retry Module**: [src/retry.rs](../src/retry.rs)
- **Our Proxy Implementation**: [src/proxy/mod.rs](../src/proxy/mod.rs)

---

**Conclusion**: Retry logic is ALREADY WORKING through Pingora's built-in system. We can enhance it in v1.1 by implementing custom retry decision hooks for S3-specific error handling.
