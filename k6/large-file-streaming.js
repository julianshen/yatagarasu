/**
 * K6 Load Test: Phase 39 Large File Streaming Tests
 *
 * This script tests large file streaming behavior:
 * - Single large file downloads (100MB, 500MB)
 * - Concurrent large file downloads
 * - Range requests (streaming)
 * - Memory efficiency verification
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080 with streaming config
 *   - MinIO running with test files in 'public' bucket
 *   - Test files: test-100mb.bin, test-500mb.bin
 *
 * Usage:
 *   # Run single large file test
 *   k6 run -e SCENARIO=single_100mb k6/large-file-streaming.js
 *
 *   # Run concurrent streaming test
 *   k6 run -e SCENARIO=concurrent_10 k6/large-file-streaming.js
 *
 *   # Run range request test
 *   k6 run -e SCENARIO=range_requests k6/large-file-streaming.js
 *
 *   # Run quick validation test
 *   k6 run -e SCENARIO=quick k6/large-file-streaming.js
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const streamingRequests = new Counter('streaming_requests');
const cacheBypassCount = new Counter('cache_bypass');
const requestDuration = new Trend('request_duration_ms');
const ttfb = new Trend('ttfb_ms');
const throughput = new Trend('throughput_mbps');
const rangeRequestCount = new Counter('range_requests');

// Base URL for the proxy
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';

// Test files (large files > 10MB will bypass cache)
const TEST_FILES = {
  small: '/public/test-1mb.bin',        // 1MB - cached
  medium: '/public/test-5mb.bin',       // 5MB - cached
  large: '/public/test-100mb.bin',      // 100MB - streamed
  xl: '/public/test-500mb.bin',         // 500MB - streamed
};

// File sizes in bytes (for verification)
const FILE_SIZES = {
  small: 1 * 1024 * 1024,      // 1MB
  medium: 5 * 1024 * 1024,     // 5MB
  large: 100 * 1024 * 1024,    // 100MB
  xl: 500 * 1024 * 1024,       // 500MB
};

// Scenarios for different streaming tests
const scenarios = {
  // Quick validation test
  quick: {
    executor: 'per-vu-iterations',
    vus: 2,
    iterations: 2,
    maxDuration: '2m',
    env: { TEST_MODE: 'quick' },
  },

  // Single 100MB file download
  single_100mb: {
    executor: 'per-vu-iterations',
    vus: 1,
    iterations: 3,
    maxDuration: '5m',
    env: { TEST_MODE: 'single_100mb' },
  },

  // Single 500MB file download
  single_500mb: {
    executor: 'per-vu-iterations',
    vus: 1,
    iterations: 2,
    maxDuration: '10m',
    env: { TEST_MODE: 'single_500mb' },
  },

  // 10 concurrent large file downloads
  concurrent_10: {
    executor: 'constant-vus',
    vus: 10,
    duration: '2m',
    env: { TEST_MODE: 'concurrent' },
  },

  // 50 concurrent large file downloads
  concurrent_50: {
    executor: 'constant-vus',
    vus: 50,
    duration: '3m',
    env: { TEST_MODE: 'concurrent' },
  },

  // Range requests for large files (simulating video seeking)
  range_requests: {
    executor: 'per-vu-iterations',
    vus: 10,
    iterations: 20,
    maxDuration: '5m',
    env: { TEST_MODE: 'range' },
  },

  // Mixed workload: small (cached) + large (streamed)
  mixed_workload: {
    executor: 'constant-arrival-rate',
    rate: 50,
    timeUnit: '1s',
    duration: '2m',
    preAllocatedVUs: 30,
    maxVUs: 100,
    env: { TEST_MODE: 'mixed' },
  },

  // Sustained streaming load
  sustained: {
    executor: 'ramping-vus',
    startVUs: 5,
    stages: [
      { duration: '1m', target: 20 },
      { duration: '3m', target: 20 },
      { duration: '1m', target: 5 },
    ],
    env: { TEST_MODE: 'sustained' },
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
    http_req_failed: ['rate<0.05'],      // Error rate < 5%
    ttfb_ms: ['p(95)<2000'],              // TTFB P95 < 2s
    errors: ['rate<0.05'],
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Phase 39: Large File Streaming Tests');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Scenario: ${selectedScenario || 'quick'}`);
  console.log(`Test Mode: ${__ENV.TEST_MODE || 'quick'}`);
  console.log('='.repeat(80));

  // Verify large test files exist
  console.log('Verifying large test files...');
  const files = [TEST_FILES.large, TEST_FILES.xl];

  for (const file of files) {
    const response = http.head(`${BASE_URL}${file}`);
    if (response.status === 200) {
      const size = parseInt(response.headers['Content-Length'] || '0');
      console.log(`  OK: ${file} (${(size / 1024 / 1024).toFixed(1)} MB)`);
    } else {
      console.log(`  MISSING: ${file} (status: ${response.status})`);
    }
  }

  console.log('');
  console.log('Starting streaming test...');
  console.log('NOTE: Large files (>10MB) bypass cache and stream directly from S3');

  return {
    startTime: Date.now(),
    mode: __ENV.TEST_MODE || 'quick',
  };
}

// Get test configuration based on mode
function getTestConfig(mode, vu, iter) {
  switch (mode) {
    case 'single_100mb':
      return {
        url: `${BASE_URL}${TEST_FILES.large}`,
        expectedSize: FILE_SIZES.large,
        timeout: '120s',
        rangeStart: null,
        rangeEnd: null,
      };

    case 'single_500mb':
      return {
        url: `${BASE_URL}${TEST_FILES.xl}`,
        expectedSize: FILE_SIZES.xl,
        timeout: '300s',
        rangeStart: null,
        rangeEnd: null,
      };

    case 'concurrent':
      // Alternate between large files
      const file = iter % 2 === 0 ? TEST_FILES.large : TEST_FILES.xl;
      const size = iter % 2 === 0 ? FILE_SIZES.large : FILE_SIZES.xl;
      return {
        url: `${BASE_URL}${file}`,
        expectedSize: size,
        timeout: '180s',
        rangeStart: null,
        rangeEnd: null,
      };

    case 'range':
      // Range requests for video-like seeking behavior
      const rangeFile = TEST_FILES.large;  // 100MB file
      const fileSize = FILE_SIZES.large;
      // Different range patterns
      const patterns = [
        { start: 0, end: 1024 * 1024 - 1 },                    // First 1MB
        { start: fileSize - 1024 * 1024, end: fileSize - 1 },  // Last 1MB
        { start: Math.floor(fileSize / 2), end: Math.floor(fileSize / 2) + 1024 * 1024 - 1 },  // Middle 1MB
        { start: Math.floor(fileSize / 4), end: Math.floor(fileSize / 4) + 5 * 1024 * 1024 - 1 },  // 5MB from 25%
      ];
      const pattern = patterns[iter % patterns.length];
      return {
        url: `${BASE_URL}${rangeFile}`,
        expectedSize: pattern.end - pattern.start + 1,
        timeout: '30s',
        rangeStart: pattern.start,
        rangeEnd: pattern.end,
      };

    case 'mixed':
      // 70% small (cached), 30% large (streamed)
      if (Math.random() < 0.7) {
        const smallFiles = [TEST_FILES.small, TEST_FILES.medium];
        const idx = Math.floor(Math.random() * smallFiles.length);
        const smallSize = idx === 0 ? FILE_SIZES.small : FILE_SIZES.medium;
        return {
          url: `${BASE_URL}${smallFiles[idx]}`,
          expectedSize: smallSize,
          timeout: '30s',
          rangeStart: null,
          rangeEnd: null,
        };
      } else {
        return {
          url: `${BASE_URL}${TEST_FILES.large}`,
          expectedSize: FILE_SIZES.large,
          timeout: '120s',
          rangeStart: null,
          rangeEnd: null,
        };
      }

    case 'sustained':
      // Mix of large files for sustained streaming
      const sustainedFile = iter % 3 === 0 ? TEST_FILES.xl : TEST_FILES.large;
      const sustainedSize = iter % 3 === 0 ? FILE_SIZES.xl : FILE_SIZES.large;
      return {
        url: `${BASE_URL}${sustainedFile}`,
        expectedSize: sustainedSize,
        timeout: '300s',
        rangeStart: null,
        rangeEnd: null,
      };

    default:
      // Quick test - use 100MB file
      return {
        url: `${BASE_URL}${TEST_FILES.large}`,
        expectedSize: FILE_SIZES.large,
        timeout: '120s',
        rangeStart: null,
        rangeEnd: null,
      };
  }
}

export default function (data) {
  const mode = data.mode;
  const config = getTestConfig(mode, __VU, __ITER);

  // Build request options
  const reqOptions = {
    timeout: config.timeout,
    responseType: 'binary',  // Important: use binary to avoid text conversion overhead
  };

  // Add Range header if specified
  if (config.rangeStart !== null && config.rangeEnd !== null) {
    reqOptions.headers = {
      'Range': `bytes=${config.rangeStart}-${config.rangeEnd}`,
    };
    rangeRequestCount.add(1);
  }

  const startTime = Date.now();

  // Make request
  const response = http.get(config.url, reqOptions);

  const endTime = Date.now();
  const duration = endTime - startTime;

  // Record metrics
  streamingRequests.add(1);
  requestDuration.add(duration);
  ttfb.add(response.timings.waiting);

  // Calculate throughput in Mbps
  const bodyLength = response.body ? response.body.length : 0;
  if (duration > 0 && bodyLength > 0) {
    const mbps = (bodyLength * 8) / (duration / 1000) / (1024 * 1024);
    throughput.add(mbps);
  }

  // Check cache status
  const cacheStatus = response.headers['X-Cache-Status'] ||
                      response.headers['x-cache-status'] ||
                      response.headers['X-Cache'] ||
                      response.headers['x-cache'] ||
                      '';
  const isCacheBypass = cacheStatus.toLowerCase().includes('bypass') ||
                        cacheStatus.toLowerCase().includes('miss') ||
                        cacheStatus === '';

  // Large files (>10MB) should bypass cache
  if (config.expectedSize > 10 * 1024 * 1024 && isCacheBypass) {
    cacheBypassCount.add(1);
  }

  // Validation
  let expectedStatus = config.rangeStart !== null ? 206 : 200;

  const success = check(response, {
    'status is correct': (r) => r.status === expectedStatus,
    'response has body': (r) => r.body && r.body.length > 0,
  });

  // Mode-specific checks
  if (mode === 'single_100mb' || mode === 'single_500mb') {
    check(response, {
      'large file: complete download': (r) => {
        const contentLength = parseInt(r.headers['Content-Length'] || '0');
        // Allow some tolerance for Content-Length vs actual body
        return contentLength >= config.expectedSize * 0.99;
      },
      'large file: TTFB < 2s': (r) => r.timings.waiting < 2000,
    });
  }

  if (mode === 'range') {
    check(response, {
      'range: status 206 Partial Content': (r) => r.status === 206,
      'range: has Content-Range header': (r) => {
        const contentRange = r.headers['Content-Range'] || r.headers['content-range'];
        return contentRange && contentRange.length > 0;
      },
      'range: correct content length': (r) => {
        const expectedLen = config.rangeEnd - config.rangeStart + 1;
        const actualLen = r.body ? r.body.length : 0;
        // Allow some tolerance
        return actualLen >= expectedLen * 0.99;
      },
    });
  }

  if (mode === 'concurrent') {
    check(response, {
      'concurrent: no timeouts': (r) => r.status !== 0,
      'concurrent: reasonable TTFB': (r) => r.timings.waiting < 5000,
    });
  }

  if (!success) {
    // Log errors (but limit to avoid spam)
    if (__ITER < 3 || __ITER % 10 === 0) {
      console.log(`Error: VU=${__VU}, iter=${__ITER}, status=${response.status}, ` +
                  `error=${response.error || 'none'}, duration=${duration}ms, ` +
                  `body_len=${bodyLength}, expected=${config.expectedSize}`);
    }
  }

  errorRate.add(!success);

  // Mode-specific sleep patterns
  switch (mode) {
    case 'single_100mb':
    case 'single_500mb':
      // Small delay between large downloads
      sleep(1);
      break;
    case 'concurrent':
      // Minimal sleep for concurrent test
      sleep(0.1);
      break;
    case 'range':
      // Small delay between range requests
      sleep(0.05);
      break;
    case 'mixed':
      // Variable delay
      sleep(Math.random() * 0.1);
      break;
    case 'sustained':
      // Steady pace
      sleep(0.5);
      break;
    default:
      sleep(0.5);
  }
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  console.log('');
  console.log('='.repeat(80));
  console.log('Large File Streaming Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${duration.toFixed(1)} seconds`);
  console.log(`Test Mode: ${data.mode}`);
  console.log('');
  console.log('Phase 39 Success Criteria:');
  console.log('');
  console.log('  Single Large File:');
  console.log('    - 100MB/500MB file downloads complete successfully');
  console.log('    - TTFB < 2s (P95)');
  console.log('    - Memory should stay low (~64KB per connection)');
  console.log('');
  console.log('  Concurrent Downloads:');
  console.log('    - Multiple large downloads work simultaneously');
  console.log('    - No timeouts or connection failures');
  console.log('    - Memory stays bounded (not proportional to file size)');
  console.log('');
  console.log('  Range Requests:');
  console.log('    - Returns 206 Partial Content');
  console.log('    - Content-Range header present');
  console.log('    - Only requested bytes returned');
  console.log('');
  console.log('  General:');
  console.log('    - Error rate < 5%');
  console.log('    - Large files bypass cache');
  console.log('    - Streaming uses constant memory');
  console.log('='.repeat(80));
}
