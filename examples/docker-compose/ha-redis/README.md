# HA Redis Docker Compose Example

High availability setup with multiple Yatagarasu proxy instances, shared Redis cache, nginx load balancer, and MinIO distributed storage.

## Architecture

```
                    ┌─────────────┐
                    │   Client    │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │    Nginx    │ :8080
                    │ Load Balancer│
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────▼──────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │ Yatagarasu  │ │ Yatagarasu  │ │ Yatagarasu  │
    │  Instance 1 │ │  Instance 2 │ │  Instance N │
    └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
           │               │               │
           └───────────────┼───────────────┘
                           │
                    ┌──────▼──────┐
                    │    Redis    │ :6379
                    │ Shared Cache│
                    └──────┬──────┘
                           │
           ┌───────────────┴───────────────┐
           │                               │
    ┌──────▼──────┐                 ┌──────▼──────┐
    │   MinIO 1   │ :9000           │   MinIO 2   │
    │  (2 drives) │◄───────────────►│  (2 drives) │
    └─────────────┘  Erasure Coding └─────────────┘
```

## Prerequisites

- Docker and Docker Compose installed
- Ports 8080, 6379, 9000, 9001, 9090 available

## Quick Start

```bash
# Start with 2 proxy instances (default)
docker compose up -d

# Wait for services to be ready
until [ "$(curl -s -o /dev/null -w '%{http_code}' http://localhost:8080/health)" == "200" ]; do
  echo "Waiting for services..."; sleep 2
done

# Test the proxy
curl http://localhost:8080/public/hello.txt
```

## Scaling

```bash
# Scale to 3 instances
docker compose up -d --scale yatagarasu=3

# Scale to 5 instances
docker compose up -d --scale yatagarasu=5
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| nginx | 8080 | Load balancer (entry point) |
| nginx | 9090 | Aggregated metrics |
| redis | 6379 | Shared cache |
| minio1 | 9000 | S3 API (primary node) |
| minio1 | 9001 | MinIO Console |
| minio2 | - | S3 replica node (internal only) |

## Verification

```bash
# Health check
curl http://localhost:8080/health

# Make multiple requests to see load balancing
for i in {1..10}; do
  curl -s -I http://localhost:8080/public/hello.txt | grep X-Upstream
done

# Check cache hits (second request should be faster)
time curl -s http://localhost:8080/public/large.bin > /dev/null
time curl -s http://localhost:8080/public/large.bin > /dev/null

# Check Redis cache
docker exec yatagarasu-ha-redis redis-cli keys '*'

# Check metrics
curl http://localhost:9090/metrics | grep cache_hit
```

## Cache Behavior

- **Memory cache**: Per-instance, 100MB, 5 min TTL
- **Redis cache**: Shared across instances, 256MB, 1 hour TTL
- **Cache hits**: All instances share Redis, so cached data is available everywhere

## Configuration

The setup includes:

- `config.yaml` - Proxy config with Redis cache enabled
- `nginx.conf` - Load balancer configuration

## Cleanup

```bash
# Stop and remove containers
docker compose down

# Also remove volumes
docker compose down -v
```

## Next Steps

- Try the [Full Stack example](../full-stack/) for OPA/OpenFGA authorization
