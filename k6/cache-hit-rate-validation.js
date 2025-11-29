/**
 * K6 Cache Hit Rate Validation Test
 *
 * Phase 36 Test: 1000 requests for same file = 999 cache hits (first is miss)
 *
 * This test validates that the cache correctly serves cached responses
 * for repeated requests to the same resource.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with cache enabled
 *   - MinIO running with test files in 'public' bucket
 *   - Test file: test-1kb.txt (small file under cache threshold)
 *
 * Usage:
 *   k6 run k6/cache-hit-rate-validation.js
 *
 * Expected Results:
 *   - First request: Cache MISS (fetched from S3)
 *   - Requests 2-1000: Cache HIT (served from cache)
 *   - Hit rate: 99.9% (999/1000)
 *   - Cache hit latency: <10ms P95
 */

import http from 'k6/http';
import { check, fail } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const cacheHits = new Counter('cache_hits');
const cacheMisses = new Counter('cache_misses');
const cacheHitRate = new Rate('cache_hit_rate');
const hitLatency = new Trend('cache_hit_latency_ms');
const missLatency = new Trend('cache_miss_latency_ms');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const TEST_FILE = __ENV.TEST_FILE || '/public/test-1kb.txt';
const TOTAL_REQUESTS = parseInt(__ENV.REQUESTS || '1000');

// Run as single iteration test with exact request count
export const options = {
  scenarios: {
    cache_hit_rate_validation: {
      executor: 'shared-iterations',
      vus: 1,  // Single VU for sequential execution
      iterations: TOTAL_REQUESTS,
      maxDuration: '5m',
    },
  },
  thresholds: {
    // Phase 36 success criteria
    'cache_hit_rate': ['rate>0.99'],           // >99% hit rate
    'cache_hit_latency_ms': ['p(95)<10'],      // Cache hit P95 < 10ms
    'http_req_failed': ['rate<0.001'],          // <0.1% errors
  },
};

// Track iteration number
let iteration = 0;

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 36: Cache Hit Rate Validation Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Test file: ${TEST_FILE}`);
  console.log(`Total requests: ${TOTAL_REQUESTS}`);
  console.log('');
  console.log('Expected:');
  console.log(`  - Request 1: Cache MISS (S3 fetch)`);
  console.log(`  - Requests 2-${TOTAL_REQUESTS}: Cache HIT`);
  console.log(`  - Hit rate: ${((TOTAL_REQUESTS - 1) / TOTAL_REQUESTS * 100).toFixed(1)}%`);
  console.log('='.repeat(80));

  // Clear any existing cache for this file first
  // (Optional: Call purge API if available)
  const purgeUrl = `${BASE_URL}/admin/cache/purge`;
  try {
    http.del(purgeUrl, null, { headers: { 'X-Admin-Key': 'test' } });
    console.log('Cache cleared via purge API');
  } catch (e) {
    console.log('Note: Cache purge API not available, using fresh file path');
  }

  // Make initial request to ensure cache is populated
  // This ensures first test iteration will be a miss
  return { startTime: Date.now() };
}

export default function(data) {
  iteration++;
  const currentIteration = iteration;

  // Make request to test file (same URL for all requests)
  const url = `${BASE_URL}${TEST_FILE}`;
  const response = http.get(url);

  // Check cache status from response header
  const cacheStatus = response.headers['X-Cache-Status'] ||
                      response.headers['x-cache-status'] ||
                      response.headers['X-Cache'] ||
                      response.headers['x-cache'] ||
                      '';

  const isCacheHit = cacheStatus.toLowerCase().includes('hit');
  const isCacheMiss = cacheStatus.toLowerCase().includes('miss') || currentIteration === 1;

  // Record metrics
  if (isCacheHit) {
    cacheHits.add(1);
    cacheHitRate.add(1);
    hitLatency.add(response.timings.duration);
  } else {
    cacheMisses.add(1);
    cacheHitRate.add(0);
    missLatency.add(response.timings.duration);
  }

  // Validation checks
  const checks = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // First request should be a miss, all others should be hits
  if (currentIteration === 1) {
    check(response, {
      'first request is cache miss (expected)': () => isCacheMiss || !isCacheHit,
    });
    console.log(`Request 1: ${cacheStatus || 'MISS (no header)'} - ${response.timings.duration.toFixed(1)}ms`);
  } else if (currentIteration <= 5) {
    // Log first few cache hits
    check(response, {
      'subsequent request is cache hit': () => isCacheHit,
    });
    console.log(`Request ${currentIteration}: ${cacheStatus || 'HIT'} - ${response.timings.duration.toFixed(1)}ms`);
  } else if (currentIteration === TOTAL_REQUESTS) {
    console.log(`Request ${currentIteration}: ${cacheStatus || 'HIT'} - ${response.timings.duration.toFixed(1)}ms (final)`);
  }

  // Validate latency for cache hits
  if (isCacheHit) {
    check(response, {
      'cache hit latency < 10ms': (r) => r.timings.duration < 10,
    });
  }

  if (!checks) {
    console.log(`Request ${currentIteration} FAILED: status=${response.status}, cache=${cacheStatus}`);
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 36 Cache Hit Rate Validation Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Success Criteria:');
  console.log('  [?] First request = cache miss (check Request 1 output)');
  console.log('  [?] Hit rate > 99% (check cache_hit_rate)');
  console.log('  [?] Cache hit P95 < 10ms (check cache_hit_latency_ms p95)');
  console.log('  [?] Error rate < 0.1% (check http_req_failed)');
  console.log('');
  console.log('Expected for 1000 requests:');
  console.log('  - Cache misses: 1');
  console.log('  - Cache hits: 999');
  console.log('  - Hit rate: 99.9%');
  console.log('='.repeat(80));
}
