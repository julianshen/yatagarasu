/**
 * K6 Test: Phase 60 Hot Reload Under Load
 *
 * This script tests that config reload (SIGHUP) doesn't drop requests.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - Test files in MinIO
 *
 * Usage:
 *   # Run load test (reload config manually with SIGHUP during test)
 *   k6 run k6/hot-reload.js
 *
 *   # Run for 5 minutes to allow multiple reloads
 *   k6 run -e DURATION=5m k6/hot-reload.js
 */

import http from 'k6/http';
import { check } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const successRate = new Rate('success');
const requestLatency = new Trend('request_latency_ms');
const totalRequests = new Counter('total_requests');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const DURATION = __ENV.DURATION || '2m';

// Test endpoints
const TEST_ENDPOINTS = [
  '/health',
  '/public/test-1kb.txt',
];

export const options = {
  scenarios: {
    sustained_load: {
      executor: 'constant-arrival-rate',
      rate: 100,           // 100 RPS
      timeUnit: '1s',
      duration: DURATION,
      preAllocatedVUs: 20,
      maxVUs: 50,
    },
  },
  thresholds: {
    // Zero tolerance for errors during hot reload
    'http_req_failed': ['rate<0.001'],  // <0.1% errors
    'errors': ['rate<0.001'],
    'http_req_duration': ['p(95)<200'],  // P95 < 200ms
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 60: Hot Reload Under Load Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Duration: ${DURATION}`);
  console.log('');
  console.log('Instructions:');
  console.log('  1. This test runs for ' + DURATION);
  console.log('  2. During the test, trigger config reload with:');
  console.log('     kill -HUP $(pgrep yatagarasu)');
  console.log('  3. Monitor for any errors in the output');
  console.log('');
  console.log('Success Criteria:');
  console.log('  - Error rate < 0.1%');
  console.log('  - P95 latency < 200ms');
  console.log('  - Zero dropped requests during reload');
  console.log('='.repeat(80));

  // Verify proxy is accessible
  const healthCheck = http.get(`${BASE_URL}/health`);
  if (healthCheck.status !== 200) {
    console.error('ERROR: Proxy health check failed');
    return { error: true };
  }

  // Warm cache
  for (const endpoint of TEST_ENDPOINTS) {
    if (!endpoint.includes('health')) {
      http.get(`${BASE_URL}${endpoint}`);
    }
  }

  return { startTime: Date.now() };
}

let requestId = 0;

export default function (data) {
  if (data && data.error) return;

  const endpoint = TEST_ENDPOINTS[requestId % TEST_ENDPOINTS.length];
  requestId++;

  const response = http.get(`${BASE_URL}${endpoint}`, {
    timeout: '10s',
  });

  totalRequests.add(1);
  requestLatency.add(response.timings.duration);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
  });

  errorRate.add(!success);
  successRate.add(success);

  if (!success) {
    console.log(`ERROR at ${new Date().toISOString()}: ${endpoint} returned ${response.status}`);
  }
}

export function teardown(data) {
  if (data && data.error) return;

  const duration = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 60: Hot Reload Test Complete');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Check the results above for:');
  console.log('  - http_req_failed should be 0%');
  console.log('  - errors should be 0%');
  console.log('  - No ERROR logs during the test');
  console.log('='.repeat(80));
}
