/**
 * K6 Test: Phase 56.2 Extreme Concurrency Tests
 *
 * This script tests the proxy under extreme concurrent load scenarios.
 * It focuses on cache hits (small files) to isolate connection handling.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - Test files in MinIO: test-1kb.txt, test-10kb.txt
 *
 * Usage:
 *   # 1,000 concurrent connections test
 *   k6 run -e SCENARIO=concurrent_1k k6/extreme-concurrency.js
 *
 *   # 5,000 concurrent connections test
 *   k6 run -e SCENARIO=concurrent_5k k6/extreme-concurrency.js
 *
 *   # 10,000 concurrent connections test
 *   k6 run -e SCENARIO=concurrent_10k k6/extreme-concurrency.js
 *
 *   # High RPS sustained test
 *   k6 run -e SCENARIO=high_rps k6/extreme-concurrency.js
 *
 *   # Find max sustainable RPS
 *   k6 run -e SCENARIO=ramp_to_max k6/extreme-concurrency.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const successRate = new Rate('success');
const requestLatency = new Trend('request_latency_ms');
const requestCount = new Counter('total_requests');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Small cached files for high-throughput testing
const TEST_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
];

// Scenarios for different concurrency levels
const scenarios = {
  // 1,000 concurrent connections
  concurrent_1k: {
    executor: 'constant-vus',
    vus: 1000,
    duration: '1m',
  },

  // 5,000 concurrent connections
  concurrent_5k: {
    executor: 'constant-vus',
    vus: 5000,
    duration: '1m',
  },

  // 10,000 concurrent connections (requires ulimit tuning)
  concurrent_10k: {
    executor: 'constant-vus',
    vus: 10000,
    duration: '1m',
  },

  // High RPS sustained (5000 RPS)
  high_rps: {
    executor: 'constant-arrival-rate',
    rate: 5000,
    timeUnit: '1s',
    duration: '1m',
    preAllocatedVUs: 200,
    maxVUs: 1000,
  },

  // Ramp to find max sustainable RPS
  ramp_to_max: {
    executor: 'ramping-arrival-rate',
    startRate: 1000,
    timeUnit: '1s',
    preAllocatedVUs: 500,
    maxVUs: 5000,
    stages: [
      { target: 2000, duration: '30s' },  // Ramp to 2000 RPS
      { target: 5000, duration: '30s' },  // Ramp to 5000 RPS
      { target: 10000, duration: '30s' }, // Ramp to 10000 RPS
      { target: 15000, duration: '30s' }, // Ramp to 15000 RPS
      { target: 20000, duration: '30s' }, // Ramp to 20000 RPS
    ],
  },

  // Quick validation test
  quick: {
    executor: 'constant-vus',
    vus: 100,
    duration: '30s',
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO || 'quick';
const activeScenario = scenarios[selectedScenario];

if (!activeScenario) {
  console.error(`Unknown scenario: ${selectedScenario}`);
  console.error(`Available: ${Object.keys(scenarios).join(', ')}`);
}

export const options = {
  scenarios: {
    [selectedScenario]: activeScenario,
  },
  thresholds: {
    // Performance targets
    'http_req_duration{expected_response:true}': ['p(95)<100', 'p(99)<500'],
    'http_req_failed{expected_response:true}': ['rate<0.01'],
    errors: ['rate<0.01'],
    success: ['rate>0.99'],
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 56.2: Extreme Concurrency Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario}`);
  console.log('='.repeat(80));

  // Warm cache
  console.log('Warming cache...');
  for (const file of TEST_FILES) {
    const response = http.get(`${BASE_URL}${file}`);
    if (response.status !== 200) {
      console.log(`Warning: Warmup failed for ${file}: ${response.status}`);
    }
  }
  // Extra warmup passes
  for (let i = 0; i < 10; i++) {
    for (const file of TEST_FILES) {
      http.get(`${BASE_URL}${file}`);
    }
  }
  console.log('Cache warmed.');

  return { startTime: Date.now() };
}

let requestId = 0;

export default function (data) {
  // Alternate between test files
  const testFile = TEST_FILES[requestId % TEST_FILES.length];
  requestId++;

  const response = http.get(`${BASE_URL}${testFile}`, {
    tags: { expected_response: 'true' },
  });

  requestCount.add(1);
  requestLatency.add(response.timings.duration);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'latency < 100ms': (r) => r.timings.duration < 100,
  });

  errorRate.add(!success);
  successRate.add(success);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 56.2: Extreme Concurrency Test Complete');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Success Criteria:');
  console.log('  - Error rate < 1%');
  console.log('  - P95 latency < 100ms');
  console.log('  - P99 latency < 500ms');
  console.log('  - Graceful behavior under load');
  console.log('='.repeat(80));
}
