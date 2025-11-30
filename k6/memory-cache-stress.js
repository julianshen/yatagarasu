/**
 * K6 Stress Test: Phase 38.1 Memory Cache Extreme Concurrency
 *
 * This script stress tests the memory cache with extreme concurrent requests.
 * It validates that the cache handles high concurrency without crashes or
 * excessive error rates.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with memory cache enabled
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt
 *
 * Usage:
 *   # Run 10k concurrent requests test
 *   k6 run -e SCENARIO=stress_10k k6/memory-cache-stress.js
 *
 *   # Run 50k concurrent requests test
 *   k6 run -e SCENARIO=stress_50k k6/memory-cache-stress.js
 *
 *   # Run saturation test
 *   k6 run -e SCENARIO=saturation k6/memory-cache-stress.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const requestDuration = new Trend('request_duration_ms');
const requestCount = new Counter('total_requests');
const successfulRequests = new Counter('successful_requests');
const failedRequests = new Counter('failed_requests');
const concurrentVUs = new Gauge('concurrent_vus');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test file - use 1KB file for stress testing (minimal network overhead)
const TEST_FILE = '/public/test-1kb.txt';

// Stress test scenarios
const scenarios = {
  // 10,000 concurrent requests stress test
  stress_10k: {
    executor: 'constant-vus',
    vus: 500,          // 500 concurrent VUs
    duration: '30s',   // Each VU makes ~20 requests = 10,000 total
    env: { STRESS_LEVEL: '10k' },
  },

  // 50,000 concurrent requests stress test
  stress_50k: {
    executor: 'constant-vus',
    vus: 1000,         // 1000 concurrent VUs
    duration: '1m',    // Each VU makes ~50 requests = 50,000 total
    env: { STRESS_LEVEL: '50k' },
  },

  // Ramping VUs to find saturation point
  saturation: {
    executor: 'ramping-vus',
    startVUs: 10,
    stages: [
      { duration: '30s', target: 100 },    // Ramp up to 100 VUs
      { duration: '30s', target: 500 },    // Ramp up to 500 VUs
      { duration: '30s', target: 1000 },   // Ramp up to 1000 VUs
      { duration: '30s', target: 2000 },   // Ramp up to 2000 VUs (extreme)
      { duration: '30s', target: 500 },    // Ramp back down
      { duration: '30s', target: 100 },    // Cool down
    ],
    env: { STRESS_LEVEL: 'saturation' },
  },

  // Quick validation test
  quick_stress: {
    executor: 'constant-vus',
    vus: 100,
    duration: '30s',
    env: { STRESS_LEVEL: 'quick' },
  },

  // Spike test - sudden burst of traffic
  spike_10k: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '5s', target: 1000 },    // Sudden spike to 1000 VUs
      { duration: '20s', target: 1000 },   // Hold at 1000 VUs
      { duration: '5s', target: 0 },       // Sudden drop
    ],
    env: { STRESS_LEVEL: 'spike' },
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : { quick_stress: scenarios.quick_stress };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: ['p(95)<500'],   // P95 < 500ms under stress
    http_req_failed: ['rate<0.05'],     // Error rate < 5% (graceful degradation)
    errors: ['rate<0.05'],              // Error rate < 5%
  },
  // Disable DNS caching to avoid OS limits
  dns: {
    ttl: '1m',
    select: 'first',
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 38.1: Memory Cache Stress Tests - Extreme Concurrency');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'quick_stress'}`);
  console.log(`Stress Level: ${__ENV.STRESS_LEVEL || 'quick'}`);
  console.log('='.repeat(80));

  // Pre-warm cache with test file
  console.log('Pre-warming cache...');
  for (let i = 0; i < 10; i++) {
    const response = http.get(`${BASE_URL}${TEST_FILE}`);
    if (response.status !== 200) {
      console.log(`Warning: Warmup request failed: ${response.status}`);
    }
  }
  // Give time for async cache operations
  sleep(2);
  console.log('Cache warmed up.');
  console.log('');
  console.log('Starting stress test...');

  return { startTime: Date.now() };
}

export default function (data) {
  // Track concurrent VUs
  concurrentVUs.add(__VU);

  // Make request
  const response = http.get(`${BASE_URL}${TEST_FILE}`, {
    timeout: '30s',  // Longer timeout for stress conditions
  });

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);

  // Check cache status
  const cacheStatus = response.headers['X-Cache-Status'] ||
                      response.headers['x-cache-status'] ||
                      response.headers['X-Cache'] ||
                      response.headers['x-cache'] ||
                      '';
  const isCacheHit = cacheStatus.toLowerCase().includes('hit');
  cacheHitRate.add(isCacheHit);

  // Validation
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  if (success) {
    successfulRequests.add(1);
  } else {
    failedRequests.add(1);
    // Log first few errors for debugging
    if (failedRequests.value < 10) {
      console.log(`Error: status=${response.status}, duration=${response.timings.duration}ms`);
    }
  }

  errorRate.add(!success);

  // Minimal sleep to prevent overwhelming the system
  // In stress tests, we want maximum pressure, so we use a very short sleep
  sleep(0.01);  // 10ms between requests per VU
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Stress Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Phase 38.1 Success Criteria:');
  console.log('');
  console.log('  Extreme Concurrency:');
  console.log('    - 10,000 concurrent requests: No crashes');
  console.log('    - 50,000 concurrent requests: No crashes');
  console.log('    - Error rate < 5% (graceful degradation)');
  console.log('    - P95 latency < 500ms under stress');
  console.log('');
  console.log('  Thread Pool Saturation:');
  console.log('    - System remains responsive at saturation');
  console.log('    - Requests queue rather than fail');
  console.log('    - Recovery after load decreases');
  console.log('');
  console.log('  Graceful Degradation:');
  console.log('    - No OOM kills');
  console.log('    - No deadlocks');
  console.log('    - Error messages are meaningful');
  console.log('='.repeat(80));
}
