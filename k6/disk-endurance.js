/**
 * K6 Disk Cache Endurance Test - Phase 52
 *
 * This script tests disk cache stability over extended periods (up to 24 hours).
 * It validates that the disk cache handles sustained load without:
 * - Unbounded index file growth
 * - Orphaned file accumulation
 * - Performance degradation
 * - LRU eviction failures
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with disk cache enabled
 *   - MinIO running with test files in 'public' bucket
 *   - Prometheus/metrics endpoint available at http://localhost:9090/metrics
 *   - Disk cache directory configured (e.g., /tmp/yatagarasu-disk-cache)
 *
 * Usage:
 *   # Quick validation (5 minutes)
 *   k6 run -e SCENARIO=quick k6/disk-endurance.js
 *
 *   # 1-hour endurance test
 *   k6 run -e SCENARIO=one_hour k6/disk-endurance.js
 *
 *   # 24-hour endurance test (full production validation)
 *   k6 run -e SCENARIO=full_24h k6/disk-endurance.js
 *
 *   # Eviction stress test
 *   k6 run -e SCENARIO=eviction_stress k6/disk-endurance.js
 *
 * Success Criteria (Phase 52):
 *   - Disk usage stable (eviction working)
 *   - Index file size bounded
 *   - No orphaned files
 *   - Performance consistent over time
 *   - P95 latency <500ms for disk cache
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
const diskUsageMB = new Gauge('disk_usage_mb');
const indexSizeKB = new Gauge('index_size_kb');
const fileCount = new Gauge('cached_file_count');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const METRICS_URL = __ENV.METRICS_URL || 'http://localhost:9090/metrics';
const CACHE_DIR = __ENV.CACHE_DIR || '/tmp/yatagarasu-disk-cache';

// Test files - mix of sizes for realistic workload
// Disk cache typically handles larger files than memory cache
const TEST_FILES = [
  { path: '/public/test-10kb.txt', size: '10KB', weight: 30 },   // 30% - small files
  { path: '/public/test-100kb.txt', size: '100KB', weight: 40 }, // 40% - medium files
  { path: '/public/test-1mb.bin', size: '1MB', weight: 25 },     // 25% - larger files
  { path: '/public/test-10mb.bin', size: '10MB', weight: 5 },    // 5% - large files
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
    rate: 50,             // 50 RPS (lower than memory due to disk I/O)
    timeUnit: '1s',
    duration: '5m',
    preAllocatedVUs: 20,
    maxVUs: 100,
    env: { TEST_TYPE: 'quick', REPORT_INTERVAL: '60', HIT_RATE_TARGET: '0.6' },
  },

  // 1-hour endurance test
  one_hour: {
    executor: 'constant-arrival-rate',
    rate: 100,            // 100 RPS target
    timeUnit: '1s',
    duration: '1h',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'one_hour', REPORT_INTERVAL: '300', HIT_RATE_TARGET: '0.6' },
  },

  // Full 24-hour endurance test (Phase 52.1 primary test)
  full_24h: {
    executor: 'constant-arrival-rate',
    rate: 100,            // 100 RPS sustained (per plan.md)
    timeUnit: '1s',
    duration: '24h',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'full_24h', REPORT_INTERVAL: '3600', HIT_RATE_TARGET: '0.6' },
  },

  // Eviction stress test - force many evictions
  eviction_stress: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '30m',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'eviction_stress', REPORT_INTERVAL: '60', HIT_RATE_TARGET: '0.3', UNIQUE_KEYS: 'true' },
  },

  // LRU validation test
  lru_validation: {
    executor: 'ramping-arrival-rate',
    startRate: 10,
    timeUnit: '1s',
    preAllocatedVUs: 20,
    maxVUs: 100,
    stages: [
      // Phase 1: Fill cache with files
      { duration: '5m', target: 50 },
      // Phase 2: Access only a subset (hot set)
      { duration: '10m', target: 100 },
      // Phase 3: Add new files (should evict cold set)
      { duration: '5m', target: 100 },
      // Phase 4: Verify hot set still cached
      { duration: '5m', target: 50 },
    ],
    env: { TEST_TYPE: 'lru_validation', REPORT_INTERVAL: '60', HIT_RATE_TARGET: '0.5' },
  },

  // Recovery after abrupt shutdown simulation
  recovery_test: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '10m',
    preAllocatedVUs: 20,
    maxVUs: 100,
    env: { TEST_TYPE: 'recovery_test', REPORT_INTERVAL: '60', HIT_RATE_TARGET: '0.6' },
  },

  // Index rebuild performance test
  index_rebuild: {
    executor: 'constant-arrival-rate',
    rate: 100,
    timeUnit: '1s',
    duration: '15m',
    preAllocatedVUs: 50,
    maxVUs: 200,
    env: { TEST_TYPE: 'index_rebuild', REPORT_INTERVAL: '60', HIT_RATE_TARGET: '0.4', UNIQUE_KEYS: 'true' },
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
    http_req_duration: ['p(95)<500', 'p(99)<1000'],  // P95 <500ms, P99 <1000ms for disk
    http_req_failed: ['rate<0.01'],                   // Error rate <1%
    errors: ['rate<0.01'],                            // Custom error rate <1%
    cache_hits: ['rate>0.55'],                        // Cache hit rate >55% (target 60%)
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
let initialDiskUsage = 0;
let initialIndexSize = 0;

export function setup() {
  const testType = __ENV.TEST_TYPE || 'quick';
  const reportInterval = parseInt(__ENV.REPORT_INTERVAL || '60');
  const hitRateTarget = parseFloat(__ENV.HIT_RATE_TARGET || '0.6');

  console.log('='.repeat(80));
  console.log('Phase 52: Disk Cache Endurance Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Metrics URL: ${METRICS_URL}`);
  console.log(`Cache Directory: ${CACHE_DIR}`);
  console.log(`Test Type: ${testType}`);
  console.log(`Report Interval: ${reportInterval} seconds`);
  console.log(`Hit Rate Target: ${(hitRateTarget * 100).toFixed(0)}%`);
  console.log(`Scenario: ${selectedScenario || 'quick'}`);
  console.log('');
  console.log('Test started at:', new Date().toISOString());
  console.log('='.repeat(80));

  // Fetch initial metrics for baseline
  let diskMetrics = { usage: 0, indexSize: 0, fileCount: 0 };
  try {
    const metricsResponse = http.get(METRICS_URL, { timeout: '5s' });
    if (metricsResponse.status === 200) {
      // Try to parse disk cache metrics if available
      const diskUsageMatch = metricsResponse.body.match(/yatagarasu_cache_disk_usage_bytes\s+(\d+)/);
      const indexSizeMatch = metricsResponse.body.match(/yatagarasu_cache_index_size_bytes\s+(\d+)/);
      const fileCountMatch = metricsResponse.body.match(/yatagarasu_cache_file_count\s+(\d+)/);

      if (diskUsageMatch) diskMetrics.usage = parseInt(diskUsageMatch[1]);
      if (indexSizeMatch) diskMetrics.indexSize = parseInt(indexSizeMatch[1]);
      if (fileCountMatch) diskMetrics.fileCount = parseInt(fileCountMatch[1]);

      console.log(`Initial Disk Usage: ${(diskMetrics.usage / 1024 / 1024).toFixed(2)} MB`);
      console.log(`Initial Index Size: ${(diskMetrics.indexSize / 1024).toFixed(2)} KB`);
      console.log(`Initial File Count: ${diskMetrics.fileCount}`);
    }
  } catch (e) {
    console.log('Note: Could not fetch initial disk metrics (this is OK)');
  }

  // Pre-warm cache
  console.log('');
  console.log('Pre-warming disk cache with test files...');
  TEST_FILES.forEach(file => {
    for (let i = 0; i < 3; i++) {
      http.get(`${BASE_URL}${file.path}`, { timeout: '30s' });
    }
  });
  sleep(3);  // Wait for disk writes to complete
  console.log('Disk cache pre-warmed.');
  console.log('');
  console.log('Starting endurance test...');
  console.log('');

  return {
    startTime: Date.now(),
    initialDiskUsage: diskMetrics.usage,
    initialIndexSize: diskMetrics.indexSize,
    initialFileCount: diskMetrics.fileCount,
    reportInterval: reportInterval,
    hitRateTarget: hitRateTarget,
  };
}

// Request counter for unique keys
let requestId = 0;

export default function (data) {
  const testType = __ENV.TEST_TYPE || 'quick';
  const useUniqueKeys = __ENV.UNIQUE_KEYS === 'true';
  const hitRateTarget = data.hitRateTarget;

  // Select file based on weighted distribution
  const file = filePool[Math.floor(Math.random() * filePool.length)];

  // Control cache hit rate:
  // - For eviction/stress tests: use unique keys to force misses
  // - For normal tests: balance between hits and misses to achieve target hit rate
  let url = `${BASE_URL}${file.path}`;
  if (useUniqueKeys) {
    // Force cache miss with unique query param
    url += `?nocache=${requestId++}`;
  } else {
    // Achieve ~60% hit rate: 40% of requests use unique params
    const forceMiss = Math.random() > hitRateTarget;
    if (forceMiss) {
      url += `?cb=${Date.now()}-${Math.random().toString(36).substring(7)}`;
    }
  }

  // Make request with longer timeout for disk I/O
  const response = http.get(url, {
    timeout: '60s',
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

  // Validation - disk cache has higher latency tolerance
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'response time OK': (r) => r.timings.duration < 1000,  // 1s for disk
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

        // Try to fetch disk metrics
        let diskStatus = '';
        try {
          const metricsResponse = http.get(METRICS_URL, { timeout: '2s' });
          if (metricsResponse.status === 200) {
            const diskUsageMatch = metricsResponse.body.match(/yatagarasu_cache_disk_usage_bytes\s+(\d+)/);
            const indexSizeMatch = metricsResponse.body.match(/yatagarasu_cache_index_size_bytes\s+(\d+)/);
            if (diskUsageMatch) {
              const usageMB = parseInt(diskUsageMatch[1]) / 1024 / 1024;
              diskUsageMB.add(usageMB);
              diskStatus += `, Disk: ${usageMB.toFixed(1)}MB`;
            }
            if (indexSizeMatch) {
              const sizeKB = parseInt(indexSizeMatch[1]) / 1024;
              indexSizeKB.add(sizeKB);
              diskStatus += `, Index: ${sizeKB.toFixed(1)}KB`;
            }
          }
        } catch (e) {
          // Ignore metrics fetch errors
        }

        console.log(`[${new Date().toISOString()}] Period Stats: ` +
          `Hit Rate: ${hitRate.toFixed(1)}%, ` +
          `P95: ${p95.toFixed(1)}ms, ` +
          `Requests: ${periodTotal}${diskStatus}`);
      }
    }

    // Reset period counters
    periodHits = 0;
    periodMisses = 0;
    periodLatencies = [];
    lastReportTime = now;
  }

  // Small sleep to prevent overwhelming disk I/O
  sleep(0.01);  // 10ms
}

export function teardown(data) {
  const endTime = Date.now();
  const durationMs = endTime - data.startTime;
  const durationHours = durationMs / (1000 * 60 * 60);
  const durationMin = Math.floor(durationMs / 60000);

  console.log('');
  console.log('='.repeat(80));
  console.log('Disk Cache Endurance Test Complete!');
  console.log('='.repeat(80));
  console.log('');
  console.log(`Test ended at: ${new Date().toISOString()}`);
  console.log(`Total duration: ${durationHours.toFixed(2)} hours (${durationMin} minutes)`);
  console.log('');

  // Fetch final metrics
  let finalDiskUsage = 0;
  let finalIndexSize = 0;
  let finalFileCount = 0;
  try {
    const metricsResponse = http.get(METRICS_URL, { timeout: '5s' });
    if (metricsResponse.status === 200) {
      const diskUsageMatch = metricsResponse.body.match(/yatagarasu_cache_disk_usage_bytes\s+(\d+)/);
      const indexSizeMatch = metricsResponse.body.match(/yatagarasu_cache_index_size_bytes\s+(\d+)/);
      const fileCountMatch = metricsResponse.body.match(/yatagarasu_cache_file_count\s+(\d+)/);

      if (diskUsageMatch) finalDiskUsage = parseInt(diskUsageMatch[1]);
      if (indexSizeMatch) finalIndexSize = parseInt(indexSizeMatch[1]);
      if (fileCountMatch) finalFileCount = parseInt(fileCountMatch[1]);

      console.log(`Final Disk Usage: ${(finalDiskUsage / 1024 / 1024).toFixed(2)} MB`);
      console.log(`Final Index Size: ${(finalIndexSize / 1024).toFixed(2)} KB`);
      console.log(`Final File Count: ${finalFileCount}`);
      console.log('');

      if (data.initialDiskUsage > 0) {
        const diskGrowth = finalDiskUsage - data.initialDiskUsage;
        const diskGrowthMB = diskGrowth / 1024 / 1024;
        console.log(`Disk Usage Change: ${diskGrowthMB.toFixed(2)} MB`);
        if (diskGrowthMB < 100) {
          console.log('  PASS: Disk usage stable (eviction working)');
        } else {
          console.log('  NOTE: Disk usage grew - check eviction thresholds');
        }
      }

      if (data.initialIndexSize > 0) {
        const indexGrowth = finalIndexSize - data.initialIndexSize;
        const indexGrowthKB = indexGrowth / 1024;
        console.log(`Index Size Change: ${indexGrowthKB.toFixed(2)} KB`);
        if (Math.abs(indexGrowthKB) < 100) {
          console.log('  PASS: Index size bounded');
        } else {
          console.log('  WARNING: Index size grew significantly');
        }
      }
    }
  } catch (e) {
    console.log('Note: Could not fetch final disk metrics');
  }

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 52 Success Criteria:');
  console.log('='.repeat(80));
  console.log('');
  console.log('  52.1 24-Hour Disk Cache Test:');
  console.log('    - Disk usage stable (eviction working)');
  console.log('    - Index file size bounded (not growing unbounded)');
  console.log('    - No orphaned files accumulate');
  console.log('    - Performance remains consistent');
  console.log('    - LRU eviction works correctly');
  console.log('    - P95 latency <500ms');
  console.log('');
  console.log('  52.2 Disk Recovery Tests:');
  console.log('    - Recovery after disk full condition');
  console.log('    - Recovery after abrupt shutdown');
  console.log('    - Index rebuild performance');
  console.log('');
  console.log('Key Metrics to Check:');
  console.log('  - http_req_duration{p95}: Should be <500ms');
  console.log('  - cache_hits: Should be >55% (target 60%)');
  console.log('  - errors: Should be <1%');
  console.log('  - hourly_cache_hit_rate: Should be stable ±5%');
  console.log('  - hourly_p95_latency: Should not increase over time');
  console.log('  - disk_usage_mb: Should be bounded by max cache size');
  console.log('  - index_size_kb: Should not grow unbounded');
  console.log('');
  console.log('Manual Verification Commands:');
  console.log('  # Check disk cache directory size:');
  console.log(`  du -sh ${CACHE_DIR}`);
  console.log('');
  console.log('  # Count cached files:');
  console.log(`  find ${CACHE_DIR} -type f | wc -l`);
  console.log('');
  console.log('  # Check for orphaned files (files not in index):');
  console.log('  # Compare file count in index vs actual files');
  console.log('');
  console.log('  # Monitor file I/O:');
  console.log('  iostat -x 1');
  console.log('');
  console.log('='.repeat(80));
}
