# YATAGARASU - MULTI-INSTANCE DEPLOYMENT GUIDE

**Version**: v1.2.0
**Purpose**: Horizontal scaling and production deployment patterns

---

## TABLE OF CONTENTS

1. [Overview](#overview)
2. [Architecture Patterns](#architecture-patterns)
3. [Load Balancer Configuration](#load-balancer-configuration)
4. [Shared Cache Setup](#shared-cache-setup)
5. [Kubernetes Deployment](#kubernetes-deployment)
6. [Docker Compose Multi-Instance](#docker-compose-multi-instance)
7. [Health Checks and Readiness](#health-checks-and-readiness)
8. [Session Affinity](#session-affinity)

---

## OVERVIEW

Yatagarasu supports horizontal scaling for high-availability and increased throughput. Each instance is stateless (except for local cache), making scaling straightforward.

### When to Scale Horizontally

| Indicator | Threshold | Action |
|-----------|-----------|--------|
| CPU usage | >70% sustained | Add instances |
| Memory | >80% of limit | Add instances or reduce cache |
| Request queue | >100 pending | Add instances |
| P95 latency | >100ms | Add instances |
| RPS target | >5000/instance | Add instances |

### Scaling Considerations

- **Stateless design**: No sticky sessions required for functionality
- **Cache coherence**: Consider Redis for shared cache across instances
- **Configuration**: All instances should use identical config
- **Graceful shutdown**: Ensure load balancer drains before termination

---

## ARCHITECTURE PATTERNS

### Pattern 1: Simple Load Balancing

```
                    ┌─────────────────┐
                    │  Load Balancer  │
                    │  (nginx/HAProxy)│
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────┴────┐         ┌────┴────┐         ┌────┴────┐
    │Yatagarasu│         │Yatagarasu│         │Yatagarasu│
    │Instance 1│         │Instance 2│         │Instance 3│
    │(Memory   │         │(Memory   │         │(Memory   │
    │ Cache)   │         │ Cache)   │         │ Cache)   │
    └────┬────┘         └────┬────┘         └────┬────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                    ┌────────┴────────┐
                    │    S3 Backend   │
                    └─────────────────┘
```

**Pros**: Simple, no shared state
**Cons**: Cache duplication, inconsistent cache across instances

### Pattern 2: Shared Redis Cache

```
                    ┌─────────────────┐
                    │  Load Balancer  │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
    ┌────┴────┐         ┌────┴────┐         ┌────┴────┐
    │Yatagarasu│         │Yatagarasu│         │Yatagarasu│
    │Instance 1│         │Instance 2│         │Instance 3│
    └────┬────┘         └────┬────┘         └────┬────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                    ┌────────┴────────┐
                    │  Redis Cluster  │
                    │ (Shared Cache)  │
                    └────────┬────────┘
                             │
                    ┌────────┴────────┐
                    │    S3 Backend   │
                    └─────────────────┘
```

**Pros**: Consistent cache, efficient memory usage
**Cons**: Redis dependency, network latency for cache

### Pattern 3: Tiered Cache (Memory + Redis)

```
    ┌─────────────────┐
    │Yatagarasu       │
    ├─────────────────┤
    │ L1: Memory Cache│ ← Fast, per-instance
    │ L2: Redis Cache │ ← Shared across instances
    └────────┬────────┘
             │
    ┌────────┴────────┐
    │    S3 Backend   │
    └─────────────────┘
```

**Recommended**: Best of both worlds

---

## LOAD BALANCER CONFIGURATION

### Nginx Configuration

```nginx
upstream yatagarasu {
    least_conn;  # Use least connections algorithm

    server yatagarasu-1:8080 weight=1 max_fails=3 fail_timeout=30s;
    server yatagarasu-2:8080 weight=1 max_fails=3 fail_timeout=30s;
    server yatagarasu-3:8080 weight=1 max_fails=3 fail_timeout=30s;

    keepalive 100;  # Connection pooling
}

server {
    listen 80;
    server_name proxy.example.com;

    location / {
        proxy_pass http://yatagarasu;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

        # Timeouts
        proxy_connect_timeout 10s;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;

        # Buffering (disable for streaming)
        proxy_buffering off;
        proxy_request_buffering off;
    }

    location /health {
        proxy_pass http://yatagarasu;
        proxy_connect_timeout 5s;
        proxy_read_timeout 5s;
    }
}
```

### HAProxy Configuration

```haproxy
frontend http_front
    bind *:80
    default_backend yatagarasu_backend

backend yatagarasu_backend
    balance leastconn
    option httpchk GET /health
    http-check expect status 200

    server yatagarasu-1 yatagarasu-1:8080 check inter 5s fall 3 rise 2
    server yatagarasu-2 yatagarasu-2:8080 check inter 5s fall 3 rise 2
    server yatagarasu-3 yatagarasu-3:8080 check inter 5s fall 3 rise 2
```

---

## SHARED CACHE SETUP

### Redis Configuration

**Yatagarasu config.yaml**:

```yaml
cache:
  cache_layers:
    - memory  # L1: Fast local cache
    - redis   # L2: Shared cache

  memory:
    max_cache_size_mb: 64
    max_item_size_mb: 5
    default_ttl_seconds: 300

  redis:
    enabled: true
    url: "redis://redis-cluster:6379"
    # Or for Redis Cluster
    # cluster_urls:
    #   - "redis://redis-1:6379"
    #   - "redis://redis-2:6379"
    #   - "redis://redis-3:6379"
    max_connections: 100
    connection_timeout_ms: 5000
    default_ttl_seconds: 3600
```

### Redis Cluster (Production)

```yaml
# docker-compose.redis-cluster.yml
services:
  redis-node-1:
    image: redis:7-alpine
    command: redis-server --cluster-enabled yes --cluster-config-file nodes.conf --cluster-node-timeout 5000
    ports:
      - "6379:6379"

  redis-node-2:
    image: redis:7-alpine
    command: redis-server --cluster-enabled yes --cluster-config-file nodes.conf --cluster-node-timeout 5000
    ports:
      - "6380:6379"

  redis-node-3:
    image: redis:7-alpine
    command: redis-server --cluster-enabled yes --cluster-config-file nodes.conf --cluster-node-timeout 5000
    ports:
      - "6381:6379"
```

---

## KUBERNETES DEPLOYMENT

### Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: yatagarasu
  labels:
    app: yatagarasu
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
      containers:
        - name: yatagarasu
          image: yatagarasu:v1.2.0
          ports:
            - containerPort: 8080
              name: http
            - containerPort: 9090
              name: metrics
          env:
            - name: JWT_SECRET
              valueFrom:
                secretKeyRef:
                  name: yatagarasu-secrets
                  key: jwt-secret
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: yatagarasu-secrets
                  key: aws-access-key
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: yatagarasu-secrets
                  key: aws-secret-key
          resources:
            requests:
              cpu: "500m"
              memory: "256Mi"
            limits:
              cpu: "2000m"
              memory: "1Gi"
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
          volumeMounts:
            - name: config
              mountPath: /etc/yatagarasu
      volumes:
        - name: config
          configMap:
            name: yatagarasu-config
      terminationGracePeriodSeconds: 30
---
apiVersion: v1
kind: Service
metadata:
  name: yatagarasu
spec:
  selector:
    app: yatagarasu
  ports:
    - name: http
      port: 80
      targetPort: 8080
    - name: metrics
      port: 9090
      targetPort: 9090
  type: ClusterIP
---
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
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

### Ingress Configuration

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: yatagarasu
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "0"  # Unlimited for large files
    nginx.ingress.kubernetes.io/proxy-buffering: "off"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
spec:
  ingressClassName: nginx
  rules:
    - host: proxy.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: yatagarasu
                port:
                  number: 80
```

### PodDisruptionBudget

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: yatagarasu
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: yatagarasu
```

---

## DOCKER COMPOSE MULTI-INSTANCE

### docker-compose.multi.yml

```yaml
version: '3.8'

services:
  nginx:
    image: nginx:alpine
    ports:
      - "8080:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - yatagarasu-1
      - yatagarasu-2
      - yatagarasu-3

  yatagarasu-1:
    image: yatagarasu:v1.2.0
    environment:
      - JWT_SECRET=${JWT_SECRET}
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    depends_on:
      - redis
      - minio

  yatagarasu-2:
    image: yatagarasu:v1.2.0
    environment:
      - JWT_SECRET=${JWT_SECRET}
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    depends_on:
      - redis
      - minio

  yatagarasu-3:
    image: yatagarasu:v1.2.0
    environment:
      - JWT_SECRET=${JWT_SECRET}
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    depends_on:
      - redis
      - minio

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru

  minio:
    image: minio/minio
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      - MINIO_ROOT_USER=minioadmin
      - MINIO_ROOT_PASSWORD=minioadmin
    command: server /data --console-address ":9001"
```

---

## HEALTH CHECKS AND READINESS

### Health Endpoints

| Endpoint | Purpose | Response |
|----------|---------|----------|
| `/health` | Liveness | `200 OK` if process running |
| `/ready` | Readiness | `200 OK` if ready for traffic |
| `/metrics` | Prometheus | Metrics in Prometheus format |

### Readiness Conditions

The `/ready` endpoint returns `200 OK` when:
- Configuration loaded successfully
- S3 backend connectivity verified (if configured)
- Cache layers initialized
- No critical errors

### Graceful Shutdown

When receiving SIGTERM:
1. Stop accepting new connections
2. Return `503` on `/ready` (removed from load balancer)
3. Wait for in-flight requests (up to 30s default)
4. Close connections and exit

---

## SESSION AFFINITY

Session affinity (sticky sessions) is **NOT required** for Yatagarasu, but can improve cache hit rates.

### When to Use

- **With memory-only cache**: Improves hit rate
- **With shared Redis cache**: Not needed
- **With tiered cache**: Optional, minor improvement

### Nginx Configuration (IP Hash)

```nginx
upstream yatagarasu {
    ip_hash;  # Enable session affinity by client IP
    server yatagarasu-1:8080;
    server yatagarasu-2:8080;
    server yatagarasu-3:8080;
}
```

### Kubernetes (Session Affinity)

```yaml
apiVersion: v1
kind: Service
metadata:
  name: yatagarasu
spec:
  selector:
    app: yatagarasu
  sessionAffinity: ClientIP
  sessionAffinityConfig:
    clientIP:
      timeoutSeconds: 3600
  ports:
    - port: 80
      targetPort: 8080
```

---

## RESOURCE SIZING

### Per-Instance Guidelines

| Workload | CPU | Memory | Instances |
|----------|-----|--------|-----------|
| Light (<500 RPS) | 1 core | 512MB | 2 |
| Medium (<2000 RPS) | 2 cores | 1GB | 3 |
| Heavy (<10000 RPS) | 4 cores | 2GB | 5 |
| Very Heavy (>10000 RPS) | 4 cores | 4GB | 10+ |

### Scaling Formula

```
instances = ceil(target_rps / 5000) + 1  # +1 for redundancy
memory_per_instance = cache_size_mb + 256MB  # Base overhead
```

---

## SEE ALSO

- [OPERATIONS.md](OPERATIONS.md) - Monitoring and alerts
- [BENCHMARK_RESULTS_V1.2.md](BENCHMARK_RESULTS_V1.2.md) - Performance baselines
- [GRACEFUL_SHUTDOWN.md](GRACEFUL_SHUTDOWN.md) - Shutdown behavior

---

*Generated: December 2025*
*Yatagarasu v1.2.0 Deployment Guide*
