// K6 Load Test: Baseline Throughput
// Target: >1,000 req/s for 30 seconds
// Tests: Simple GET requests to small file

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export let options = {
  scenarios: {
    constant_request_rate: {
      executor: 'constant-arrival-rate',
      rate: 1000, // 1000 requests per second
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 50, // Start with 50 VUs
      maxVUs: 200, // Allow up to 200 VUs if needed
    },
  },
  thresholds: {
    'http_req_duration': ['p(95)<100'], // 95% of requests under 100ms
    'http_req_failed': ['rate<0.01'],   // Error rate under 1%
    'errors': ['rate<0.01'],
  },
};

export default function () {
  const res = http.get('http://localhost:8080/test/sample.txt');

  const result = check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 100ms': (r) => r.timings.duration < 100,
    'body contains content': (r) => r.body && r.body.length > 0,
  });

  errorRate.add(!result);
}
