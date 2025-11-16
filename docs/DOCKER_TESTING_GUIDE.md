# Docker-Based Testing Guide for Phase 28

**Purpose**: Test Linux io-uring backend on any platform using Docker
**Use Cases**: macOS/Windows development, CI/CD, kernel version testing

---

## Quick Start

### Run All Tests on Linux
```bash
# Build and run tests in Docker
docker-compose -f docker/docker-compose.test.yml run test-linux

# Run specific test
docker-compose -f docker/docker-compose.test.yml run test-linux \
  cargo test --test disk_cache_uring --features io-uring
```

### Run Benchmarks on Linux
```bash
docker-compose -f docker/docker-compose.test.yml run bench-linux
```

### Build for Linux (with io-uring)
```bash
docker-compose -f docker/docker-compose.test.yml run build-linux
```

---

## Testing Strategy

### Local Development (macOS/Windows)

**Scenario**: Developer on macOS wants to test io-uring backend

```bash
# 1. Develop using tokio::fs backend (local)
cargo test

# 2. Test io-uring backend (Docker)
docker-compose -f docker/docker-compose.test.yml run test-linux

# 3. Benchmark comparison
docker-compose -f docker/docker-compose.test.yml run bench-linux
```

**Workflow**:
- Write code on macOS (uses tokio::fs)
- Run quick tests locally
- Run full test suite in Docker for io-uring validation
- Commit when both pass

---

## CI/CD Integration

### GitHub Actions Example

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all
      - run: cargo clippy -- -D warnings

  test-linux-tokio:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all

  test-linux-uring:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - name: Install io-uring dependencies
        run: sudo apt-get update && sudo apt-get install -y liburing-dev
      - name: Run tests with io-uring
        run: cargo test --all --features io-uring
      - name: Run benchmarks
        run: cargo bench disk_cache

  test-linux-docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build Docker image
        run: docker build -t yatagarasu-test -f docker/Dockerfile.test-linux .
      - name: Run tests in Docker
        run: docker-compose -f docker/docker-compose.test.yml run test-linux
```

---

## Kernel Version Testing

### Testing Different Kernels

**Test io-uring availability on different kernel versions**:

```dockerfile
# Dockerfile.test-linux-5.10 (minimum io-uring version)
FROM ubuntu:20.04  # Kernel 5.4 (no io-uring)
# Tests should fall back to tokio::fs

# Dockerfile.test-linux-5.15 (stable io-uring)
FROM ubuntu:22.04  # Kernel 5.15
# Tests should use io-uring

# Dockerfile.test-linux-6.1 (latest)
FROM ubuntu:23.04  # Kernel 6.1
# Tests should use io-uring with all features
```

### Fallback Testing

**Verify graceful fallback when io-uring unavailable**:

```bash
# Build with io-uring support but run on old kernel
docker run --rm \
  -v $(pwd):/workspace \
  ubuntu:18.04 \  # Old kernel, no io-uring
  /workspace/target/release/yatagarasu

# Should log: "io-uring not available, using tokio::fs fallback"
```

---

## Performance Testing in Docker

### Benchmark Setup

**Create isolated tmpfs for consistent benchmarks**:

```bash
# Run benchmark with tmpfs mount
docker run --rm \
  -v $(pwd):/workspace \
  --mount type=tmpfs,destination=/tmp/cache,tmpfs-size=1g \
  yatagarasu-test \
  cargo bench disk_cache
```

### Benchmark Comparison Script

```bash
#!/bin/bash
# scripts/bench-compare.sh

echo "Running tokio::fs benchmark..."
cargo bench disk_cache > bench-tokio.txt

echo "Running io-uring benchmark (Docker)..."
docker-compose -f docker/docker-compose.test.yml run bench-linux > bench-uring.txt

echo "Comparison:"
echo "tokio::fs:"
grep "time:" bench-tokio.txt | head -5
echo ""
echo "io-uring:"
grep "time:" bench-uring.txt | head -5
```

---

## Test Matrix

### Phase 28 Test Coverage

| Platform | Backend | Test Environment | Status |
|----------|---------|------------------|--------|
| Linux (native) | io-uring | Linux dev machine | ✅ Primary |
| Linux (Docker) | io-uring | macOS/Windows dev | ✅ Development |
| Linux (native) | tokio::fs | Linux dev machine | ✅ Fallback |
| macOS | tokio::fs | macOS dev machine | ✅ Primary |
| Windows | tokio::fs | Windows dev machine | ✅ Primary |
| CI (GitHub) | io-uring | ubuntu-latest | ✅ Automation |
| CI (GitHub) | tokio::fs | macos-latest | ✅ Automation |

---

## Development Workflow

### Recommended Workflow for Phase 28

```bash
# 1. Start working on Phase 28.1 (abstractions)
# Work locally on macOS/Windows - uses tokio::fs

cargo test --test disk_cache_abstractions

# 2. Implement tokio::fs backend (28.5)
# Test locally

cargo test --test disk_cache_tokio

# 3. Implement io-uring backend (28.6)
# Test in Docker

docker-compose -f docker/docker-compose.test.yml run test-linux \
  cargo test --test disk_cache_uring --features io-uring

# 4. Cross-platform validation (28.10)
# Test on all platforms

cargo test                                # Local (macOS)
docker-compose -f docker/docker-compose.test.yml run test-linux  # Linux

# 5. Performance validation (28.11)
# Benchmark both backends

cargo bench disk_cache                    # Local baseline
docker-compose -f docker/docker-compose.test.yml run bench-linux # io-uring
```

---

## Troubleshooting

### "io-uring not available" in Docker

**Problem**: Docker container can't use io-uring even on Linux

**Cause**: Docker Desktop on macOS/Windows uses VM with older kernel

**Solution**: Run on native Linux or use GitHub Actions

```bash
# Check kernel version in container
docker run --rm ubuntu:22.04 uname -r

# If < 5.10, io-uring won't work
```

### Slow Docker builds

**Problem**: Rebuilding Rust dependencies on every run

**Solution**: Use volume caching (already in docker-compose.test.yml)

```yaml
volumes:
  - cargo-cache:/root/.cargo/registry  # Cache dependencies
  - target-cache:/workspace/target      # Cache build artifacts
```

### Permission issues with volumes

**Problem**: Files created by Docker owned by root

**Solution**: Use user mapping or fix permissions after

```bash
# Option 1: Run as current user
docker run --rm --user $(id -u):$(id -g) ...

# Option 2: Fix ownership after
sudo chown -R $(whoami) target/
```

---

## Advanced: Multi-Architecture Testing

### Test on ARM64 (Apple Silicon)

```bash
# Build for ARM64 Linux
docker buildx build --platform linux/arm64 \
  -f docker/Dockerfile.test-linux \
  -t yatagarasu-test-arm64 .

# Run tests on ARM64
docker run --rm --platform linux/arm64 \
  yatagarasu-test-arm64 \
  cargo test --features io-uring
```

### Test on x86_64 (Intel/AMD)

```bash
# Build for x86_64 Linux
docker buildx build --platform linux/amd64 \
  -f docker/Dockerfile.test-linux \
  -t yatagarasu-test-amd64 .

# Run tests on x86_64
docker run --rm --platform linux/amd64 \
  yatagarasu-test-amd64 \
  cargo test --features io-uring
```

---

## Makefile Integration

### Add Docker commands to Makefile

```makefile
# Makefile
.PHONY: test test-linux test-all bench bench-linux

# Local tests (current platform)
test:
	cargo test

# Linux tests (Docker)
test-linux:
	docker-compose -f docker/docker-compose.test.yml run test-linux

# All platforms
test-all: test test-linux

# Local benchmarks
bench:
	cargo bench disk_cache

# Linux benchmarks (Docker)
bench-linux:
	docker-compose -f docker/docker-compose.test.yml run bench-linux

# Build for Linux with io-uring
build-linux:
	docker-compose -f docker/docker-compose.test.yml run build-linux

# Clean Docker volumes
clean-docker:
	docker-compose -f docker/docker-compose.test.yml down -v
```

**Usage**:
```bash
make test          # Test locally (macOS/Windows)
make test-linux    # Test in Docker (Linux io-uring)
make test-all      # Test everywhere
make bench-linux   # Benchmark io-uring vs tokio::fs
```

---

## Summary

### Benefits of Docker Testing

✅ **Cross-platform development**: Test Linux code on macOS/Windows
✅ **Consistent environment**: Same kernel version across team
✅ **CI/CD integration**: Automated testing on every commit
✅ **Kernel isolation**: Test different kernel versions
✅ **Performance validation**: Isolated benchmarking environment

### When to Use Docker

**Use Docker when**:
- Developing on macOS/Windows (no io-uring)
- Testing io-uring backend before deployment
- Running CI/CD pipelines
- Benchmarking io-uring performance
- Testing on specific kernel versions

**Don't need Docker when**:
- Developing on Linux 5.10+
- Testing tokio::fs backend only
- Quick local tests during development

---

**Next Steps**:
1. Set up Docker environment: `docker-compose -f docker/docker-compose.test.yml build`
2. Run initial tests: `docker-compose -f docker/docker-compose.test.yml run test-linux`
3. Add to CI/CD pipeline
4. Document results in performance report

---

**Ready to start Phase 28 with Docker-enabled testing!**
