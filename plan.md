# Yatagarasu Implementation Plan

This document tracks the test-driven development of Yatagarasu. Each test should be implemented one at a time following the Red-Green-Refactor cycle.

## How to Use This Plan

1. Find the next unmarked test (marked with `[ ]`)
2. Write the test and watch it fail (Red)
3. Write the minimum code to make it pass (Green)
4. Refactor if needed while keeping tests green
5. Mark the test complete with `[x]`
6. Commit (separately for structural and behavioral changes)
7. Move to the next test

## Legend

- `[ ]` - Not yet implemented
- `[x]` - Implemented and passing
- `[~]` - In progress
- `[!]` - Blocked or needs discussion

---

# Yatagarasu Implementation Plan

This document tracks the test-driven development of Yatagarasu S3 proxy. Each test should be implemented one at a time following the Red-Green-Refactor cycle.

## How to Use This Plan

1. Find the next unmarked test (marked with `[ ]`)
2. Write the test and watch it fail (Red)
3. Write the minimum code to make it pass (Green)
4. Refactor if needed while keeping tests green
5. Mark the test complete with `[x]`
6. Commit (separately for structural and behavioral changes)
7. Move to the next test

## Legend

- `[ ]` - Not yet implemented
- `[x]` - Implemented and passing
- `[~]` - In progress
- `[!]` - Blocked or needs discussion

---

## Phase 1: Foundation and Project Setup

### Project Structure
- [x] Test: Cargo project compiles without errors
- [x] Test: Basic dependency imports work (Pingora, Tokio)
- [x] Test: Can run `cargo test` successfully (even with no tests yet)
- [x] Test: Can run `cargo clippy` without warnings
- [x] Test: Can run `cargo fmt --check` successfully

### Directory Structure
Create and verify basic project structure:
```
yatagarasu/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config/
│   │   └── mod.rs
│   ├── router/
│   │   └── mod.rs
│   ├── auth/
│   │   └── mod.rs
│   ├── s3/
│   │   └── mod.rs
│   ├── cache/
│   │   └── mod.rs
│   ├── proxy/
│   │   └── mod.rs
│   └── error.rs
├── tests/
│   ├── integration/
│   └── fixtures/
└── benches/
```

- [x] Test: Can create and import `config` module
- [x] Test: Can create and import `router` module
- [x] Test: Can create and import `auth` module
- [x] Test: Can create and import `s3` module
- [x] Test: Can create and import `error` module
- [x] Test: Can create and import `cache` module
- [x] Test: Can create and import `proxy` module

---

## Phase 2: Configuration Management

### Configuration - Basic Structure
- [x] Test: Can create empty Config struct
- [x] Test: Can deserialize minimal valid YAML config
- [x] Test: Can access server address from config
- [x] Test: Can access server port from config
- [x] Test: Config deserialization fails with empty file
- [x] Test: Config deserialization fails with invalid YAML

### Configuration - Bucket Config
- [x] Test: Can parse single bucket configuration
- [x] Test: Can parse multiple bucket configurations
- [x] Test: Can access bucket name from config
- [x] Test: Can access bucket path_prefix from config
- [x] Test: Can access S3 bucket name from config
- [x] Test: Can access S3 region from config
- [x] Test: Can access S3 access_key from config
- [x] Test: Can access S3 secret_key from config
- [x] Test: Rejects bucket config with missing required fields
- [x] Test: Rejects bucket config with empty path_prefix
- [x] Test: Rejects bucket config with duplicate path_prefix

### Configuration - Environment Variable Substitution
- [x] Test: Can substitute environment variable in access_key
- [x] Test: Can substitute environment variable in secret_key
- [x] Test: Can substitute environment variable in JWT secret
- [x] Test: Substitution fails gracefully when env var missing
- [x] Test: Can use literal value (no substitution) for non-sensitive fields

### Configuration - Auth Config
- [x] Test: Can parse JWT config with enabled=true
- [x] Test: Can parse JWT config with enabled=false
- [x] Test: Can parse multiple token sources
- [x] Test: Can parse header token source with prefix
- [x] Test: Can parse query parameter token source
- [x] Test: Can parse custom header token source
- [x] Test: Can parse JWT algorithm (HS256)
- [x] Test: Can parse JWT secret
- [x] Test: Rejects JWT config with invalid algorithm
- [x] Test: Rejects auth config missing JWT secret when enabled=true

### Configuration - Claims Verification
- [x] Test: Can parse single claim verification rule
- [x] Test: Can parse multiple claim verification rules
- [x] Test: Can parse "equals" operator
- [x] Test: Can parse string claim value
- [x] Test: Can parse numeric claim value
- [x] Test: Can parse boolean claim value
- [x] Test: Can parse array claim value (for "in" operator)
- [x] Test: Rejects claim verification with unknown operator

### Configuration - Validation
- [x] Test: Validates that all path_prefixes are unique
- [x] Test: Validates that all path_prefixes start with /
- [x] Test: Validates that bucket names are not empty
- [x] Test: Validates that JWT secret exists when auth is enabled
- [x] Test: Validates that at least one token source exists when JWT enabled
- [x] Test: Full config validation passes with valid config
- [x] Test: Full config validation fails with invalid config

### Configuration - Loading from File
- [x] Test: Can load config from YAML file path
- [x] Test: Returns error for non-existent file
- [x] Test: Returns error for unreadable file
- [x] Test: Returns error for malformed YAML

---

## Phase 3: Path Routing

### Router - Basic Path Matching
- [x] Test: Can create router with empty bucket list
- [x] Test: Can create router with single bucket config
- [x] Test: Can create router with multiple bucket configs
- [x] Test: Router matches exact path prefix
- [x] Test: Router matches path with trailing segments
- [x] Test: Router returns None for unmapped path
- [x] Test: Router returns correct bucket for first matching prefix
- [x] Test: Router handles path without leading slash (rejects or normalizes)

### Router - Path Normalization
- [x] Test: Normalizes paths with double slashes (`//`)
- [x] Test: Normalizes paths with trailing slash
- [x] Test: Handles URL-encoded paths correctly
- [x] Test: Handles special characters in paths
- [x] Test: Preserves case sensitivity in paths

### Router - Prefix Matching Edge Cases
- [x] Test: Matches longest prefix when multiple prefixes match
- [x] Test: `/products/foo` matches `/products` not `/prod`
- [x] Test: Handles root path `/` correctly
- [x] Test: Handles path prefixes with query parameters
- [x] Test: Handles path prefixes with fragments (strips them)

### Router - S3 Key Extraction
- [x] Test: Extracts S3 key by removing path prefix
- [x] Test: Handles path prefix with trailing slash
- [x] Test: Handles path prefix without trailing slash
- [x] Test: Extracts nested S3 keys correctly (e.g., `folder/subfolder/file.txt`)
- [x] Test: Handles S3 key with special characters
- [x] Test: Handles empty S3 key (prefix is the full path)

### Router - Performance
- [x] Test: Router lookup is O(1) or O(log n) for reasonable config sizes
- [x] Test: Can handle 100+ bucket configurations efficiently

---

## Phase 4: JWT Authentication

### JWT - Token Extraction from Header
- [x] Test: Extracts token from Authorization header with "Bearer " prefix
- [x] Test: Extracts token from Authorization header without prefix
- [x] Test: Extracts token from custom header (e.g., X-Auth-Token)
- [x] Test: Returns None when Authorization header missing
- [x] Test: Returns None when Authorization header malformed
- [x] Test: Handles whitespace in Authorization header value
- [x] Test: Case-insensitive header name matching

### JWT - Token Extraction from Query Parameter
- [x] Test: Extracts token from query parameter by name
- [x] Test: Returns None when query parameter missing
- [x] Test: Handles URL-encoded token in query parameter
- [x] Test: Handles multiple query parameters (ignores others)
- [x] Test: Handles empty query parameter value

### JWT - Token Extraction Priority
- [x] Test: Tries all configured sources in order
- [x] Test: Returns first valid token found
- [x] Test: Returns None if no sources have valid token
- [x] Test: Header source checked before query parameter
- [x] Test: Configurable source order is respected

### JWT - Token Validation (HS256)
- [x] Test: Validates correctly signed JWT with HS256
- [x] Test: Rejects JWT with invalid signature
- [x] Test: Rejects JWT with expired `exp` claim
- [x] Test: Rejects JWT with future `nbf` (not before) claim
- [x] Test: Accepts JWT with valid `exp` and `nbf` claims
- [x] Test: Rejects malformed JWT (not 3 parts)
- [x] Test: Rejects JWT with invalid Base64 encoding
- [x] Test: Rejects JWT with invalid JSON in payload

### JWT - Claims Extraction
- [x] Test: Extracts standard claims (sub, iss, exp, iat)
- [x] Test: Extracts custom claims from payload
- [x] Test: Handles missing optional claims gracefully
- [x] Test: Handles nested claim structures
- [x] Test: Handles array claims
- [x] Test: Handles null claim values

### JWT - Claims Verification (Equals Operator)
- [x] Test: Verifies string claim equals expected value
- [x] Test: Verifies numeric claim equals expected value
- [ ] Test: Verifies boolean claim equals expected value
- [ ] Test: Fails when claim value doesn't match
- [ ] Test: Fails when claim is missing
- [ ] Test: Case-sensitive string comparison

### JWT - Claims Verification (Multiple Rules)
- [ ] Test: Passes when all verification rules pass (AND logic)
- [ ] Test: Fails when any verification rule fails
- [ ] Test: Handles verification with empty rules list (always passes)
- [ ] Test: Evaluates all rules even if first fails (for better error messages)

### JWT - Authentication Middleware
- [ ] Test: Passes request through when auth disabled for route
- [ ] Test: Extracts and validates JWT when auth enabled
- [ ] Test: Returns 401 when JWT missing and auth required
- [ ] Test: Returns 401 when JWT invalid and auth required
- [ ] Test: Returns 403 when JWT valid but claims verification fails
- [ ] Test: Attaches validated claims to request context
- [ ] Test: Error response includes clear error message

---

## Phase 5: S3 Integration

### S3 Client - Basic Setup
- [ ] Test: Can create S3 client with valid credentials
- [ ] Test: Can create S3 client with region configuration
- [ ] Test: Can create S3 client with custom endpoint (for MinIO/LocalStack)
- [ ] Test: Client creation fails with empty credentials
- [ ] Test: Can create multiple independent S3 clients (one per bucket)

### S3 Signature v4 - Request Signing
- [ ] Test: Generates valid AWS Signature v4 for GET request
- [ ] Test: Signature includes all required headers
- [ ] Test: Signature includes Authorization header with correct format
- [ ] Test: Signature includes x-amz-date header
- [ ] Test: Signature includes x-amz-content-sha256 header
- [ ] Test: Canonical request is generated correctly
- [ ] Test: String to sign is generated correctly
- [ ] Test: Signing key derivation works correctly

### S3 Operations - GET Object
- [ ] Test: Can build GET object request with key
- [ ] Test: GET request includes correct bucket and key in URL
- [ ] Test: GET request includes proper AWS signature headers
- [ ] Test: GET request handles S3 keys with special characters
- [ ] Test: GET request handles S3 keys with URL-unsafe characters
- [ ] Test: GET request preserves original path structure

### S3 Operations - HEAD Object
- [ ] Test: Can build HEAD object request with key
- [ ] Test: HEAD request includes correct HTTP method
- [ ] Test: HEAD request includes same headers as GET
- [ ] Test: HEAD request returns object metadata without body

### S3 Response - Success Handling
- [ ] Test: Parses 200 OK response from S3
- [ ] Test: Extracts content-type header from S3 response
- [ ] Test: Extracts content-length header from S3 response
- [ ] Test: Extracts ETag header from S3 response
- [ ] Test: Extracts Last-Modified header from S3 response
- [ ] Test: Preserves custom S3 metadata headers (x-amz-meta-*)
- [ ] Test: Streams response body to client

### S3 Response - Error Handling
- [ ] Test: Handles 404 Not Found from S3 (object doesn't exist)
- [ ] Test: Handles 403 Forbidden from S3 (access denied)
- [ ] Test: Handles 400 Bad Request from S3 (invalid request)
- [ ] Test: Handles 500 Internal Server Error from S3
- [ ] Test: Handles 503 Service Unavailable from S3 (slow down)
- [ ] Test: Parses S3 XML error response body
- [ ] Test: Extracts error code and message from S3 error response
- [ ] Test: Maps S3 errors to appropriate HTTP status codes

### S3 Response - Streaming
- [ ] Test: Can stream small file (<1MB) efficiently
- [ ] Test: Can stream medium file (10MB) efficiently
- [ ] Test: Can stream large file (100MB) without buffering entire file
- [ ] Test: Streaming stops if client disconnects
- [ ] Test: Memory usage stays constant during streaming
- [ ] Test: Can handle concurrent streams to multiple clients

### S3 Range Requests - Header Parsing
- [ ] Test: Can parse Range header with single range (bytes=0-1023)
- [ ] Test: Can parse Range header with open-ended range (bytes=1000-)
- [ ] Test: Can parse Range header with suffix range (bytes=-1000)
- [ ] Test: Can parse Range header with multiple ranges
- [ ] Test: Handles invalid Range header syntax gracefully
- [ ] Test: Includes Accept-Ranges: bytes in response headers

### S3 Range Requests - Request Handling
- [ ] Test: Forwards Range header to S3 with AWS signature
- [ ] Test: Returns 206 Partial Content for valid range
- [ ] Test: Returns Content-Range header with correct format
- [ ] Test: Streams only requested bytes (not full file)
- [ ] Test: Returns 416 Range Not Satisfiable for out-of-bounds range
- [ ] Test: Handles If-Range conditional requests correctly
- [ ] Test: Graceful fallback to 200 OK for invalid range syntax

### S3 Range Requests - Caching Behavior
- [ ] Test: Range requests bypass cache (never cached)
- [ ] Test: Range request doesn't populate cache
- [ ] Test: Cached full file doesn't satisfy range request (fetches from S3)
- [ ] Test: Range requests work when cache enabled for bucket

### S3 Range Requests - Authentication
- [ ] Test: Range requests work on public buckets
- [ ] Test: Range requests require JWT on private buckets
- [ ] Test: Returns 401 before processing range if auth fails
- [ ] Test: JWT validation happens before range validation

### S3 Range Requests - Performance
- [ ] Test: Memory usage constant for range requests (~64KB buffer)
- [ ] Test: Client disconnect cancels S3 range stream
- [ ] Test: Multiple concurrent range requests work independently
- [ ] Test: Range request latency similar to full file (~500ms TTFB)

### S3 Integration - Mock Tests
- [ ] Test: GET object works with mocked S3 backend
- [ ] Test: HEAD object works with mocked S3 backend
- [ ] Test: Error responses work with mocked S3 backend
- [ ] Test: Can mock different buckets with different responses

---

## Phase 6: Pingora Proxy Integration

### Pingora - Server Setup
- [ ] Test: Can create Pingora server with config
- [ ] Test: Server listens on configured address and port
- [ ] Test: Server can handle HTTP/1.1 requests
- [ ] Test: Server can handle HTTP/2 requests (if enabled)
- [ ] Test: Server handles graceful shutdown
- [ ] Test: Server rejects requests before fully initialized

### Pingora - Request Handler
- [ ] Test: Handler receives incoming HTTP request
- [ ] Test: Handler can access request method
- [ ] Test: Handler can access request path
- [ ] Test: Handler can access request headers
- [ ] Test: Handler can access request query parameters
- [ ] Test: Handler runs router to determine target bucket
- [ ] Test: Handler runs auth middleware when configured
- [ ] Test: Handler builds S3 request from HTTP request

### Pingora - Response Handler
- [ ] Test: Can send response status code
- [ ] Test: Can send response headers
- [ ] Test: Can send response body
- [ ] Test: Can stream response body chunks
- [ ] Test: Handles connection close during streaming
- [ ] Test: Sets appropriate content-type header
- [ ] Test: Preserves S3 response headers in proxy response

### Pingora - Error Responses
- [ ] Test: Returns 400 for malformed requests
- [ ] Test: Returns 401 for unauthorized requests
- [ ] Test: Returns 403 for forbidden requests
- [ ] Test: Returns 404 for not found
- [ ] Test: Returns 500 for internal errors
- [ ] Test: Returns 502 for bad gateway (S3 errors)
- [ ] Test: Returns 503 for service unavailable
- [ ] Test: Error responses include JSON body with error details
- [ ] Test: Error responses don't leak sensitive information

### Pingora - Middleware Chain
- [ ] Test: Request passes through router first
- [ ] Test: Request passes through auth middleware second
- [ ] Test: Request reaches S3 handler third
- [ ] Test: Middleware can short-circuit request (return early)
- [ ] Test: Middleware can modify request context
- [ ] Test: Middleware errors are handled gracefully

---

## Phase 7: Integration Tests (Full Stack)

### End-to-End - Single Bucket, No Auth
- [ ] Test: GET /bucket-a/file.txt returns object from bucket A
- [ ] Test: HEAD /bucket-a/file.txt returns metadata from bucket A
- [ ] Test: GET /bucket-a/nonexistent.txt returns 404
- [ ] Test: GET /unmapped/file.txt returns 404
- [ ] Test: Response includes correct content-type header
- [ ] Test: Response includes S3 ETag header

### End-to-End - Multiple Buckets, No Auth
- [ ] Test: GET /bucket-a/file.txt routes to bucket A
- [ ] Test: GET /bucket-b/file.txt routes to bucket B
- [ ] Test: Buckets use independent credentials
- [ ] Test: Can access objects from both buckets concurrently
- [ ] Test: Bucket A credentials don't work for bucket B

### End-to-End - Single Bucket with JWT Auth
- [ ] Test: GET without JWT returns 401
- [ ] Test: GET with valid JWT returns object
- [ ] Test: GET with expired JWT returns 401
- [ ] Test: GET with invalid signature JWT returns 401
- [ ] Test: JWT from Authorization header works
- [ ] Test: JWT from query parameter works
- [ ] Test: JWT from custom header works

### End-to-End - Claims Verification
- [ ] Test: Valid JWT with correct claims returns object
- [ ] Test: Valid JWT with incorrect claims returns 403
- [ ] Test: Valid JWT with missing required claim returns 403
- [ ] Test: Multiple claim verification rules enforced

### End-to-End - Mixed Auth Configuration
- [ ] Test: Public bucket accessible without JWT
- [ ] Test: Private bucket requires JWT
- [ ] Test: Can access public and private buckets in same proxy instance
- [ ] Test: Auth configuration independent per bucket

### End-to-End - Error Scenarios
- [ ] Test: S3 connection timeout handled gracefully
- [ ] Test: Invalid S3 credentials return appropriate error
- [ ] Test: S3 bucket doesn't exist returns 404
- [ ] Test: Network error to S3 returns 502
- [ ] Test: All errors logged with sufficient context

### End-to-End - Concurrent Requests
- [ ] Test: Can handle 100 concurrent requests
- [ ] Test: Can handle 1000 concurrent requests
- [ ] Test: No race conditions with shared state
- [ ] Test: Memory usage reasonable under concurrent load
- [ ] Test: No credential leakage between concurrent requests

### End-to-End - Large File Streaming
- [ ] Test: Can stream 100MB file
- [ ] Test: Can stream 1GB file (if system allows)
- [ ] Test: Memory usage stays constant during large file stream
- [ ] Test: Client disconnect stops streaming immediately
- [ ] Test: Multiple concurrent large file streams work correctly

---

## Phase 8: Performance and Optimization

### Performance - Benchmarks
- [ ] Test: JWT validation completes in <1ms
- [ ] Test: Path routing completes in <10μs
- [ ] Test: S3 signature generation completes in <100μs
- [ ] Test: Request handling end-to-end <100ms P95 (cached)
- [ ] Test: Request handling end-to-end <500ms P95 (S3)
- [ ] Test: Throughput >10,000 req/s on test hardware

### Performance - Resource Usage
- [ ] Test: Memory usage <500MB for idle proxy
- [ ] Test: Memory usage scales linearly with connections
- [ ] Test: CPU usage <50% under moderate load
- [ ] Test: No memory leaks over 1 hour stress test
- [ ] Test: No file descriptor leaks

### Performance - Scalability
- [ ] Test: Performance degrades gracefully under overload
- [ ] Test: System remains responsive at 2x expected load
- [ ] Test: Can handle 10,000 concurrent connections
- [ ] Test: Horizontal scaling works (multiple proxy instances)

### Optimization - Code Efficiency
- [ ] Benchmark: Compare before/after optimization changes
- [ ] Test: No unnecessary allocations in hot paths
- [ ] Test: No unnecessary string copies
- [ ] Test: Efficient use of async/await (no blocking)
- [ ] Test: Connection pooling for S3 requests

---

## Phase 9: Configuration Hot Reload

### Hot Reload - Infrastructure
- [ ] Test: Can detect configuration file changes
- [ ] Test: Can reload configuration on SIGHUP signal
- [ ] Test: Can reload configuration via management API endpoint
- [ ] Test: Validates new configuration before applying
- [ ] Test: Rejects invalid configuration during reload

### Hot Reload - Safe Updates
- [ ] Test: In-flight requests complete with old config
- [ ] Test: New requests use new config immediately after reload
- [ ] Test: No dropped connections during reload
- [ ] Test: No race conditions during config swap
- [ ] Test: Atomic config update (all or nothing)

### Hot Reload - Credential Rotation
- [ ] Test: Can update S3 credentials via reload
- [ ] Test: Can update JWT secret via reload
- [ ] Test: Old credentials continue working during grace period
- [ ] Test: New credentials work immediately after reload
- [ ] Test: Logs successful credential rotation

### Hot Reload - Error Handling
- [ ] Test: Failed reload doesn't affect running service
- [ ] Test: Failed reload logs clear error message
- [ ] Test: Can retry failed reload after fixing config
- [ ] Test: Service continues with old config if reload fails

---

## Phase 10: Observability and Monitoring

### Logging - Request Logging
- [ ] Test: Logs all incoming requests with timestamp
- [ ] Test: Logs request method, path, status code
- [ ] Test: Logs request duration
- [ ] Test: Logs JWT subject (if authenticated)
- [ ] Test: Logs target bucket and S3 key
- [ ] Test: Logs unique request ID for correlation
- [ ] Test: Logs don't include sensitive data (tokens, credentials)

### Logging - Error Logging
- [ ] Test: Logs all errors with stack traces
- [ ] Test: Logs auth failures with reason
- [ ] Test: Logs S3 errors with response details
- [ ] Test: Logs configuration errors on startup
- [ ] Test: Error logs include request context
- [ ] Test: Logs are structured JSON format

### Metrics - Request Metrics
- [ ] Test: Exports request count by status code
- [ ] Test: Exports request duration histogram
- [ ] Test: Exports requests per bucket
- [ ] Test: Exports requests per route
- [ ] Test: Exports concurrent request gauge
- [ ] Test: Exports total bytes transferred

### Metrics - System Metrics
- [ ] Test: Exports memory usage
- [ ] Test: Exports CPU usage
- [ ] Test: Exports open file descriptors
- [ ] Test: Exports Tokio task metrics
- [ ] Test: Exports connection pool metrics

### Metrics - Business Metrics
- [ ] Test: Exports authentication success/failure rate
- [ ] Test: Exports S3 request count by operation
- [ ] Test: Exports S3 error rate
- [ ] Test: Exports cache hit/miss rate (if caching enabled)

### Metrics - Prometheus Format
- [ ] Test: Metrics endpoint returns Prometheus text format
- [ ] Test: Metrics endpoint accessible at /metrics
- [ ] Test: Metrics include proper labels
- [ ] Test: Metrics include help text
- [ ] Test: Metrics include type metadata

---

## Phase 11: Production Readiness

### Health Checks
- [ ] Test: Health check endpoint returns 200 when healthy
- [ ] Test: Health check endpoint returns 503 when unhealthy
- [ ] Test: Health check verifies S3 connectivity
- [ ] Test: Health check verifies configuration loaded
- [ ] Test: Health check is fast (<100ms)
- [ ] Test: Liveness check (basic aliveness)
- [ ] Test: Readiness check (ready to serve traffic)

### Graceful Shutdown
- [ ] Test: Responds to SIGTERM signal
- [ ] Test: Stops accepting new connections
- [ ] Test: Waits for in-flight requests to complete
- [ ] Test: Closes S3 connections gracefully
- [ ] Test: Shutdown timeout works (force close after N seconds)
- [ ] Test: Logs shutdown events

### Error Recovery
- [ ] Test: Recovers from panics without crashing
- [ ] Test: Recovers from temporary S3 outages
- [ ] Test: Implements retry with exponential backoff
- [ ] Test: Implements circuit breaker for failing S3 buckets
- [ ] Test: Circuit breaker opens after threshold failures
- [ ] Test: Circuit breaker closes after cooldown period

### Security Hardening
- [ ] Test: No credentials logged anywhere
- [ ] Test: No sensitive data in error messages
- [ ] Test: No stack traces to clients (only in logs)
- [ ] Test: Request size limits enforced
- [ ] Test: Request timeout enforced
- [ ] Test: Rate limiting per client (optional feature)
- [ ] Test: TLS configuration validated
- [ ] Test: Headers sanitized before logging

### Documentation
- [ ] README with setup instructions complete
- [ ] Configuration reference documentation complete
- [ ] API documentation complete
- [ ] Architecture documentation complete
- [ ] Deployment guide complete
- [ ] Troubleshooting guide complete
- [ ] Security considerations documented

### Final Validation
- [ ] All tests passing (unit, integration, e2e)
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Test coverage >90%
- [ ] Performance benchmarks meet requirements
- [ ] Security review completed
- [ ] Documentation reviewed and accurate
- [ ] Example configurations tested and working

---

## Notes and Decisions

### Design Decisions

**Decision:** Use Pingora instead of plain Tokio+Hyper  
**Rationale:** Pingora provides better performance, connection pooling, and production-ready proxy features out of the box

**Decision:** One S3 client per bucket configuration  
**Rationale:** Complete credential isolation, no risk of using wrong credentials for a bucket

**Decision:** JWT validation only (no issuance)  
**Rationale:** Keeps proxy focused; JWT issuance is handled by identity provider

**Decision:** Read-only S3 operations for v1.0  
**Rationale:** Simpler implementation, most use cases are read-heavy

**Decision:** Synchronous config reload (not automatic file watching)  
**Rationale:** More predictable, operator controls when reload happens

### Technical Decisions

**Decision:** Use `jsonwebtoken` crate  
**Rationale:** Well-maintained, widely used, supports all common algorithms

**Decision:** Use AWS SDK for Rust over rusoto  
**Rationale:** Official SDK, better maintained, async-first design

**Decision:** YAML for configuration  
**Rationale:** More readable than JSON, less verbose than TOML for nested structures

**Decision:** Structured JSON logging  
**Rationale:** Better for log aggregation systems (ELK, Datadog, etc.)

### Blocked Items

None currently

### Questions/Clarifications Needed

- [ ] Should we support S3 LIST operations in v1.0?
- [ ] Should caching be in v1.0 or deferred to v1.1?
- [ ] What's the expected maximum number of buckets per proxy instance?
- [ ] Do we need multi-region S3 support in v1.0?

---

## Completed Phases

*As phases are completed, move them here with completion date and summary*

---

## Test Execution Commands

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test file
cargo test --test integration_auth

# Run tests with coverage (using tarpaulin)
cargo tarpaulin --out Html --output-dir coverage

# Run tests with output (see println!)
cargo test -- --nocapture

# Run only fast tests (excluding long-running e2e tests)
cargo test --lib && cargo test --test integration_*

# Run benchmarks
cargo bench

# Run with specific test pattern
cargo test jwt_validation

# Run clippy
cargo clippy -- -D warnings

# Run formatter check
cargo fmt -- --check

# Build release binary
cargo build --release
```

### Performance Testing

```bash
# Run with perf profiling (Linux only)
cargo build --release
perf record --call-graph dwarf ./target/release/yatagarasu
perf report

# Load testing with wrk
wrk -t12 -c400 -d30s http://localhost:8080/products/test.txt

# Load testing with hey
hey -n 100000 -c 100 -m GET http://localhost:8080/products/test.txt
```

### Integration Test Setup

```bash
# Start MinIO for integration tests
docker run -d -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  minio/minio server /data --console-address ":9001"

# Run integration tests with MinIO
TEST_S3_ENDPOINT=http://localhost:9000 cargo test --test integration_*

# Stop MinIO
docker stop $(docker ps -q --filter ancestor=minio/minio)
```

---

## Development Workflow Reminder

**The TDD Rhythm:**
1. 🔴 **Red** - Write a failing test
2. 🟢 **Green** - Make it pass with minimum code
3. 🔵 **Refactor** - Clean up while keeping tests green
4. 💾 **Commit** - Commit with appropriate [STRUCTURAL] or [BEHAVIORAL] prefix
5. 🔄 **Repeat** - Next test

**Key Principles:**
- One test at a time
- Minimum code to pass
- Refactor only when green
- Separate structural from behavioral commits
- Run all tests after each change
- No commits with failing tests
- No commits with warnings

**When Claude says "go":**
1. Claude reads this plan.md
2. Claude finds next `[ ]` test
3. Claude implements test (Red)
4. Claude implements minimum code (Green)
5. Claude refactors if needed (Refactor)
6. Claude marks test `[x]` and commits
7. Claude asks for next "go" command

**Quality Gates:**
- ✅ All tests must pass
- ✅ No compiler warnings
- ✅ No clippy warnings
- ✅ Code formatted with rustfmt
- ✅ Test coverage >90%
- ✅ Benchmarks meet performance targets

Let's build Yatagarasu one test at a time! 🚀
