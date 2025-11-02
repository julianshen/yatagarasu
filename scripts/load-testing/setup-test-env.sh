#!/bin/bash
# Setup complete load testing environment for Yatagarasu
# Creates MinIO, uploads test files, generates test config

set -e

echo "🚀 Setting up Yatagarasu load testing environment..."
echo ""

# Configuration
MINIO_PORT="${MINIO_PORT:-9000}"
MINIO_CONSOLE_PORT="${MINIO_CONSOLE_PORT:-9001}"
MINIO_ROOT_USER="${MINIO_ROOT_USER:-minioadmin}"
MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:-minioadmin}"
CONTAINER_NAME="${CONTAINER_NAME:-yatagarasu-minio}"

# Test buckets
PUBLIC_BUCKET="test-public"
PRIVATE_BUCKET="test-private"

echo "Step 1: Starting MinIO..."
# Check if container already exists
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "  Removing existing container: $CONTAINER_NAME"
    docker rm -f "$CONTAINER_NAME" > /dev/null
fi

# Start MinIO
docker run -d \
    --name "$CONTAINER_NAME" \
    -p "$MINIO_PORT:9000" \
    -p "$MINIO_CONSOLE_PORT:9001" \
    -e "MINIO_ROOT_USER=$MINIO_ROOT_USER" \
    -e "MINIO_ROOT_PASSWORD=$MINIO_ROOT_PASSWORD" \
    minio/minio server /data --console-address ":9001" > /dev/null

echo "  Waiting for MinIO to be ready..."
sleep 3

# Check if MinIO is responding
if ! curl -s "http://localhost:$MINIO_PORT/minio/health/live" > /dev/null; then
    echo "❌ MinIO failed to start. Check logs with: docker logs $CONTAINER_NAME"
    exit 1
fi
echo "  ✅ MinIO started"

echo ""
echo "Step 2: Creating test buckets..."
# Configure mc (MinIO Client)
docker exec $CONTAINER_NAME mc alias set local http://localhost:9000 $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD > /dev/null 2>&1

# Create buckets
docker exec $CONTAINER_NAME mc mb local/$PUBLIC_BUCKET > /dev/null 2>&1 || true
docker exec $CONTAINER_NAME mc mb local/$PRIVATE_BUCKET > /dev/null 2>&1 || true
echo "  ✅ Created buckets: $PUBLIC_BUCKET, $PRIVATE_BUCKET"

echo ""
echo "Step 3: Uploading test files..."
# Create test files
mkdir -p /tmp/yatagarasu-test

# Small file (1KB)
echo "This is a small test file for Yatagarasu load testing. $(date)" > /tmp/yatagarasu-test/sample.txt
dd if=/dev/zero of=/tmp/yatagarasu-test/1kb.bin bs=1024 count=1 2>/dev/null

# Medium file (100KB)
dd if=/dev/zero of=/tmp/yatagarasu-test/100kb.bin bs=1024 count=100 2>/dev/null

# Large file (10MB)
dd if=/dev/zero of=/tmp/yatagarasu-test/10mb.bin bs=1048576 count=10 2>/dev/null

# Very large file (100MB) - for streaming tests
dd if=/dev/zero of=/tmp/yatagarasu-test/100mb.bin bs=1048576 count=100 2>/dev/null

# Upload to MinIO
for file in sample.txt 1kb.bin 100kb.bin 10mb.bin 100mb.bin; do
    docker exec -i $CONTAINER_NAME mc cp - local/$PUBLIC_BUCKET/$file < /tmp/yatagarasu-test/$file > /dev/null
    docker exec -i $CONTAINER_NAME mc cp - local/$PRIVATE_BUCKET/$file < /tmp/yatagarasu-test/$file > /dev/null
done

echo "  ✅ Uploaded test files (1KB, 100KB, 10MB, 100MB)"

echo ""
echo "Step 4: Generating Yatagarasu config..."
cat > /tmp/yatagarasu-test/config.yaml <<EOF
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  # Public bucket (no authentication)
  - name: "public"
    path_prefix: "/test"
    s3:
      endpoint: "http://localhost:$MINIO_PORT"
      region: "us-east-1"
      bucket: "$PUBLIC_BUCKET"
      access_key: "$MINIO_ROOT_USER"
      secret_key: "$MINIO_ROOT_PASSWORD"
    auth:
      enabled: false

  # Private bucket (JWT authentication required)
  - name: "private"
    path_prefix: "/private"
    s3:
      endpoint: "http://localhost:$MINIO_PORT"
      region: "us-east-1"
      bucket: "$PRIVATE_BUCKET"
      access_key: "$MINIO_ROOT_USER"
      secret_key: "$MINIO_ROOT_PASSWORD"
    auth:
      enabled: true

# JWT configuration (for private bucket)
jwt:
  enabled: true
  secret: "load-test-secret-key-12345"
  algorithm: "HS256"
  token_sources:
    - type: "bearer"
  claims: []
EOF

echo "  ✅ Config created: /tmp/yatagarasu-test/config.yaml"

echo ""
echo "══════════════════════════════════════════════════════════"
echo "  LOAD TESTING ENVIRONMENT READY! 🎉"
echo "══════════════════════════════════════════════════════════"
echo ""
echo "MinIO:"
echo "  - S3 API: http://localhost:$MINIO_PORT"
echo "  - Console: http://localhost:$MINIO_CONSOLE_PORT"
echo "  - User: $MINIO_ROOT_USER"
echo "  - Password: $MINIO_ROOT_PASSWORD"
echo ""
echo "Test Files Uploaded:"
echo "  - /test/sample.txt (1KB text)"
echo "  - /test/1kb.bin (1KB binary)"
echo "  - /test/100kb.bin (100KB binary)"
echo "  - /test/10mb.bin (10MB binary)"
echo "  - /test/100mb.bin (100MB binary)"
echo ""
echo "Next Steps:"
echo "  1. Start Yatagarasu proxy:"
echo "     cargo run --release -- --config /tmp/yatagarasu-test/config.yaml"
echo ""
echo "  2. Run load tests with K6:"
echo "     k6 run scripts/load-testing/test-basic.js"
echo "     k6 run scripts/load-testing/test-concurrent.js"
echo "     k6 run scripts/load-testing/test-streaming.js -e LARGE_FILE=/test/100mb.bin"
echo ""
echo "  3. Generate JWT token for private bucket tests:"
echo "     # Install jwt-cli: cargo install jwt-cli"
echo "     jwt encode --secret 'load-test-secret-key-12345' '{\"sub\":\"testuser\",\"exp\":9999999999}'"
echo ""
echo "  4. Clean up when done:"
echo "     docker stop $CONTAINER_NAME && docker rm $CONTAINER_NAME"
echo "     rm -rf /tmp/yatagarasu-test"
echo ""
