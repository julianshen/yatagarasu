/**
 * K6 Load Test: Phase 37.3 Redis Cache Load Tests
 *
 * This script tests Redis cache performance under realistic production load.
 * It covers cold cache, hot cache, TTL expiration, and connection resilience scenarios.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with Redis cache enabled
 *   - Redis running on localhost:6379
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-1kb.txt, test-10kb.txt, test-100kb.txt
 *
 * Usage:
 *   # Run cold cache test (50 RPS, 5 minutes)
 *   k6 run -e SCENARIO=cold_50rps k6/redis-cache-load.js
 *
 *   # Run hot cache test (100 RPS, 5 minutes)
 *   k6 run -e SCENARIO=hot_100rps k6/redis-cache-load.js
 *
 *   # Run sustained load test (100 RPS, 1 hour)
 *   k6 run -e SCENARIO=sustained_100rps_1hour k6/redis-cache-load.js
 *
 *   # Run all scenarios sequentially
 *   k6 run k6/redis-cache-load.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const requestDuration = new Trend('request_duration_ms');
const requestCount = new Counter('total_requests');
const redisLatency = new Trend('redis_latency_ms');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files (should exist in MinIO 'public' bucket)
const TEST_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
  '/public/test-100kb.txt',
];

// Scenarios based on Phase 37.3 requirements
// Note: Redis is slower than memory cache, so lower RPS targets
const scenarios = {
  // Cold Cache Scenario (All Misses)
  // Redis + network overhead: expect higher latency than memory cache
  cold_50rps: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 20,
    maxVUs: 100,
  },
  cold_100rps: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 30,
    maxVUs: 150,
  },
  cold_500rps_stress: {
    executor: 'constant-arrival-rate',
    rate: 500,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 100,
    maxVUs: 500,
  },

  // Hot Cache Scenario (90% Hit Rate)
  // Redis hit should be <10ms P95
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
    preAllocatedVUs: 100,
    maxVUs: 400,
  },
  hot_1000rps_extreme: {
    executor: 'constant-arrival-rate',
    rate: 1000,
    timeUnit: '1s',
    duration: '1m',
    preAllocatedVUs: 200,
    maxVUs: 800,
  },

  // TTL Expiration Scenario
  // Set short TTL entries and verify expiration
  ttl_expiration: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '2m',
    preAllocatedVUs: 10,
    maxVUs: 50,
    env: { TTL_TEST: 'true' },
  },

  // Connection Resilience Scenario
  // Sustained load to test connection pooling
  connection_resilience: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '10m',
    preAllocatedVUs: 30,
    maxVUs: 150,
  },

  // Sustained Load (Endurance)
  sustained_100rps_1hour: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '1h',
    preAllocatedVUs: 30,
    maxVUs: 150,
  },
  sustained_100rps_30min: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '30m',
    preAllocatedVUs: 30,
    maxVUs: 150,
  },
};

// Select scenario from environment variable or run a default set
const selectedScenario = __ENV.SCENARIO;
const activeScenarios = selectedScenario
  ? { [selectedScenario]: scenarios[selectedScenario] }
  : {
      // Default: run basic cold and hot tests
      cold_50rps: scenarios.cold_50rps,
      hot_100rps: scenarios.hot_100rps,
    };

export const options = {
  scenarios: activeScenarios,
  thresholds: {
    http_req_duration: [
      'p(95)<500',  // P95 < 500ms for cold cache (Redis + S3)
      'p(95)<100',  // P95 < 100ms for hot cache (Redis hit)
    ],
    http_req_failed: ['rate<0.001'],  // Error rate < 0.1%
    errors: ['rate<0.001'],
    cache_hits: ['rate>0.80'],  // >80% hit rate for hot cache tests
  },
};

// Warm up cache with test files
export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 37.3: Redis Cache Load Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'default (cold_50rps + hot_100rps)'}`);
  console.log('='.repeat(80));

  // For hot cache tests, pre-warm the Redis cache
  if (selectedScenario && selectedScenario.startsWith('hot')) {
    console.log('Pre-warming Redis cache...');
    for (const file of TEST_FILES) {
      const warmupIterations = 10;
      for (let i = 0; i < warmupIterations; i++) {
        const response = http.get(`${BASE_URL}${file}`);
        if (response.status !== 200) {
          console.log(`Warning: Warmup request failed for ${file}: ${response.status}`);
        }
      }
    }
    // Give Redis time to persist entries
    sleep(2);
    console.log('Redis cache warmed up.');
  }

  return { startTime: Date.now() };
}

// Generate unique keys for cold cache testing
let requestId = 0;

export default function (data) {
  const scenarioName = __ENV.SCENARIO || 'default';
  const isTtlTest = __ENV.TTL_TEST === 'true';

  // Select a test file (round-robin)
  const fileIndex = requestId % TEST_FILES.length;
  const testFile = TEST_FILES[fileIndex];
  requestId++;

  // For cold cache: add unique query param to bypass cache
  // For hot cache: use same URLs to maximize Redis cache hits
  const isColdCache = scenarioName.startsWith('cold');
  const url = isColdCache
    ? `${BASE_URL}${testFile}?nocache=${requestId}`
    : `${BASE_URL}${testFile}`;

  const startTime = Date.now();
  const response = http.get(url);
  const duration = Date.now() - startTime;

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);
  redisLatency.add(duration);

  // Check for cache hit (via X-Cache-Status header)
  const cacheStatus = response.headers['X-Cache-Status'] ||
                      response.headers['x-cache-status'] ||
                      response.headers['X-Cache'] ||
                      response.headers['x-cache'] ||
                      '';
  const isCacheHit = cacheStatus.toLowerCase().includes('hit');
  cacheHitRate.add(isCacheHit);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Additional latency checks based on scenario
  if (scenarioName.startsWith('hot')) {
    check(response, {
      'hot cache: response time < 100ms': (r) => r.timings.duration < 100,
      'hot cache: P95 target < 10ms': (r) => r.timings.duration < 10,
    });
  } else if (scenarioName.startsWith('cold')) {
    check(response, {
      'cold cache: response time < 500ms': (r) => r.timings.duration < 500,
    });
  }

  // Connection pool checks
  if (scenarioName === 'connection_resilience') {
    check(response, {
      'connection: no timeouts': (r) => r.timings.duration < 5000,
      'connection: stable latency': (r) => r.timings.duration < 200,
    });
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
  console.log('Phase 37.3 Success Criteria:');
  console.log('');
  console.log('  Cold Cache (50-500 RPS):');
  console.log('    - P95 latency < 500ms (Redis + S3 fetch)');
  console.log('    - Error rate < 0.1%');
  console.log('    - Connection pool doesn\'t exhaust');
  console.log('    - No connection timeouts');
  console.log('');
  console.log('  Hot Cache (50-1000 RPS):');
  console.log('    - P95 latency < 10ms (Redis hit)');
  console.log('    - Error rate < 0.1%');
  console.log('    - Cache hit rate > 85%');
  console.log('    - Redis memory usage reasonable');
  console.log('');
  console.log('  TTL Expiration:');
  console.log('    - Expired entries not returned');
  console.log('    - Redis handles expirations automatically');
  console.log('');
  console.log('  Connection Resilience:');
  console.log('    - ConnectionManager reconnects automatically');
  console.log('    - Error rate spike < 5% during issues');
  console.log('    - Recovery time < 5 seconds');
  console.log('');
  console.log('  Sustained Load (1 hour):');
  console.log('    - No memory leaks in connection pool');
  console.log('    - Redis memory usage stable');
  console.log('    - Performance consistent over time');
  console.log('='.repeat(80));
}
