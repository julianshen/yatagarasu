# Yatagarasu v1.2.0 Development Plan

**Version**: 1.2.0
**Status**: Planning
**Focus**: Performance Benchmarks, Extended Testing, Production Hardening, Advanced Features
**Methodology**: TDD + Tidy First

---

## Overview

v1.2.0 focuses on production hardening through comprehensive benchmarking, long-duration endurance testing, extreme-scale scenarios, and advanced features that were deferred from v1.1.0.

### Version Summary

| Milestone | Description | Phases | Status |
|-----------|-------------|--------|--------|
| 1 | Performance Benchmarks (Criterion) | 40-43 | Planned |
| 2 | Extended JWT Support | 44-47 | Planned |
| 3 | OpenFGA Integration | 48-50 | Planned |
| 4 | Endurance & Long-Duration Testing | 51-54 | Planned |
| 5 | Extreme Scale & Stress Testing | 55-58 | Planned |
| 6 | Production Resilience | 59-61 | Planned |
| 7 | Horizontal Scaling | 62-64 | Planned |
| 8 | Advanced Features | 65-67 | Planned |
| 9 | Documentation & Polish | 68 | Planned |

---

## MILESTONE 1: Performance Benchmarks (Criterion)

### Goals
- Establish baseline performance metrics using Criterion benchmarks
- Identify bottlenecks and optimization opportunities
- Create reproducible benchmark suite for regression testing

---

### PHASE 40: Core Component Benchmarks

**Objective**: Create Criterion benchmarks for fundamental operations

#### 40.1 JWT Validation Benchmarks
- [x] Setup: Add criterion dev-dependency to Cargo.toml
- [x] Bench: HS256 JWT validation (target: <1ms) - 1.78µs ✓
- [x] Bench: HS256 with 5 claims verification - 2.12µs ✓
- [x] Bench: HS256 with 10 claims verification - 2.98µs ✓
- [x] Bench: Token extraction from Bearer header - 1.44µs ✓
- [x] Bench: Token extraction from query parameter - 2.2ns ✓
- [x] Bench: Token extraction from custom header - 1.58µs ✓
- [x] Bench: Claims parsing with nested objects - 2.58µs ✓
- [x] Bench: Expired token detection - 1.45µs ✓
- [x] Report: Generate baseline metrics (HS384: 1.95µs, HS512: 2.16µs, claims: 2.13µs)

**Success Criteria**:
- JWT validation P99 <1ms
- Token extraction P99 <100μs
- Benchmark variance <5%

#### 40.2 Routing Benchmarks
- [x] Bench: Single bucket path matching (target: <10μs) - 41.8ns ✓
- [x] Bench: 5 bucket path matching - 81.8ns ✓
- [x] Bench: 10 bucket path matching - 95.9ns ✓
- [x] Bench: 50 bucket path matching - 183ns ✓
- [x] Bench: Longest prefix match with nested paths - short: 43.8ns, medium: 46.8ns, long: 75.8ns ✓
- [x] Bench: Path normalization overhead - clean: 74.1ns, double slashes: 77.6ns (~5% overhead) ✓
- [x] Bench: Bucket lookup by name - O(n) linear: 5b=7.5ns, 10b=14.5ns, 50b=72.7ns, 100b=144.7ns ✓
- [x] Report: Generate baseline metrics - See summary below ✓

**Phase 40.2 Routing Benchmark Summary**:
| Benchmark | Result | Target | Status |
|-----------|--------|--------|--------|
| Single bucket routing | 41.8ns | <10μs | PASS |
| 10 bucket routing | 95.9ns | <10μs | PASS |
| 50 bucket routing | 183ns | <10μs | PASS |
| Longest prefix (nested) | 43-76ns | <10μs | PASS |
| Path normalization overhead | ~5% | N/A | OK |
| Bucket lookup by name (100b) | 145ns | <10μs | PASS |

**Success Criteria**:
- Path routing P99 <10μs for 10 buckets - **ACHIEVED** (95.9ns = 0.096μs)
- Linear scaling with bucket count - **ACHIEVED** (O(n) behavior confirmed)
- No allocation per routing decision - **ACHIEVED** (stack-only operations)

#### 40.3 S3 Signature Benchmarks
- [ ] Bench: SigV4 signature generation (target: <100μs)
- [ ] Bench: Canonical request creation
- [ ] Bench: String to sign creation
- [ ] Bench: HMAC-SHA256 computation
- [ ] Bench: Date formatting (ISO 8601)
- [ ] Bench: Header canonicalization with 5 headers
- [ ] Bench: Header canonicalization with 15 headers
- [ ] Report: Generate baseline metrics

**Success Criteria**:
- SigV4 signature P99 <100μs
- No excessive allocations
- Reusable signing key optimization verified

---

### PHASE 41: Cache Layer Benchmarks

**Objective**: Benchmark cache operations across all layers

#### 41.1 Memory Cache Benchmarks
- [ ] Bench: Cache get hit (warm cache)
- [ ] Bench: Cache get miss
- [ ] Bench: Cache set (small entry <1KB)
- [ ] Bench: Cache set (medium entry 100KB)
- [ ] Bench: Cache set (large entry 1MB)
- [ ] Bench: Cache delete single entry
- [ ] Bench: Cache eviction under memory pressure
- [ ] Bench: Concurrent get (10 threads)
- [ ] Bench: Concurrent get (100 threads)
- [ ] Bench: Cache key generation (hash function)
- [ ] Report: Generate baseline metrics

**Success Criteria**:
- Cache hit P99 <1ms
- Cache miss P99 <100μs
- Concurrent access scales linearly

#### 41.2 Disk Cache Benchmarks
- [ ] Bench: Disk cache get hit (SSD)
- [ ] Bench: Disk cache get miss
- [ ] Bench: Disk cache set (small entry)
- [ ] Bench: Disk cache set (large entry)
- [ ] Bench: Index lookup time
- [ ] Bench: Index update time
- [ ] Bench: LRU eviction performance
- [ ] Bench: Orphan file detection
- [ ] Report: Compare vs memory cache

**Success Criteria**:
- Disk hit P99 <10ms
- Index lookup O(1)
- Eviction doesn't block reads

#### 41.3 Redis Cache Benchmarks
- [ ] Bench: Redis get hit (local Redis)
- [ ] Bench: Redis get miss
- [ ] Bench: Redis set (small entry)
- [ ] Bench: Redis set (1MB entry)
- [ ] Bench: Redis delete
- [ ] Bench: Connection pool checkout time
- [ ] Bench: Pipeline vs single operations
- [ ] Bench: Serialization overhead (bincode)
- [ ] Report: Compare vs memory/disk

**Success Criteria**:
- Redis hit P99 <5ms (local)
- Connection pool efficient
- Serialization overhead <10%

---

### PHASE 42: Proxy Pipeline Benchmarks

**Objective**: Benchmark end-to-end request processing

#### 42.1 Request Processing Benchmarks
- [ ] Bench: Minimal request (health check)
- [ ] Bench: Request parsing overhead
- [ ] Bench: Response header construction
- [ ] Bench: Full pipeline (cache hit, no auth)
- [ ] Bench: Full pipeline (cache hit, JWT auth)
- [ ] Bench: Full pipeline (cache miss, S3 fetch)
- [ ] Bench: Range request parsing
- [ ] Bench: Multi-range request parsing
- [ ] Report: Identify pipeline bottlenecks

**Success Criteria**:
- Health check P99 <100μs
- Cache hit pipeline P99 <10ms
- S3 fetch dominated by network latency

#### 42.2 Streaming Benchmarks
- [ ] Bench: Stream initialization overhead
- [ ] Bench: Chunk processing (64KB chunks)
- [ ] Bench: Chunk processing (1MB chunks)
- [ ] Bench: Backpressure handling
- [ ] Bench: Client disconnect detection
- [ ] Bench: Memory allocation during streaming
- [ ] Report: Verify constant memory streaming

**Success Criteria**:
- Streaming overhead <1% of throughput
- Memory constant regardless of file size
- Backpressure doesn't cause stalls

---

### PHASE 43: Benchmark Infrastructure

**Objective**: Set up CI/CD benchmark integration

#### 43.1 Benchmark CI Setup
- [ ] Setup: GitHub Actions workflow for benchmarks
- [ ] Setup: Benchmark result storage (artifact/DB)
- [ ] Setup: Regression detection (>10% threshold)
- [ ] Setup: Benchmark comparison between commits
- [ ] Test: PR benchmark comments
- [ ] Document: How to run benchmarks locally

#### 43.2 Benchmark Dashboard
- [ ] Setup: Historical benchmark tracking
- [ ] Setup: Visualization (optional: grafana/charts)
- [ ] Setup: Alert on regression
- [ ] Document: Benchmark interpretation guide

**Deliverables**:
- `benches/` directory with all Criterion benchmarks
- CI workflow running on each PR
- Baseline metrics documented

---

## MILESTONE 2: Extended JWT Support

### Goals
- Support RS256 and ES256 algorithms
- Support JWKS (JSON Web Key Set) for key rotation
- Enable enterprise authentication scenarios

---

### PHASE 44: RS256 Support

**Objective**: Add RSA signature verification

#### 44.1 RS256 Implementation
- [ ] Test: Can parse RS256 config example
- [ ] Test: Load RSA public key from PEM
- [ ] Test: Validate RS256 signed JWT
- [ ] Test: Reject tampered RS256 JWT
- [ ] Test: Reject RS256 with wrong key
- [ ] Impl: Add RS256 to JwtAlgorithm enum
- [ ] Impl: RSA signature verification
- [ ] Doc: RS256 configuration example

#### 44.2 RS256 Key Management
- [ ] Test: Load RSA key from file path
- [ ] Test: Load RSA key from environment variable
- [ ] Test: Handle invalid RSA key format
- [ ] Test: Handle RSA key with passphrase (error)
- [ ] Impl: RSA key loading utilities

**Success Criteria**:
- RS256 validation works with standard libraries
- Configuration intuitive
- Clear error messages for key issues

---

### PHASE 45: ES256 Support

**Objective**: Add ECDSA signature verification

#### 45.1 ES256 Implementation
- [ ] Test: Can parse ES256 config example
- [ ] Test: Load EC public key from PEM
- [ ] Test: Validate ES256 signed JWT
- [ ] Test: Reject tampered ES256 JWT
- [ ] Test: Reject ES256 with wrong key
- [ ] Impl: Add ES256 to JwtAlgorithm enum
- [ ] Impl: ECDSA signature verification
- [ ] Doc: ES256 configuration example

#### 45.2 ES256 Key Management
- [ ] Test: Load EC key from file path
- [ ] Test: Load EC key from environment variable
- [ ] Test: Handle invalid EC key format
- [ ] Test: Validate EC key curve (P-256)
- [ ] Impl: EC key loading utilities

**Success Criteria**:
- ES256 validation works with standard libraries
- P-256 curve enforced
- Performance comparable to HS256

---

### PHASE 46: JWKS Support

**Objective**: Support JSON Web Key Sets for key rotation

#### 46.1 JWKS Fetching
- [ ] Test: Can parse JWKS config example
- [ ] Test: Fetch JWKS from URL
- [ ] Test: Parse JWKS JSON response
- [ ] Test: Extract RSA keys from JWKS
- [ ] Test: Extract EC keys from JWKS
- [ ] Test: Handle JWKS fetch timeout
- [ ] Test: Handle JWKS parse error
- [ ] Impl: JWKS HTTP client
- [ ] Impl: JWKS parser

#### 46.2 JWKS Key Matching
- [ ] Test: Match JWT kid to JWKS key
- [ ] Test: Return error if kid not in JWKS
- [ ] Test: Handle JWT without kid (use first key)
- [ ] Test: Handle multiple keys with same algorithm
- [ ] Impl: Key selection logic

#### 46.3 JWKS Caching & Refresh
- [ ] Test: Cache JWKS response (configurable TTL)
- [ ] Test: Refresh JWKS on cache expiry
- [ ] Test: Refresh JWKS on unknown kid (grace refresh)
- [ ] Test: Rate limit JWKS refreshes
- [ ] Impl: JWKS cache with TTL
- [ ] Doc: JWKS refresh configuration

**Success Criteria**:
- JWKS integrates with Auth0/Okta/Keycloak
- Key rotation seamless
- Reasonable caching prevents excessive fetches

---

### PHASE 47: JWT Security Hardening

**Objective**: Prevent JWT-related attacks

#### 47.1 Algorithm Confusion Prevention
- [ ] Test: Reject HS256 JWT with RS256 config (alg confusion)
- [ ] Test: Reject RS256 JWT with HS256 config
- [ ] Test: Reject "none" algorithm JWT
- [ ] Test: Reject algorithm downgrade attempts
- [ ] Impl: Strict algorithm enforcement

#### 47.2 Integration Tests
- [ ] Test: End-to-end test with RS256 JWT
- [ ] Test: End-to-end test with ES256 JWT
- [ ] Test: End-to-end test with JWKS
- [ ] Test: Key rotation scenario (old + new key both work)
- [ ] Test: Multi-algorithm configuration

**Success Criteria**:
- No algorithm confusion vulnerabilities
- All standard enterprise JWT scenarios work
- Comprehensive test coverage

---

## MILESTONE 3: OpenFGA Integration

### Goals
- Add relationship-based access control via OpenFGA (https://openfga.dev/)
- Enable fine-grained authorization based on Google Zanzibar model
- Support both OPA (policy-based) and OpenFGA (relationship-based) authorization
- Provide flexible choice between authorization approaches per bucket

### Why OpenFGA?

OpenFGA provides **relationship-based access control (ReBAC)** which complements OPA's policy-based approach:

| Feature | OPA (Rego) | OpenFGA |
|---------|------------|---------|
| Model | Policy-based (ABAC) | Relationship-based (ReBAC) |
| Best for | Complex business rules | Object hierarchies, sharing |
| Query | "Can user X do Y?" | "Does user X have relation R to object O?" |
| Performance | Inline evaluation | Graph traversal |
| Use case | File type rules, time-based access | Folder sharing, team hierarchies |

---

### PHASE 48: OpenFGA Client Foundation

**Objective**: Create OpenFGA client with basic authorization checks

**Reference**: https://openfga.dev/docs/getting-started

#### 48.1 OpenFGA Configuration
- [ ] Test: Parse OpenFGA config from bucket auth section
- [ ] Test: Validate OpenFGA endpoint URL
- [ ] Test: Validate store_id configuration
- [ ] Test: Validate authorization_model_id (optional)
- [ ] Test: Support API token authentication
- [ ] Impl: Add OpenFgaConfig struct to config module
- [ ] Doc: OpenFGA configuration example in config.yaml

```yaml
# Example OpenFGA config
buckets:
  - name: "shared-files"
    auth:
      enabled: true
      provider: "openfga"  # or "opa" or "jwt"
      openfga:
        endpoint: "http://localhost:8080"
        store_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV"
        authorization_model_id: "01GXSA8YR785C4FYS3C0RTG7B1"  # optional
        api_token: "${OPENFGA_API_TOKEN}"  # optional, for cloud
```

#### 48.2 OpenFGA Client Implementation
- [ ] Test: Create OpenFGA HTTP client
- [ ] Test: Client handles connection errors gracefully
- [ ] Test: Client implements timeout (configurable)
- [ ] Test: Client retries on transient failures
- [ ] Impl: OpenFgaClient struct with reqwest
- [ ] Impl: Check endpoint for authorization queries

#### 48.3 Authorization Check API
- [ ] Test: Check() returns allowed=true for permitted access
- [ ] Test: Check() returns allowed=false for denied access
- [ ] Test: Check() handles network timeout
- [ ] Test: Check() handles invalid store_id
- [ ] Test: Check() handles 400 Bad Request (invalid tuple)
- [ ] Impl: Check API call with user, relation, object

**Success Criteria**:
- OpenFGA client connects successfully
- Authorization checks return correct results
- Errors handled gracefully

---

### PHASE 49: OpenFGA Integration with Proxy

**Objective**: Integrate OpenFGA authorization into request flow

#### 49.1 Authorization Model Design
- [ ] Design: Map S3 paths to OpenFGA objects
- [ ] Design: Define relations (viewer, editor, owner, etc.)
- [ ] Design: Define user extraction from JWT
- [ ] Doc: Example authorization model for S3 proxy

```
# Example OpenFGA model for S3 access
type user

type folder
  relations
    define owner: [user]
    define editor: [user, folder#editor]
    define viewer: [user, folder#viewer, folder#editor, folder#owner]

type file
  relations
    define parent: [folder]
    define owner: [user]
    define editor: [user] or editor from parent
    define viewer: [user] or viewer from parent or editor
```

#### 49.2 Request Authorization Flow
- [ ] Test: Extract user ID from JWT claims (configurable claim)
- [ ] Test: Build OpenFGA object from bucket + path
- [ ] Test: Build OpenFGA relation from HTTP method (GET→viewer, PUT→editor)
- [ ] Test: Check authorization before proxying
- [ ] Test: Return 403 on authorization failure
- [ ] Test: Return 500 on OpenFGA error (fail closed)
- [ ] Impl: OpenFGA authorizer middleware

#### 49.3 Authorization Caching
- [ ] Test: Cache positive authorization decisions (configurable TTL)
- [ ] Test: Cache negative authorization decisions (shorter TTL)
- [ ] Test: Cache key includes user, relation, object
- [ ] Test: Cache invalidation on TTL expiry
- [ ] Impl: Moka cache for OpenFGA decisions
- [ ] Config: decision_cache_ttl_seconds (default: 60)

**Success Criteria**:
- Authorization integrated into request flow
- Proper error handling and fail-closed behavior
- Caching reduces OpenFGA load

---

### PHASE 50: OpenFGA Testing & Documentation

**Objective**: Comprehensive testing and documentation

#### 50.1 Integration Tests
- [ ] Test: End-to-end with real OpenFGA container
- [ ] Test: Folder hierarchy permission inheritance
- [ ] Test: User can access shared folder
- [ ] Test: User denied access to unshared folder
- [ ] Test: Owner has full access
- [ ] Test: Mixed OPA + OpenFGA buckets work together

#### 50.2 Load Testing
- [ ] Setup: k6 script for OpenFGA load testing
- [ ] Test: 500 RPS with 80% cache hit rate
- [ ] Verify: P95 latency <100ms (with caching)
- [ ] Verify: P95 latency <500ms (without caching)
- [ ] Verify: OpenFGA doesn't become bottleneck

#### 50.3 Docker Compose Setup
- [ ] Setup: docker-compose.openfga.yml with OpenFGA server
- [ ] Setup: Pre-loaded authorization model
- [ ] Setup: Sample data for testing
- [ ] Doc: How to run OpenFGA locally

#### 50.4 Documentation
- [ ] Doc: OpenFGA vs OPA comparison and when to use each
- [ ] Doc: Configuration guide for OpenFGA
- [ ] Doc: Example authorization models for common use cases
- [ ] Doc: Performance tuning (caching, connection pooling)

**Deliverables**:
- `src/auth/openfga/` module with client and authorizer
- `docker-compose.openfga.yml` for local testing
- `k6/openfga-load.js` load testing script
- Documentation in `docs/OPENFGA.md`

**Success Criteria**:
- OpenFGA works alongside existing OPA
- Easy to configure per-bucket
- Well documented with examples

---

## MILESTONE 4: Endurance & Long-Duration Testing

### Goals
- Verify stability over extended periods (24+ hours)
- Detect memory leaks and resource exhaustion
- Validate production readiness

---

### PHASE 51: Memory Cache Endurance

**Objective**: Test memory cache stability over 24+ hours

#### 51.1 24-Hour Memory Cache Test
- [ ] Setup: k6 script for 24-hour sustained load
- [ ] Test: 500 RPS, 24 hours, 70% hit rate
- [ ] Monitor: CPU usage over time (should be flat)
- [ ] Monitor: Memory usage over time (should be flat)
- [ ] Monitor: Cache hit rate stability
- [ ] Verify: No memory leaks (RSS stable)
- [ ] Verify: No gradual performance degradation
- [ ] Verify: P95 latency consistent throughout
- [ ] Report: Generate 24-hour metrics summary

**Success Criteria**:
- Memory growth <10% over 24 hours
- No performance degradation
- Cache hit rate stable ±5%

#### 51.2 Memory Pressure Recovery
- [ ] Test: Fill cache to 100% repeatedly
- [ ] Test: Verify eviction reclaims memory
- [ ] Test: Verify no fragmentation buildup
- [ ] Monitor: Memory efficiency over time

---

### PHASE 52: Disk Cache Endurance

**Objective**: Test disk cache stability over 24+ hours

#### 52.1 24-Hour Disk Cache Test
- [ ] Setup: k6 script for 24-hour disk cache load
- [ ] Test: 100 RPS, 24 hours, 60% hit rate
- [ ] Monitor: Disk usage over time
- [ ] Monitor: Index file size (should not grow unbounded)
- [ ] Verify: No orphaned files accumulate
- [ ] Verify: Performance remains consistent
- [ ] Verify: LRU eviction works correctly
- [ ] Report: Generate 24-hour metrics summary

**Success Criteria**:
- Disk usage stable (eviction working)
- Index file size bounded
- No orphaned files

#### 52.2 Disk Recovery Tests
- [ ] Test: Recovery after disk full condition
- [ ] Test: Recovery after abrupt shutdown
- [ ] Test: Index rebuild performance

---

### PHASE 53: Redis Cache Endurance

**Objective**: Test Redis cache stability over 24+ hours

#### 53.1 24-Hour Redis Cache Test
- [ ] Setup: k6 script for 24-hour Redis load
- [ ] Test: 100 RPS, 24 hours, 70% hit rate
- [ ] Monitor: Redis connection pool stability
- [ ] Verify: No connection leaks
- [ ] Verify: Redis memory stable
- [ ] Verify: TTL expiration working correctly
- [ ] Report: Generate 24-hour metrics summary

#### 53.2 Redis Advanced Configuration Tests
- [ ] Test: Redis maxmemory-policy=allkeys-lru
- [ ] Verify: Redis evictions happen correctly
- [ ] Test: Redis with authentication
- [ ] Test: Redis Sentinel failover (if applicable)

**Success Criteria**:
- Connection pool stable
- No connection leaks
- Redis memory bounded

---

### PHASE 54: Tiered Cache Endurance

**Objective**: Test tiered cache stability over extended period

#### 54.1 Extended Tiered Cache Test
- [ ] Test: 100 RPS, 2 hours, 80% total hit rate
- [ ] Verify: Memory layer stays within limits
- [ ] Verify: Disk layer evicts correctly
- [ ] Verify: Redis layer TTLs work correctly
- [ ] Verify: Promotion keeps hot data in fast layers
- [ ] Monitor: Per-layer hit rates over time

#### 54.2 Layer Failure Recovery
- [ ] Test: 100 RPS, disable Redis mid-test → verify fallback to disk
- [ ] Test: 100 RPS, disable disk mid-test → verify fallback to memory
- [ ] Verify: Error rate <1% during layer failure
- [ ] Verify: Automatic recovery when layer restored

**Success Criteria**:
- Graceful degradation on layer failure
- Automatic recovery
- No data corruption

---

## MILESTONE 5: Extreme Scale & Stress Testing

### Goals
- Test behavior at extreme scales
- Identify breaking points and limits
- Document scaling characteristics

---

### PHASE 55: Extreme Large File Streaming

**Objective**: Test streaming with very large files (5GB+)

#### 55.1 5GB File Streaming
- [ ] Setup: Create 5GB test file in MinIO
- [ ] Test: Stream 5GB file, verify memory <100MB
- [ ] Test: 5 concurrent 5GB downloads
- [ ] Test: Range requests on 5GB file
- [ ] Monitor: Memory during streaming
- [ ] Verify: Throughput matches network capacity

#### 55.2 10GB File Streaming
- [ ] Setup: Create 10GB test file in MinIO
- [ ] Test: Stream 10GB file, verify memory <100MB
- [ ] Test: 3 concurrent 10GB downloads
- [ ] Monitor: Memory stability over download duration
- [ ] Verify: No timeout issues

**Success Criteria**:
- Memory constant regardless of file size
- Throughput network-limited, not proxy-limited
- No timeouts on large files

---

### PHASE 56: Extreme Concurrency

**Objective**: Test high concurrency scenarios

#### 56.1 High Concurrent Downloads
- [ ] Test: 50 concurrent 1GB downloads
- [ ] Test: 100 concurrent 5GB downloads (if infrastructure allows)
- [ ] Monitor: Memory usage per connection
- [ ] Monitor: File descriptor usage
- [ ] Verify: No connection drops

#### 56.2 Massive Concurrent Requests
- [ ] Test: 10,000 concurrent connections (cache hits)
- [ ] Test: Measure max sustainable concurrency
- [ ] Verify: Graceful behavior at limits
- [ ] Document: Recommended max concurrency

**Success Criteria**:
- Linear memory growth with connections
- Graceful degradation at limits
- Clear documentation of limits

---

### PHASE 57: Mixed Workload Testing

**Objective**: Test realistic production workloads

#### 57.1 Cache + Streaming Mix
- [ ] Test: 50% small files (<1MB, cached), 50% large files (>10MB, streamed)
- [ ] Test: 1000 RPS total, 10 minutes
- [ ] Verify: Small files benefit from cache
- [ ] Verify: Large files stream efficiently
- [ ] Verify: Cache metrics only track cacheable files

#### 57.2 Resource Isolation
- [ ] Test: Concurrent cache hits and large file streams
- [ ] Verify: Cache hits fast (<10ms) even during streaming load
- [ ] Verify: Streaming doesn't impact cache performance
- [ ] Verify: Resource isolation between paths

#### 57.3 Extended Mixed Load
- [ ] Test: 100 concurrent users, 1GB files each, 30 minutes
- [ ] Verify: Memory stable <1GB total
- [ ] Verify: Throughput limited only by network/S3
- [ ] Verify: No performance degradation over time
- [ ] Verify: P95 TTFB <500ms

**Success Criteria**:
- Workloads don't interfere
- Consistent performance
- Memory predictable

---

### PHASE 58: CPU Core Scaling

**Objective**: Measure performance scaling with CPU cores

#### 58.1 Linear Scaling Tests
- [ ] Test: 1 CPU core, measure max RPS
- [ ] Test: 2 CPU cores, measure max RPS
- [ ] Test: 4 CPU cores, measure max RPS
- [ ] Test: 8 CPU cores, measure max RPS
- [ ] Test: 16 CPU cores, measure max RPS
- [ ] Verify: Performance scales linearly (up to a point)
- [ ] Measure: Identify CPU bottleneck point

#### 58.2 Thread Pool Optimization
- [ ] Measure: Tokio runtime thread pool usage per core count
- [ ] Measure: Work stealing effectiveness
- [ ] Verify: No thread pool starvation
- [ ] Document: Recommended worker thread configuration

**Deliverables**:
- Scaling characteristics documentation
- Core count recommendations

---

## MILESTONE 6: Production Resilience

### Goals
- Test failure scenarios
- Verify graceful degradation
- Validate hot reload and shutdown

---

### PHASE 59: Backend Failure Handling

**Objective**: Test S3 backend failure scenarios

#### 59.1 S3 Error Handling
- [ ] Test: S3 503 errors → circuit breaker opens
- [ ] Test: S3 unreachable → 504 Gateway Timeout
- [ ] Test: Slow S3 (2s+ latency) → timeouts work
- [ ] Test: High error rate (50% 500s) → circuit breaker protects

#### 59.2 Cache Failure Handling
- [ ] Test: Memory cache full → eviction works
- [ ] Test: Disk cache full → eviction works
- [ ] Test: Redis connection lost → falls back to disk
- [ ] Test: Disk I/O errors → logs error, continues serving

#### 59.3 Replica Failover (Future)
- [ ] Test: Primary replica failure → failover <5s
- [ ] Test: Backup failure → tertiary fallback
- [ ] Test: Primary recovery → returns to primary
- [ ] Test: Failover during load → <1% error rate spike

**Success Criteria**:
- Clear error responses
- Graceful degradation
- Automatic recovery

---

### PHASE 60: Hot Reload Testing

**Objective**: Verify zero-downtime configuration updates

#### 60.1 Hot Reload Under Load
- [ ] Test: Config reload while serving 100+ req/s
- [ ] Test: Zero dropped requests during reload
- [ ] Test: New config applies immediately
- [ ] Test: Cache preserved during reload

#### 60.2 Graceful Shutdown
- [ ] Test: SIGTERM while serving 1000+ connections
- [ ] Test: All in-flight requests complete
- [ ] Test: No broken pipes or connection resets
- [ ] Test: Cache state persisted (disk/redis)

**Success Criteria**:
- Zero dropped requests on reload
- Graceful connection draining
- State preservation

---

### PHASE 61: OPA Load Testing

**Objective**: Verify OPA integration performance

#### 61.1 OPA Performance Tests
- [ ] Execute: `opa_constant_rate` - 500 req/s for 30s (baseline throughput)
- [ ] Execute: `opa_ramping` - 10→100→50 VUs (find saturation point)
- [ ] Execute: `opa_cache_hit` - 1000 req/s same user (cache effectiveness)
- [ ] Execute: `opa_cache_miss` - 200 req/s unique paths (uncached evaluation)

#### 61.2 OPA Verification
- [ ] Verify: P95 latency <200ms (with OPA + S3 backend)
- [ ] Verify: Auth latency P95 <50ms (OPA evaluation only)
- [ ] Verify: Error rate <1%
- [ ] Verify: Throughput >500 req/s with OPA enabled

#### 61.3 OPA Documentation
- [ ] Document: Compare baseline (JWT-only) vs OPA-enabled latency
- [ ] Document: Cache hit rate under realistic workload
- [ ] Document: OPA saturation point

---

## MILESTONE 7: Horizontal Scaling

### Goals
- Verify multi-instance deployment
- Test shared cache scenarios
- Document scaling recommendations

---

### PHASE 62: Cache Size Scaling

**Objective**: Test behavior at different cache sizes

#### 62.1 Cache Size Tests
- [ ] Test: 1GB cache size, measure hit rate + eviction time
- [ ] Test: 10GB cache size, measure hit rate + eviction time
- [ ] Test: 50GB cache size, measure hit rate + eviction time
- [ ] Verify: Eviction performance doesn't degrade with size
- [ ] Verify: Memory usage matches configuration
- [ ] Measure: Index lookup time at different cache sizes

#### 62.2 Cache Efficiency
- [ ] Measure: Bytes per cached entry overhead (metadata)
- [ ] Measure: Memory fragmentation over time
- [ ] Verify: No memory leaks at large cache sizes
- [ ] Document: Recommended max cache size for different memory configs

---

### PHASE 63: Multi-Instance Testing

**Objective**: Test horizontal scaling with shared Redis

#### 63.1 Shared Cache Tests
- [ ] Test: 2 proxy instances + shared Redis cache
- [ ] Test: 5 proxy instances + shared Redis cache
- [ ] Test: 10 proxy instances + shared Redis cache
- [ ] Verify: Cache sharing works correctly
- [ ] Verify: No cache inconsistencies
- [ ] Verify: Combined throughput scales linearly
- [ ] Measure: Redis becomes bottleneck at N instances

#### 63.2 Load Balancer Integration
- [ ] Test: Round-robin load balancing
- [ ] Test: Least-connections load balancing
- [ ] Verify: Sticky sessions not required (stateless proxy)
- [ ] Verify: Health check endpoints work correctly

#### 63.3 Cache Consistency
- [ ] Verify: All instances see same cached data (via Redis)
- [ ] Verify: Cache invalidation propagates to all instances
- [ ] Measure: Invalidation propagation latency
- [ ] Test: Split-brain scenario recovery

---

### PHASE 64: Kubernetes Deployment

**Objective**: Production Kubernetes deployment testing

#### 64.1 K8s Scaling Tests
- [ ] Test: HPA scales based on CPU
- [ ] Test: HPA scales based on request rate
- [ ] Test: Pod startup time <30s
- [ ] Test: Graceful pod termination
- [ ] Verify: No request loss during scaling

#### 64.2 K8s Resilience
- [ ] Test: Pod crash and restart
- [ ] Test: Node failure (if test cluster allows)
- [ ] Test: Rolling update with zero downtime
- [ ] Verify: PDB (PodDisruptionBudget) works

---

## MILESTONE 8: Advanced Features

### Goals
- Admin API enhancements
- Audit logging
- Advanced I/O optimizations

---

### PHASE 65: Cache Admin Enhancements

**Objective**: Enhanced cache management APIs

#### 65.1 Admin JWT Authentication (Optional)
- [ ] Test: Requires admin claim in JWT
- [ ] Test: Returns 403 without admin claim
- [ ] Impl: Admin role verification

#### 65.2 Enhanced Cache Stats
- [ ] Test: Stats include per-bucket breakdown
- [ ] Test: Metrics include layer label (memory, disk, redis)
- [ ] Test: Metrics include bucket label
- [ ] Impl: Enhanced metrics collection

#### 65.3 Cache Write-Through Improvements
- [ ] Test: set() writes to memory synchronously
- [ ] Test: Writes to disk/redis asynchronously
- [ ] Test: Async writes queued in background
- [ ] Test: Background write failures logged

---

### PHASE 66: Audit Logging

**Objective**: Add comprehensive audit logging

#### 66.1 Audit Log Middleware
- [ ] Test: Create audit log middleware for Pingora
- [ ] Test: Middleware runs on every request
- [ ] Test: Logs request start
- [ ] Test: Logs request completion
- [ ] Test: Logs request failure/error
- [ ] Test: Logs errors with request context

#### 66.2 Audit Log Fields
- [ ] Impl: Timestamp (ISO 8601)
- [ ] Impl: Request ID (trace ID)
- [ ] Impl: Client IP
- [ ] Impl: User ID (from JWT)
- [ ] Impl: HTTP method, path, status
- [ ] Impl: Response time
- [ ] Impl: Bytes transferred

---

### PHASE 67: Advanced Optimizations (Conditional)

**Objective**: Implement if benchmarks prove value

#### 67.1 Dedicated I/O Thread (if spawn_blocking insufficient)
- [ ] Impl: Create dedicated thread with IoUring instance
- [ ] Impl: Channel-based request/response
- [ ] Test: No file descriptor leaks under load
- [ ] Test: Proper cleanup on errors
- [ ] Bench: Compare vs spawn_blocking

#### 67.2 Buffer Pool Management
- [ ] Impl: Buffer pools for zero-copy patterns
- [ ] Impl: Pre-allocated chunk buffers
- [ ] Test: No allocation per request
- [ ] Bench: Compare with/without buffer pool

**Note**: Only implement if Phase 42 benchmarks show these are bottlenecks.

---

## MILESTONE 9: Documentation & Polish

### PHASE 68: Final Documentation

**Objective**: Complete documentation for v1.2.0

#### 68.1 Performance Documentation
- [ ] Doc: Benchmark results summary
- [ ] Doc: Scaling recommendations
- [ ] Doc: Tuning guide (cache sizes, thread counts)
- [ ] Doc: Resource requirements per RPS

#### 68.2 Operations Documentation
- [ ] Doc: 24-hour endurance test results
- [ ] Doc: Failure recovery procedures
- [ ] Doc: Monitoring recommendations
- [ ] Doc: Alert thresholds

#### 68.3 Feature Documentation
- [ ] Doc: RS256/ES256/JWKS configuration
- [ ] Doc: OPA integration guide updates
- [ ] Doc: Multi-instance deployment guide
- [ ] Doc: Kubernetes best practices

---

## Test Infrastructure Requirements

### For v1.2.0 Testing

| Requirement | Purpose |
|-------------|---------|
| 5GB, 10GB test files in MinIO | Extreme streaming tests |
| 24-hour k6 test capability | Endurance testing |
| Multi-core test environment | Scaling tests |
| Multi-node Kubernetes | Horizontal scaling |
| Redis Sentinel (optional) | HA Redis testing |
| JWKS endpoint (mock) | JWT testing |

### New k6 Scripts Needed

- `k6/24-hour-memory.js` - 24-hour memory cache endurance
- `k6/24-hour-disk.js` - 24-hour disk cache endurance
- `k6/24-hour-redis.js` - 24-hour Redis cache endurance
- `k6/extreme-streaming.js` - 5GB/10GB file tests
- `k6/mixed-workload.js` - Cache + streaming mix
- `k6/cpu-scaling.js` - CPU core scaling tests

---

## Timeline Estimate

| Milestone | Estimated Duration |
|-----------|-------------------|
| M1: Performance Benchmarks | 2 weeks |
| M2: Extended JWT | 1 week |
| M3: OpenFGA Integration | 1 week |
| M4: Endurance Testing | 2 weeks (includes 24hr tests) |
| M5: Extreme Scale | 1 week |
| M6: Production Resilience | 1 week |
| M7: Horizontal Scaling | 1 week |
| M8: Advanced Features | 1 week |
| M9: Documentation | 1 week |

**Total**: ~11 weeks

---

## Success Metrics for v1.2.0

| Metric | Target |
|--------|--------|
| Criterion benchmarks | All core operations baselined |
| 24-hour endurance | Memory stable, no degradation |
| JWT algorithms | RS256, ES256, JWKS working |
| OpenFGA integration | Per-object authorization working |
| Extreme files | 10GB streaming, memory <100MB |
| Multi-instance | 10 instances + Redis works |
| Documentation | 100% coverage of new features |

---

## Notes

- Items marked "DEFERRED" from v1.1.0 are now planned for v1.2.0
- Benchmarks should run on consistent hardware for reproducibility
- Endurance tests may be run in stages (4h, 12h, 24h)
- Advanced optimizations (Phase 67) conditional on benchmark results

---

**Version**: 1.2.0
**Created**: 2024-11-30
**Status**: Planning
