// K6 Load Test: Streaming latency test
// Tests large file streaming performance and TTFB
// Target: <100ms TTFB, constant memory usage

import http from 'k6/http';
import { check } from 'k6';
import { Trend } from 'k6/metrics';

const ttfb = new Trend('time_to_first_byte');
const downloadTime = new Trend('download_time');

export const options = {
  scenarios: {
    streaming_test: {
      executor: 'constant-vus',
      vus: 10,
      duration: '1m',
    },
  },
  thresholds: {
    'time_to_first_byte': ['p(95)<100'], // TTFB < 100ms at P95
    'http_req_duration': ['p(95)<5000'], // Complete download < 5s at P95
    'errors': ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const LARGE_FILE = __ENV.LARGE_FILE || '/test/largefile.bin';

export default function () {
  const response = http.get(`${BASE_URL}${LARGE_FILE}`, {
    tags: { name: 'LargeFileDownload' },
  });

  // Record TTFB (time until first byte received)
  ttfb.add(response.timings.waiting);

  // Record total download time
  downloadTime.add(response.timings.duration);

  check(response, {
    'status is 200': (r) => r.status === 200,
    'TTFB < 100ms': (r) => r.timings.waiting < 100,
    'has content': (r) => r.body.length > 0,
  });
}
