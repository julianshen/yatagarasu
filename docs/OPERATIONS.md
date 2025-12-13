# YATAGARASU v1.3.0 - OPERATIONS GUIDE

**Version**: v1.3.0
**Purpose**: Production operations, monitoring, and incident response

---

## TABLE OF CONTENTS

1. [Endurance Test Results](#1-endurance-test-results)
2. [Monitoring Setup](#2-monitoring-setup)
3. [Alert Thresholds](#3-alert-thresholds)
4. [Failure Recovery Procedures](#4-failure-recovery-procedures)
5. [Runbook: Common Issues](#5-runbook-common-issues)

---

## 1. ENDURANCE TEST RESULTS

### 24-Hour Stability Summary

Yatagarasu v1.3.0 has been validated through extended endurance testing:

| Test | Duration | VUs | Result | Memory |
|------|----------|-----|--------|--------|
| Standard Load | 1 hour | 50 | PASS | Stable at ~60MB |
| Extended Load | 4 hours | 100 | PASS | Stable at ~80MB |
| High Concurrency | 2 hours | 1000 | PASS | P95 <200ms |
| Mixed Workload | 5 minutes | 200 | PASS | 0% errors |
| 5GB Streaming | 60s | 5 | PASS | Memory stable |
| 10GB Streaming | 60s | 3 | PASS | Memory stable |

### Memory Behavior

**Observed Pattern**:
- Initial memory: ~20 MB
- Stabilized memory (under load): 60-80 MB
- Memory growth during warmup: Normal (cache filling)
- Post-stabilization: No continuous growth (no leak)

**Memory Components**:
- Connection buffers: ~64KB per active connection
- Cache layer: Configurable (default 64MB memory cache)
- Internal structures: ~10-20MB baseline

### CPU Utilization

| Load Level | CPU Usage | Notes |
|------------|-----------|-------|
| Idle | <1% | Minimal background activity |
| Light (100 req/s) | 2-5% | Mostly I/O bound |
| Medium (500 req/s) | 5-15% | Efficient processing |
| Heavy (1000+ req/s) | 15-40% | Scales linearly |
| Peak observed | 41% | Brief spikes, not sustained |

---

## 2. MONITORING SETUP

### Prometheus Metrics Endpoint

Yatagarasu exposes metrics at `http://localhost:9090/metrics` (configurable).

### Key Metrics

#### Request Metrics
```
# Total request count
yatagarasu_requests_total

# Request latency histogram (ms)
yatagarasu_request_duration_ms{quantile="0.5|0.95|0.99"}

# Error count by type
yatagarasu_errors_total{type="auth|s3|cache|timeout"}
```

#### Cache Metrics
```
# Cache operations
cache_hits_total
cache_misses_total
cache_evictions_total
cache_purges_total

# Cache state (gauges)
cache_size_bytes
cache_items_count

# Per-layer breakdown
cache_hits_by_layer{layer="memory|disk|redis"}
cache_size_by_layer{layer="memory|disk|redis"}
cache_items_by_layer{layer="memory|disk|redis"}

# Cache latency (microseconds)
cache_get_duration_us{quantile="0.5|0.95|0.99"}
cache_set_duration_us{quantile="0.5|0.95|0.99"}
```

#### S3 Backend Metrics
```
# S3 operation counts
s3_operations_total{operation="get|head|list"}

# S3 latency (milliseconds)
s3_latency_ms{quantile="0.5|0.95|0.99"}

# S3 errors by type
s3_errors_total{type="timeout|auth|notfound|server"}

# Retry statistics
s3_retry_attempts_total
s3_retry_success_total
s3_retry_exhausted_total
```

#### OPA/Authorization Metrics
```
opa_cache_hits_total
opa_cache_misses_total
opa_evaluation_duration_us{quantile="0.5|0.95"}
```

### Grafana Dashboard Queries

**Request Rate (RPS)**:
```promql
rate(yatagarasu_requests_total[1m])
```

**P95 Latency**:
```promql
histogram_quantile(0.95, rate(yatagarasu_request_duration_ms_bucket[5m]))
```

**Cache Hit Rate**:
```promql
rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))
```

**Error Rate**:
```promql
rate(yatagarasu_errors_total[5m]) / rate(yatagarasu_requests_total[5m])
```

**Memory Cache Utilization**:
```promql
cache_size_by_layer{layer="memory"} / (64 * 1024 * 1024)  # Assuming 64MB max
```

---

## 3. ALERT THRESHOLDS

### Critical Alerts (Page Immediately)

| Alert | Condition | Action |
|-------|-----------|--------|
| High Error Rate | `error_rate > 1%` for 5m | Investigate immediately |
| Service Down | No response for 30s | Check process, restart if needed |
| Memory Exhaustion | RSS > 90% of limit | Scale up or reduce cache size |
| S3 Backend Failure | `s3_errors_total` rate > 10/s | Check S3 connectivity |

### Warning Alerts (Investigate Soon)

| Alert | Condition | Action |
|-------|-----------|--------|
| Elevated Latency | `P95 > 100ms` for 5m | Check S3 backend latency |
| Cache Miss Spike | `hit_rate < 50%` for 10m | Review cache config |
| Memory Growth | `memory_growth > 50MB/hour` | Monitor for potential leak |
| Retry Rate High | `retry_rate > 5%` | Check S3 backend health |

### Informational Alerts

| Alert | Condition | Notes |
|-------|-----------|-------|
| High Concurrency | Connections > 500 | Consider scaling |
| Cache Evictions | `eviction_rate > 100/s` | Consider larger cache |
| Configuration Reload | Config file changed | Verify new settings |

### Prometheus Alert Rules

```yaml
groups:
  - name: yatagarasu
    rules:
      - alert: YatagarasuHighErrorRate
        expr: |
          rate(yatagarasu_errors_total[5m])
          / rate(yatagarasu_requests_total[5m]) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate on Yatagarasu"
          description: "Error rate is {{ $value | humanizePercentage }}"

      - alert: YatagarasuHighLatency
        expr: |
          histogram_quantile(0.95,
            rate(yatagarasu_request_duration_ms_bucket[5m])) > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High P95 latency on Yatagarasu"
          description: "P95 latency is {{ $value }}ms"

      - alert: YatagarasuLowCacheHitRate
        expr: |
          rate(cache_hits_total[10m])
          / (rate(cache_hits_total[10m]) + rate(cache_misses_total[10m])) < 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low cache hit rate"
          description: "Cache hit rate is {{ $value | humanizePercentage }}"

      - alert: YatagarasuS3BackendErrors
        expr: rate(s3_errors_total[5m]) > 10
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "S3 backend errors"
          description: "S3 error rate: {{ $value }}/s"
```

---

## 4. FAILURE RECOVERY PROCEDURES

### 4.1 Process Crash Recovery

**Symptoms**: Service unavailable, no process running

**Recovery Steps**:
1. Check logs for crash reason:
   ```bash
   journalctl -u yatagarasu --since "10 minutes ago"
   # or
   docker logs yatagarasu --tail 100
   ```

2. Verify configuration is valid:
   ```bash
   ./yatagarasu --config config.yaml --validate
   ```

3. Restart service:
   ```bash
   systemctl restart yatagarasu
   # or
   docker restart yatagarasu
   ```

4. Verify health:
   ```bash
   curl http://localhost:8080/health
   ```

### 4.2 S3 Backend Unavailable

**Symptoms**: High error rate, S3 timeout errors in logs

**Recovery Steps**:
1. Verify S3 connectivity:
   ```bash
   aws s3 ls s3://your-bucket --endpoint-url https://your-s3-endpoint
   ```

2. Check S3 credentials:
   ```bash
   # Verify env vars are set
   echo $AWS_ACCESS_KEY_ID
   echo $AWS_SECRET_ACCESS_KEY
   ```

3. Check network connectivity:
   ```bash
   curl -v https://your-s3-endpoint/health
   ```

4. If S3 is down, enable maintenance mode (if available) or return cached content only

### 4.3 Memory Exhaustion

**Symptoms**: OOM kills, slow responses, high memory usage

**Recovery Steps**:
1. Check current memory:
   ```bash
   ps aux | grep yatagarasu | awk '{print $6}'  # RSS in KB
   ```

2. Reduce cache size immediately:
   ```yaml
   # In config.yaml
   cache:
     memory:
       max_cache_size_mb: 32  # Reduce from 64
   ```

3. Reload configuration:
   ```bash
   kill -HUP $(pgrep yatagarasu)
   ```

4. Consider horizontal scaling if problem persists

### 4.4 High Latency

**Symptoms**: P95 latency > 100ms, slow responses

**Diagnosis**:
1. Check S3 backend latency:
   ```bash
   curl -w "@curl-format.txt" -o /dev/null -s https://s3-endpoint/bucket/file
   ```

2. Check cache hit rate:
   ```bash
   curl -s localhost:9090/metrics | grep cache_hits
   ```

3. Check CPU/Memory:
   ```bash
   top -p $(pgrep yatagarasu)
   ```

**Recovery**:
- If S3 is slow: Nothing to do at proxy level, contact S3 provider
- If cache hit rate is low: Increase cache size or TTL
- If CPU is high: Scale horizontally

### 4.5 Configuration Reload Failure

**Symptoms**: Config changes not taking effect, reload errors in logs

**Recovery Steps**:
1. Validate new configuration:
   ```bash
   ./yatagarasu --config new-config.yaml --validate
   ```

2. Check for syntax errors:
   ```bash
   yamllint config.yaml
   ```

3. Apply changes with restart if hot reload fails:
   ```bash
   systemctl restart yatagarasu
   ```

---

## 5. RUNBOOK: COMMON ISSUES

### Issue: "Connection Refused" Errors

**Cause**: Service not running or port conflict

**Solution**:
```bash
# Check if process is running
pgrep yatagarasu

# Check port binding
netstat -tlnp | grep 8080

# Check for port conflicts
lsof -i :8080
```

### Issue: High Memory Usage After Startup

**Cause**: Cache warming, normal behavior

**Expected**: Memory grows to configured cache size, then stabilizes

**Action**: Monitor for 30 minutes. If growth continues beyond cache size limit, investigate.

### Issue: Intermittent 503 Errors

**Cause**: S3 backend rate limiting or temporary unavailability

**Solution**:
1. Check S3 error logs
2. Verify retry configuration is enabled
3. Consider adding request queuing

### Issue: JWT Authentication Failures

**Cause**: Token expired, wrong signing key, claims mismatch

**Diagnosis**:
```bash
# Decode token (without verification)
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq .

# Check expiration
echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq '.exp'
```

### Issue: Cache Not Working (All Misses)

**Cause**: Cache layer misconfiguration, TTL too short

**Diagnosis**:
```bash
# Check cache metrics
curl -s localhost:9090/metrics | grep -E "cache_(hits|misses|size)"

# Verify cache config
grep -A 10 "cache:" config.yaml
```

### Issue: Slow First Request After Restart

**Cause**: Cold cache, connection pool warming

**Expected**: First few requests may be slower (100-500ms)

**Action**: Implement cache pre-warming if critical:
```bash
# Pre-warm common files
for file in popular-file1.jpg popular-file2.css; do
  curl -s http://localhost:8080/bucket/$file > /dev/null
done
```

---

## APPENDIX: HEALTH CHECK ENDPOINTS

| Endpoint | Purpose | Expected Response |
|----------|---------|-------------------|
| `/health` | Liveness probe | `200 OK` |
| `/ready` | Readiness probe | `200 OK` when ready |
| `/metrics` | Prometheus metrics | Metric text |

### Kubernetes Probes

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

---

## APPENDIX: LOG ANALYSIS

### Important Log Patterns

```bash
# Errors
grep -E "ERROR|WARN" /var/log/yatagarasu/*.log

# S3 issues
grep "s3" /var/log/yatagarasu/*.log | grep -i error

# Authentication failures
grep "auth" /var/log/yatagarasu/*.log | grep -i fail

# Slow requests
grep "duration_ms" /var/log/yatagarasu/*.log | awk -F'duration_ms=' '$2 > 100'
```

### Log Levels

| Level | Use Case |
|-------|----------|
| ERROR | Failures requiring attention |
| WARN | Recoverable issues |
| INFO | Normal operations |
| DEBUG | Troubleshooting (verbose) |
| TRACE | Development only |

---

*Generated: December 2025*
*Yatagarasu v1.3.0 Operations Guide*
