/**
 * K6 Backend Comparison Benchmark
 *
 * Compares throughput and latency for different file sizes:
 * - 10KB  (small, cache-friendly)
 * - 1MB   (typical asset)
 * - 10MB  (large image/document)
 * - 100MB (video segment)
 * - 1GB   (full video - reduced concurrency)
 *
 * Usage:
 *   # Run all file sizes (default)
 *   k6 run k6/backend-comparison.js
 *
 *   # Run specific file size
 *   k6 run -e FILE_SIZE=10kb k6/backend-comparison.js
 *   k6 run -e FILE_SIZE=1mb k6/backend-comparison.js
 *   k6 run -e FILE_SIZE=10mb k6/backend-comparison.js
 *   k6 run -e FILE_SIZE=100mb k6/backend-comparison.js
 *   k6 run -e FILE_SIZE=1gb k6/backend-comparison.js
 *
 *   # Override duration and VUs
 *   k6 run -e FILE_SIZE=1mb -e DURATION=120s -e VUS=20 k6/backend-comparison.js
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - Backend (MinIO or RustFS) with test files uploaded
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';

// Configuration from environment variables
const FILE_SIZE = __ENV.FILE_SIZE || 'all';
const DURATION = __ENV.DURATION || '30s';
const BASE_VUS = parseInt(__ENV.VUS) || 10;
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// File configurations
const FILES = {
  '10kb': {
    path: '/benchmark/test-10kb.bin',
    size: 10 * 1024,
    vus: BASE_VUS,
    duration: DURATION,
  },
  '1mb': {
    path: '/benchmark/test-1mb.bin',
    size: 1 * 1024 * 1024,
    vus: BASE_VUS,
    duration: DURATION,
  },
  '10mb': {
    path: '/benchmark/test-10mb.bin',
    size: 10 * 1024 * 1024,
    vus: Math.max(5, Math.floor(BASE_VUS / 2)),
    duration: DURATION,
  },
  '100mb': {
    path: '/benchmark/test-100mb.bin',
    size: 100 * 1024 * 1024,
    vus: Math.max(3, Math.floor(BASE_VUS / 4)),
    duration: DURATION,
  },
  '1gb': {
    path: '/benchmark/test-1gb.bin',
    size: 1024 * 1024 * 1024,
    vus: 2, // Low concurrency for 1GB files
    duration: '60s', // Longer duration for fewer requests
  },
};

// Custom metrics per file size
const metrics = {};
for (const size of Object.keys(FILES)) {
  metrics[size] = {
    requests: new Counter(`requests_${size}`),
    errors: new Rate(`errors_${size}`),
    duration: new Trend(`duration_${size}`),
    throughput: new Trend(`throughput_mbps_${size}`),
    ttfb: new Trend(`ttfb_${size}`),
  };
}

// Determine which scenarios to run
function getScenarios() {
  const scenarios = {};

  if (FILE_SIZE === 'all') {
    // Run all file sizes sequentially
    let startTime = 0;
    for (const [size, config] of Object.entries(FILES)) {
      scenarios[`bench_${size}`] = {
        executor: 'constant-vus',
        vus: config.vus,
        duration: config.duration,
        startTime: `${startTime}s`,
        env: { CURRENT_SIZE: size },
        tags: { file_size: size },
      };
      // Add buffer between scenarios
      startTime += parseInt(config.duration) + 10;
    }
  } else if (FILES[FILE_SIZE]) {
    const config = FILES[FILE_SIZE];
    scenarios[`bench_${FILE_SIZE}`] = {
      executor: 'constant-vus',
      vus: config.vus,
      duration: config.duration,
      env: { CURRENT_SIZE: FILE_SIZE },
      tags: { file_size: FILE_SIZE },
    };
  } else {
    throw new Error(`Invalid FILE_SIZE: ${FILE_SIZE}. Valid options: ${Object.keys(FILES).join(', ')}, all`);
  }

  return scenarios;
}

// Export options
export const options = {
  scenarios: getScenarios(),
  thresholds: {
    // Global thresholds
    http_req_failed: ['rate<0.01'], // <1% errors

    // Per-size thresholds (P95 latency expectations)
    'duration_10kb': ['p(95)<100'],    // 10KB: <100ms
    'duration_1mb': ['p(95)<500'],     // 1MB: <500ms
    'duration_10mb': ['p(95)<2000'],   // 10MB: <2s
    'duration_100mb': ['p(95)<10000'], // 100MB: <10s
    'duration_1gb': ['p(95)<60000'],   // 1GB: <60s
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(90)', 'p(95)', 'p(99)'],
};

// Setup function
export function setup() {
  const testSizes = FILE_SIZE === 'all' ? Object.keys(FILES) : [FILE_SIZE];

  console.log('='.repeat(80));
  console.log('Backend Comparison Benchmark - Yatagarasu S3 Proxy');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`File sizes: ${testSizes.join(', ')}`);
  console.log(`Base VUs: ${BASE_VUS}`);
  console.log('='.repeat(80));

  // Verify files exist
  for (const size of testSizes) {
    const url = `${BASE_URL}${FILES[size].path}`;
    const res = http.head(url);
    if (res.status !== 200) {
      console.error(`WARNING: File not found: ${url} (status: ${res.status})`);
    } else {
      console.log(`Verified: ${size} file exists (${FILES[size].size} bytes)`);
    }
  }

  console.log('='.repeat(80));
  return { startTime: Date.now() };
}

// Main test function
export default function (data) {
  const currentSize = __ENV.CURRENT_SIZE;
  if (!currentSize || !FILES[currentSize]) {
    return;
  }

  const config = FILES[currentSize];
  const url = `${BASE_URL}${config.path}`;
  const m = metrics[currentSize];

  const startTime = Date.now();
  const response = http.get(url, {
    responseType: 'binary', // Important for large files
    timeout: '120s',
  });
  const endTime = Date.now();

  // Calculate metrics
  const durationMs = endTime - startTime;
  const ttfbMs = response.timings.waiting;
  const bytesReceived = response.body ? response.body.byteLength : 0;
  const throughputMbps = (bytesReceived * 8) / (durationMs / 1000) / 1000000;

  // Record metrics
  m.requests.add(1);
  m.duration.add(durationMs);
  m.ttfb.add(ttfbMs);
  m.throughput.add(throughputMbps);

  // Validation
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'body size matches': (r) => r.body && r.body.byteLength === config.size,
  });

  m.errors.add(!success);

  if (!success) {
    console.error(`Error for ${currentSize}: status=${response.status}, body=${response.body ? response.body.byteLength : 0}`);
  }

  // Small think time between requests (not for 1GB files)
  if (currentSize !== '1gb') {
    sleep(0.1);
  }
}

// Teardown function
export function teardown(data) {
  const elapsed = (Date.now() - data.startTime) / 1000;

  console.log('');
  console.log('='.repeat(80));
  console.log('Benchmark Complete!');
  console.log('='.repeat(80));
  console.log(`Total elapsed: ${elapsed.toFixed(1)}s`);
  console.log('');
  console.log('Key Metrics to Compare:');
  console.log('  - requests_*: Total requests completed');
  console.log('  - duration_*: Request duration (ms)');
  console.log('  - throughput_mbps_*: Network throughput (Mbps)');
  console.log('  - ttfb_*: Time to first byte (ms)');
  console.log('  - errors_*: Error rate');
  console.log('');
  console.log('Lower duration/ttfb = Better');
  console.log('Higher throughput = Better');
  console.log('='.repeat(80));
}
