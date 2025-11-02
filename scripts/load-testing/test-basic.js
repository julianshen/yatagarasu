// K6 Load Test: Basic throughput test
// Tests baseline throughput with simple GET requests
// Target: >1,000 req/s single core

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const ttfb = new Trend('time_to_first_byte', true);

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 50 },  // Ramp up to 50 users
    { duration: '1m', target: 100 },  // Ramp up to 100 users
    { duration: '2m', target: 100 },  // Stay at 100 users
    { duration: '30s', target: 0 },   // Ramp down to 0 users
  ],
  thresholds: {
    // 95% of requests should be below 100ms
    'http_req_duration': ['p(95)<100'],
    // Error rate should be below 1%
    'errors': ['rate<0.01'],
    // Request rate should be above 1,000 req/s
    'http_reqs': ['rate>1000'],
  },
};

// Test data
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const TEST_PATH = __ENV.TEST_PATH || '/test/sample.txt';

export default function () {
  const url = `${BASE_URL}${TEST_PATH}`;

  const params = {
    headers: {
      'Accept': 'text/plain',
    },
    tags: {
      name: 'BasicGET',
    },
  };

  const response = http.get(url, params);

  // Record time to first byte
  ttfb.add(response.timings.waiting);

  // Verify response
  const checkResult = check(response, {
    'status is 200': (r) => r.status === 200,
    'response has content': (r) => r.body.length > 0,
    'response time < 100ms': (r) => r.timings.duration < 100,
  });

  // Track errors
  errorRate.add(!checkResult);

  // Think time between requests (optional)
  sleep(0.1);
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'load-test-results.json': JSON.stringify(data),
  };
}

function textSummary(data, options) {
  const indent = options.indent || '';
  const enableColors = options.enableColors || false;

  let summary = '\n' + indent + '══════════════════════════════════════════\n';
  summary += indent + '  YATAGARASU LOAD TEST RESULTS\n';
  summary += indent + '══════════════════════════════════════════\n\n';

  // Request statistics
  summary += indent + 'Requests:\n';
  summary += indent + `  Total: ${data.metrics.http_reqs.values.count}\n`;
  summary += indent + `  Rate: ${data.metrics.http_reqs.values.rate.toFixed(2)} req/s\n`;
  summary += indent + `  Duration: ${(data.state.testRunDurationMs / 1000).toFixed(2)}s\n\n`;

  // Response time statistics
  summary += indent + 'Response Times:\n';
  summary += indent + `  Min: ${data.metrics.http_req_duration.values.min.toFixed(2)}ms\n`;
  summary += indent + `  Max: ${data.metrics.http_req_duration.values.max.toFixed(2)}ms\n`;
  summary += indent + `  Avg: ${data.metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
  summary += indent + `  P50: ${data.metrics.http_req_duration.values['p(50)'].toFixed(2)}ms\n`;
  summary += indent + `  P90: ${data.metrics.http_req_duration.values['p(90)'].toFixed(2)}ms\n`;
  summary += indent + `  P95: ${data.metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
  summary += indent + `  P99: ${data.metrics.http_req_duration.values['p(99)'].toFixed(2)}ms\n\n`;

  // Error rate
  const errorRate = data.metrics.errors ? data.metrics.errors.values.rate : 0;
  summary += indent + `Error Rate: ${(errorRate * 100).toFixed(2)}%\n\n`;

  // Threshold results
  summary += indent + 'Thresholds:\n';
  for (const [name, threshold] of Object.entries(data.metrics)) {
    if (threshold.thresholds) {
      for (const [thresholdName, result] of Object.entries(threshold.thresholds)) {
        const status = result.ok ? '✓' : '✗';
        summary += indent + `  ${status} ${name}: ${thresholdName}\n`;
      }
    }
  }

  summary += indent + '\n══════════════════════════════════════════\n';

  return summary;
}
