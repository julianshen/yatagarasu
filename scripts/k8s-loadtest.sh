#!/bin/bash
# K8s Load Test Helper Script
#
# Usage:
#   ./scripts/k8s-loadtest.sh deploy     # Deploy all resources
#   ./scripts/k8s-loadtest.sh test       # Run cache recovery test
#   ./scripts/k8s-loadtest.sh restart    # Test restart recovery
#   ./scripts/k8s-loadtest.sh logs       # View proxy logs
#   ./scripts/k8s-loadtest.sh cleanup    # Delete all resources
#   ./scripts/k8s-loadtest.sh port-forward # Forward ports locally

set -e

NAMESPACE="yatagarasu-loadtest"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
K8S_DIR="${SCRIPT_DIR}/../k8s/loadtest"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

cmd_deploy() {
    log_info "Building yatagarasu:test image..."
    docker build -t yatagarasu:test "${SCRIPT_DIR}/.."

    log_info "Deploying to Kubernetes..."
    kubectl apply -k "${K8S_DIR}"

    log_info "Waiting for MinIO to be ready..."
    kubectl wait --for=condition=ready pod -l app=minio -n "${NAMESPACE}" --timeout=120s

    log_info "Running MinIO setup job..."
    kubectl delete job minio-setup -n "${NAMESPACE}" --ignore-not-found
    kubectl apply -f "${K8S_DIR}/minio.yaml" -n "${NAMESPACE}"
    kubectl wait --for=condition=complete job/minio-setup -n "${NAMESPACE}" --timeout=120s

    log_info "Waiting for Yatagarasu to be ready..."
    kubectl wait --for=condition=ready pod -l app=yatagarasu -n "${NAMESPACE}" --timeout=120s

    log_info "Deployment complete!"
    echo ""
    kubectl get pods -n "${NAMESPACE}"
}

cmd_test() {
    log_info "Running disk cache recovery test..."

    # Delete old job if exists
    kubectl delete job disk-cache-recovery-test -n "${NAMESPACE}" --ignore-not-found

    # Apply and wait for job
    kubectl apply -f "${K8S_DIR}/tests.yaml"

    log_info "Waiting for test job to start..."
    sleep 3

    # Follow logs
    kubectl logs -f job/disk-cache-recovery-test -n "${NAMESPACE}"

    # Check result
    if kubectl wait --for=condition=complete job/disk-cache-recovery-test -n "${NAMESPACE}" --timeout=300s 2>/dev/null; then
        log_info "Test completed successfully!"
    else
        log_error "Test failed!"
        exit 1
    fi
}

cmd_restart() {
    log_info "Testing restart recovery..."

    # First populate cache
    log_info "Step 1: Populate cache..."
    cmd_test

    # Restart deployment
    log_info "Step 2: Rolling restart of Yatagarasu..."
    kubectl rollout restart deployment/yatagarasu -n "${NAMESPACE}"

    log_info "Waiting for pod to be ready after restart..."
    kubectl rollout status deployment/yatagarasu -n "${NAMESPACE}" --timeout=120s

    # Run test again
    log_info "Step 3: Verify cache after restart..."
    cmd_test

    log_info "Restart recovery test complete!"
}

cmd_logs() {
    kubectl logs -f deployment/yatagarasu -n "${NAMESPACE}"
}

cmd_port_forward() {
    log_info "Forwarding ports..."
    log_info "  Proxy:   http://localhost:8080"
    log_info "  Metrics: http://localhost:9090"
    log_info "  MinIO:   http://localhost:9000 (console: 9001)"
    echo ""
    log_info "Press Ctrl+C to stop"

    # Run port-forwards in background
    kubectl port-forward svc/yatagarasu -n "${NAMESPACE}" 8080:8080 9090:9090 &
    kubectl port-forward svc/minio -n "${NAMESPACE}" 9000:9000 9001:9001 &

    wait
}

cmd_cleanup() {
    log_info "Cleaning up K8s resources..."
    kubectl delete namespace "${NAMESPACE}" --ignore-not-found
    log_info "Cleanup complete!"
}

cmd_status() {
    echo "=== Pods ==="
    kubectl get pods -n "${NAMESPACE}" -o wide
    echo ""
    echo "=== Services ==="
    kubectl get svc -n "${NAMESPACE}"
    echo ""
    echo "=== PVCs ==="
    kubectl get pvc -n "${NAMESPACE}"
}

# Main
case "${1:-help}" in
    deploy)
        cmd_deploy
        ;;
    test)
        cmd_test
        ;;
    restart)
        cmd_restart
        ;;
    logs)
        cmd_logs
        ;;
    port-forward|pf)
        cmd_port_forward
        ;;
    cleanup|delete)
        cmd_cleanup
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "K8s Load Test Helper"
        echo ""
        echo "Usage: $0 <command>"
        echo ""
        echo "Commands:"
        echo "  deploy       Deploy MinIO and Yatagarasu to K8s"
        echo "  test         Run disk cache recovery test"
        echo "  restart      Test cache persistence across pod restart"
        echo "  logs         View Yatagarasu logs"
        echo "  port-forward Forward ports to localhost"
        echo "  status       Show pod/service status"
        echo "  cleanup      Delete all K8s resources"
        ;;
esac
