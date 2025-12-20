# Yatagarasu Compression Feature Plan (Phase 40)

**Version**: 1.0  
**Status**: Planning  
**Target Release**: v1.5.0  
**Last Updated**: 2025-12-20

## Overview

Phase 40 adds comprehensive request/response compression support to Yatagarasu, enabling:
- **Response compression** (gzip, brotli, deflate) with Accept-Encoding negotiation
- **Request decompression** for compressed request bodies
- **Smart caching** of compressed variants with Vary header support
- **Configurable compression** with per-bucket settings and tuning options
- **Observability** with compression metrics and tracing

## Architecture

```
Client Request (Accept-Encoding: gzip, br)
    ↓
Request Decompression (if Content-Encoding present)
    ↓
Router → Auth → Cache Check
    ↓
S3 Fetch (if cache miss)
    ↓
Response Compression Negotiation (select best algorithm)
    ↓
Compress Response (if size > threshold)
    ↓
Add Content-Encoding & Vary headers
    ↓
Cache Compressed Variant (if caching enabled)
    ↓
Stream to Client
```

## Key Design Decisions

1. **Compression is optional** - Disabled by default, enabled per-bucket
2. **Smart thresholds** - Only compress if response > min_size (default 1KB)
3. **Algorithm selection** - Client preference (Accept-Encoding) + server preference
4. **Cache variants** - Store compressed and uncompressed separately (Vary header)
5. **Streaming-friendly** - Compress while streaming (no full buffering)
6. **Performance-first** - Fast algorithms (gzip) preferred over ratio (brotli)

## Compression Algorithms

| Algorithm | Speed | Ratio | Use Case |
|-----------|-------|-------|----------|
| gzip      | Fast  | Good  | Default, widely supported |
| brotli    | Slow  | Best  | Static content, pre-compression |
| deflate   | Fast  | Fair  | Legacy support |

## Configuration Example

```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 6  # 1-11 for gzip, 1-11 for brotli
  min_response_size_bytes: 1024  # Don't compress < 1KB
  max_response_size_bytes: 104857600  # Don't compress > 100MB
  algorithms:
    - name: "gzip"
      enabled: true
      level: 6
    - name: "brotli"
      enabled: true
      level: 4
    - name: "deflate"
      enabled: false

buckets:
  - name: "public"
    compression:
      enabled: true
      algorithms: ["gzip", "brotli"]
      min_size: 512  # Override global setting
```

## Phase Breakdown

### Phase 40.1: Infrastructure (Core Types & Config)
- Compression enum (Gzip, Brotli, Deflate)
- CompressionConfig struct
- CompressionError type
- Algorithm negotiation logic
- Configuration parsing & validation

### Phase 40.2: Response Compression
- Accept-Encoding header parsing
- Algorithm selection (client + server preference)
- Response compression middleware
- Content-Encoding header injection
- Streaming compression support

### Phase 40.3: Request Decompression
- Content-Encoding header parsing
- Request body decompression
- Error handling for invalid compressed data
- Transparent decompression in pipeline

### Phase 40.4: Cache Integration
- Vary header handling (Accept-Encoding)
- Compressed variant caching
- Cache key variations
- Conditional compression based on cache state

### Phase 40.5: Configuration & Tuning
- Per-bucket compression settings
- Global compression defaults
- Compression level tuning
- Size threshold configuration
- Algorithm enable/disable

### Phase 40.6: Metrics & Observability
- Compression ratio metrics
- Algorithm usage counters
- Compression time histograms
- Tracing for compression decisions
- Performance impact analysis

### Phase 40.7: Testing & Benchmarking
- Unit tests for all algorithms
- Integration tests with real compression
- Benchmark compression performance
- Test cache variant handling
- Test error scenarios

### Phase 40.8: Documentation
- Configuration reference
- Performance characteristics
- Best practices guide
- Troubleshooting guide
- Migration guide from v1.4

## Dependencies to Add

```toml
flate2 = "1.0"  # gzip/deflate
brotli = "7.0"  # brotli compression
```

## Performance Targets

- Gzip compression: <10ms for 1MB file
- Brotli compression: <50ms for 1MB file
- Compression ratio: 60-80% for text, 20-40% for binary
- Memory overhead: <1MB per concurrent compression

## Testing Strategy

- Unit tests: Algorithm correctness, header parsing
- Integration tests: End-to-end compression with S3
- Benchmark tests: Compression speed vs ratio
- Cache tests: Variant storage and retrieval
- Error tests: Invalid compressed data handling

## Success Criteria

- ✅ All compression algorithms working
- ✅ Accept-Encoding negotiation correct
- ✅ Cache variants properly stored/retrieved
- ✅ Compression metrics accurate
- ✅ Performance targets met
- ✅ >90% test coverage
- ✅ Zero clippy warnings
- ✅ Documentation complete

