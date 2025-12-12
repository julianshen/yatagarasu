---
title: Metrics Configuration
layout: default
parent: Configuration
nav_order: 6
---

# Metrics Configuration

Configure Prometheus metrics export.
{: .fs-6 .fw-300 }

---

## Basic Configuration

```yaml
metrics:
  enabled: true
  port: 9090
```

---

## Options

| Option | Type | Default | Description |
|:-------|:-----|:--------|:------------|
| `enabled` | boolean | false | Enable metrics endpoint |
| `port` | integer | 9090 | Port for metrics server |

---

## Accessing Metrics

```bash
curl http://localhost:9090/metrics
```

---

## Available Metrics

### Request Metrics

```prometheus
# Total requests by bucket and status
yatagarasu_requests_total{bucket="my-bucket",status="200"}

# Request duration histogram
yatagarasu_request_duration_seconds{bucket="my-bucket",quantile="0.5"}
yatagarasu_request_duration_seconds{bucket="my-bucket",quantile="0.9"}
yatagarasu_request_duration_seconds{bucket="my-bucket",quantile="0.99"}

# Bytes transferred
yatagarasu_bytes_sent_total{bucket="my-bucket"}
yatagarasu_bytes_received_total{bucket="my-bucket"}

# Active connections
yatagarasu_active_connections
```

### S3 Backend Metrics

```prometheus
# S3 requests by replica and status
yatagarasu_s3_requests_total{bucket="my-bucket",replica="primary",status="200"}

# S3 request duration
yatagarasu_s3_request_duration_seconds{bucket="my-bucket",replica="primary"}

# Replica health (1=healthy, 0=unhealthy)
yatagarasu_replica_health{bucket="my-bucket",replica="primary"}
```

### Cache Metrics

```prometheus
# Cache hits and misses
yatagarasu_cache_hits_total{tier="memory"}
yatagarasu_cache_misses_total{tier="memory"}
yatagarasu_cache_hits_total{tier="redis"}
yatagarasu_cache_misses_total{tier="redis"}

# Cache size
yatagarasu_cache_size_bytes{tier="memory"}
yatagarasu_cache_entries{tier="memory"}

# Cache operation duration
yatagarasu_cache_get_duration_seconds{tier="memory"}
yatagarasu_cache_set_duration_seconds{tier="memory"}
```

### Authentication Metrics

```prometheus
# JWT validation
yatagarasu_jwt_validations_total{result="success"}
yatagarasu_jwt_validations_total{result="failure",reason="expired"}
yatagarasu_jwt_validations_total{result="failure",reason="invalid_signature"}

# JWT validation duration
yatagarasu_jwt_validation_duration_seconds
```

### Authorization Metrics

```prometheus
# Authorization decisions
yatagarasu_authorization_checks_total{type="opa",result="allow"}
yatagarasu_authorization_checks_total{type="opa",result="deny"}

# Authorization duration
yatagarasu_authorization_duration_seconds{type="opa"}

# Authorization cache
yatagarasu_authorization_cache_hits_total
yatagarasu_authorization_cache_misses_total
```

### Circuit Breaker Metrics

```prometheus
# Circuit breaker state (0=closed, 1=open, 2=half-open)
yatagarasu_circuit_breaker_state{replica="primary"}

# Circuit breaker transitions
yatagarasu_circuit_breaker_transitions_total{replica="primary",from="closed",to="open"}
```

### Rate Limiting Metrics

```prometheus
# Rate limited requests
yatagarasu_rate_limited_requests_total{bucket="my-bucket"}
```

---

## Prometheus Scrape Config

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'yatagarasu'
    static_configs:
      - targets:
          - 'yatagarasu-1:9090'
          - 'yatagarasu-2:9090'
    scrape_interval: 15s
```

---

## Kubernetes ServiceMonitor

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: yatagarasu
  labels:
    release: prometheus
spec:
  selector:
    matchLabels:
      app: yatagarasu
  endpoints:
    - port: metrics
      interval: 15s
      path: /metrics
```

---

## Grafana Dashboards

### Key Panels

1. **Request Rate**: `rate(yatagarasu_requests_total[5m])`
2. **Error Rate**: `rate(yatagarasu_requests_total{status=~"5.."}[5m])`
3. **P95 Latency**: `histogram_quantile(0.95, rate(yatagarasu_request_duration_seconds_bucket[5m]))`
4. **Cache Hit Rate**: `rate(yatagarasu_cache_hits_total[5m]) / (rate(yatagarasu_cache_hits_total[5m]) + rate(yatagarasu_cache_misses_total[5m]))`

### Example Queries

```promql
# Request rate by bucket
sum by (bucket) (rate(yatagarasu_requests_total[5m]))

# Error rate percentage
sum(rate(yatagarasu_requests_total{status=~"5.."}[5m]))
/ sum(rate(yatagarasu_requests_total[5m])) * 100

# Cache hit rate
sum(rate(yatagarasu_cache_hits_total{tier="memory"}[5m]))
/ (sum(rate(yatagarasu_cache_hits_total{tier="memory"}[5m]))
   + sum(rate(yatagarasu_cache_misses_total{tier="memory"}[5m])))

# S3 backend latency P95
histogram_quantile(0.95,
  sum by (le, replica) (rate(yatagarasu_s3_request_duration_seconds_bucket[5m])))
```

---

## See Also

- [Operations Guide](/yatagarasu/operations/)
- [Monitoring Tutorial](/yatagarasu/tutorials/monitoring/)
