# Benchmarking Guide

This guide explains how to run performance benchmarks for Yatagarasu.

## Overview

Yatagarasu uses two complementary benchmarking approaches:

1. **Criterion Benchmarks** - Rust micro-benchmarks for individual components
2. **k6 Load Tests** - End-to-end HTTP benchmarks for proxy pipeline

## Quick Start

```bash
# Run all Criterion benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench jwt_validation
cargo bench --bench s3_signature
cargo bench --bench routing
cargo bench --bench request_processing
cargo bench --bench cache_comparison
cargo bench --bench disk_cache

# Run k6 proxy pipeline tests (requires running proxy)
./target/release/yatagarasu --config config.k6test.yaml &
k6 run k6/proxy-pipeline.js
```

## Criterion Benchmarks

### Available Benchmark Suites

| Suite | File | Description |
|-------|------|-------------|
| JWT Validation | `benches/jwt_validation.rs` | Token parsing, signature verification |
| S3 Signature | `benches/s3_signature.rs` | AWS SigV4 signing performance |
| Routing | `benches/routing.rs` | Path-to-bucket routing |
| Request Processing | `benches/request_processing.rs` | Range header parsing |
| Cache Comparison | `benches/cache_comparison.rs` | Memory vs disk vs Redis cache |
| Disk Cache | `benches/disk_cache.rs` | Disk cache read/write operations |

### Running Benchmarks

```bash
# Full benchmark run (takes 5-10 minutes)
cargo bench

# Quick benchmark (fewer samples)
cargo bench -- --quick

# Specific benchmark function
cargo bench --bench jwt_validation -- "hs256"

# Save baseline for comparison
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main

# Generate HTML report only
cargo bench -- --noplot
```

### Reading Results

Criterion outputs results to `target/criterion/`. Each benchmark includes:

```
benchmark_name
├── report/
│   └── index.html    # Interactive HTML report
├── new/
│   └── estimates.json  # Current run statistics
└── change/
    └── estimates.json  # Comparison to baseline
```

Key metrics to watch:
- **Mean**: Average execution time
- **Std Dev**: Consistency of results
- **P95/P99**: Tail latency percentiles
- **Regression/Improvement**: Change from baseline

### Performance Targets

| Benchmark | Target | Reason |
|-----------|--------|--------|
| JWT HS256 validation | <50μs | Auth shouldn't block requests |
| S3 SigV4 signing | <100μs | Per-request overhead |
| Path routing | <10μs | Called on every request |
| Range header parsing | <1μs | Fast header processing |
| Memory cache hit | <10μs | Near-instant responses |
| Disk cache hit | <1ms | SSD latency |
| Redis cache hit | <5ms | Network round-trip |

## k6 Load Tests

### Available Scenarios

| Scenario | File | Description |
|----------|------|-------------|
| Proxy Pipeline | `k6/proxy-pipeline.js` | Health, cache hit/miss, range, streaming |
| Memory Pressure | `k6/memory-pressure.js` | Cache eviction under load |
| Disk Cache Stress | `k6/disk-cache-stress.js` | Disk I/O stress tests |
| Redis Stress | `k6/redis-stress.js` | Redis connection pool tests |

### Running k6 Tests

```bash
# Start the proxy first
./target/release/yatagarasu --config config.k6test.yaml &

# Run all scenarios
k6 run k6/proxy-pipeline.js

# Run specific scenario
k6 run -e SCENARIO=health k6/proxy-pipeline.js
k6 run -e SCENARIO=cache_hit k6/proxy-pipeline.js
k6 run -e SCENARIO=range k6/proxy-pipeline.js

# Custom duration and VUs
k6 run -e SCENARIO=health --duration=60s --vus=100 k6/proxy-pipeline.js

# Output JSON for analysis
k6 run --out json=results.json k6/proxy-pipeline.js
```

### k6 Performance Targets

| Metric | Target | Threshold |
|--------|--------|-----------|
| Health check P99 | <100μs | p(99)<100 |
| Cache hit P99 | <10ms | p(99)<10 |
| Error rate | <1% | rate<0.01 |
| Streaming TTFB | <500ms | - |

## CI Integration

Benchmarks run automatically on:
- Every push to `main`/`master`
- Every pull request (with comparison to base)

### What Happens in CI

1. **Build**: Compile benchmarks in release mode
2. **Run**: Execute all benchmark suites
3. **Compare**: Check for >10% regression
4. **Report**: Post results as PR comment
5. **Store**: Save results as artifacts (30 days)

### Skipping Benchmark Checks

If a PR intentionally introduces a performance regression:

```bash
git commit -m "Refactor: Simplify auth flow [benchmark-skip]"
```

The `[benchmark-skip]` marker prevents the benchmark check from failing.

## Local Development Tips

### Quick Iteration

For fast feedback during development:

```bash
# Run only the benchmark you're working on
cargo bench --bench jwt_validation -- "specific_test" --quick

# Skip HTML report generation
cargo bench -- --noplot
```

### Comparing Changes

Before committing performance-sensitive code:

```bash
# Save baseline on current commit
cargo bench -- --save-baseline before

# Make your changes, then compare
cargo bench -- --baseline before
```

### Profiling

For deeper analysis:

```bash
# Generate flamegraph (requires cargo-flamegraph)
cargo flamegraph --bench jwt_validation -- --bench

# Use perf (Linux)
perf record cargo bench --bench jwt_validation
perf report
```

## Interpreting Results

### Criterion Output

```
jwt_validation/hs256   time:   [45.234 μs 45.891 μs 46.612 μs]
                       change: [-2.1234% +0.1234% +2.4567%] (p = 0.12 > 0.05)
                       No change in performance detected.
```

- **time**: [lower bound, estimate, upper bound]
- **change**: Percentage change from baseline
- **p-value**: Statistical significance (p < 0.05 = significant)

### When to Investigate

- Regression > 10%: Likely a real performance issue
- Regression 5-10%: Review changes, may be acceptable
- Regression < 5%: Usually noise, ignore unless persistent

### Common False Positives

- CPU throttling (laptop on battery)
- Background processes
- First run after code change (cold cache)
- Different hardware (CI vs local)

## Adding New Benchmarks

1. Create benchmark file in `benches/`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");

    group.bench_function("operation", |b| {
        b.iter(|| {
            // Code to benchmark
            black_box(my_function())
        })
    });

    group.finish();
}

criterion_group!(benches, bench_my_feature);
criterion_main!(benches);
```

2. Add to `Cargo.toml`:

```toml
[[bench]]
name = "my_feature"
harness = false
```

3. Run and verify:

```bash
cargo bench --bench my_feature
```

## Troubleshooting

### Benchmarks Timeout

Increase sample size timeout:
```rust
group.measurement_time(Duration::from_secs(30));
```

### Results Too Variable

Increase sample count:
```rust
group.sample_size(200);
```

### Can't Compare to Baseline

Baselines are stored in `target/criterion/`. Clean builds remove them:
```bash
# Keep baselines across clean builds
cp -r target/criterion ~/.criterion-backup
cargo clean
mv ~/.criterion-backup target/criterion
```

## Benchmark Interpretation Guide

### Understanding Criterion Output

When you run `cargo bench`, Criterion produces output like this:

```
jwt_validation/hs256   time:   [1.72 µs 1.78 µs 1.84 µs]
                       change: [-2.12% +0.12% +2.45%] (p = 0.12 > 0.05)
                       No change in performance detected.
```

**Breaking it down:**

| Component | Meaning |
|-----------|---------|
| `[1.72 µs 1.78 µs 1.84 µs]` | [lower bound, **estimate**, upper bound] of execution time |
| `change: [-2.12% +0.12% +2.45%]` | Performance change from baseline [lower, estimate, upper] |
| `p = 0.12 > 0.05` | Statistical significance (p < 0.05 = significant change) |
| `No change in performance detected` | Criterion's conclusion |

### What the Numbers Mean

**Time Measurements:**
- **Mean**: Average execution time across all samples
- **Lower/Upper bounds**: 95% confidence interval
- **Tighter bounds = more consistent performance**

**Performance Change:**
- **Negative %**: Improvement (faster)
- **Positive %**: Regression (slower)
- **p-value < 0.05**: Statistically significant change
- **p-value > 0.05**: Could be measurement noise

### Regression Severity Guide

| Change | Severity | Action |
|--------|----------|--------|
| < 5% | Noise | Usually ignore |
| 5-10% | Minor | Review if persistent across runs |
| 10-20% | Moderate | Investigate the cause |
| > 20% | Severe | Fix before merging |
| > 50% | Critical | Likely a bug or algorithmic change |

### Common Causes of Performance Changes

**False Positives (fake regressions):**
- CPU frequency scaling (laptop on battery)
- Background processes consuming CPU
- First run after code change (cold instruction cache)
- Thermal throttling on warm machine
- Different hardware (local vs CI)

**Real Regressions:**
- Algorithm complexity change (O(n) → O(n²))
- Added synchronization (locks, atomics)
- Increased memory allocations
- Cache-unfriendly data access patterns
- Unintended function calls in hot paths

### Interpreting k6 Results

k6 output shows different metrics:

```
http_req_duration..........: avg=1.23ms min=0.5ms med=1.1ms max=15ms p(90)=2ms p(95)=3ms
```

| Metric | Meaning | Target |
|--------|---------|--------|
| `avg` | Average response time | Use for baseline |
| `med` | Median (50th percentile) | Better than avg for skewed data |
| `p(90)` | 90% of requests below this | Good for capacity planning |
| `p(95)` | 95th percentile | Common SLA metric |
| `p(99)` | 99th percentile (tail latency) | Important for user experience |
| `max` | Worst case | Check for outliers |

### Dashboard Metrics

The benchmark dashboard at `https://<owner>.github.io/<repo>/benchmarks/` shows:

1. **JWT Validation**: Token parsing and signature verification
   - Target: <5µs for HS256
   - Impact: Auth overhead on every protected request

2. **S3 Signature**: AWS SigV4 signing performance
   - Target: <10µs
   - Impact: Per-request overhead to S3

3. **Routing**: Path-to-bucket matching
   - Target: <1µs
   - Impact: Very hot path, called on every request

4. **Cache Operations**: Memory/disk cache read/write
   - Memory target: <100µs
   - Disk target: <10ms
   - Impact: Determines cache hit latency

### When to Investigate

**Always investigate when:**
- Change > 10% and p < 0.05 (statistically significant)
- Multiple benchmarks regress together
- Regression appears on multiple consecutive commits
- P99 latency increases significantly

**Can usually ignore when:**
- Change < 5%
- p-value > 0.05 (not statistically significant)
- Only one run shows regression
- Regression disappears on re-run

### Debugging Slow Benchmarks

```bash
# Generate flamegraph for specific benchmark
cargo flamegraph --bench jwt_validation -- --bench

# Profile with perf (Linux)
perf record cargo bench --bench jwt_validation
perf report

# Check allocations with DHAT (requires nightly)
cargo +nightly bench --bench jwt_validation -- --profile-time=10
```

## Benchmark Dashboard

After each push to main, benchmark results are published to GitHub Pages:

**URL**: `https://<owner>.github.io/<repo>/benchmarks/`

The dashboard shows:
- Historical performance trends over last 100 commits
- Per-component charts (JWT, S3, Routing, Cache)
- Latest commit information
- Interactive hover for exact values

### Setting Up GitHub Pages

1. Go to repository Settings > Pages
2. Set Source to "Deploy from a branch"
3. Select `gh-pages` branch, `/ (root)` folder
4. Save

The benchmark workflow automatically creates and updates the `gh-pages` branch.

## Regression Alerts

When a benchmark regresses >10% on the main branch:

1. **GitHub Issue** is automatically created with:
   - Commit that caused the regression
   - Link to workflow run
   - Instructions for investigation

2. **PR Comments** show benchmark results for review before merge

3. **Workflow Annotations** highlight regressions in the Actions UI

To skip regression check for expected changes:
```bash
git commit -m "Refactor: Add logging [benchmark-skip]"
```

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [k6 Documentation](https://k6.io/docs/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
