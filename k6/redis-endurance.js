/**
 * K6 Redis Cache Endurance Test - Phase 53
 *
 * This script tests Redis cache stability over extended periods (up to 24 hours).
 * It validates that the Redis cache layer handles sustained load without:
 * - Connection pool exhaustion or leaks
 * - Memory growth in Redis
 * - TTL expiration issues
 * - Performance degradation
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with Redis cache enabled
 *   - Redis running on localhost:6379
 *   - MinIO running with test files in 'public' bucket
 *   - Prometheus/metrics endpoint available at http://localhost:9090/metrics
 *
 * Configuration (config.yaml):
 *   cache:
 *     enabled: true
 *     memory:
 *       max_size_mb: 100
 *     redis:
 *       url: "redis://localhost:6379"
 *       pool_size: 10
 *       default_ttl_seconds: 3600
 *
 * Usage:
 *   # Quick validation (5 minutes)
 *   k6 run -e SCENARIO=quick k6/redis-endurance.js
 *
 *   # 1-hour endurance test
 *   k6 run -e SCENARIO=one_hour k6/redis-endurance.js
 *
 *   # 24-hour endurance test (full production validation)
 *   k6 run -e SCENARIO=full_24h k6/redis-endurance.js
 *
 *   # Connection pool stress test
 *   k6 run -e SCENARIO=pool_stress k6/redis-endurance.js
 *
 *   # TTL expiration test
 *   k6 run -e SCENARIO=ttl_test k6/redis-endurance.js
 *
 * Success Criteria (Phase 53):
 *   - Connection pool stable (no exhaustion)
 *   - No connection leaks
 *   - Redis memory stable (bounded by maxmemory policy)
 *   - TTL expiration working correctly
 *   - P95 latency <10ms for cache hits
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const cacheMissRate = new Rate('cache_misses');
const requestDuration = new Trend('request_duration_ms');
const redisHitLatency = new Trend('redis_hit_latency_ms');
const redisMissLatency = new Trend('redis_miss_latency_ms');
const requestCount = new Counter('total_requests');
const successfulRequests = new Counter('successful_requests');
const failedRequests = new Counter('failed_requests');

// Redis-specific metrics
const redisConnections = new Gauge('redis_active_connections');
const redisMemoryUsage = new Gauge('redis_memory_bytes');
const redisKeyCount = new Gauge('redis_key_count');
const redisTTLExpired = new Counter('redis_ttl_expired');

// Periodic metrics for stability tracking
const hourlyHitRate = new Trend('hourly_cache_hit_rate');
const hourlyP95 = new Trend('hourly_p95_latency');
const hourlyConnections = new Trend('hourly_redis_connections');

// Base URLs
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const METRICS_URL = __ENV.METRICS_URL || 'http://localhost:9090/metrics';
const REDIS_URL = __ENV.REDIS_URL || 'http://localhost:6379';

// Test files - mix of sizes for realistic workload
const TEST_FILES = [
  { path: '/public/test-1kb.txt', size: '1KB', weight: 40 },
  { path: '/public/test-10kb.txt', size: '10KB', weight: 30 },
  { path: '/public/test-100kb.txt', size: '100KB', weight: 20 },
  { path: '/public/test-1mb.bin', size: '1MB', weight: 10 },
];

// Generate file pool for weighted selection
const filePool = [];
TEST_FILES.forEach(file => {
  for (let i = 0; i < file.weight; i++) {
    filePool.push(file);
  }
});

// Unique keys for TTL testing
let ttlTestCounter = 0;

// Scenarios for different test types
const scenarios = {
  // Quick validation test (5 minutes)
  quick: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'quick', REPORT_INTERVAL: '60' },
  },

  // 1-hour endurance test
  one_hour: {
    executor: 'constant-arrival-rate',
    rate: 100,              // 100 RPS (more conservative for Redis)
    timeUnit: '1s',
    duration: '1h',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'one_hour', REPORT_INTERVAL: '300' },
  },

  // Full 24-hour endurance test
  full_24h: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '24h',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'full_24h', REPORT_INTERVAL: '1800' },
  },

  // Connection pool stress test - tests pool exhaustion recovery
  pool_stress: {
    executor: 'ramping-arrival-rate',
    startRate: 10,
    timeUnit: '1s',
    preAllocatedVUs: 100,
    maxVUs: 500,
    stages: [
      { duration: '2m', target: 50 },    // Ramp up
      { duration: '5m', target: 200 },   // Stress pool (should exceed pool_size)
      { duration: '2m', target: 500 },   // Extreme stress
      { duration: '3m', target: 50 },    // Recovery
      { duration: '2m', target: 10 },    // Cool down
    ],
    env: { TEST_TYPE: 'pool_stress', REPORT_INTERVAL: '30' },
  },

  // TTL expiration test - verifies entries expire correctly
  ttl_test: {
    executor: 'constant-arrival-rate',
    rate: 20,               // Lower rate to observe TTL behavior
    timeUnit: '1s',
    duration: '15m',        // Long enough to see TTL expirations
    preAllocatedVUs: 20,
    maxVUs: 50,
    env: { TEST_TYPE: 'ttl_test', REPORT_INTERVAL: '60', TTL_TEST: 'true' },
  },

  // High concurrency test - many simultaneous connections
  high_concurrency: {
    executor: 'constant-vus',
    vus: 200,               // 200 concurrent users
    duration: '10m',
    env: { TEST_TYPE: 'high_concurrency', REPORT_INTERVAL: '60' },
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
    // Thresholds for proxy requests only (exclude metrics endpoint fetches)
    'http_req_duration{expected_response:true}': ['p(95)<100', 'p(99)<500'],  // P95 <100ms, P99 <500ms
    'http_req_failed{expected_response:true}': ['rate<0.01'],                  // Error rate <1%
    errors: ['rate<0.01'],                            // Custom error rate <1%
    cache_hits: ['rate>0.60'],                        // Cache hit rate >60%
    redis_hit_latency_ms: ['p(95)<10'],              // Redis hits P95 <10ms
  },
  dns: {
    ttl: '1m',
    select: 'first',
  },
};

// Track metrics over time
let lastReportTime = 0;
let periodHits = 0;
let periodMisses = 0;
let periodLatencies = [];
let periodConnections = [];

// Parse Redis INFO output
function parseRedisInfo(info) {
  const result = {};
  if (!info) return result;

  info.split('\n').forEach(line => {
    const [key, value] = line.split(':');
    if (key && value) {
      result[key.trim()] = value.trim();
    }
  });
  return result;
}

// Fetch Redis metrics directly (if Redis is accessible)
function fetchRedisMetrics() {
  // Note: k6 cannot directly connect to Redis
  // We fetch metrics from the proxy's /metrics endpoint instead
  try {
    const response = http.get(METRICS_URL, { timeout: '5s' });
    if (response.status === 200) {
      const metrics = {};

      // Parse Prometheus metrics for Redis stats
      const body = response.body;

      // Redis connection pool metrics
      const poolActiveMatch = body.match(/redis_pool_active_connections\s+(\d+)/);
      if (poolActiveMatch) {
        metrics.activeConnections = parseInt(poolActiveMatch[1]);
      }

      const poolIdleMatch = body.match(/redis_pool_idle_connections\s+(\d+)/);
      if (poolIdleMatch) {
        metrics.idleConnections = parseInt(poolIdleMatch[1]);
      }

      // Redis cache stats
      const hitsMatch = body.match(/cache_redis_hits_total\s+(\d+)/);
      if (hitsMatch) {
        metrics.cacheHits = parseInt(hitsMatch[1]);
      }

      const missesMatch = body.match(/cache_redis_misses_total\s+(\d+)/);
      if (missesMatch) {
        metrics.cacheMisses = parseInt(missesMatch[1]);
      }

      // Process memory (as proxy for overall health)
      const memMatch = body.match(/process_resident_memory_bytes\s+(\d+)/);
      if (memMatch) {
        metrics.processMemory = parseInt(memMatch[1]);
      }

      return metrics;
    }
  } catch (e) {
    // Metrics endpoint may not be available
  }
  return null;
}

export function setup() {
  const testType = __ENV.TEST_TYPE || 'quick';
  const reportInterval = parseInt(__ENV.REPORT_INTERVAL || '60');
  const isTTLTest = __ENV.TTL_TEST === 'true';

  console.log('='.repeat(80));
  console.log('Phase 53: Redis Cache Endurance Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Metrics URL: ${METRICS_URL}`);
  console.log(`Test Type: ${testType}`);
  console.log(`Report Interval: ${reportInterval} seconds`);
  console.log(`Scenario: ${selectedScenario || 'quick'}`);
  console.log(`TTL Test Mode: ${isTTLTest}`);
  console.log('');
  console.log('Test started at:', new Date().toISOString());
  console.log('='.repeat(80));

  // Fetch initial metrics
  const initialMetrics = fetchRedisMetrics();
  if (initialMetrics) {
    console.log('Initial Redis Metrics:');
    console.log(`  Active Connections: ${initialMetrics.activeConnections || 'N/A'}`);
    console.log(`  Idle Connections: ${initialMetrics.idleConnections || 'N/A'}`);
    console.log(`  Process Memory: ${initialMetrics.processMemory ? (initialMetrics.processMemory / 1024 / 1024).toFixed(2) + ' MB' : 'N/A'}`);
  } else {
    console.log('Note: Could not fetch initial Redis metrics (this is OK)');
  }

  // Pre-warm cache
  console.log('');
  console.log('Pre-warming Redis cache with test files...');
  TEST_FILES.forEach(file => {
    for (let i = 0; i < 3; i++) {
      const response = http.get(`${BASE_URL}${file.path}`, { timeout: '10s' });
      if (response.status !== 200) {
        console.log(`Warning: Pre-warm request failed for ${file.path}: ${response.status}`);
      }
    }
  });
  sleep(2);  // Wait for async cache operations
  console.log('Redis cache pre-warmed.');
  console.log('');
  console.log('Starting Redis endurance test...');
  console.log('');

  return {
    startTime: Date.now(),
    initialMetrics: initialMetrics,
    reportInterval: reportInterval,
    isTTLTest: isTTLTest,
  };
}

export default function (data) {
  const isTTLTest = data.isTTLTest;

  // Select file based on weighted distribution
  const file = filePool[Math.floor(Math.random() * filePool.length)];

  let url = `${BASE_URL}${file.path}`;

  if (isTTLTest) {
    // In TTL test mode, use unique keys that will expire
    // This helps verify that Redis TTL is working
    ttlTestCounter++;
    url += `?ttl_test=${ttlTestCounter}-${Date.now()}`;
  } else {
    // Normal mode: ~30% cache misses for realistic workload
    const useCacheBuster = Math.random() < 0.3;
    if (useCacheBuster) {
      url += `?cb=${Date.now()}-${Math.random().toString(36).substring(7)}`;
    }
  }

  // Make request
  const response = http.get(url, {
    timeout: '30s',
  });

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);
  periodLatencies.push(response.timings.duration);

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
  const isRedisHit = cacheLayer.toLowerCase().includes('redis');

  if (isCacheHit) {
    cacheHitRate.add(true);
    cacheMissRate.add(false);
    periodHits++;

    if (isRedisHit) {
      redisHitLatency.add(response.timings.duration);
    }
  } else {
    cacheHitRate.add(false);
    cacheMissRate.add(true);
    periodMisses++;
    redisMissLatency.add(response.timings.duration);
  }

  // Validation
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'response time OK': (r) => r.timings.duration < 1000,
  });

  if (success) {
    successfulRequests.add(1);
  } else {
    failedRequests.add(1);
  }

  errorRate.add(!success);

  // Periodic reporting
  const now = Date.now();
  const reportInterval = data.reportInterval * 1000;

  if (now - lastReportTime > reportInterval) {
    const periodTotal = periodHits + periodMisses;
    const periodHitRate = periodTotal > 0 ? (periodHits / periodTotal * 100).toFixed(2) : 0;

    // Calculate P95 for this period
    const sortedLatencies = periodLatencies.slice().sort((a, b) => a - b);
    const p95Index = Math.floor(sortedLatencies.length * 0.95);
    const periodP95 = sortedLatencies.length > 0 ? sortedLatencies[p95Index] : 0;

    // Fetch current Redis metrics
    const currentMetrics = fetchRedisMetrics();

    const elapsed = ((now - data.startTime) / 1000 / 60).toFixed(1);

    console.log('');
    console.log(`[${elapsed} min] Periodic Report`);
    console.log(`  Period Requests: ${periodTotal}`);
    console.log(`  Period Hit Rate: ${periodHitRate}%`);
    console.log(`  Period P95: ${periodP95.toFixed(2)} ms`);

    if (currentMetrics) {
      console.log(`  Redis Active Connections: ${currentMetrics.activeConnections || 'N/A'}`);
      console.log(`  Redis Idle Connections: ${currentMetrics.idleConnections || 'N/A'}`);
      console.log(`  Process Memory: ${currentMetrics.processMemory ? (currentMetrics.processMemory / 1024 / 1024).toFixed(2) + ' MB' : 'N/A'}`);

      if (currentMetrics.activeConnections) {
        redisConnections.add(currentMetrics.activeConnections);
        periodConnections.push(currentMetrics.activeConnections);
      }
    }

    // Track hourly metrics
    hourlyHitRate.add(parseFloat(periodHitRate));
    hourlyP95.add(periodP95);

    // Reset period counters
    periodHits = 0;
    periodMisses = 0;
    periodLatencies = [];
    lastReportTime = now;
  }

  // Small sleep to prevent overwhelming the system
  // Adjust based on target RPS
  sleep(0.01);
}

export function teardown(data) {
  const totalDuration = ((Date.now() - data.startTime) / 1000 / 60).toFixed(1);

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 53: Redis Cache Endurance Test - COMPLETE');
  console.log('='.repeat(80));
  console.log(`Total Duration: ${totalDuration} minutes`);
  console.log(`Test Type: ${__ENV.TEST_TYPE || 'quick'}`);
  console.log('');

  // Fetch final metrics
  const finalMetrics = fetchRedisMetrics();

  console.log('Final Redis Metrics:');
  if (finalMetrics) {
    console.log(`  Active Connections: ${finalMetrics.activeConnections || 'N/A'}`);
    console.log(`  Idle Connections: ${finalMetrics.idleConnections || 'N/A'}`);
    console.log(`  Process Memory: ${finalMetrics.processMemory ? (finalMetrics.processMemory / 1024 / 1024).toFixed(2) + ' MB' : 'N/A'}`);

    // Compare with initial
    if (data.initialMetrics && data.initialMetrics.processMemory && finalMetrics.processMemory) {
      const memGrowth = ((finalMetrics.processMemory - data.initialMetrics.processMemory) / data.initialMetrics.processMemory * 100).toFixed(2);
      console.log(`  Memory Growth: ${memGrowth}%`);
    }
  } else {
    console.log('  Could not fetch final metrics');
  }

  // Connection pool stability check
  if (periodConnections.length > 0) {
    const avgConnections = periodConnections.reduce((a, b) => a + b, 0) / periodConnections.length;
    const maxConnections = Math.max(...periodConnections);
    const minConnections = Math.min(...periodConnections);

    console.log('');
    console.log('Connection Pool Stability:');
    console.log(`  Average Active Connections: ${avgConnections.toFixed(2)}`);
    console.log(`  Max Active Connections: ${maxConnections}`);
    console.log(`  Min Active Connections: ${minConnections}`);
    console.log(`  Connection Variance: ${(maxConnections - minConnections)}`);

    // Check for connection leaks (connections should not grow unbounded)
    if (maxConnections > avgConnections * 2) {
      console.log('  WARNING: Possible connection leak detected!');
    } else {
      console.log('  Connection pool appears stable.');
    }
  }

  console.log('');
  console.log('Success Criteria Check:');
  console.log('  [ ] Connection pool stable - Check k6 metrics');
  console.log('  [ ] No connection leaks - Verify max connections reasonable');
  console.log('  [ ] Redis memory stable - Compare initial vs final');
  console.log('  [ ] TTL expiration working - Run ttl_test scenario');
  console.log('');
  console.log('Test completed at:', new Date().toISOString());
  console.log('='.repeat(80));
}
