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
    image: ghcr.io/julianshen/yatagarasu:latest
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
    image: ghcr.io/julianshen/yatagarasu:latest
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
```yaml
version: "3.8"

services:
  # ============================================
  # Load Balancer & Proxy
  # ============================================
  nginx:
    image: nginx:alpine
    container_name: yatagarasu-full-nginx
    ports:
      - "8080:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - yatagarasu-1
      - yatagarasu-2
    networks:
      - yatagarasu-full

  yatagarasu-1:
    image: ghcr.io/julianshen/yatagarasu:latest
    container_name: full-stack-yatagarasu-1
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET=your-super-secret-jwt-key-change-in-production
      - RUST_LOG=info
    depends_on:
      minio:
        condition: service_healthy
      redis:
        condition: service_healthy
      openfga-setup:
        condition: service_completed_successfully
      opa:
        condition: service_healthy
      openfga:
        condition: service_started
    networks:
      - yatagarasu-full

  yatagarasu-2:
    image: ghcr.io/julianshen/yatagarasu:latest
    container_name: full-stack-yatagarasu-2
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - MINIO_ACCESS_KEY=minioadmin
      - MINIO_SECRET_KEY=minioadmin
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET=your-super-secret-jwt-key-change-in-production
      - RUST_LOG=info
    depends_on:
      minio:
        condition: service_healthy
      redis:
        condition: service_healthy
      openfga-setup:
        condition: service_completed_successfully
      opa:
        condition: service_healthy
      openfga:
        condition: service_started
    networks:
      - yatagarasu-full

  # ============================================
  # Storage Backend
  # ============================================
  minio:
    image: minio/minio:latest
    container_name: yatagarasu-full-minio
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: [ "CMD", "curl", "-f", "http://localhost:9000/minio/health/live" ]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - yatagarasu-full

  minio-setup:
    image: minio/mc:latest
    container_name: yatagarasu-full-minio-setup
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c " echo 'Setting up MinIO buckets...'; mc alias set myminio http://minio:9000 minioadmin minioadmin; mc mb --ignore-existing myminio/public-assets; mc mb --ignore-existing myminio/opa-protected; mc mb --ignore-existing myminio/openfga-protected; mc anonymous set public myminio/public-assets; echo 'Hello from Public Bucket!' | mc pipe myminio/public-assets/hello.txt; echo 'OPA Protected Content' | mc pipe myminio/opa-protected/test.txt; echo 'OpenFGA Protected Content' | mc pipe myminio/openfga-protected/secret.txt; dd if=/dev/urandom of=/tmp/data.bin bs=100K count=1 2>/dev/null; mc cp /tmp/data.bin myminio/public-assets/data.bin; mc cp /tmp/data.bin myminio/opa-protected/data.bin; mc cp /tmp/data.bin myminio/openfga-protected/data.bin; echo 'MinIO setup complete!'; "
    networks:
      - yatagarasu-full

  # ============================================
  # Cache Layer
  # ============================================
  redis:
    image: redis:7-alpine
    container_name: yatagarasu-full-redis
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes --maxmemory 256mb --maxmemory-policy allkeys-lru
    healthcheck:
      test: [ "CMD", "redis-cli", "ping" ]
      interval: 5s
      timeout: 3s
      retries: 5
    networks:
      - yatagarasu-full

  # ============================================
  # OPA (Open Policy Agent)
  # ============================================
  opa:
    image: openpolicyagent/opa:latest-debug
    container_name: yatagarasu-full-opa
    ports:
      - "8181:8181"
    command:
      - "run"
      - "--server"
      - "--addr=0.0.0.0:8181"
      - "/policies"
    volumes:
      - ./opa/policy.rego:/policies/policy.rego:ro
    healthcheck:
      test: [ "CMD-SHELL", "wget -qO- http://localhost:8181/health || exit 1" ]
      interval: 10s
      timeout: 5s
      retries: 3
    networks:
      - yatagarasu-full

  # ============================================
  # OpenFGA
  # ============================================
  postgres:
    image: postgres:15-alpine
    container_name: yatagarasu-full-postgres
    environment:
      POSTGRES_USER: openfga
      POSTGRES_PASSWORD: openfga
      POSTGRES_DB: openfga
    healthcheck:
      test: [ "CMD-SHELL", "pg_isready -U openfga" ]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - yatagarasu-full

  openfga-migrate:
    image: openfga/openfga:latest
    container_name: yatagarasu-full-openfga-migrate
    command: migrate
    environment:
      OPENFGA_DATASTORE_ENGINE: postgres
      OPENFGA_DATASTORE_URI: postgres://openfga:openfga@postgres:5432/openfga?sslmode=disable
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - yatagarasu-full

  openfga:
    image: openfga/openfga:latest
    container_name: yatagarasu-full-openfga
    ports:
      - "8082:8080" # API port
      - "8083:3000" # Playground port
      - "3000:3000" # gRPC port
    command: run
    environment:
      OPENFGA_DATASTORE_ENGINE: postgres
      OPENFGA_DATASTORE_URI: postgres://openfga:openfga@postgres:5432/openfga?sslmode=disable
      OPENFGA_LOG_FORMAT: json
    depends_on:
      openfga-migrate:
        condition: service_completed_successfully
    networks:
      - yatagarasu-full

  openfga-setup:
    image: curlimages/curl:latest
    container_name: yatagarasu-full-openfga-setup
    depends_on:
      openfga:
        condition: service_started
    volumes:
      - ./openfga/model.json:/model.json:ro
      - ./openfga/tuples.json:/tuples.json:ro
    entrypoint: >
      /bin/sh -c "
      echo 'Waiting for OpenFGA...'
      while ! curl -s http://openfga:8080/healthz > /dev/null; do
        echo 'Waiting for OpenFGA to be healthy...'
        sleep 2
      done

      echo 'Configuring OpenFGA...'
      
      # Determine Store ID - either create or list existing
      EXISTING_STORES=\$(curl -s http://openfga:8080/stores | grep -o '\"id\":\"[^\"]*\"' | cut -d'\"' -f4)
      
      if [ -z \"\$EXISTING_STORES\" ]; then
          echo 'Creating new store...'
          STORE_ID=\$(curl -s -X POST http://openfga:8080/stores -H 'Content-Type: application/json' -d '{\"name\": \"yatagarasu\"}' | grep -o '\"id\":\"[^\"]*\"' | cut -d'\"' -f4)
      else
          echo 'Using existing store...'
          STORE_ID=\$(echo \$EXISTING_STORES | head -n 1)
      fi
      
      echo \"OPENFGA_STORE_ID=\$STORE_ID\"
      
      echo 'Writing Authorization Model...'
      MODEL_ID=\$(curl -s -X POST http://openfga:8080/stores/\$STORE_ID/authorization-models -H 'Content-Type: application/json' -d @/model.json | grep -o '\"authorization_model_id\":\"[^\"]*\"' | cut -d'\"' -f4)
      echo \"MODEL_ID=\$MODEL_ID\"
      
      echo 'Writing Tuples...'
      curl -s -X POST http://openfga:8080/stores/\$STORE_ID/write -H 'Content-Type: application/json' -d @/tuples.json
      
      echo 'OpenFGA Setup Complete!'
      echo 'IMPORTANT: Update config.yaml with this Store ID: \$STORE_ID'
      "
    networks:
      - yatagarasu-full

networks:
  yatagarasu-full:
    driver: bridge
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
    image: ghcr.io/julianshen/yatagarasu:latest
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
