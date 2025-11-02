// K6 Load Test: Streaming and TTFB
// Target: TTFB <100ms for P95
// Tests: Time to first byte for streaming responses

import http from 'k6/http';
import { check } from 'k6';
import { Trend, Rate } from 'k6/metrics';

const ttfb = new Trend('time_to_first_byte', true);
const errorRate = new Rate('errors');

export let options = {
  scenarios: {
    streaming_latency: {
      executor: 'constant-arrival-rate',
      rate: 100, // 100 requests per second
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      maxVUs: 50,
    },
  },
  thresholds: {
    'time_to_first_byte': ['p(95)<100', 'p(99)<200'], // TTFB targets
    'http_req_failed': ['rate<0.01'],
    'http_req_duration': ['p(95)<500'],
    'errors': ['rate<0.01'],
  },
};

const files = [
  '/test/sample.txt',
  '/test/1kb.bin',
];

export default function () {
  const file = files[Math.floor(Math.random() * files.length)];
  const url = `http://localhost:8080${file}`;

  const res = http.get(url);

  // TTFB is the waiting time (time between request sent and first byte received)
  ttfb.add(res.timings.waiting);

  const result = check(res, {
    'status is 200': (r) => r.status === 200,
    'TTFB < 100ms (P95 target)': (r) => r.timings.waiting < 100,
    'TTFB < 200ms (P99 target)': (r) => r.timings.waiting < 200,
    'total response time < 500ms': (r) => r.timings.duration < 500,
    'has content': (r) => r.body && r.body.length > 0,
  });

  errorRate.add(!result);
}

export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    '/tmp/k6-streaming-results.json': JSON.stringify(data),
  };
}

function textSummary(data, opts) {
  const indent = opts.indent || '';
  const summary = [];

  summary.push(`${indent}TTFB (Time to First Byte):`);
  summary.push(`${indent}  avg: ${data.metrics.time_to_first_byte.values.avg.toFixed(2)}ms`);
  summary.push(`${indent}  p95: ${data.metrics.time_to_first_byte.values['p(95)'].toFixed(2)}ms`);
  summary.push(`${indent}  p99: ${data.metrics.time_to_first_byte.values['p(99)'].toFixed(2)}ms`);
  summary.push(`${indent}  max: ${data.metrics.time_to_first_byte.values.max.toFixed(2)}ms`);

  return summary.join('\n');
}
