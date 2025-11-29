/**
 * K6 Stability Test - 10 minute sustained load
 *
 * Tests:
 * - Memory stability (no leaks)
 * - Sustained throughput
 * - Error rate under prolonged load
 *
 * Usage:
 *   k6 run k6/stability-test.js
 */

import http from 'k6/http';
import { check } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

const errorRate = new Rate('errors');
const requestDuration = new Trend('request_duration_ms');
const totalRequests = new Counter('total_requests');

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const DURATION = __ENV.DURATION || '10m';

const TEST_FILES = [
  '/public/test-1kb.txt',
  '/public/test-10kb.txt',
  '/public/test-100kb.txt',
  '/public/test-1mb.bin',
];

export const options = {
  scenarios: {
    sustained_load: {
      executor: 'constant-arrival-rate',
      rate: 500,
      timeUnit: '1s',
      duration: DURATION,
      preAllocatedVUs: 50,
      maxVUs: 100,
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<100'],  // P95 < 100ms
    http_req_failed: ['rate<0.001'],   // Error rate < 0.1%
    errors: ['rate<0.001'],
  },
};

export function setup() {
  console.log('='.repeat(80));
  console.log('Stability Test - 10 Minute Sustained Load');
  console.log('='.repeat(80));
  console.log(`Base URL: ${BASE_URL}`);
  console.log(`Duration: ${DURATION}`);
  console.log(`Rate: 500 req/s`);
  console.log('');
  console.log('Monitoring for:');
  console.log('  - Memory stability (no growth over time)');
  console.log('  - Consistent latency');
  console.log('  - Zero errors');
  console.log('='.repeat(80));

  // Warm up
  for (let i = 0; i < 10; i++) {
    http.get(`${BASE_URL}${TEST_FILES[0]}`);
  }

  return { startTime: Date.now() };
}

let requestId = 0;

export default function (data) {
  const fileIndex = requestId % TEST_FILES.length;
  const url = `${BASE_URL}${TEST_FILES[fileIndex]}?id=${requestId++}`;

  const response = http.get(url);

  totalRequests.add(1);
  requestDuration.add(response.timings.duration);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has body': (r) => r.body && r.body.length > 0,
    'latency < 100ms': (r) => r.timings.duration < 100,
  });

  errorRate.add(!success);
}

export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000;
  const minutes = Math.floor(duration / 60);
  const seconds = Math.floor(duration % 60);

  console.log('');
  console.log('='.repeat(80));
  console.log('Stability Test Complete!');
  console.log('='.repeat(80));
  console.log(`Duration: ${minutes}m ${seconds}s`);
  console.log('');
  console.log('Success Criteria:');
  console.log('  [?] Memory stable (check container stats)');
  console.log('  [?] P95 latency < 100ms');
  console.log('  [?] Error rate < 0.1%');
  console.log('  [?] No crashes or restarts');
  console.log('='.repeat(80));
}
