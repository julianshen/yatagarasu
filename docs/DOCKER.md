# Docker Deployment Guide

This guide covers running Yatagarasu in Docker containers for development, testing, and production.

## Quick Start with Docker Compose

The easiest way to test Yatagarasu is with Docker Compose and MinIO:

```bash
# Start MinIO and setup test buckets
docker-compose up -d

# Check logs
docker-compose logs -f

# Access MinIO Console: http://localhost:9001
# Username: minioadmin
# Password: minioadmin

# Test endpoints (once proxy is implemented in v0.2.0)
curl http://localhost:8080/public/hello.txt
curl http://localhost:8080/health
```

## Docker Compose Services

The `docker-compose.yml` includes:

1. **MinIO** - S3-compatible storage
   - S3 API: `localhost:9000`
   - Console: `localhost:9001`
   - Pre-configured with test buckets

2. **MinIO Setup** - Initialization container
   - Creates test buckets
   - Uploads sample test files
   - Sets bucket policies

3. **Yatagarasu** (commented out until v0.2.0)
   - HTTP proxy: `localhost:8080`
   - Metrics: `localhost:9090`

## Building the Docker Image

```bash
# Build the image
docker build -t yatagarasu:latest .

# Check image size
docker images yatagarasu:latest

# Run the container
docker run -d \
  --name yatagarasu \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  yatagarasu:latest
```

## Production Deployment

### Using Docker Compose

1. Copy example files:
```bash
cp .env.example .env
cp config.example.yaml config.yaml
```

2. Edit `.env` with your AWS credentials:
```bash
AWS_ACCESS_KEY_PUBLIC=your-key-here
AWS_SECRET_KEY_PUBLIC=your-secret-here
JWT_SECRET=your-jwt-secret-minimum-32-chars
```

3. Edit `config.yaml` with your bucket configuration

4. Start the services:
```bash
docker-compose up -d yatagarasu
```

### Using Pre-built Images (v0.4.0+)

Pull from GitHub Container Registry:

```bash
# Pull latest version
docker pull ghcr.io/yourusername/yatagarasu:latest

# Pull specific version
docker pull ghcr.io/yourusername/yatagarasu:v0.4.0

# Run the container
docker run -d \
  --name yatagarasu \
  -p 8080:8080 \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -e AWS_ACCESS_KEY_PUBLIC=$AWS_ACCESS_KEY \
  -e AWS_SECRET_KEY_PUBLIC=$AWS_SECRET_KEY \
  -e JWT_SECRET=$JWT_SECRET \
  ghcr.io/yourusername/yatagarasu:latest
```

## Docker Image Details

The Dockerfile uses multi-stage builds:

1. **Builder stage** (`rust:1.70-slim`)
   - Compiles Rust code
   - Strips debug symbols
   - ~1.5GB intermediate image

2. **Runtime stage** (`debian:bookworm-slim`)
   - Only contains binary and dependencies
   - Runs as non-root user (yatagarasu:1000)
   - Final image: ~50-100MB

### Security Features

- ✅ Runs as non-root user
- ✅ Minimal attack surface (distroless-style)
- ✅ No shell in final image
- ✅ Health check included
- ✅ Proper signal handling (SIGTERM)

## Environment Variables

Override config values with environment variables:

```bash
# AWS credentials
-e AWS_ACCESS_KEY_PUBLIC=xxx
-e AWS_SECRET_KEY_PUBLIC=xxx
-e AWS_ACCESS_KEY_PRIVATE=xxx
-e AWS_SECRET_KEY_PRIVATE=xxx

# JWT configuration
-e JWT_SECRET=your-secret-key

# Server configuration
-e SERVER_ADDRESS=0.0.0.0:8080
-e METRICS_PORT=9090

# Logging
-e RUST_LOG=info
-e LOG_FORMAT=json
```

## Volume Mounts

Mount configuration and logs:

```bash
docker run -d \
  -v $(pwd)/config.yaml:/etc/yatagarasu/config.yaml:ro \
  -v $(pwd)/logs:/var/log/yatagarasu \
  yatagarasu:latest
```

## Health Checks

The Docker image includes a health check:

```bash
# Check container health
docker inspect --format='{{.State.Health.Status}}' yatagarasu

# View health check logs
docker inspect --format='{{range .State.Health.Log}}{{.Output}}{{end}}' yatagarasu
```

Health check endpoint: `http://localhost:8080/health`

## Networking

### Bridge Network (default)

Containers can communicate via service names:

```yaml
services:
  minio:
    # Accessible as http://minio:9000 from yatagarasu container

  yatagarasu:
    environment:
      S3_ENDPOINT: http://minio:9000
```

### Host Network

For maximum performance:

```bash
docker run --network host yatagarasu:latest
```

## Troubleshooting

### Container won't start

```bash
# Check logs
docker logs yatagarasu

# Check if port is in use
sudo lsof -i :8080

# Verify config file exists
docker exec yatagarasu ls -la /etc/yatagarasu/
```

### Can't connect to MinIO

```bash
# Verify MinIO is running
docker ps | grep minio

# Check MinIO health
curl http://localhost:9000/minio/health/live

# Check network connectivity
docker exec yatagarasu ping -c 3 minio
```

### Permission denied

The container runs as user `yatagarasu` (UID 1000). Ensure mounted volumes have correct permissions:

```bash
# Fix config permissions
chmod 644 config.yaml

# Fix log directory permissions
chown -R 1000:1000 logs/
```

## Performance Tuning

### Resource Limits

```bash
docker run -d \
  --memory=512m \
  --cpus=2 \
  yatagarasu:latest
```

### Logging

Structured JSON logs go to stdout:

```bash
# View logs
docker logs -f yatagarasu

# Export logs to file
docker logs yatagarasu > yatagarasu.log

# Send to log aggregator
docker logs yatagarasu | fluentd
```

## Kubernetes Deployment (Future)

Example Kubernetes manifests will be provided in v0.4.0+:

- Deployment with rolling updates
- Service with load balancer
- ConfigMap for configuration
- Secret for credentials
- HPA for autoscaling

## CI/CD Integration

GitHub Actions workflows are included:

- `.github/workflows/ci.yml` - Run tests on every push
- `.github/workflows/release.yml` - Build and publish on tags

See [CI/CD documentation](../README.md#cicd) for details.

---

**Status**: Docker support will be available in **v0.4.0** (after server implementation in v0.2.0-v0.3.0)

For now, you can use `docker-compose.yml` to run MinIO for local testing.
