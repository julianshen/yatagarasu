# Yatagarasu v1.0 Test Plan
**Created**: 2025-11-15
**Current Status**: ~97% Complete (Phase 19, 21-25 done)
**Goal**: Complete remaining tests to reach production-ready v1.0

---

## Overview

This test plan covers all remaining unmarked tests in plan.md, organized by priority and feasibility.

**Total Unmarked Tests**: ~30
**Blocking v1.0**: 9 tests (Priority 1)
**Optional/Nice-to-Have**: 14 tests (Priority 2)
**Post-v1.0**: 7 tests (Priority 3 - deferred)

---

## Priority 1: Essential for v1.0 Release ⚡ (9 tests)

These tests MUST pass before v1.0 release. They verify core production requirements.

### 1.1 K6 Load Testing (Phase 17 - Performance Verification)

**Objective**: Verify production performance targets with realistic load

**Location**: `plan.md` lines 1167-1170

**Tests**:
- [ ] **Test 1**: Baseline throughput test (>1,000 req/s) with K6
  - **Script**: `k6/throughput.js`
  - **Target**: 1,000+ requests/second (single core)
  - **Duration**: 60 seconds
  - **VUs**: 10 concurrent users
  - **Pass Criteria**: Avg RPS > 1000, P95 latency < 50ms

- [ ] **Test 2**: Concurrent connections test (100 users) with K6
  - **Script**: `k6/concurrent.js`
  - **Target**: Handle 100 concurrent connections
  - **Duration**: 120 seconds
  - **VUs**: 100 concurrent users
  - **Pass Criteria**: 0 failed requests, P95 latency < 100ms

- [ ] **Test 3**: Streaming latency test (TTFB < 100ms) with K6
  - **Script**: `k6/streaming.js`
  - **Target**: TTFB (Time To First Byte) < 100ms
  - **File Size**: 10MB test file
  - **VUs**: 10 concurrent users
  - **Pass Criteria**: P95 TTFB < 100ms, streaming starts immediately

- [ ] **Test 4**: Stability test (1 hour under load) with K6
  - **Script**: `k6/stability.js`
  - **Target**: Run for 1 hour without crashes
  - **Duration**: 3600 seconds (1 hour)
  - **VUs**: 50 concurrent users (constant)
  - **Pass Criteria**: 0 crashes, <0.1% error rate, memory stable

**Implementation**:
1. Create `k6/` directory with test scripts
2. Write K6 JavaScript test files
3. Document how to run: `k6 run k6/throughput.js`
4. Add results to `docs/PERFORMANCE.md`

**Dependencies**:
- K6 installed (`brew install k6` or https://k6.io/docs/get-started/installation/)
- MinIO running (`docker-compose up`)
- Yatagarasu proxy running (`cargo run --release`)
- Test files uploaded to MinIO (1KB, 100KB, 1MB, 10MB)

**Estimated Time**: 4 hours (1h script writing, 2h testing, 1h documentation)

---

### 1.2 Resource Monitoring Tests (Phase 17)

**Objective**: Verify no memory leaks and reasonable resource usage

**Location**: `plan.md` lines 1173-1176

**Tests**:
- [ ] **Test 5**: Memory stays constant during streaming (no memory leaks)
  - **Method**: Monitor RSS during 1-hour streaming test
  - **Tool**: `ps aux | grep yatagarasu` or `htop`
  - **Pass Criteria**: Memory growth < 5MB over 1 hour

- [ ] **Test 6**: Baseline memory usage < 50MB (idle proxy)
  - **Method**: Start proxy, wait 60s, measure RSS
  - **Tool**: `ps -o rss,cmd -p $(pgrep yatagarasu)`
  - **Pass Criteria**: RSS < 50MB after warmup

- [ ] **Test 7**: CPU usage reasonable under load
  - **Method**: Monitor CPU during concurrent test
  - **Tool**: `top` or `htop`
  - **Pass Criteria**: CPU < 80% average during 100 concurrent users

- [ ] **Test 8**: File descriptors no leaks
  - **Method**: Monitor open FDs during stability test
  - **Tool**: `lsof -p $(pgrep yatagarasu) | wc -l`
  - **Pass Criteria**: FD count stable (no continuous growth)

**Implementation**:
- Add monitoring script: `scripts/monitor-resources.sh`
- Run during K6 stability test
- Capture metrics every 10 seconds
- Generate report: `docs/RESOURCE_USAGE.md`

**Estimated Time**: 2 hours (1h script, 1h testing/documentation)

---

### 1.3 Performance Targets Verification (Phase 17)

**Objective**: Verify all performance targets met

**Location**: `plan.md` lines 1125-1135

**Tests**:
- [ ] **Test 9**: Small file (1KB) end-to-end < 10ms (P95)
  - **Method**: K6 custom metric for 1KB file
  - **Pass Criteria**: P95 latency < 10ms
  - **Note**: May need to run separately from load tests

**Implementation**:
- Add to `k6/small-file.js`
- Use K6 Trend metric for P95 calculation
- Document results in `docs/PERFORMANCE.md`

**Estimated Time**: 1 hour

---

## Priority 2: Important but Optional for v1.0 🔵 (14 tests)

These tests improve confidence but are not strictly required for v1.0 release.

### 2.1 Extended Memory Leak Testing (Phase 22 - Chaos)

**Objective**: Verify long-term stability and memory safety

**Location**: `plan.md` lines 1504-1508

**Tests**:
- [ ] Test: 24 hour sustained load (no memory growth)
- [ ] Test: Repeated config reloads (no memory leak)
- [ ] Test: 1 million requests (memory stays constant)
- [ ] Test: Large file downloads (no buffering leak)

**Implementation**:
- **Tool**: Valgrind memcheck (Linux) or Instruments (macOS)
- **Script**: `scripts/long-running-test.sh`
- **Duration**: 24 hours
- **Status**: OPTIONAL - Run during final v1.0 verification

**Estimated Time**: 26 hours (24h run + 2h setup/analysis)

---

### 2.2 Resource Exhaustion Tests (Phase 22 - Chaos)

**Objective**: Verify graceful degradation under resource limits

**Location**: `plan.md` lines 1511-1512

**Tests**:
- [ ] Test: File descriptor limit reached returns 503
- [ ] Test: Memory limit reached returns 503

**Implementation**:
- Use `ulimit -n 100` to limit FDs
- Use `ulimit -v` to limit memory
- Verify proxy returns 503 Service Unavailable (not crash)
- **Status**: OPTIONAL - Requires manual testing

**Estimated Time**: 2 hours

---

### 2.3 Partial Response Handling (Phase 20 - Future)

**Objective**: Handle client disconnections gracefully

**Location**: `plan.md` line 1480

**Tests**:
- [ ] Test: Connection closed mid-stream (partial response)

**Status**: Deferred to v1.1 (marked as TODO in plan.md)

---

### 2.4 Retry Logic Tests (Phase 20 - Blocked)

**Objective**: Verify automatic retry on transient S3 errors

**Location**: `plan.md` lines 1483-1487

**Tests**:
- [ ] Test: Transient S3 errors retried automatically (500, 503)
- [ ] Test: Exponential backoff between retries (100ms, 200ms, 400ms)
- [ ] Test: Max retry attempts configurable (default 3)
- [ ] Test: Non-retriable errors fail fast (404, 403, 400)
- [ ] Test: Retry metrics tracked (attempts, success, final failure)

**Status**: BLOCKED - Awaiting Pingora integration (see `docs/RETRY_INTEGRATION.md`)
**Note**: Retry is built-in to Pingora, needs documentation update

**Estimated Time**: N/A (blocked)

---

### 2.5 Security Review (Phase 8)

**Objective**: Manual security review of entire codebase

**Location**: `plan.md` line 733

**Test**:
- [ ] Security review completed (manual review required)

**Checklist**:
- [ ] Review all authentication code (JWT validation, claims)
- [ ] Review all S3 signature generation (no credential leaks)
- [ ] Review all input validation (path traversal, SQL injection)
- [ ] Review all error messages (no sensitive data leaked)
- [ ] Review all logging (no credentials logged)
- [ ] Review all dependencies (`cargo audit`)
- [ ] Review all unsafe code (none should exist)
- [ ] Review all network code (proper TLS validation)

**Implementation**:
- Manual code review session
- Document findings in `docs/SECURITY_REVIEW.md`
- Address any findings before v1.0

**Estimated Time**: 4 hours

---

## Priority 3: Post-v1.0 (7 tests) 🔮

These tests are explicitly deferred to post-v1.0 releases.

### 3.1 Docker Image Publishing (Phase 24 - Section D)

**Location**: `plan.md` lines 2203-2213

**Status**: Deferred until after v1.0 release (no public publishing needed yet)

**Tests**:
- [ ] `.github/workflows/release.yml` exists
- [ ] Release workflow triggers on git tags `v*.*.*`
- [ ] Builds for amd64 and arm64
- [ ] Tags images with version and `latest`
- [ ] Pushes to ghcr.io
- [ ] Creates GitHub Release
- [ ] Attaches binary artifacts

**Estimated Time**: 8 hours (when needed)

---

## Test Execution Plan

### Phase 1: K6 Load Testing (Priority 1.1 - 1.3)
**Duration**: 1 day
**Steps**:
1. ✅ Install K6: `brew install k6`
2. ✅ Create `k6/` directory
3. ✅ Write 4 K6 test scripts (throughput, concurrent, streaming, stability)
4. ✅ Upload test files to MinIO (1KB, 100KB, 1MB, 10MB)
5. ✅ Run tests: `cargo run --release` + `k6 run k6/throughput.js`
6. ✅ Document results in `docs/PERFORMANCE.md`
7. ✅ Mark tests complete in `plan.md`

### Phase 2: Resource Monitoring (Priority 1.2)
**Duration**: 4 hours
**Steps**:
1. ✅ Write `scripts/monitor-resources.sh`
2. ✅ Run during K6 stability test
3. ✅ Analyze memory/CPU/FD usage
4. ✅ Document in `docs/RESOURCE_USAGE.md`
5. ✅ Mark tests complete in `plan.md`

### Phase 3: Security Review (Priority 2.5)
**Duration**: 4 hours
**Steps**:
1. ✅ Manual code review session
2. ✅ Run `cargo audit`
3. ✅ Document findings in `docs/SECURITY_REVIEW.md`
4. ✅ Address critical findings
5. ✅ Mark complete in `plan.md`

### Phase 4: Final v1.0 Verification
**Duration**: 2 hours
**Steps**:
1. ✅ All Priority 1 tests passing
2. ✅ All quality gates passing (cargo test, clippy, fmt)
3. ✅ Update README.md and plan.md to v1.0 status
4. ✅ Update CHANGELOG.md with v1.0 release notes
5. ✅ Commit: `[RELEASE] Yatagarasu v1.0.0 - Production Ready`
6. ✅ Tag: `git tag -a v1.0.0 -m "Release v1.0.0"`

---

## Success Criteria for v1.0

### Functional Requirements ✅
- [x] Multi-bucket routing with longest prefix matching
- [x] JWT authentication with flexible claims verification
- [x] AWS Signature V4 signing and S3 forwarding
- [x] Configuration hot reload (SIGHUP, /admin/reload API)
- [x] Rate limiting (global, per-IP, per-bucket)
- [x] Circuit breaker with automatic failure detection
- [x] Health endpoints (/health, /ready)
- [x] High Availability with multi-replica failover
- [x] Read-only enforcement (405 for PUT/POST/DELETE/PATCH)
- [x] Docker containerization (41.2MB distroless image)

### Performance Requirements 🔄
- [ ] Baseline throughput > 1,000 req/s ← **Priority 1**
- [ ] Small file latency < 10ms P95 ← **Priority 1**
- [ ] Streaming TTFB < 100ms ← **Priority 1**
- [ ] Handle 100 concurrent connections ← **Priority 1**
- [ ] 1 hour stability (no crashes) ← **Priority 1**

### Quality Requirements ✅
- [x] 98.43% test coverage on library modules
- [x] All unit tests passing (171 tests)
- [x] All integration tests passing
- [x] No clippy warnings (`cargo clippy -- -D warnings`)
- [x] Properly formatted (`cargo fmt --check`)
- [x] No security vulnerabilities (`cargo audit`)

### Documentation Requirements 🔄
- [x] README.md with quick start guide
- [x] docs/CONFIG_RELOAD.md (hot reload)
- [x] docs/GRACEFUL_SHUTDOWN.md
- [x] docs/RETRY_INTEGRATION.md
- [ ] docs/PERFORMANCE.md ← **Priority 1** (create during load testing)
- [ ] docs/SECURITY_REVIEW.md ← **Priority 2** (create during review)

---

## Estimated Total Time to v1.0

**Priority 1 (Blocking)**: ~8 hours
- K6 load testing: 4 hours
- Resource monitoring: 2 hours
- Performance verification: 1 hour
- Documentation: 1 hour

**Priority 2 (Optional)**: ~32 hours
- 24h memory leak test: 26 hours
- Resource exhaustion: 2 hours
- Security review: 4 hours

**Minimum Path to v1.0**: 8 hours (Priority 1 only)
**Recommended Path to v1.0**: 12 hours (Priority 1 + Security Review)

---

## Next Steps

1. **Immediate**: Start with Priority 1.1 (K6 Load Testing)
   - Create k6/ directory with test scripts
   - Run throughput test first (quickest feedback)

2. **Then**: Priority 1.2 (Resource Monitoring)
   - Run during stability test to collect metrics

3. **Finally**: Priority 2.5 (Security Review)
   - Manual review before v1.0 release

4. **Release**: Tag v1.0.0 when all Priority 1 tests pass

---

## Reference

- **Plan.md**: Full implementation plan with all phases
- **README.md**: Project overview and current status
- **CLAUDE.md**: TDD workflow and development guidelines
- **K6 Docs**: https://k6.io/docs/
- **Performance Targets**: See plan.md Phase 17 lines 1125-1135
