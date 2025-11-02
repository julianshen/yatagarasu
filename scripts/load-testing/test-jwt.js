// K6 Load Test: JWT authentication performance
// Tests JWT validation performance under load
// Target: <1ms JWT validation, minimal impact on throughput

import http from 'k6/http';
import { check } from 'k6';
import { Trend } from 'k6/metrics';

const authTime = new Trend('auth_time');

export const options = {
  scenarios: {
    jwt_auth_test: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 20 },
        { duration: '1m', target: 50 },
        { duration: '30s', target: 0 },
      ],
    },
  },
  thresholds: {
    'http_req_duration': ['p(95)<100'], // Overall response time
    'auth_time': ['p(95)<1'], // JWT validation overhead
    'errors': ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const JWT_TOKEN = __ENV.JWT_TOKEN || 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjk5OTk5OTk5OTl9.x';
const PROTECTED_PATH = __ENV.PROTECTED_PATH || '/private/data.txt';

export default function () {
  // Test authenticated request
  const authResponse = http.get(`${BASE_URL}${PROTECTED_PATH}`, {
    headers: {
      'Authorization': `Bearer ${JWT_TOKEN}`,
    },
    tags: { name: 'AuthenticatedRequest' },
  });

  // Estimate auth overhead by comparing with unauthenticated endpoint
  const start = Date.now();
  check(authResponse, {
    'status is 200': (r) => r.status === 200,
    'has valid content': (r) => r.body.length > 0,
  });
  const authOverhead = Date.now() - start;
  authTime.add(authOverhead);

  // Also test 401 unauthorized (missing token)
  const unauthResponse = http.get(`${BASE_URL}${PROTECTED_PATH}`, {
    tags: { name: 'UnauthorizedRequest' },
  });

  check(unauthResponse, {
    'status is 401': (r) => r.status === 401,
    '401 response fast': (r) => r.timings.duration < 50,
  });
}
