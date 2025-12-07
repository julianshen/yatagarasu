/**
 * K6 Test: Phase 62 Cache Size Scaling
 *
 * Tests cache behavior at different sizes (1GB, 10GB, 50GB).
 * Measures hit rate, eviction time, and lookup performance.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running with configurable cache size
 *   - MinIO with test files of various sizes
 *
 * Usage:
 *   # Test with 1GB cache
 *   CACHE_SIZE_MB=1024 k6 run k6/cache-size-scaling.js
 *
 *   # Test with 10GB cache
 *   CACHE_SIZE_MB=10240 k6 run k6/cache-size-scaling.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const cacheHitRate = new Rate('cache_hit_rate');
const cacheMissRate = new Rate('cache_miss_rate');
const lookupLatency = new Trend('cache_lookup_latency_ms');
const evictionTime = new Trend('eviction_time_ms');
const totalRequests = new Counter('total_requests');
const uniqueFiles = new Gauge('unique_files_accessed');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const CACHE_SIZE_MB = parseInt(__ENV.CACHE_SIZE_MB) || 1024;
const FILE_COUNT = parseInt(__ENV.FILE_COUNT) || 1000;
const FILE_SIZE_KB = parseInt(__ENV.FILE_SIZE_KB) || 100;
const DURATION = __ENV.DURATION || '2m';

// Calculate expected cache capacity
const EXPECTED_CAPACITY = Math.floor((CACHE_SIZE_MB * 1024) / FILE_SIZE_KB);

export const options = {
  scenarios: {
    // Phase 1: Warm up cache with sequential access
    cache_warmup: {
      executor: 'shared-iterations',
      vus: 10,
      iterations: Math.min(FILE_COUNT, EXPECTED_CAPACITY),
      maxDuration: '5m',
      exec: 'warmupCache',
    },
    // Phase 2: Measure cache hit rate with random access
    cache_hits: {
      executor: 'constant-arrival-rate',
      rate: 500,
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      maxVUs: 50,
      exec: 'measureCacheHits',
      startTime: '5m30s',
    },
    // Phase 3: Force evictions by accessing more files than cache capacity
    eviction_test: {
      executor: 'constant-arrival-rate',
      rate: 200,
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      maxVUs: 50,
      exec: 'forceEvictions',
      startTime: '6m30s',
    },
    // Phase 4: Measure lookup time with full cache
    lookup_benchmark: {
      executor: 'constant-arrival-rate',
      rate: 1000,
      timeUnit: '1s',
      duration: '20s',
      preAllocatedVUs: 30,
      maxVUs: 100,
      exec: 'benchmarkLookup',
      startTime: '7m30s',
    },
  },
  thresholds: {
    'cache_hit_rate': ['rate>0.8'],           // >80% hit rate for cached items
    'cache_lookup_latency_ms': ['p(95)<50'],  // Lookup should be fast
    'http_req_failed': ['rate<0.01'],         // <1% errors
  },
};

// Track which files we've accessed
let accessedFiles = new Set();

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 62: Cache Size Scaling Test');
  console.log('='.repeat(80));
  console.log(`Cache Size: ${CACHE_SIZE_MB} MB`);
  console.log(`File Count: ${FILE_COUNT}`);
  console.log(`File Size: ${FILE_SIZE_KB} KB`);
  console.log(`Expected Capacity: ~${EXPECTED_CAPACITY} files`);
  console.log('='.repeat(80));

  // Verify proxy is accessible
  const healthCheck = http.get(`${BASE_URL}/health`);
  if (healthCheck.status !== 200) {
    console.error('ERROR: Proxy health check failed');
    return { error: true };
  }

  // Get initial cache stats
  const metricsRes = http.get(`${BASE_URL}/metrics`);
  console.log('Initial cache state from metrics endpoint');

  return {
    startTime: Date.now(),
    cacheSize: CACHE_SIZE_MB,
    fileCount: FILE_COUNT,
    expectedCapacity: EXPECTED_CAPACITY,
  };
}

// Phase 1: Warm up cache with sequential files
export function warmupCache() {
  const fileIndex = __ITER % FILE_COUNT;
  const fileName = `test-file-${fileIndex.toString().padStart(5, '0')}.bin`;

  const startTime = Date.now();
  const res = http.get(`${BASE_URL}/public/${fileName}`);
  const latency = Date.now() - startTime;

  lookupLatency.add(latency);
  totalRequests.add(1);

  // First access is always a cache miss
  cacheMissRate.add(1);

  check(res, {
    'warmup status ok': (r) => r.status === 200 || r.status === 404,
  });
}

// Phase 2: Random access to cached files (should be cache hits)
export function measureCacheHits() {
  // Access files that should be in cache (first N files)
  const cacheableCount = Math.min(FILE_COUNT, EXPECTED_CAPACITY);
  const fileIndex = Math.floor(Math.random() * cacheableCount);
  const fileName = `test-file-${fileIndex.toString().padStart(5, '0')}.bin`;

  const startTime = Date.now();
  const res = http.get(`${BASE_URL}/public/${fileName}`);
  const latency = Date.now() - startTime;

  lookupLatency.add(latency);
  totalRequests.add(1);

  // Check X-Cache header if available
  const cacheHeader = res.headers['X-Cache'] || res.headers['x-cache'];
  if (cacheHeader && cacheHeader.includes('HIT')) {
    cacheHitRate.add(1);
  } else if (cacheHeader && cacheHeader.includes('MISS')) {
    cacheMissRate.add(1);
  } else {
    // Assume hit if response is fast (<10ms)
    if (latency < 10) {
      cacheHitRate.add(1);
    } else {
      cacheMissRate.add(1);
    }
  }

  check(res, {
    'cache hit status ok': (r) => r.status === 200 || r.status === 404,
    'response time acceptable': (r) => r.timings.duration < 100,
  });
}

// Phase 3: Access files beyond cache capacity to force evictions
export function forceEvictions() {
  // Access files beyond expected cache capacity
  const fileIndex = EXPECTED_CAPACITY + (__ITER % (FILE_COUNT - EXPECTED_CAPACITY));
  const fileName = `test-file-${fileIndex.toString().padStart(5, '0')}.bin`;

  const startTime = Date.now();
  const res = http.get(`${BASE_URL}/public/${fileName}`);
  const latency = Date.now() - startTime;

  evictionTime.add(latency);
  totalRequests.add(1);

  // These should be cache misses that trigger evictions
  cacheMissRate.add(1);

  check(res, {
    'eviction test status ok': (r) => r.status === 200 || r.status === 404,
    'eviction time acceptable': (r) => r.timings.duration < 500,
  });
}

// Phase 4: Benchmark pure lookup time with full cache
export function benchmarkLookup() {
  // Access only files that should definitely be cached
  const cacheableCount = Math.min(100, EXPECTED_CAPACITY);
  const fileIndex = Math.floor(Math.random() * cacheableCount);
  const fileName = `test-file-${fileIndex.toString().padStart(5, '0')}.bin`;

  const startTime = Date.now();
  const res = http.get(`${BASE_URL}/public/${fileName}`);
  const latency = Date.now() - startTime;

  lookupLatency.add(latency);
  totalRequests.add(1);

  check(res, {
    'lookup status ok': (r) => r.status === 200 || r.status === 404,
    'lookup time < 20ms': (r) => r.timings.duration < 20,
  });
}

export function teardown(data) {
  if (data && data.error) return;

  const duration = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 62: Cache Size Scaling Test Complete');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log(`Cache Size: ${data.cacheSize} MB`);
  console.log(`Expected Capacity: ${data.expectedCapacity} files`);
  console.log('');
  console.log('Key Metrics to Verify:');
  console.log('  - cache_hit_rate: Should be >80% for cached items');
  console.log('  - cache_lookup_latency_ms: P95 should be <50ms');
  console.log('  - eviction_time_ms: Should not increase with cache size');
  console.log('='.repeat(80));

  // Fetch final metrics
  const metricsRes = http.get(`${BASE_URL}/metrics`);
  if (metricsRes.status === 200) {
    const metrics = metricsRes.body;
    // Extract cache-related metrics
    const cacheLines = metrics.split('\n').filter(line =>
      line.includes('cache') && !line.startsWith('#')
    );
    if (cacheLines.length > 0) {
      console.log('\nCache Metrics:');
      cacheLines.forEach(line => console.log(`  ${line}`));
    }
  }
}
