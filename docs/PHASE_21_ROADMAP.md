# Phase 21: Production Hardening & Resilience - Implementation Roadmap

## Overview

Phase 21 transforms Yatagarasu from a functional S3 proxy into a production-grade service capable of handling real-world operational challenges: network failures, resource constraints, malicious traffic, and sustained high load.

**Total Scope:** 41+ tests across 8 categories
**Estimated Effort:** 15-25 hours
**Priority Approach:** Implement critical features first, defer optional features

---

## Priority Classification

### 🔴 **CRITICAL** (Must Have for Production)
Features essential for basic production operation:
- Request timeout handling
- Retry logic with backoff
- Malformed request handling (security)
- Resource exhaustion handling

### 🟡 **HIGH** (Strongly Recommended)
Features that significantly improve reliability:
- Connection pooling
- Circuit breaker pattern
- Memory leak prevention

### 🟢 **MEDIUM** (Nice to Have)
Features that enhance operational quality:
- Advanced rate limiting
- Comprehensive chaos testing

---

## Implementation Categories (Prioritized)

### 1. 🔴 **CRITICAL: Request Timeout Handling** ✅ COMPLETED

**Priority:** CRITICAL - Prevents resource exhaustion from hung requests

**Tests Required:** 5
- [x] Request timeout configurable (default 30s)
- [x] S3 request timeout separate from total timeout
- [x] Slow S3 response returns 502 Bad Gateway (Pingora timeout behavior)
- [x] Timeout cancels S3 request (no resource leak)
- [x] Partial response handling (connection closed mid-stream)

**Implementation Steps:**
1. Add `timeout` field to `ServerConfig` (default 30s)
2. Add `s3_timeout` field to `S3Config` (default 20s)
3. Wrap S3 requests with `tokio::time::timeout`
4. Return 504 Gateway Timeout on timeout
5. Ensure timeout cancels in-progress S3 request

**Dependencies:** None

**Estimated Time:** 2-3 hours

**Config Example:**
```yaml
server:
  address: "127.0.0.1"
  port: 8080
  request_timeout: 30  # seconds

buckets:
  - name: slow-bucket
    s3:
      timeout: 20  # S3-specific timeout
```

---

### 2. 🔴 **CRITICAL: Retry Logic with Exponential Backoff**

**Priority:** CRITICAL - Handles transient S3 failures gracefully

**Tests Required:** 5
- [ ] Transient S3 errors retried automatically (500, 503)
- [ ] Exponential backoff between retries (100ms, 200ms, 400ms)
- [ ] Max retry attempts configurable (default 3)
- [ ] Non-retriable errors fail fast (404, 403, 400)
- [ ] Retry metrics tracked (attempts, success, final failure)

**Implementation Steps:**
1. Add `retry` configuration to `S3Config`
2. Implement retry policy with exponential backoff
3. Classify S3 errors (retriable vs non-retriable)
4. Add retry metrics to `Metrics` struct
5. Wrap S3 requests with retry logic

**Dependencies:** `tokio-retry` or custom implementation

**Estimated Time:** 3-4 hours

**Config Example:**
```yaml
buckets:
  - name: products
    s3:
      retry:
        max_attempts: 3
        initial_backoff_ms: 100
        max_backoff_ms: 1000
```

**Retriable Errors:**
- 500 Internal Server Error
- 503 Service Unavailable
- Network timeouts
- Connection reset

**Non-Retriable Errors:**
- 400 Bad Request
- 403 Forbidden
- 404 Not Found
- 416 Range Not Satisfiable

---

### 3. 🔴 **CRITICAL: Malformed Request Handling (Security)**

**Priority:** CRITICAL - Prevents security vulnerabilities

**Tests Required:** 7+
- [ ] Invalid HTTP returns 400 Bad Request
- [ ] Missing required headers returns 400
- [ ] Request too large returns 413 Payload Too Large
- [ ] Request header too large returns 431
- [ ] Malformed JWT returns 403 Forbidden (not crash)
- [ ] SQL injection in path returns 400 (not processed)
- [ ] Path traversal blocked (../, ..\, etc.)

**Implementation Steps:**
1. Add request size limits to `ServerConfig`
2. Validate all incoming requests before routing
3. Sanitize path parameters (block ../, %2e%2e, etc.)
4. Add comprehensive input validation
5. Handle malformed JWT gracefully

**Dependencies:** None

**Estimated Time:** 3-4 hours

**Security Validations:**
- Path traversal: `../`, `..\`, `%2e%2e%2f`, `%252e%252e%252f`
- Null bytes: `%00`
- SQL injection: `'; DROP TABLE`, `' OR '1'='1`
- XSS: `<script>`, `javascript:`
- Header injection: `\r\n`, `\n`

**Config Example:**
```yaml
server:
  max_request_size: 104857600  # 100MB
  max_header_size: 8192        # 8KB
  max_uri_length: 2048         # 2KB
```

---

### 4. 🔴 **CRITICAL: Resource Exhaustion Handling** ✅ COMPLETED

**Priority:** CRITICAL - Prevents service crashes under load

**Tests Required:** 4
- [x] File descriptor limit reached returns 503
- [x] Memory limit reached returns 503
- [x] Graceful degradation under resource pressure
- [x] Automatic recovery when resources available

**Implementation Steps:**
1. Monitor file descriptor usage
2. Monitor memory usage
3. Return 503 Service Unavailable when near limits
4. Implement graceful degradation (drop non-critical features)
5. Auto-recover when resources available

**Dependencies:** System resource monitoring

**Estimated Time:** 2-3 hours

**Graceful Degradation Strategy:**
1. **80% capacity:** Log warning
2. **90% capacity:** Disable metrics collection
3. **95% capacity:** Return 503 for new requests
4. **< 80% capacity:** Resume normal operation

---

### 5. 🟡 **HIGH: Connection Pooling & Concurrency Limiting** ✅ COMPLETED

**Priority:** HIGH - Improves performance and resource efficiency

**Tests Required:** 6
- [x] Connection pool size configurable per bucket (via Pingora HttpPeer)
- [x] Pool reuses connections efficiently (Pingora handles pooling)
- [x] Connections released after request completes (automatic)
- [x] Max concurrent requests enforced (prevents resource exhaustion)
- [x] Requests rejected when at max concurrency (no queueing - fail fast)
- [x] Requests fail fast if limit reached (503 Service Unavailable)

**Implementation Steps:**
1. Add `connection_pool_size` to `S3Config`
2. Add `max_concurrent_requests` to `ServerConfig`
3. Configure AWS SDK connection pool
4. Implement request queue with limits
5. Add connection pool metrics

**Dependencies:** AWS SDK configuration

**Estimated Time:** 4-5 hours

**Config Example:**
```yaml
server:
  max_concurrent_requests: 1000

buckets:
  - name: products
    s3:
      connection_pool_size: 50
      max_idle_connections: 10
```

---

### 6. 🟡 **HIGH: Circuit Breaker Pattern**

**Priority:** HIGH - Prevents cascading failures

**Tests Required:** 5
- [ ] High S3 error rate opens circuit (fail fast)
- [ ] Circuit breaker timeout (try again after cooldown)
- [ ] Successful request closes circuit
- [ ] Circuit breaker state exposed via metrics
- [ ] Circuit breaker per bucket (isolation)

**Implementation Steps:**
1. Implement circuit breaker state machine (Closed → Open → Half-Open)
2. Add circuit breaker configuration to `S3Config`
3. Track error rate per bucket
4. Open circuit after threshold failures
5. Add circuit breaker metrics

**Dependencies:** `circuit-breaker` crate or custom implementation

**Estimated Time:** 4-5 hours

**Circuit Breaker States:**
- **Closed:** Normal operation, requests pass through
- **Open:** Too many failures, reject requests immediately (503)
- **Half-Open:** After timeout, allow test request
  - Success → Closed
  - Failure → Open

**Config Example:**
```yaml
buckets:
  - name: products
    s3:
      circuit_breaker:
        failure_threshold: 5       # Open after 5 failures
        success_threshold: 2       # Close after 2 successes in half-open
        timeout_seconds: 60        # Try again after 60s
        half_open_max_requests: 3  # Allow 3 test requests
```

---

### 7. 🟡 **HIGH: Memory Leak Prevention**

**Priority:** HIGH - Ensures long-term stability

**Tests Required:** 4
- [ ] 24 hour sustained load (no memory growth)
- [ ] Repeated config reloads (no memory leak)
- [ ] 1 million requests (memory stays constant)
- [ ] Large file uploads/downloads (no buffering leak)

**Implementation Steps:**
1. Run 24-hour load test with constant memory monitoring
2. Test repeated config reloads (1000+ reloads)
3. Run 1M request test with memory profiling
4. Verify streaming doesn't accumulate buffers
5. Use Valgrind (Linux) or Instruments (macOS) for leak detection

**Dependencies:** Load testing tools (`wrk`, `hey`, `k6`)

**Estimated Time:** 6-8 hours (mostly testing time)

**Memory Profiling Commands:**
```bash
# Linux - Valgrind
valgrind --leak-check=full --show-leak-kinds=all ./target/release/yatagarasu

# macOS - Instruments
instruments -t Leaks -D leak_trace.trace ./target/release/yatagarasu

# Continuous monitoring
watch -n 1 'ps aux | grep yatagarasu | grep -v grep | awk "{print \$6}"'
```

---

### 8. 🟢 **MEDIUM: Rate Limiting (Optional)**

**Priority:** MEDIUM - Protects against abuse

**Tests Required:** 5
- [ ] Rate limit per bucket configurable
- [ ] Rate limit per client IP configurable
- [ ] Exceeded rate limit returns 429 Too Many Requests
- [ ] Rate limit window (sliding window vs fixed window)
- [ ] Rate limit metrics exposed

**Implementation Steps:**
1. Add rate limit configuration
2. Implement sliding window algorithm
3. Track requests per bucket and per IP
4. Return 429 when limit exceeded
5. Add rate limit metrics

**Dependencies:** `governor` crate (token bucket algorithm)

**Estimated Time:** 4-5 hours

**Config Example:**
```yaml
server:
  rate_limit:
    enabled: true
    global:
      requests_per_second: 1000
    per_ip:
      requests_per_second: 10

buckets:
  - name: products
    s3:
      rate_limit:
        requests_per_second: 100
```

---

## Implementation Order (Recommended)

### Sprint 1: Critical Safety (6-8 hours)
1. ✅ Request timeout handling (2-3h)
2. ✅ Retry logic with backoff (3-4h)
3. ✅ Malformed request handling (3-4h)

**Deliverable:** Proxy handles failures gracefully and rejects malicious input

### Sprint 2: Resource Management ✅ COMPLETED (6-8 hours)
4. ✅ Resource exhaustion handling (2-3h)
5. ✅ Connection pooling & concurrency limiting (4-5h)
6. ✅ Concurrency limiting metrics (1h)

**Deliverable:** Proxy operates reliably under high load

**Implementation Summary:**
- Resource monitor with auto-detected system limits (FD, memory)
- Graceful degradation strategy (warning → critical → exhausted)
- Concurrency limiting via Tokio Semaphore (default: 1000 concurrent requests)
- Connection pooling via Pingora HttpPeer (default: 50 connections per bucket)
- Prometheus metrics for monitoring concurrency rejections
- All 507 tests passing

### Sprint 3: Resilience Patterns (4-5 hours)
6. ✅ Circuit breaker pattern (4-5h)

**Deliverable:** Proxy prevents cascading failures

### Sprint 4: Long-Term Stability (6-8 hours)
7. ✅ Memory leak prevention testing (6-8h)

**Deliverable:** Proxy verified for production deployment

### Sprint 5: Optional Enhancements (4-5 hours)
8. ⚪ Rate limiting (4-5h) - if needed

**Deliverable:** Abuse protection

---

## Testing Strategy

### Unit Tests
- Fast, isolated tests for each component
- Test error conditions and edge cases
- Mock S3 responses for retry/timeout tests

### Integration Tests
- Use LocalStack or MinIO for S3 backend
- Test with real network conditions
- Simulate failures (slow responses, errors)

### Load Tests
- Use `wrk`, `hey`, or `k6` for sustained load
- Monitor metrics during test
- Verify no resource leaks

### Chaos Tests
- Introduce random failures
- Kill S3 backend mid-request
- Disconnect network randomly
- Fill disk space
- Exhaust file descriptors

---

## Metrics to Add

### Connection Pool Metrics ✅ COMPLETED
- ✅ `concurrency_limit_rejections_total` - Requests rejected due to concurrency limit (503)
- ⚪ `connection_pool_size{bucket}` - Pool size (Pingora internal)
- ⚪ `connection_pool_active{bucket}` - Active connections (Pingora internal)
- ⚪ `connection_pool_idle{bucket}` - Idle connections (Pingora internal)
- ⚪ `connection_pool_wait_time_ms{bucket}` - Wait time for connection (future enhancement)

### Retry Metrics
- `s3_retry_attempts_total{bucket}` - Total retry attempts
- `s3_retry_success_total{bucket}` - Successful retries
- `s3_retry_exhausted_total{bucket}` - Retries exhausted

### Circuit Breaker Metrics
- `circuit_breaker_state{bucket}` - Current state (0=closed, 1=open, 2=half-open)
- `circuit_breaker_opened_total{bucket}` - Times circuit opened
- `circuit_breaker_failures{bucket}` - Consecutive failures

### Rate Limit Metrics
- `rate_limit_exceeded_total{bucket}` - Rate limit hits
- `rate_limit_current_rate{bucket}` - Current request rate

### Resource Metrics
- `file_descriptors_used` - Open file descriptors
- `memory_allocated_bytes` - Allocated memory
- `request_queue_depth` - Pending requests

---

## Configuration Schema Updates

Add to `config/mod.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    #[serde(default = "default_timeout")]
    pub request_timeout: u64,  // seconds
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_requests: usize,
    #[serde(default)]
    pub limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimits {
    #[serde(default = "default_max_request_size")]
    pub max_request_size: usize,  // bytes
    #[serde(default = "default_max_header_size")]
    pub max_header_size: usize,   // bytes
    #[serde(default = "default_max_uri_length")]
    pub max_uri_length: usize,    // bytes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    // ... existing fields ...
    #[serde(default = "default_s3_timeout")]
    pub timeout: u64,  // seconds
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default)]
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    #[serde(default = "default_pool_size")]
    pub size: usize,
    #[serde(default = "default_max_idle")]
    pub max_idle: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_seconds: u64,
    pub half_open_max_requests: u32,
}
```

---

## Success Criteria

### Phase 21 Complete When:
✅ All critical tests passing (timeout, retry, security, resources)
✅ High-priority tests passing (connection pool, circuit breaker)
✅ 24-hour load test completed without memory growth
✅ 1M request test completed successfully
✅ All new metrics exposed via `/metrics`
✅ Configuration documentation updated
✅ Production deployment guide created

---

## Next Steps

1. **Immediate:** Start with Sprint 1 (Request Timeout Handling)
2. **Week 1:** Complete Sprints 1-2 (Critical + Resource Management)
3. **Week 2:** Complete Sprints 3-4 (Resilience + Stability)
4. **Optional:** Sprint 5 if rate limiting needed

**Ready to begin Sprint 1: Request Timeout Handling**
