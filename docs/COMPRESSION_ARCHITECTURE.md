# Compression Architecture & Integration Points

## Module Structure

```
src/compression/
├── mod.rs              # Public API, main compression module
├── config.rs           # CompressionConfig, validation
├── algorithms.rs       # Compression enum, algorithm traits
├── negotiation.rs      # Accept-Encoding parsing, algorithm selection
├── response.rs         # Response compression middleware
├── request.rs          # Request decompression middleware
├── cache.rs            # Cache integration, Vary header handling
├── metrics.rs          # Compression metrics
└── error.rs            # CompressionError type
```

## Core Types

### Compression Enum
```rust
pub enum Compression {
    Gzip,
    Brotli,
    Deflate,
}
```

### CompressionConfig
```rust
pub struct CompressionConfig {
    pub enabled: bool,
    pub default_algorithm: Compression,
    pub compression_level: u32,
    pub min_response_size_bytes: usize,
    pub max_response_size_bytes: usize,
    pub algorithms: HashMap<Compression, AlgorithmConfig>,
}

pub struct AlgorithmConfig {
    pub enabled: bool,
    pub level: u32,
}
```

### CompressionError
```rust
pub enum CompressionError {
    InvalidAlgorithm(String),
    CompressionFailed(String),
    DecompressionFailed(String),
    InvalidConfig(String),
}
```

## Integration Points

### 1. Configuration Loading (src/config/mod.rs)
- Add `compression: Option<CompressionConfig>` to ServerConfig
- Add `compression: Option<CompressionConfig>` to BucketConfig
- Parse compression settings from YAML
- Validate compression configuration

### 2. Request Pipeline (src/pipeline/mod.rs)
- Add `compression_algorithm: Option<Compression>` to RequestContext
- Add `is_response_compressed: bool` to RequestContext
- Store negotiated compression algorithm in context

### 3. Proxy Handler (src/proxy/mod.rs)
- Call compression negotiation in `request_filter()`
- Call response compression in `upstream_response_filter()`
- Call response compression in `response_body_filter()`
- Add compression metrics

### 4. Cache Layer (src/cache/mod.rs)
- Store compressed variants separately
- Include compression algorithm in cache key
- Handle Vary: Accept-Encoding header
- Retrieve correct variant based on Accept-Encoding

### 5. Metrics (src/metrics/mod.rs)
- Add compression_ratio gauge
- Add algorithm_usage counter
- Add compression_time histogram
- Add bytes_saved counter

### 6. Observability (src/observability/mod.rs)
- Add compression decision tracing
- Log selected algorithm
- Log compression ratio
- Include in request logging

## Request Flow with Compression

```
1. Client sends: GET /file.txt
   Headers: Accept-Encoding: gzip, br

2. Proxy receives request
   → request_filter() calls compression negotiation
   → Selects "gzip" (client preference)
   → Stores in RequestContext

3. Fetch from S3 (or cache)
   → Response: 200 OK, Content-Type: text/plain, body: "Hello World"

4. upstream_response_filter()
   → Check if compression enabled
   → Check response size > min_threshold
   → Check not already compressed
   → Prepare for compression

5. response_body_filter()
   → Compress chunks while streaming
   → Update Content-Encoding header
   → Add Vary header
   → Track compression metrics

6. Send to client
   → Content-Encoding: gzip
   → Vary: Accept-Encoding
   → Compressed body

7. Cache (if enabled)
   → Store compressed variant
   → Cache key includes algorithm
   → Store Vary header
```

## Compression Decision Tree

```
Is compression enabled?
├─ No → Skip compression
└─ Yes
   ├─ Is response already compressed?
   │  ├─ Yes → Skip compression
   │  └─ No
   │     ├─ Is response size < min_threshold?
   │     │  ├─ Yes → Skip compression
   │     │  └─ No
   │     │     ├─ Is response size > max_threshold?
   │     │     │  ├─ Yes → Skip compression
   │     │     │  └─ No
   │     │     │     ├─ Is content type compressible?
   │     │     │     │  ├─ No → Skip compression
   │     │     │     │  └─ Yes
   │     │     │     │     ├─ Select algorithm from Accept-Encoding
   │     │     │     │     ├─ Compress response
   │     │     │     │     ├─ Add Content-Encoding header
   │     │     │     │     ├─ Add Vary header
   │     │     │     │     └─ Cache compressed variant
```

## Compressible Content Types

```rust
const COMPRESSIBLE_TYPES: &[&str] = &[
    "text/",
    "application/json",
    "application/xml",
    "application/javascript",
    "application/x-www-form-urlencoded",
    "image/svg+xml",
];

const INCOMPRESSIBLE_TYPES: &[&str] = &[
    "image/",
    "video/",
    "audio/",
    "application/zip",
    "application/gzip",
    "application/x-rar-compressed",
];
```

## Performance Considerations

### Compression Overhead
- Gzip: ~5-10ms for 1MB text
- Brotli: ~50-100ms for 1MB text
- Deflate: ~3-5ms for 1MB text

### Compression Ratio
- Text: 60-80% reduction
- JSON: 70-85% reduction
- HTML: 70-80% reduction
- Binary: 10-30% reduction
- Already compressed: 0-5% reduction

### Memory Usage
- Gzip: ~32KB buffer
- Brotli: ~64KB buffer
- Deflate: ~32KB buffer

## Testing Strategy

### Unit Tests
- Algorithm correctness
- Header parsing
- Configuration validation
- Error handling

### Integration Tests
- End-to-end compression with S3
- Cache variant handling
- Compression with authentication
- Compression with range requests

### Benchmarks
- Compression speed by algorithm
- Compression ratio by content type
- Memory usage during compression
- Impact on request latency

## Backward Compatibility

- Compression disabled by default
- No breaking changes to existing APIs
- Existing cache entries remain valid
- Graceful fallback if compression fails

