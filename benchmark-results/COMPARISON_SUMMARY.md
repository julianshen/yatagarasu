# S3 Backend Comparison: MinIO vs RustFS

**Date:** 2025-12-27
**Test Duration:** 30s per file size (60s for 1GB)
**Virtual Users:** 10 (scaled down for larger files)
**Yatagarasu Cache:** Disabled (pure backend throughput test)

## Summary Results

### Duration (Lower = Better)

| File Size | MinIO (ms) | RustFS (ms) | Difference | Winner |
|-----------|------------|-------------|------------|--------|
| **10KB** | avg=7.3, p95=11 | avg=8.7, p95=12 | MinIO 16% faster | 🏆 MinIO |
| **1MB** | avg=13.0, p95=20 | avg=11.5, p95=16 | RustFS 12% faster | 🏆 RustFS |
| **10MB** | avg=28, p95=35 | avg=27, p95=32 | Similar (~3%) | Tie |
| **100MB** | avg=59, p95=69 | avg=57, p95=65 | Similar (~3%) | Tie |
| **1GB** | avg=760, p95=1943 | avg=687, p95=1785 | RustFS 10% faster | 🏆 RustFS |

### Throughput (Higher = Better)

| File Size | MinIO (Mbps) | RustFS (Mbps) | Winner |
|-----------|--------------|---------------|--------|
| **10KB** | ~10-16 Mbps | ~8-12 Mbps | 🏆 MinIO |
| **1MB** | ~780 Mbps | ~950 Mbps | 🏆 RustFS |
| **10MB** | ~3,100 Mbps | ~3,400 Mbps | 🏆 RustFS |
| **100MB** | ~14,800 Mbps | ~15,200 Mbps | 🏆 RustFS |
| **1GB** | ~14,400 Mbps | ~15,600 Mbps | 🏆 RustFS |

### Time to First Byte (TTFB) - Lower = Better

| File Size | MinIO (ms) | RustFS (ms) | Winner |
|-----------|------------|-------------|--------|
| **10KB** | avg=7.1, p95=10.3 | avg=8.3, p95=11.2 | 🏆 MinIO |
| **1MB** | avg=10.5, p95=17.4 | avg=9.1, p95=14.2 | 🏆 RustFS |
| **10MB** | avg=15.3, p95=20.6 | avg=14.8, p95=19.2 | 🏆 RustFS |
| **100MB** | avg=2.2, p95=4.4 | avg=2.0, p95=3.8 | 🏆 RustFS |
| **1GB** | avg=3.8, p95=15.7 | avg=3.2, p95=12.4 | 🏆 RustFS |

### Requests Completed

| File Size | MinIO | RustFS | Winner |
|-----------|-------|--------|--------|
| **10KB** | 2,771 | 2,790 | Tie |
| **1MB** | 2,646 | 2,685 | Tie |
| **10MB** | 1,167 | 1,220 | 🏆 RustFS |
| **100MB** | 563 | 585 | 🏆 RustFS |
| **1GB** | 158 | 172 | 🏆 RustFS |

## Key Findings

### 1. Small Files (10KB) - MinIO Wins
- MinIO has slightly better latency for very small objects
- RustFS's claimed 2.3x advantage for 4KB objects **not observed** at 10KB
- Both perform similarly with negligible real-world difference

### 2. Medium Files (1MB) - RustFS Wins
- RustFS shows ~12% better average latency
- Higher throughput due to better streaming efficiency
- Clear advantage for typical web asset sizes

### 3. Large Files (10MB-1GB) - RustFS Wins
- RustFS consistently outperforms for large file streaming
- ~8-10% better throughput at scale
- Better TTFB indicates faster connection handling

### 4. Error Rate
- **MinIO:** 0% errors for 10KB-100MB, some timeouts on 1GB
- **RustFS:** 0% errors across all file sizes

## Recommendations

| Use Case | Recommendation |
|----------|----------------|
| **CDN for small assets** (icons, thumbnails) | Either backend works well |
| **Image/document serving** (1-10MB) | 🏆 RustFS |
| **Video streaming** (100MB+) | 🏆 RustFS |
| **Mixed workload** | 🏆 RustFS (better overall throughput) |
| **Production stability** | MinIO (mature, battle-tested) |

## Test Environment

- **Host:** macOS Darwin (Docker Desktop / OrbStack)
- **Backend Resources:** 2 CPU, 2GB RAM per container
- **Proxy:** Yatagarasu (no caching)
- **Network:** Docker bridge network (local)

## Notes

1. RustFS is still in **alpha** (v1.0.0-alpha.76) - use caution in production
2. MinIO is **production-ready** with years of battle-testing
3. These benchmarks are local - real-world results may vary with network latency
4. Both backends are S3-compatible and work seamlessly with Yatagarasu

## Raw Data Files

- `minio_20251227_*.json` - MinIO benchmark data
- `rustfs_20251227_*.json` - RustFS benchmark data
