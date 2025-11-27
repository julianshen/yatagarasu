// K6 Load Test: OPA Authorization
// Target: Measure OPA authorization overhead and throughput
// Tests: Requests with JWT + OPA policy evaluation
//
// Prerequisites:
// 1. Start OPA server: docker run -p 8181:8181 openpolicyagent/opa run --server
// 2. Load policy: curl -X PUT http://localhost:8181/v1/policies/authz --data-binary @policies/authz.rego
// 3. Start proxy with OPA config: cargo run -- --config config.loadtest-opa.yaml
// 4. Run test: k6 run k6-opa.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Counter, Trend } from 'k6/metrics';
import encoding from 'k6/encoding';

// Custom metrics
const errorRate = new Rate('errors');
const opaAllowed = new Counter('opa_allowed');
const opaDenied = new Counter('opa_denied');
const authLatency = new Trend('auth_latency');

// Test configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const JWT_SECRET = __ENV.JWT_SECRET || 'test-secret-key-for-load-testing-only';

// Test scenarios
export let options = {
  scenarios: {
    // Scenario 1: Constant rate with OPA (main throughput test)
    opa_constant_rate: {
      executor: 'constant-arrival-rate',
      rate: 500, // 500 requests per second
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 50,
      maxVUs: 200,
      exec: 'testOpaAuthorization',
    },
    // Scenario 2: Ramping VUs to find saturation point
    opa_ramping: {
      executor: 'ramping-vus',
      startVUs: 10,
      stages: [
        { duration: '10s', target: 50 },
        { duration: '20s', target: 100 },
        { duration: '10s', target: 50 },
      ],
      exec: 'testOpaAuthorization',
      startTime: '35s', // Start after constant rate test
    },
    // Scenario 3: Cache effectiveness test (same user, same resource)
    opa_cache_hit: {
      executor: 'constant-arrival-rate',
      rate: 1000, // Higher rate to test cache
      timeUnit: '1s',
      duration: '20s',
      preAllocatedVUs: 50,
      maxVUs: 100,
      exec: 'testOpaCacheHit',
      startTime: '80s', // Start after ramping test
    },
    // Scenario 4: Cache miss test (different users)
    opa_cache_miss: {
      executor: 'constant-arrival-rate',
      rate: 200,
      timeUnit: '1s',
      duration: '20s',
      preAllocatedVUs: 50,
      maxVUs: 100,
      exec: 'testOpaCacheMiss',
      startTime: '105s',
    },
  },
  thresholds: {
    // Overall thresholds
    'http_req_duration': ['p(95)<200'], // 95% under 200ms (OPA adds latency)
    'http_req_failed': ['rate<0.01'],   // Error rate under 1%
    'errors': ['rate<0.01'],
    // OPA-specific thresholds
    'auth_latency': ['p(95)<50'],       // Auth latency under 50ms P95
  },
};

// Generate a simple JWT (HS256) for testing
function generateJwt(payload) {
  const header = encoding.b64encode(JSON.stringify({ alg: 'HS256', typ: 'JWT' }), 'rawurl');
  const payloadB64 = encoding.b64encode(JSON.stringify(payload), 'rawurl');
  // Note: k6 doesn't have native HMAC, so we use a pre-generated signature for testing
  // In production tests, use a proper JWT library or pre-generate tokens
  const signature = 'test_signature'; // Replace with actual signature for real tests
  return `${header}.${payloadB64}.${signature}`;
}

// Pre-generated tokens for different test scenarios
// These should be generated with the actual JWT_SECRET before running tests
const TOKENS = {
  admin: __ENV.ADMIN_TOKEN || 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiIsInJvbGVzIjpbImFkbWluIl0sImV4cCI6MTkwMDAwMDAwMH0.test',
  user: __ENV.USER_TOKEN || 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyMSIsInJvbGVzIjpbInVzZXIiXSwiYWxsb3dlZF9idWNrZXQiOiJwcml2YXRlIiwiZXhwIjoxOTAwMDAwMDAwfQ.test',
  denied: __ENV.DENIED_TOKEN || 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkZW5pZWQiLCJyb2xlcyI6W10sImV4cCI6MTkwMDAwMDAwMH0.test',
};

// Main OPA authorization test
export function testOpaAuthorization() {
  const startTime = new Date();

  // Alternate between admin and user tokens
  const token = Math.random() > 0.5 ? TOKENS.admin : TOKENS.user;

  const res = http.get(`${BASE_URL}/opa-protected/test-file.txt`, {
    headers: {
      'Authorization': `Bearer ${token}`,
    },
  });

  const authTime = new Date() - startTime;
  authLatency.add(authTime);

  const result = check(res, {
    'status is 200 or 403': (r) => r.status === 200 || r.status === 403,
    'response time < 200ms': (r) => r.timings.duration < 200,
  });

  if (res.status === 200) {
    opaAllowed.add(1);
  } else if (res.status === 403) {
    opaDenied.add(1);
  }

  errorRate.add(!result);
}

// Cache hit test - same user, same resource repeatedly
export function testOpaCacheHit() {
  const startTime = new Date();

  // Always use admin token for consistent cache hits
  const res = http.get(`${BASE_URL}/opa-protected/cached-file.txt`, {
    headers: {
      'Authorization': `Bearer ${TOKENS.admin}`,
    },
  });

  const authTime = new Date() - startTime;
  authLatency.add(authTime);

  const result = check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 100ms (cached)': (r) => r.timings.duration < 100,
  });

  if (res.status === 200) {
    opaAllowed.add(1);
  }

  errorRate.add(!result);
}

// Cache miss test - different users to test OPA without cache
export function testOpaCacheMiss() {
  const startTime = new Date();

  // Use unique path for each request to avoid cache
  const uniquePath = `/opa-protected/unique-${__VU}-${__ITER}.txt`;

  const res = http.get(`${BASE_URL}${uniquePath}`, {
    headers: {
      'Authorization': `Bearer ${TOKENS.admin}`,
    },
  });

  const authTime = new Date() - startTime;
  authLatency.add(authTime);

  const result = check(res, {
    'status is 200 or 404': (r) => r.status === 200 || r.status === 404,
    'response time < 300ms (uncached)': (r) => r.timings.duration < 300,
  });

  errorRate.add(!result && res.status !== 404);
}

// Default function (runs if no scenario specified)
export default function () {
  testOpaAuthorization();
  sleep(0.1);
}

// Setup: Run once before tests
export function setup() {
  console.log('Starting OPA Load Test');
  console.log(`Base URL: ${BASE_URL}`);

  // Verify OPA is accessible
  const healthCheck = http.get(`${BASE_URL}/health`);
  if (healthCheck.status !== 200) {
    console.warn('Warning: Proxy health check failed');
  }

  return { startTime: new Date().toISOString() };
}

// Teardown: Run once after tests
export function teardown(data) {
  console.log(`OPA Load Test completed`);
  console.log(`Started at: ${data.startTime}`);
  console.log(`Finished at: ${new Date().toISOString()}`);
}
