# Simple Docker Compose Example

Minimal Yatagarasu setup with a single proxy instance and MinIO S3-compatible storage.

## Prerequisites

- Docker and Docker Compose installed
- Ports 8080, 9000, 9001, 9090 available

## Quick Start

```bash
# Start the services
docker compose up -d

# Wait for services to be ready (about 10 seconds)
sleep 10

# Test the proxy
curl http://localhost:8080/public/hello.txt
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| yatagarasu | 8080 | S3 Proxy |
| yatagarasu | 9090 | Prometheus metrics |
| minio | 9000 | S3 API |
| minio | 9001 | MinIO Console |

## Verification

```bash
# Health check
curl http://localhost:8080/health

# Get a text file
curl http://localhost:8080/public/hello.txt

# Get JSON
curl http://localhost:8080/public/welcome.json

# Get binary file
curl -o /tmp/sample.bin http://localhost:8080/public/sample.bin

# Check metrics
curl http://localhost:9090/metrics
```

## MinIO Console

Access the MinIO web console at http://localhost:9001

- **Username**: minioadmin
- **Password**: minioadmin

## Configuration

The proxy configuration is in `config.yaml`. It defines:

- Single public bucket at `/public` path prefix
- No authentication required
- Connects to MinIO at `http://minio:9000`

## Cleanup

```bash
# Stop and remove containers
docker compose down

# Also remove volumes (deletes all data)
docker compose down -v
```

## Next Steps

- Try the [HA Redis example](../ha-redis/) for high availability setup
- Try the [Full Stack example](../full-stack/) for OPA/OpenFGA authorization
