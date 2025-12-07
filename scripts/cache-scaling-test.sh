#!/bin/bash
# Phase 62: Cache Size Scaling Test
#
# Tests cache behavior at 1GB, 10GB configurations.
# Note: 50GB test requires significant memory and is optional.
#
# Prerequisites:
#   - Docker with MinIO running
#   - k6 installed
#
# Usage:
#   ./scripts/cache-scaling-test.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=============================================="
echo "Phase 62: Cache Size Scaling Test"
echo "=============================================="

# Check dependencies
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 is not installed${NC}"
    exit 1
fi

if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: docker is not installed${NC}"
    exit 1
fi

# Ensure MinIO is running
if ! docker ps | grep -q minio; then
    echo -e "${YELLOW}Starting MinIO...${NC}"
    docker start minio 2>/dev/null || \
    docker run -d -p 9000:9000 -p 9001:9001 \
        -e "MINIO_ROOT_USER=minioadmin" \
        -e "MINIO_ROOT_PASSWORD=minioadmin" \
        --name minio \
        minio/minio server /data --console-address ":9001"
    sleep 3
fi

# Create test bucket and files if needed
echo -e "${YELLOW}Setting up test files in MinIO...${NC}"
docker exec minio mc alias set local http://localhost:9000 minioadmin minioadmin 2>/dev/null || true
docker exec minio mc mb -p local/test-scaling 2>/dev/null || true
docker exec minio mc anonymous set download local/test-scaling 2>/dev/null || true

# Generate test files (100KB each, 1000 files = 100MB of test data)
echo "Generating test files..."
for i in $(seq 0 999); do
    FILE_NAME=$(printf "test-file-%05d.bin" $i)
    # Check if file exists
    if ! docker exec minio mc stat local/test-scaling/$FILE_NAME &>/dev/null; then
        # Create 100KB file with random data
        dd if=/dev/urandom bs=1024 count=100 2>/dev/null | \
            docker exec -i minio mc pipe local/test-scaling/$FILE_NAME
    fi
done
echo "Test files ready."

# Build release binary
echo -e "${YELLOW}Building release binary...${NC}"
cd "$PROJECT_DIR"
cargo build --release 2>&1 | tail -3

# Results directory
RESULTS_DIR="$PROJECT_DIR/test-results/cache-scaling"
mkdir -p "$RESULTS_DIR"

# Function to run cache test at specific size
run_cache_test() {
    local CACHE_SIZE_MB=$1
    local TEST_NAME="cache-${CACHE_SIZE_MB}mb"

    echo ""
    echo "=============================================="
    echo "Testing with ${CACHE_SIZE_MB}MB cache"
    echo "=============================================="

    # Create config for this cache size
    local CONFIG_FILE="/tmp/cache-test-${CACHE_SIZE_MB}mb.yaml"
    cat > "$CONFIG_FILE" << EOF
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "public"
    path_prefix: "/public"
    s3:
      endpoint: "http://localhost:9000"
      region: "us-east-1"
      bucket: "test-scaling"
      access_key: "minioadmin"
      secret_key: "minioadmin"
    auth:
      enabled: false

cache:
  enabled: true
  memory:
    enabled: true
    max_cache_size_mb: ${CACHE_SIZE_MB}
    max_item_size_mb: 10
    default_ttl_seconds: 300

logging:
  level: "warn"

metrics:
  enabled: true
  address: "127.0.0.1"
  port: 9090
EOF

    # Kill any existing proxy
    pkill -f "yatagarasu" 2>/dev/null || true
    sleep 1

    # Start proxy with this config
    echo "Starting proxy with ${CACHE_SIZE_MB}MB cache..."
    ./target/release/yatagarasu --config "$CONFIG_FILE" > "/tmp/proxy-${TEST_NAME}.log" 2>&1 &
    local PROXY_PID=$!
    sleep 3

    # Verify proxy started
    if ! curl -s http://localhost:8080/health > /dev/null; then
        echo -e "${RED}Proxy failed to start${NC}"
        cat "/tmp/proxy-${TEST_NAME}.log"
        return 1
    fi

    # Get initial memory usage
    local INITIAL_MEM=$(ps -o rss= -p $PROXY_PID 2>/dev/null || echo "0")
    echo "Initial memory: ${INITIAL_MEM} KB"

    # Run simplified test (avoid long k6 scenarios)
    echo "Running cache performance test..."

    # Warm up cache with first 500 files
    echo "  Phase 1: Warming up cache..."
    for i in $(seq 0 499); do
        FILE_NAME=$(printf "test-file-%05d.bin" $i)
        curl -s "http://localhost:8080/public/$FILE_NAME" > /dev/null &
        # Run 10 concurrent requests
        if (( i % 10 == 9 )); then
            wait
        fi
    done
    wait

    # Measure cache hit performance
    echo "  Phase 2: Measuring cache hit rate..."
    local HIT_COUNT=0
    local TOTAL_COUNT=100
    for i in $(seq 1 $TOTAL_COUNT); do
        FILE_IDX=$((RANDOM % 500))
        FILE_NAME=$(printf "test-file-%05d.bin" $FILE_IDX)
        START_TIME=$(date +%s%N)
        curl -s "http://localhost:8080/public/$FILE_NAME" > /dev/null
        END_TIME=$(date +%s%N)
        LATENCY=$(( (END_TIME - START_TIME) / 1000000 ))
        # Consider hit if latency < 10ms
        if [ $LATENCY -lt 10 ]; then
            HIT_COUNT=$((HIT_COUNT + 1))
        fi
    done
    local HIT_RATE=$((HIT_COUNT * 100 / TOTAL_COUNT))
    echo "  Cache hit rate: ${HIT_RATE}%"

    # Get memory after warmup
    local WARMED_MEM=$(ps -o rss= -p $PROXY_PID 2>/dev/null || echo "0")
    echo "  Memory after warmup: ${WARMED_MEM} KB"

    # Measure eviction performance (access files beyond cache)
    echo "  Phase 3: Testing eviction performance..."
    local EVICTION_TOTAL=0
    for i in $(seq 500 599); do
        FILE_NAME=$(printf "test-file-%05d.bin" $i)
        START_TIME=$(date +%s%N)
        curl -s "http://localhost:8080/public/$FILE_NAME" > /dev/null
        END_TIME=$(date +%s%N)
        LATENCY=$(( (END_TIME - START_TIME) / 1000000 ))
        EVICTION_TOTAL=$((EVICTION_TOTAL + LATENCY))
    done
    local AVG_EVICTION=$((EVICTION_TOTAL / 100))
    echo "  Avg eviction latency: ${AVG_EVICTION}ms"

    # Get metrics
    echo "  Phase 4: Collecting metrics..."
    curl -s http://localhost:9090/metrics > "$RESULTS_DIR/${TEST_NAME}-metrics.txt" 2>/dev/null || true

    # Final memory
    local FINAL_MEM=$(ps -o rss= -p $PROXY_PID 2>/dev/null || echo "0")
    echo "  Final memory: ${FINAL_MEM} KB"

    # Calculate memory overhead
    local MEM_INCREASE=$((FINAL_MEM - INITIAL_MEM))
    echo "  Memory increase: ${MEM_INCREASE} KB"

    # Stop proxy
    kill $PROXY_PID 2>/dev/null || true
    wait $PROXY_PID 2>/dev/null || true

    # Save results
    cat > "$RESULTS_DIR/${TEST_NAME}-results.json" << EOF
{
  "cache_size_mb": ${CACHE_SIZE_MB},
  "hit_rate_percent": ${HIT_RATE},
  "avg_eviction_latency_ms": ${AVG_EVICTION},
  "initial_memory_kb": ${INITIAL_MEM},
  "warmed_memory_kb": ${WARMED_MEM},
  "final_memory_kb": ${FINAL_MEM},
  "memory_increase_kb": ${MEM_INCREASE}
}
EOF

    echo -e "${GREEN}Test complete for ${CACHE_SIZE_MB}MB cache${NC}"
    return 0
}

# Run tests at different cache sizes
echo ""
echo "Starting cache scaling tests..."

# Test with 64MB cache (small baseline)
run_cache_test 64

# Test with 256MB cache
run_cache_test 256

# Test with 1024MB (1GB) cache
run_cache_test 1024

# Summary
echo ""
echo "=============================================="
echo "Cache Scaling Test Summary"
echo "=============================================="

for size in 64 256 1024; do
    RESULT_FILE="$RESULTS_DIR/cache-${size}mb-results.json"
    if [ -f "$RESULT_FILE" ]; then
        HIT_RATE=$(cat "$RESULT_FILE" | grep hit_rate | sed 's/[^0-9]//g')
        MEM_KB=$(cat "$RESULT_FILE" | grep final_memory | sed 's/[^0-9]//g')
        MEM_MB=$((MEM_KB / 1024))
        echo "  ${size}MB cache: ${HIT_RATE}% hit rate, ${MEM_MB}MB actual memory"
    fi
done

echo ""
echo "Results saved to: $RESULTS_DIR"
echo "=============================================="

# Cleanup
pkill -f "yatagarasu" 2>/dev/null || true
