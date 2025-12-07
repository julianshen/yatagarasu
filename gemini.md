# gemini.md

This file provides guidance to Gemini when working with code in this repository.

---

# YATAGARASU - S3 PROXY DEVELOPMENT GUIDE

**Project**: High-performance S3 proxy built with Rust and Cloudflare Pingora
**Methodology**: Kent Beck's Test-Driven Development (TDD) + Tidy First principles
**Language**: Rust 1.70+
**Test Coverage Target**: >90%

---

## PROJECT OVERVIEW

Yatagarasu is a high-performance S3 proxy that provides:
- **Multi-bucket routing** with isolated credentials
- **Flexible JWT authentication** (optional per-bucket)
- **Zero-copy streaming** for large files (constant memory usage)
- **Smart caching** for small files (<10MB)
- **HTTP Range request** support for video seeking and parallel downloads
- **Hot reload** configuration without downtime

### Quick Architecture
```
Client → Pingora HTTP Server → Router → JWT Auth (optional)
  → Cache Check → S3 Client (SigV4) → S3 Backend → Stream Response
```

**Key Insight**: Large files stream directly through the proxy without buffering to disk (constant ~64KB memory per connection). Small files can be cached in memory.

---

## DEVELOPMENT COMMANDS

### Build & Test
```bash
# Build the project
cargo build

# Build release binary
cargo build --release

# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run specific test by name
cargo test jwt_validation

# Run tests with output visible
cargo test -- --nocapture

# Run fast tests only (skip slow e2e)
cargo test --lib && cargo test --test integration_*
```

### Code Quality
```bash
# Lint (must pass before commit)

cargo clippy -- -D warnings

# Format code (must pass before commit)

cargo fmt

# Check formatting without applying

cargo fmt --check

# Test coverage report

cargo tarpaulin --out Html --output-dir coverage
```

### Integration Testing with MinIO
```bash
# Start MinIO (S3-compatible storage for testing)

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

### Running the Proxy
```bash
# Run with config file

cargo run -- --config config.yaml

# Run with environment variables

AWS_ACCESS_KEY_PRODUCTS=xxx \
AWS_SECRET_KEY_PRODUCTS=yyy \
JWT_SECRET=zzz \
cargo run -- --config config.yaml

# Check metrics

curl http://localhost:9090/metrics

# Check health

curl http://localhost:8080/health
```

---

## CODE ARCHITECTURE

### Module Structure
```
src/
├── main.rs           # Application entry point, server setup
├── lib.rs            # Library root, public API
├── config/           # Configuration loading, validation, env var substitution
│   └── mod.rs
├── router/           # Path-to-bucket routing, longest prefix matching
│   └── mod.rs
├── auth/             # JWT extraction (header/query/custom), validation, claims
│   └── mod.rs
├── s3/               # S3 client, AWS Signature v4, credential isolation
│   └── mod.rs
├── proxy/            # Pingora proxy implementation, streaming logic
│   └── mod.rs
├── cache/            # Cache layer (heap/mmap/disk), LRU eviction (v1.1)
│   └── mod.rs
└── error.rs          # Error types, HTTP status mapping, user-friendly messages
```

### Key Design Principles

**1. Per-Bucket Credential Isolation**
- Each bucket config gets its own S3 client
- No shared credentials = no risk of using wrong bucket credentials
- Security through isolation

**2. Zero-Copy Streaming for Large Files**
- Files >10MB stream directly through proxy (no buffering to disk)
- Constant ~64KB memory per connection regardless of file size
- Enables serving GB+ files to thousands of concurrent clients

**3. Smart Caching for Small Files**
- Files <10MB cached in memory (configurable)
- Async cache writes don't block client response
- Cache hits served in <10ms

**4. Range Requests Always Streamed**
- HTTP Range requests bypass cache entirely
- Always fetch from S3 on-demand
- Useful for video seeking, parallel downloads
- Memory: constant ~64KB per range request

**5. Optional Per-Bucket Authentication**
- JWT validation only (no token issuance)
- Multiple token sources: Authorization header, query param, custom header
- Custom claims verification with operators (equals, contains, in, gt, lt)
- Mixed public/private buckets in same proxy instance

---

## TDD WORKFLOW WITH PLAN.MD

### The "Go" Command
When you see "go" from the user:
1. Read `plan.md` or `plan_v1.2.md` to find the next unmarked test `[ ]`
2. Implement the test (Red phase - watch it fail)
3. Write minimum code to pass (Green phase)
4. Refactor if needed (keep tests green)
5. Mark test `[x]` in the plan file
6. Commit with appropriate prefix
7. Wait for next "go"

### Test Execution Strategy
- **Red Phase**: Write failing test first
- **Green Phase**: Minimum code to pass
- **Refactor Phase**: Improve structure while tests stay green
- **Commit**: Mark test complete, commit with prefix

### Example Test Progression
```
Phase 1: Foundation
[ ] Test: Cargo project compiles without errors
[ ] Test: Basic dependency imports work (Pingora, Tokio)
...

Phase 2: Configuration
[ ] Test: Can deserialize minimal valid YAML config
[ ] Test: Can access server address from config
...

Phase 4: JWT Authentication
[ ] Test: Extracts token from Authorization header with "Bearer " prefix
[ ] Test: Validates correctly signed JWT with HS256
...
```

---

## CORE DEVELOPMENT PRINCIPLES

- Always follow the TDD cycle: **Red → Green → Refactor**
- Write the simplest failing test first
- Implement the minimum code needed to make tests pass
- Refactor only after tests are passing
- Follow Beck's "Tidy First" approach by separating structural changes from behavioral changes
- Maintain high code quality throughout development
- Prioritize user experience and clarity in all implementations

---

## TDD METHODOLOGY GUIDANCE

- Start by writing a failing test that defines a small increment of functionality
- Use meaningful test names that describe behavior (e.g., `test_jwt_extracts_bearer_token`, `test_router_matches_longest_prefix`)
- Make test failures clear and informative with descriptive assertion messages
- Write just enough code to make the test pass - no more
- Once tests pass, consider if refactoring is needed
- Repeat the cycle for new functionality
- When fixing a defect:
  1. First write an API-level failing test that demonstrates the bug
  2. Then write the smallest possible test that replicates the problem
  3. Finally, implement the fix to make both tests pass

---

## TIDY FIRST APPROACH

Separate all changes into two distinct types:

### 1. STRUCTURAL CHANGES
Rearranging code without changing behavior:
- Renaming variables, methods, or classes for clarity
- Extracting methods or functions
- Moving code to more appropriate locations
- Reorganizing imports or dependencies
- Reformatting code

### 2. BEHAVIORAL CHANGES
Adding or modifying actual functionality:
- Implementing new features
- Fixing bugs that change program behavior
- Modifying algorithms or logic
- Adding new dependencies that change behavior

### Critical Rules:
- **Never mix structural and behavioral changes in the same commit**
- **Always make structural changes first** when both are needed
- Validate structural changes do not alter behavior by running tests before and after
- If a structural change breaks tests, revert and investigate

---

## COMMIT DISCIPLINE

Only commit when:
1. **ALL tests are passing** - No exceptions
2. **ALL compiler/linter warnings have been resolved** - Zero warnings policy (`cargo clippy -- -D warnings`)
3. **Code is properly formatted** - Run `cargo fmt` before commit
4. **The change represents a single logical unit of work** - One concept per commit
5. **Commit messages clearly state** whether the commit contains structural or behavioral changes
6. **Submit changes as a PR** - All changes must be submitted as a Pull Request, not directly pushed.

### Commit Message Format:
```
[STRUCTURAL] Extract validation logic into separate method
[BEHAVIORAL] Add support for JSON input format
[BEHAVIORAL] Fix authentication token expiration handling
[STRUCTURAL] Rename UserData to UserProfile for clarity
```

Use small, frequent commits rather than large, infrequent ones. Each commit should tell a story.

---

## CODE QUALITY STANDARDS

- **Eliminate duplication ruthlessly** - DRY principle applied consistently
- **Express intent clearly** through naming and structure - Code should read like prose
- **Make dependencies explicit** - No hidden coupling
- **Keep methods small** and focused on a single responsibility - SRP always
- **Minimize state and side effects** - Prefer pure functions when possible
- **Use the simplest solution** that could possibly work - YAGNI principle
- **Document why, not what** - The code shows what; comments explain why

### Rust-Specific Quality Standards
- Use `Result<T, E>` for error handling (no panics in production code)
- Prefer `&str` over `String` for function parameters when ownership not needed
- Use `impl Trait` or generics to avoid dynamic dispatch when possible
- Leverage Rust's type system for correctness (newtype pattern for validation)
- Write idiomatic Rust (follow clippy suggestions)

---

## REFACTORING GUIDELINES

- **Refactor only when tests are passing** (in the "Green" phase)
- **Use established refactoring patterns** with their proper names:
  - Extract Method
  - Rename Variable
  - Move Method
  - Extract Class/Module
  - Inline Method
  - etc.
- **Make one refactoring change at a time** - Small, safe steps
- **Run tests after each refactoring step** - Continuous validation
- **Prioritize refactorings that**:
  - Remove duplication
  - Improve clarity and readability
  - Reduce complexity
  - Make future changes easier

---

## EXAMPLE WORKFLOW

When approaching a new feature:

1. **Red Phase**: Write a simple failing test for a small part of the feature
   ```rust
   #[test]
   fn test_extracts_bearer_token_from_header() {
       let headers = create_test_headers("Authorization", "Bearer abc123");
       let token = extract_token(&headers, &TokenSource::BearerHeader);
       assert_eq!(token, Some("abc123".to_string()));
   }
   ```

2. **Green Phase**: Implement the bare minimum to make it pass
   ```rust
   fn extract_token(headers: &Headers, source: &TokenSource) -> Option<String> {
       match source {
           TokenSource::BearerHeader => {
               headers.get("authorization")
                   .and_then(|v| v.strip_prefix("Bearer "))
                   .map(|s| s.to_string())
           }
       }
   }
   ```

3. **Run Tests**: Confirm all tests pass (Green state)
   ```bash
   $ cargo test
   running 1 test
   test test_extracts_bearer_token_from_header ... ok
   ```

4. **Refactor Phase** (Tidy First): Make any necessary structural changes
   - Run tests after each structural change
   - Ensure tests remain green throughout

5. **Commit Structural Changes**: Separately from behavioral changes
   ```bash
   git commit -m "[STRUCTURAL] Extract token extraction to separate module"
   ```

6. **Continue**: Add another test for the next small increment of functionality

7. **Commit Behavioral Changes**: When feature increment is complete
   ```bash
   git commit -m "[BEHAVIORAL] Add Bearer token extraction from Authorization header"
   ```

8. **Repeat** until the feature is complete

---

## TESTING STRATEGY

### Test Pyramid
```
     /\
    /  \     E2E Tests (5%) - Full proxy with MinIO
   /____\
  /      \   Integration Tests (15%) - Component interactions
 /________\
/          \ Unit Tests (80%) - Individual functions
```

### Test Levels

**Unit Tests** (>90% coverage target)
- Individual functions and methods
- Fast execution (<1s total)
- No external dependencies (mock S3, JWT)
- Test behavior, not implementation

**Integration Tests** (all critical paths)
- Component interactions (router + auth + s3)
- May use test doubles or in-memory services
- Moderate execution time (<10s total)

**End-to-End Tests** (main workflows)
- Full stack with real MinIO instance
- Slow but comprehensive
- Mark with `#[ignore]` for normal test runs
- Run before releases

### Test File Organization
```
tests/
├── unit/              # Fast unit tests
│   ├── config_test.rs
│   ├── router_test.rs
│   ├── jwt_test.rs
│   └── s3_test.rs
├── integration/       # Component integration
│   ├── auth_flow_test.rs
│   └── routing_test.rs
└── e2e/              # Full stack tests
    └── proxy_test.rs
```

- Keep tests fast - slow tests kill TDD rhythm
- Make tests independent - no test should depend on another
- Use test doubles (mocks, stubs, fakes) judiciously
- Test behavior, not implementation details

---

## DEVELOPMENT RHYTHM

Always write one test at a time, make it run, then improve structure. Always run all tests (except explicitly marked long-running tests) after each change.

**The rhythm is:**
1. Add a test (Red)
2. Make it pass (Green)
3. Clean up (Refactor)
4. Commit
5. Repeat

This rhythm should become automatic, a comfortable cycle that produces clean, well-tested code.

---

## QUALITY GATES (Must Pass Before Commit)

```bash
# All must succeed:
cargo test              # ✅ All tests passing
cargo clippy -- -D warnings  # ✅ No warnings
cargo fmt --check       # ✅ Code formatted
# Coverage >90% (check with cargo tarpaulin)
```

**Never commit when:**
- Any test is failing
- Clippy shows any warnings
- Code is not formatted
- Mixing structural and behavioral changes

---

## COMMON PATTERNS IN THIS CODEBASE

### Configuration with Environment Variable Substitution
```rust
// In config module: Replace ${VAR_NAME} with env var value
let value = config_value.replace("${AWS_KEY}", &env::var("AWS_KEY")?);
```

### Per-Bucket S3 Client Creation
```rust
// Each bucket gets isolated S3 client
for bucket_config in config.buckets {
    let client = create_s3_client(&bucket_config.s3);
    clients.insert(bucket_config.name, client);
}
```

### Streaming S3 Response
```rust
// Stream chunks without buffering full file
while let Some(chunk) = s3_stream.next().await {
    client_stream.send(chunk).await?;
    if client_stream.is_closed() {
        s3_stream.cancel(); // Stop S3 transfer if client gone
        break;
    }
}
```

### JWT Token Source Priority
```rust
// Try each configured source in order
for source in &config.jwt.token_sources {
    if let Some(token) = extract_token(request, source) {
        return validate_jwt(&token, &config.jwt.secret);
    }
}
```

---

## WHEN IN DOUBT

- Write a test first
- Make the smallest possible change
- Keep tests passing
- Commit frequently with clear prefixes
- Consult `plan.md` or `plan_v1.2.md` for the next test to implement
- Review `spec.md` for requirements clarification
- Check `docs/` for architecture details (streaming, caching, range requests)

---

## PERFORMANCE TARGETS

| Metric | Target | How to Verify |
|--------|--------|---------------|
| JWT validation | <1ms | `cargo bench jwt_validation` |
| Path routing | <10μs | `cargo bench routing` |
| S3 signature gen | <100μs | `cargo bench s3_signature` |
| Cache hit response | <10ms P95 | Integration test with timing |
| S3 streaming TTFB | <500ms P95 | E2E test with MinIO |
| Throughput | >10,000 req/s | Load test with `wrk` or `hey` |
| Memory per connection | ~64KB | Monitor during stress test |

---

## DOCUMENTATION RESOURCES

- **README.md** - Project overview, quick start, features
- **spec.md** - Complete technical specification (35KB)
- **plan.md** / **plan_v1.2.md** - TDD implementation plan with test checklist
- **docs/STREAMING_ARCHITECTURE.md** - Detailed streaming and caching architecture
- **docs/RANGE_REQUESTS.md** - HTTP Range request support
- **docs/PARALLEL_DOWNLOADS.md** - Parallel download via range requests
- **docs/CACHE_MANAGEMENT.md** - Cache purge/renewal/conditional requests

---

## WORKFLOW SUMMARY

```
┌─────────────────────────────────────────────────────┐
│ 1. User says "go"                                   │
│ 2. Read plan_v1.2.md, find next [ ] test            │
│ 3. Write failing test (Red)                         │
│ 4. Write minimum code to pass (Green)               │
│ 5. Refactor if needed (keep green)                  │
│ 6. Run cargo test, clippy, fmt                      │
│ 7. Mark test [x] in plan_v1.2.md                    │
│ 8. Commit with [BEHAVIORAL] or [STRUCTURAL] prefix  │
│ 9. Repeat                                            │
└─────────────────────────────────────────────────────┘
```

Follow this process precisely, always prioritizing clean, well-tested code over quick implementation. **Quality is not negotiable.**

---

**Ready to start? Say "go" and let's implement the next test!**
