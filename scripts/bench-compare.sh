#!/bin/bash
# Compare disk cache performance: macOS (TokioFsBackend) vs Linux (UringBackend)

set -e

echo "====================================================================="
echo "Disk Cache Performance Comparison"
echo "====================================================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Run macOS benchmarks (TokioFsBackend)
echo -e "${BLUE}[1/3] Running benchmarks on macOS (TokioFsBackend)...${NC}"
echo ""
cargo bench --bench disk_cache -- --save-baseline macos-tokio 2>&1 | grep -E "(time:|Found|Benchmarking)"
echo ""

# Step 2: Build Linux Docker image
echo -e "${BLUE}[2/3] Building Linux Docker image with UringBackend...${NC}"
echo ""
docker-compose -f docker/docker-compose.bench.yml build benchmarks
echo ""

# Step 3: Run Linux benchmarks (UringBackend)
echo -e "${BLUE}[3/3] Running benchmarks on Linux (UringBackend)...${NC}"
echo ""
docker-compose -f docker/docker-compose.bench.yml run --rm benchmarks \
    cargo bench --bench disk_cache -- --save-baseline linux-uring 2>&1 | grep -E "(time:|Found|Benchmarking)"
echo ""

# Summary
echo "====================================================================="
echo -e "${GREEN}Benchmark Comparison Complete!${NC}"
echo "====================================================================="
echo ""
echo -e "${YELLOW}Results:${NC}"
echo "  - macOS (TokioFsBackend):  target/criterion/*/macos-tokio/"
echo "  - Linux (UringBackend):    target/criterion/*/linux-uring/"
echo ""
echo -e "${YELLOW}View HTML Reports:${NC}"
echo "  open target/criterion/report/index.html"
echo ""
echo -e "${YELLOW}Compare Baselines:${NC}"
echo "  cargo bench --bench disk_cache -- --baseline macos-tokio"
echo ""
