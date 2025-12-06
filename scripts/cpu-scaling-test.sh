#!/bin/bash
#
# Phase 58: CPU Core Scaling Test Script
#
# This script runs the Yatagarasu proxy with different CPU core limits
# and measures throughput at each level to determine scaling characteristics.
#
# Prerequisites:
#   - Docker installed and running
#   - MinIO running on localhost:9000
#   - k6 installed
#   - Test files uploaded to MinIO (test-1kb.txt, test-10kb.txt)
#
# Usage:
#   ./scripts/cpu-scaling-test.sh
#
# Output:
#   Results are saved to cpu-scaling-results.md
#

set -e

# Configuration
PROXY_IMAGE="yatagarasu:latest"
PROXY_PORT=8080
MINIO_HOST="host.docker.internal"  # Access host's MinIO from Docker
RESULTS_FILE="cpu-scaling-results.md"
SCENARIO="${SCENARIO:-find_max}"

# CPU core counts to test (adjust based on your machine)
CORE_COUNTS=(1 2 4 8)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "============================================================"
echo "Phase 58: CPU Core Scaling Test"
echo "============================================================"
echo ""

# Check prerequisites
command -v docker >/dev/null 2>&1 || { echo -e "${RED}Error: docker is required${NC}"; exit 1; }
command -v k6 >/dev/null 2>&1 || { echo -e "${RED}Error: k6 is required${NC}"; exit 1; }

# Check if Docker image exists, if not build it
if ! docker image inspect "$PROXY_IMAGE" >/dev/null 2>&1; then
    echo -e "${YELLOW}Building Docker image...${NC}"
    docker build -t "$PROXY_IMAGE" .
fi

# Initialize results file
cat > "$RESULTS_FILE" << 'EOF'
# Phase 58: CPU Core Scaling Test Results

**Date**: $(date -Iseconds)
**Test Scenario**: find_max (ramping to find max sustainable RPS)

## Results Summary

| Cores | Max RPS | P95 Latency | P99 Latency | Error Rate | Notes |
|-------|---------|-------------|-------------|------------|-------|
EOF

# Replace date placeholder
sed -i.bak "s/\$(date -Iseconds)/$(date -Iseconds)/" "$RESULTS_FILE" && rm -f "$RESULTS_FILE.bak"

run_test() {
    local cores=$1
    echo ""
    echo -e "${GREEN}============================================================${NC}"
    echo -e "${GREEN}Testing with $cores CPU core(s)${NC}"
    echo -e "${GREEN}============================================================${NC}"

    # Stop any existing container
    docker rm -f yatagarasu-scaling-test 2>/dev/null || true

    # Create a minimal config for testing
    local config_file=$(mktemp)
    cat > "$config_file" << EOF
server:
  address: "0.0.0.0"
  port: 8080

buckets:
  - name: public
    path_prefix: /public
    s3:
      endpoint: http://${MINIO_HOST}:9000
      bucket: test-bucket
      region: us-east-1
      access_key: minioadmin
      secret_key: minioadmin
    auth:
      enabled: false

cache:
  enabled: true
  memory:
    enabled: true
    max_cache_size_mb: 256
    max_item_size_mb: 10
    default_ttl_seconds: 3600
EOF

    # Start proxy container with CPU limit
    echo "Starting proxy with --cpus=$cores..."
    docker run -d \
        --name yatagarasu-scaling-test \
        --cpus="$cores" \
        -p "$PROXY_PORT:8080" \
        -v "$config_file:/app/config.yaml:ro" \
        --add-host=host.docker.internal:host-gateway \
        "$PROXY_IMAGE" \
        --config /app/config.yaml

    # Wait for proxy to be ready
    echo "Waiting for proxy to be ready..."
    local max_wait=30
    local waited=0
    while ! curl -s "http://localhost:$PROXY_PORT/health" >/dev/null 2>&1; do
        sleep 1
        waited=$((waited + 1))
        if [ $waited -ge $max_wait ]; then
            echo -e "${RED}Proxy failed to start within ${max_wait}s${NC}"
            docker logs yatagarasu-scaling-test
            docker rm -f yatagarasu-scaling-test
            rm -f "$config_file"
            return 1
        fi
    done
    echo "Proxy is ready."

    # Run k6 test
    echo "Running k6 load test..."
    local k6_output=$(mktemp)

    k6 run \
        -e SCENARIO="$SCENARIO" \
        -e CORES="$cores" \
        -e BASE_URL="http://localhost:$PROXY_PORT" \
        --summary-export="$k6_output.json" \
        k6/cpu-scaling.js 2>&1 | tee "$k6_output"

    # Extract metrics from k6 output
    local rps=$(grep -oP 'http_reqs[^:]*:\s*\K[\d.]+(?=/s)' "$k6_output" | head -1 || echo "N/A")
    local p95=$(grep -oP 'http_req_duration[^:]*p\(95\)=\K[\d.]+' "$k6_output" | head -1 || echo "N/A")
    local p99=$(grep -oP 'http_req_duration[^:]*p\(99\)=\K[\d.]+' "$k6_output" | head -1 || echo "N/A")
    local errors=$(grep -oP 'http_req_failed[^:]*:\s*\K[\d.]+%' "$k6_output" | head -1 || echo "N/A")

    # Append to results
    echo "| $cores | $rps | ${p95}ms | ${p99}ms | $errors | - |" >> "$RESULTS_FILE"

    echo ""
    echo -e "${GREEN}Results for $cores core(s):${NC}"
    echo "  RPS: $rps"
    echo "  P95: ${p95}ms"
    echo "  P99: ${p99}ms"
    echo "  Errors: $errors"

    # Cleanup
    docker rm -f yatagarasu-scaling-test
    rm -f "$config_file" "$k6_output" "$k6_output.json"
}

# Run tests for each core count
for cores in "${CORE_COUNTS[@]}"; do
    run_test "$cores"
done

# Add analysis section to results
cat >> "$RESULTS_FILE" << 'EOF'

## Analysis

### Scaling Efficiency

Scaling efficiency = (RPS at N cores) / (RPS at 1 core × N)

| Cores | Scaling Efficiency |
|-------|-------------------|
| 1     | 100% (baseline)   |
| 2     | TBD               |
| 4     | TBD               |
| 8     | TBD               |

### Observations

- **Linear scaling range**: Cores 1-N show near-linear scaling
- **Diminishing returns**: Above N cores, efficiency drops
- **Bottleneck identified**: [CPU/Memory/Network/...]

### Recommendations

Based on these results:
- For workloads up to X RPS: Use N cores
- For workloads up to Y RPS: Use M cores
- Maximum effective core count: Z cores

## Thread Pool Analysis

Tokio runtime uses work-stealing across threads. Observations:
- Work stealing effectiveness: TBD
- Thread pool starvation: None observed / Observed at X RPS

EOF

echo ""
echo "============================================================"
echo "Phase 58: CPU Core Scaling Test Complete"
echo "============================================================"
echo ""
echo "Results saved to: $RESULTS_FILE"
echo ""
echo "Next steps:"
echo "1. Review $RESULTS_FILE"
echo "2. Calculate scaling efficiency"
echo "3. Update plan_v1.2.md with results"
