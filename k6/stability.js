/**
 * K6 Stability Test (1 Hour)
 *
 * Objective: Verify proxy runs for 1 hour under load without crashes
 *
 * Test Configuration:
 * - Duration: 3600 seconds (1 hour)
 * - Virtual Users: 50 concurrent users (constant)
 * - Mixed workload: Small files, large files, HEAD requests
 * - Realistic think time
 *
 * Success Criteria:
 * - 0 crashes (proxy stays running)
 * - Error rate < 0.1%
 * - Memory stays constant (< 5MB growth)
 * - No degradation over time
 *
 * Usage:
 *   k6 run k6/stability.js
 *
 * Monitor Resources:
 *   ./scripts/monitor-resources.sh > stability-metrics.log &
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - MinIO running with test bucket
 *   - All test files uploaded (1KB, 10KB, 100KB, 1MB, 10MB)
 *   - Resource monitoring script running
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const connectionErrors = new Rate('connection_errors');
const requestCount = new Counter('total_requests');
const smallFileLatency = new Trend('small_file_latency');
const largeFileLatency = new Trend('large_file_latency');

// Test options
export const options = {
  vus: 50,              // 50 constant concurrent users
  duration: '3600s',    // 1 hour = 3600 seconds

  thresholds: {
    http_req_duration: ['p(95)<500'],   // 95% of requests < 500ms
    http_req_failed: ['rate<0.001'],    // Error rate < 0.1%
    errors: ['rate<0.001'],             // Custom error rate < 0.1%
    connection_errors: ['rate<0.001'],  // Connection errors < 0.1%
  },
};

// Setup
export function setup() {
  console.log('='.repeat(80));
  console.log('K6 Stability Test (1 Hour) - Yatagarasu S3 Proxy');
  console.log('='.repeat(80));
  console.log('Duration: 3600 seconds (1 hour)');
  console.log('Virtual Users: 50 (constant)');
  console.log('Workload: Mixed (small/large files, GET/HEAD requests)');
  console.log('');
  console.log('⚠️  IMPORTANT:');
  console.log('   Run resource monitoring script in parallel:');
  console.log('   ./scripts/monitor-resources.sh > stability-metrics.log &');
  console.log('');
  console.log('Test started at:', new Date().toISOString());
  console.log('Expected completion:', new Date(Date.now() + 3600000).toISOString());
  console.log('='.repeat(80));

  // Return start time for teardown analysis
  return {
    startTime: Date.now(),
  };
}

// Main test function - mixed workload
export default function () {
  // 70% GET requests, 20% large files, 10% HEAD requests (realistic mix)
  const rand = Math.random();

  let url, method, isLargeFile;

  if (rand < 0.5) {
    // 50% small files (1KB, 10KB)
    const smallFiles = [
      'http://localhost:8080/public/test-1kb.txt',
      'http://localhost:8080/public/test-10kb.txt',
    ];
    url = smallFiles[Math.floor(Math.random() * smallFiles.length)];
    method = 'GET';
    isLargeFile = false;
  } else if (rand < 0.7) {
    // 20% medium files (100KB, 1MB)
    const mediumFiles = [
      'http://localhost:8080/public/test-100kb.txt',
      'http://localhost:8080/public/test-1mb.bin',
    ];
    url = mediumFiles[Math.floor(Math.random() * mediumFiles.length)];
    method = 'GET';
    isLargeFile = false;
  } else if (rand < 0.9) {
    // 20% large files (10MB)
    url = 'http://localhost:8080/public/test-10mb.bin';
    method = 'GET';
    isLargeFile = true;
  } else {
    // 10% HEAD requests (metadata only)
    url = 'http://localhost:8080/public/test-1kb.txt';
    method = 'HEAD';
    isLargeFile = false;
  }

  const params = {
    timeout: '30s',
  };

  let response;
  if (method === 'HEAD') {
    response = http.head(url, params);
  } else {
    response = http.get(url, params);
  }

  requestCount.add(1);

  // Track latency by file size
  if (isLargeFile) {
    largeFileLatency.add(response.timings.duration);
  } else {
    smallFileLatency.add(response.timings.duration);
  }

  // Check for connection errors
  const hasConnectionError = response.error_code !== 0;
  connectionErrors.add(hasConnectionError);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'no connection error': (r) => r.error_code === 0,
    'no timeout': (r) => r.error_code !== 1050, // K6 timeout error code
  });

  errorRate.add(!success);

  // Realistic think time (user reading/processing)
  sleep(Math.random() * 2 + 0.5); // 0.5-2.5s random think time
}

// Teardown
export function teardown(data) {
  const endTime = Date.now();
  const durationMs = endTime - data.startTime;
  const durationMin = Math.floor(durationMs / 60000);

  console.log('='.repeat(80));
  console.log('Stability Test Complete!');
  console.log('='.repeat(80));
  console.log('Test ended at:', new Date().toISOString());
  console.log('Actual duration:', durationMin, 'minutes');
  console.log('');
  console.log('Check results above for:');
  console.log('  - http_reqs: Total requests processed');
  console.log('  - http_req_failed: Failed request rate');
  console.log('  - connection_errors: Connection error rate');
  console.log('  - small_file_latency (p95): Small file response time');
  console.log('  - large_file_latency (p95): Large file response time');
  console.log('');
  console.log('⚠️  IMPORTANT: Check resource monitoring logs:');
  console.log('   cat stability-metrics.log');
  console.log('');
  console.log('Success Criteria:');
  console.log('  ✓ Proxy stayed running (no crashes)');
  console.log('  ✓ Error rate < 0.1%');
  console.log('  ✓ Memory growth < 5MB (check logs)');
  console.log('  ✓ No performance degradation over time');
  console.log('='.repeat(80));
}
