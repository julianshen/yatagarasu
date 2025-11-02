#!/bin/bash
# Start MinIO for load testing
# MinIO provides a lightweight S3-compatible storage for testing

set -e

MINIO_PORT="${MINIO_PORT:-9000}"
MINIO_CONSOLE_PORT="${MINIO_CONSOLE_PORT:-9001}"
MINIO_ROOT_USER="${MINIO_ROOT_USER:-minioadmin}"
MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:-minioadmin}"
CONTAINER_NAME="${CONTAINER_NAME:-yatagarasu-minio}"

echo "Starting MinIO for Yatagarasu load testing..."
echo "  - S3 API: http://localhost:$MINIO_PORT"
echo "  - Console: http://localhost:$MINIO_CONSOLE_PORT"
echo "  - User: $MINIO_ROOT_USER"
echo "  - Password: $MINIO_ROOT_PASSWORD"

# Check if container already exists
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "Removing existing container: $CONTAINER_NAME"
    docker rm -f "$CONTAINER_NAME"
fi

# Start MinIO
docker run -d \
    --name "$CONTAINER_NAME" \
    -p "$MINIO_PORT:9000" \
    -p "$MINIO_CONSOLE_PORT:9001" \
    -e "MINIO_ROOT_USER=$MINIO_ROOT_USER" \
    -e "MINIO_ROOT_PASSWORD=$MINIO_ROOT_PASSWORD" \
    minio/minio server /data --console-address ":9001"

echo ""
echo "Waiting for MinIO to be ready..."
sleep 3

# Check if MinIO is responding
if curl -s "http://localhost:$MINIO_PORT/minio/health/live" > /dev/null; then
    echo "✅ MinIO is ready!"
else
    echo "❌ MinIO is not responding. Check logs with: docker logs $CONTAINER_NAME"
    exit 1
fi

echo ""
echo "MinIO started successfully!"
echo ""
echo "Next steps:"
echo "  1. Create a test bucket:"
echo "     docker exec $CONTAINER_NAME mc alias set local http://localhost:9000 $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD"
echo "     docker exec $CONTAINER_NAME mc mb local/test-bucket"
echo ""
echo "  2. Upload test files:"
echo "     echo 'Hello from Yatagarasu' > /tmp/test.txt"
echo "     docker exec -i $CONTAINER_NAME mc cp - local/test-bucket/test.txt < /tmp/test.txt"
echo ""
echo "  3. Update config.yaml with MinIO endpoint:"
echo "     endpoint: \"http://localhost:$MINIO_PORT\""
echo "     access_key: \"$MINIO_ROOT_USER\""
echo "     secret_key: \"$MINIO_ROOT_PASSWORD\""
echo ""
echo "To stop MinIO:"
echo "  docker stop $CONTAINER_NAME"
echo "  docker rm $CONTAINER_NAME"
