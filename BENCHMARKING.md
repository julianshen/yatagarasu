# Disk Cache Benchmarking Guide

This guide explains how to run performance benchmarks for the Yatagarasu disk cache, comparing TokioFsBackend (macOS/cross-platform) with UringBackend (Linux io-uring).

## Quick Start

### Run All Benchmarks (Recommended)

```bash
# Compare macOS (TokioFsBackend) vs Linux (UringBackend)
./scripts/bench-compare.sh
```

This script will:
1. Run benchmarks on your local macOS machine (TokioFsBackend)
2. Build a Linux Docker container
3. Run benchmarks in the container (UringBackend)
4. Save baseline results for comparison

### Run Individual Benchmarks

#### macOS / Current Platform

```bash
# Run all disk cache benchmarks
cargo bench --bench disk_cache

# Run specific benchmark
cargo bench --bench disk_cache -- small_file_read

# Save as baseline for comparison
cargo bench --bench disk_cache -- --save-baseline my-baseline
```

#### Linux (Docker)

```bash
# Run benchmarks in Linux container
docker-compose -f docker-compose.bench.yml run --rm benchmarks

# Or using docker-compose shorthand
docker-compose -f docker-compose.bench.yml up benchmarks
```

## Benchmark Results

### Current Performance (Phase 28.11)

#### macOS with TokioFsBackend

| Operation | Mean Latency | P95 Latency | Target | Status |
|-----------|--------------|-------------|--------|--------|
| 4KB read | 17.7 µs | <50 µs | <10ms | ✅ Pass |
| 4KB write | 428 µs | <1ms | <10ms | ✅ Pass |
| 10MB read | 558 µs | <2ms | <10ms | ✅ Pass |
| 10MB write | 2.43 ms | <5ms | <10ms | ✅ Pass |
| LRU eviction | 390 µs | <1ms | <10ms | ✅ Pass |

**Summary:** All operations well below P95 <10ms target ✅

#### Linux with UringBackend

*Results TBD - Requires running `./scripts/bench-compare.sh` or Docker benchmarks*

**Expected targets:**
- Small files (4KB): 2-3x faster than TokioFsBackend
- Large files (10MB): 20-40% faster than TokioFsBackend
- P95 latency: <5ms (vs <10ms for TokioFsBackend)

## Viewing Results

### HTML Reports

Criterion generates beautiful HTML reports with charts:

```bash
# Open benchmark report in browser
open target/criterion/report/index.html
```

### Compare Baselines

```bash
# Compare current run against saved baseline
cargo bench --bench disk_cache -- --baseline macos-tokio

# List all saved baselines
ls target/criterion/*/
```

## Understanding the Benchmarks

### Small File Benchmarks (4KB)

- **4kb_write**: Write 4KB entries to cache (tests atomic file writes)
- **4kb_read**: Read 4KB entries from cache (tests file I/O + index lookup)

**What they test:** Fast path for small assets (icons, thumbnails, API responses)

### Large File Benchmarks (10MB)

- **10mb_write**: Write 10MB entries to cache
- **10mb_read**: Read 10MB entries from cache

**What they test:** Large file handling (images, videos, documents)

### Mixed Size Benchmarks

Tests cache performance across file sizes: 1KB, 4KB, 16KB, 64KB, 256KB, 1MB

**What they test:** Real-world workload with varied file sizes

### Eviction Benchmark

Tests LRU eviction performance when cache is full (10KB cache, 4KB entries)

**What they test:** Cache pressure scenarios with frequent evictions

## Docker Configuration

### System Requirements

- Docker Desktop or Docker Engine
- 4GB RAM allocated to Docker
- 2 CPU cores allocated to Docker

### Dockerfile Details

**Base image:** `rust:1.70-bookworm` (Debian 12 with Rust 1.70)

**Why Bookworm?**
- Linux kernel 6.1+ (io-uring support)
- Modern libc with io-uring system calls
- Stable Debian base

### Performance Tips

1. **Use tmpfs for benchmarks** (already configured in docker-compose.bench.yml)
   - Ensures consistent I/O performance
   - Avoids Docker volume overhead

2. **Allocate sufficient resources**
   - Minimum: 2 CPU cores, 4GB RAM
   - Recommended: 4 CPU cores, 8GB RAM

3. **Run benchmarks multiple times**
   - First run: "warm up" (build cache, load binaries)
   - Second run: Accurate results
   - Use `--baseline` to save results

## Troubleshooting

### io-uring Not Working in Docker

**Symptoms:** UringBackend falls back to TokioFsBackend

**Causes:**
- Docker Desktop on macOS/Windows uses a VM with older kernel
- Host kernel doesn't support io-uring

**Solutions:**
1. Update Docker Desktop to latest version
2. Check kernel version in container: `docker run --rm rust:1.70-bookworm uname -r`
3. Use Linux host (cloud VM, CI/CD) for guaranteed io-uring support

### Benchmarks Take Too Long

**Solution:** Reduce sample size

```rust
// In benches/disk_cache.rs
group.sample_size(10); // Default is 100
```

### Docker Build Fails

**Solution:** Clear Docker cache

```bash
docker-compose -f docker-compose.bench.yml build --no-cache
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: 1.70

      - name: Run benchmarks (Linux/UringBackend)
        run: cargo bench --bench disk_cache -- --save-baseline linux-ci

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/
```

## Advanced Usage

### Custom Benchmark Configuration

Edit `benches/disk_cache.rs` to:
- Change file sizes
- Adjust sample sizes
- Add new benchmark scenarios

### Comparing Multiple Baselines

```bash
# Save different configurations
cargo bench -- --save-baseline tokio-v1
docker-compose -f docker-compose.bench.yml run --rm benchmarks \
    cargo bench -- --save-baseline uring-v1

# Compare later
cargo bench -- --baseline tokio-v1
cargo bench -- --baseline uring-v1
```

### Profiling

For detailed performance analysis:

```bash
# Install flamegraph
cargo install flamegraph

# Profile benchmarks
sudo cargo flamegraph --bench disk_cache

# Open flamegraph.svg in browser
```

## Performance Targets Summary

| Metric | Target | Actual (macOS) | Actual (Linux) |
|--------|--------|----------------|----------------|
| P95 Latency (small) | <10ms | <50µs ✅ | TBD |
| P95 Latency (large) | <10ms | <3ms ✅ | TBD |
| Small file speedup | 2-3x | Baseline | TBD |
| Large file speedup | 1.2-1.4x | Baseline | TBD |

---

**Next Steps:**
1. Run `./scripts/bench-compare.sh` to get Linux results
2. Compare TokioFsBackend vs UringBackend performance
3. Document findings in Phase 28.11 completion
