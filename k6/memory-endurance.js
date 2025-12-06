/**
 * K6 Memory Cache Endurance Test - Phase 51
 *
 * This script tests memory cache stability over extended periods (up to 24 hours).
 * It validates that the cache handles sustained load without memory leaks or
 * performance degradation.
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with memory cache enabled
 *   - MinIO running with test files in 'public' bucket
 *   - Prometheus/metrics endpoint available at http://localhost:9090/metrics
 *
 * Usage:
 *   # Quick validation (5 minutes)
 *   k6 run -e SCENARIO=quick k6/memory-endurance.js
 *
 *   # 1-hour endurance test
 *   k6 run -e SCENARIO=one_hour k6/memory-endurance.js
 *
 *   # 24-hour endurance test (full production validation)
 *   k6 run -e SCENARIO=full_24h k6/memory-endurance.js
 *
 *   # Memory pressure recovery test
 *   k6 run -e SCENARIO=pressure_recovery k6/memory-endurance.js
 *
 * Success Criteria (Phase 51):
 *   - Memory growth <10% over 24 hours
 *   - No performance degradation
 *   - Cache hit rate stable ±5%
 *   - P95 latency consistent throughout
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const cacheHitRate = new Rate('cache_hits');
const cacheMissRate = new Rate('cache_misses');
const requestDuration = new Trend('request_duration_ms');
const p95Latency = new Trend('p95_latency_ms');
const requestCount = new Counter('total_requests');
const successfulRequests = new Counter('successful_requests');
const failedRequests = new Counter('failed_requests');

// Periodic metrics for stability tracking
const hourlyHitRate = new Trend('hourly_cache_hit_rate');
const hourlyP95 = new Trend('hourly_p95_latency');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const METRICS_URL = __ENV.METRICS_URL || 'http://localhost:9090/metrics';

// Test files - mix of sizes for realistic workload
const TEST_FILES = [
  { path: '/public/test-1kb.txt', size: '1KB', weight: 40 },    // 40% - small files (high cache hit)
  { path: '/public/test-10kb.txt', size: '10KB', weight: 30 },  // 30% - medium files
  { path: '/public/test-100kb.txt', size: '100KB', weight: 20 }, // 20% - larger files
  { path: '/public/test-1mb.bin', size: '1MB', weight: 10 },    // 10% - large files
];

// Generate file pool for weighted selection
const filePool = [];
TEST_FILES.forEach(file => {
  for (let i = 0; i < file.weight; i++) {
    filePool.push(file);
  }
});

// Scenarios for different test durations
const scenarios = {
  // Quick validation test (5 minutes)
  quick: {
    executor: 'constant-arrival-rate',
    rate: 100,            // 100 RPS
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'quick', REPORT_INTERVAL: '60' },
  },

  // 1-hour endurance test
  one_hour: {
    executor: 'constant-arrival-rate',
    rate: 500,            // 500 RPS target
    timeUnit: '1s',
    duration: '1h',
    preAllocatedVUs: 100,
    maxVUs: 500,
    env: { TEST_TYPE: 'one_hour', REPORT_INTERVAL: '300' },  // Report every 5 minutes
  },

  // Full 24-hour endurance test
  full_24h: {
    executor: 'constant-arrival-rate',
    rate: 500,            // 500 RPS sustained
    timeUnit: '1s',
    duration: '24h',
    preAllocatedVUs: 100,
    maxVUs: 500,
    env: { TEST_TYPE: 'full_24h', REPORT_INTERVAL: '3600' },  // Report every hour
  },

  // Memory pressure recovery test (Phase 51.2)
  pressure_recovery: {
    executor: 'ramping-arrival-rate',
    startRate: 100,
    timeUnit: '1s',
    preAllocatedVUs: 50,
    maxVUs: 1000,
    stages: [
      // Phase 1: Normal load
      { duration: '5m', target: 500 },
      // Phase 2: High pressure (fill cache)
      { duration: '10m', target: 2000 },
      // Phase 3: Recovery period
      { duration: '5m', target: 100 },
      // Phase 4: Normal load again
      { duration: '5m', target: 500 },
      // Phase 5: Another pressure spike
      { duration: '10m', target: 2000 },
      // Phase 6: Final recovery
      { duration: '5m', target: 100 },
    ],
    env: { TEST_TYPE: 'pressure_recovery', REPORT_INTERVAL: '60' },
  },

  // Soak test with gradual ramp
  soak: {
    executor: 'ramping-arrival-rate',
    startRate: 50,
    timeUnit: '1s',
    preAllocatedVUs: 50,
    maxVUs: 500,
    stages: [
      { duration: '10m', target: 500 },   // Ramp up over 10 minutes
      { duration: '5h50m', target: 500 }, // Hold at 500 RPS for ~6 hours
    ],
    env: { TEST_TYPE: 'soak', REPORT_INTERVAL: '1800' },  // Report every 30 minutes
  },

  // 1 million requests test (Phase 51.1 - memory stays constant)
  // 2000 RPS * 500s = 1,000,000 requests in ~8-9 minutes
  million_requests: {
    executor: 'constant-arrival-rate',
    rate: 2000,           // 2000 RPS for fast execution
    timeUnit: '1s',
    duration: '8m20s',    // 8:20 = 500 seconds * 2000 RPS = 1,000,000 requests
    preAllocatedVUs: 200,
    maxVUs: 1000,
    env: { TEST_TYPE: 'million_requests', REPORT_INTERVAL: '60' },
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
    http_req_duration: ['p(95)<100', 'p(99)<200'],  // P95 <100ms, P99 <200ms
    http_req_failed: ['rate<0.01'],                  // Error rate <1%
    errors: ['rate<0.01'],                           // Custom error rate <1%
    cache_hits: ['rate>0.65'],                       // Cache hit rate >65% (target 70%)
  },
  // Disable DNS caching
  dns: {
    ttl: '1m',
    select: 'first',
  },
};

// Track metrics over time
let metricsHistory = [];
let lastReportTime = 0;
let periodHits = 0;
let periodMisses = 0;
let periodLatencies = [];

export function setup() {
  const testType = __ENV.TEST_TYPE || 'quick';
  const reportInterval = parseInt(__ENV.REPORT_INTERVAL || '60');

  console.log('='.repeat(80));
  console.log('Phase 51: Memory Cache Endurance Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Metrics URL: ${METRICS_URL}`);
  console.log(`Test Type: ${testType}`);
  console.log(`Report Interval: ${reportInterval} seconds`);
  console.log(`Scenario: ${selectedScenario || 'quick'}`);
  console.log('');
  console.log('Test started at:', new Date().toISOString());
  console.log('='.repeat(80));

  // Fetch initial metrics for baseline
  let initialMemory = 0;
  try {
    const metricsResponse = http.get(METRICS_URL, { timeout: '5s' });
    if (metricsResponse.status === 200) {
      // Try to parse process_resident_memory_bytes if available
      const memMatch = metricsResponse.body.match(/process_resident_memory_bytes\s+(\d+)/);
      if (memMatch) {
        initialMemory = parseInt(memMatch[1]);
        console.log(`Initial RSS Memory: ${(initialMemory / 1024 / 1024).toFixed(2)} MB`);
      }
    }
  } catch (e) {
    console.log('Note: Could not fetch initial metrics (this is OK)');
  }

  // Pre-warm cache
  console.log('');
  console.log('Pre-warming cache with test files...');
  TEST_FILES.forEach(file => {
    for (let i = 0; i < 5; i++) {
      http.get(`${BASE_URL}${file.path}`, { timeout: '10s' });
    }
  });
  sleep(2);  // Wait for async cache operations
  console.log('Cache pre-warmed.');
  console.log('');
  console.log('Starting endurance test...');
  console.log('');

  return {
    startTime: Date.now(),
    initialMemory: initialMemory,
    reportInterval: reportInterval,
  };
}

export default function (data) {
  // Select file based on weighted distribution
  const file = filePool[Math.floor(Math.random() * filePool.length)];

  // Occasionally request uncached paths to maintain ~70% hit rate
  // 30% of requests use unique path variations to simulate cache misses
  const useCacheBuster = Math.random() < 0.3;
  let url = `${BASE_URL}${file.path}`;
  if (useCacheBuster) {
    // Add unique query param to force cache miss
    url += `?cb=${Date.now()}-${Math.random().toString(36).substring(7)}`;
  }

  // Make request
  const response = http.get(url, {
    timeout: '30s',
  });

  // Record metrics
  requestCount.add(1);
  requestDuration.add(response.timings.duration);
  periodLatencies.push(response.timings.duration);

  // Check cache status
  const cacheStatus = response.headers['X-Cache-Status'] ||
                      response.headers['x-cache-status'] ||
                      response.headers['X-Cache'] ||
                      response.headers['x-cache'] ||
                      '';
  const isCacheHit = cacheStatus.toLowerCase().includes('hit');

  if (isCacheHit) {
    cacheHitRate.add(true);
    cacheMissRate.add(false);
    periodHits++;
  } else {
    cacheHitRate.add(false);
    cacheMissRate.add(true);
    periodMisses++;
  }

  // Validation
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'response time OK': (r) => r.timings.duration < 500,
  });

  if (success) {
    successfulRequests.add(1);
  } else {
    failedRequests.add(1);
  }

  errorRate.add(!success);

  // Periodic reporting
  const now = Date.now();
  const reportInterval = data.reportInterval * 1000;  // Convert to ms
  if (now - lastReportTime > reportInterval) {
    // Calculate period metrics
    const periodTotal = periodHits + periodMisses;
    if (periodTotal > 0) {
      const hitRate = (periodHits / periodTotal) * 100;
      hourlyHitRate.add(hitRate);

      // Calculate P95 for period
      if (periodLatencies.length > 0) {
        periodLatencies.sort((a, b) => a - b);
        const p95Index = Math.floor(periodLatencies.length * 0.95);
        const p95 = periodLatencies[p95Index] || periodLatencies[periodLatencies.length - 1];
        hourlyP95.add(p95);

        console.log(`[${new Date().toISOString()}] Period Stats: ` +
          `Hit Rate: ${hitRate.toFixed(1)}%, ` +
          `P95: ${p95.toFixed(1)}ms, ` +
          `Requests: ${periodTotal}`);
      }
    }

    // Reset period counters
    periodHits = 0;
    periodMisses = 0;
    periodLatencies = [];
    lastReportTime = now;
  }

  // Small sleep to pace requests (constant-arrival-rate handles this, but just in case)
  // This helps achieve more realistic distribution
  sleep(0.001);  // 1ms
}

export function teardown(data) {
  const endTime = Date.now();
  const durationMs = endTime - data.startTime;
  const durationHours = durationMs / (1000 * 60 * 60);
  const durationMin = Math.floor(durationMs / 60000);

  console.log('');
  console.log('='.repeat(80));
  console.log('Memory Cache Endurance Test Complete!');
  console.log('='.repeat(80));
  console.log('');
  console.log(`Test ended at: ${new Date().toISOString()}`);
  console.log(`Total duration: ${durationHours.toFixed(2)} hours (${durationMin} minutes)`);
  console.log('');

  // Fetch final metrics
  let finalMemory = 0;
  try {
    const metricsResponse = http.get(METRICS_URL, { timeout: '5s' });
    if (metricsResponse.status === 200) {
      const memMatch = metricsResponse.body.match(/process_resident_memory_bytes\s+(\d+)/);
      if (memMatch) {
        finalMemory = parseInt(memMatch[1]);
        console.log(`Final RSS Memory: ${(finalMemory / 1024 / 1024).toFixed(2)} MB`);

        if (data.initialMemory > 0) {
          const memoryGrowth = finalMemory - data.initialMemory;
          const growthPercent = (memoryGrowth / data.initialMemory) * 100;
          console.log(`Memory Growth: ${(memoryGrowth / 1024 / 1024).toFixed(2)} MB (${growthPercent.toFixed(1)}%)`);

          if (growthPercent < 10) {
            console.log('  PASS: Memory growth <10%');
          } else {
            console.log('  WARNING: Memory growth >=10% - potential memory leak');
          }
        }
      }
    }
  } catch (e) {
    console.log('Note: Could not fetch final metrics');
  }

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 51 Success Criteria:');
  console.log('='.repeat(80));
  console.log('');
  console.log('  51.1 24-Hour Memory Cache Test:');
  console.log('    - Memory growth <10% over test duration');
  console.log('    - Cache hit rate stable at ~70% (±5%)');
  console.log('    - P95 latency consistent throughout');
  console.log('    - No performance degradation');
  console.log('');
  console.log('  51.2 Memory Pressure Recovery:');
  console.log('    - Cache fills and evicts correctly');
  console.log('    - Memory reclaimed after eviction');
  console.log('    - No fragmentation buildup');
  console.log('');
  console.log('Key Metrics to Check:');
  console.log('  - http_req_duration{p95}: Should be <100ms');
  console.log('  - cache_hits: Should be >65%');
  console.log('  - errors: Should be <1%');
  console.log('  - hourly_cache_hit_rate: Should be stable ±5%');
  console.log('  - hourly_p95_latency: Should not increase over time');
  console.log('');
  console.log('='.repeat(80));
}
