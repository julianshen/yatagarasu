# Compression Performance Guide

## Compression Ratios by Content Type

### Text Content (Highly Compressible)
| Content Type | Gzip | Brotli | Deflate |
|---|---|---|---|
| HTML | 65-75% | 75-85% | 55-65% |
| CSS | 70-80% | 80-90% | 60-70% |
| JavaScript | 65-75% | 75-85% | 55-65% |
| JSON | 60-70% | 70-80% | 50-60% |
| XML | 65-75% | 75-85% | 55-65% |
| Plain Text | 70-80% | 80-90% | 60-70% |

### Semi-Compressible Content
| Content Type | Gzip | Brotli | Deflate |
|---|---|---|---|
| SVG | 60-70% | 70-80% | 50-60% |
| WebAssembly | 50-60% | 60-70% | 40-50% |

### Already-Compressed Content (Not Recommended)
| Content Type | Compression Ratio |
|---|---|
| PNG | 0-5% (skip compression) |
| JPEG | 0-2% (skip compression) |
| WebP | 0-2% (skip compression) |
| MP4 | 0-1% (skip compression) |
| MP3 | 0-1% (skip compression) |

## Compression Speed by Algorithm

### Speed Comparison (1MB Text, Level 6)
| Algorithm | Time | Throughput |
|---|---|---|
| Deflate | ~30ms | 33 MB/s |
| Gzip | ~50ms | 20 MB/s |
| Brotli | ~200ms | 5 MB/s |

### Speed by Compression Level
| Level | Gzip | Brotli | Deflate |
|---|---|---|---|
| 1 | 10ms | 50ms | 8ms |
| 3 | 20ms | 100ms | 15ms |
| 6 | 50ms | 200ms | 30ms |
| 9 | 150ms | 500ms | 100ms |
| 11 | 300ms | 1000ms | 200ms |

## Bandwidth Savings

### Example: 1MB HTML File
- **Original Size**: 1,000 KB
- **Gzip (Level 6)**: 250 KB (75% savings)
- **Brotli (Level 6)**: 150 KB (85% savings)
- **Deflate (Level 6)**: 350 KB (65% savings)

### Example: 10MB JavaScript Bundle
- **Original Size**: 10,000 KB
- **Gzip (Level 6)**: 2,500 KB (75% savings)
- **Brotli (Level 6)**: 1,500 KB (85% savings)
- **Deflate (Level 6)**: 3,500 KB (65% savings)

## CPU Impact

### CPU Usage by Algorithm (1MB, Level 6)
| Algorithm | CPU Time | CPU Cores |
|---|---|---|
| Deflate | 30ms | 1 core |
| Gzip | 50ms | 1 core |
| Brotli | 200ms | 1 core |

### Throughput Impact
- **Gzip Level 6**: ~20 MB/s per core
- **Brotli Level 6**: ~5 MB/s per core
- **Deflate Level 6**: ~33 MB/s per core

## Memory Usage

### Per-Connection Memory
- **Streaming Mode**: ~64KB (constant, regardless of file size)
- **Buffering Mode**: Not used (streaming only)

### Total Memory for 1000 Concurrent Connections
- **Streaming**: ~64MB
- **Buffering**: Not applicable

## Optimization Strategies

### Strategy 1: Maximize Compression Ratio
```yaml
compression:
  default_algorithm: "brotli"
  compression_level: 9
  min_response_size_bytes: 256
```
- **Best For**: Static content, CDN, bandwidth-critical
- **Trade-off**: Higher CPU usage
- **Bandwidth Savings**: 85% for text

### Strategy 2: Balance Speed and Compression
```yaml
compression:
  default_algorithm: "gzip"
  compression_level: 6
  min_response_size_bytes: 1024
```
- **Best For**: General purpose, most scenarios
- **Trade-off**: Moderate CPU usage
- **Bandwidth Savings**: 75% for text

### Strategy 3: Minimize CPU Usage
```yaml
compression:
  default_algorithm: "gzip"
  compression_level: 3
  min_response_size_bytes: 2048
```
- **Best For**: High-traffic, CPU-constrained
- **Trade-off**: Lower compression ratio
- **Bandwidth Savings**: 60% for text

### Strategy 4: Selective Compression
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 6

buckets:
  - name: "static"
    compression:
      default_algorithm: "brotli"
      compression_level: 9
  
  - name: "api"
    compression:
      default_algorithm: "gzip"
      compression_level: 3
```
- **Best For**: Mixed workloads
- **Trade-off**: Complex configuration
- **Bandwidth Savings**: Optimized per bucket

## Benchmarking

### Measure Compression Ratio
```bash
# Original size
ls -lh file.txt

# Compressed size
gzip -c file.txt | wc -c

# Ratio
echo "scale=2; compressed / original * 100" | bc
```

### Measure Compression Speed
```bash
# Time compression
time gzip -c file.txt > /dev/null

# Calculate throughput
echo "scale=2; file_size / time_seconds" | bc
```

### Monitor Proxy Metrics
```bash
# Check compression metrics
curl http://localhost:9090/metrics | grep compression
```

## Tuning Recommendations

### For Static Content (CDN)
- Algorithm: Brotli
- Level: 9
- Min Size: 256 bytes
- Max Size: 100MB
- Expected Savings: 85% bandwidth

### For API Responses
- Algorithm: Gzip
- Level: 6
- Min Size: 1KB
- Max Size: 10MB
- Expected Savings: 75% bandwidth

### For Real-Time Content
- Algorithm: Gzip
- Level: 3
- Min Size: 2KB
- Max Size: 5MB
- Expected Savings: 60% bandwidth

### For High-Traffic Scenarios
- Algorithm: Gzip
- Level: 2
- Min Size: 4KB
- Max Size: 10MB
- Expected Savings: 50% bandwidth

## Monitoring

### Key Metrics to Track
1. **Compression Ratio**: Should be 50-85% for text
2. **Bytes Saved**: Total bandwidth reduction
3. **Compression Time**: Should be <100ms for most responses
4. **Cache Hit Rate**: Should improve with compression
5. **CPU Usage**: Should be <10% for typical workloads

### Alert Thresholds
- Compression Ratio < 40%: Investigate content type
- Compression Time > 500ms: Reduce compression level
- CPU Usage > 50%: Reduce compression level or disable
- Cache Hit Rate < 50%: Check Vary header handling

