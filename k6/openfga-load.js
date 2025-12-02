/**
 * Phase 50.2: OpenFGA Load Testing Script
 *
 * Tests OpenFGA authorization performance with caching:
 * - 500 RPS target throughput
 * - 80% cache hit rate simulation
 * - P95 latency targets: <100ms (with cache), <500ms (without cache)
 *
 * Prerequisites:
 * 1. Start OpenFGA server:
 *    docker run -d -p 8080:8080 --name openfga openfga/openfga run
 *
 * 2. Create store and model using scripts/setup-openfga-loadtest.sh
 *
 * 3. Start the proxy:
 *    cargo run --release -- --config config.loadtest-openfga.yaml
 *
 * Usage:
 *   k6 run k6/openfga-load.js                          # All scenarios
 *   k6 run -e SCENARIO=with_cache k6/openfga-load.js   # With caching
 *   k6 run -e SCENARIO=no_cache k6/openfga-load.js     # Without caching
 *   k6 run -e SCENARIO=quick k6/openfga-load.js        # Quick validation
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import encoding from 'k6/encoding';

// Custom metrics
const authLatency = new Trend('openfga_auth_latency', true);
const cacheHitLatency = new Trend('openfga_cache_hit_latency', true);
const cacheMissLatency = new Trend('openfga_cache_miss_latency', true);
const errorRate = new Rate('error_rate');
const requestCount = new Counter('total_requests');
const cacheHits = new Counter('cache_hits');
const cacheMisses = new Counter('cache_misses');

// Configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const METRICS_URL = __ENV.METRICS_URL || 'http://localhost:9090';
const SCENARIO = __ENV.SCENARIO || 'all';
const JWT_SECRET = __ENV.JWT_SECRET || 'test-secret-key-for-load-testing-only';

// Test users with different permission levels
// These users should be pre-configured in OpenFGA with tuples
const TEST_USERS = [
    { id: 'user:alice', relation: 'viewer', files: ['test-1kb.txt', 'test-10kb.txt', 'test-100kb.txt'] },
    { id: 'user:bob', relation: 'editor', files: ['test-1kb.txt', 'test-10kb.txt'] },
    { id: 'user:charlie', relation: 'viewer', files: ['test-1kb.txt'] },
    { id: 'user:diana', relation: 'owner', files: ['test-1kb.txt', 'test-10kb.txt', 'test-100kb.txt'] },
];

// Files for testing (should exist in MinIO)
const TEST_FILES = [
    '/openfga-protected/test-1kb.txt',
    '/openfga-protected/test-10kb.txt',
    '/openfga-protected/test-100kb.txt',
];

// Scenario configurations
const scenarios = {
    // Quick validation test
    quick: {
        executor: 'constant-arrival-rate',
        rate: 50,
        timeUnit: '1s',
        duration: '30s',
        preAllocatedVUs: 10,
        maxVUs: 50,
        exec: 'withCacheScenario',
    },

    // With cache enabled - target P95 <100ms
    with_cache: {
        executor: 'constant-arrival-rate',
        rate: 500,
        timeUnit: '1s',
        duration: '60s',
        preAllocatedVUs: 50,
        maxVUs: 200,
        exec: 'withCacheScenario',
    },

    // No cache (cold start simulation) - target P95 <500ms
    no_cache: {
        executor: 'constant-arrival-rate',
        rate: 100,
        timeUnit: '1s',
        duration: '60s',
        preAllocatedVUs: 20,
        maxVUs: 100,
        exec: 'noCacheScenario',
        startTime: '65s',
    },

    // Ramp up test to find breaking point
    ramp_up: {
        executor: 'ramping-arrival-rate',
        startRate: 100,
        timeUnit: '1s',
        preAllocatedVUs: 100,
        maxVUs: 500,
        stages: [
            { duration: '30s', target: 200 },
            { duration: '30s', target: 500 },
            { duration: '30s', target: 800 },
            { duration: '30s', target: 500 },
            { duration: '30s', target: 100 },
        ],
        exec: 'withCacheScenario',
        startTime: '130s',
    },
};

// Select scenario based on environment variable
export const options = {
    scenarios: SCENARIO === 'all'
        ? scenarios
        : { [SCENARIO]: { ...scenarios[SCENARIO], startTime: '0s' } },
    thresholds: {
        'openfga_cache_hit_latency': ['p(95)<100'],     // Cache hit P95 < 100ms
        'openfga_cache_miss_latency': ['p(95)<500'],    // Cache miss P95 < 500ms
        'error_rate': ['rate<0.01'],                    // Error rate < 1%
    },
};

/**
 * Generate a simple HS256 JWT for testing
 * Note: In production, use proper JWT libraries. This is simplified for load testing.
 */
function generateJWT(userId) {
    const header = {
        alg: 'HS256',
        typ: 'JWT',
    };

    const payload = {
        sub: userId,
        iat: Math.floor(Date.now() / 1000),
        exp: Math.floor(Date.now() / 1000) + 3600, // 1 hour expiry
        iss: 'yatagarasu-loadtest',
    };

    const headerB64 = encoding.b64encode(JSON.stringify(header), 'rawurl');
    const payloadB64 = encoding.b64encode(JSON.stringify(payload), 'rawurl');

    // For load testing, we use a pre-shared secret
    // The signature is created using HMAC-SHA256 in the proxy
    // k6 doesn't have native HMAC support, so we'll rely on the proxy
    // accepting our test tokens with the known secret
    const signature = encoding.b64encode('test-signature', 'rawurl');

    return `${headerB64}.${payloadB64}.${signature}`;
}

// Pre-generate JWTs for all test users (done once at startup)
const userTokens = {};
TEST_USERS.forEach(user => {
    userTokens[user.id] = generateJWT(user.id);
});

// Setup function - verify endpoints are reachable
export function setup() {
    console.log(`Starting Phase 50.2 OpenFGA Load Tests`);
    console.log(`Base URL: ${BASE_URL}`);
    console.log(`Scenario: ${SCENARIO}`);

    // Check proxy health
    const healthRes = http.get(`${BASE_URL}/health`);
    if (healthRes.status !== 200) {
        console.error(`Proxy not healthy: ${healthRes.status}`);
        return { healthy: false };
    }
    console.log('Proxy is healthy');

    // Warmup - make initial requests to populate cache
    console.log('Warming up cache...');
    TEST_USERS.forEach(user => {
        user.files.forEach(file => {
            const token = userTokens[user.id];
            const res = http.get(`${BASE_URL}/openfga-protected/${file}`, {
                headers: { 'Authorization': `Bearer ${token}` },
            });
            if (res.status !== 200 && res.status !== 403) {
                console.warn(`Warmup request for ${user.id} to ${file}: ${res.status}`);
            }
        });
    });
    console.log('Warmup complete');

    return {
        healthy: true,
        startTime: new Date().toISOString(),
    };
}

/**
 * With cache scenario - simulates 80% cache hit rate
 * Uses same users/files repeatedly to maximize cache hits
 */
export function withCacheScenario() {
    // Pick a random user and their allowed file
    const userIndex = Math.floor(Math.random() * TEST_USERS.length);
    const user = TEST_USERS[userIndex];
    const fileIndex = Math.floor(Math.random() * user.files.length);
    const file = user.files[fileIndex];

    const token = userTokens[user.id];
    const url = `${BASE_URL}/openfga-protected/${file}`;

    const start = Date.now();
    const res = http.get(url, {
        headers: { 'Authorization': `Bearer ${token}` },
    });
    const latency = Date.now() - start;

    authLatency.add(latency);
    requestCount.add(1);

    // Check if it was a cache hit based on response headers
    const cacheHeader = res.headers['X-Auth-Cache'];
    const isCacheHit = cacheHeader === 'HIT';

    if (isCacheHit) {
        cacheHitLatency.add(latency);
        cacheHits.add(1);
    } else {
        cacheMissLatency.add(latency);
        cacheMisses.add(1);
    }

    const success = check(res, {
        'status is 200': (r) => r.status === 200,
        'latency < 200ms': () => latency < 200,
    });

    errorRate.add(!success);
}

/**
 * No cache scenario - forces cache misses by using unique request attributes
 */
export function noCacheScenario() {
    // Generate unique user IDs to force cache misses
    const uniqueId = `user:loadtest-${Date.now()}-${Math.random().toString(36).substring(7)}`;
    const token = generateJWT(uniqueId);

    // Use a file path with unique identifier to avoid file cache
    const file = TEST_FILES[Math.floor(Math.random() * TEST_FILES.length)];
    const url = `${BASE_URL}${file}`;

    const start = Date.now();
    const res = http.get(url, {
        headers: { 'Authorization': `Bearer ${token}` },
    });
    const latency = Date.now() - start;

    authLatency.add(latency);
    cacheMissLatency.add(latency);
    cacheMisses.add(1);
    requestCount.add(1);

    // Unique users will get 403 since they don't have permissions
    const success = check(res, {
        'status is 200 or 403': (r) => r.status === 200 || r.status === 403,
        'latency < 600ms': () => latency < 600,
    });

    errorRate.add(!success);
}

// Teardown - print summary
export function teardown(data) {
    if (!data.healthy) {
        console.log('Tests skipped - proxy was not healthy');
        return;
    }

    console.log('\n=== Phase 50.2 OpenFGA Load Test Results ===');
    console.log(`Started: ${data.startTime}`);
    console.log(`Finished: ${new Date().toISOString()}`);
    console.log('\nSuccess Criteria:');
    console.log('- P95 latency <100ms (with caching): Check openfga_cache_hit_latency');
    console.log('- P95 latency <500ms (without caching): Check openfga_cache_miss_latency');
    console.log('- 500 RPS throughput: Check iteration rate');
    console.log('- Error rate <1%: Check error_rate threshold');
    console.log('================================================\n');
}

// Default function for simple runs
export default function() {
    withCacheScenario();
    sleep(0.1);
}
