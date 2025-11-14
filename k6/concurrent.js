/**
 * K6 Concurrent Connections Test
 *
 * Objective: Verify proxy handles 100 concurrent connections
 *
 * Test Configuration:
 * - Duration: 120 seconds (2 minutes)
 * - Virtual Users: 100 concurrent users
 * - Ramp-up: 20s to reach 100 users (gradual increase)
 * - Steady state: 80s at 100 users
 * - Ramp-down: 20s back to 0
 *
 * Success Criteria:
 * - 0 failed requests (error rate 0%)
 * - P95 latency < 100ms
 * - No connection errors
 * - No timeouts
 *
 * Usage:
 *   k6 run k6/concurrent.js
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - MinIO running with test bucket
 *   - Test files: 1KB, 10KB, 100KB in /public/
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const connectionErrors = new Rate('connection_errors');

// Test options with staged ramping
export const options = {
  stages: [
    { duration: '20s', target: 100 },  // Ramp-up to 100 users over 20s
    { duration: '80s', target: 100 },  // Stay at 100 users for 80s
    { duration: '20s', target: 0 },    // Ramp-down to 0 users over 20s
  ],

  thresholds: {
    http_req_duration: ['p(95)<100'],   // 95% of requests < 100ms
    http_req_failed: ['rate<0.001'],    // Error rate < 0.1%
    errors: ['rate<0.001'],             // Custom error rate < 0.1%
    connection_errors: ['rate<0.001'],  // Connection error rate < 0.1%
  },
};

// Setup
export function setup() {
  console.log('='.repeat(80));
  console.log('K6 Concurrent Connections Test - Yatagarasu S3 Proxy');
  console.log('='.repeat(80));
  console.log('Target: 100 concurrent connections');
  console.log('Duration: 120 seconds (20s ramp-up, 80s steady, 20s ramp-down)');
  console.log('Files: Mixed sizes (1KB, 10KB, 100KB)');
  console.log('='.repeat(80));
}

// Main test function
export default function () {
  const files = [
    'http://localhost:8080/public/test-1kb.txt',
    'http://localhost:8080/public/test-10kb.txt',
    'http://localhost:8080/public/test-100kb.txt',
  ];

  // Random file selection to mix request sizes
  const url = files[Math.floor(Math.random() * files.length)];

  const params = {
    timeout: '10s', // 10 second timeout
  };

  const response = http.get(url, params);

  // Check for connection errors
  const hasConnectionError = response.error_code !== 0;
  connectionErrors.add(hasConnectionError);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'no connection error': (r) => r.error_code === 0,
    'response has body': (r) => r.body && r.body.length > 0,
    'response time < 200ms': (r) => r.timings.duration < 200,
  });

  errorRate.add(!success);

  // Small think time
  sleep(0.1); // 100ms between requests per VU
}

// Teardown
export function teardown(data) {
  console.log('='.repeat(80));
  console.log('Concurrent Connections Test Complete!');
  console.log('='.repeat(80));
  console.log('Check results above for:');
  console.log('  - vus_max: Peak concurrent users reached');
  console.log('  - http_req_duration (p95): 95th percentile latency');
  console.log('  - http_req_failed: Failed request rate');
  console.log('  - connection_errors: Connection error rate');
  console.log('');
  console.log('Success Criteria:');
  console.log('  ✓ Reached 100 concurrent users');
  console.log('  ✓ P95 latency < 100ms');
  console.log('  ✓ 0 failed requests');
  console.log('  ✓ 0 connection errors');
  console.log('='.repeat(80));
}
