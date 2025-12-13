---
title: Operations
layout: default
nav_order: 6
has_children: true
permalink: /operations/
---

# Operations Guide

Run Yatagarasu reliably in production.
{: .fs-6 .fw-300 }

---

## Quick Links

| Guide | Description |
|:------|:------------|
| [Monitoring](/yatagarasu/operations/monitoring/) | Prometheus metrics and Grafana dashboards |
| [Troubleshooting](/yatagarasu/operations/troubleshooting/) | Common issues and solutions |
| [Performance](/yatagarasu/operations/performance/) | Tuning and optimization |

---

## Runtime Operations

### Health Checks

```bash
# Liveness check
curl http://localhost:8080/health
# {"status":"ok"}

# Readiness check (includes backend health)
curl http://localhost:8080/ready
# {"status":"ok","backends":[{"name":"primary","healthy":true}]}
```

### Hot Reload

Reload configuration without downtime:

```bash
# Linux/macOS
kill -HUP $(pgrep yatagarasu)

# Docker
docker kill --signal=HUP yatagarasu

# Kubernetes
kubectl exec deployment/yatagarasu -- kill -HUP 1
```

### Graceful Shutdown

```bash
# SIGTERM - complete in-flight requests
kill -TERM $(pgrep yatagarasu)

# Docker
docker stop yatagarasu

# Kubernetes (automatic with terminationGracePeriodSeconds)
```

---

## Monitoring

### Prometheus Metrics

```bash
curl http://localhost:9090/metrics
```

Key metrics:

| Metric | Description |
|:-------|:------------|
| `yatagarasu_requests_total` | Total requests by bucket/status |
| `yatagarasu_request_duration_seconds` | Request latency histogram |
| `yatagarasu_cache_hits_total` | Cache hits by tier |
| `yatagarasu_replica_health` | S3 replica health (1=up) |
| `yatagarasu_circuit_breaker_state` | Circuit breaker state |

### Essential Alerts

```yaml
groups:
  - name: yatagarasu
    rules:
      # High error rate
      - alert: YatagarasuHighErrorRate
        expr: |
          sum(rate(yatagarasu_requests_total{status=~"5.."}[5m]))
          / sum(rate(yatagarasu_requests_total[5m])) > 0.01
        for: 5m
        labels:
          severity: critical

      # High latency
      - alert: YatagarasuHighLatency
        expr: |
          histogram_quantile(0.95, rate(yatagarasu_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning

      # Low cache hit rate
      - alert: YatagarasuLowCacheHitRate
        expr: |
          sum(rate(yatagarasu_cache_hits_total[5m]))
          / (sum(rate(yatagarasu_cache_hits_total[5m]))
             + sum(rate(yatagarasu_cache_misses_total[5m]))) < 0.5
        for: 15m
        labels:
          severity: warning

      # S3 replica down
      - alert: YatagarasuReplicaDown
        expr: yatagarasu_replica_health == 0
        for: 5m
        labels:
          severity: warning
```

---

## Logging

### Log Levels

| Level | Use Case |
|:------|:---------|
| `error` | Production (minimal) |
| `warn` | Production (recommended) |
| `info` | Production (verbose) |
| `debug` | Troubleshooting |
| `trace` | Deep debugging |

### Change Log Level

```bash
# Environment variable
RUST_LOG=debug yatagarasu --config config.yaml

# Configuration file
logging:
  level: "debug"
```

### Log Analysis

```bash
# Count errors
docker logs yatagarasu 2>&1 | grep -c '"level":"error"'

# Find slow requests (>1s)
docker logs yatagarasu 2>&1 | jq 'select(.duration_ms > 1000)'

# Track specific request
docker logs yatagarasu 2>&1 | jq 'select(.request_id == "abc123")'
```

---

## Maintenance Tasks

### Clear Cache

Currently requires restart. API endpoint planned for v1.3.

```bash
# Docker
docker restart yatagarasu

# Kubernetes
kubectl rollout restart deployment/yatagarasu
```

### Rotate Secrets

1. Update secret in environment/secret store
2. Trigger hot reload

```bash
# Update Kubernetes secret
kubectl create secret generic s3-credentials \
  --from-literal=access-key=NEW_KEY \
  --from-literal=secret-key=NEW_SECRET \
  --dry-run=client -o yaml | kubectl apply -f -

# Reload configuration
kubectl exec deployment/yatagarasu -- kill -HUP 1
```

### Update Configuration

1. Modify configuration file
2. Send SIGHUP for hot reload

```bash
# Edit config
vim config.yaml

# Hot reload
kill -HUP $(pgrep yatagarasu)
```

---

## Resource Management

### Memory Sizing

| Component | Typical Usage |
|:----------|:--------------|
| Base | 50-100MB |
| Per connection | ~64KB |
| Memory cache | Configurable |
| Redis connections | ~1MB |

Example: 1000 connections + 512MB cache = ~600MB

### CPU Sizing

| Workload | Threads |
|:---------|:--------|
| Light (<1000 RPS) | 2 |
| Medium (1000-5000 RPS) | 4 |
| Heavy (>5000 RPS) | 8+ |

### File Descriptors

For high concurrency:

```bash
# Check current limit
ulimit -n

# Increase limit
ulimit -n 65535
```

Or in systemd:

```ini
[Service]
LimitNOFILE=65535
```

---

## Backup and Recovery

### Configuration Backup

```bash
# Backup config
cp config.yaml config.yaml.backup

# Version control
git add config.yaml
git commit -m "Update configuration"
```

### Cache Recovery

- Memory cache: Automatically rebuilt on startup
- Disk cache: Persists across restarts
- Redis cache: Depends on Redis persistence settings

---

## Security Operations

### Audit Logs

Enable detailed request logging:

```yaml
logging:
  level: "info"
  request_logging:
    enabled: true
    include_headers: false
    include_response_time: true
```

### Credential Rotation

1. Create new credentials in S3/auth provider
2. Update secrets
3. Hot reload configuration
4. Verify with test request
5. Revoke old credentials

### Security Scanning

```bash
# Scan container image
trivy image ghcr.io/julianshen/yatagarasu:latest

# Check for vulnerabilities
grype ghcr.io/julianshen/yatagarasu:latest
```

---

## Runbooks

### High Error Rate

1. Check error logs: `docker logs yatagarasu | grep error`
2. Verify S3 connectivity: `curl https://s3.amazonaws.com`
3. Check replica health: `curl localhost:9090/metrics | grep replica_health`
4. Review circuit breaker state
5. Restart if needed: `docker restart yatagarasu`

### High Latency

1. Check cache hit rate in metrics
2. Verify S3 backend latency
3. Check for resource exhaustion (CPU, memory)
4. Review slow query logs
5. Consider scaling up instances

### Memory Issues

1. Check current usage: `docker stats yatagarasu`
2. Review cache configuration
3. Look for memory leaks (growing usage over time)
4. Reduce cache size if needed
5. Restart to clear memory

---

## See Also

- [Monitoring Guide](/yatagarasu/operations/monitoring/)
- [Troubleshooting](/yatagarasu/operations/troubleshooting/)
- [Performance Guide](/yatagarasu/operations/performance/)
