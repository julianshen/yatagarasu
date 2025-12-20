# Compression Implementation Plan - Detailed Test Cases

## Phase 40.1: Compression Infrastructure

### Core Types & Enums
- [ ] Test: Can create Compression enum with Gzip, Brotli, Deflate variants
- [ ] Test: Can convert Compression to string ("gzip", "br", "deflate")
- [ ] Test: Can parse string to Compression enum
- [ ] Test: Unknown compression algorithm returns error

### Configuration Structures
- [ ] Test: Can parse compression config from YAML
- [ ] Test: Can access enabled flag from config
- [ ] Test: Can access default_algorithm from config
- [ ] Test: Can access compression_level from config
- [ ] Test: Can access min_response_size_bytes from config
- [ ] Test: Can access max_response_size_bytes from config
- [ ] Test: Can access per-algorithm settings (enabled, level)
- [ ] Test: Can access per-bucket compression overrides

### Configuration Validation
- [ ] Test: Validates compression_level is 1-11
- [ ] Test: Validates min_size < max_size
- [ ] Test: Validates at least one algorithm enabled
- [ ] Test: Rejects invalid algorithm names
- [ ] Test: Provides clear error messages for invalid config

### Error Handling
- [ ] Test: CompressionError enum with variants (InvalidAlgorithm, CompressionFailed, DecompressionFailed)
- [ ] Test: Errors convert to HTTP status codes
- [ ] Test: Error messages don't leak implementation details

### Algorithm Negotiation
- [ ] Test: Selects algorithm from Accept-Encoding header
- [ ] Test: Respects client preference order
- [ ] Test: Falls back to gzip if preferred unavailable
- [ ] Test: Returns None if no acceptable algorithm
- [ ] Test: Handles quality values (q=0.8)
- [ ] Test: Handles wildcard (*) in Accept-Encoding

## Phase 40.2: Response Compression

### Accept-Encoding Parsing
- [ ] Test: Parses "Accept-Encoding: gzip"
- [ ] Test: Parses "Accept-Encoding: gzip, deflate"
- [ ] Test: Parses "Accept-Encoding: gzip;q=1.0, br;q=0.8"
- [ ] Test: Handles missing Accept-Encoding header
- [ ] Test: Handles empty Accept-Encoding header
- [ ] Test: Case-insensitive header matching

### Response Compression Middleware
- [ ] Test: Compresses response if size > min_threshold
- [ ] Test: Skips compression if size < min_threshold
- [ ] Test: Skips compression if size > max_threshold
- [ ] Test: Skips compression if already compressed (Content-Encoding present)
- [ ] Test: Skips compression for certain content types (images, video)
- [ ] Test: Compresses text/json/xml content types

### Content-Encoding Header
- [ ] Test: Adds "Content-Encoding: gzip" header
- [ ] Test: Adds "Content-Encoding: br" header
- [ ] Test: Adds "Content-Encoding: deflate" header
- [ ] Test: Removes Content-Length header (chunked encoding)
- [ ] Test: Adds Transfer-Encoding: chunked if needed
- [ ] Test: Preserves other response headers

### Streaming Compression
- [ ] Test: Compresses while streaming (no full buffering)
- [ ] Test: Memory usage constant during compression
- [ ] Test: Client disconnect stops compression
- [ ] Test: Handles compression errors gracefully

## Phase 40.3: Request Decompression

### Content-Encoding Parsing
- [ ] Test: Parses "Content-Encoding: gzip"
- [ ] Test: Parses "Content-Encoding: br"
- [ ] Test: Parses "Content-Encoding: deflate"
- [ ] Test: Handles missing Content-Encoding header
- [ ] Test: Case-insensitive header matching

### Request Decompression
- [ ] Test: Decompresses gzip request body
- [ ] Test: Decompresses brotli request body
- [ ] Test: Decompresses deflate request body
- [ ] Test: Passes through uncompressed request body
- [ ] Test: Returns 400 for invalid compressed data
- [ ] Test: Returns 415 for unsupported Content-Encoding

### Pipeline Integration
- [ ] Test: Decompression happens before routing
- [ ] Test: Decompressed body available to auth/cache
- [ ] Test: Decompression errors handled gracefully

## Phase 40.4: Cache Integration

### Vary Header Handling
- [ ] Test: Adds "Vary: Accept-Encoding" header
- [ ] Test: Preserves existing Vary headers
- [ ] Test: Combines multiple Vary values correctly

### Compressed Variant Caching
- [ ] Test: Caches gzip variant separately from uncompressed
- [ ] Test: Caches brotli variant separately
- [ ] Test: Cache key includes compression algorithm
- [ ] Test: Retrieves correct variant based on Accept-Encoding

### Cache Key Variations
- [ ] Test: Different Accept-Encoding = different cache keys
- [ ] Test: Same content, different compression = different entries
- [ ] Test: Uncompressed version cached alongside compressed

## Phase 40.5: Configuration & Tuning

### Per-Bucket Settings
- [ ] Test: Can override global compression settings per bucket
- [ ] Test: Can disable compression for specific bucket
- [ ] Test: Can set different algorithms per bucket
- [ ] Test: Can set different compression levels per bucket

### Compression Level Tuning
- [ ] Test: Level 1 = fastest compression
- [ ] Test: Level 11 = best compression ratio
- [ ] Test: Invalid levels rejected

### Size Thresholds
- [ ] Test: min_response_size_bytes prevents small file compression
- [ ] Test: max_response_size_bytes prevents large file compression
- [ ] Test: Threshold configuration per bucket

## Phase 40.6: Metrics & Observability

### Compression Metrics
- [ ] Test: Tracks compression ratio (original/compressed)
- [ ] Test: Tracks algorithm usage (gzip/brotli/deflate)
- [ ] Test: Tracks compression time histogram
- [ ] Test: Tracks bytes saved by compression
- [ ] Test: Tracks compression errors

### Tracing
- [ ] Test: Logs compression decision (why/why not)
- [ ] Test: Logs selected algorithm
- [ ] Test: Logs compression ratio
- [ ] Test: Includes request_id in compression logs

## Phase 40.7: Testing & Benchmarking

### Unit Tests
- [ ] Test: All algorithms compress/decompress correctly
- [ ] Test: Compression is reversible (compress → decompress = original)
- [ ] Test: Header parsing handles edge cases
- [ ] Test: Configuration validation comprehensive

### Integration Tests
- [ ] Test: End-to-end compression with S3 objects
- [ ] Test: Compression with cache hits/misses
- [ ] Test: Compression with range requests
- [ ] Test: Compression with authentication

### Benchmarks
- [ ] Benchmark: Gzip compression speed (1MB, 10MB, 100MB)
- [ ] Benchmark: Brotli compression speed
- [ ] Benchmark: Compression ratio for different content types
- [ ] Benchmark: Memory usage during compression

## Phase 40.8: Documentation

### Configuration Reference
- [ ] Document all compression config options
- [ ] Provide example configurations
- [ ] Document per-bucket overrides

### Performance Guide
- [ ] Document compression ratios by content type
- [ ] Document compression speed by algorithm
- [ ] Provide tuning recommendations

### Best Practices
- [ ] When to enable/disable compression
- [ ] Algorithm selection guide
- [ ] Performance tuning tips
- [ ] Troubleshooting guide

