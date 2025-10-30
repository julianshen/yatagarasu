# Yatagarasu (八咫烏)

> _"The three-legged crow that guides the way to secure S3 access"_

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-TDD-green.svg)](plan.md)

A high-performance S3 proxy built with Cloudflare's Pingora framework and Rust, providing intelligent routing, multi-bucket support, and flexible JWT authentication.

## What is Yatagarasu?

Yatagarasu is a reimplementation of [s3-envoy-proxy](https://github.com/julianshen/s3-envoy-proxy) using modern Rust async architecture. It provides:

- 🚀 **High Performance**: 70% lower CPU usage compared to traditional proxies (via Pingora)
- 🗂️ **Multi-Bucket Routing**: Map different S3 buckets to different URL paths with isolated credentials
- 🔐 **Flexible JWT Auth**: Optional authentication with multiple token sources (header, query, custom)
- 🎯 **Custom Claims**: Verify JWT claims with configurable logic (role, tenant, etc.)
- 📊 **Observable**: Prometheus metrics and structured JSON logging
- 🔄 **Hot Reload**: Update configuration without downtime
- 🧪 **Well-Tested**: >90% test coverage following TDD principles

**Name Origin**: Yatagarasu (八咫烏) is the three-legged crow in Japanese mythology that serves as a divine messenger and guide. Like its namesake, this proxy guides and securely routes requests to the appropriate S3 buckets.

## Quick Start

### Prerequisites

- Rust 1.70 or later
- S3-compatible storage (AWS S3, MinIO, LocalStack, etc.)
- (Optional) JWT token issuer for authentication

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/yatagarasu.git
cd yatagarasu

# Build
cargo build --release

# Run tests
cargo test

# Run the proxy
./target/release/yatagarasu --config config.yaml
```

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

### Example Requests

```bash
# Access public bucket
curl http://localhost:8080/products/image.png

# Access private bucket with JWT
curl -H "Authorization: Bearer eyJhbGc..." \
  http://localhost:8080/private/data.json

# Or with query parameter
curl http://localhost:8080/private/data.json?token=eyJhbGc...

# Check metrics
curl http://localhost:9090/metrics
```

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

### ✅ Core Features (v1.0)

- [x] **Multi-Bucket Routing**: Map S3 buckets to URL path prefixes
- [x] **Credential Isolation**: Each bucket uses independent AWS credentials
- [x] **JWT Authentication**: Optional, per-bucket authentication
- [x] **Multiple Token Sources**: Extract JWT from header, query param, or custom header
- [x] **Custom Claims Verification**: Flexible rules engine for JWT claims
- [x] **S3 Operations**: GET and HEAD object support
- [x] **Response Streaming**: Efficient streaming of large S3 objects
- [x] **Configuration Hot Reload**: Update config without downtime
- [x] **Prometheus Metrics**: Request counts, latencies, error rates
- [x] **Structured Logging**: JSON logs for aggregation systems
- [x] **Health Checks**: Liveness and readiness endpoints
- [x] **Graceful Shutdown**: Clean shutdown without dropping requests

### 🚧 Planned Features (v1.1+)

- [ ] Advanced caching (mmap, disk layers)
- [ ] Cache invalidation API
- [ ] Request/response transformation
- [ ] Rate limiting per client
- [ ] Multi-region S3 failover

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

### Performance Targets

- JWT validation: <1ms per token
- Path routing: <10μs per request
- S3 signature generation: <100μs
- Request handling: <100ms P95 (cached), <500ms P95 (S3)
- Throughput: >10,000 requests/second
- Memory: <500MB base, scales linearly with connections

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

**Current Phase**: Phase 1 - Foundation and Project Setup

**Progress**:

- Tests written: 0
- Tests passing: 0
- Test coverage: 0%

**Next Milestone**: Complete Phase 1 project setup and configuration management

See [plan.md](plan.md) for detailed implementation status.

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
