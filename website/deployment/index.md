---
title: Deployment
layout: default
nav_order: 5
has_children: true
permalink: /deployment/
---

# Deployment Guide

Deploy Yatagarasu for production use.
{: .fs-6 .fw-300 }

---

## Deployment Options

| Method | Best For |
|:-------|:---------|
| [Docker](/yatagarasu/deployment/docker/) | Single instance, quick deployment |
| [Kubernetes](/yatagarasu/deployment/kubernetes/) | Production, auto-scaling |
| [High Availability](/yatagarasu/deployment/high-availability/) | Multi-region, failover |

---

## Quick Reference

### Minimum Requirements

| Resource | Minimum | Recommended |
|:---------|:--------|:------------|
| CPU | 1 core | 2+ cores |
| Memory | 256MB | 1GB |
| Disk | 100MB | 10GB (with disk cache) |

### Port Requirements

| Port | Purpose |
|:-----|:--------|
| 8080 | HTTP proxy |
| 9090 | Prometheus metrics |

### Network Requirements

- Outbound: S3 endpoints (HTTPS/443)
- Outbound: Redis/Valkey (6379) if using distributed cache
- Outbound: OPA/OpenFGA if using authorization

---

## Environment Variables

| Variable | Description |
|:---------|:------------|
| `AWS_ACCESS_KEY_ID` | S3 access key |
| `AWS_SECRET_ACCESS_KEY` | S3 secret key |
| `JWT_SECRET` | JWT signing secret |
| `REDIS_URL` | Redis connection URL |
| `RUST_LOG` | Log level override |

---

## Configuration File Locations

| Location | Priority |
|:---------|:---------|
| `--config /path/to/config.yaml` | 1 (highest) |
| `/etc/yatagarasu/config.yaml` | 2 |
| `./config.yaml` | 3 (lowest) |

---

## Health Checks

```bash
# Liveness probe - basic health
curl http://localhost:8080/health
# {"status":"ok"}

# Readiness probe - includes backend health
curl http://localhost:8080/ready
# {"status":"ok","backends":[{"name":"primary","healthy":true}]}
```

---

## Graceful Operations

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

# Docker (default stop signal)
docker stop yatagarasu

# Kubernetes (automatic with terminationGracePeriodSeconds)
kubectl delete pod yatagarasu-xxx
```

---

## Deployment Checklist

### Pre-deployment

- [ ] Configuration file validated
- [ ] S3 credentials tested
- [ ] Network connectivity verified
- [ ] Resource limits configured
- [ ] Health check endpoints accessible

### Security

- [ ] TLS termination configured (ingress/load balancer)
- [ ] Credentials stored in secrets
- [ ] Network policies applied
- [ ] Rate limiting enabled
- [ ] Audit logging enabled

### Monitoring

- [ ] Prometheus scraping configured
- [ ] Grafana dashboards imported
- [ ] Alerting rules defined
- [ ] Log aggregation configured

### High Availability

- [ ] Multiple replicas deployed
- [ ] S3 replica failover configured
- [ ] Load balancer configured
- [ ] PodDisruptionBudget created

---

## Architecture Patterns

### Single Instance

```
                 +-------------+
Client --------> | Yatagarasu  | --------> S3
                 +-------------+
                       |
                       v
                 +-------------+
                 |    Redis    |  (optional)
                 +-------------+
```

### Multi-Instance with Load Balancer

```
                 +-------------+
            +--> | Yatagarasu  | --+
            |    +-------------+   |
Client ---> LB                     +---> S3
            |    +-------------+   |
            +--> | Yatagarasu  | --+
                 +-------------+
                       |
                       v
                 +-------------+
                 |    Redis    |
                 +-------------+
```

### Multi-Region HA

```
              Region A                    Region B
         +-------------+             +-------------+
Client ->| Yatagarasu  |------------>| Yatagarasu  |
  |      +-------------+             +-------------+
  |            |                           |
  |            v                           v
  |      +----------+                +----------+
  |      |  S3 (A)  |                |  S3 (B)  |
  |      +----------+                +----------+
  |
  +---> Global Load Balancer (Route53, Cloudflare, etc.)
```

---

## Next Steps

- [Docker Deployment](/yatagarasu/deployment/docker/)
- [Kubernetes Deployment](/yatagarasu/deployment/kubernetes/)
- [High Availability Setup](/yatagarasu/deployment/high-availability/)
