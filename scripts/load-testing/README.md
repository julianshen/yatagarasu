# Yatagarasu Load Testing with K6

This directory contains K6 load testing scripts for validating Yatagarasu proxy performance.

## Prerequisites

### 1. Install K6

**macOS**:
```bash
brew install k6
```

**Linux**:
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Windows**:
```bash
choco install k6
```

Or download from: https://k6.io/docs/get-started/installation/

### 2. Install Docker

Required for running MinIO (S3-compatible storage).

Download from: https://www.docker.com/get-started

## Quick Start

### 1. Setup Test Environment

Run the setup script to start MinIO and create test data:

```bash
chmod +x setup-test-env.sh start-minio.sh
./setup-test-env.sh
```

This will:
- Start MinIO container on port 9000
- Create test buckets (`test-public`, `test-private`)
- Upload test files (1KB, 100KB, 10MB, 100MB)
- Generate Yatagarasu config at `/tmp/yatagarasu-test/config.yaml`

### 2. Start Yatagarasu Proxy

In another terminal:

```bash
cargo run --release -- --config /tmp/yatagarasu-test/config.yaml
```

Wait for output: `Starting Yatagarasu S3 Proxy`

### 3. Run Load Tests

**Basic throughput test** (target: >1,000 req/s):
```bash
k6 run test-basic.js
```

**Concurrent connections test** (100 concurrent users):
```bash
k6 run test-concurrent.js
```

**Streaming latency test** (TTFB < 100ms):
```bash
k6 run test-streaming.js
```

**JWT authentication test** (validation < 1ms):
```bash
# First generate a JWT token
TOKEN=$(jwt encode --secret 'load-test-secret-key-12345' '{"sub":"testuser","exp":9999999999}')

# Run test with JWT
k6 run test-jwt.js -e JWT_TOKEN="$TOKEN"
```

## Test Scripts

### test-basic.js

**Purpose**: Baseline throughput and latency testing

**Scenarios**:
- Ramps from 0 to 100 concurrent users over 2 minutes
- Stays at 100 users for 2 minutes
- Measures request rate, latency percentiles

**Thresholds**:
- ✅ P95 latency < 100ms
- ✅ Request rate > 1,000 req/s
- ✅ Error rate < 1%

**Environment Variables**:
- `BASE_URL`: Proxy URL (default: http://localhost:8080)
- `TEST_PATH`: Path to test file (default: /test/sample.txt)

**Example**:
```bash
k6 run test-basic.js \
  -e BASE_URL=http://localhost:8080 \
  -e TEST_PATH=/test/1kb.bin
```

### test-concurrent.js

**Purpose**: Validate behavior under high concurrent load

**Scenarios**:
- 100 constant concurrent users for 2 minutes
- Ensures at least 1,000 requests completed

**Thresholds**:
- ✅ P95 latency < 200ms (slightly relaxed for concurrency)
- ✅ Total requests > 1,000
- ✅ Error rate < 1%

**Example**:
```bash
k6 run test-concurrent.js
```

### test-streaming.js

**Purpose**: Large file streaming performance (TTFB and throughput)

**Scenarios**:
- 10 concurrent users downloading large files
- Measures time to first byte (TTFB)
- Validates constant memory usage (monitor separately)

**Thresholds**:
- ✅ TTFB P95 < 100ms (critical for streaming)
- ✅ Complete download P95 < 5 seconds (for reasonable file sizes)
- ✅ Error rate < 1%

**Example**:
```bash
k6 run test-streaming.js -e LARGE_FILE=/test/100mb.bin
```

### test-jwt.js

**Purpose**: JWT authentication performance validation

**Scenarios**:
- Ramps from 0 to 50 concurrent users
- Tests both authenticated and unauthenticated requests
- Measures auth overhead

**Thresholds**:
- ✅ Overall P95 latency < 100ms
- ✅ JWT validation overhead < 1ms
- ✅ Error rate < 1%

**Example**:
```bash
TOKEN=$(jwt encode --secret 'load-test-secret-key-12345' '{"sub":"testuser","exp":9999999999}')
k6 run test-jwt.js \
  -e JWT_TOKEN="$TOKEN" \
  -e PROTECTED_PATH=/private/sample.txt
```

## Advanced Usage

### Custom Test Duration

```bash
k6 run test-basic.js --duration 5m --vus 200
```

### Export Metrics to JSON

```bash
k6 run test-basic.js --out json=results.json
```

### Run Multiple Tests in Sequence

```bash
for test in test-*.js; do
    echo "Running $test..."
    k6 run "$test"
done
```

### Cloud Execution (K6 Cloud)

```bash
# Sign up at https://app.k6.io/
k6 cloud test-basic.js
```

## Monitoring During Tests

### 1. Watch Proxy Logs

```bash
# In terminal running Yatagarasu
# Look for error messages, slow requests
```

### 2. Monitor System Resources

**CPU Usage**:
```bash
top -pid $(pgrep yatagarasu)
```

**Memory Usage**:
```bash
ps aux | grep yatagarasu
# Or use htop for visual monitoring
```

**Network Connections**:
```bash
lsof -i :8080 | wc -l  # Count active connections
```

### 3. MinIO Metrics

Open MinIO console: http://localhost:9001

Monitor:
- Request rate
- Bandwidth usage
- Error rate

## Interpreting Results

### Successful Test Output

```
✓ http_req_duration..............: avg=45ms  p(95)=89ms  ✅ Pass
✓ http_reqs.......................: 54321 1345 req/s     ✅ Pass
✓ errors..........................: 0.05% (27/54321)     ✅ Pass
```

**What this means**:
- Average latency: 45ms (good!)
- P95 latency: 89ms (under 100ms threshold ✅)
- Throughput: 1,345 req/s (exceeds 1,000 req/s target ✅)
- Error rate: 0.05% (under 1% threshold ✅)

### Failed Test Output

```
✗ http_req_duration..............: avg=250ms p(95)=450ms ❌ Fail
✓ http_reqs.......................: 5432 543 req/s      ❌ Fail
✗ errors..........................: 5.2% (283/5432)     ❌ Fail
```

**What this means**:
- Latency too high (P95 = 450ms, threshold 100ms) ❌
- Throughput too low (543 req/s, target 1,000 req/s) ❌
- Too many errors (5.2%, threshold 1%) ❌

**Possible causes**:
- Proxy bottleneck (check CPU usage)
- MinIO bottleneck (check MinIO metrics)
- Network issues (check `netstat`)
- Configuration issues (check Yatagarasu logs)

## Performance Baselines

From `plan.md`:

| Metric | Target | Validation |
|--------|--------|------------|
| Throughput | >1,000 req/s | test-basic.js |
| JWT Validation | <1ms P95 | test-jwt.js |
| Path Routing | <10μs P95 | Benchmark (not load test) |
| S3 Signature | <100μs P95 | Benchmark (not load test) |
| Streaming TTFB | <100ms P95 | test-streaming.js |
| Memory (baseline) | <50MB idle | Manual monitoring |
| Memory (streaming) | Constant | Manual monitoring |
| Concurrent Connections | 100+ | test-concurrent.js |
| Stability | 1 hour under load | Extended run |

## Troubleshooting

### "Connection refused" errors

**Problem**: K6 can't connect to proxy

**Solution**:
1. Verify proxy is running: `curl http://localhost:8080/test/sample.txt`
2. Check proxy logs for errors
3. Verify port 8080 is not in use: `lsof -i :8080`

### High error rate (>1%)

**Problem**: Proxy returning errors

**Solution**:
1. Check proxy logs for error messages
2. Verify MinIO is running: `curl http://localhost:9000/minio/health/live`
3. Check MinIO logs: `docker logs yatagarasu-minio`
4. Verify test files exist in buckets

### Low throughput (<1,000 req/s)

**Problem**: Not meeting performance targets

**Solution**:
1. Check CPU usage (proxy should not be maxed out)
2. Run in release mode: `cargo run --release`
3. Increase worker threads (if Pingora supports configuration)
4. Profile with: `cargo flamegraph`

### Memory leaks

**Problem**: Memory usage grows over time

**Solution**:
1. Monitor memory during test: `watch -n 1 'ps aux | grep yatagarasu'`
2. Run extended test: `k6 run test-basic.js --duration 30m`
3. Check for memory growth
4. If leak confirmed, profile with Valgrind or heaptrack

## Cleanup

Stop MinIO and remove test data:

```bash
docker stop yatagarasu-minio
docker rm yatagarasu-minio
rm -rf /tmp/yatagarasu-test
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Load Tests

on: [pull_request]

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup test environment
        run: ./scripts/load-testing/setup-test-env.sh

      - name: Start proxy
        run: |
          cargo run --release -- --config /tmp/yatagarasu-test/config.yaml &
          sleep 5

      - name: Install K6
        run: |
          sudo apt-key adv --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6

      - name: Run load tests
        run: |
          k6 run scripts/load-testing/test-basic.js
          k6 run scripts/load-testing/test-concurrent.js
```

## Additional Resources

- K6 Documentation: https://k6.io/docs/
- K6 Test Examples: https://k6.io/docs/examples/
- MinIO Documentation: https://min.io/docs/minio/linux/index.html
- Yatagarasu Performance Docs: `../../docs/PERFORMANCE.md` (if exists)
