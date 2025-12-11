---
title: Docker Compose
layout: default
parent: Getting Started
nav_order: 3
---

# Docker Compose Setup

Set up a complete development environment with Docker Compose.
{: .fs-6 .fw-300 }

---

## Quick Start

Clone and run the default Docker Compose setup:

```bash
git clone https://github.com/julianshen/yatagarasu.git
cd yatagarasu

# Start all services
docker compose up -d

# Test the proxy
curl http://localhost:8080/public/hello.txt
# Output: Hello, World!

# View logs
docker compose logs -f yatagarasu

# Stop all services
docker compose down
```

This starts:
- **Yatagarasu** on port 8080 (metrics on 9090)
- **MinIO** (S3) on port 9000 (console on 9001)
- Pre-created test bucket with sample files

---

## Basic Setup with MinIO

Create a minimal `docker-compose.yml`:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
    depends_on:
      minio:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 5s
      retries: 3

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 10s
      timeout: 5s
      retries: 3
    volumes:
      - minio-data:/data

  # Initialize MinIO with test bucket
  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set local http://minio:9000 minioadmin minioadmin;
      mc mb local/test-bucket --ignore-existing;
      echo 'Hello, World!' | mc pipe local/test-bucket/hello.txt;
      echo 'Bucket initialized!';
      "

volumes:
  minio-data:
```

Create the configuration file:

```yaml
# config.yaml
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test"
    path_prefix: "/public"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

metrics:
  enabled: true
  port: 9090
```

---

## Setup with Valkey (Redis-Compatible Cache)

Add distributed caching with Valkey:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://valkey:6379
    depends_on:
      minio:
        condition: service_healthy
      valkey:
        condition: service_healthy

  valkey:
    image: valkey/valkey:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - valkey-data:/data
    healthcheck:
      test: ["CMD", "valkey-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 3
    command: valkey-server --appendonly yes

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 10s
      timeout: 5s
      retries: 3
    volumes:
      - minio-data:/data

  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set local http://minio:9000 minioadmin minioadmin;
      mc mb local/test-bucket --ignore-existing;
      echo 'Hello from MinIO!' | mc pipe local/test-bucket/hello.txt;
      "

volumes:
  minio-data:
  valkey-data:
```

Configuration with Valkey cache:

```yaml
# config.yaml
server:
  address: "0.0.0.0:8080"
  threads: 4

buckets:
  - name: "test"
    path_prefix: "/public"
    s3:
      bucket: "test-bucket"
      region: "us-east-1"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: false

cache:
  memory:
    max_capacity: 268435456  # 256MB
    ttl_seconds: 3600
  redis:
    enabled: true
    url: "${REDIS_URL}"
    max_capacity: 536870912  # 512MB
    ttl_seconds: 7200

metrics:
  enabled: true
  port: 9090
```

---

## Full Stack Setup

Complete production-like environment with all components:

```yaml
version: "3.8"

services:
  # Load Balancer
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - yatagarasu-1
      - yatagarasu-2

  # Yatagarasu Instance 1
  yatagarasu-1:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
      - cache-1:/var/cache/yatagarasu
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://valkey:6379
      - JWT_SECRET=your-jwt-secret-here
    depends_on:
      minio:
        condition: service_healthy
      valkey:
        condition: service_healthy

  # Yatagarasu Instance 2
  yatagarasu-2:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
      - cache-2:/var/cache/yatagarasu
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://valkey:6379
      - JWT_SECRET=your-jwt-secret-here
    depends_on:
      minio:
        condition: service_healthy
      valkey:
        condition: service_healthy

  # Valkey (Redis-compatible)
  valkey:
    image: valkey/valkey:7-alpine
    volumes:
      - valkey-data:/data
    healthcheck:
      test: ["CMD", "valkey-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 3

  # MinIO S3
  minio:
    image: minio/minio:latest
    ports:
      - "9001:9001"  # Console only
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 10s
      timeout: 5s
      retries: 3
    volumes:
      - minio-data:/data

  # Prometheus (optional monitoring)
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9092:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'

  # Grafana (optional dashboards)
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana

volumes:
  minio-data:
  valkey-data:
  cache-1:
  cache-2:
  grafana-data:
```

Nginx load balancer configuration (`nginx.conf`):

```nginx
events {
    worker_connections 1024;
}

http {
    upstream yatagarasu {
        least_conn;
        server yatagarasu-1:8080;
        server yatagarasu-2:8080;
    }

    server {
        listen 80;

        location / {
            proxy_pass http://yatagarasu;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            # Streaming support
            proxy_buffering off;
            proxy_request_buffering off;
        }

        location /health {
            proxy_pass http://yatagarasu;
        }
    }
}
```

Prometheus configuration (`prometheus.yml`):

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'yatagarasu'
    static_configs:
      - targets:
          - 'yatagarasu-1:9090'
          - 'yatagarasu-2:9090'
```

---

## Setup with OPA (Open Policy Agent)

Add policy-based authorization:

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - JWT_SECRET=your-jwt-secret
    depends_on:
      - minio
      - opa

  opa:
    image: openpolicyagent/opa:latest
    ports:
      - "8181:8181"
    volumes:
      - ./policies:/policies
    command:
      - "run"
      - "--server"
      - "--addr=0.0.0.0:8181"
      - "/policies"

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  minio-data:
```

OPA policy example (`policies/yatagarasu.rego`):

```rego
package yatagarasu.authz

default allow = false

# Allow access if user has required role
allow {
    input.claims.roles[_] == "admin"
}

allow {
    input.claims.roles[_] == "reader"
    input.method == "GET"
}

# Allow access to specific paths
allow {
    startswith(input.path, "/public/")
}
```

Configuration with OPA:

```yaml
buckets:
  - name: "protected"
    path_prefix: "/protected"
    s3:
      bucket: "protected-bucket"
      endpoint: "http://minio:9000"
      access_key: "${MINIO_ACCESS_KEY}"
      secret_key: "${MINIO_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
    authorization:
      type: "opa"
      url: "http://opa:8181"
      policy_path: "yatagarasu/authz/allow"
```

---

## Scaling Instances

Scale Yatagarasu instances dynamically:

```bash
# Scale to 5 instances
docker compose up -d --scale yatagarasu=5

# Check all instances
docker compose ps

# View logs from all instances
docker compose logs -f yatagarasu
```

---

## Useful Commands

```bash
# Start in foreground (see all logs)
docker compose up

# Start in background
docker compose up -d

# View logs
docker compose logs -f yatagarasu

# Restart specific service
docker compose restart yatagarasu

# Stop and remove everything
docker compose down -v

# Rebuild and restart
docker compose up -d --build --force-recreate

# Check service health
docker compose ps

# Execute command in container
docker compose exec yatagarasu curl localhost:8080/health
```

---

## Next Steps

- [Kubernetes Deployment](/yatagarasu/deployment/kubernetes/) - Production K8s setup
- [High Availability](/yatagarasu/deployment/high-availability/) - HA configuration
- [Configuration Reference](/yatagarasu/configuration/) - All options explained
