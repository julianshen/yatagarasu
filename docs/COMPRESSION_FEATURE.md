# Compression Feature Documentation

## Overview

Yatagarasu includes a comprehensive compression feature that automatically compresses HTTP responses to reduce bandwidth usage and improve performance. The feature supports multiple compression algorithms and provides fine-grained control over when and how compression is applied.

## Supported Algorithms

### Gzip (RFC 1952)
- **Speed**: Fast
- **Compression Ratio**: Good (typically 60-70% reduction for text)
- **Compatibility**: Excellent (supported by all modern browsers)
- **Use Case**: Default choice for most content types
- **Compression Levels**: 1-11 (default: 6)

### Brotli (RFC 7932)
- **Speed**: Slower than gzip
- **Compression Ratio**: Excellent (typically 70-80% reduction for text)
- **Compatibility**: Good (supported by modern browsers, not IE)
- **Use Case**: Best compression ratio when speed is not critical
- **Compression Levels**: 1-11 (default: 6)

### Deflate (RFC 1951)
- **Speed**: Fast
- **Compression Ratio**: Fair (typically 50-60% reduction for text)
- **Compatibility**: Good (supported by most browsers)
- **Use Case**: Fallback option
- **Compression Levels**: 1-11 (default: 6)

## Configuration

### Global Configuration

```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 6
  min_response_size_bytes: 1024        # Don't compress responses < 1KB
  max_response_size_bytes: 104857600   # Don't compress responses > 100MB
  algorithms:
    gzip:
      level: 6
    brotli:
      level: 6
    deflate:
      level: 6
```

### Per-Bucket Configuration

Override global settings for specific buckets:

```yaml
buckets:
  - name: "public-assets"
    compression:
      enabled: true
      default_algorithm: "brotli"
      compression_level: 9
      min_response_size_bytes: 512
      max_response_size_bytes: 50000000
```

## Content Type Handling

### Compressible Content Types
- `text/*` - All text formats
- `application/json` - JSON data
- `application/xml` - XML data
- `application/javascript` - JavaScript
- `image/svg+xml` - SVG images
- `application/wasm` - WebAssembly

### Non-Compressible Content Types
- `image/*` - Already compressed (PNG, JPEG, WebP, etc.)
- `video/*` - Already compressed (MP4, WebM, etc.)
- `audio/*` - Already compressed (MP3, AAC, etc.)
- `application/octet-stream` - Binary data

## Size Thresholds

Compression is only applied when response size is within configured thresholds:

- **Minimum Size**: Default 1KB
  - Responses smaller than this are not compressed (overhead not worth it)
  - Configurable per bucket

- **Maximum Size**: Default 100MB
  - Responses larger than this are not compressed (memory/time constraints)
  - Configurable per bucket

## Compression Decision Flow

```
1. Is compression enabled globally? → No → Skip compression
2. Is compression enabled for this bucket? → No → Skip compression
3. Is response already compressed? → Yes → Skip compression
4. Is content type compressible? → No → Skip compression
5. Is response size within thresholds? → No → Skip compression
6. Does client accept compression? → No → Skip compression
7. Apply compression with negotiated algorithm
```

## Client Negotiation

The proxy respects the `Accept-Encoding` header from clients:

```
Accept-Encoding: gzip, deflate, br;q=0.9, *;q=0.1
```

The proxy:
1. Parses client preferences and quality values (q=0.0-1.0)
2. Selects the best algorithm that is:
   - Accepted by the client
   - Enabled in configuration
   - Supported by the proxy
3. Applies compression with selected algorithm

## Caching

Compressed responses are cached separately from uncompressed versions:

- **Cache Key Suffix**:
  - Uncompressed: `:uncompressed`
  - Gzip: `:compressed:gzip`
  - Brotli: `:compressed:br`
  - Deflate: `:compressed:deflate`

- **Vary Header**: Automatically set to `Accept-Encoding` to indicate that response varies by compression

## Performance Characteristics

### Compression Ratios (Text Content)
- **Gzip**: 60-70% reduction
- **Brotli**: 70-80% reduction
- **Deflate**: 50-60% reduction

### Compression Speed (1MB text)
- **Gzip**: ~50ms (level 6)
- **Brotli**: ~200ms (level 6)
- **Deflate**: ~30ms (level 6)

### Memory Usage
- Per-connection: ~64KB (streaming)
- No buffering of full response to disk

## Metrics

The compression feature tracks:

- **Compression Ratio**: `compressed_size / original_size`
- **Bytes Saved**: `original_size - compressed_size`
- **Percentage Saved**: `(bytes_saved / original_size) * 100`
- **Throughput**: `original_size / compression_time`
- **Decision Reason**: Why compression was/wasn't applied

## Best Practices

### When to Enable Compression
- ✅ Text-heavy content (HTML, CSS, JSON, XML)
- ✅ JavaScript files
- ✅ SVG images
- ✅ High-bandwidth scenarios
- ✅ Mobile clients with limited bandwidth

### When to Disable Compression
- ❌ Already-compressed content (images, video, audio)
- ❌ Very small responses (<1KB)
- ❌ Very large responses (>100MB)
- ❌ Real-time streaming
- ❌ Low-CPU environments

### Algorithm Selection
- **Gzip**: Default choice (good balance of speed and compression)
- **Brotli**: When compression ratio is critical (slower)
- **Deflate**: Fallback for compatibility

### Compression Level Tuning
- **Level 1-3**: Fast compression, lower ratio (use for real-time)
- **Level 4-6**: Balanced (default level 6)
- **Level 7-9**: Slower, better ratio (use for static content)
- **Level 10-11**: Very slow, best ratio (rarely needed)

## Troubleshooting

### Compression Not Applied
Check in order:
1. Is compression enabled globally? (`enabled: true`)
2. Is compression enabled for the bucket?
3. Is the content type compressible?
4. Is the response size within thresholds?
5. Does the client accept compression? (Check `Accept-Encoding` header)

### High CPU Usage
- Reduce compression level (use 4-5 instead of 9-11)
- Increase minimum response size threshold
- Disable compression for large files
- Use gzip instead of brotli

### Cache Misses
- Ensure `Vary: Accept-Encoding` header is set
- Check that cache keys include compression variant
- Verify client sends consistent `Accept-Encoding` header

