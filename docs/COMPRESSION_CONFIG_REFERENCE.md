# Compression Configuration Reference

## Global Configuration Options

### `compression.enabled`
- **Type**: `boolean`
- **Default**: `false`
- **Description**: Enable/disable compression globally
- **Example**: `enabled: true`

### `compression.default_algorithm`
- **Type**: `string`
- **Default**: `"gzip"`
- **Valid Values**: `"gzip"`, `"brotli"`, `"deflate"`
- **Description**: Default compression algorithm when client accepts multiple
- **Example**: `default_algorithm: "brotli"`

### `compression.compression_level`
- **Type**: `integer`
- **Default**: `6`
- **Valid Range**: `1-11`
- **Description**: Default compression level for all algorithms
  - 1-3: Fast compression, lower ratio
  - 4-6: Balanced (recommended)
  - 7-9: Slower, better ratio
  - 10-11: Very slow, best ratio
- **Example**: `compression_level: 6`

### `compression.min_response_size_bytes`
- **Type**: `integer`
- **Default**: `1024` (1KB)
- **Description**: Minimum response size to compress
- **Example**: `min_response_size_bytes: 512`

### `compression.max_response_size_bytes`
- **Type**: `integer`
- **Default**: `104857600` (100MB)
- **Description**: Maximum response size to compress
- **Example**: `max_response_size_bytes: 50000000`

### `compression.algorithms`
- **Type**: `object`
- **Description**: Per-algorithm configuration
- **Subkeys**:
  - `gzip.level`: Compression level for gzip (1-11)
  - `brotli.level`: Compression level for brotli (1-11)
  - `deflate.level`: Compression level for deflate (1-11)
- **Example**:
  ```yaml
  algorithms:
    gzip:
      level: 6
    brotli:
      level: 8
    deflate:
      level: 5
  ```

## Per-Bucket Configuration Options

All global options can be overridden per bucket:

```yaml
buckets:
  - name: "my-bucket"
    compression:
      enabled: true
      default_algorithm: "brotli"
      compression_level: 9
      min_response_size_bytes: 512
      max_response_size_bytes: 50000000
      algorithms:
        gzip:
          level: 7
        brotli:
          level: 9
        deflate:
          level: 6
```

## Configuration Examples

### Example 1: Aggressive Compression (High Ratio)
```yaml
compression:
  enabled: true
  default_algorithm: "brotli"
  compression_level: 9
  min_response_size_bytes: 256
  max_response_size_bytes: 100000000
```
- **Use Case**: Static content, CDN, low-bandwidth scenarios
- **Trade-off**: Higher CPU usage, slower compression

### Example 2: Balanced Configuration (Recommended)
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 6
  min_response_size_bytes: 1024
  max_response_size_bytes: 104857600
```
- **Use Case**: General purpose, most scenarios
- **Trade-off**: Good balance of speed and compression

### Example 3: Fast Compression (Low Latency)
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 3
  min_response_size_bytes: 2048
  max_response_size_bytes: 10485760
```
- **Use Case**: Real-time content, high-traffic scenarios
- **Trade-off**: Lower compression ratio, faster

### Example 4: Selective Compression (Per-Bucket)
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 6

buckets:
  - name: "static-assets"
    compression:
      enabled: true
      default_algorithm: "brotli"
      compression_level: 9
  
  - name: "api-responses"
    compression:
      enabled: true
      default_algorithm: "gzip"
      compression_level: 6
  
  - name: "video-files"
    compression:
      enabled: false
```

## Environment Variable Substitution

Configuration values can reference environment variables:

```yaml
compression:
  enabled: ${COMPRESSION_ENABLED}
  default_algorithm: ${COMPRESSION_ALGORITHM}
  compression_level: ${COMPRESSION_LEVEL}
```

Environment variables:
```bash
export COMPRESSION_ENABLED=true
export COMPRESSION_ALGORITHM=gzip
export COMPRESSION_LEVEL=6
```

## Validation Rules

The configuration is validated on startup:

1. **Compression Level**: Must be 1-11 if specified
2. **Size Thresholds**: `min_size < max_size` if both specified
3. **Algorithm Names**: Must be "gzip", "brotli", or "deflate"
4. **Per-Algorithm Levels**: Must be 1-11 if specified

Invalid configurations will cause startup failure with clear error messages.

## Performance Tuning Guide

### For Maximum Compression Ratio
```yaml
compression:
  default_algorithm: "brotli"
  compression_level: 9
  min_response_size_bytes: 256
```

### For Minimum Latency
```yaml
compression:
  default_algorithm: "gzip"
  compression_level: 3
  min_response_size_bytes: 2048
```

### For Balanced Performance
```yaml
compression:
  default_algorithm: "gzip"
  compression_level: 6
  min_response_size_bytes: 1024
```

### For CPU-Constrained Environments
```yaml
compression:
  default_algorithm: "gzip"
  compression_level: 2
  min_response_size_bytes: 4096
  max_response_size_bytes: 10485760
```

