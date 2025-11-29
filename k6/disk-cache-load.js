/**
 * K6 Load Test: Phase 37.2 Disk Cache Load Tests
 *
 * This script tests disk cache performance under realistic production load.
 * Disk cache has lower throughput than memory cache due to file I/O.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - Proxy configured with disk cache enabled
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt, test-10kb.txt, test-100kb.txt, test-1mb.bin
 *
 * Usage:
 *   # Run cold cache test (50 RPS, 5 minutes)
 *   k6 run -e SCENARIO=cold_50rps k6/disk-cache-load.js
 *
 *   # Run hot cache test (100 RPS, 5 minutes)
 *   k6 run -e SCENARIO=hot_100rps k6/disk-cache-load.js
 *
 *   # Run eviction test (fills cache then continues writing)
 *   k6 run -e SCENARIO=eviction_stress k6/disk-cache-load.js
 *
 *   # Run sustained test (1 hour)
 *   k6 run -e SCENARIO=sustained_100rps_1hour k6/disk-cache-load.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const requestDuration = new Trend('request_duration_ms');
const requestCount = new Counter('total_requests');
const fileDescriptorGauge = new Gauge('file_descriptors');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files for disk cache (larger files to stress disk I/O)
const TEST_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
  '/public/test-100kb.txt',
  '/public/test-1mb.bin',
];

// Large file for eviction testing
const LARGE_FILES = [
  '/public/test-1mb.bin',
  '/public/test-10mb.bin',
];

// Scenarios based on Phase 37.2 requirements
// Note: Disk cache has lower RPS targets than memory cache
const scenarios = {
  // Cold Cache Scenario (All Misses) - Lower RPS due to disk I/O
  cold_50rps: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 10,
    maxVUs: 50,
  },
  cold_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 20,
    maxVUs: 100,
  },
  cold_500rps_stress: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },

  // Hot Cache Scenario (90% Hit Rate)
  hot_50rps: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 10,
    maxVUs: 50,
  },
  hot_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 20,
    maxVUs: 100,
  },
  hot_500rps: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
  },

  // Eviction Under Load - Use large files to trigger eviction
  eviction_stress: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '10m',
    preAllocatedVUs: 10,
    maxVUs: 50,
    env: { USE_LARGE_FILES: 'true', UNIQUE_KEYS: 'true' },
  },

  // Sustained Load (Endurance)
  sustained_100rps_1hour: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '1h',
    preAllocatedVUs: 20,
    maxVUs: 100,
  },
  sustained_50rps_30min: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '30m',
    preAllocatedVUs: 10,
    maxVUs: 50,
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : {
      // Default: run basic cold and hot disk tests
      cold_50rps: scenarios.cold_50rps,
      hot_100rps: scenarios.hot_100rps,
    };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: [
      'p(95)<500',  // P95 < 500ms for disk cache (higher than memory)
      'p(95)<100',  // P95 < 100ms for hot disk cache
    ],
    http_req_failed: ['rate<0.001'],  // Error rate < 0.1%
    errors: ['rate<0.001'],
  },
};

// Pre-warm cache for hot cache tests
export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 37.2: Disk Cache Load Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'default (cold_50rps + hot_100rps)'}`);
  console.log('');
  console.log('Note: Disk cache has lower throughput than memory cache.');
  console.log('Monitor file descriptors and disk I/O during test.');
  console.log('='.repeat(80));

  // For hot cache tests, pre-warm the cache
  if (selectedScenario && selectedScenario.startsWith('hot')) {
    console.log('Pre-warming disk cache...');
    for (const file of TEST_FILES) {
      // Multiple requests to ensure caching
      for (let i = 0; i < 5; i++) {
        http.get(`${BASE_URL}${file}`);
      }
    }
    // Wait for disk writes to complete
    sleep(2);
    console.log('Disk cache warmed up.');
  }

  return { startTime: Date.now() };
}

// Request counter for unique keys
let requestId = 0;

export default function (data) {
  const scenarioName = __ENV.SCENARIO || 'default';
  const useLargeFiles = __ENV.USE_LARGE_FILES === 'true';
  const useUniqueKeys = __ENV.UNIQUE_KEYS === 'true';

  // Select test file based on scenario
  const fileList = useLargeFiles ? LARGE_FILES : TEST_FILES;
  const fileIndex = requestId % fileList.length;
  const testFile = fileList[fileIndex];
  requestId++;

  // For cold cache or eviction: add unique query param
  // For hot cache: use same URLs to maximize cache hits
  const isColdOrEviction = scenarioName.startsWith('cold') || scenarioName.includes('eviction') || useUniqueKeys;
  const url = isColdOrEviction
    ? `${BASE_URL}${testFile}?nocache=${requestId}`
    : `${BASE_URL}${testFile}`;

  const response = http.get(url);

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);

  // Check for cache hit
  const cacheStatus = response.headers['X-Cache-Status'];
  const isCacheHit = cacheStatus === 'HIT' || cacheStatus === 'hit';
  cacheHitRate.add(isCacheHit);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Disk cache latency checks (higher thresholds than memory)
  if (scenarioName.startsWith('hot')) {
    check(response, {
      'hot disk cache: response time < 100ms': (r) => r.timings.duration < 100,
    });
  } else {
    check(response, {
      'cold disk cache: response time < 500ms': (r) => r.timings.duration < 500,
    });
  }

  errorRate.add(!success);

  // Small sleep to prevent overwhelming disk I/O
  if (scenarioName.includes('eviction')) {
    sleep(0.05); // 50ms between eviction requests
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('='.repeat(80));
  console.log('Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Phase 37.2 Success Criteria:');
  console.log('');
  console.log('  Cold Cache (50-500 RPS):');
  console.log('    - P95 latency < 500ms');
  console.log('    - Error rate < 0.1%');
  console.log('    - tokio::fs doesn\'t block async runtime');
  console.log('    - File descriptor count < 1000');
  console.log('');
  console.log('  Hot Cache (50-500 RPS):');
  console.log('    - P95 latency < 100ms');
  console.log('    - Error rate < 0.1%');
  console.log('    - Cache hit rate > 85%');
  console.log('    - Disk I/O doesn\'t overwhelm system');
  console.log('');
  console.log('  Eviction Under Load:');
  console.log('    - LRU eviction happens correctly');
  console.log('    - Old files deleted promptly');
  console.log('    - Disk space stays below threshold');
  console.log('    - No file descriptor leaks');
  console.log('');
  console.log('  Sustained Load:');
  console.log('    - Index file size doesn\'t grow unbounded');
  console.log('    - No disk space leaks');
  console.log('    - Performance consistent over time');
  console.log('='.repeat(80));
  console.log('');
  console.log('Manual Verification Commands:');
  console.log('  # Check file descriptors:');
  console.log('  lsof -p $(pgrep yatagarasu) | wc -l');
  console.log('');
  console.log('  # Check disk usage:');
  console.log('  du -sh /path/to/cache/directory');
  console.log('');
  console.log('  # Monitor I/O:');
  console.log('  iostat -x 1');
}
