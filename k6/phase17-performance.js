/**
 * K6 Phase 17 Performance Validation Tests
 *
 * Validates the following performance criteria from plan.md:
 * - Baseline throughput > 1,000 req/s
 * - Small file (1KB) end-to-end < 10ms (P95)
 * - Handles 100 concurrent connections
 * - Handles 1,000 requests without errors
 * - Streaming latency < 100ms (TTFB)
 *
 * Usage:
 *   k6 run k6/phase17-performance.js
 *   k6 run -e SCENARIO=throughput k6/phase17-performance.js
 *   k6 run -e SCENARIO=concurrent k6/phase17-performance.js
 *   k6 run -e SCENARIO=latency k6/phase17-performance.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const ttfb = new Trend('ttfb_ms');
const requestCount = new Counter('total_requests');

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files
const SMALL_FILE = '/public/test-1kb.txt';
const MEDIUM_FILE = '/public/test-100kb.txt';
const LARGE_FILE = '/public/test-1mb.bin';

// Scenarios for different test objectives
const scenarios = {
  // Test 1: Baseline throughput > 1,000 req/s
  throughput: {
    executor: 'constant-arrival-rate',
    rate: 1500, // Target 1500 req/s to ensure we exceed 1000
    timeUnit: '1s',
    duration: '30s',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },

  // Test 2: 100 concurrent connections
  concurrent: {
    executor: 'constant-vus',
    vus: 100,
    duration: '30s',
  },

  // Test 3: Small file latency test (P95 < 10ms)
  latency: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '30s',
    preAllocatedVUs: 20,
    maxVUs: 100,
  },

  // Test 4: 1,000 requests without errors
  reliability: {
    executor: 'shared-iterations',
    vus: 50,
    iterations: 1000,
    maxDuration: '60s',
  },

  // Test 5: Streaming TTFB < 100ms (with larger files)
  streaming: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '30s',
    preAllocatedVUs: 20,
    maxVUs: 50,
  },

  // Combined test - all criteria
  full: {
    executor: 'ramping-arrival-rate',
    startRate: 100,
    timeUnit: '1s',
    preAllocatedVUs: 50,
    maxVUs: 200,
    stages: [
      { target: 500, duration: '30s' },   // Ramp up
      { target: 1500, duration: '30s' },  // Peak load
      { target: 500, duration: '30s' },   // Sustain
      { target: 0, duration: '10s' },     // Ramp down
    ],
  },
};

// Select scenario from environment or run all
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : {
      throughput: scenarios.throughput,
      latency: scenarios.latency,
    };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    // Phase 17 success criteria
    'http_req_duration': [
      'p(95)<10',    // Small file P95 < 10ms
    ],
    'http_req_failed': [
      'rate<0.001',  // Error rate < 0.1%
    ],
    'ttfb_ms': [
      'p(95)<100',   // TTFB P95 < 100ms
    ],
    'http_reqs': [
      'rate>1000',   // Throughput > 1000 req/s
    ],
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 17: Performance Validation Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'default (throughput + latency)'}`);
  console.log('');
  console.log('Success Criteria:');
  console.log('  - Throughput: > 1,000 req/s');
  console.log('  - Small file (1KB) P95: < 10ms');
  console.log('  - TTFB P95: < 100ms');
  console.log('  - Error rate: < 0.1%');
  console.log('  - 100 concurrent connections: OK');
  console.log('  - 1,000 requests: 0 errors');
  console.log('='.repeat(80));

  // Warm up
  for (let i = 0; i < 10; i++) {
    http.get(`${BASE_URL}${SMALL_FILE}`);
  }

  return { startTime: Date.now() };
}

let requestId = 0;

export default function (data) {
  const scenarioName = __ENV.SCENARIO || 'default';

  // Select file based on scenario
  let testFile = SMALL_FILE;
  if (scenarioName === 'streaming') {
    testFile = LARGE_FILE;
  }

  // Make request with unique ID for cold cache testing
  const url = `${BASE_URL}${testFile}?id=${requestId++}`;
  const response = http.get(url);

  // Record metrics
  requestCount.add(1);
  ttfb.add(response.timings.waiting);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Scenario-specific checks
  if (scenarioName === 'latency' || scenarioName === 'throughput') {
    check(response, {
      'small file P95 < 10ms': (r) => r.timings.duration < 10,
    });
  }

  if (scenarioName === 'streaming') {
    check(response, {
      'TTFB < 100ms': (r) => r.timings.waiting < 100,
    });
  }

  errorRate.add(!success);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 17 Performance Validation Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Review thresholds above to verify all criteria passed.');
  console.log('');
  console.log('Criteria Checklist:');
  console.log('  [?] Throughput > 1,000 req/s (check http_reqs rate)');
  console.log('  [?] Small file P95 < 10ms (check http_req_duration p95)');
  console.log('  [?] TTFB P95 < 100ms (check ttfb_ms p95)');
  console.log('  [?] Error rate < 0.1% (check http_req_failed)');
  console.log('  [?] 100 concurrent: Run with -e SCENARIO=concurrent');
  console.log('  [?] 1,000 requests: Run with -e SCENARIO=reliability');
  console.log('='.repeat(80));
}
