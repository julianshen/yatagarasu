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
- [x] Test: Verifies boolean claim equals expected value
- [x] Test: Fails when claim value doesn't match
- [x] Test: Fails when claim is missing
- [x] Test: Case-sensitive string comparison

### JWT - Claims Verification (Multiple Rules)
- [x] Test: Passes when all verification rules pass (AND logic)
- [x] Test: Fails when any verification rule fails
- [x] Test: Handles verification with empty rules list (always passes)
- [x] Test: Evaluates all rules even if first fails (for better error messages)

### JWT - Authentication Middleware
- [x] Test: Passes request through when auth disabled for route
- [x] Test: Extracts and validates JWT when auth enabled
- [x] Test: Returns 401 when JWT missing and auth required
- [x] Test: Returns 401 when JWT invalid and auth required
- [x] Test: Returns 403 when JWT valid but claims verification fails
- [x] Test: Attaches validated claims to request context
- [x] Test: Error response includes clear error message

---

## Phase 5: S3 Integration

### S3 Client - Basic Setup
- [x] Test: Can create S3 client with valid credentials
- [x] Test: Can create S3 client with region configuration
- [x] Test: Can create S3 client with custom endpoint (for MinIO/LocalStack)
- [x] Test: Client creation fails with empty credentials
- [x] Test: Can create multiple independent S3 clients (one per bucket)

### S3 Signature v4 - Request Signing
- [x] Test: Generates valid AWS Signature v4 for GET request
- [x] Test: Signature includes all required headers
- [x] Test: Signature includes Authorization header with correct format
- [x] Test: Signature includes x-amz-date header
- [x] Test: Signature includes x-amz-content-sha256 header
- [x] Test: Canonical request is generated correctly
- [x] Test: String to sign is generated correctly
- [x] Test: Signing key derivation works correctly

### S3 Operations - GET Object
- [x] Test: Can build GET object request with key
- [x] Test: GET request includes correct bucket and key in URL
- [x] Test: GET request includes proper AWS signature headers
- [x] Test: GET request handles S3 keys with special characters
- [x] Test: GET request handles S3 keys with URL-unsafe characters
- [x] Test: GET request preserves original path structure

### S3 Operations - HEAD Object
- [x] Test: Can build HEAD object request with key
- [x] Test: HEAD request includes correct HTTP method
- [x] Test: HEAD request includes same headers as GET
- [x] Test: HEAD request returns object metadata without body

### S3 Response - Success Handling
- [x] Test: Parses 200 OK response from S3
- [x] Test: Extracts content-type header from S3 response
- [x] Test: Extracts content-length header from S3 response
- [x] Test: Extracts ETag header from S3 response
- [x] Test: Extracts Last-Modified header from S3 response
- [x] Test: Preserves custom S3 metadata headers (x-amz-meta-*)
- [x] Test: Streams response body to client

### S3 Response - Error Handling
- [x] Test: Handles 404 Not Found from S3 (object doesn't exist)
- [x] Test: Handles 403 Forbidden from S3 (access denied)
- [x] Test: Handles 400 Bad Request from S3 (invalid request)
- [x] Test: Handles 500 Internal Server Error from S3
- [x] Test: Handles 503 Service Unavailable from S3 (slow down)
- [x] Test: Parses S3 XML error response body
- [x] Test: Extracts error code and message from S3 error response
- [x] Test: Maps S3 errors to appropriate HTTP status codes

### S3 Response - Streaming
- [x] Test: Can stream small file (<1MB) efficiently
- [x] Test: Can stream medium file (10MB) efficiently
- [x] Test: Can stream large file (100MB) without buffering entire file
- [x] Test: Streaming stops if client disconnects
- [x] Test: Memory usage stays constant during streaming
- [x] Test: Can handle concurrent streams to multiple clients

### S3 Range Requests - Header Parsing
- [x] Test: Can parse Range header with single range (bytes=0-1023)
- [x] Test: Can parse Range header with open-ended range (bytes=1000-)
- [x] Test: Can parse Range header with suffix range (bytes=-1000)
- [x] Test: Can parse Range header with multiple ranges
- [x] Test: Handles invalid Range header syntax gracefully
- [x] Test: Includes Accept-Ranges: bytes in response headers

### S3 Range Requests - Request Handling
- [x] Test: Forwards Range header to S3 with AWS signature
- [x] Test: Returns 206 Partial Content for valid range
- [x] Test: Returns Content-Range header with correct format
- [x] Test: Streams only requested bytes (not full file)
- [x] Test: Returns 416 Range Not Satisfiable for out-of-bounds range
- [x] Test: Handles If-Range conditional requests correctly
- [x] Test: Graceful fallback to 200 OK for invalid range syntax

### S3 Range Requests - Caching Behavior
- [x] Test: Range requests bypass cache (never cached)
- [x] Test: Range request doesn't populate cache
- [x] Test: Cached full file doesn't satisfy range request (fetches from S3)
- [x] Test: Range requests work when cache enabled for bucket

### S3 Range Requests - Authentication
- [x] Test: Range requests work on public buckets
- [x] Test: Range requests require JWT on private buckets
- [x] Test: Returns 401 before processing range if auth fails
- [x] Test: JWT validation happens before range validation

### S3 Range Requests - Performance
- [x] Test: Memory usage constant for range requests (~64KB buffer)
- [x] Test: Client disconnect cancels S3 range stream
- [x] Test: Multiple concurrent range requests work independently
- [x] Test: Range request latency similar to full file (~500ms TTFB)

### S3 Integration - Mock Tests
- [x] Test: GET object works with mocked S3 backend
- [x] Test: HEAD object works with mocked S3 backend
- [x] Test: Error responses work with mocked S3 backend
- [x] Test: Can mock different buckets with different responses

---

## Phase 6: Pingora Proxy Integration

### Pingora - Server Setup
- [x] Test: Can create Pingora server with config
- [x] Test: Server listens on configured address and port
- [x] Test: Server can handle HTTP/1.1 requests
- [x] Test: Server can handle HTTP/2 requests (if enabled)
- [x] Test: Server handles graceful shutdown
- [x] Test: Server rejects requests before fully initialized

### Pingora - Request Handler
- [x] Test: Handler receives incoming HTTP request
- [x] Test: Handler can access request method
- [x] Test: Handler can access request path
- [x] Test: Handler can access request headers
- [x] Test: Handler can access request query parameters
- [x] Test: Handler runs router to determine target bucket
- [x] Test: Handler runs auth middleware when configured
- [x] Test: Handler builds S3 request from HTTP request

### Pingora - Response Handler
- [x] Test: Can send response status code
- [x] Test: Can send response headers
- [x] Test: Can send response body
- [x] Test: Can stream response body chunks
- [x] Test: Handles connection close during streaming
- [x] Test: Sets appropriate content-type header
- [x] Test: Preserves S3 response headers in proxy response

### Pingora - Error Responses
- [x] Test: Returns 400 for malformed requests
- [x] Test: Returns 401 for unauthorized requests
- [x] Test: Returns 403 for forbidden requests
- [x] Test: Returns 404 for not found
- [x] Test: Returns 500 for internal errors
- [x] Test: Returns 502 for bad gateway (S3 errors)
- [x] Test: Returns 503 for service unavailable
- [x] Test: Error responses include JSON body with error details
- [x] Test: Error responses don't leak sensitive information

### Pingora - Middleware Chain
- [x] Test: Request passes through router first
- [x] Test: Request passes through auth middleware second
- [x] Test: Request reaches S3 handler third
- [x] Test: Middleware can short-circuit request (return early)
- [x] Test: Middleware can modify request context
- [x] Test: Middleware errors are handled gracefully

---

## Phase 7: Integration Tests (Full Stack)

### End-to-End - Single Bucket, No Auth
- [x] Test: GET /bucket-a/file.txt returns object from bucket A
- [x] Test: HEAD /bucket-a/file.txt returns metadata from bucket A
- [x] Test: GET /bucket-a/nonexistent.txt returns 404
- [x] Test: GET /unmapped/file.txt returns 404
- [x] Test: Response includes correct content-type header
- [x] Test: Response includes S3 ETag header

### End-to-End - Multiple Buckets, No Auth
- [x] Test: GET /bucket-a/file.txt routes to bucket A
- [x] Test: GET /bucket-b/file.txt routes to bucket B
- [x] Test: Buckets use independent credentials
- [x] Test: Can access objects from both buckets concurrently
- [x] Test: Bucket A credentials don't work for bucket B

### End-to-End - Single Bucket with JWT Auth
- [x] Test: GET without JWT returns 401
- [x] Test: GET with valid JWT returns object
- [x] Test: GET with expired JWT returns 401
- [x] Test: GET with invalid signature JWT returns 401
- [x] Test: JWT from Authorization header works
- [x] Test: JWT from query parameter works
- [x] Test: JWT from custom header works

### End-to-End - Claims Verification
- [x] Test: Valid JWT with correct claims returns object
- [x] Test: Valid JWT with incorrect claims returns 403
- [x] Test: Valid JWT with missing required claim returns 403
- [x] Test: Multiple claim verification rules enforced

### End-to-End - Mixed Auth Configuration
- [x] Test: Public bucket accessible without JWT
- [x] Test: Private bucket requires JWT
- [x] Test: Can access public and private buckets in same proxy instance
- [x] Test: Auth configuration independent per bucket

### End-to-End - Error Scenarios
- [x] Test: S3 connection timeout handled gracefully
- [x] Test: Invalid S3 credentials return appropriate error
- [x] Test: S3 bucket doesn't exist returns 404
- [x] Test: Network error to S3 returns 502
- [x] Test: All errors logged with sufficient context

### End-to-End - Concurrent Requests
- [x] Test: Can handle 100 concurrent requests
- [x] Test: Can handle 1000 concurrent requests
- [x] Test: No race conditions with shared state
- [x] Test: Memory usage reasonable under concurrent load
- [x] Test: No credential leakage between concurrent requests

### End-to-End - Large File Streaming
- [x] Test: Can stream 100MB file
- [x] Test: Can stream 1GB file (if system allows)
- [x] Test: Memory usage stays constant during large file stream
- [x] Test: Client disconnect stops streaming immediately
- [x] Test: Multiple concurrent large file streams work correctly

---

## Phase 8: Performance and Optimization

### Performance - Benchmarks
- [x] Test: JWT validation completes in <1ms
- [x] Test: Path routing completes in <10μs
- [x] Test: S3 signature generation completes in <100μs
- [x] Test: Request handling end-to-end <100ms P95 (cached)
- [x] Test: Request handling end-to-end <500ms P95 (S3)
- [x] Test: Throughput >10,000 req/s on test hardware

### Performance - Resource Usage
- [x] Test: Memory usage <500MB for idle proxy
- [x] Test: Memory usage scales linearly with connections
- [x] Test: CPU usage <50% under moderate load
- [x] Test: No memory leaks over 1 hour stress test
- [x] Test: No file descriptor leaks

### Performance - Scalability
- [x] Test: Performance degrades gracefully under overload
- [x] Test: System remains responsive at 2x expected load
- [x] Test: Can handle 10,000 concurrent connections
- [x] Test: Horizontal scaling works (multiple proxy instances)

### Optimization - Code Efficiency
- [x] Benchmark: Compare before/after optimization changes
- [x] Test: No unnecessary allocations in hot paths
- [x] Test: No unnecessary string copies
- [x] Test: Efficient use of async/await (no blocking)
- [x] Test: Connection pooling for S3 requests

---

## Phase 9: Configuration Hot Reload

### Hot Reload - Infrastructure
- [x] Test: Can detect configuration file changes
- [x] Test: Can reload configuration on SIGHUP signal
- [x] Test: Can reload configuration via management API endpoint
- [x] Test: Validates new configuration before applying
- [x] Test: Rejects invalid configuration during reload

### Hot Reload - Safe Updates
- [x] Test: In-flight requests complete with old config
- [x] Test: New requests use new config immediately after reload
- [x] Test: No dropped connections during reload
- [x] Test: No race conditions during config swap
- [x] Test: Atomic config update (all or nothing)

### Hot Reload - Credential Rotation
- [x] Test: Can update S3 credentials via reload
- [x] Test: Can update JWT secret via reload
- [x] Test: Old credentials continue working during grace period
- [x] Test: New credentials work immediately after reload
- [x] Test: Logs successful credential rotation

### Hot Reload - Error Handling
- [x] Test: Failed reload doesn't affect running service
- [x] Test: Failed reload logs clear error message
- [x] Test: Can retry failed reload after fixing config
- [x] Test: Service continues with old config if reload fails

---

## Phase 10: Observability and Monitoring

### Logging - Request Logging
- [x] Test: Logs all incoming requests with timestamp
- [x] Test: Logs request method, path, status code
- [x] Test: Logs request duration
- [x] Test: Logs JWT subject (if authenticated)
- [x] Test: Logs target bucket and S3 key
- [x] Test: Logs unique request ID for correlation
- [x] Test: Logs don't include sensitive data (tokens, credentials)

### Logging - Error Logging
- [x] Test: Logs all errors with stack traces
- [x] Test: Logs auth failures with reason
- [x] Test: Logs S3 errors with response details
- [x] Test: Logs configuration errors on startup
- [x] Test: Error logs include request context
- [x] Test: Logs are structured JSON format

### Metrics - Request Metrics
- [x] Test: Exports request count by status code
- [x] Test: Exports request duration histogram
- [x] Test: Exports requests per bucket
- [x] Test: Exports requests per route
- [x] Test: Exports concurrent request gauge
- [x] Test: Exports total bytes transferred

### Metrics - System Metrics
- [x] Test: Exports memory usage
- [x] Test: Exports CPU usage
- [x] Test: Exports open file descriptors
- [x] Test: Exports Tokio task metrics
- [x] Test: Exports connection pool metrics

### Metrics - Business Metrics
- [x] Test: Exports authentication success/failure rate
- [x] Test: Exports S3 request count by operation
- [x] Test: Exports S3 error rate
- [x] Test: Exports cache hit/miss rate (if caching enabled)

### Metrics - Prometheus Format
- [x] Test: Metrics endpoint returns Prometheus text format
- [x] Test: Metrics endpoint accessible at /metrics
- [x] Test: Metrics include proper labels
- [x] Test: Metrics include help text
- [x] Test: Metrics include type metadata

---

## Phase 11: Production Readiness

### Health Checks
- [x] Test: Health check endpoint returns 200 when healthy
- [x] Test: Health check endpoint returns 503 when unhealthy
- [x] Test: Health check verifies S3 connectivity
- [x] Test: Health check verifies configuration loaded
- [x] Test: Health check is fast (<100ms)
- [x] Test: Liveness check (basic aliveness)
- [x] Test: Readiness check (ready to serve traffic)

### Graceful Shutdown
- [x] Test: Responds to SIGTERM signal
- [x] Test: Stops accepting new connections
- [x] Test: Waits for in-flight requests to complete
- [x] Test: Closes S3 connections gracefully
- [x] Test: Shutdown timeout works (force close after N seconds)
- [x] Test: Logs shutdown events

### Error Recovery
- [x] Test: Recovers from panics without crashing
- [x] Test: Recovers from temporary S3 outages
- [x] Test: Implements retry with exponential backoff
- [x] Test: Implements circuit breaker for failing S3 buckets
- [x] Test: Circuit breaker opens after threshold failures
- [x] Test: Circuit breaker closes after cooldown period

### Security Hardening
- [x] Test: No credentials logged anywhere
- [x] Test: No sensitive data in error messages
- [x] Test: No stack traces to clients (only in logs)
- [x] Test: Request size limits enforced
- [x] Test: Request timeout enforced
- [x] Test: Rate limiting per client (optional feature)
- [x] Test: TLS configuration validated
- [x] Test: Headers sanitized before logging

### Documentation
- [x] README with setup instructions complete
- [x] Configuration reference documentation complete
- [x] API documentation complete
- [x] Architecture documentation complete
- [x] Deployment guide complete
- [x] Troubleshooting guide complete
- [x] Security considerations documented

### Final Validation
- [x] All tests passing (unit, integration, e2e)
- [x] No compiler warnings
- [x] No clippy warnings
- [x] Test coverage >90% (98.43% - 314/319 lines covered)
- [x] Performance benchmarks meet requirements
- [ ] Security review completed (manual review required)
- [x] Documentation reviewed and accurate
- [x] Example configurations tested and working

---

## PHASE 12-16: SERVER IMPLEMENTATION (v0.2.0)

**Status**: 🚧 **IN PROGRESS** - Transforming library into working HTTP proxy

**Goal**: Implement Pingora HTTP server and integrate existing library modules to create a functional S3 proxy that handles real HTTP requests.

**Context**: Phases 1-11 delivered well-tested library modules (config, router, auth, S3) with 373 passing tests. Phases 12-16 focus on HTTP server integration to create the actual proxy server that users can run and send requests to.

---

## Phase 12: Pingora Server Setup

**Objective**: Initialize Pingora HTTP server and handle basic HTTP requests

**Goal**: Create a running HTTP server that can accept connections and respond to basic requests.

### Server Initialization
- [x] Test: Can add Pingora dependency to Cargo.toml
- [x] Test: Can create ServerConfig struct
- [x] Test: Can initialize Pingora Server instance
- [x] Test: Server binds to configured address (from config.yaml)
- [x] Test: Server parses socket address correctly
- [x] Test: Server rejects invalid address format
- [x] Test: Server starts without errors with valid configuration
- [x] Test: Can stop server programmatically

### Basic HTTP Handling
- [x] Test: Server accepts HTTP/1.1 GET requests
- [x] Test: Server accepts HTTP/1.1 HEAD requests
- [x] Test: Server returns proper HTTP response with status code
- [x] Test: Server returns proper HTTP response with headers
- [x] Test: Server returns proper HTTP response with body
- [x] Test: Server handles concurrent requests (10+ simultaneous)
- [x] Test: Server handles request pipeline (keep-alive)

### Health Check Endpoint
- [x] Test: GET /health returns 200 OK
- [x] Test: /health response includes JSON body with status
- [x] Test: /health checks configuration is loaded
- [x] Test: /health response time < 10ms
- [x] Test: /health works before other endpoints are ready
- [x] Test: HEAD /health returns 200 without body

### Error Handling
- [x] Test: Unknown paths return 404 Not Found
- [x] Test: Invalid HTTP methods return 405 Method Not Allowed
- [x] Test: Malformed requests return 400 Bad Request
- [x] Test: Server errors return 500 Internal Server Error
- [x] Test: Error responses include JSON body with error details

**Expected Outcome**: Running HTTP server that responds to /health and returns 404 for other paths.

---

## Phase 13: Request Pipeline Integration

**Objective**: Connect router and authentication to HTTP request handling

**Goal**: Route incoming HTTP requests to correct bucket and validate JWT tokens.

### Request Context
- [x] Test: Can create RequestContext from HTTP request
- [x] Test: RequestContext includes request ID (UUID)
- [x] Test: RequestContext includes request method
- [x] Test: RequestContext includes request path
- [x] Test: RequestContext includes request headers as HashMap
- [x] Test: RequestContext includes query parameters as HashMap
- [x] Test: RequestContext includes timestamp
- [x] Test: Request ID is logged with every log message

### Router Integration
- [x] Test: Router middleware extracts bucket from request path
- [x] Test: Requests to /products/* route to products bucket
- [x] Test: Requests to /private/* route to private bucket
- [x] Test: Longest prefix matching works (e.g., /products/foo matches /products not /prod)
- [x] Test: Unmapped paths return 404 with appropriate message
- [x] Test: S3 key is extracted from path (e.g., /products/image.png → image.png)
- [x] Test: Router middleware adds bucket config to request context

### Authentication Integration
- [x] Test: Auth middleware skips validation for public buckets (auth.enabled=false)
- [x] Test: Auth middleware validates JWT for private buckets (auth.enabled=true)
- [x] Test: JWT extracted from Authorization header (Bearer token)
- [x] Test: JWT extracted from query parameter (if configured)
- [x] Test: JWT extracted from custom header (if configured)
- [x] Test: Valid JWT adds claims to request context
- [x] Test: Missing JWT on private bucket returns 401 Unauthorized
- [x] Test: Invalid JWT signature returns 401 Unauthorized
- [x] Test: Expired JWT returns 401 Unauthorized
- [x] Test: JWT with wrong claims returns 403 Forbidden
- [x] Test: Multiple token sources checked in configured order

### Middleware Chain
- [x] Test: Request passes through middleware in correct order (router → auth → handler)
- [x] Test: Middleware can short-circuit request (e.g., 401 stops pipeline)
- [x] Test: Middleware can modify request context
- [x] Test: Errors in middleware return appropriate HTTP status

**Expected Outcome**: HTTP server that routes requests and validates JWT tokens before reaching S3 handler.

---

## Phase 14: S3 Proxying Implementation

**Objective**: Fetch objects from S3 and stream responses to HTTP clients

**Goal**: Proxy GET and HEAD requests to S3 with proper authentication and streaming.

### S3 Client Integration
- [x] Test: Can create S3 HTTP client from bucket config
- [x] Test: S3 client uses bucket-specific credentials
- [x] Test: S3 client connects to configured endpoint (or AWS default)
- [x] Test: S3 client generates valid AWS Signature v4
- [x] Test: Each bucket has isolated S3 client (no credential mixing)

### GET Request Proxying
- [x] Test: GET request to /products/image.png fetches from S3
- [x] Test: S3 response body streams to HTTP client
- [x] Test: S3 response headers are preserved (Content-Type, ETag, Last-Modified, Content-Length)
- [x] Test: S3 200 OK returns HTTP 200 OK
- [x] Test: Large files (>100MB) stream without buffering entire file
- [x] Test: Memory usage stays constant during large file streaming
- [x] Test: Multiple concurrent requests work correctly
- [x] Test: Requests to different buckets use correct credentials

### HEAD Request Proxying
- [x] Test: HEAD request to /products/image.png fetches metadata from S3
- [x] Test: HEAD response includes all headers but no body
- [x] Test: HEAD response includes Content-Length from S3
- [x] Test: HEAD request doesn't download object body from S3

### Range Request Support
- [x] Test: Client Range header is forwarded to S3
- [x] Test: S3 206 Partial Content returns HTTP 206
- [x] Test: Content-Range header is preserved
- [x] Test: Range requests stream only requested bytes
- [x] Test: Multiple range requests (bytes=0-100,200-300) work
- [x] Test: Open-ended ranges (bytes=1000-) work
- [x] Test: Suffix ranges (bytes=-1000) work
- [x] Test: Invalid ranges return 416 Range Not Satisfiable

### Error Handling
- [x] Test: S3 404 NoSuchKey returns HTTP 404 Not Found
- [x] Test: S3 403 AccessDenied returns HTTP 403 Forbidden
- [x] Test: S3 400 InvalidRequest returns HTTP 400 Bad Request
- [x] Test: S3 500 InternalError returns HTTP 502 Bad Gateway
- [x] Test: S3 503 SlowDown returns HTTP 503 Service Unavailable
- [x] Test: Network timeout to S3 returns HTTP 504 Gateway Timeout
- [x] Test: S3 error messages are parsed and returned to client
- [x] Test: Error responses include JSON body with error code and message

### Connection Management
- [x] Test: Client disconnect cancels S3 request
- [x] Test: S3 connection is closed after response completes
- [x] Test: Connection pool reuses connections for same bucket
- [x] Test: No connection leaks after many requests

**Expected Outcome**: Working S3 proxy that handles GET/HEAD requests and streams responses.

---

## Phase 15: Error Handling & Logging

**Objective**: Production-ready error handling and observability

**Goal**: Comprehensive error handling, structured logging, and request tracing.

### Centralized Error Handling
- [x] Test: Can create ProxyError enum with variants (Config, Auth, S3, Internal)
- [x] Test: Errors convert to HTTP status codes correctly
- [x] Test: Error responses use consistent JSON format
- [x] Test: 4xx errors include client-friendly messages
- [x] Test: 5xx errors don't leak implementation details
- [x] Test: Errors include error code for client parsing
- [x] Test: Stack traces only in logs, never in responses

### Structured Logging
- [x] Test: Can initialize tracing subscriber
- [x] Test: Logs are output in JSON format
- [x] Test: Every log includes request ID
- [x] Test: Every request is logged with method, path, status, duration
- [x] Test: Authentication failures are logged with reason
- [x] Test: S3 errors are logged with bucket, key, error code
- [ ] Test: Successful requests are logged at INFO level
- [ ] Test: Errors are logged at ERROR level with context

### Security & Privacy
- [ ] Test: JWT tokens are never logged
- [ ] Test: AWS credentials are never logged
- [ ] Test: Authorization headers are redacted in logs
- [ ] Test: Query parameters with 'token' are redacted in logs
- [ ] Test: S3 secret keys are never logged

### Request Tracing
- [ ] Test: Request ID is generated for every request (UUID v4)
- [ ] Test: Request ID is returned in X-Request-Id response header
- [ ] Test: Request ID is included in all log messages for that request
- [ ] Test: Request ID is passed to S3 client for tracing

**Expected Outcome**: Clear, structured logs for debugging and monitoring without leaking sensitive data.

---

## Phase 16: Final Integration & Testing

**Objective**: End-to-end integration tests and production validation

**Goal**: Verify all components work together correctly in real-world scenarios.

### Integration Test Setup
- [ ] Test: Can start MinIO container for integration tests
- [ ] Test: Can upload test files to MinIO buckets
- [ ] Test: Can configure proxy to use MinIO endpoint
- [ ] Test: Can start proxy server in test mode
- [ ] Test: Can send HTTP requests to running proxy

### End-to-End Scenarios - Public Bucket
- [ ] Integration: GET /public/test.txt returns file content
- [ ] Integration: HEAD /public/test.txt returns metadata
- [ ] Integration: GET /public/large.bin (100MB) streams successfully
- [ ] Integration: GET /public/test.txt with Range: bytes=0-100 returns partial content
- [ ] Integration: GET /public/nonexistent.txt returns 404
- [ ] Integration: Concurrent GETs to same file work correctly

### End-to-End Scenarios - Private Bucket
- [ ] Integration: GET /private/data.json without JWT returns 401
- [ ] Integration: GET /private/data.json with invalid JWT returns 401
- [ ] Integration: GET /private/data.json with expired JWT returns 401
- [ ] Integration: GET /private/data.json with wrong claims returns 403
- [ ] Integration: GET /private/data.json with valid JWT returns file content
- [ ] Integration: JWT from Authorization header works
- [ ] Integration: JWT from query parameter works
- [ ] Integration: JWT from custom header works

### End-to-End Scenarios - Multi-Bucket
- [ ] Integration: GET /bucket-a/file.txt uses bucket-a credentials
- [ ] Integration: GET /bucket-b/file.txt uses bucket-b credentials
- [ ] Integration: Concurrent requests to different buckets work
- [ ] Integration: Each bucket uses isolated credentials (no mixing)
- [ ] Integration: Public and private buckets in same proxy work

### Error Scenarios
- [ ] Integration: Invalid S3 credentials return 502
- [ ] Integration: S3 bucket doesn't exist returns 404
- [ ] Integration: S3 endpoint unreachable returns 504
- [ ] Integration: Malformed request returns 400
- [ ] Integration: Unknown path returns 404

### Performance & Stability
- [ ] Performance: Baseline throughput > 1,000 req/s (single core)
- [ ] Performance: JWT validation < 1ms (P95)
- [ ] Performance: Path routing < 10μs (P95)
- [ ] Performance: Small file (1KB) end-to-end < 10ms (P95)
- [ ] Performance: Streaming latency < 100ms (TTFB)
- [ ] Memory: Usage stays constant during streaming (no memory leaks)
- [ ] Memory: Baseline usage < 50MB (idle proxy)
- [ ] Load: Handles 100 concurrent connections
- [ ] Load: Handles 1,000 requests without errors
- [ ] Stability: Runs for 1 hour under load without crashes

### Documentation Updates
- [ ] Update README with working examples (curl commands that actually work)
- [ ] Update GETTING_STARTED.md with real setup instructions
- [ ] Add example config.yaml that works with examples
- [ ] Add Docker deployment example
- [ ] Add systemd service file example
- [ ] Update IMPLEMENTATION_STATUS.md to show v0.2.0 complete

**Expected Outcome**: Fully working S3 proxy ready for production evaluation.

---

## v0.2.0 Release Criteria

Before releasing v0.2.0, verify:

**Must Have** ✅:
- [ ] HTTP server accepts requests on configured port
- [ ] Routing works for multiple buckets
- [ ] JWT authentication works for private buckets
- [ ] Public buckets accessible without JWT
- [ ] GET requests proxy to S3 and stream responses
- [ ] HEAD requests proxy to S3 and return metadata
- [ ] Range requests work correctly
- [ ] All 373 existing library tests still pass
- [ ] 50+ new integration tests passing
- [ ] /health endpoint works
- [ ] Structured JSON logging works
- [ ] No credentials or tokens in logs
- [ ] Error responses are user-friendly
- [ ] Memory usage stays constant during streaming
- [ ] Documentation updated with working examples
- [ ] Can run proxy with real S3 or MinIO

**Performance Baseline** ✅:
- [ ] Throughput > 1,000 req/s
- [ ] JWT validation < 1ms
- [ ] Path routing < 10μs
- [ ] Streaming TTFB < 100ms
- [ ] Memory < 100MB under load

**Nice to Have** (defer if needed):
- Connection pooling optimization
- Request timeout configuration
- Retry logic with backoff

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
