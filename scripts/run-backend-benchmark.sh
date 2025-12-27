#!/bin/bash
# Backend Comparison Benchmark Script
# Compares MinIO vs RustFS performance through Yatagarasu proxy
#
# Usage:
#   ./scripts/run-backend-benchmark.sh           # Run full comparison
#   ./scripts/run-backend-benchmark.sh minio     # MinIO only
#   ./scripts/run-backend-benchmark.sh rustfs    # RustFS only
#   ./scripts/run-backend-benchmark.sh quick     # Quick test (10KB, 1MB only)
#
# Requirements:
#   - Docker and Docker Compose
#   - k6 (https://k6.io/docs/getting-started/installation/)
#
# Output:
#   Results saved to benchmark-results/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$PROJECT_DIR/benchmark-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DURATION=${DURATION:-30s}
VUS=${VUS:-10}
FILE_SIZES=${FILE_SIZES:-"10kb 1mb 10mb 100mb 1gb"}

# Parse arguments
MODE=${1:-full}

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║          YATAGARASU S3 BACKEND BENCHMARK                            ║"
echo "║          MinIO vs RustFS Comparison                                 ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check prerequisites
check_prerequisites() {
    echo -e "${YELLOW}Checking prerequisites...${NC}"

    if ! command -v docker &> /dev/null; then
        echo -e "${RED}Error: Docker is not installed${NC}"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo -e "${RED}Error: Docker Compose is not installed${NC}"
        exit 1
    fi

    if ! command -v k6 &> /dev/null; then
        echo -e "${RED}Error: k6 is not installed${NC}"
        echo "Install with: brew install k6 (macOS) or see https://k6.io/docs/getting-started/installation/"
        exit 1
    fi

    echo -e "${GREEN}All prerequisites met!${NC}"
}

# Create results directory
setup_results_dir() {
    mkdir -p "$RESULTS_DIR"
    echo -e "${BLUE}Results will be saved to: $RESULTS_DIR${NC}"
}

# Build Yatagarasu image
build_proxy() {
    echo -e "${YELLOW}Building Yatagarasu proxy image...${NC}"
    cd "$PROJECT_DIR"
    docker build -t yatagarasu:benchmark -f Dockerfile . > /dev/null 2>&1
    echo -e "${GREEN}Proxy image built!${NC}"
}

# Run benchmark for a specific backend
run_benchmark() {
    local backend=$1
    local compose_file="$PROJECT_DIR/docker/docker-compose.benchmark-${backend}.yml"
    local result_file="$RESULTS_DIR/${backend}_${TIMESTAMP}.json"

    echo ""
    local backend_upper=$(echo "$backend" | tr '[:lower:]' '[:upper:]')
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Benchmarking: ${backend_upper}${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"

    # Start services
    echo -e "${YELLOW}Starting ${backend} services...${NC}"
    docker-compose -f "$compose_file" up -d

    # Wait for services to be healthy
    echo -e "${YELLOW}Waiting for services to be ready...${NC}"
    sleep 5

    # Wait for setup to complete (file uploads)
    local setup_container="benchmark-${backend}-setup"
    if [ "$backend" = "minio" ]; then
        setup_container="benchmark-minio-setup"
    else
        setup_container="benchmark-rustfs-setup"
    fi

    echo -e "${YELLOW}Waiting for file uploads to complete...${NC}"
    local max_wait=300
    local waited=0
    while docker ps -a --filter "name=${setup_container}" --filter "status=running" | grep -q "${setup_container}"; do
        sleep 5
        waited=$((waited + 5))
        if [ $waited -ge $max_wait ]; then
            echo -e "${RED}Timeout waiting for setup to complete${NC}"
            break
        fi
        echo "  Still uploading files... (${waited}s)"
    done

    # Verify proxy is healthy
    echo -e "${YELLOW}Verifying proxy is healthy...${NC}"
    local retries=0
    while ! curl -sf http://localhost:8080/health > /dev/null 2>&1; do
        sleep 2
        retries=$((retries + 1))
        if [ $retries -ge 30 ]; then
            echo -e "${RED}Proxy failed to become healthy${NC}"
            docker-compose -f "$compose_file" logs yatagarasu
            docker-compose -f "$compose_file" down -v
            return 1
        fi
    done
    echo -e "${GREEN}Proxy is healthy!${NC}"

    # Run k6 benchmark
    echo -e "${YELLOW}Running k6 benchmark...${NC}"
    echo ""

    if [ "$MODE" = "quick" ]; then
        # Quick mode: only 10KB and 1MB
        k6 run \
            -e FILE_SIZE=10kb \
            -e DURATION=15s \
            -e VUS=$VUS \
            --out json="$RESULTS_DIR/${backend}_10kb_${TIMESTAMP}.json" \
            "$PROJECT_DIR/k6/backend-comparison.js"

        k6 run \
            -e FILE_SIZE=1mb \
            -e DURATION=15s \
            -e VUS=$VUS \
            --out json="$RESULTS_DIR/${backend}_1mb_${TIMESTAMP}.json" \
            "$PROJECT_DIR/k6/backend-comparison.js"
    else
        # Full benchmark: all file sizes
        k6 run \
            -e FILE_SIZE=all \
            -e DURATION=$DURATION \
            -e VUS=$VUS \
            --out json="$result_file" \
            "$PROJECT_DIR/k6/backend-comparison.js"
    fi

    echo ""
    echo -e "${GREEN}Benchmark for ${backend} complete!${NC}"

    # Cleanup
    echo -e "${YELLOW}Cleaning up ${backend} services...${NC}"
    docker-compose -f "$compose_file" down -v > /dev/null 2>&1
    echo -e "${GREEN}Cleanup complete!${NC}"
}

# Generate comparison report
generate_report() {
    local minio_file="$RESULTS_DIR/minio_${TIMESTAMP}.json"
    local rustfs_file="$RESULTS_DIR/rustfs_${TIMESTAMP}.json"
    local report_file="$RESULTS_DIR/comparison_${TIMESTAMP}.md"

    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Generating Comparison Report${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"

    cat > "$report_file" << EOF
# S3 Backend Comparison: MinIO vs RustFS

**Date:** $(date)
**Duration:** ${DURATION} per file size
**Virtual Users:** ${VUS}
**File Sizes:** 10KB, 1MB, 10MB, 100MB, 1GB

## Summary

| Metric | MinIO | RustFS | Winner |
|--------|-------|--------|--------|
| 10KB Throughput | - | - | - |
| 1MB Throughput | - | - | - |
| 10MB Throughput | - | - | - |
| 100MB Throughput | - | - | - |
| 1GB Throughput | - | - | - |

## Raw Results

### MinIO Results
\`\`\`
$(cat "$minio_file" 2>/dev/null || echo "No results file found")
\`\`\`

### RustFS Results
\`\`\`
$(cat "$rustfs_file" 2>/dev/null || echo "No results file found")
\`\`\`

## Notes

- RustFS claims ~2.3x faster throughput for small objects (4KB)
- Larger files may show smaller differences due to network/disk IO limits
- Cache was disabled on Yatagarasu to measure pure backend performance

## How to Interpret

- **Lower duration** = Better (faster response times)
- **Higher throughput_mbps** = Better (more data transferred)
- **Lower ttfb** = Better (faster time to first byte)
- **Lower errors** = Better (more reliable)

EOF

    echo -e "${GREEN}Report saved to: $report_file${NC}"
}

# Main execution
main() {
    check_prerequisites
    setup_results_dir

    case $MODE in
        minio)
            build_proxy
            run_benchmark "minio"
            ;;
        rustfs)
            build_proxy
            run_benchmark "rustfs"
            ;;
        quick)
            build_proxy
            echo -e "${YELLOW}Running quick benchmark (10KB & 1MB only)...${NC}"
            run_benchmark "minio"
            run_benchmark "rustfs"
            generate_report
            ;;
        full|*)
            build_proxy
            run_benchmark "minio"
            run_benchmark "rustfs"
            generate_report
            ;;
    esac

    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║  BENCHMARK COMPLETE!                                               ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Results saved to: $RESULTS_DIR"
    echo ""
}

main "$@"
