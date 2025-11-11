# PRD: High Availability S3 Bucket Replication

**Version**: 1.0
**Status**: 📋 DRAFT - Awaiting Review
**Target Release**: v0.4.0 or v1.0
**Author**: Claude Code
**Date**: 2025-11-09

---

## Executive Summary

Enable Yatagarasu to configure multiple replicated S3 buckets for a single logical endpoint, providing high availability and automatic failover when primary buckets become unavailable.

**Key Benefits**:
- ✅ **Zero-downtime**: Automatic failover to replica buckets when primary fails
- ✅ **Geographic redundancy**: Support multi-region S3 deployments
- ✅ **Disaster recovery**: Continue serving content during S3 outages
- ✅ **Read scaling**: Distribute read load across replicas (optional)

---

## Problem Statement

### Current Limitation

Today, each path prefix maps to a **single S3 bucket**:

```yaml
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-products-bucket"
      region: "us-west-2"
```

**What happens if the S3 bucket fails?**
- ❌ All requests to `/products/*` return 502/503 errors
- ❌ No automatic failover to backup bucket
- ❌ Manual intervention required to switch buckets
- ❌ Downtime until primary bucket recovers

### Target Use Cases

1. **Multi-Region DR**: Primary bucket in `us-west-2`, replica in `eu-west-1`
2. **Cross-Cloud HA**: AWS S3 primary, MinIO/Wasabi secondary
3. **Active-Passive**: Production bucket with read-only replica for DR
4. **Read Scaling** (future): Distribute reads across replicas for higher throughput

---

## Requirements

### Functional Requirements

#### FR1: Multiple Buckets per Endpoint
**Priority**: P0 (Must Have)

Users can configure multiple S3 buckets for a single path prefix:

```yaml
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          endpoint: null  # AWS S3
          access_key: "${AWS_ACCESS_KEY_PRIMARY}"
          secret_key: "${AWS_SECRET_KEY_PRIMARY}"
          priority: 1  # Lower number = higher priority

        - name: "replica-eu"
          bucket: "products-eu-west-1"
          region: "eu-west-1"
          endpoint: null  # AWS S3
          access_key: "${AWS_ACCESS_KEY_EU}"
          secret_key: "${AWS_SECRET_KEY_EU}"
          priority: 2

        - name: "replica-minio"
          bucket: "products-backup"
          region: "us-east-1"
          endpoint: "https://minio.internal.example.com"
          access_key: "${MINIO_ACCESS_KEY}"
          secret_key: "${MINIO_SECRET_KEY}"
          priority: 3
```

**Behavior**:
- Proxy tries buckets in priority order (1 → 2 → 3)
- If priority 1 fails, automatically tries priority 2
- If all replicas fail, return 502/503 to client

#### FR2: Automatic Failover
**Priority**: P0 (Must Have)

When primary bucket fails:
1. Detect failure (connection error, timeout, 5xx error)
2. Immediately try next replica in priority order
3. Return response from first successful replica
4. Log failover event with replica names

**Failover Triggers**:
- ✅ Connection refused / network error
- ✅ DNS resolution failure
- ✅ Request timeout (configurable per replica)
- ✅ HTTP 500, 502, 503, 504 from S3
- ❌ HTTP 403 (auth error) - don't failover, return to client
- ❌ HTTP 404 (not found) - don't failover, return to client

**Example Log**:
```
WARN  Replica 'primary' failed (connection timeout), failing over to 'replica-eu'
      request_id=550e8400-..., bucket=products, error=ConnectionTimeout
```

#### FR3: Health-Aware Routing
**Priority**: P0 (Must Have)

Integrate with existing `/ready` endpoint and circuit breaker:

- Unhealthy replicas are skipped during failover
- `/ready` endpoint shows per-replica health:
  ```json
  {
    "status": "ready",
    "backends": {
      "products": {
        "status": "degraded",
        "replicas": {
          "primary": "unhealthy",
          "replica-eu": "healthy",
          "replica-minio": "healthy"
        }
      }
    }
  }
  ```
- Circuit breaker per replica (not per logical bucket)

#### FR4: Metrics and Observability
**Priority**: P0 (Must Have)

New Prometheus metrics:
```
# Requests per replica
http_requests_total{bucket="products",replica="primary"} 1000
http_requests_total{bucket="products",replica="replica-eu"} 50

# Failover events
bucket_failovers_total{bucket="products",from="primary",to="replica-eu"} 3

# Replica health
replica_health{bucket="products",replica="primary"} 0  # 0=unhealthy, 1=healthy
replica_health{bucket="products",replica="replica-eu"} 1
```

Enhanced logging:
```
INFO  Request served from replica 'primary'
      request_id=..., bucket=products, replica=primary, duration_ms=45

WARN  Failover: primary → replica-eu
      request_id=..., bucket=products, reason=ConnectionTimeout, attempt=1
```

#### FR5: Backward Compatibility
**Priority**: P0 (Must Have)

Existing single-bucket configurations continue to work:

```yaml
# Old format (still valid)
buckets:
  - name: "products"
    path_prefix: "/products"
    s3:
      bucket: "my-bucket"
      region: "us-west-2"
```

Internally treated as single replica with priority 1.

### Non-Functional Requirements

#### NFR1: Performance
- **Failover Latency**: <100ms to detect and failover to next replica
- **No Performance Degradation**: When all replicas healthy, same performance as v0.3.0
- **Retry Budget**: Max 2 failover attempts per request (3 total tries)

#### NFR2: Reliability
- **Data Consistency**: Proxy assumes replicas are eventually consistent (doesn't guarantee read-after-write consistency)
- **Stale Reads**: Proxy may serve slightly stale data from replicas (acceptable for read-heavy workloads)
- **Write Operations**: Not supported in v1 (read-only proxy)

#### NFR3: Configuration Validation
- Validate replica priority is unique within bucket
- Validate at least one replica configured
- Detect misconfigurations at startup (fail fast)

---

## Technical Design

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│  Client Request: GET /products/image.jpg               │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Router: Match /products → "products" bucket           │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  ReplicaSet: products                                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │ Replica 1 (priority=1): products-us-west-2      │  │
│  │ Replica 2 (priority=2): products-eu-west-1      │  │
│  │ Replica 3 (priority=3): products-minio-backup   │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Try Replica 1 (primary)                                │
│  ├─ Healthy? → Request succeeds ✅                      │
│  └─ Failed?  → Failover to Replica 2                   │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  Return Response from First Successful Replica         │
└─────────────────────────────────────────────────────────┘
```

### Data Structures

```rust
// New configuration structure
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3ReplicaSet,  // Changed from S3Config
    pub auth: Option<AuthConfig>,
}

pub struct S3ReplicaSet {
    pub replicas: Vec<S3Replica>,
}

pub struct S3Replica {
    pub name: String,           // e.g., "primary", "replica-eu"
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub priority: u8,           // 1-255, lower = higher priority
    pub timeout: Option<u64>,   // Override default timeout
}

// Runtime state
pub struct ReplicaSetState {
    replicas: Vec<ReplicaState>,  // Sorted by priority
}

pub struct ReplicaState {
    replica: S3Replica,
    client: S3Client,
    circuit_breaker: CircuitBreaker,
    health: AtomicBool,
}
```

### Request Flow (Pseudocode)

```rust
async fn proxy_request(
    session: &mut Session,
    ctx: &mut RequestContext,
    replica_set: &ReplicaSetState,
) -> Result<Response> {
    let mut last_error = None;

    // Try replicas in priority order
    for replica_state in replica_set.replicas.iter() {
        // Skip unhealthy replicas
        if !replica_state.is_healthy() {
            tracing::warn!(
                request_id = %ctx.request_id(),
                bucket = %ctx.bucket(),
                replica = %replica_state.name(),
                "Skipping unhealthy replica"
            );
            continue;
        }

        // Try this replica
        match try_replica(session, ctx, replica_state).await {
            Ok(response) => {
                // Success! Log and return
                tracing::info!(
                    request_id = %ctx.request_id(),
                    bucket = %ctx.bucket(),
                    replica = %replica_state.name(),
                    "Request served from replica"
                );
                return Ok(response);
            }
            Err(e) if is_retriable_error(&e) => {
                // Retriable error - try next replica
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %ctx.bucket(),
                    replica = %replica_state.name(),
                    error = %e,
                    "Replica failed, trying next"
                );
                last_error = Some(e);
                continue;
            }
            Err(e) => {
                // Non-retriable error (404, 403) - return immediately
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %ctx.bucket(),
                    replica = %replica_state.name(),
                    error = %e,
                    "Non-retriable error, returning to client"
                );
                return Err(e);
            }
        }
    }

    // All replicas failed
    Err(last_error.unwrap_or(ProxyError::AllReplicasFailed))
}

fn is_retriable_error(error: &ProxyError) -> bool {
    match error {
        ProxyError::ConnectionError(_) => true,
        ProxyError::Timeout => true,
        ProxyError::S3Error(status) if *status >= 500 => true,
        _ => false,
    }
}
```

### Configuration Migration

**Option 1: Backward-Compatible (Recommended)**

```yaml
# Old format still works
buckets:
  - name: "products"
    s3:
      bucket: "my-bucket"
      region: "us-west-2"

# New format with replicas
buckets:
  - name: "media"
    s3:
      replicas:
        - name: "primary"
          bucket: "media-us"
          region: "us-west-2"
          priority: 1
```

Internally, old format is converted to single-replica format.

**Option 2: Breaking Change (Not Recommended)**

Require all configs to use new `replicas` format. Would break existing deployments.

---

## User Experience

### Configuration Example

```yaml
server:
  address: "0.0.0.0"
  port: 8080

buckets:
  # Single bucket (backward compatible)
  - name: "public-assets"
    path_prefix: "/assets"
    s3:
      bucket: "cdn-assets"
      region: "us-west-2"

  # Multi-region HA setup
  - name: "products"
    path_prefix: "/products"
    s3:
      replicas:
        # Primary: AWS S3 us-west-2
        - name: "primary"
          bucket: "products-us-west-2"
          region: "us-west-2"
          access_key: "${AWS_ACCESS_KEY_US}"
          secret_key: "${AWS_SECRET_KEY_US}"
          priority: 1
          timeout: 10  # 10s timeout

        # Replica: AWS S3 eu-west-1
        - name: "eu-replica"
          bucket: "products-eu-west-1"
          region: "eu-west-1"
          access_key: "${AWS_ACCESS_KEY_EU}"
          secret_key: "${AWS_SECRET_KEY_EU}"
          priority: 2
          timeout: 10

        # Backup: MinIO on-prem
        - name: "onprem-backup"
          bucket: "products-backup"
          region: "us-east-1"
          endpoint: "https://minio.internal.company.com"
          access_key: "${MINIO_ACCESS_KEY}"
          secret_key: "${MINIO_SECRET_KEY}"
          priority: 3
          timeout: 5  # Faster timeout for internal MinIO

jwt:
  enabled: false
```

### Observability

**Metrics Query Examples** (Prometheus/Grafana):

```promql
# Failover rate per bucket
rate(bucket_failovers_total[5m])

# Requests per replica
sum by (replica) (rate(http_requests_total{bucket="products"}[5m]))

# Replica health status
replica_health{bucket="products"}
```

**Log Examples**:

```
# Normal operation
INFO  Request served from replica 'primary'
      request_id=550e8400-..., bucket=products, replica=primary,
      status=200, duration_ms=45

# Failover event
WARN  Replica 'primary' failed, failing over to 'eu-replica'
      request_id=550e8400-..., bucket=products, from=primary, to=eu-replica,
      error=ConnectionTimeout, attempt=2

# All replicas failed
ERROR All replicas failed for bucket 'products'
      request_id=550e8400-..., bucket=products, attempted=3,
      errors=[ConnectionTimeout, ConnectionTimeout, 500InternalError]
```

---

## Testing Strategy

### Unit Tests (50+ tests)

```rust
// src/replica_set/mod.rs tests
#[test]
fn test_replicas_sorted_by_priority() {
    // Verify replicas are sorted 1, 2, 3...
}

#[test]
fn test_failover_on_connection_error() {
    // Mock primary fails, verify tries replica
}

#[test]
fn test_no_failover_on_404() {
    // 404 should return immediately, not try replicas
}

#[test]
fn test_skip_unhealthy_replicas() {
    // If replica is circuit-broken, skip it
}

#[test]
fn test_all_replicas_failed_returns_502() {
    // If all fail, return 502 Bad Gateway
}
```

### Integration Tests (20+ tests)

```rust
// tests/integration/replica_set_test.rs
#[tokio::test]
async fn test_failover_to_replica_when_primary_down() {
    // Start proxy with 2 replicas
    // Stop primary S3 instance
    // Verify request succeeds from replica
}

#[tokio::test]
async fn test_health_endpoint_shows_replica_status() {
    // Verify /ready shows per-replica health
}

#[tokio::test]
async fn test_metrics_track_replica_requests() {
    // Verify metrics differentiate replicas
}
```

### E2E Tests (5+ tests)

```rust
// tests/e2e/ha_scenario_test.rs
#[tokio::test]
#[ignore]
async fn test_sustained_load_with_failover() {
    // K6 load test with simulated failures
}
```

---

## Rollout Plan

### Phase 1: Core Failover (v0.4.0 or v1.0)
- ✅ Configuration parsing for replicas
- ✅ Priority-based failover logic
- ✅ Health checks per replica
- ✅ Metrics per replica
- ✅ Backward compatibility

**Estimated Effort**: 3-5 days (with TDD)

### Phase 2: Advanced Features (v1.1+)
- 🚧 Weighted load balancing across replicas
- 🚧 Sticky sessions (same client → same replica)
- 🚧 Active-active read distribution
- 🚧 Replica lag monitoring

**Estimated Effort**: 5-7 days

---

## Open Questions

1. **Replica Consistency**: How do we handle eventual consistency lag between replicas?
   - **Proposed**: Document assumption that replicas are eventually consistent
   - **Accept**: Clients may see stale data during failover

2. **Failover Budget**: Should we limit failover attempts?
   - **Proposed**: Max 2 failovers (3 total attempts) per request
   - **Alternative**: Try all replicas before failing

3. **Write Operations**: Should we support write failover?
   - **Proposed**: v1.0 is read-only, writes fail immediately
   - **Future**: v1.1 could support write replication

4. **Priority Ties**: What if two replicas have same priority?
   - **Proposed**: Validation error at startup (priorities must be unique)
   - **Alternative**: Use lexicographic order on replica name

5. **Circuit Breaker Scope**: Per-replica or per-bucket?
   - **Proposed**: Per-replica (more granular)
   - **Trade-off**: More memory overhead

---

## Success Metrics

### Launch Criteria (v0.4.0 or v1.0)

- ✅ All unit tests passing (50+ tests)
- ✅ All integration tests passing (20+ tests)
- ✅ Backward compatibility verified (existing configs work)
- ✅ Documentation: User guide + config examples
- ✅ Performance: No degradation when all replicas healthy
- ✅ Failover latency: <100ms p99

### Post-Launch Metrics (30 days)

- 📊 Failover success rate: >99%
- 📊 Zero config migration issues reported
- 📊 Failover latency: <100ms p95
- 📊 No performance regression vs. v0.3.0

---

## Alternatives Considered

### Alternative 1: DNS-Based Failover
**Approach**: Use DNS round-robin or Route53 health checks

**Pros**: No code changes needed
**Cons**:
- DNS cache issues (stale DNS)
- No per-replica metrics
- Less control over failover logic

**Decision**: ❌ Rejected - Insufficient control and observability

### Alternative 2: External Load Balancer
**Approach**: Use HAProxy/NGINX in front of multiple proxies

**Pros**: Battle-tested load balancing
**Cons**:
- Additional operational complexity
- Extra network hop
- Harder to correlate logs/metrics

**Decision**: ❌ Rejected - Adds deployment complexity

### Alternative 3: Application-Level HA (This PRD)
**Approach**: Built-in replica failover in Yatagarasu

**Pros**:
- ✅ Zero external dependencies
- ✅ Full control over failover logic
- ✅ Rich observability (metrics, logs per replica)
- ✅ Simple deployment (single binary)

**Decision**: ✅ **Selected** - Best balance of simplicity and capability

---

## Appendix

### Related Documents
- [Phase 21: Production Hardening](../plan.md#phase-21-production-hardening--resilience)
- [Phase 22: Graceful Shutdown & Observability](../plan.md#phase-22-graceful-shutdown--observability)
- [Circuit Breaker Design](../src/circuit_breaker/mod.rs)

### References
- AWS S3 Cross-Region Replication: https://docs.aws.amazon.com/AmazonS3/latest/userguide/replication.html
- Cloudflare Load Balancing: https://developers.cloudflare.com/load-balancing/
- Netflix Hystrix (Circuit Breaker pattern): https://github.com/Netflix/Hystrix

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-11-09 | Claude Code | Initial PRD draft for review |

---

**Status**: 📋 **AWAITING REVIEW** - Please review and approve before implementation planning.
