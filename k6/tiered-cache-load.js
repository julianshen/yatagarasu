/**
 * K6 Load Test: Phase 37.4 Tiered Cache Load Tests
 *
 * This script tests tiered cache performance (memory -> disk -> redis).
 * It validates promotion between layers and multi-layer hit scenarios.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with tiered cache enabled
 *   - Redis running on localhost:6379
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt, test-10kb.txt, test-100kb.txt
 *
 * Usage:
 *   # Run promotion test (100 RPS)
 *   k6 run -e SCENARIO=promotion_100rps k6/tiered-cache-load.js
 *
 *   # Run multi-layer test
 *   k6 run -e SCENARIO=multi_layer_100rps k6/tiered-cache-load.js
 *
 *   # Run sustained load test
 *   k6 run -e SCENARIO=sustained_100rps k6/tiered-cache-load.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const l1HitRate = new Rate('l1_memory_hits');
const l2HitRate = new Rate('l2_disk_hits');
const l3HitRate = new Rate('l3_redis_hits');
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

// Scenarios based on Phase 37.4 requirements
const scenarios = {
  // Promotion Test: Prime redis cache, verify promotion to disk+memory
  promotion_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '2m',
    preAllocatedVUs: 30,
    maxVUs: 150,
    env: { TEST_MODE: 'promotion' },
  },

  // Multi-Layer Test: Test different hit rates per layer
  multi_layer_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '3m',
    preAllocatedVUs: 30,
    maxVUs: 150,
    env: { TEST_MODE: 'multi_layer' },
  },

  // Scaling Test: 500 RPS to verify all layers scale
  scaling_500rps: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '2m',
    preAllocatedVUs: 100,
    maxVUs: 400,
    env: { TEST_MODE: 'scaling' },
  },

  // Sustained Load: 100 RPS for longer duration
  sustained_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '10m',
    preAllocatedVUs: 30,
    maxVUs: 150,
    env: { TEST_MODE: 'sustained' },
  },

  // Quick validation test
  quick_test: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '1m',
    preAllocatedVUs: 10,
    maxVUs: 50,
    env: { TEST_MODE: 'quick' },
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : { quick_test: scenarios.quick_test };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: ['p(95)<200'],  // P95 < 200ms
    http_req_failed: ['rate<0.001'],   // Error rate < 0.1%
    errors: ['rate<0.001'],
    cache_hits: ['rate>0.80'],         // >80% total cache hit rate
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 37.4: Tiered Cache Load Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'quick_test'}`);
  console.log(`Test Mode: ${__ENV.TEST_MODE || 'quick'}`);
  console.log('='.repeat(80));

  // Pre-warm cache with test files
  console.log('Pre-warming tiered cache...');
  for (const file of TEST_FILES) {
    // Make multiple requests to ensure promotion through layers
    for (let i = 0; i < 5; i++) {
      const response = http.get(`${BASE_URL}${file}`);
      if (response.status !== 200) {
        console.log(`Warning: Warmup request failed for ${file}: ${response.status}`);
      }
    }
  }
  // Give time for async cache operations
  sleep(3);
  console.log('Tiered cache warmed up.');

  return { startTime: Date.now() };
}

let requestId = 0;

export default function (data) {
  const testMode = __ENV.TEST_MODE || 'quick';

  // Select a test file (round-robin)
  const fileIndex = requestId % TEST_FILES.length;
  const testFile = TEST_FILES[fileIndex];
  requestId++;

  // For promotion test: use same files to test L1 hit rate increase over time
  // For multi-layer test: mix of cached and uncached files
  let url;
  if (testMode === 'multi_layer') {
    // 10% of requests are for "new" files (should be L3 or miss)
    if (Math.random() < 0.1) {
      url = `${BASE_URL}${testFile}?variant=${requestId}`;
    } else {
      url = `${BASE_URL}${testFile}`;
    }
  } else {
    url = `${BASE_URL}${testFile}`;
  }

  const response = http.get(url);

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);

  // Check cache status from response headers
  const cacheStatus = response.headers['X-Cache-Status'] ||
                      response.headers['x-cache-status'] ||
                      response.headers['X-Cache'] ||
                      response.headers['x-cache'] ||
                      '';

  const cacheLayer = response.headers['X-Cache-Layer'] ||
                     response.headers['x-cache-layer'] ||
                     '';

  const isCacheHit = cacheStatus.toLowerCase().includes('hit');
  cacheHitRate.add(isCacheHit);

  // Track per-layer hit rates (if header available)
  const layer = cacheLayer.toLowerCase();
  if (layer === 'memory' || layer === 'l1') {
    l1HitRate.add(1);
    l2HitRate.add(0);
    l3HitRate.add(0);
  } else if (layer === 'disk' || layer === 'l2') {
    l1HitRate.add(0);
    l2HitRate.add(1);
    l3HitRate.add(0);
  } else if (layer === 'redis' || layer === 'l3') {
    l1HitRate.add(0);
    l2HitRate.add(0);
    l3HitRate.add(1);
  } else if (isCacheHit) {
    // Cache hit but layer unknown - count as L1 (fastest)
    l1HitRate.add(1);
    l2HitRate.add(0);
    l3HitRate.add(0);
  }

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Latency checks based on expected layer
  if (isCacheHit) {
    if (layer === 'memory' || layer === 'l1') {
      check(response, {
        'L1 hit latency < 10ms': (r) => r.timings.duration < 10,
      });
    } else if (layer === 'disk' || layer === 'l2') {
      check(response, {
        'L2 hit latency < 50ms': (r) => r.timings.duration < 50,
      });
    } else if (layer === 'redis' || layer === 'l3') {
      check(response, {
        'L3 hit latency < 100ms': (r) => r.timings.duration < 100,
      });
    } else {
      check(response, {
        'cache hit latency < 50ms': (r) => r.timings.duration < 50,
      });
    }
  }

  errorRate.add(!success);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Phase 37.4 Success Criteria:');
  console.log('');
  console.log('  Promotion Under Load:');
  console.log('    - L1 (memory) hit rate increases over time');
  console.log('    - Promotion doesn\'t block responses');
  console.log('    - Promotion failures logged but don\'t fail requests');
  console.log('');
  console.log('  Multi-Layer Performance:');
  console.log('    - L1 (memory): P95 < 10ms');
  console.log('    - L2 (disk): P95 < 50ms');
  console.log('    - L3 (redis): P95 < 100ms');
  console.log('    - Total hit rate > 80%');
  console.log('');
  console.log('  Scaling:');
  console.log('    - All layers scale to 500 RPS');
  console.log('    - No errors under load');
  console.log('');
  console.log('  Sustained Load:');
  console.log('    - Memory layer stays within limits');
  console.log('    - Disk layer evicts correctly');
  console.log('    - Redis TTLs work correctly');
  console.log('='.repeat(80));
}
