#!/bin/bash
#
# Phase 40: Graceful Shutdown Test Script
#
# This script tests graceful shutdown behavior:
# - SIGTERM handling while serving concurrent connections
# - All in-flight requests complete without errors
# - No broken pipes or connection resets
#
# Prerequisites:
#   - Yatagarasu proxy binary built (cargo build --release)
#   - MinIO running with test files
#
# Usage:
#   ./scripts/test-graceful-shutdown.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="${PROJECT_ROOT}/config/loadtest/config.k6test.yaml"
PROXY_BINARY="${PROJECT_ROOT}/target/release/yatagarasu"
LOG_FILE="/tmp/yatagarasu-shutdown-test.log"
RESULTS_FILE="/tmp/graceful-shutdown-results.txt"

# Test parameters
NUM_CONCURRENT_REQUESTS=100
REQUEST_DURATION_SECONDS=10

echo "========================================"
echo " Phase 40: Graceful Shutdown Tests"
echo "========================================"
echo ""

# Check prerequisites
if [ ! -f "$PROXY_BINARY" ]; then
    echo -e "${RED}Error: Proxy binary not found at $PROXY_BINARY${NC}"
    echo "Run: cargo build --release"
    exit 1
fi

# Clean up any existing proxy processes
echo "Cleaning up existing processes..."
pkill -9 -f yatagarasu 2>/dev/null || true
sleep 2

# Clear results file
> "$RESULTS_FILE"

#
# Test 1: Basic SIGTERM handling
#
echo ""
echo -e "${YELLOW}Test 1: Basic SIGTERM Handling${NC}"
echo "Starting proxy..."

"$PROXY_BINARY" --config "$CONFIG_FILE" > "$LOG_FILE" 2>&1 &
PROXY_PID=$!
echo "Proxy started with PID: $PROXY_PID"
sleep 3

# Verify proxy is healthy
if curl -sf http://localhost:8080/health > /dev/null; then
    echo -e "${GREEN}✓ Proxy is healthy${NC}"
else
    echo -e "${RED}✗ Proxy health check failed${NC}"
    exit 1
fi

# Send SIGTERM
echo "Sending SIGTERM to proxy..."
kill -TERM $PROXY_PID

# Wait for graceful shutdown (max 10 seconds)
WAIT_COUNT=0
while kill -0 $PROXY_PID 2>/dev/null; do
    sleep 1
    WAIT_COUNT=$((WAIT_COUNT + 1))
    if [ $WAIT_COUNT -ge 10 ]; then
        echo -e "${RED}✗ Proxy did not shut down within 10 seconds${NC}"
        kill -9 $PROXY_PID 2>/dev/null || true
        echo "TEST1_RESULT=FAIL" >> "$RESULTS_FILE"
        break
    fi
done

if [ $WAIT_COUNT -lt 10 ]; then
    echo -e "${GREEN}✓ Proxy shut down gracefully in ${WAIT_COUNT}s${NC}"
    echo "TEST1_RESULT=PASS" >> "$RESULTS_FILE"
fi

#
# Test 2: SIGTERM with concurrent slow requests
#
echo ""
echo -e "${YELLOW}Test 2: SIGTERM During Active Slow Downloads${NC}"

# Start proxy fresh
"$PROXY_BINARY" --config "$CONFIG_FILE" > "$LOG_FILE" 2>&1 &
PROXY_PID=$!
echo "Proxy started with PID: $PROXY_PID"
sleep 3

# Verify proxy is healthy
if ! curl -sf http://localhost:8080/health > /dev/null; then
    echo -e "${RED}✗ Proxy health check failed${NC}"
    exit 1
fi

# Start multiple concurrent downloads of large file in background
echo "Starting $NUM_CONCURRENT_REQUESTS concurrent large file downloads..."
CURL_PIDS=()
CURL_RESULTS_DIR="/tmp/graceful-shutdown-curls"
rm -rf "$CURL_RESULTS_DIR"
mkdir -p "$CURL_RESULTS_DIR"

for i in $(seq 1 $NUM_CONCURRENT_REQUESTS); do
    # Use 10MB file with rate limiting to simulate slow download
    curl -sf --limit-rate 500K \
        -o /dev/null \
        -w "request_$i: http_code=%{http_code} time=%{time_total}s size=%{size_download}\n" \
        "http://localhost:8080/public/test-10mb.bin" \
        > "$CURL_RESULTS_DIR/result_$i.txt" 2>&1 &
    CURL_PIDS+=($!)
done

# Wait a moment for requests to start
sleep 2

# Count active requests
ACTIVE_REQUESTS=$(pgrep -c -f "curl.*localhost:8080" 2>/dev/null || echo "0")
echo "Active curl requests: $ACTIVE_REQUESTS"

# Send SIGTERM while requests are in-flight
echo "Sending SIGTERM while $ACTIVE_REQUESTS requests are in-flight..."
kill -TERM $PROXY_PID

# Wait for all curl requests to complete
echo "Waiting for all curl requests to complete..."
CURL_FAILURES=0
for pid in "${CURL_PIDS[@]}"; do
    wait $pid 2>/dev/null || CURL_FAILURES=$((CURL_FAILURES + 1))
done

# Wait for proxy to fully shut down
WAIT_COUNT=0
while kill -0 $PROXY_PID 2>/dev/null; do
    sleep 1
    WAIT_COUNT=$((WAIT_COUNT + 1))
    if [ $WAIT_COUNT -ge 30 ]; then
        echo -e "${RED}✗ Proxy did not shut down within 30 seconds${NC}"
        kill -9 $PROXY_PID 2>/dev/null || true
        break
    fi
done

# Analyze results
SUCCESSFUL_REQUESTS=0
FAILED_REQUESTS=0
CONNECTION_RESET=0

for result_file in "$CURL_RESULTS_DIR"/*.txt; do
    if [ -f "$result_file" ]; then
        content=$(cat "$result_file")
        if echo "$content" | grep -q "http_code=200"; then
            SUCCESSFUL_REQUESTS=$((SUCCESSFUL_REQUESTS + 1))
        elif echo "$content" | grep -qi "reset\|broken\|aborted"; then
            CONNECTION_RESET=$((CONNECTION_RESET + 1))
            FAILED_REQUESTS=$((FAILED_REQUESTS + 1))
        else
            FAILED_REQUESTS=$((FAILED_REQUESTS + 1))
        fi
    fi
done

echo ""
echo "Results:"
echo "  Total requests: $NUM_CONCURRENT_REQUESTS"
echo "  Successful: $SUCCESSFUL_REQUESTS"
echo "  Failed: $FAILED_REQUESTS"
echo "  Connection resets: $CONNECTION_RESET"
echo "  Shutdown time: ${WAIT_COUNT}s"

# Success criteria: >95% success rate, 0 connection resets
SUCCESS_RATE=$(echo "scale=2; $SUCCESSFUL_REQUESTS * 100 / $NUM_CONCURRENT_REQUESTS" | bc)
if [ $CONNECTION_RESET -eq 0 ] && [ $(echo "$SUCCESS_RATE >= 95" | bc) -eq 1 ]; then
    echo -e "${GREEN}✓ Test 2 PASSED: ${SUCCESS_RATE}% success rate, no connection resets${NC}"
    echo "TEST2_RESULT=PASS" >> "$RESULTS_FILE"
    echo "TEST2_SUCCESS_RATE=$SUCCESS_RATE" >> "$RESULTS_FILE"
else
    echo -e "${RED}✗ Test 2 FAILED: ${SUCCESS_RATE}% success rate, $CONNECTION_RESET connection resets${NC}"
    echo "TEST2_RESULT=FAIL" >> "$RESULTS_FILE"
fi

#
# Test 3: Rapid SIGTERM (no grace period)
#
echo ""
echo -e "${YELLOW}Test 3: Multiple Shutdown Cycles${NC}"

CYCLE_FAILURES=0
for cycle in $(seq 1 5); do
    echo "Cycle $cycle/5..."

    # Start proxy
    "$PROXY_BINARY" --config "$CONFIG_FILE" > "$LOG_FILE" 2>&1 &
    PROXY_PID=$!
    sleep 2

    # Start some requests
    for i in $(seq 1 10); do
        curl -sf -o /dev/null "http://localhost:8080/public/test-1kb.txt" &
    done

    # Immediate SIGTERM
    sleep 0.5
    kill -TERM $PROXY_PID

    # Wait for shutdown
    WAIT_COUNT=0
    while kill -0 $PROXY_PID 2>/dev/null; do
        sleep 0.5
        WAIT_COUNT=$((WAIT_COUNT + 1))
        if [ $WAIT_COUNT -ge 20 ]; then
            kill -9 $PROXY_PID 2>/dev/null || true
            CYCLE_FAILURES=$((CYCLE_FAILURES + 1))
            break
        fi
    done

    # Wait for all background curl processes
    wait 2>/dev/null || true
done

if [ $CYCLE_FAILURES -eq 0 ]; then
    echo -e "${GREEN}✓ Test 3 PASSED: All 5 shutdown cycles completed gracefully${NC}"
    echo "TEST3_RESULT=PASS" >> "$RESULTS_FILE"
else
    echo -e "${RED}✗ Test 3 FAILED: $CYCLE_FAILURES cycles required force kill${NC}"
    echo "TEST3_RESULT=FAIL" >> "$RESULTS_FILE"
fi

#
# Test 4: Verify no broken pipes during streaming shutdown
#
echo ""
echo -e "${YELLOW}Test 4: Streaming Shutdown (100MB file)${NC}"

# Start proxy
"$PROXY_BINARY" --config "$CONFIG_FILE" > "$LOG_FILE" 2>&1 &
PROXY_PID=$!
sleep 3

# Start a large file download (streaming)
echo "Starting 100MB file download..."
STREAM_RESULT_FILE="/tmp/stream_result.txt"
curl -sf \
    -o /dev/null \
    -w "http_code=%{http_code} time=%{time_total}s size=%{size_download}\n" \
    "http://localhost:8080/public/test-100mb.bin" \
    > "$STREAM_RESULT_FILE" 2>&1 &
STREAM_PID=$!

# Wait for download to start
sleep 3

# Check download is in progress
if ! kill -0 $STREAM_PID 2>/dev/null; then
    echo "Download completed quickly (from cache or fast network)"
else
    echo "Download in progress, sending SIGTERM..."
    kill -TERM $PROXY_PID

    # Wait for download to complete
    wait $STREAM_PID 2>/dev/null
    STREAM_EXIT=$?
fi

# Wait for proxy shutdown
WAIT_COUNT=0
while kill -0 $PROXY_PID 2>/dev/null; do
    sleep 1
    WAIT_COUNT=$((WAIT_COUNT + 1))
    if [ $WAIT_COUNT -ge 30 ]; then
        kill -9 $PROXY_PID 2>/dev/null || true
        break
    fi
done

# Check result
if [ -f "$STREAM_RESULT_FILE" ]; then
    STREAM_RESULT=$(cat "$STREAM_RESULT_FILE")
    if echo "$STREAM_RESULT" | grep -q "http_code=200"; then
        echo -e "${GREEN}✓ Test 4 PASSED: Streaming download completed successfully${NC}"
        echo "TEST4_RESULT=PASS" >> "$RESULTS_FILE"
    else
        echo -e "${RED}✗ Test 4 FAILED: Streaming download failed: $STREAM_RESULT${NC}"
        echo "TEST4_RESULT=FAIL" >> "$RESULTS_FILE"
    fi
else
    echo -e "${RED}✗ Test 4 FAILED: No result file${NC}"
    echo "TEST4_RESULT=FAIL" >> "$RESULTS_FILE"
fi

#
# Summary
#
echo ""
echo "========================================"
echo " Test Summary"
echo "========================================"

TOTAL_PASS=0
TOTAL_FAIL=0

while IFS='=' read -r key value; do
    if [[ $key == *"_RESULT" ]]; then
        TEST_NAME=$(echo "$key" | sed 's/_RESULT//')
        if [ "$value" = "PASS" ]; then
            echo -e "${GREEN}✓ $TEST_NAME: PASS${NC}"
            TOTAL_PASS=$((TOTAL_PASS + 1))
        else
            echo -e "${RED}✗ $TEST_NAME: FAIL${NC}"
            TOTAL_FAIL=$((TOTAL_FAIL + 1))
        fi
    fi
done < "$RESULTS_FILE"

echo ""
echo "Total: $TOTAL_PASS passed, $TOTAL_FAIL failed"
echo ""

# Cleanup
rm -rf "$CURL_RESULTS_DIR" 2>/dev/null || true
pkill -9 -f yatagarasu 2>/dev/null || true

if [ $TOTAL_FAIL -eq 0 ]; then
    echo -e "${GREEN}All graceful shutdown tests PASSED!${NC}"
    exit 0
else
    echo -e "${RED}Some graceful shutdown tests FAILED${NC}"
    exit 1
fi
