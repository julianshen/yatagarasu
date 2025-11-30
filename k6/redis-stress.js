/**
 * K6 Stress Test: Phase 38.3 Redis Cache Stress Tests
 *
 * This script stress tests the Redis cache with various scenarios:
 * - Connection pool exhaustion (10,000 concurrent requests)
 * - Large entry stress (1000 entries of 1MB each)
 * - Redis server stress monitoring
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with Redis cache enabled
 *   - Redis running on localhost:6379
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt, test-10kb.txt, test-100kb.txt, test-1mb.bin
 *
 * Usage:
 *   # Run connection pool exhaustion test
 *   k6 run -e SCENARIO=pool_exhaustion k6/redis-stress.js
 *
 *   # Run large entry stress test
 *   k6 run -e SCENARIO=large_entries k6/redis-stress.js
 *
 *   # Run quick validation test
 *   k6 run -e SCENARIO=quick k6/redis-stress.js
 *
 *   # Run server stress test
 *   k6 run -e SCENARIO=server_stress k6/redis-stress.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const connectionErrors = new Counter('connection_errors');
const requestDuration = new Trend('request_duration_ms');
const requestCount = new Counter('total_requests');
const successfulRequests = new Counter('successful_requests');
const failedRequests = new Counter('failed_requests');
const largeEntriesStored = new Counter('large_entries_stored');
const poolWaitTime = new Trend('pool_wait_time_ms');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files available in MinIO
const TEST_FILES = {
  small: '/public/test-1kb.txt',
  medium: '/public/test-10kb.txt',
  large: '/public/test-100kb.txt',
  xl: '/public/test-1mb.bin',
};

// Scenarios for different stress tests
const scenarios = {
  // Connection Pool Exhaustion: 10,000 concurrent requests
  // Tests max_pool_size limits and queue behavior
  pool_exhaustion: {
    executor: 'constant-arrival-rate',
    rate: 1000,              // 1000 requests per second
    timeUnit: '1s',
    duration: '1m',          // 1 minute sustained
    preAllocatedVUs: 200,
    maxVUs: 1000,            // Up to 1000 VUs
    env: { TEST_MODE: 'pool_exhaustion' },
  },

  // Burst test: Many concurrent requests at once
  burst_10k: {
    executor: 'per-vu-iterations',
    vus: 500,
    iterations: 20,          // 500 VUs * 20 = 10,000 requests
    maxDuration: '2m',
    env: { TEST_MODE: 'burst' },
  },

  // Large Entry Stress: 1000 entries of 1MB each (~1GB in Redis)
  large_entries: {
    executor: 'per-vu-iterations',
    vus: 10,
    iterations: 100,         // 10 VUs * 100 = 1000 unique large entries
    maxDuration: '10m',
    env: { TEST_MODE: 'large_entries' },
  },

  // Server Stress: Sustained high load on Redis
  server_stress: {
    executor: 'ramping-vus',
    startVUs: 10,
    stages: [
      { duration: '30s', target: 100 },   // Ramp up
      { duration: '2m', target: 200 },    // Sustained high
      { duration: '1m', target: 500 },    // Peak stress
      { duration: '30s', target: 100 },   // Cool down
      { duration: '30s', target: 10 },    // Recovery
    ],
    env: { TEST_MODE: 'server_stress' },
  },

  // Quick validation test
  quick: {
    executor: 'constant-vus',
    vus: 20,
    duration: '30s',
    env: { TEST_MODE: 'quick' },
  },

  // Connection resilience - test reconnection behavior
  connection_resilience: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 30,
    maxVUs: 100,
    env: { TEST_MODE: 'connection_resilience' },
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
    http_req_duration: ['p(95)<2000'],   // P95 < 2s (Redis can be slower under stress)
    http_req_failed: ['rate<0.10'],      // Error rate < 10% (allow some during pool exhaustion)
    errors: ['rate<0.10'],
    connection_errors: ['count<100'],    // Less than 100 connection errors
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 38.3: Redis Cache Stress Tests');
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

// Get test URL based on test mode and iteration
function getTestUrl(mode, vu, iter) {
  switch (mode) {
    case 'pool_exhaustion':
    case 'burst':
      // Use same files to maximize cache hits during pool stress
      // This isolates connection pool behavior from S3 latency
      const poolFile = iter % 3 === 0 ? TEST_FILES.small :
                       iter % 3 === 1 ? TEST_FILES.medium : TEST_FILES.large;
      return `${BASE_URL}${poolFile}`;

    case 'large_entries':
      // Each VU+iteration combo gets unique cache key
      // Forces new Redis entries to fill memory
      return `${BASE_URL}${TEST_FILES.xl}?large_entry=${vu}_${iter}`;

    case 'server_stress':
      // Mix of file sizes to stress Redis with varied payloads
      const stressIdx = (vu + iter) % 4;
      const stressFiles = [TEST_FILES.small, TEST_FILES.medium, TEST_FILES.large, TEST_FILES.xl];
      const baseFile = stressFiles[stressIdx];
      // 70% repeated keys (cache hits), 30% unique keys
      if (Math.random() < 0.7) {
        return `${BASE_URL}${baseFile}?stress_key=${iter % 50}`;
      } else {
        return `${BASE_URL}${baseFile}?stress_unique=${vu}_${iter}`;
      }

    case 'connection_resilience':
      // Steady pattern to observe connection behavior
      return `${BASE_URL}${TEST_FILES.medium}`;

    default:
      // Quick test - simple file rotation
      const quickIdx = iter % 3;
      const quickFiles = [TEST_FILES.small, TEST_FILES.medium, TEST_FILES.large];
      return `${BASE_URL}${quickFiles[quickIdx]}`;
  }
}

export default function (data) {
  const mode = data.mode;
  const url = getTestUrl(mode, __VU, __ITER);

  const startTime = Date.now();

  // Make request with appropriate timeout
  const timeout = mode === 'pool_exhaustion' || mode === 'burst' ? '30s' : '60s';
  const response = http.get(url, { timeout });

  const duration = Date.now() - startTime;

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

  // Track large entry storage
  if (mode === 'large_entries' && isCacheMiss && response.status === 200) {
    largeEntriesStored.add(1);
  }

  // Check for connection-related errors
  if (response.status === 0 ||
      response.error_code !== 0 ||
      (response.error && response.error.includes('connection'))) {
    connectionErrors.add(1);
  }

  // Estimate pool wait time from response timing
  // High wait_time suggests pool exhaustion queuing
  if (response.timings.waiting > 100) {
    poolWaitTime.add(response.timings.waiting);
  }

  // Validation
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Mode-specific checks
  if (mode === 'pool_exhaustion' || mode === 'burst') {
    check(response, {
      'pool: no connection refused': (r) => r.status !== 0,
      'pool: response time < 10s': (r) => r.timings.duration < 10000,
    });
  }

  if (mode === 'large_entries') {
    check(response, {
      'large: 1MB file retrieved': (r) => r.body && r.body.length > 900000,
    });
  }

  if (mode === 'server_stress') {
    check(response, {
      'stress: response time < 5s': (r) => r.timings.duration < 5000,
    });
  }

  if (success) {
    successfulRequests.add(1);
  } else {
    failedRequests.add(1);
    // Log errors (but limit to avoid spam)
    if (__ITER < 5 || __ITER % 100 === 0) {
      console.log(`Error: VU=${__VU}, iter=${__ITER}, status=${response.status}, ` +
                  `error=${response.error || 'none'}, duration=${duration}ms`);
    }
  }

  errorRate.add(!success);

  // Mode-specific sleep patterns
  switch (mode) {
    case 'pool_exhaustion':
    case 'burst':
      // Minimal sleep to maximize concurrent load
      // The executor handles rate limiting
      break;
    case 'large_entries':
      // Small delay between large file fetches
      sleep(0.1);  // 100ms
      break;
    case 'server_stress':
      // Variable sleep for realistic patterns
      sleep(Math.random() * 0.05);  // 0-50ms
      break;
    case 'connection_resilience':
      // Steady pace
      sleep(0.01);  // 10ms
      break;
    default:
      sleep(0.05);  // 50ms default
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Redis Cache Stress Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log(`Test Mode: ${data.mode}`);
  console.log('');
  console.log('Phase 38.3 Success Criteria:');
  console.log('');
  console.log('  Connection Pool Exhaustion:');
  console.log('    - 10,000 concurrent requests handled');
  console.log('    - Connection pool saturation measured');
  console.log('    - Queue waits observed when pool full');
  console.log('    - No connection refused errors');
  console.log('    - Graceful degradation (increased latency, not failures)');
  console.log('');
  console.log('  Large Entry Stress:');
  console.log('    - 1000 entries of 1MB each stored (~1GB)');
  console.log('    - Redis memory usage acceptable');
  console.log('    - Serialization handles large data');
  console.log('    - No MessagePack limits hit');
  console.log('');
  console.log('  Redis Server Stress:');
  console.log('    - Redis CPU/memory stable');
  console.log('    - Redis not a bottleneck');
  console.log('    - Evictions happen correctly (if configured)');
  console.log('');
  console.log('  General:');
  console.log('    - Error rate < 10%');
  console.log('    - P95 latency < 2s under stress');
  console.log('    - Connection errors < 100');
  console.log('='.repeat(80));
}
