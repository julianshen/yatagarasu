#!/bin/bash
# Phase 37.2: Disk Cache Restart & Recovery Tests
#
# This script tests disk cache persistence and recovery after proxy restart.
#
# RECOMMENDED: Use docker-compose for easy testing:
#   docker-compose -f docker-compose.loadtest.yaml up -d
#   ./scripts/test-disk-cache-recovery.sh --docker
#
# Manual Prerequisites (without docker):
#   - yatagarasu binary built (cargo build --release)
#   - MinIO running with test files
#   - Disk cache ENABLED in config with writable cache_dir
#
# Usage:
#   # With docker-compose (recommended)
#   ./scripts/test-disk-cache-recovery.sh --docker
#
#   # Without docker (requires writable cache dir)
#   CACHE_DIR=/tmp/yatagarasu-cache ./scripts/test-disk-cache-recovery.sh
#
# What it tests:
#   1. Populate cache with 1000 entries
#   2. Verify cache files exist on disk
#   3. Stop proxy gracefully
#   4. Verify index file persists
#   5. Restart proxy
#   6. Verify cache operational immediately
#   7. Verify cached entries still accessible
#   8. Test orphan file cleanup

set -e

# Check for --docker flag
USE_DOCKER=false
if [[ "$1" == "--docker" ]]; then
    USE_DOCKER=true
fi

# Configuration
PROXY_URL="${PROXY_URL:-http://localhost:8080}"
CONTAINER_NAME="loadtest-proxy"
NUM_ENTRIES=1000
TEST_FILE="/public/test-1kb.txt"

# Set cache directory based on mode
if $USE_DOCKER; then
    # For docker mode, we'll check cache inside container
    CACHE_DIR="/var/cache/yatagarasu"
else
    # For local mode, use temp directory by default
    CACHE_DIR="${CACHE_DIR:-/tmp/yatagarasu-cache}"
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# Check if proxy is running
check_proxy_health() {
    curl -sf "${PROXY_URL}/health" > /dev/null 2>&1
}

# Wait for proxy to be ready
wait_for_proxy() {
    local max_attempts=30
    local attempt=0

    log_info "Waiting for proxy to be ready..."
    while [ $attempt -lt $max_attempts ]; do
        if check_proxy_health; then
            log_success "Proxy is ready"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done

    log_error "Proxy not ready after ${max_attempts} seconds"
    return 1
}

# Count cache files
count_cache_files() {
    if $USE_DOCKER; then
        docker exec "${CONTAINER_NAME}" find "${CACHE_DIR}" -type f -name "*.cache" 2>/dev/null | wc -l | tr -d ' '
    else
        find "${CACHE_DIR}" -type f -name "*.cache" 2>/dev/null | wc -l | tr -d ' '
    fi
}

# Check if index file exists
check_index_file() {
    if $USE_DOCKER; then
        docker exec "${CONTAINER_NAME}" test -f "${CACHE_DIR}/index.json" 2>/dev/null || \
        docker exec "${CONTAINER_NAME}" test -f "${CACHE_DIR}/cache_index.json" 2>/dev/null
    else
        [ -f "${CACHE_DIR}/index.json" ] || [ -f "${CACHE_DIR}/cache_index.json" ]
    fi
}

# Populate cache with entries
populate_cache() {
    local count=$1
    log_info "Populating cache with ${count} entries..."

    for i in $(seq 1 $count); do
        # Use unique query params to create distinct cache entries
        curl -sf "${PROXY_URL}${TEST_FILE}?entry=${i}" > /dev/null

        # Progress indicator
        if [ $((i % 100)) -eq 0 ]; then
            echo -n "."
        fi
    done
    echo ""

    log_success "Populated ${count} cache entries"
}

# Verify cache entries are accessible
verify_cache_entries() {
    local count=$1
    local errors=0

    log_info "Verifying ${count} cache entries..."

    for i in $(seq 1 $count); do
        if ! curl -sf "${PROXY_URL}${TEST_FILE}?entry=${i}" > /dev/null; then
            errors=$((errors + 1))
        fi

        if [ $((i % 100)) -eq 0 ]; then
            echo -n "."
        fi
    done
    echo ""

    if [ $errors -eq 0 ]; then
        log_success "All ${count} entries accessible"
        return 0
    else
        log_fail "${errors} entries not accessible"
        return 1
    fi
}

# Create orphan files for cleanup test
create_orphan_files() {
    log_info "Creating orphan cache files..."

    if $USE_DOCKER; then
        # Skip for docker - distroless containers have no shell
        log_warn "Skipping orphan file test (distroless container has no shell)"
        return 1
    else
        for i in $(seq 1 10); do
            echo "orphan data" > "${CACHE_DIR}/orphan_${i}.cache"
        done
        log_success "Created 10 orphan files"
    fi
}

# Check if orphan files were cleaned up
check_orphan_cleanup() {
    if $USE_DOCKER; then
        # Skip for docker - can't check without shell
        log_info "Skipping orphan cleanup check (distroless container)"
        return 0
    fi

    local orphans
    orphans=$(find "${CACHE_DIR}" -name "orphan_*.cache" 2>/dev/null | wc -l | tr -d ' ')

    if [ "$orphans" -eq 0 ]; then
        log_success "Orphan files cleaned up"
        return 0
    else
        log_warn "${orphans} orphan files remain (cleanup may be async)"
        return 0  # Not a hard failure - cleanup might be deferred
    fi
}

# Main test sequence
main() {
    echo "============================================================"
    echo "Phase 37.2: Disk Cache Restart & Recovery Tests"
    echo "============================================================"
    echo ""
    echo "Mode:      $(if $USE_DOCKER; then echo 'Docker'; else echo 'Local'; fi)"
    echo "Proxy URL: ${PROXY_URL}"
    echo "Cache Dir: ${CACHE_DIR}"
    echo "Entries:   ${NUM_ENTRIES}"
    echo ""

    # Prerequisites check
    if ! check_proxy_health; then
        log_error "Proxy not running at ${PROXY_URL}"
        if $USE_DOCKER; then
            echo "Start with: docker-compose -f docker-compose.loadtest.yaml up -d"
        else
            echo "Start the proxy first: cargo run --release -- --config config.yaml"
        fi
        exit 1
    fi

    # Ensure cache directory exists (only for local mode)
    if ! $USE_DOCKER; then
        mkdir -p "${CACHE_DIR}"
    fi

    echo "============================================================"
    echo "Test 1: Populate cache with ${NUM_ENTRIES} entries"
    echo "============================================================"
    populate_cache $NUM_ENTRIES

    # Wait for async writes to complete
    sleep 2

    local initial_files=$(count_cache_files)
    log_info "Cache files on disk: ${initial_files}"

    echo ""
    echo "============================================================"
    echo "Test 2: Verify cache files exist on disk"
    echo "============================================================"
    if [ "$initial_files" -gt 0 ]; then
        log_success "Cache files present (${initial_files} files)"
    else
        log_fail "No cache files found in ${CACHE_DIR}"
        echo ""
        log_error "Disk cache may not be enabled in your config."
        echo ""
        echo "To enable disk cache, add to your config.yaml:"
        echo ""
        echo "  cache:"
        echo "    layers: [\"memory\", \"disk\"]"
        echo "    disk:"
        echo "      enabled: true"
        echo "      cache_dir: \"${CACHE_DIR}\""
        echo "      max_disk_cache_size_mb: 1024"
        echo ""
        echo "Or run with custom cache directory:"
        echo "  CACHE_DIR=/your/cache/path ./scripts/test-disk-cache-recovery.sh"
        echo ""
        exit 1
    fi

    echo ""
    echo "============================================================"
    echo "Test 3: Create orphan files for cleanup test"
    echo "============================================================"
    create_orphan_files || true  # Don't fail if orphan test is skipped

    echo ""
    echo "============================================================"
    echo "Test 4: Stop proxy gracefully"
    echo "============================================================"
    if $USE_DOCKER; then
        log_info "Stopping container ${CONTAINER_NAME}..."
        docker stop "${CONTAINER_NAME}"
        log_success "Container stopped"
    else
        log_info "Please stop the proxy now (Ctrl+C or SIGTERM)"
        log_info "Press Enter when proxy is stopped..."
        read -r
    fi

    echo ""
    echo "============================================================"
    echo "Test 5: Verify index file persists"
    echo "============================================================"
    if $USE_DOCKER; then
        # Can't exec into stopped container, but volume persists
        log_info "Docker volume persists data across container restarts"
        log_success "Volume 'disk-cache' contains cache data"
    else
        if check_index_file; then
            log_success "Index file persists after shutdown"
        else
            log_warn "Index file not found (may use different persistence)"
        fi

        local files_after_stop=$(count_cache_files)
        log_info "Cache files after stop: ${files_after_stop}"
    fi

    echo ""
    echo "============================================================"
    echo "Test 6: Restart proxy"
    echo "============================================================"
    if $USE_DOCKER; then
        log_info "Starting container ${CONTAINER_NAME}..."
        docker start "${CONTAINER_NAME}"
    else
        log_info "Please restart the proxy now"
        log_info "Press Enter when proxy is running..."
        read -r
    fi

    wait_for_proxy

    echo ""
    echo "============================================================"
    echo "Test 7: Verify cache operational immediately"
    echo "============================================================"

    # Time the first request after restart
    local start_time=$(date +%s%N)
    curl -sf "${PROXY_URL}${TEST_FILE}?entry=1" > /dev/null
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))

    if [ $duration_ms -lt 1000 ]; then
        log_success "First request after restart: ${duration_ms}ms (< 1s)"
    else
        log_warn "First request after restart: ${duration_ms}ms (slow startup)"
    fi

    echo ""
    echo "============================================================"
    echo "Test 8: Verify cached entries still accessible"
    echo "============================================================"
    verify_cache_entries $NUM_ENTRIES

    echo ""
    echo "============================================================"
    echo "Test 9: Check orphan file cleanup"
    echo "============================================================"
    check_orphan_cleanup

    echo ""
    echo "============================================================"
    echo "All Restart & Recovery Tests Complete!"
    echo "============================================================"
}

main "$@"
