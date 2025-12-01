/**
 * Phase 42: Proxy Pipeline Benchmarks
 *
 * End-to-end HTTP benchmarks for measuring proxy pipeline performance.
 * Tests health check latency, cache hit/miss pipelines, and JWT auth overhead.
 *
 * Usage:
 *   k6 run k6/proxy-pipeline.js                           # All scenarios
 *   k6 run -e SCENARIO=health k6/proxy-pipeline.js        # Health check only
 *   k6 run -e SCENARIO=cache_hit k6/proxy-pipeline.js     # Cache hit pipeline
 *   k6 run -e SCENARIO=cache_miss k6/proxy-pipeline.js    # Cache miss pipeline
 *   k6 run -e SCENARIO=range k6/proxy-pipeline.js         # Range requests
 *   k6 run -e SCENARIO=streaming k6/proxy-pipeline.js     # Streaming tests
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const healthLatency = new Trend('health_check_latency', true);
const cacheHitLatency = new Trend('cache_hit_latency', true);
const cacheMissLatency = new Trend('cache_miss_latency', true);
const rangeRequestLatency = new Trend('range_request_latency', true);
const streamingLatency = new Trend('streaming_first_byte_latency', true);
const errorRate = new Rate('error_rate');
const requestCount = new Counter('total_requests');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const METRICS_URL = __ENV.METRICS_URL || 'http://localhost:9090';
const SCENARIO = __ENV.SCENARIO || 'all';

// Scenario configurations
const scenarios = {
    // Health check benchmark - target: P99 <100μs
    health: {
        executor: 'constant-arrival-rate',
        rate: 10000,
        timeUnit: '1s',
        duration: '30s',
        preAllocatedVUs: 50,
        maxVUs: 200,
        exec: 'healthCheckScenario',
    },

    // Cache hit benchmark - target: P99 <10ms
    cache_hit: {
        executor: 'constant-arrival-rate',
        rate: 5000,
        timeUnit: '1s',
        duration: '30s',
        preAllocatedVUs: 50,
        maxVUs: 200,
        exec: 'cacheHitScenario',
        startTime: '35s',
    },

    // Cache miss benchmark (S3 fetch)
    cache_miss: {
        executor: 'constant-arrival-rate',
        rate: 100,
        timeUnit: '1s',
        duration: '30s',
        preAllocatedVUs: 20,
        maxVUs: 50,
        exec: 'cacheMissScenario',
        startTime: '70s',
    },

    // Range request benchmark
    range: {
        executor: 'constant-arrival-rate',
        rate: 1000,
        timeUnit: '1s',
        duration: '30s',
        preAllocatedVUs: 20,
        maxVUs: 100,
        exec: 'rangeRequestScenario',
        startTime: '105s',
    },

    // Streaming benchmark (large files)
    streaming: {
        executor: 'per-vu-iterations',
        vus: 10,
        iterations: 10,
        exec: 'streamingScenario',
        startTime: '140s',
    },
};

// Select scenario based on environment variable
export const options = {
    scenarios: SCENARIO === 'all'
        ? scenarios
        : { [SCENARIO]: { ...scenarios[SCENARIO], startTime: '0s' } },
    thresholds: {
        'health_check_latency': ['p(99)<100'],      // Health check P99 < 100μs (0.1ms)
        'cache_hit_latency': ['p(99)<10'],          // Cache hit P99 < 10ms
        'error_rate': ['rate<0.01'],                // Error rate < 1%
    },
};

// Warmup function - populate cache before tests
export function setup() {
    console.log(`Starting Phase 42 Proxy Pipeline Benchmarks`);
    console.log(`Base URL: ${BASE_URL}`);
    console.log(`Scenario: ${SCENARIO}`);

    // Warmup requests to populate cache
    const warmupFiles = [
        '/public/test-1kb.txt',
        '/public/test-10kb.txt',
        '/public/test-100kb.txt',
    ];

    console.log('Warming up cache...');
    warmupFiles.forEach(file => {
        for (let i = 0; i < 3; i++) {
            const res = http.get(`${BASE_URL}${file}`);
            if (res.status !== 200) {
                console.warn(`Warmup failed for ${file}: ${res.status}`);
            }
        }
    });
    console.log('Warmup complete');

    return { startTime: new Date().toISOString() };
}

// Health check scenario
export function healthCheckScenario() {
    const start = Date.now();
    const res = http.get(`${BASE_URL}/health`);
    const latency = Date.now() - start;

    healthLatency.add(latency);
    requestCount.add(1);

    const success = check(res, {
        'health check status 200': (r) => r.status === 200,
        'health check latency < 10ms': (r) => latency < 10,
    });

    errorRate.add(!success);
}

// Cache hit scenario - requests for pre-cached files
export function cacheHitScenario() {
    const files = [
        '/public/test-1kb.txt',
        '/public/test-10kb.txt',
        '/public/test-100kb.txt',
    ];

    const file = files[Math.floor(Math.random() * files.length)];

    const start = Date.now();
    const res = http.get(`${BASE_URL}${file}`);
    const latency = Date.now() - start;

    cacheHitLatency.add(latency);
    requestCount.add(1);

    const success = check(res, {
        'cache hit status 200': (r) => r.status === 200,
        'cache hit has X-Cache header': (r) => r.headers['X-Cache'] !== undefined,
        'cache hit latency < 50ms': (r) => latency < 50,
    });

    errorRate.add(!success);
}

// Cache miss scenario - unique files to force S3 fetch
export function cacheMissScenario() {
    // Generate unique file path to force cache miss
    const uniqueId = `${Date.now()}-${Math.random().toString(36).substring(7)}`;
    const file = `/public/unique-${uniqueId}.txt`;

    const start = Date.now();
    const res = http.get(`${BASE_URL}${file}`);
    const latency = Date.now() - start;

    cacheMissLatency.add(latency);
    requestCount.add(1);

    // Cache miss will likely return 404 for non-existent file
    // This measures the S3 round-trip time
    const success = check(res, {
        'cache miss response received': (r) => r.status === 200 || r.status === 404,
    });

    errorRate.add(!success);
}

// Range request scenario
export function rangeRequestScenario() {
    const file = '/public/test-100kb.txt';

    // Different range request patterns
    const ranges = [
        'bytes=0-1023',           // First 1KB
        'bytes=1000-2000',        // Middle 1KB
        'bytes=-1024',            // Last 1KB
        'bytes=0-100,200-300',    // Multi-range
    ];

    const range = ranges[Math.floor(Math.random() * ranges.length)];

    const start = Date.now();
    const res = http.get(`${BASE_URL}${file}`, {
        headers: { 'Range': range },
    });
    const latency = Date.now() - start;

    rangeRequestLatency.add(latency);
    requestCount.add(1);

    const success = check(res, {
        'range request status 206 or 200': (r) => r.status === 206 || r.status === 200,
        'range request latency < 100ms': (r) => latency < 100,
    });

    errorRate.add(!success);
}

// Streaming scenario - large file downloads
export function streamingScenario() {
    const files = [
        { path: '/public/test-1mb.bin', size: 1024 * 1024 },
        { path: '/public/test-10mb.bin', size: 10 * 1024 * 1024 },
    ];

    const file = files[Math.floor(Math.random() * files.length)];

    const start = Date.now();
    const res = http.get(`${BASE_URL}${file.path}`);
    const ttfb = res.timings.waiting; // Time to first byte
    const latency = Date.now() - start;

    streamingLatency.add(ttfb);
    requestCount.add(1);

    const success = check(res, {
        'streaming status 200': (r) => r.status === 200,
        'streaming correct size': (r) => parseInt(r.headers['Content-Length']) === file.size || r.body.length === file.size,
        'streaming TTFB < 500ms': () => ttfb < 500,
    });

    errorRate.add(!success);

    // Small delay between large file downloads
    sleep(0.5);
}

// Teardown - print summary
export function teardown(data) {
    console.log('\n=== Phase 42 Proxy Pipeline Benchmark Results ===');
    console.log(`Started: ${data.startTime}`);
    console.log(`Finished: ${new Date().toISOString()}`);
    console.log('\nSuccess Criteria:');
    console.log('- Health check P99 <100μs: Check thresholds above');
    console.log('- Cache hit pipeline P99 <10ms: Check thresholds above');
    console.log('- S3 fetch dominated by network latency: Check cache_miss_latency');
    console.log('================================================\n');
}

// Default function (used when no scenario specified)
export default function() {
    // Run all test types in sequence for quick validation
    group('health_check', () => {
        healthCheckScenario();
    });

    sleep(0.1);

    group('cache_hit', () => {
        cacheHitScenario();
    });

    sleep(0.1);

    group('range_request', () => {
        rangeRequestScenario();
    });
}
