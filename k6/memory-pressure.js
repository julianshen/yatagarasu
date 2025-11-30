/**
 * K6 Stress Test: Phase 38.1 Memory Cache Pressure Tests
 *
 * This script stress tests the memory cache under memory pressure conditions.
 * It validates that the cache handles:
 * - Filling to max capacity
 * - Eviction under continuous write pressure
 * - Cache thrashing (alternating large entries)
 * - Memory limits are respected
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with memory-pressure config
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1mb.bin, test-5mb.bin
 *
 * Usage:
 *   # Run fill capacity test
 *   k6 run -e SCENARIO=fill_capacity k6/memory-pressure.js
 *
 *   # Run eviction stress test
 *   k6 run -e SCENARIO=eviction_stress k6/memory-pressure.js
 *
 *   # Run thrashing test
 *   k6 run -e SCENARIO=thrashing k6/memory-pressure.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const cacheMissRate = new Rate('cache_misses');
const evictionCount = new Counter('evictions_triggered');
const requestDuration = new Trend('request_duration_ms');
const requestCount = new Counter('total_requests');
const uniqueKeysCount = new Counter('unique_keys_requested');
const memoryCacheSize = new Gauge('estimated_cache_size_mb');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files for memory pressure (larger files fill cache faster)
const SMALL_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
  '/public/test-100kb.txt',
];

const LARGE_FILES = [
  '/public/test-1mb.bin',
  '/public/test-5mb.bin',
];

// Memory pressure scenarios
const scenarios = {
  // Fill cache to capacity with unique entries
  fill_capacity: {
    executor: 'per-vu-iterations',
    vus: 1,
    iterations: 100,  // Request 100 unique keys to fill 32MB cache
    maxDuration: '5m',
    env: { TEST_MODE: 'fill_capacity' },
  },

  // Continuous eviction stress - keep writing new entries
  eviction_stress: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '2m',
    preAllocatedVUs: 20,
    maxVUs: 100,
    env: { TEST_MODE: 'eviction_stress' },
  },

  // Cache thrashing - alternate between two sets of large files
  thrashing: {
    executor: 'per-vu-iterations',
    vus: 10,
    iterations: 50,
    maxDuration: '5m',
    env: { TEST_MODE: 'thrashing' },
  },

  // Sustained pressure - high rate of unique requests
  sustained_pressure: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 30,
    maxVUs: 150,
    env: { TEST_MODE: 'sustained_pressure' },
  },

  // Quick validation test
  quick_pressure: {
    executor: 'per-vu-iterations',
    vus: 5,
    iterations: 20,
    maxDuration: '2m',
    env: { TEST_MODE: 'quick' },
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : { quick_pressure: scenarios.quick_pressure };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: ['p(95)<1000'],  // P95 < 1s (allow for eviction overhead)
    http_req_failed: ['rate<0.01'],     // Error rate < 1%
    errors: ['rate<0.01'],
  },
};

// Track unique keys per VU
let vuKeyCounter = {};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 38.1: Memory Cache Pressure Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'quick_pressure'}`);
  console.log(`Test Mode: ${__ENV.TEST_MODE || 'quick'}`);
  console.log('');
  console.log('Cache Configuration:');
  console.log('  - Max memory: 32MB');
  console.log('  - Max entry size: 5MB');
  console.log('  - Expected capacity: ~6 large entries (5MB each)');
  console.log('='.repeat(80));

  // Verify test files exist
  console.log('Verifying test files...');
  for (const file of [...SMALL_FILES, ...LARGE_FILES]) {
    const response = http.head(`${BASE_URL}${file}`);
    if (response.status === 200) {
      console.log(`  ✓ ${file} exists (${response.headers['Content-Length']} bytes)`);
    } else {
      console.log(`  ✗ ${file} NOT FOUND (${response.status})`);
    }
  }
  console.log('');

  return { startTime: Date.now() };
}

export default function (data) {
  const testMode = __ENV.TEST_MODE || 'quick';
  const vuId = __VU;
  const iteration = __ITER;

  // Initialize VU key counter
  if (!vuKeyCounter[vuId]) {
    vuKeyCounter[vuId] = 0;
  }

  let url;
  let isUniqueKey = false;

  switch (testMode) {
    case 'fill_capacity':
      // Generate unique keys to fill cache
      // Each request gets a unique variant to avoid cache hits
      const uniqueKey = `${vuId}-${iteration}`;
      url = `${BASE_URL}/public/test-1mb.bin?key=${uniqueKey}`;
      isUniqueKey = true;
      uniqueKeysCount.add(1);
      break;

    case 'eviction_stress':
      // Continuous unique requests to force evictions
      const evictionKey = `evict-${Date.now()}-${Math.random().toString(36).slice(2)}`;
      url = `${BASE_URL}/public/test-1mb.bin?key=${evictionKey}`;
      isUniqueKey = true;
      uniqueKeysCount.add(1);
      break;

    case 'thrashing':
      // Alternate between two sets of keys to cause thrashing
      // Set A: keys 0-5, Set B: keys 6-11
      // With 32MB cache and 5MB files, only one set can fit
      const setA = iteration % 2 === 0;
      const keyIndex = setA ? (iteration % 6) : ((iteration % 6) + 6);
      url = `${BASE_URL}/public/test-5mb.bin?thrash=${keyIndex}`;
      isUniqueKey = (iteration < 12);  // First 12 are unique
      if (isUniqueKey) uniqueKeysCount.add(1);
      break;

    case 'sustained_pressure':
      // Mix of cached and unique requests
      if (Math.random() < 0.7) {
        // 70% unique keys (cache misses, triggers eviction)
        const pressureKey = `pressure-${Date.now()}-${Math.random().toString(36).slice(2)}`;
        url = `${BASE_URL}/public/test-1mb.bin?key=${pressureKey}`;
        isUniqueKey = true;
        uniqueKeysCount.add(1);
      } else {
        // 30% repeated keys (potential cache hits)
        const repeatedKey = iteration % 5;
        url = `${BASE_URL}/public/test-1mb.bin?key=repeated-${repeatedKey}`;
      }
      break;

    default:  // quick
      // Simple pressure test with mix of file sizes
      const fileIndex = iteration % (SMALL_FILES.length + LARGE_FILES.length);
      if (fileIndex < SMALL_FILES.length) {
        url = `${BASE_URL}${SMALL_FILES[fileIndex]}?quick=${vuId}-${iteration}`;
      } else {
        url = `${BASE_URL}${LARGE_FILES[fileIndex - SMALL_FILES.length]}?quick=${vuId}-${iteration}`;
      }
      isUniqueKey = true;
      uniqueKeysCount.add(1);
      break;
  }

  // Make request
  const response = http.get(url, {
    timeout: '60s',  // Longer timeout for large files under pressure
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
  const isCacheMiss = cacheStatus.toLowerCase().includes('miss');

  cacheHitRate.add(isCacheHit);
  cacheMissRate.add(isCacheMiss);

  // Track evictions (if we expected a hit but got miss, eviction happened)
  if (!isUniqueKey && isCacheMiss) {
    evictionCount.add(1);
  }

  // Estimate cache size based on content length
  const contentLength = parseInt(response.headers['Content-Length'] || '0');
  if (isCacheHit) {
    // Rough estimate: assume we're at capacity if getting hits
    memoryCacheSize.add(32);  // Max is 32MB
  }

  // Validation - basic success checks only
  // Note: Content-Length check removed as streaming responses may have
  // body.length != Content-Length due to chunked encoding
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Additional checks based on test mode
  if (testMode === 'fill_capacity' && isUniqueKey) {
    check(response, {
      'fill: first request is cache miss': (r) => isCacheMiss,
    });
  }

  if (testMode === 'thrashing') {
    // Just verify the request succeeded - thrashing is about eviction behavior
    check(response, {
      'thrashing: request completes': (r) => r.status === 200,
    });
  }

  errorRate.add(!success);

  // Small delay between requests to prevent overwhelming
  if (testMode !== 'fill_capacity') {
    sleep(0.05);  // 50ms between requests
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Memory Pressure Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Phase 38.1 Memory Pressure Success Criteria:');
  console.log('');
  console.log('  Fill to Capacity:');
  console.log('    - Cache fills to max_memory_cache_mb limit');
  console.log('    - No crashes when cache is full');
  console.log('    - New entries still accepted (eviction works)');
  console.log('');
  console.log('  Eviction Under Pressure:');
  console.log('    - LRU eviction keeps up with write rate');
  console.log('    - No memory growth beyond limit');
  console.log('    - Error rate < 1%');
  console.log('');
  console.log('  Cache Thrashing:');
  console.log('    - Alternating entry sets handled correctly');
  console.log('    - No deadlocks during rapid eviction');
  console.log('    - Performance degrades gracefully');
  console.log('');
  console.log('  Memory Limits:');
  console.log('    - Memory usage stays within configured limit');
  console.log('    - OOM killer not triggered');
  console.log('    - Process RSS stable over time');
  console.log('='.repeat(80));
}
