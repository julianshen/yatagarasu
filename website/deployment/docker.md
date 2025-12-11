---
title: Docker Deployment
layout: default
parent: Deployment
nav_order: 1
---

# Docker Deployment

Deploy Yatagarasu with Docker for production use.
{: .fs-6 .fw-300 }

---

## Docker Image

```bash
# Latest stable release
docker pull ghcr.io/julianshen/yatagarasu:1.2.0

# Latest development
docker pull ghcr.io/julianshen/yatagarasu:latest
```

Image details:
- Base: Distroless (minimal attack surface)
- Size: ~40MB
- Architectures: linux/amd64, linux/arm64
- User: Non-root (UID 65532)

---

## Basic Deployment

```bash
docker run -d \
  --name yatagarasu \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9090:9090 \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
  -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

---

## Production Configuration

### Resource Limits

```bash
docker run -d \
  --name yatagarasu \
  --restart unless-stopped \
  --memory 1g \
  --memory-reservation 512m \
  --cpus 2 \
  -p 8080:8080 \
  -p 9090:9090 \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
  -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

### With Disk Cache

```bash
docker run -d \
  --name yatagarasu \
  --restart unless-stopped \
  --memory 1g \
  --cpus 2 \
  -p 8080:8080 \
  -p 9090:9090 \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -v yatagarasu-cache:/var/cache/yatagarasu \
  -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
  -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

### Health Checks

```bash
docker run -d \
  --name yatagarasu \
  --restart unless-stopped \
  --health-cmd="curl -f http://localhost:8080/health || exit 1" \
  --health-interval=10s \
  --health-timeout=5s \
  --health-retries=3 \
  -p 8080:8080 \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

---

## Docker Compose (Production)

```yaml
version: "3.8"

services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
      - cache-data:/var/cache/yatagarasu
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - JWT_SECRET=${JWT_SECRET}
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 1G
        reservations:
          cpus: '0.5'
          memory: 256M
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 10s
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"

volumes:
  cache-data:
```

---

## Multi-Instance with Nginx

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
    restart: unless-stopped

  yatagarasu-1:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - REDIS_URL=redis://redis:6379
    depends_on:
      - redis
    restart: unless-stopped

  yatagarasu-2:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    volumes:
      - ./config.yaml:/etc/yatagarasu/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - REDIS_URL=redis://redis:6379
    depends_on:
      - redis
    restart: unless-stopped

  redis:
    image: valkey/valkey:7-alpine
    volumes:
      - redis-data:/data
    command: valkey-server --maxmemory 256mb --maxmemory-policy allkeys-lru
    restart: unless-stopped

volumes:
  redis-data:
```

Nginx configuration:

```nginx
events {
    worker_connections 2048;
}

http {
    upstream yatagarasu {
        least_conn;
        server yatagarasu-1:8080;
        server yatagarasu-2:8080;
        keepalive 32;
    }

    server {
        listen 80;

        location / {
            proxy_pass http://yatagarasu;
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;

            # Streaming support
            proxy_buffering off;
            proxy_request_buffering off;

            # Timeouts
            proxy_connect_timeout 10s;
            proxy_read_timeout 300s;
            proxy_send_timeout 300s;
        }

        location /health {
            proxy_pass http://yatagarasu;
        }
    }
}
```

---

## Security Hardening

### Read-only Root Filesystem

```bash
docker run -d \
  --name yatagarasu \
  --read-only \
  --tmpfs /tmp \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -v yatagarasu-cache:/var/cache/yatagarasu \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

### Drop Capabilities

```bash
docker run -d \
  --name yatagarasu \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  ghcr.io/julianshen/yatagarasu:1.2.0
```

### Docker Compose Security

```yaml
services:
  yatagarasu:
    image: ghcr.io/julianshen/yatagarasu:1.2.0
    read_only: true
    tmpfs:
      - /tmp
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
```

---

## Operations

### Hot Reload

```bash
docker kill --signal=HUP yatagarasu
```

### Graceful Shutdown

```bash
# Default stop signal is SIGTERM
docker stop yatagarasu

# With custom timeout
docker stop --time 30 yatagarasu
```

### View Logs

```bash
# Follow logs
docker logs -f yatagarasu

# Last 100 lines with timestamps
docker logs --tail 100 -t yatagarasu
```

### Shell Access (Debugging)

```bash
# Note: Distroless image has no shell
# Use debug variant for troubleshooting
docker run -it --rm \
  ghcr.io/julianshen/yatagarasu:1.2.0-debug \
  /bin/sh
```

---

## Monitoring

### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'yatagarasu'
    docker_sd_configs:
      - host: unix:///var/run/docker.sock
    relabel_configs:
      - source_labels: [__meta_docker_container_name]
        regex: '/yatagarasu.*'
        action: keep
      - source_labels: [__address__]
        regex: '(.+):8080'
        replacement: '${1}:9090'
        target_label: __address__
```

### Docker Stats

```bash
# Resource usage
docker stats yatagarasu

# Format output
docker stats --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"
```

---

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs yatagarasu

# Check configuration
docker run --rm \
  -v /path/to/config.yaml:/etc/yatagarasu/config.yaml:ro \
  ghcr.io/julianshen/yatagarasu:1.2.0 \
  --config /etc/yatagarasu/config.yaml --validate
```

### Performance Issues

```bash
# Check resource usage
docker stats yatagarasu

# Check for OOM kills
docker inspect yatagarasu | grep -A5 State
```

### Network Issues

```bash
# Test S3 connectivity from container
docker exec yatagarasu curl -I https://s3.amazonaws.com

# Check DNS resolution
docker exec yatagarasu nslookup s3.amazonaws.com
```

---

## See Also

- [Docker Compose Tutorial](/yatagarasu/getting-started/docker-compose/)
- [Kubernetes Deployment](/yatagarasu/deployment/kubernetes/)
- [High Availability](/yatagarasu/deployment/high-availability/)
