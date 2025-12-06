/**
 * K6 Test: Phase 58 CPU Core Scaling Tests
 *
 * This script measures proxy throughput to establish baseline performance.
 * The CPU core limitation is done externally via Docker's --cpus flag
 * or taskset (Linux) before running this script.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - Test files in MinIO: test-1kb.txt, test-10kb.txt
 *   - Proxy started with limited cores (see scripts/cpu-scaling-test.sh)
 *
 * Usage:
 *   # Quick baseline test (30s)
 *   k6 run k6/cpu-scaling.js
 *
 *   # Full saturation test (2m)
 *   k6 run -e SCENARIO=saturation k6/cpu-scaling.js
 *
 *   # Find max RPS
 *   k6 run -e SCENARIO=find_max k6/cpu-scaling.js
 *
 *   # Sustained load for stability measurement
 *   k6 run -e SCENARIO=sustained k6/cpu-scaling.js
 *
 * Environment Variables:
 *   - BASE_URL: Proxy URL (default: http://localhost:8080)
 *   - SCENARIO: Test scenario (default: baseline)
 *   - CORES: Number of CPU cores (for labeling, default: auto)
 */

import http from 'k6/http';
import { check } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics for scaling analysis
const errorRate = new Rate('errors');
const successRate = new Rate('success');
const requestLatency = new Trend('request_latency_ms');
const totalRequests = new Counter('total_requests');
const maxRps = new Gauge('max_rps_achieved');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const CORES = __ENV.CORES || 'auto';

// Small cached files for CPU-bound testing (minimize I/O impact)
const TEST_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
];

// Scenarios for different test types
const scenarios = {
  // Quick baseline: 30s at moderate load
  baseline: {
    executor: 'constant-arrival-rate',
    rate: 2000,
    timeUnit: '1s',
    duration: '30s',
    preAllocatedVUs: 100,
    maxVUs: 500,
  },

  // Saturation test: ramp until errors appear
  saturation: {
    executor: 'ramping-arrival-rate',
    startRate: 1000,
    timeUnit: '1s',
    preAllocatedVUs: 200,
    maxVUs: 2000,
    stages: [
      { target: 2000, duration: '20s' },
      { target: 5000, duration: '20s' },
      { target: 10000, duration: '20s' },
      { target: 15000, duration: '20s' },
      { target: 20000, duration: '20s' },
      { target: 25000, duration: '20s' },
    ],
  },

  // Find maximum sustainable RPS
  find_max: {
    executor: 'ramping-arrival-rate',
    startRate: 500,
    timeUnit: '1s',
    preAllocatedVUs: 100,
    maxVUs: 3000,
    stages: [
      { target: 1000, duration: '15s' },
      { target: 2000, duration: '15s' },
      { target: 3000, duration: '15s' },
      { target: 5000, duration: '15s' },
      { target: 8000, duration: '15s' },
      { target: 10000, duration: '15s' },
      { target: 12000, duration: '15s' },
      { target: 15000, duration: '15s' },
    ],
  },

  // Sustained load for stability (5 minutes)
  sustained: {
    executor: 'constant-arrival-rate',
    rate: 5000,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 200,
    maxVUs: 1000,
  },

  // Quick validation (10s)
  quick: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '10s',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },
};

// Select scenario from environment
const selectedScenario = __ENV.SCENARIO || 'baseline';
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
    'http_req_failed': ['rate<0.05'],
    'http_req_duration': ['p(95)<500'],
    'errors': ['rate<0.05'],
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 58: CPU Core Scaling Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`CPU Cores: ${CORES}`);
  console.log(`Scenario: ${selectedScenario}`);
  console.log('='.repeat(80));

  // Verify proxy is accessible
  const healthCheck = http.get(`${BASE_URL}/health`);
  if (healthCheck.status !== 200) {
    console.error(`ERROR: Proxy health check failed (status ${healthCheck.status})`);
    return { error: true };
  }
  console.log('Proxy health check: OK');

  // Warm cache
  console.log('Warming cache...');
  for (const file of TEST_FILES) {
    http.get(`${BASE_URL}${file}`);
  }
  for (let i = 0; i < 20; i++) {
    for (const file of TEST_FILES) {
      http.get(`${BASE_URL}${file}`);
    }
  }
  console.log('Cache warmed.');

  return { startTime: Date.now(), cores: CORES };
}

let requestId = 0;
let intervalRequests = 0;
let lastInterval = Date.now();

export default function (data) {
  if (data && data.error) return;

  const testFile = TEST_FILES[requestId % TEST_FILES.length];
  requestId++;
  intervalRequests++;

  const response = http.get(`${BASE_URL}${testFile}`, {
    tags: { expected_response: 'true' },
    timeout: '10s',
  });

  totalRequests.add(1);
  requestLatency.add(response.timings.duration);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  errorRate.add(!success);
  successRate.add(success);

  const now = Date.now();
  if (now - lastInterval >= 1000) {
    const rps = intervalRequests / ((now - lastInterval) / 1000);
    maxRps.add(rps);
    intervalRequests = 0;
    lastInterval = now;
  }
}

export function teardown(data) {
  if (data && data.error) return;

  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 58: CPU Core Scaling Test Complete');
  console.log('='.repeat(80));
  console.log(`CPU Cores: ${data.cores}`);
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log(`Scenario: ${selectedScenario}`);
  console.log('='.repeat(80));
}
