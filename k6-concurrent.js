// K6 Load Test: Concurrent Connections
// Target: 100 concurrent users for 60 seconds
// Tests: Sustained concurrent connections with realistic think time

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

const errors = new Counter('errors');
const errorRate = new Rate('error_rate');
const ttfb = new Trend('time_to_first_byte');

export let options = {
  scenarios: {
    concurrent_users: {
      executor: 'constant-vus',
      vus: 100, // 100 concurrent virtual users
      duration: '60s',
    },
  },
  thresholds: {
    'http_req_duration': ['p(95)<500', 'p(99)<1000'], // 95th percentile < 500ms, 99th < 1s
    'http_req_failed': ['rate<0.01'],   // Error rate under 1%
    'time_to_first_byte': ['p(95)<200'], // TTFB 95th percentile < 200ms
    'error_rate': ['rate<0.01'],
  },
};

const files = [
  '/test/sample.txt',
  '/test/1kb.bin',
];

export default function () {
  // Pick random file
  const file = files[Math.floor(Math.random() * files.length)];
  const url = `http://localhost:8080${file}`;

  const res = http.get(url);

  // Record TTFB
  ttfb.add(res.timings.waiting);

  const result = check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 1s': (r) => r.timings.duration < 1000,
    'TTFB < 200ms': (r) => r.timings.waiting < 200,
    'has content': (r) => r.body && r.body.length > 0,
  });

  if (!result) {
    errors.add(1);
  }
  errorRate.add(!result);

  // Realistic user think time: 500ms-1500ms
  sleep(0.5 + Math.random());
}
