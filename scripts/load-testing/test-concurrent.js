// K6 Load Test: Concurrent connections test
// Tests proxy behavior under high concurrent load
// Target: 100 concurrent connections handling 1,000 requests

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export const options = {
  scenarios: {
    // Constant load: 100 concurrent users
    concurrent_users: {
      executor: 'constant-vus',
      vus: 100,
      duration: '2m',
    },
  },
  thresholds: {
    'http_req_duration': ['p(95)<200'],
    'errors': ['rate<0.01'],
    'http_reqs': ['count>1000'], // At least 1,000 requests total
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const TEST_PATH = __ENV.TEST_PATH || '/test/sample.txt';

export default function () {
  const response = http.get(`${BASE_URL}${TEST_PATH}`);

  const checkResult = check(response, {
    'status is 200': (r) => r.status === 200,
    'no server errors': (r) => r.status < 500,
  });

  errorRate.add(!checkResult);

  sleep(0.5); // Half-second think time
}
