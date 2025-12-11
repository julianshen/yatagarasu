---
title: High Availability
layout: default
parent: Deployment
nav_order: 3
---

# High Availability Deployment

Configure Yatagarasu for maximum availability and resilience.
{: .fs-6 .fw-300 }

---

## HA Architecture

```
                    Global Load Balancer
                          |
            +-------------+-------------+
            |                           |
      Region A                    Region B
            |                           |
   +--------+--------+         +--------+--------+
   |                 |         |                 |
Yatagarasu (x3)   Valkey    Yatagarasu (x3)   Valkey
   |                 |         |                 |
   +--------+--------+         +--------+--------+
            |                           |
      +-----+-----+               +-----+-----+
      |           |               |           |
   S3 (A)      S3 (B)          S3 (A)      S3 (B)
  Primary     Replica         Replica     Primary
```

---

## HA Components

### 1. Multiple Yatagarasu Instances

Run 3+ replicas for redundancy:

```yaml
# Kubernetes
apiVersion: apps/v1
kind: Deployment
metadata:
  name: yatagarasu
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
```

### 2. S3 Replica Failover

Configure multiple S3 backends:

```yaml
buckets:
  - name: "ha-assets"
    path_prefix: "/assets"
    s3:
      bucket: "assets-primary"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"

      replicas:
        - name: "primary"
          region: "us-west-2"
          priority: 1
          timeout_seconds: 5

        - name: "backup-east"
          region: "us-east-1"
          priority: 2
          timeout_seconds: 8

        - name: "backup-eu"
          region: "eu-west-1"
          priority: 3
          timeout_seconds: 15

      circuit_breaker:
        failure_threshold: 5
        success_threshold: 2
        timeout_seconds: 60

    auth:
      enabled: false
```

### 3. Distributed Cache (Valkey/Redis)

Share cache across instances:

```yaml
cache:
  memory:
    max_capacity: 268435456  # 256MB per instance
    ttl_seconds: 1800

  redis:
    enabled: true
    url: "redis://valkey:6379"
    max_capacity: 2147483648  # 2GB shared
    ttl_seconds: 7200
    pool_size: 20
```

### 4. Load Balancer

Distribute traffic across instances:

```nginx
# NGINX upstream configuration
upstream yatagarasu {
    least_conn;
    server yatagarasu-1:8080 weight=1;
    server yatagarasu-2:8080 weight=1;
    server yatagarasu-3:8080 weight=1;

    keepalive 32;
}

server {
    listen 80;

    location / {
        proxy_pass http://yatagarasu;
        proxy_http_version 1.1;
        proxy_set_header Connection "";

        # Health check
        proxy_next_upstream error timeout http_502 http_503;
        proxy_next_upstream_tries 3;
    }
}
```

---

## Kubernetes HA Setup

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: yatagarasu
spec:
  replicas: 3
  selector:
    matchLabels:
      app: yatagarasu
  template:
    metadata:
      labels:
        app: yatagarasu
    spec:
      # Spread across nodes
      topologySpreadConstraints:
        - maxSkew: 1
          topologyKey: kubernetes.io/hostname
          whenUnsatisfiable: DoNotSchedule
          labelSelector:
            matchLabels:
              app: yatagarasu

      # Prefer different zones
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: yatagarasu
                topologyKey: topology.kubernetes.io/zone

      containers:
        - name: yatagarasu
          image: ghcr.io/julianshen/yatagarasu:1.2.0
          # ... rest of container spec
```

### PodDisruptionBudget

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: yatagarasu
spec:
  minAvailable: 2  # Always keep 2 pods running
  selector:
    matchLabels:
      app: yatagarasu
```

### HorizontalPodAutoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: yatagarasu
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: yatagarasu
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

---

## Docker Compose HA Setup

```yaml
version: "3.8"

services:
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - yatagarasu-1
      - yatagarasu-2
      - yatagarasu-3
    restart: unless-stopped

  yatagarasu-1:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - REDIS_URL=redis://valkey:6379
    depends_on:
      - valkey
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: "1"

  yatagarasu-2:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - REDIS_URL=redis://valkey:6379
    depends_on:
      - valkey
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: "1"

  yatagarasu-3:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - REDIS_URL=redis://valkey:6379
    depends_on:
      - valkey
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: "1"

  valkey:
    image: valkey/valkey:7-alpine
    volumes:
      - valkey-data:/data
    command: valkey-server --maxmemory 512mb --maxmemory-policy allkeys-lru --appendonly yes
    restart: unless-stopped

volumes:
  valkey-data:
```

---

## Multi-Region Setup

### Configuration

```yaml
buckets:
  - name: "global-assets"
    path_prefix: "/assets"
    s3:
      bucket: "assets"
      region: "us-west-2"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"

      replicas:
        # Primary region (fastest)
        - name: "us-west"
          region: "us-west-2"
          bucket: "assets-us-west"
          priority: 1
          timeout_seconds: 5

        # Secondary region
        - name: "us-east"
          region: "us-east-1"
          bucket: "assets-us-east"
          priority: 2
          timeout_seconds: 8

        # Europe fallback
        - name: "eu-west"
          region: "eu-west-1"
          bucket: "assets-eu-west"
          priority: 3
          timeout_seconds: 15

        # Different provider (R2) as DR
        - name: "cloudflare-dr"
          endpoint: "https://xxx.r2.cloudflarestorage.com"
          bucket: "assets-dr"
          access_key: "${R2_ACCESS_KEY}"
          secret_key: "${R2_SECRET_KEY}"
          priority: 4
          timeout_seconds: 20

      circuit_breaker:
        failure_threshold: 3
        success_threshold: 2
        timeout_seconds: 30
```

### S3 Cross-Region Replication

Enable S3 replication for automatic data sync:

```bash
# Enable versioning (required for replication)
aws s3api put-bucket-versioning \
  --bucket assets-us-west \
  --versioning-configuration Status=Enabled

# Create replication rule
aws s3api put-bucket-replication \
  --bucket assets-us-west \
  --replication-configuration file://replication.json
```

---

## Health Checking

### Liveness Probe

Checks if the process is alive:

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  failureThreshold: 3
```

### Readiness Probe

Checks if ready to receive traffic:

```yaml
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  failureThreshold: 3
```

The `/ready` endpoint checks S3 backend connectivity.

---

## Circuit Breaker Configuration

```yaml
circuit_breaker:
  # Open circuit after this many consecutive failures
  failure_threshold: 5

  # Close circuit after this many consecutive successes
  success_threshold: 2

  # Wait this long before trying again
  timeout_seconds: 60

  # Allow this many requests through when half-open
  half_open_requests: 1
```

### States

| State | Behavior |
|:------|:---------|
| Closed | All requests go through |
| Open | Requests fail immediately, try next replica |
| Half-Open | Allow limited requests to test recovery |

---

## Graceful Shutdown

Ensure zero dropped requests during deployments:

```yaml
spec:
  terminationGracePeriodSeconds: 30
  containers:
    - name: yatagarasu
      lifecycle:
        preStop:
          exec:
            command: ["/bin/sh", "-c", "sleep 5"]
```

Yatagarasu handles SIGTERM by:
1. Stopping new connection acceptance
2. Completing in-flight requests
3. Exiting cleanly

---

## Monitoring HA

### Key Metrics

```promql
# Replica health
yatagarasu_replica_health{replica="primary"}

# Circuit breaker state
yatagarasu_circuit_breaker_state{replica="primary"}

# Failover events
rate(yatagarasu_replica_failover_total[5m])

# Request distribution across instances
sum by (instance) (rate(yatagarasu_requests_total[5m]))
```

### Alerting Rules

```yaml
groups:
  - name: yatagarasu-ha
    rules:
      - alert: YatagarasuReplicaDown
        expr: yatagarasu_replica_health == 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "S3 replica is unhealthy"

      - alert: YatagarasuAllReplicasDown
        expr: sum(yatagarasu_replica_health) == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "All S3 replicas are down"

      - alert: YatagarasuHighFailoverRate
        expr: rate(yatagarasu_replica_failover_total[5m]) > 1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High S3 failover rate"
```

---

## Best Practices

1. **Run 3+ replicas** - Survive multiple failures
2. **Spread across zones** - Use topology constraints
3. **Use PodDisruptionBudget** - Prevent accidental downtime
4. **Configure health probes** - Enable automatic recovery
5. **Monitor replica health** - Alert on degraded state
6. **Test failover regularly** - Validate recovery works
7. **Use distributed cache** - Share state across instances

---

## See Also

- [High Availability Tutorial](/yatagarasu/tutorials/high-availability/)
- [Operations Guide](/yatagarasu/operations/)
- [Troubleshooting](/yatagarasu/operations/troubleshooting/)
