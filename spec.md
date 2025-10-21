# Yatagarasu - Product Specification

## Overview

**Product Name:** Yatagarasu (八咫烏)  
**Version:** 1.0.0  
**Last Updated:** October 21, 2025  
**Original MVP:** https://github.com/julianshen/s3-envoy-proxy

Yatagarasu is a high-performance S3 proxy built with Cloudflare's Pingora framework and Rust. It provides intelligent routing, multi-bucket support, and flexible JWT authentication for S3 object storage access.

*Yatagarasu (八咫烏) is the three-legged crow in Japanese mythology that serves as a divine guide and messenger. Like its namesake, this proxy guides and securely routes requests to the appropriate S3 buckets.*

## Purpose and Goals

### Primary Purpose
Reimplement the S3 Envoy proxy MVP with enhanced performance, security, and flexibility using Pingora and Rust, providing a production-ready S3 proxy solution with advanced authentication and multi-bucket routing capabilities.

### Key Goals
- **Performance**: Leverage Pingora's architecture for 70% lower CPU usage compared to traditional proxies
- **Multi-tenancy**: Support multiple S3 buckets with isolated credentials on different paths
- **Security**: Implement flexible, optional JWT authentication with custom claims verification
- **Flexibility**: Allow configurable JWT token sources (query params, headers, custom headers)
- **Reliability**: Provide production-grade error handling and logging
- **Maintainability**: Follow TDD principles for high code quality and test coverage

### Success Criteria
- Successfully proxy requests to multiple S3 buckets with path-based routing
- JWT authentication working with all three token sources (query, auth header, custom header)
- Performance meets or exceeds Envoy-based MVP with lower resource usage
- 90%+ test coverage with comprehensive integration tests
- Zero-downtime credential rotation support
- Clear, actionable error messages for troubleshooting

## Target Users

### Primary Users
DevOps Engineers and Platform Teams who need to provide secure, multi-tenant S3 access to their applications and services.

### User Personas

1. **Platform Engineer**
   - Role: Infrastructure and platform services
   - Goals: 
     - Provide secure S3 access to multiple teams/applications
     - Manage credentials centrally without exposing AWS keys to applications
     - Monitor and control S3 access patterns
   - Pain Points: 
     - Managing separate S3 credentials for each application
     - Implementing consistent authentication across services
     - Performance overhead of existing proxy solutions

2. **Security Engineer**
   - Role: Security and compliance
   - Goals:
     - Enforce authentication on S3 access
     - Implement fine-grained access control with JWT claims
     - Audit S3 access patterns
   - Pain Points:
     - Direct S3 access bypasses authentication
     - Limited ability to verify user context beyond AWS IAM
     - Need for custom authorization logic

3. **Application Developer**
   - Role: Backend/frontend development
   - Goals:
     - Access S3 objects without managing AWS credentials
     - Simple HTTP-based S3 access
     - Fast, reliable object retrieval
   - Pain Points:
     - Complex AWS SDK integration
     - Credential management and rotation
     - Testing S3 integration locally

## Functional Requirements

### Core Features

#### Feature 1: Multi-Bucket Path Routing
**Priority:** High  
**Description:** Route incoming HTTP requests to different S3 buckets based on URL path prefixes. Each bucket has its own AWS credentials (access key and secret key) for isolated access control.

**User Stories:**
- As a platform engineer, I want to map `/bucket-a/*` to S3 bucket A and `/bucket-b/*` to S3 bucket B, so that I can serve content from multiple buckets through a single proxy endpoint
- As a developer, I want to access objects via simple HTTP paths like `/products/image.png` without knowing the underlying S3 bucket name or credentials
- As an administrator, I want to configure bucket mappings without code changes, so that I can add or modify buckets dynamically

**Acceptance Criteria:**
- [ ] Given a request to `/bucket-a/file.txt`, when the proxy receives it, then it routes to the configured S3 bucket A with bucket A's credentials
- [ ] Given a request to `/bucket-b/data.json`, when the proxy receives it, then it routes to the configured S3 bucket B with bucket B's credentials
- [ ] Given multiple path prefixes configured, when requests arrive, then each routes to its correct bucket independently
- [ ] Given a request to an unmapped path, when the proxy receives it, then it returns a 404 with clear error message
- [ ] Given a configuration file with bucket mappings, when the proxy starts, then it loads all mappings successfully
- [ ] Given bucket credentials, when making S3 requests, then credentials are properly isolated per bucket (no credential leakage)

**Dependencies:** Pingora HTTP proxy framework, AWS SDK or rusoto for S3 signing

---

#### Feature 2: Flexible JWT Authentication
**Priority:** High  
**Description:** Implement optional, configurable JWT authentication that can extract tokens from multiple sources and verify custom claims with configurable logic.

**User Stories:**
- As a security engineer, I want to require JWT authentication for specific paths/buckets, so that only authorized users can access sensitive S3 objects
- As a developer, I want JWT tokens accepted from query parameters (`?token=xxx`), Authorization header (`Bearer xxx`), or custom headers (`X-Auth-Token: xxx`), so that I can support different client types
- As an administrator, I want to configure custom JWT claims verification (e.g., `role=admin`, `tenant=acme`), so that I can implement fine-grained access control
- As a platform engineer, I want JWT authentication to be optional per bucket/path, so that public and private content can coexist

**Acceptance Criteria:**
- [ ] Given JWT in query parameter `?token=xxx`, when configured, then token is extracted and validated
- [ ] Given JWT in Authorization header `Bearer xxx`, when configured, then token is extracted and validated
- [ ] Given JWT in custom header `X-Auth-Token: xxx`, when configured, then token is extracted and validated
- [ ] Given multiple token sources configured, when any valid token is found, then authentication succeeds
- [ ] Given custom claims verification rules (e.g., `role=admin`), when JWT is validated, then claims are checked according to rules
- [ ] Given an invalid or expired JWT, when authentication is required, then request returns 401 Unauthorized with clear error message
- [ ] Given a path configured without authentication, when request arrives, then it proceeds without JWT validation
- [ ] Given a path configured with authentication, when no valid JWT is provided, then request returns 401 Unauthorized
- [ ] Given JWT secret/public key in configuration, when starting up, then keys are loaded securely

**Dependencies:** jsonwebtoken crate, JWT configuration management

---

#### Feature 3: S3 Request Proxying and Signing
**Priority:** High  
**Description:** Transform HTTP requests into properly signed AWS S3 API requests using AWS Signature Version 4, handle S3 responses, and stream content back to clients. **Uses zero-copy streaming architecture - never buffers entire files to disk.**

**User Stories:**
- As a client application, I want to make standard HTTP GET requests to the proxy, so that the proxy handles S3 authentication and signing for me
- As the proxy, I want to generate valid AWS Signature v4 signatures, so that S3 accepts my requests
- As a developer, I want support for common S3 operations (GET object, HEAD object, LIST objects), so that I can perform standard S3 workflows through the proxy
- As a client, I want efficient streaming of large S3 objects, so that memory usage stays reasonable

**Acceptance Criteria:**
- [ ] Given a GET request for an object, when proxied to S3, then request is signed with AWS Signature v4
- [ ] Given a valid S3 object path, when requested, then object content is returned with correct content-type
- [ ] Given a HEAD request for an object, when proxied to S3, then object metadata is returned without body
- [ ] Given a LIST request for a prefix, when proxied to S3, then object list is returned in proper format
- [ ] Given a large object (>100MB), when streaming to client, then memory usage remains constant (streaming not buffering)
- [ ] Given an S3 error (404, 403, etc.), when returned, then proxy translates it to appropriate HTTP status with clear message
- [ ] Given S3 response headers (ETag, Last-Modified, etc.), when returned, then they are preserved in proxy response

**Range Request Support:**
- [ ] Given a Range header in request, when proxied to S3, then Range header forwarded with AWS signature
- [ ] Given a valid byte range, when S3 returns 206, then proxy returns 206 Partial Content to client
- [ ] Given a range request, when served, then only requested bytes streamed (not full file, saves bandwidth)
- [ ] Given a range request, when cache enabled, then cache bypassed (range requests never cached)
- [ ] Given invalid range syntax, when requested, then graceful fallback to 200 OK full file
- [ ] Given range exceeding file size, when requested, then 416 Range Not Satisfiable returned
- [ ] Given If-Range header with matching ETag, when requested, then range returned; else full file
- [ ] Given any response, when sent, then Accept-Ranges: bytes header included to indicate support

**Streaming Architecture:**
- **Zero-copy streaming**: S3 response chunks flow directly to client without local buffering
- **Constant memory**: ~64KB buffer per connection regardless of file size
- **Low latency**: First byte to client (TTFB) within ~100-500ms
- **Disconnect handling**: S3 stream cancelled immediately if client disconnects
- **No local storage**: Files are never written to proxy's disk before serving

**Caching Behavior:**
- **Small files** (<10MB configurable): Buffered for caching when cache enabled
- **Large files** (>10MB): Always streamed, never cached
- **Range requests**: Always streamed from S3
- **Cache writes**: Async/background, don't block client response

See [STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md) for detailed sequence diagrams.

**Dependencies:** AWS SDK for Rust or rusoto, Pingora streaming capabilities

---

#### Feature 4: Configuration Management
**Priority:** High  
**Description:** Support flexible configuration via YAML/TOML files for bucket mappings, JWT settings, and proxy behavior. Enable hot-reload for configuration updates without downtime.

**User Stories:**
- As an administrator, I want to define all buckets, paths, and authentication rules in a configuration file, so that I can manage proxy behavior declaratively
- As an operator, I want to update configuration and reload without restarting the proxy, so that I can avoid service disruption
- As a developer, I want clear configuration validation errors on startup, so that I can quickly fix misconfigurations

**Acceptance Criteria:**
- [ ] Given a YAML configuration file, when proxy starts, then configuration is loaded and validated
- [ ] Given invalid configuration (missing required fields, invalid formats), when starting, then proxy exits with clear error messages
- [ ] Given a configuration file change, when signaled to reload, then new configuration is applied without dropping connections
- [ ] Given environment variable overrides, when present, then they take precedence over file configuration
- [ ] Given sensitive credentials in config, when loaded, then they are handled securely (not logged)

**Dependencies:** serde, config crate, YAML parser

---

### Secondary Features

#### Feature 5: Advanced Caching
**Priority:** Medium  
**Description:** Implement optional caching layer (heap, mmap, or disk) for frequently accessed **small files** to reduce S3 requests and improve performance. Large files are always streamed for memory efficiency.

**Caching Strategy:**
- **Small files** (<10MB configurable): Eligible for caching
- **Large files** (>10MB): Always streamed, never cached (memory efficiency)
- **Range requests**: Always streamed from S3 (no partial caching)
- **Cache write**: Asynchronous, doesn't block client response

**User Stories:**
- As a platform engineer, I want to cache hot objects in memory, so that repeated requests are served faster
- As an administrator, I want to configure cache TTL and size limits per bucket, so that I can balance performance and freshness
- As an operator, I want cache statistics and metrics, so that I can tune cache settings

**Acceptance Criteria:**
- [ ] Given cache enabled and file <10MB, when requested twice, then second request served from cache
- [ ] Given cache enabled and file >10MB, when requested, then file is streamed (not cached)
- [ ] Given cache TTL expired, when requested, then file is re-fetched from S3
- [ ] Given cache max_size reached, when new file cached, then LRU eviction occurs
- [ ] Given cache miss, when file cached, then client response not delayed by cache write
- [ ] Given range request, when requested, then always fetched from S3 (not cached)

**Cache Layers (v1.0 uses heap cache, others deferred to v1.1):**
- **Heap cache**: In-memory HashMap, fast but limited by RAM
- **Mmap cache** (v1.1): Memory-mapped files, larger capacity
- **Disk cache** (v1.1): Persistent cache, largest capacity

See [STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md) for cache flow diagrams.

#### Feature 6: Request Logging and Metrics
**Priority:** Medium  
**Description:** Comprehensive logging of requests, S3 operations, and authentication events with Prometheus metrics export.

**User Stories:**
- As an operator, I want detailed logs of all proxy operations, so that I can troubleshoot issues
- As a monitoring engineer, I want Prometheus metrics (request counts, latencies, error rates), so that I can monitor proxy health

#### Feature 7: Rate Limiting
**Priority:** Low  
**Description:** Implement per-client or per-path rate limiting to protect S3 buckets from abuse.

**User Stories:**
- As a security engineer, I want to rate-limit requests per JWT subject, so that no single user can overwhelm the system

## Non-Functional Requirements

### Performance
- **Response time:** <100ms P95 for cached objects, <500ms P95 for S3 requests
- **Throughput:** 10,000+ requests/second on commodity hardware
- **Resource efficiency:** 70% lower CPU usage compared to Envoy-based solution
- **Memory usage:** <500MB base, scales linearly with concurrent connections
- **Scalability:** Horizontal scaling via multiple proxy instances (stateless design)

### Security
- **Authentication:** JWT validation with configurable algorithms (HS256, RS256, ES256)
- **Authorization:** Custom claims-based access control per bucket/path
- **Credential isolation:** Complete separation of AWS credentials per bucket
- **Secrets management:** Support for environment variables and secure config files
- **TLS:** Support for HTTPS upstream connections to S3
- **Compliance:** No credential logging, secure memory handling for tokens and keys

### Reliability
- **Availability:** 99.9% uptime target (depends on S3 availability)
- **Error handling:** 
  - Graceful handling of S3 errors with proper HTTP status codes
  - Retry logic with exponential backoff for transient failures
  - Circuit breaker pattern for failing S3 buckets
- **Data integrity:** 
  - Pass through S3 ETags for integrity verification
  - No data corruption during proxying
- **Fault tolerance:** Continue serving other buckets even if one bucket is unavailable

### Usability
- **Configuration:** Clear, self-documenting YAML/TOML configuration format
- **Error messages:** Human-readable error messages for all failure scenarios
- **Documentation:** 
  - Comprehensive README with examples
  - Configuration reference documentation
  - API documentation for all endpoints
- **Deployment:** Single binary deployment with minimal dependencies

### Maintainability
- **Code quality:** Follow Kent Beck's TDD principles rigorously
- **Test coverage:** >90% unit test coverage, comprehensive integration tests
- **Code documentation:** Document complex logic and design decisions
- **Logging:** Structured logging with appropriate log levels
- **Metrics:** Prometheus metrics for observability
- **Configuration validation:** Fail fast with clear errors on misconfiguration

## Technical Specifications

### Architecture

**Architecture Pattern:** Asynchronous proxy with Pingora framework

**High-Level Architecture:**
```
Client Request
    ↓
Pingora HTTP Server
    ↓
Request Router (path matching)
    ↓
JWT Authenticator (optional, per-route)
    ↓
Bucket Resolver (determine target bucket)
    ↓
S3 Request Builder (sign with SigV4)
    ↓
S3 Backend (AWS or compatible)
    ↓
Response Streamer
    ↓
Client Response
```

**Key Components:**
1. **HTTP Server Layer (Pingora):** Handles HTTP/HTTPS connections, request parsing
2. **Router:** Maps request paths to bucket configurations
3. **JWT Middleware:** Extracts and validates JWT tokens from configured sources
4. **Claims Validator:** Evaluates custom claims verification logic
5. **S3 Client:** Signs requests with AWS Signature v4, handles S3 API communication
6. **Configuration Manager:** Loads and validates configuration, supports hot reload
7. **Metrics Collector:** Exports Prometheus metrics
8. **Logger:** Structured logging with tracing integration

**Data Flow:**
1. Client sends HTTP request to proxy
2. Router matches path prefix to bucket configuration
3. If JWT required, middleware extracts token from configured source(s)
4. JWT validated and claims checked against custom rules
5. Request transformed to S3 API request with proper signing
6. S3 response streamed back to client with appropriate headers
7. Metrics and logs recorded

### Technology Stack

- **Language:** Rust 1.70+ (stable)
- **Proxy Framework:** Cloudflare Pingora (latest stable)
- **S3 Integration:** 
  - `aws-sdk-s3` (official AWS SDK for Rust) - primary choice
  - OR `rusoto_s3` (alternative if SDK limitations)
- **JWT:** `jsonwebtoken` crate for validation
- **Async Runtime:** Tokio (via Pingora)
- **Serialization:** `serde` with `serde_yaml` and `serde_json`
- **Configuration:** `config` crate for layered configuration
- **Logging:** `tracing` + `tracing-subscriber` for structured logging
- **Metrics:** `prometheus` crate for metrics export
- **Testing Framework:** 
  - `cargo test` (unit tests)
  - `rstest` for parameterized tests
  - Integration tests with test S3 backend (MinIO or LocalStack)
- **Build Tools:** Cargo (standard Rust toolchain)
- **HTTP Client:** Hyper (via Pingora/AWS SDK)

### Dependencies (Cargo.toml)
```toml
[dependencies]
pingora = "0.1"  # Cloudflare's proxy framework
aws-sdk-s3 = "1.0"  # AWS S3 SDK
aws-config = "1.0"  # AWS configuration
tokio = { version = "1.35", features = ["full"] }  # Async runtime
jsonwebtoken = "9.2"  # JWT validation
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"  # YAML config parsing
serde_json = "1.0"  # JSON handling
config = "0.14"  # Configuration management
tracing = "0.1"  # Structured logging
tracing-subscriber = "0.3"  # Logging backend
prometheus = "0.13"  # Metrics
hyper = "1.0"  # HTTP primitives
anyhow = "1.0"  # Error handling
thiserror = "1.0"  # Error derive macros

[dev-dependencies]
rstest = "0.18"  # Parameterized testing
mockall = "0.12"  # Mocking framework
testcontainers = "0.15"  # Container-based integration tests
```

### APIs and Integrations

**Internal APIs:**
- Configuration API: Load, validate, reload configuration
- Router API: Match paths to bucket configurations
- JWT API: Extract, validate, verify JWT tokens
- S3 API: Sign and execute S3 requests
- Metrics API: Export Prometheus metrics on `/metrics` endpoint

**External APIs:**
- **AWS S3 API:** Standard S3 REST API with Signature Version 4
  - GET Object
  - HEAD Object
  - LIST Objects (optional)
- **JWT Validation:** Standard JWT validation (no external service)

**Data Formats:** 
- Configuration: YAML or TOML
- Responses: Original S3 content-types (binary, JSON, XML, etc.)
- Metrics: Prometheus text format
- Logs: JSON structured logs

### Data Model

#### Configuration Schema

```yaml
# config.yaml
server:
  address: "0.0.0.0:8080"
  https:
    enabled: true
    cert_path: "/path/to/cert.pem"
    key_path: "/path/to/key.pem"
  
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-east-1"
      endpoint: "https://s3.amazonaws.com"  # optional, for S3-compatible services
      access_key: "${AWS_ACCESS_KEY_PRODUCTS}"  # env var substitution
      secret_key: "${AWS_SECRET_KEY_PRODUCTS}"
    auth:
      enabled: true
      jwt:
        token_sources:
          - type: "header"
            name: "Authorization"
            prefix: "Bearer "
          - type: "query"
            name: "token"
          - type: "header"
            name: "X-Auth-Token"
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        claims_verification:
          - claim: "role"
            operator: "equals"
            value: "admin"
          - claim: "tenant"
            operator: "equals"
            value: "acme"
    cache:
      enabled: true
      ttl: 3600
      max_size: "1GB"
  
  - name: "public-assets"
    path_prefix: "/assets"
    s3:
      bucket: "public-assets-bucket"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY_ASSETS}"
      secret_key: "${AWS_SECRET_KEY_ASSETS}"
    auth:
      enabled: false  # Public access
    cache:
      enabled: true
      ttl: 86400
      max_size: "5GB"

logging:
  level: "info"
  format: "json"
  
metrics:
  enabled: true
  port: 9090
```

#### Runtime Data Structures

**BucketConfig:**
```rust
struct BucketConfig {
    name: String,
    path_prefix: String,
    s3_config: S3Config,
    auth_config: Option<AuthConfig>,
    cache_config: Option<CacheConfig>,
}
```

**S3Config:**
```rust
struct S3Config {
    bucket: String,
    region: String,
    endpoint: Option<String>,
    access_key: String,
    secret_key: String,
}
```

**AuthConfig:**
```rust
struct AuthConfig {
    enabled: bool,
    jwt: JwtConfig,
}

struct JwtConfig {
    token_sources: Vec<TokenSource>,
    secret: String,
    algorithm: Algorithm,
    claims_verification: Vec<ClaimVerification>,
}

enum TokenSource {
    Header { name: String, prefix: Option<String> },
    Query { name: String },
}

struct ClaimVerification {
    claim: String,
    operator: ClaimOperator,
    value: serde_json::Value,
}

enum ClaimOperator {
    Equals,
    Contains,
    In,
    GreaterThan,
    LessThan,
}
```

## User Interface Specifications

### Main Workflows
1. **Workflow 1:** [Name]
   - Step 1: [Description]
   - Step 2: [Description]
   - Step 3: [Description]

2. **Workflow 2:** [Name]
   - Step 1: [Description]
   - Step 2: [Description]

### Key UI Components
- [Component 1]: [Description]
- [Component 2]: [Description]

## Error Handling

### Error Categories

#### User Input Errors
**How to handle:** Return 400 Bad Request with clear error message
- Invalid path format
- Malformed JWT token
- Missing required headers
- Invalid query parameters

**Example Response:**
```json
{
  "error": "Invalid request",
  "message": "Path must start with a configured prefix",
  "details": "Requested path '/invalid' does not match any configured bucket"
}
```

#### Authentication/Authorization Errors
**How to handle:** Return appropriate 401/403 status codes
- 401 Unauthorized: Missing or invalid JWT
- 403 Forbidden: Valid JWT but failed claims verification

**Example Response:**
```json
{
  "error": "Unauthorized",
  "message": "JWT token is expired",
  "details": "Token expired at 2025-10-21T10:00:00Z"
}
```

#### S3 Errors
**How to handle:** Map S3 errors to appropriate HTTP status codes
- 404: S3 object not found
- 403: S3 access denied (credentials issue)
- 500: S3 service error

**Example Response:**
```json
{
  "error": "Not found",
  "message": "Object does not exist",
  "details": "Key 'path/to/object.txt' not found in bucket 'my-bucket'"
}
```

#### System Errors
**How to handle:** Return 500 Internal Server Error, log full details
- Configuration errors
- Network failures to S3
- Internal proxy errors

**Example Response:**
```json
{
  "error": "Internal server error",
  "message": "Failed to communicate with S3",
  "request_id": "abc123"
}
```

### Error Messages

All error messages must be:
- **Clear and actionable:** Tell user what went wrong and how to fix it
- **User-friendly:** No Rust panic messages or technical jargon for external clients
- **Secure:** Don't expose internal paths, credentials, or sensitive config
- **Logged:** All errors logged with full context for debugging (request ID, path, bucket, etc.)
- **Structured:** JSON format for easy parsing by clients

### Error Logging

Internal logs should include:
```rust
error!(
    error = %e,
    request_id = %req_id,
    path = %path,
    bucket = %bucket_name,
    "Failed to fetch object from S3"
);
```

## Testing Strategy

Following TDD principles outlined in CLAUDE.md:

### Test Pyramid

```
       /\
      /  \     E2E Tests (5%)
     /____\    - Full proxy workflow with real S3 (MinIO/LocalStack)
    /      \   
   /  Inte  \  Integration Tests (15%)
  /  gration \  - Component interactions
 /____________\ - Router + Auth + S3 client
/              \
/  Unit Tests   \ Unit Tests (80%)
/________________\ - Individual functions and modules
```

### Test Levels

#### 1. Unit Tests
**Coverage target:** >90% of code  
**Run frequency:** After every code change (in TDD cycle)

**Test areas:**
- Configuration parsing and validation
- Path matching and routing logic
- JWT token extraction from different sources
- JWT validation and claims verification
- AWS Signature v4 generation
- Error handling and error message generation
- Utility functions and helpers

**Example tests:**
```rust
#[test]
fn test_extract_token_from_bearer_header() {
    let headers = /* ... */;
    let token = extract_token(&headers, &TokenSource::Header { 
        name: "Authorization".into(), 
        prefix: Some("Bearer ".into()) 
    });
    assert!(token.is_some());
}

#[test]
fn test_jwt_claims_verification_equals() {
    let claims = json!({"role": "admin"});
    let verification = ClaimVerification {
        claim: "role".into(),
        operator: ClaimOperator::Equals,
        value: json!("admin"),
    };
    assert!(verify_claim(&claims, &verification));
}
```

#### 2. Integration Tests
**Coverage target:** All critical interaction paths  
**Run frequency:** Before every commit

**Test areas:**
- Router + Auth middleware integration
- Auth middleware + S3 client integration
- Configuration loading + Router initialization
- Metrics collection across components
- Error propagation through layers

**Example tests:**
```rust
#[tokio::test]
async fn test_authenticated_s3_request_flow() {
    // Setup: Create test config, mock S3
    let config = test_config();
    let proxy = setup_proxy(config);
    
    // Execute: Send request with JWT
    let response = proxy.request()
        .header("Authorization", "Bearer valid-token")
        .path("/products/test.txt")
        .send()
        .await;
    
    // Verify: Check S3 was called with correct signature
    assert_eq!(response.status(), 200);
    assert_s3_called_with_signature();
}
```

#### 3. End-to-End Tests
**Coverage target:** All main user workflows  
**Run frequency:** Before every release (can be slow)

**Test areas:**
- Full request flow: Client → Proxy → S3 (MinIO/LocalStack) → Client
- Multi-bucket routing with real S3-compatible storage
- JWT authentication with real token generation
- Caching behavior with real cache backend
- Error scenarios with real S3 errors

**Test setup:** Use Docker Compose with:
- MinIO or LocalStack for S3-compatible storage
- Test JWT issuer service
- Yatagarasu proxy instance

**Example tests:**
```rust
#[tokio::test]
#[ignore] // Long-running test
async fn test_e2e_multi_bucket_access() {
    // Setup: Start MinIO, create buckets, configure proxy
    let test_env = E2ETestEnvironment::new().await;
    
    // Bucket A: PUT object
    test_env.minio.put_object("bucket-a", "file.txt", "content").await;
    
    // Bucket B: PUT object
    test_env.minio.put_object("bucket-b", "data.json", "{}").await;
    
    // Test: Access via proxy
    let resp_a = test_env.proxy.get("/bucket-a/file.txt").await;
    let resp_b = test_env.proxy.get("/bucket-b/data.json").await;
    
    assert_eq!(resp_a.text(), "content");
    assert_eq!(resp_b.text(), "{}");
}
```

### Test Data

- **Realistic test data:** Use realistic S3 object names, JWT claims, bucket configurations
- **Edge cases:** Test boundary conditions (empty files, large files, special characters in paths)
- **Error scenarios:** Test all error paths (missing files, expired tokens, network failures)
- **Security tests:** Test authentication bypass attempts, privilege escalation attempts

### Test Fixtures

Create reusable test fixtures:
```rust
// tests/fixtures/mod.rs
pub fn test_bucket_config() -> BucketConfig { /* ... */ }
pub fn test_jwt_config() -> JwtConfig { /* ... */ }
pub fn valid_jwt_token() -> String { /* ... */ }
pub fn expired_jwt_token() -> String { /* ... */ }
```

### Performance Tests

Use `criterion` for benchmarking:
```rust
#[bench]
fn bench_jwt_validation(b: &mut Bencher) {
    let token = valid_jwt_token();
    b.iter(|| validate_jwt(&token));
}

#[bench]
fn bench_s3_signature_generation(b: &mut Bencher) {
    let request = test_s3_request();
    b.iter(|| generate_signature(&request));
}
```

Target benchmarks:
- JWT validation: <1ms per token
- S3 signature generation: <100μs
- Path routing: <10μs per request

## Constraints and Assumptions

### Constraints
- **Pingora framework:** Must use Pingora's async architecture and APIs
- **Rust language:** Must be Rust (no mixed-language codebase)
- **S3 API compatibility:** Limited to S3-compatible APIs (AWS S3, MinIO, etc.)
- **Stateless design:** No persistent state in proxy (allows horizontal scaling)
- **Configuration reload:** Must support hot reload without dropped connections
- **Single tenant per bucket:** Each bucket config represents one S3 bucket only

### Assumptions
- S3 backend is available and responsive
- JWT issuer is external (proxy only validates, doesn't issue tokens)
- Clients can handle standard HTTP error codes and JSON error responses
- Network connectivity to S3 endpoints is reliable
- Configuration files are managed externally (not modified by proxy)
- TLS certificates are provided by operators (not auto-generated)
- Clock synchronization is maintained (for JWT exp validation and S3 signatures)

## Out of Scope

The following are explicitly out of scope for version 1.0:

### Not Supported
- **S3 write operations** (PUT, POST, DELETE) - Read-only proxy
- **S3 multipart uploads** - Complex to proxy correctly
- **S3 website hosting features** - Index documents, error documents
- **Custom S3 transformations** - Image resizing, video transcoding
- **JWT token issuance** - Only validation, not generation
- **User management** - No built-in user database
- **OAuth/OIDC flows** - Only JWT validation, not full OAuth
- **Billing/metering** - No usage tracking for billing
- **CDN features** - No edge caching or geographic distribution
- **WAF/DDoS protection** - Basic proxy, not a security gateway
- **GraphQL API** - REST/HTTP only

### Deferred to Future Versions
- S3 write operations (PUT, DELETE)
- Advanced caching strategies (LRU, LFU)
- Cache invalidation APIs
- Request/response transformation
- A/B testing support
- Blue/green bucket deployments
- Automatic failover between regions
- Built-in load balancing across S3 regions

## Future Considerations

Features and improvements to consider for future versions:

### Version 1.1
- **Write support:** Enable PUT/DELETE operations with proper access control
- **Advanced caching:** Implement mmap and disk cache layers
- **Cache invalidation:** API endpoints to purge cache entries
- **Cache pre-warming:** Recursive path prefetching to populate cache on startup or schedule
- **Metrics dashboard:** Built-in web UI for metrics visualization

### Version 1.2
- **S3 SELECT support:** Allow SQL-like queries on S3 objects
- **Request transformation:** Modify requests/responses based on rules
- **Multi-region failover:** Automatic failover to backup S3 regions
- **Compression:** Transparent gzip/brotli compression of responses

### Version 2.0
- **GraphQL API:** GraphQL layer over S3 objects
- **Smart caching:** ML-based cache eviction and predictive prefetching/pre-warming
- **Object transformation:** On-the-fly image resizing, video transcoding
- **Edge deployment:** Deploy proxy to edge locations for lower latency

## Glossary

- **AWS Signature v4:** Authentication protocol for AWS API requests
- **Bearer token:** Authentication token passed in Authorization header
- **Bucket:** S3 storage container for objects
- **Claims:** Key-value pairs in JWT payload (e.g., `sub`, `role`, `exp`)
- **ETag:** Entity tag used by S3 for cache validation and integrity
- **Hot reload:** Updating configuration without restarting the service
- **JWT (JSON Web Token):** Compact token format for claims-based authentication
- **MinIO:** S3-compatible open-source object storage
- **Path prefix:** URL path component used for routing (e.g., `/products`)
- **Pingora:** Cloudflare's Rust framework for building network services
- **S3:** Amazon Simple Storage Service, object storage service
- **S3-compatible:** Services implementing S3 API (MinIO, Ceph, etc.)
- **TLS (Transport Layer Security):** Cryptographic protocol for secure communications
- **TTL (Time To Live):** Duration for which cached data is considered fresh
- **Yatagarasu (八咫烏):** Three-legged crow from Japanese mythology; divine messenger and guide

## References

### Project
- **Original MVP:** https://github.com/julianshen/s3-envoy-proxy
- **CLAUDE.md:** Development methodology guide (Kent Beck's TDD)
- **plan.md:** TDD implementation plan with test checklist

### Technology Documentation
- **Pingora:** https://github.com/cloudflare/pingora
- **AWS SDK for Rust:** https://aws.amazon.com/sdk-for-rust/
- **JWT in Rust:** https://docs.rs/jsonwebtoken/
- **Tokio:** https://tokio.rs/
- **AWS Signature v4:** https://docs.aws.amazon.com/general/latest/gr/signature-version-4.html
- **S3 API Reference:** https://docs.aws.amazon.com/AmazonS3/latest/API/

### Testing Tools
- **MinIO:** https://min.io/ (S3-compatible storage for testing)
- **LocalStack:** https://localstack.cloud/ (AWS service emulation)
- **Testcontainers:** https://docs.rs/testcontainers/ (Container-based tests)

### Methodology
- **Kent Beck's TDD:** https://www.amazon.com/Test-Driven-Development-Kent-Beck/dp/0321146530
- **Tidy First?:** https://www.amazon.com/Tidy-First-Personal-Exercise-Empirical/dp/1098151240

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0   | 2025-10-21 | Team | Initial specification for Pingora-based S3 proxy |
| 0.1.0   | 2025-10-20 | Team | MVP specification (Envoy-based) |

---

## Approval

This specification should be reviewed and approved by:

- [ ] **Technical Lead:** Architecture and technical decisions
- [ ] **Security Engineer:** Security and authentication requirements
- [ ] **Platform Engineer:** Deployment and operational requirements
- [ ] **Product Owner:** Feature completeness and priorities

---

**Note:** This specification follows Kent Beck's principles of clarity, simplicity, and incremental development. It will evolve as we learn through the TDD process. Updates should be made through the same discipline: test first, then change specification to match learned reality.
