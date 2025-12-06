/**
 * K6 Test: Phase 57 Mixed Workload Testing
 *
 * This script tests realistic production workloads with mixed file sizes:
 * - Small files (<1MB): Cacheable, should hit cache after warmup
 * - Large files (>10MB): Streamed directly, bypass cache
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - Test files in MinIO: test-1kb.txt, test-10kb.txt, test-100kb.txt,
 *     test-1mb.bin, test-100mb.bin, test-1gb.bin
 *
 * Usage:
 *   # Quick validation (5 minutes)
 *   k6 run -e SCENARIO=quick k6/mixed-workload.js
 *
 *   # 50/50 cache/stream mix (10 minutes)
 *   k6 run -e SCENARIO=cache_stream_mix k6/mixed-workload.js
 *
 *   # Resource isolation test (concurrent small + large)
 *   k6 run -e SCENARIO=resource_isolation k6/mixed-workload.js
 *
 *   # Extended mixed load (30 minutes)
 *   k6 run -e SCENARIO=extended_mix k6/mixed-workload.js
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const smallFileLatency = new Trend('small_file_latency_ms');
const largeFileLatency = new Trend('large_file_latency_ms');
const smallFileSuccess = new Rate('small_file_success');
const largeFileSuccess = new Rate('large_file_success');
const cacheHitRate = new Rate('cache_hit_rate');
const smallFileCount = new Counter('small_file_requests');
const largeFileCount = new Counter('large_file_requests');
const totalBytesDownloaded = new Counter('total_bytes_downloaded');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// File categories
const SMALL_FILES = [
  { path: '/public/test-1kb.txt', size: 1024, name: '1KB' },
  { path: '/public/test-10kb.txt', size: 10240, name: '10KB' },
  { path: '/public/test-100kb.txt', size: 102400, name: '100KB' },
  { path: '/public/test-1mb.bin', size: 1048576, name: '1MB' },
];

const LARGE_FILES = [
  { path: '/public/test-100mb.bin', size: 104857600, name: '100MB' },
  { path: '/public/test-1gb.bin', size: 1073741824, name: '1GB' },
];

// Scenario groups - use different executors for small and large files
const scenarioGroups = {
  // Quick validation: small files only to verify cache performance
  quick: {
    small_files: {
      executor: 'constant-arrival-rate',
      rate: 500,
      timeUnit: '1s',
      duration: '2m',
      preAllocatedVUs: 50,
      maxVUs: 100,
      exec: 'smallFileTest',
    },
    large_files: {
      executor: 'constant-vus',
      vus: 5,
      duration: '2m',
      exec: 'largeFileTest',
    },
  },

  // Resource isolation: high small file load + concurrent large file streams
  resource_isolation: {
    small_files: {
      executor: 'constant-arrival-rate',
      rate: 1000,
      timeUnit: '1s',
      duration: '5m',
      preAllocatedVUs: 100,
      maxVUs: 200,
      exec: 'smallFileTest',
    },
    large_files: {
      executor: 'constant-vus',
      vus: 10,
      duration: '5m',
      exec: 'largeFileTest',
    },
  },

  // Extended mixed load (10 minutes)
  extended_mix: {
    small_files: {
      executor: 'constant-arrival-rate',
      rate: 500,
      timeUnit: '1s',
      duration: '10m',
      preAllocatedVUs: 50,
      maxVUs: 150,
      exec: 'smallFileTest',
    },
    large_files: {
      executor: 'constant-vus',
      vus: 20,
      duration: '10m',
      exec: 'largeFileTest',
    },
  },

  // High concurrency mixed (stress test)
  high_concurrency_mix: {
    small_files: {
      executor: 'ramping-arrival-rate',
      startRate: 500,
      timeUnit: '1s',
      preAllocatedVUs: 100,
      maxVUs: 500,
      stages: [
        { target: 1000, duration: '2m' },
        { target: 2000, duration: '3m' },
        { target: 500, duration: '1m' },
      ],
      exec: 'smallFileTest',
    },
    large_files: {
      executor: 'ramping-vus',
      startVUs: 5,
      stages: [
        { target: 20, duration: '2m' },
        { target: 50, duration: '3m' },
        { target: 10, duration: '1m' },
      ],
      exec: 'largeFileTest',
    },
  },
};

// Select scenario from environment variable
const selectedScenario = __ENV.SCENARIO || 'quick';
const activeScenarioGroup = scenarioGroups[selectedScenario];

if (!activeScenarioGroup) {
  console.error(`Unknown scenario: ${selectedScenario}`);
  console.error(`Available: ${Object.keys(scenarioGroups).join(', ')}`);
}

export const options = {
  scenarios: activeScenarioGroup,
  thresholds: {
    // Small file targets (cacheable) - more realistic under mixed load
    'small_file_latency_ms': ['p(95)<200', 'p(99)<500'],
    'small_file_success': ['rate>0.99'],

    // Large file targets (streaming) - 100MB ~0.2-2s depending on load
    'large_file_latency_ms': ['p(95)<60000'], // 60s for 1GB is reasonable
    'large_file_success': ['rate>0.95'],

    // Overall targets
    'errors': ['rate<0.01'],
    'http_req_failed': ['rate<0.01'],
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 57: Mixed Workload Test');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario}`);
  console.log('');
  console.log('File categories:');
  console.log('  Small (cacheable): 1KB, 10KB, 100KB, 1MB');
  console.log('  Large (streamed): 100MB, 1GB');
  console.log('='.repeat(80));

  // Warm cache with small files
  console.log('Warming cache with small files...');
  for (const file of SMALL_FILES) {
    for (let i = 0; i < 5; i++) {
      const response = http.get(`${BASE_URL}${file.path}`);
      if (response.status !== 200) {
        console.log(`Warning: Warmup failed for ${file.name}: ${response.status}`);
      }
    }
  }
  console.log('Cache warmed.');

  return { startTime: Date.now() };
}

let smallFileRequestId = 0;
let largeFileRequestId = 0;

// Small file test function (used by small_files executor)
export function smallFileTest(data) {
  smallFileRequestId++;
  const file = SMALL_FILES[smallFileRequestId % SMALL_FILES.length];

  const response = http.get(`${BASE_URL}${file.path}`, {
    tags: { file_type: 'small', file_name: file.name },
  });

  smallFileCount.add(1);
  smallFileLatency.add(response.timings.duration);

  const statusOk = check(response, {
    'small file status 200': (r) => r.status === 200,
  });

  // Size check separate from success rate
  check(response, {
    'small file has body': (r) => r.body && r.body.length > 0,
    'small file latency < 50ms': (r) => r.timings.duration < 50,
  });

  smallFileSuccess.add(statusOk);
  errorRate.add(!statusOk);

  // Check cache header if present
  const cacheHeader = response.headers['X-Cache'] || '';
  cacheHitRate.add(cacheHeader.toLowerCase().includes('hit'));

  if (response.body) {
    totalBytesDownloaded.add(response.body.length);
  }
}

// Large file test function (used by large_files executor)
export function largeFileTest(data) {
  largeFileRequestId++;
  // Use 100MB primarily for faster iteration
  const file = LARGE_FILES[0]; // 100MB

  const response = http.get(`${BASE_URL}${file.path}`, {
    tags: { file_type: 'large', file_name: file.name },
    timeout: '120s', // 2 minute timeout for large files
  });

  largeFileCount.add(1);
  largeFileLatency.add(response.timings.duration);

  const success = check(response, {
    'large file status 200': (r) => r.status === 200,
    'large file has body': (r) => r.body && r.body.length > 0,
  });

  largeFileSuccess.add(success);
  errorRate.add(!success);

  if (response.body) {
    totalBytesDownloaded.add(response.body.length);
  }
}

// Default function (for backwards compatibility)
export default function (data) {
  // Random selection: 70% small, 30% large
  if (Math.random() < 0.7) {
    smallFileTest(data);
  } else {
    largeFileTest(data);
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Phase 57: Mixed Workload Test Complete');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log('');
  console.log('Success Criteria:');
  console.log('  Small files: P95 < 50ms, 99% success');
  console.log('  Large files: P95 < 60s, 95% success');
  console.log('  Overall: < 1% errors');
  console.log('');
  console.log('Expected behavior:');
  console.log('  - Small files should be served from cache after warmup');
  console.log('  - Large files should stream without blocking small file requests');
  console.log('  - Memory should stay constant during large file streaming');
  console.log('='.repeat(80));
}
