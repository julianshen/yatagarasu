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

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [k6 Documentation](https://k6.io/docs/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
