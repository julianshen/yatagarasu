/**
 * K6 Load Test: Phase 37.1 Memory Cache Load Tests
 *
 * This script tests memory cache performance under realistic production load.
 * It covers cold cache, hot cache, and mixed workload scenarios.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt, test-10kb.txt, test-100kb.txt
 *
 * Usage:
 *   # Run cold cache test (100 RPS, 5 minutes)
 *   k6 run -e SCENARIO=cold_100rps k6/memory-cache-load.js
 *
 *   # Run hot cache test (1000 RPS, 5 minutes)
 *   k6 run -e SCENARIO=hot_1000rps k6/memory-cache-load.js
 *
 *   # Run mixed workload test (70% read, 30% write)
 *   k6 run -e SCENARIO=mixed_workload k6/memory-cache-load.js
 *
 *   # Run all scenarios sequentially
 *   k6 run k6/memory-cache-load.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const requestDuration = new Trend('request_duration_ms');
const requestCount = new Counter('total_requests');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files (should exist in MinIO 'public' bucket)
const TEST_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
  '/public/test-100kb.txt',
];

// Scenarios based on Phase 37.1 requirements
const scenarios = {
  // Cold Cache Scenario (All Misses)
  cold_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 20,
    maxVUs: 100,
  },
  cold_500rps: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },
  cold_1000rps: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 100,
    maxVUs: 400,
  },
  cold_5000rps_stress: {
    executor: 'constant-arrival-rate',
    rate: 5000,
    timeUnit: '1s',
    duration: '1m',
    preAllocatedVUs: 200,
    maxVUs: 1000,
  },

  // Hot Cache Scenario (90% Hit Rate)
  hot_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 10,
    maxVUs: 50,
  },
  hot_500rps: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },
  hot_1000rps: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 100,
    maxVUs: 400,
  },
  hot_5000rps: {
    executor: 'constant-arrival-rate',
    rate: 5000,
    timeUnit: '1s',
    duration: '1m',
    preAllocatedVUs: 200,
    maxVUs: 1000,
  },
  hot_10000rps_extreme: {
    executor: 'constant-arrival-rate',
    rate: 10000,
    timeUnit: '1s',
    duration: '30s',
    preAllocatedVUs: 400,
    maxVUs: 2000,
  },

  // Mixed Workload
  mixed_70read_30write: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '10m',
    preAllocatedVUs: 100,
    maxVUs: 400,
    env: { READ_RATIO: '0.7' },
  },
  mixed_90read_10write: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '10m',
    preAllocatedVUs: 100,
    maxVUs: 400,
    env: { READ_RATIO: '0.9' },
  },
  mixed_50read_50write: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { READ_RATIO: '0.5' },
  },

  // Sustained Load (Endurance)
  sustained_500rps_1hour: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '1h',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },
  sustained_1000rps_30min: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '30m',
    preAllocatedVUs: 100,
    maxVUs: 400,
  },
};

// Select scenario from environment variable or run a default set
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : {
      // Default: run basic cold and hot tests
      cold_100rps: scenarios.cold_100rps,
      hot_1000rps: scenarios.hot_1000rps,
    };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: [
      'p(95)<200',  // P95 < 200ms for cold cache
      'p(95)<50',   // P95 < 50ms for hot cache (target)
    ],
    http_req_failed: ['rate<0.001'],  // Error rate < 0.1%
    errors: ['rate<0.001'],
  },
};

// Warm up cache with test files
export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 37.1: Memory Cache Load Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'default (cold_100rps + hot_1000rps)'}`);
  console.log('='.repeat(80));

  // For hot cache tests, pre-warm the cache
  if (selectedScenario && selectedScenario.startsWith('hot')) {
    console.log('Pre-warming cache...');
    for (const file of TEST_FILES) {
      const warmupIterations = 10;
      for (let i = 0; i < warmupIterations; i++) {
        http.get(`${BASE_URL}${file}`);
      }
    }
    console.log('Cache warmed up.');
  }

  return { startTime: Date.now() };
}

// Generate unique keys for cold cache testing
let requestId = 0;

export default function (data) {
  const scenarioName = __ENV.SCENARIO || 'default';
  const readRatio = parseFloat(__ENV.READ_RATIO || '1.0');

  // Select a test file (round-robin)
  const fileIndex = requestId % TEST_FILES.length;
  const testFile = TEST_FILES[fileIndex];
  requestId++;

  // For cold cache: add unique query param to bypass cache
  // For hot cache: use same URLs to maximize cache hits
  const isColdCache = scenarioName.startsWith('cold');
  const url = isColdCache
    ? `${BASE_URL}${testFile}?nocache=${requestId}`
    : `${BASE_URL}${testFile}`;

  // For mixed workload: simulate reads and writes
  const isRead = Math.random() < readRatio;

  let response;
  if (isRead) {
    response = http.get(url);
  } else {
    // Simulate write by requesting with cache-control: no-cache
    // In real scenario, this would be a PUT/POST to trigger cache invalidation
    response = http.get(url, {
      headers: { 'Cache-Control': 'no-cache' },
    });
  }

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);

  // Check for cache hit (via X-Cache-Status header if available)
  const cacheStatus = response.headers['X-Cache-Status'];
  const isCacheHit = cacheStatus === 'HIT' || cacheStatus === 'hit';
  cacheHitRate.add(isCacheHit);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Additional latency checks based on scenario
  if (scenarioName.startsWith('hot')) {
    check(response, {
      'hot cache: response time < 50ms': (r) => r.timings.duration < 50,
    });
  } else {
    check(response, {
      'cold cache: response time < 200ms': (r) => r.timings.duration < 200,
    });
  }

  errorRate.add(!success);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('='.repeat(80));
  console.log('Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Phase 37.1 Success Criteria:');
  console.log('  Cold Cache (100-1000 RPS):');
  console.log('    - P95 latency < 200ms');
  console.log('    - Error rate < 0.1%');
  console.log('');
  console.log('  Hot Cache (1000-10000 RPS):');
  console.log('    - P95 latency < 50ms');
  console.log('    - Error rate < 0.1%');
  console.log('    - Cache hit rate > 85%');
  console.log('');
  console.log('  Mixed Workload:');
  console.log('    - LRU eviction works correctly');
  console.log('    - No lock contention');
  console.log('');
  console.log('  Sustained Load:');
  console.log('    - Memory stable (no leaks)');
  console.log('    - Latency does not degrade over time');
  console.log('='.repeat(80));
}
