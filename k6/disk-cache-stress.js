/**
 * K6 Stress Test: Phase 38.2 Disk Cache Stress Tests
 *
 * This script stress tests the disk cache with various scenarios:
 * - Large cache population (many files)
 * - Rapid file operations (high throughput)
 * - Disk space exhaustion (eviction testing)
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with disk cache enabled
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt, test-10kb.txt, test-100kb.txt
 *
 * Usage:
 *   # Run large cache population test
 *   k6 run -e SCENARIO=large_cache k6/disk-cache-stress.js
 *
 *   # Run rapid operations test
 *   k6 run -e SCENARIO=rapid_ops k6/disk-cache-stress.js
 *
 *   # Run disk space exhaustion test
 *   k6 run -e SCENARIO=exhaustion k6/disk-cache-stress.js
 *
 *   # Run quick validation test
 *   k6 run -e SCENARIO=quick k6/disk-cache-stress.js
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
const filesPopulated = new Counter('files_populated');
const evictionCount = new Counter('evictions_detected');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files available in MinIO
// Using different sizes to stress disk I/O patterns
const TEST_FILES = {
  small: '/public/test-1kb.txt',    // 1KB - high volume
  medium: '/public/test-10kb.txt',  // 10KB - moderate
  large: '/public/test-100kb.txt',  // 100KB - disk I/O intensive
  xl: '/public/test-1mb.bin',       // 1MB - very disk intensive
};

// Scenarios for different stress tests
const scenarios = {
  // Large cache population: Many unique files
  // Simulates 10,000 files by using file variations
  large_cache: {
    executor: 'per-vu-iterations',
    vus: 10,
    iterations: 1000,  // 10 VUs * 1000 = 10,000 requests
    maxDuration: '15m',
    env: { TEST_MODE: 'large_cache' },
  },

  // Rapid file operations: High throughput
  // Target: 1000+ operations per second
  rapid_ops: {
    executor: 'constant-arrival-rate',
    rate: 500,           // 500 requests per second
    timeUnit: '1s',
    duration: '1m',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_MODE: 'rapid_ops' },
  },

  // Disk space exhaustion: Fill cache to trigger eviction
  // Uses larger files to fill disk faster
  exhaustion: {
    executor: 'constant-arrival-rate',
    rate: 100,           // 100 requests per second
    timeUnit: '1s',
    duration: '3m',      // 3 minutes of sustained writes
    preAllocatedVUs: 20,
    maxVUs: 100,
    env: { TEST_MODE: 'exhaustion' },
  },

  // Quick validation test
  quick: {
    executor: 'constant-vus',
    vus: 10,
    duration: '30s',
    env: { TEST_MODE: 'quick' },
  },

  // Thrashing test: Alternating read/write patterns
  thrashing: {
    executor: 'ramping-vus',
    startVUs: 5,
    stages: [
      { duration: '30s', target: 50 },   // Ramp up
      { duration: '1m', target: 50 },    // Sustained
      { duration: '30s', target: 100 },  // Spike
      { duration: '1m', target: 100 },   // Sustained high
      { duration: '30s', target: 10 },   // Cool down
    ],
    env: { TEST_MODE: 'thrashing' },
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : { quick: scenarios.quick };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: ['p(95)<1000'],  // P95 < 1s (disk I/O can be slower)
    http_req_failed: ['rate<0.05'],     // Error rate < 5%
    errors: ['rate<0.05'],              // Error rate < 5%
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 38.2: Disk Cache Stress Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'quick'}`);
  console.log(`Test Mode: ${__ENV.TEST_MODE || 'quick'}`);
  console.log('='.repeat(80));

  // Verify test files exist
  console.log('Verifying test files...');
  const files = [
    TEST_FILES.small,
    TEST_FILES.medium,
    TEST_FILES.large,
    TEST_FILES.xl,
  ];

  for (const file of files) {
    const response = http.head(`${BASE_URL}${file}`);
    if (response.status === 200) {
      console.log(`  OK: ${file}`);
    } else {
      console.log(`  MISSING: ${file} (status: ${response.status})`);
    }
  }

  console.log('');
  console.log('Starting stress test...');

  return {
    startTime: Date.now(),
    mode: __ENV.TEST_MODE || 'quick',
  };
}

// Get file path based on test mode and iteration
function getTestFile(mode, vu, iter) {
  switch (mode) {
    case 'large_cache':
      // Vary the file by adding query param to simulate many unique files
      // This forces different cache keys while using same S3 objects
      const fileIndex = (vu * 1000 + iter) % 4;
      const files = [TEST_FILES.small, TEST_FILES.medium, TEST_FILES.large, TEST_FILES.xl];
      const baseFile = files[fileIndex];
      // Add unique cache-busting param to create unique cache entries
      return `${baseFile}?variant=${vu}_${iter}`;

    case 'rapid_ops':
      // Use smaller files for rapid operations
      return TEST_FILES.small;

    case 'exhaustion':
      // Use larger files to fill disk cache faster
      // Rotate through larger files
      const exhaustIdx = (vu + iter) % 2;
      return exhaustIdx === 0 ? TEST_FILES.large : TEST_FILES.xl;

    case 'thrashing':
      // Alternate between different file sizes rapidly
      const thrashIdx = (vu + iter) % 4;
      const thrashFiles = [TEST_FILES.small, TEST_FILES.medium, TEST_FILES.large, TEST_FILES.xl];
      return thrashFiles[thrashIdx];

    default:
      return TEST_FILES.medium;
  }
}

export default function (data) {
  const mode = data.mode;
  const testFile = getTestFile(mode, __VU, __ITER);

  // Make request
  const response = http.get(`${BASE_URL}${testFile}`, {
    timeout: '30s',  // Longer timeout for disk I/O
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

  // Track cache population (misses that then succeed indicate new cache entries)
  if (isCacheMiss && response.status === 200) {
    filesPopulated.add(1);
  }

  // Validation
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  if (success) {
    successfulRequests.add(1);
  } else {
    failedRequests.add(1);
    // Log first few errors
    if (failedRequests.value < 10) {
      console.log(`Error: status=${response.status}, file=${testFile}, duration=${response.timings.duration}ms`);
    }
  }

  errorRate.add(!success);

  // Mode-specific sleep patterns
  switch (mode) {
    case 'large_cache':
      // Small sleep between large cache population requests
      sleep(0.05);  // 50ms
      break;
    case 'rapid_ops':
      // Minimal sleep for rapid operations
      sleep(0.001);  // 1ms
      break;
    case 'exhaustion':
      // Moderate sleep for exhaustion test
      sleep(0.01);  // 10ms
      break;
    case 'thrashing':
      // Variable sleep to create unpredictable patterns
      sleep(Math.random() * 0.02);  // 0-20ms random
      break;
    default:
      sleep(0.05);  // 50ms default
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Disk Cache Stress Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log(`Test Mode: ${data.mode}`);
  console.log('');
  console.log('Phase 38.2 Success Criteria:');
  console.log('');
  console.log('  Large Cache Size:');
  console.log('    - 10,000 files populated successfully');
  console.log('    - LRU eviction operates correctly');
  console.log('    - No index corruption');
  console.log('');
  console.log('  Rapid File Operations:');
  console.log('    - 500+ ops/sec sustained');
  console.log('    - File system keeps up');
  console.log('    - No file descriptor leaks');
  console.log('');
  console.log('  Disk Space Exhaustion:');
  console.log('    - Eviction triggered when full');
  console.log('    - Space reclaimed properly');
  console.log('    - No "disk full" errors');
  console.log('');
  console.log('  General:');
  console.log('    - Error rate < 5%');
  console.log('    - P95 latency < 1s');
  console.log('    - No crashes');
  console.log('='.repeat(80));
}
