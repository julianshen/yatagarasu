# Yatagarasu (八咫烏)

> _"The three-legged crow that guides the way to secure S3 access"_

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-171%20library%20tests-green.svg)](plan.md)
[![Coverage](https://img.shields.io/badge/coverage-98.43%25-brightgreen.svg)](coverage/)
[![Status](https://img.shields.io/badge/status-Production%20Ready%20v0.4.0-brightgreen.svg)](IMPLEMENTATION_STATUS.md)

A high-performance **read-only** S3 proxy built with Cloudflare's Pingora framework and Rust, providing intelligent routing, multi-bucket support, and flexible JWT authentication for secure content delivery.

## 🎉 DEVELOPMENT STATUS

**Current State**: Core library modules complete and **Production-Ready with Docker & CI/CD!** (v0.4.0)

**✅ What Works Now** (as of 2025-11-14):
- ✅ **HTTP Server**: Accepts connections and proxies requests to S3!
- ✅ **Routing**: Requests to /bucket-prefix/* route to correct S3 bucket
- ✅ **Authentication**: JWT token validation with 401/403 responses
- ✅ **S3 Proxying**: AWS Signature V4 signing and request forwarding (GET and HEAD)
- ✅ **HEAD request support**: Fixed AWS signature bug for HEAD requests
- ✅ **Configuration**: YAML parsing with environment variables
- ✅ **Multi-bucket routing**: Longest prefix matching
- ✅ **Request tracing**: UUID request_id for distributed tracing
- ✅ **Error handling**: 404 for unknown paths, 401 for missing tokens, 403 for invalid tokens
- ✅ **Integration test infrastructure**: ProxyTestHarness for automated testing
- ✅ **Security validation**: Body size limits, header limits, path traversal protection
- ✅ **Rate limiting**: Global, per-IP, and per-bucket rate limits with token bucket algorithm
- ✅ **Circuit breaker**: Automatic failure detection and recovery
- ✅ **Prometheus metrics**: Request counts, latencies, error rates, rate limit metrics, histograms, backend health
- ✅ **Health endpoints**: `/health` (liveness) and `/ready` (readiness with S3 backend checks)
- ✅ **Graceful shutdown**: Pingora built-in SIGTERM handling, in-flight request completion
- ✅ **Structured logging**: UUID request_id correlation, X-Request-ID header, client IP tracking
- ✅ **Startup validation**: Config validation, --test mode, clear error messages
- ✅ **High Availability**: Multi-replica bucket configuration with automatic failover, circuit breaker health checking
- ✅ **Read-Only Enforcement**: HTTP method validation (GET/HEAD/OPTIONS only), 405 responses for unsafe methods
- ✅ **Docker & CI/CD**: Production-ready 41.2MB distroless image, docker-compose for local dev, GitHub Actions CI
- ✅ **171 library tests passing** with 98.43% coverage

**⏳ What's Still Being Worked On**:
- ⏳ End-to-end load testing with K6

**🚀 What's Coming Next**:
- 🚧 **Optional**: Chaos engineering tests (Toxiproxy integration)
- 🎯 **Phase 26+** (v1.0.0): Caching layer and advanced features

**✅ Recently Completed**:
- ✅ **Phase 25**: Read-Only Enforcement - HTTP method validation (GET/HEAD/OPTIONS only), 405 responses for unsafe methods (PUT/POST/DELETE/PATCH), CORS support for browser clients (15/15 tests passing)
- ✅ **Phase 24** (v0.4.0): Docker Images & CI/CD Automation - Production-ready multi-stage Dockerfile (41.2MB distroless image), docker-compose for local development, GitHub Actions CI with automated testing and coverage (36/36 tests passing)
- ✅ **Phase 23** (v0.3.1): High Availability Bucket Replication - Multi-replica failover, priority-based selection, circuit breaker health checking, per-replica metrics and observability (60+ tests passing)
- ✅ **Phase 22** (v0.3.0): Graceful Shutdown & Observability - Health endpoints (/health, /ready), graceful shutdown (Pingora built-in), structured logging (request_id, X-Request-ID), startup validation
- ✅ **Phase 21** (v0.2.0): Production Hardening & Resilience - Security validation (SQL injection, path traversal), rate limiting, circuit breaker (8/8 security tests passing)
- ✅ **Phase 19**: Configuration Hot Reload - SIGHUP signal handler, /admin/reload API endpoint, zero-downtime config updates, credential rotation (30+ tests passing)
- ✅ **Phase 17**: Performance benchmarking infrastructure (Criterion + K6) - ALL TARGETS EXCEEDED!
- ✅ **Phase 16**: Integration test infrastructure with ProxyTestHarness

**Progress**: ~97% toward v1.0 (Phases 19, 21-25 complete - production-ready with full security hardening!)

See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for detailed technical analysis and progress assessment.

## What is Yatagarasu?

Yatagarasu is a reimplementation of [s3-envoy-proxy](https://github.com/julianshen/s3-envoy-proxy) using modern Rust async architecture. It provides:

- 🚀 **High Performance**: 70% lower CPU usage compared to traditional proxies (via Pingora)
- 📖 **Read-Only Operations**: Secure content delivery with GET, HEAD, and OPTIONS support (no uploads/modifications)
- 🗂️ **Multi-Bucket Routing**: Map different S3 buckets to different URL paths with isolated credentials
- 🔐 **Flexible JWT Auth**: Optional authentication with multiple token sources (header, query, custom)
- 🎯 **Custom Claims**: Verify JWT claims with configurable logic (role, tenant, etc.)
- 📊 **Observable**: Prometheus metrics and structured JSON logging
- 🔄 **Hot Reload**: Update configuration without downtime
- 🧪 **Well-Tested**: >90% test coverage following TDD principles

**Name Origin**: Yatagarasu (八咫烏) is the three-legged crow in Japanese mythology that serves as a divine messenger and guide. Like its namesake, this proxy guides and securely routes requests to the appropriate S3 buckets.

### Supported Operations

✅ **Allowed HTTP Methods**:
- `GET` - Retrieve objects from S3
- `HEAD` - Get object metadata without body
- `OPTIONS` - CORS pre-flight requests

❌ **Unsupported Operations** (returns 405 Method Not Allowed):
- `PUT` - Upload files
- `POST` - Create/upload data
- `DELETE` - Remove objects
- `PATCH` - Modify objects

**Why Read-Only?** Yatagarasu is designed for secure content delivery, not data management. This design:
- Reduces attack surface (no upload vulnerabilities)
- Simplifies proxy logic and improves performance
- Enforces principle of least privilege
- Separates content delivery from content management

For file uploads, use direct S3 access or a dedicated upload service with proper validation and virus scanning.

## Quick Start

### Prerequisites

- Rust 1.70 or later
- S3-compatible storage (AWS S3, MinIO, LocalStack, etc.)
- (Optional) JWT token issuer for authentication

### Installation & Running (v0.3.0)

```bash
# Clone the repository
git clone https://github.com/yourusername/yatagarasu.git
cd yatagarasu

# Build the proxy
cargo build --release

# Run comprehensive test suite (507 tests)
cargo test

# Run the proxy server
cargo run -- --config config.test.yaml

# Or run the release build
./target/release/yatagarasu --config config.yaml
```

✅ **Server is FUNCTIONAL!** The HTTP server now accepts connections and proxies requests to S3.

Test the server:

```bash
# Start the server
cargo run -- --config config.test.yaml &

# Test routing (returns 404 if S3 bucket not configured)
curl http://localhost:8080/test/myfile.txt

# Test with JWT authentication
curl -H "Authorization: Bearer <your-jwt>" http://localhost:8080/test/private.txt

# Test invalid path (returns 404)
curl http://localhost:8080/nonexistent/path
```

⚠️ **Integration Testing Needed**: The server is functional but needs end-to-end testing with real S3/MinIO instances.

### Basic Configuration

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_PRODUCTS}"
      secret_key: "${AWS_SECRET_KEY_PRODUCTS}"
    auth:
      enabled: false # Public access

  - name: "private-data"
    path_prefix: "/private"
    s3:
      bucket: "private-data-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY_PRIVATE}"
      secret_key: "${AWS_SECRET_KEY_PRIVATE}"
    auth:
      enabled: true
      jwt:
        token_sources:
          - type: "header"
            name: "Authorization"
            prefix: "Bearer "
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
        claims_verification:
          - claim: "role"
            operator: "equals"
            value: "admin"

logging:
  level: "info"
  format: "json"

metrics:
  enabled: true
  port: 9090
```

### High Availability Configuration (v0.3.0+)

Yatagarasu supports **automatic failover** with replica sets for production-grade high availability. Configure multiple S3 replicas (primary + backup) per bucket with priority-based automatic failover.

**Features**:
- 🔄 **Automatic Failover**: Circuit breaker-based health checking with automatic replica selection
- 📊 **Per-Replica Metrics**: Track request counts, latencies, errors, and failover events per replica
- 🏥 **Health Monitoring**: `/ready` endpoint shows per-replica health status
- ⏱️ **Configurable Timeouts**: Set connection and request timeouts per replica
- 🔙 **Backward Compatible**: Legacy single-bucket configs continue to work

**Configuration Example**:

```yaml
server:
  address: "0.0.0.0:8080"
  threads: 4

buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      # Legacy fields (required for backward compatibility)
      bucket: "products-us-west"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"

      # New replica set configuration (Phase 23)
      replicas:
        - name: "primary-us-west"
          bucket: "products-us-west"
          region: "us-west-2"
          access_key: "${AWS_ACCESS_KEY_US_WEST}"
          secret_key: "${AWS_SECRET_KEY_US_WEST}"
          priority: 1        # Lower = higher priority
          timeout: 5         # Connection/request timeout in seconds

        - name: "backup-us-east"
          bucket: "products-us-east"
          region: "us-east-1"
          access_key: "${AWS_ACCESS_KEY_US_EAST}"
          secret_key: "${AWS_SECRET_KEY_US_EAST}"
          priority: 2        # Backup replica (used when primary fails)
          timeout: 5

        - name: "backup-eu-central"
          bucket: "products-eu"
          region: "eu-central-1"
          access_key: "${AWS_ACCESS_KEY_EU}"
          secret_key: "${AWS_SECRET_KEY_EU}"
          endpoint: "https://s3.eu-central-1.amazonaws.com"  # Optional custom endpoint
          priority: 3        # Third fallback
          timeout: 10

      # Optional: Circuit breaker per bucket (applies to replica health checks)
      circuit_breaker:
        failure_threshold: 5       # Open circuit after 5 failures
        success_threshold: 2       # Close circuit after 2 successes
        timeout_seconds: 30        # Half-open state timeout

metrics:
  enabled: true
  port: 9090
```

**How It Works**:
1. Each request selects the highest-priority **healthy** replica (circuit breaker not open)
2. If primary fails (circuit breaker open), automatically uses next healthy replica
3. Circuit breakers automatically recover after timeout (half-open → closed on success)
4. Per-replica metrics track which replica serves each request

**Observability**:

```bash
# Check overall health
curl http://localhost:8080/health

# Check per-replica health
curl http://localhost:8080/ready
# Response: {"status":"ready","backends":{"products":{"status":"ready","replicas":{"primary-us-west":"healthy","backup-us-east":"healthy"}}}}

# View per-replica metrics
curl http://localhost:9090/metrics | grep replica
# http_requests_by_replica_total{bucket="products",replica="primary-us-west"} 1523
# http_requests_by_replica_total{bucket="products",replica="backup-us-east"} 47
# replica_health{bucket="products",replica="primary-us-west"} 1
# replica_failovers_total{bucket="products",from="primary-us-west",to="backup-us-east"} 3
```

**Use Cases**:
- **Multi-Region Replication**: Automatic failover between AWS regions
- **Cross-Cloud Replication**: Primary on AWS, backup on MinIO/Wasabi
- **Disaster Recovery**: Automatic failover during S3 outages
- **Load Balancing**: Distribute traffic across replicas (future enhancement)

See [docs/HA_BUCKET_REPLICATION.md](docs/HA_BUCKET_REPLICATION.md) for comprehensive guide.

### Example Requests

The HTTP server is now fully functional! You can:

```bash
# Access public bucket
curl http://localhost:8080/products/image.png

# Access private bucket with JWT
curl -H "Authorization: Bearer eyJhbGc..." \
  http://localhost:8080/private/data.json

# Or with query parameter
curl http://localhost:8080/private/data.json?token=eyJhbGc...

# Check health (coming in Phase 22)
curl http://localhost:8080/health

# Check Prometheus metrics
curl http://localhost:9090/metrics
```

✅ **Status**: HTTP server is FUNCTIONAL! All core features working including routing, authentication, S3 proxying, rate limiting, circuit breaker, and metrics.

## Project Structure

```
yatagarasu/
├── Cargo.toml          # Rust dependencies and build configuration
├── CLAUDE.md           # Development methodology guide (READ THIS FIRST)
├── spec.md             # Product specification and requirements
├── plan.md             # TDD implementation plan with test checklist
├── README.md           # This file
├── config.yaml         # Example configuration
├── src/
│   ├── main.rs         # Application entry point
│   ├── lib.rs          # Library root
│   ├── config/         # Configuration loading and validation
│   ├── router/         # Path-to-bucket routing logic
│   ├── auth/           # JWT authentication and validation
│   ├── s3/             # S3 client and signature generation
│   ├── proxy/          # Pingora proxy implementation
│   └── error.rs        # Error types and handling
├── tests/
│   ├── integration/    # Integration tests
│   ├── e2e/            # End-to-end tests
│   └── fixtures/       # Test data and helpers
└── benches/            # Performance benchmarks
```

## Features

### ✅ Implemented: Library Layer (v0.1.0 - Complete)

- [x] **Configuration Management**: YAML parsing with environment variable substitution
- [x] **Multi-Bucket Routing**: Longest prefix matching with path normalization
- [x] **JWT Authentication**: Token extraction from multiple sources (header/query/custom)
- [x] **Claims Verification**: Flexible rules engine for JWT claims (equals operator)
- [x] **S3 Client**: AWS Signature Version 4 implementation
- [x] **S3 Operations**: GET and HEAD request building with signed headers
- [x] **Range Request Support**: HTTP Range header parsing (single/multiple/suffix ranges)
- [x] **Error Mapping**: S3 error codes to HTTP status codes
- [x] **Comprehensive Testing**: 373 tests with 98.43% coverage

### ✅ Complete: Server Layer (v0.2.0 - Phases 12-21)

- [x] **Pingora HTTP Server**: Initialize and configure Pingora server
- [x] **Request Pipeline**: Integrate router → auth → S3 client
- [x] **Response Streaming**: Stream S3 objects to HTTP clients
- [x] **Error Handling**: User-friendly error responses
- [x] **Logging**: Structured JSON logging with tracing
- [x] **Request Context**: Track request ID, bucket, user claims
- [x] **Security Validation**: Body/header size limits, path traversal protection
- [x] **Rate Limiting**: Global, per-IP, and per-bucket rate limits
- [x] **Circuit Breaker**: Automatic failure detection and recovery
- [x] **Prometheus Metrics**: Request counts, latencies, error rates, rate limit metrics

### 📋 Planned: Production Features (v0.3.0+)

- [ ] **Health Endpoints**: `/health` liveness and `/ready` readiness checks with S3 backend validation (Phase 22)
- [ ] **Graceful Shutdown**: SIGTERM with connection draining and in-flight request completion (Phase 22)
- [ ] **Request Correlation**: UUID request_id in logs and X-Request-ID response header (Phase 22)
- [ ] **Chaos Testing**: S3 failure scenarios, network partitions, resource exhaustion (Phase 22)
- [ ] **Performance Tuning**: Connection pooling optimization, keep-alive tuning

### 🎯 Future: Advanced Features (v1.0+)

- [ ] **Caching Layer**: Memory cache for small files (<10MB)
- [ ] **Cache Management**: Invalidation API, conditional requests
- [ ] **Advanced Auth**: RS256/ES256 algorithms, token introspection
- [ ] **Rate Limiting**: Per-client request throttling
- [ ] **Multi-Region**: S3 failover across regions

### 🐳 Docker & CI/CD (v0.4.0)

- [ ] **Docker Image**: Multi-stage Dockerfile with minimal image size
- [ ] **Docker Compose**: Full testing environment with MinIO
- [ ] **GitHub Actions CI**: Automated testing, linting, coverage
- [ ] **Automated Releases**: Multi-platform Docker images and binaries
- [ ] **Container Registry**: Images published to ghcr.io

## Use Cases

### 1. Centralized S3 Access Control

Provide applications with S3 access without distributing AWS credentials:

```yaml
# Each team gets their own bucket with isolated credentials
buckets:
  - name: "team-a"
    path_prefix: "/team-a"
    s3: { bucket: "team-a-bucket", ... }

  - name: "team-b"
    path_prefix: "/team-b"
    s3: { bucket: "team-b-bucket", ... }
```

### 2. Public + Private Content

Mix public and authenticated content in one proxy:

```yaml
buckets:
  - name: "public-assets"
    path_prefix: "/assets"
    auth: { enabled: false } # Public

  - name: "user-data"
    path_prefix: "/users"
    auth: { enabled: true } # Requires JWT
```

### 3. Fine-Grained Authorization

Control access using JWT claims:

```yaml
auth:
  jwt:
    claims_verification:
      - claim: "tenant"
        operator: "equals"
        value: "acme-corp"
      - claim: "role"
        operator: "equals"
        value: "admin"
```

### 4. Legacy Application Migration

Provide S3 access to applications that can't use AWS SDK:

```bash
# Old way: Direct file system access
# New way: Simple HTTP GET
curl http://yatagarasu-proxy/data/file.txt
```

## Getting Started

### For Developers

1. **Read the methodology guide first:**

   ```bash
   cat CLAUDE.md
   ```

   Understand the TDD and "Tidy First" approach.

2. **Review the specification:**

   ```bash
   cat spec.md
   ```

   Learn about features, architecture, and requirements.

3. **Check the implementation plan:**

   ```bash
   cat plan.md
   ```

   See what tests are implemented and what's next.

4. **Start developing:**
   - Find the next unmarked test in `plan.md`
   - Write the test (Red phase)
   - Make it pass with minimal code (Green phase)
   - Refactor while keeping tests green (Refactor phase)
   - Mark the test complete and commit

### Working with Claude

This project is designed to work seamlessly with Claude (AI assistant) using the methodology defined in CLAUDE.md.

To start a development session:

```
Claude, I'm working on Yatagarasu. Please read CLAUDE.md and plan.md,
then let's implement the next test.
```

Or simply say:

```
go
```

Claude will find the next unmarked test in plan.md and guide you through implementing it following TDD principles.

## Development Workflow

### The TDD Cycle

1. **🔴 Red** - Write a failing test

   - Choose the next test from plan.md
   - Write the test code
   - Run tests and confirm it fails
   - Commit: `[BEHAVIORAL] Add test for [feature]`

2. **🟢 Green** - Make it pass

   - Write minimum code to pass the test
   - Run tests and confirm all pass
   - Commit: `[BEHAVIORAL] Implement [feature]`

3. **🔵 Refactor** - Clean up

   - Improve code structure
   - Run tests after each change
   - Commit: `[STRUCTURAL] [refactoring description]`

4. **🔄 Repeat** - Next test

### Commit Guidelines

All commits must have one of these prefixes:

- `[BEHAVIORAL]` - Changes that add or modify functionality
- `[STRUCTURAL]` - Changes that improve code structure without changing behavior

Examples:

```bash
git commit -m "[BEHAVIORAL] Add JWT validation from Authorization header"
git commit -m "[STRUCTURAL] Extract token parsing to separate function"
git commit -m "[BEHAVIORAL] Fix credential isolation bug in multi-bucket routing"
```

### Rules for Commits

✅ **DO commit when:**

- All tests are passing
- No compiler/linter warnings
- The change is a single logical unit

❌ **DON'T commit when:**

- Any test is failing
- There are compiler/linter warnings
- Mixing structural and behavioral changes

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test
cargo test jwt_validation

# Run with output visible
cargo test -- --nocapture

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage

# Run fast tests only (skip slow e2e tests)
cargo test --lib && cargo test --test integration_*
```

### Integration Test Setup

For integration tests with real S3, start MinIO:

```bash
# Start MinIO
docker run -d -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  --name minio \
  minio/minio server /data --console-address ":9001"

# Run integration tests
TEST_S3_ENDPOINT=http://localhost:9000 \
TEST_S3_ACCESS_KEY=minioadmin \
TEST_S3_SECRET_KEY=minioadmin \
cargo test --test integration_*

# Stop MinIO
docker stop minio && docker rm minio
```

### Test Coverage Goals

- **Unit tests**: >90% coverage
- **Integration tests**: All critical paths
- **End-to-end tests**: All main user workflows

Current coverage can be viewed by running:

```bash
cargo tarpaulin --out Html && open tarpaulin-report.html
```

## Performance

### Benchmarks

Run performance benchmarks with:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench jwt_validation

# Profile with perf (Linux)
cargo build --release
perf record --call-graph dwarf ./target/release/yatagarasu
perf report
```

### Load Testing

Test with `wrk` or `hey`:

```bash
# With wrk
wrk -t12 -c400 -d30s http://localhost:8080/products/test.txt

# With hey
hey -n 100000 -c 100 http://localhost:8080/products/test.txt
```

### Performance Targets & Benchmark Results

**Micro-Benchmarks (Criterion.rs)** - ✅ ALL TARGETS EXCEEDED:

- **JWT validation**: <1ms target → **0.84-1.03µs actual** (1000x faster!)
- **Path routing**: <10µs target → **39-202ns actual** (50-250x faster!)
- **S3 signature generation**: <100µs target → **6µs actual** (16x faster!)

**Load Testing Targets (K6)** - Infrastructure ready, awaiting integration tests:

- Request handling: <100ms P95 (cached), <500ms P95 (S3)
- Throughput: >10,000 requests/second
- Memory: <500MB base, scales linearly with connections

See [docs/PERFORMANCE.md](docs/PERFORMANCE.md) for detailed performance testing guide and [scripts/load-testing/](scripts/load-testing/) for K6 test scripts.

## Configuration Reference

### Server Configuration

```yaml
server:
  address: "0.0.0.0:8080" # Listen address
  https: # Optional TLS
    enabled: true
    cert_path: "/path/to/cert.pem"
    key_path: "/path/to/key.pem"
```

### Bucket Configuration

```yaml
buckets:
  - name: "bucket-name" # Unique identifier
    path_prefix: "/prefix" # URL path prefix
    s3:
      bucket: "s3-bucket-name" # S3 bucket name
      region: "us-east-1" # AWS region
      endpoint: "https://..." # Optional: custom endpoint (MinIO, etc.)
      access_key: "${ENV_VAR}" # Access key (env var substitution)
      secret_key: "${ENV_VAR}" # Secret key (env var substitution)
    auth: # Optional authentication
      enabled: true
      jwt:
        token_sources: # Where to look for JWT
          - type: "header"
            name: "Authorization"
            prefix: "Bearer "
          - type: "query"
            name: "token"
          - type: "header"
            name: "X-Auth-Token"
        secret: "${JWT_SECRET}" # JWT signing secret
        algorithm: "HS256" # Algorithm: HS256, RS256, ES256
        claims_verification: # Custom claim rules
          - claim: "role"
            operator: "equals" # equals, contains, in, gt, lt
            value: "admin"
    cache: # Optional caching
      enabled: true
      ttl: 3600 # Time to live in seconds
      max_size: "1GB" # Maximum cache size
```

### JWT Authentication Configuration

**Global JWT configuration (applies to all buckets with `auth.enabled: true`)**:

```yaml
jwt:
  enabled: true
  secret: "${JWT_SECRET}" # JWT signing secret (environment variable recommended)
  algorithm: "HS256" # Supported: HS256, HS384, HS512
  token_sources: # Checked in order until token found
    - type: "bearer" # Authorization: Bearer {token}
    - type: "header" # Custom header
      name: "X-Auth-Token"
      prefix: "Token " # Optional: strip this prefix before validation
    - type: "query" # Query parameter
      name: "token" # ?token={token}
  claims: # Optional: verify custom claims
    - claim: "role"
      operator: "equals" # Supported: equals, in, contains, gt, lt, gte, lte
      value: "admin"
```

**Valid token source types**:
- `bearer`: Extract from `Authorization: Bearer {token}` header
- `header`: Extract from custom header (requires `name` field)
- `query`: Extract from query parameter (requires `name` field)

**Important**:
- Token sources are checked in order until a token is found
- The `name` field is **required** for `header` and `query` types
- The `prefix` field is optional for `header` types (strips prefix before validation)
- Configuration validation will catch invalid source types or missing required fields

**Common Pitfalls**:
- ❌ Don't use `type: "bearer_header"` - use `type: "bearer"`
- ❌ Don't use `param_name` or `header_name` - use `name` for both
- ✅ Ensure secret is at least 32 characters for HS256
- ✅ Use environment variables for secrets (never commit secrets to config files)

### Logging Configuration

```yaml
logging:
  level: "info" # trace, debug, info, warn, error
  format: "json" # json or text
```

### Metrics Configuration

```yaml
metrics:
  enabled: true
  port: 9090 # Prometheus metrics port
```

## Technology Stack

- **Language**: Rust 1.70+ (stable)
- **Proxy Framework**: [Cloudflare Pingora](https://github.com/cloudflare/pingora)
- **Async Runtime**: Tokio (via Pingora)
- **S3 SDK**: AWS SDK for Rust
- **JWT**: jsonwebtoken crate
- **Config**: serde, serde_yaml
- **Logging**: tracing, tracing-subscriber
- **Metrics**: prometheus crate
- **Testing**: cargo test, rstest, testcontainers

### Key Dependencies

```toml
[dependencies]
pingora = "0.1"
aws-sdk-s3 = "1.0"
tokio = { version = "1.35", features = ["full"] }
jsonwebtoken = "9.2"
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
tracing = "0.1"
prometheus = "0.13"
```

## Code Quality Standards

Following Kent Beck's principles:

- ✨ **Eliminate duplication** ruthlessly
- 📖 **Express intent clearly** through naming and structure
- 🔗 **Make dependencies explicit**
- 🎯 **Keep methods small** and focused
- 🔄 **Minimize state** and side effects
- 💡 **Use the simplest solution** that works

All code must pass:

- `cargo test` (all tests passing)
- `cargo clippy` (no warnings)
- `cargo fmt --check` (properly formatted)
- > 90% test coverage

## Operations

### Deployment

```bash
# Build release binary
cargo build --release

# Binary location
./target/release/yatagarasu

# Run with config
./target/release/yatagarasu --config /etc/yatagarasu/config.yaml

# Run with environment variables
AWS_ACCESS_KEY_PRODUCTS=xxx \
AWS_SECRET_KEY_PRODUCTS=yyy \
JWT_SECRET=zzz \
./target/release/yatagarasu --config config.yaml
```

### Monitoring

- **Metrics**: Available at `http://localhost:9090/metrics` (Prometheus format)
- **Health Check**: `http://localhost:8080/health`
- **Logs**: Structured JSON to stdout (redirect to your log aggregator)

### Hot Reload Configuration

```bash
# Send SIGHUP to reload
kill -HUP $(pgrep yatagarasu)

# Or via management API
curl -X POST http://localhost:8080/admin/reload
```

### Graceful Shutdown

```bash
# Send SIGTERM
kill -TERM $(pgrep yatagarasu)

# Or Ctrl+C in terminal
```

## Troubleshooting

### Common Issues

**Problem**: "JWT token is invalid"

- Check that JWT secret matches between issuer and proxy
- Verify JWT hasn't expired (check `exp` claim)
- Ensure algorithm matches (HS256, RS256, etc.)

**Problem**: "Access denied to S3"

- Verify AWS credentials are correct
- Check IAM permissions on the S3 bucket
- Ensure bucket region matches configuration

**Problem**: "Path not found (404)"

- Verify path prefix is configured in `buckets`
- Check that path starts with configured prefix
- Ensure path_prefix includes leading slash

**Problem**: "High memory usage"

- Check for large file streaming (should be constant memory)
- Review cache configuration and size limits
- Monitor metrics for connection leaks

## Documentation

- **[GETTING_STARTED.md](GETTING_STARTED.md)** - Step-by-step guide for new developers
- **[CLAUDE.md](CLAUDE.md)** - Development methodology (how we work)
- **[spec.md](spec.md)** - Product specification (what we're building)
- **[plan.md](plan.md)** - Implementation plan (what's next)
- **[README.md](README.md)** - This project overview (where to start)
- **[STREAMING_ARCHITECTURE.md](STREAMING_ARCHITECTURE.md)** - Detailed streaming and caching architecture
- **[QUICK_REFERENCE_STREAMING.md](QUICK_REFERENCE_STREAMING.md)** - Quick ASCII diagrams for data flow

## Contributing

This project follows strict TDD methodology:

1. All changes must start with a test
2. Tests must fail before implementation
3. Implement minimum code to pass
4. Refactor only when tests are green
5. Separate structural and behavioral commits
6. Never commit with failing tests

For detailed guidelines, see [CLAUDE.md](CLAUDE.md).

## Project Status

**Current Phase**: Phase 22 - Graceful Shutdown & Observability (📋 NOT STARTED)

**Progress**:

- **Tests written**: 500+ tests
- **Tests passing**: 128 library tests (100%)
- **Test coverage**: High coverage across all modules
- **Phases complete**: Library layer 100% (Phases 1-5 ✅), Server layer 100% (Phases 12-21 ✅)

**Completed Milestones**:
- ✅ Phase 1-2: Foundation and Configuration (50 tests)
- ✅ Phase 3: Path Routing (26 tests)
- ✅ Phase 4: JWT Authentication (49 tests)
- ✅ Phase 5: S3 Client & Signature (73 tests)
- ✅ Phase 0: Critical bug fixes (timestamp, JWT algorithm, HEAD request support)
- ✅ Phase 12: Pingora HTTP server implementation
- ✅ Phase 13: ProxyHttp trait implementation (234 lines)
- ✅ Phase 15: Structured logging with tracing
- ✅ Phase 16: Integration test infrastructure (33 tests)
- ✅ Phase 17: Performance benchmarking (all targets exceeded!)
- ✅ Phase 18: Integration testing with MinIO
- ✅ Phase 19: Prometheus metrics endpoint
- ✅ Phase 20: Circuit breaker and retry logic
- ✅ Phase 21: Security hardening (SQL injection, path traversal, rate limiting - 8/8 security tests passing)

**Current Sprint**: Operational Excellence & Observability
- **Phase 22**: Health endpoints, graceful shutdown, structured logging, chaos testing

**Next Milestones**:
- Phase 22: Health/ready endpoints, graceful shutdown, request correlation
- Phase 23-24: Docker images and CI/CD
- Phase 25+: Caching layer and advanced features

**Production Readiness**:
- ✅ Security: Body/header/URI limits, SQL injection detection, path traversal protection
- ✅ Rate Limiting: Global, per-IP, per-bucket with token bucket algorithm
- ✅ Circuit Breaker: Automatic failure detection and recovery
- ✅ Metrics: Prometheus endpoint with comprehensive metrics
- ✅ Logging: Structured JSON with credential redaction
- 📋 Health Endpoints: Phase 22 in progress
- 📋 Graceful Shutdown: Phase 22 in progress
- 📋 Request Correlation: Phase 22 in progress

See [plan.md](plan.md) for detailed test checklist and [ROADMAP.md](ROADMAP.md) for implementation roadmap.

## Resources

### Project Resources

- **Original MVP**: https://github.com/julianshen/s3-envoy-proxy
- **Development Guide**: [CLAUDE.md](CLAUDE.md)
- **Specification**: [spec.md](spec.md)
- **Implementation Plan**: [plan.md](plan.md)

### Technology Documentation

- **Pingora**: https://github.com/cloudflare/pingora
- **AWS SDK for Rust**: https://aws.amazon.com/sdk-for-rust/
- **Tokio**: https://tokio.rs/
- **JWT in Rust**: https://docs.rs/jsonwebtoken/

### Methodology

- [Test-Driven Development](https://www.amazon.com/Test-Driven-Development-Kent-Beck/dp/0321146530) by Kent Beck
- [Tidy First?](https://www.amazon.com/Tidy-First-Personal-Exercise-Empirical/dp/1098151240) by Kent Beck

## License

[To be specified]

## Contact

[To be specified]

---

## Development Philosophy

> "Make it work, make it right, make it fast" - Kent Beck

We build software incrementally through small, tested steps. Each test drives a small behavior. Each behavior builds toward a complete feature. Each feature serves a real user need.

Quality is not an afterthought—it's built in from the first test.

---

**Ready to start? Say "go" and let's implement the next test! 🚀**
