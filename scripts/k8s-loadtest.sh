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

    # Check if namespace exists - if so, do incremental update
    if kubectl get namespace "${NAMESPACE}" &>/dev/null; then
        log_info "Namespace exists, doing incremental update..."

        # Delete jobs first (they're immutable)
        log_info "Cleaning up old jobs..."
        kubectl delete job --all -n "${NAMESPACE}" --ignore-not-found

        # Apply updates
        kubectl apply -k "${K8S_DIR}"
    else
        log_info "Creating fresh deployment..."
        kubectl apply -k "${K8S_DIR}"
    fi

    log_info "Waiting for MinIO to be ready..."
    kubectl wait --for=condition=ready pod -l app=minio -n "${NAMESPACE}" --timeout=120s

    log_info "Waiting for MinIO setup to complete..."
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
    sleep 2

    # Create test job inline (avoids immutable job issues with kubectl apply)
    kubectl create -f - <<'TESTJOB'
apiVersion: batch/v1
kind: Job
metadata:
  name: disk-cache-recovery-test
  namespace: yatagarasu-loadtest
spec:
  ttlSecondsAfterFinished: 600
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: test
          image: curlimages/curl:8.4.0
          command: ["/bin/sh", "-c"]
          args:
            - |
              set -e
              echo "============================================================"
              echo "K8s Disk Cache Recovery Test"
              echo "============================================================"
              PROXY_URL="http://yatagarasu:8080"
              NUM_ENTRIES=500
              TEST_FILE="/public/test-1kb.txt"
              echo "Waiting for proxy..."
              until curl -sf "$PROXY_URL/health" >/dev/null 2>&1; do sleep 2; done
              echo "[PASS] Proxy ready"
              echo ""
              echo "Populating cache with $NUM_ENTRIES entries..."
              i=1
              while [ $i -le $NUM_ENTRIES ]; do
                curl -sf "$PROXY_URL$TEST_FILE?entry=$i" >/dev/null
                [ $((i % 100)) -eq 0 ] && echo "  $i entries..."
                i=$((i + 1))
              done
              echo "[PASS] Cache populated"
              sleep 3
              echo ""
              echo "Verifying all entries accessible..."
              ERRORS=0
              i=1
              while [ $i -le $NUM_ENTRIES ]; do
                curl -sf "$PROXY_URL$TEST_FILE?entry=$i" >/dev/null || ERRORS=$((ERRORS + 1))
                [ $((i % 100)) -eq 0 ] && echo "  Verified $i..."
                i=$((i + 1))
              done
              if [ $ERRORS -eq 0 ]; then
                echo "[PASS] All $NUM_ENTRIES entries accessible"
              else
                echo "[FAIL] $ERRORS entries failed"
                exit 1
              fi
              echo ""
              echo "============================================================"
              echo "Test Complete!"
              echo "============================================================"
TESTJOB

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
    log_info ""
    log_info "Step 2: Rolling restart of Yatagarasu..."
    kubectl rollout restart deployment/yatagarasu -n "${NAMESPACE}"

    log_info "Waiting for pod to be ready after restart..."
    kubectl rollout status deployment/yatagarasu -n "${NAMESPACE}" --timeout=120s

    # Run test again
    log_info ""
    log_info "Step 3: Verify cache after restart..."
    cmd_test

    log_info "Restart recovery test complete!"
}

cmd_k6() {
    local duration="${1:-1m}"

    log_info "Running k6 load test: duration=${duration}"

    # Delete old job if exists
    kubectl delete job k6-load-test -n "${NAMESPACE}" --ignore-not-found
    sleep 2

    # Create k6 script configmap
    kubectl delete configmap k6-inline-script -n "${NAMESPACE}" --ignore-not-found
    kubectl create configmap k6-inline-script -n "${NAMESPACE}" --from-literal=script.js="
import http from 'k6/http';
import { check } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');
const BASE_URL = __ENV.BASE_URL || 'http://yatagarasu:8080';
const FILES = ['/public/test-1kb.txt', '/public/test-10kb.txt', '/public/test-100kb.txt'];

export const options = {
  scenarios: {
    load: {
      executor: 'constant-arrival-rate',
      rate: 50,
      timeUnit: '1s',
      duration: '${duration}',
      preAllocatedVUs: 10,
      maxVUs: 50,
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<500'],
    http_req_failed: ['rate<0.01'],
  },
};

let reqId = 0;

export default function() {
  const url = BASE_URL + FILES[reqId % FILES.length] + '?id=' + reqId++;
  const r = http.get(url);
  errorRate.add(!check(r, { 'status 200': (r) => r.status === 200 }));
}
"

    # Create k6 job
    kubectl create -f - <<'K6JOB'
apiVersion: batch/v1
kind: Job
metadata:
  name: k6-load-test
  namespace: yatagarasu-loadtest
spec:
  ttlSecondsAfterFinished: 600
  backoffLimit: 0
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: k6
          image: grafana/k6:latest
          command: ["k6", "run", "-e", "BASE_URL=http://yatagarasu:8080", "/scripts/script.js"]
          volumeMounts:
            - name: script
              mountPath: /scripts
              readOnly: true
      volumes:
        - name: script
          configMap:
            name: k6-inline-script
K6JOB

    log_info "Waiting for k6 to start..."
    sleep 5

    # Follow logs
    kubectl logs -f job/k6-load-test -n "${NAMESPACE}"
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
    k6)
        cmd_k6 "${2:-1m}"
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
        echo "  deploy         Deploy MinIO and Yatagarasu to K8s"
        echo "  test           Run disk cache recovery test"
        echo "  restart        Test cache persistence across pod restart"
        echo "  k6 [duration]  Run k6 load test (default: 1m)"
        echo "  logs           View Yatagarasu logs"
        echo "  port-forward   Forward ports to localhost"
        echo "  status         Show pod/service status"
        echo "  cleanup        Delete all K8s resources"
        ;;
esac
