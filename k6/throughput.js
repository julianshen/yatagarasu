/**
 * K6 Throughput Test
 *
 * Objective: Verify baseline throughput > 1,000 req/s
 *
 * Test Configuration:
 * - Duration: 60 seconds
 * - Virtual Users: 10 concurrent users
 * - Target: 1,000+ requests/second
 * - File: 1KB test file (small, fast requests)
 *
 * Success Criteria:
 * - Average RPS > 1,000
 * - P95 latency < 50ms
 * - Error rate < 0.1%
 *
 * Usage:
 *   k6 run k6/throughput.js
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - MinIO running with test bucket
 *   - 1KB test file uploaded to /public/test-1kb.txt
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const requestDuration = new Trend('request_duration');
const requestCount = new Counter('requests');

// Test options
export const options = {
  vus: 10,              // 10 concurrent virtual users
  duration: '60s',      // Run for 60 seconds

  thresholds: {
    http_req_duration: ['p(95)<50'],    // 95% of requests < 50ms
    http_req_failed: ['rate<0.001'],    // Error rate < 0.1%
    errors: ['rate<0.001'],             // Custom error rate < 0.1%
  },
};

// Setup function (runs once before test)
export function setup() {
  console.log('='.repeat(80));
  console.log('K6 Throughput Test - Yatagarasu S3 Proxy');
  console.log('='.repeat(80));
  console.log('Target: > 1,000 req/s');
  console.log('Duration: 60 seconds');
  console.log('Virtual Users: 10');
  console.log('File: 1KB test file');
  console.log('='.repeat(80));
}

// Main test function (runs repeatedly)
export default function () {
  const url = 'http://localhost:8080/public/test-1kb.txt';

  const response = http.get(url);

  // Record custom metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'response time < 100ms': (r) => r.timings.duration < 100,
  });

  errorRate.add(!success);

  // Small think time to simulate realistic traffic
  sleep(0.01); // 10ms between requests per VU
}

// Teardown function (runs once after test)
export function teardown(data) {
  console.log('='.repeat(80));
  console.log('Test Complete!');
  console.log('='.repeat(80));
  console.log('Check results above for:');
  console.log('  - http_reqs: Total requests made');
  console.log('  - http_req_duration (p95): 95th percentile latency');
  console.log('  - http_req_failed: Error rate');
  console.log('');
  console.log('Success Criteria:');
  console.log('  ✓ RPS > 1,000 (http_reqs / duration)');
  console.log('  ✓ P95 latency < 50ms');
  console.log('  ✓ Error rate < 0.1%');
  console.log('='.repeat(80));
}
