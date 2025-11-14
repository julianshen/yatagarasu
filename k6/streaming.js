/**
 * K6 Streaming Latency Test (TTFB)
 *
 * Objective: Verify TTFB (Time To First Byte) < 100ms for large file streaming
 *
 * Test Configuration:
 * - Duration: 60 seconds
 * - Virtual Users: 10 concurrent users
 * - File: 10MB test file (large file streaming)
 * - Metric: Time To First Byte (TTFB)
 *
 * Success Criteria:
 * - P95 TTFB < 100ms
 * - Streaming starts immediately (no buffering)
 * - No timeouts during download
 *
 * Usage:
 *   k6 run k6/streaming.js
 *
 * Prerequisites:
 *   - Yatagarasu proxy running on http://localhost:8080
 *   - MinIO running with test bucket
 *   - 10MB test file uploaded to /public/test-10mb.bin
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate } from 'k6/metrics';

// Custom metrics
const ttfbMetric = new Trend('ttfb', true); // Time To First Byte
const downloadDuration = new Trend('download_duration');
const errorRate = new Rate('errors');

// Test options
export const options = {
  vus: 10,
  duration: '60s',

  thresholds: {
    ttfb: ['p(95)<100'],              // 95% TTFB < 100ms
    http_req_duration: ['p(95)<5000'], // 95% total time < 5s (10MB file)
    http_req_failed: ['rate<0.001'],   // Error rate < 0.1%
  },
};

// Setup
export function setup() {
  console.log('='.repeat(80));
  console.log('K6 Streaming Latency Test (TTFB) - Yatagarasu S3 Proxy');
  console.log('='.repeat(80));
  console.log('Target: TTFB < 100ms (P95)');
  console.log('Duration: 60 seconds');
  console.log('Virtual Users: 10');
  console.log('File: 10MB (streaming test)');
  console.log('='.repeat(80));
}

// Main test function
export default function () {
  const url = 'http://localhost:8080/public/test-10mb.bin';

  const params = {
    timeout: '30s', // 30 second timeout for large file
  };

  const response = http.get(url, params);

  // Calculate TTFB (time to first byte)
  // In K6, waiting time is DNS + TCP + TLS + Server processing until first byte
  const ttfb = response.timings.waiting;
  ttfbMetric.add(ttfb);

  // Total download duration
  downloadDuration.add(response.timings.duration);

  // Validation checks
  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'TTFB < 100ms': (r) => r.timings.waiting < 100,
    'TTFB < 200ms': (r) => r.timings.waiting < 200, // Warning threshold
    'response has body': (r) => r.body && r.body.length > 0,
    'file size correct': (r) => r.body.length >= 10 * 1024 * 1024, // ~10MB
    'no timeout': (r) => r.error_code === 0,
  });

  errorRate.add(!success);

  // Longer think time for large file downloads
  sleep(1); // 1s between requests per VU (rate limiting)
}

// Teardown
export function teardown(data) {
  console.log('='.repeat(80));
  console.log('Streaming Latency Test Complete!');
  console.log('='.repeat(80));
  console.log('Check results above for:');
  console.log('  - ttfb (p95): 95th percentile Time To First Byte');
  console.log('  - ttfb (avg): Average TTFB');
  console.log('  - download_duration (p95): 95th percentile download time');
  console.log('  - http_req_failed: Error rate');
  console.log('');
  console.log('Success Criteria:');
  console.log('  ✓ P95 TTFB < 100ms (streaming starts immediately)');
  console.log('  ✓ No timeouts during download');
  console.log('  ✓ File size correct (~10MB)');
  console.log('='.repeat(80));
}
