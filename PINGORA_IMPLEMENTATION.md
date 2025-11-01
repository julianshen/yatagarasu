# Pingora ProxyHttp Implementation Guide

**Date**: 2025-11-01
**Purpose**: Guide for implementing Pingora's ProxyHttp trait for Yatagarasu S3 proxy

---

## Research Summary

**Sources**:
- [Official Pingora ProxyHttp docs](https://docs.rs/pingora/latest/pingora/prelude/trait.ProxyHttp.html)
- [Cloudflare load_balancer.rs example](https://github.com/cloudflare/pingora/blob/main/pingora-proxy/examples/load_balancer.rs)
- [Community simple-proxy](https://github.com/tyrchen/simple-proxy)

---

## ProxyHttp Trait Overview

### Required Methods (2)

#### 1. `new_ctx(&self) -> Self::CTX`
- Creates per-request context
- Called once per request
- Returns custom context type (use `RequestContext` from our pipeline module)

####  2. `upstream_peer(&self, session: &mut Session, ctx: &mut CTX) -> Result<Box<HttpPeer>>`
- Determines target upstream server
- Called for every request
- Returns `HttpPeer` with connection details

### Optional Methods (20+)

**Authentication & Routing**:
- `request_filter()` - Process incoming request, authenticate, return `Ok(true)` to short-circuit
- `early_request_filter()` - Called before downstream modules

**Upstream Modification**:
- `upstream_request_filter()` - Modify headers before sending to upstream (AWS SigV4 here)
- `upstream_response_filter()` - Alter upstream response headers

**Error Handling**:
- `fail_to_connect()` - Handle connection failures, enable retries
- `error_while_proxy()` - Handle errors after connection
- `fail_to_proxy()` - Handle fatal errors

**Body Processing**:
- `request_body_filter()` - Process request body chunks
- `upstream_response_body_filter()` - Process upstream response body
- `response_body_filter()` - Process outgoing response body

**Logging & Metrics**:
- `logging()` - Generate metrics/logs after request completion
- `request_summary()` - Custom error log descriptions

---

## HttpPeer Creation Pattern

```rust
use pingora_core::upstreams::peer::HttpPeer;

// For S3 endpoint
let peer = Box::new(HttpPeer::new(
    ("bucket.s3.region.amazonaws.com", 443).into(),  // SocketAddr
    true,                                              // use_tls
    "bucket.s3.region.amazonaws.com".to_string()      // SNI hostname
));
```

---

## Server Startup Pattern

```rust
use pingora_core::server::{Server, configuration::Opt};
use pingora_proxy;
use clap::Parser;

fn main() {
    // Parse command-line options
    let opt = Opt::parse();

    // Create server
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    // Create proxy instance
    let proxy = YatagarasuProxy::new(config).unwrap();

    // Create HTTP proxy service
    let mut proxy_service = pingora_proxy::http_proxy_service(
        &server.configuration,
        proxy
    );

    // Add TCP listener
    proxy_service.add_tcp("0.0.0.0:8080");

    // Optional: Add TLS listener
    // let tls_settings = TlsSettings::intermediate(&cert_path, &key_path).unwrap();
    // proxy_service.add_tls_with_settings("0.0.0.0:8443", None, tls_settings);

    // Register service and run
    server.add_service(proxy_service);
    server.run_forever();  // Blocks forever
}
```

---

## Implementation Strategy for Yatagarasu

### Phase 1: Minimal ProxyHttp (Get server running)
```rust
use async_trait::async_trait;
use pingora_proxy::{ProxyHttp, Session};
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use crate::pipeline::RequestContext;

pub struct YatagarasuProxy {
    config: Arc<Config>,
}

#[async_trait]
impl ProxyHttp for YatagarasuProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new("GET".to_string(), "/".to_string())
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX
    ) -> Result<Box<HttpPeer>> {
        // For now, hardcode to first bucket
        let bucket_config = &self.config.buckets[0];
        let endpoint = format!(
            "{}.s3.{}.amazonaws.com",
            bucket_config.s3.bucket,
            bucket_config.s3.region
        );

        let peer = Box::new(HttpPeer::new(
            (endpoint.as_str(), 443).into(),
            true,
            endpoint
        ));
        Ok(peer)
    }
}
```

### Phase 2: Add Routing
Use `request_filter()` to route based on path:

```rust
async fn request_filter(
    &self,
    session: &mut Session,
    ctx: &mut Self::CTX
) -> Result<bool> {
    let path = session.req_header().uri.path();

    // Find matching bucket
    let bucket_config = match self.router.route(path) {
        Some(config) => config,
        None => {
            // Return 404
            session.respond_error(404).await;
            return Ok(true); // Short-circuit
        }
    };

    // Store in context for upstream_peer
    ctx.set_bucket_config(bucket_config.clone());

    Ok(false) // Continue to upstream
}
```

### Phase 3: Add JWT Authentication
Extend `request_filter()`:

```rust
async fn request_filter(
    &self,
    session: &mut Session,
    ctx: &mut Self::CTX
) -> Result<bool> {
    // ... routing code ...

    // Check if auth required
    if bucket_config.auth.enabled {
        // Extract headers and query params
        let headers = extract_headers(session.req_header());
        let query_params = extract_query_params(session.req_header());

        // Authenticate
        match authenticate_request(&headers, &query_params, &self.jwt_config) {
            Ok(claims) => ctx.set_claims(claims),
            Err(_) => {
                session.respond_error(401).await;
                return Ok(true); // Short-circuit
            }
        }
    }

    Ok(false)
}
```

### Phase 4: Add AWS SigV4 Signing
Use `upstream_request_filter()`:

```rust
async fn upstream_request_filter(
    &self,
    _session: &mut Session,
    upstream_request: &mut RequestHeader,
    ctx: &mut Self::CTX
) -> Result<()> {
    let bucket_config = ctx.bucket_config().unwrap();

    // Extract S3 key from path
    let s3_key = self.router.extract_s3_key(ctx.path()).unwrap_or_default();

    // Build S3 request and get signed headers
    let s3_request = build_get_object_request(
        &bucket_config.s3.bucket,
        &s3_key,
        &bucket_config.s3.region
    );

    let signed_headers = s3_request.get_signed_headers(
        &bucket_config.s3.access_key,
        &bucket_config.s3.secret_key
    );

    // Add headers to upstream request
    for (name, value) in signed_headers {
        upstream_request.insert_header(&name, &value)?;
    }

    Ok(())
}
```

---

## Complete Implementation Checklist

### proxy/mod.rs
- [ ] Create `YatagarasuProxy` struct with config, router
- [ ] Implement `new()` constructor
- [ ] Implement `ProxyHttp` trait:
  - [ ] `new_ctx()` - Return RequestContext
  - [ ] `upstream_peer()` - Return S3 HttpPeer from context
  - [ ] `request_filter()` - Route and authenticate
  - [ ] `upstream_request_filter()` - Add S3 signature headers

### main.rs
- [ ] Load configuration from file
- [ ] Create YatagarasuProxy instance
- [ ] Initialize Pingora Server
- [ ] Create http_proxy_service
- [ ] Add TCP listener on configured port
- [ ] Add service to server
- [ ] Call run_forever()

### Helper Functions
- [ ] `extract_headers()` - Convert Pingora headers to HashMap
- [ ] `extract_query_params()` - Parse query string to HashMap

---

## Testing Strategy

### Unit Tests
- Test YatagarasuProxy::new() with valid config
- Test context creation
- Mock Session to test request_filter logic

### Integration Tests
- Start server in test
- Send HTTP request to /health
- Verify 200 OK response
- Test with real MinIO instance

---

## Estimated Effort

- **Minimal implementation** (Phases 1): ~2 hours, ~60 lines
- **With routing** (Phase 2): +1 hour, +40 lines
- **With auth** (Phase 3): +1 hour, +40 lines
- **With S3 signing** (Phase 4): +1 hour, +30 lines
- **Total complete implementation**: 4-6 hours, ~170 lines

---

## Next Steps

1. Implement minimal ProxyHttp (Phase 1)
2. Update main.rs to start server
3. Test that server accepts connections
4. Add routing (Phase 2)
5. Add authentication (Phase 3)
6. Add S3 signing (Phase 4)
7. Integration test with MinIO

**Following TDD**: Write test → Watch fail → Implement → Watch pass → Refactor
