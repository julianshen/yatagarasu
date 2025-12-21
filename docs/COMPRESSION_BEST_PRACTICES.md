# Compression Best Practices & Troubleshooting

## Best Practices

### 1. Enable Compression for Text Content
✅ **DO**: Enable compression for HTML, CSS, JavaScript, JSON, XML
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
```

❌ **DON'T**: Enable compression for already-compressed formats (images, video, audio)

### 2. Set Appropriate Size Thresholds
✅ **DO**: Skip compression for very small responses
```yaml
compression:
  min_response_size_bytes: 1024  # 1KB minimum
```

❌ **DON'T**: Compress responses < 1KB (overhead not worth it)

### 3. Use Gzip as Default
✅ **DO**: Use gzip for broad compatibility
```yaml
compression:
  default_algorithm: "gzip"
```

❌ **DON'T**: Use brotli as default (not supported by older browsers)

### 4. Respect Client Preferences
✅ **DO**: Honor Accept-Encoding header
- Proxy automatically negotiates with client
- Selects best algorithm client accepts

❌ **DON'T**: Force compression algorithm client doesn't accept

### 5. Monitor Compression Metrics
✅ **DO**: Track compression ratio and performance
```bash
curl http://localhost:9090/metrics | grep compression
```

❌ **DON'T**: Assume compression is working without verification

### 6. Use Per-Bucket Configuration
✅ **DO**: Optimize compression per bucket
```yaml
buckets:
  - name: "static"
    compression:
      default_algorithm: "brotli"
      compression_level: 9
```

❌ **DON'T**: Use one-size-fits-all configuration

### 7. Cache Compressed Variants
✅ **DO**: Ensure Vary header is set
- Proxy automatically adds `Vary: Accept-Encoding`
- Different algorithms cached separately

❌ **DON'T**: Serve wrong compression variant from cache

### 8. Test with Real Clients
✅ **DO**: Test with actual browsers and clients
```bash
curl -H "Accept-Encoding: gzip, deflate, br" http://localhost:8080/file
```

❌ **DON'T**: Assume configuration works without testing

## Troubleshooting Guide

### Issue 1: Compression Not Applied

**Symptoms**: Response not compressed despite configuration

**Diagnosis**:
1. Check if compression is enabled globally
   ```yaml
   compression:
     enabled: true
   ```

2. Check if compression is enabled for bucket
   ```yaml
   buckets:
     - name: "my-bucket"
       compression:
         enabled: true
   ```

3. Check content type is compressible
   - Text, JSON, XML, JavaScript: ✅ Compressible
   - Images, video, audio: ❌ Not compressible

4. Check response size is within thresholds
   ```bash
   # Check response size
   curl -I http://localhost:8080/file | grep Content-Length
   ```

5. Check client accepts compression
   ```bash
   # Check Accept-Encoding header
   curl -v http://localhost:8080/file 2>&1 | grep Accept-Encoding
   ```

**Solution**:
- Enable compression: `enabled: true`
- Check content type is in compressible list
- Adjust size thresholds if needed
- Ensure client sends Accept-Encoding header

### Issue 2: High CPU Usage

**Symptoms**: CPU usage spikes when compression enabled

**Diagnosis**:
1. Check compression level
   ```yaml
   compression:
     compression_level: 9  # Too high?
   ```

2. Check algorithm choice
   - Brotli: Slower than gzip
   - Gzip: Faster than brotli
   - Deflate: Fastest

3. Check response sizes
   - Large responses take longer to compress

**Solution**:
- Reduce compression level (use 3-6 instead of 9-11)
- Switch to gzip (faster than brotli)
- Increase min_response_size_bytes
- Disable compression for large files

### Issue 3: Cache Misses

**Symptoms**: Cache hit rate drops after enabling compression

**Diagnosis**:
1. Check Vary header
   ```bash
   curl -I http://localhost:8080/file | grep Vary
   ```
   Should include `Accept-Encoding`

2. Check cache key generation
   - Different algorithms should have different keys
   - Uncompressed should have different key

3. Check client consistency
   - Different clients may send different Accept-Encoding

**Solution**:
- Ensure Vary header includes Accept-Encoding
- Verify cache keys include compression variant
- Monitor cache hit rate per algorithm

### Issue 4: Compression Ratio Too Low

**Symptoms**: Compression ratio < 50% for text content

**Diagnosis**:
1. Check content type
   - Already-compressed formats won't compress well
   - Binary data won't compress well

2. Check compression level
   - Level 1-3: Lower ratio
   - Level 6: Balanced
   - Level 9-11: Better ratio

3. Check response size
   - Very small responses may not compress well

**Solution**:
- Verify content type is actually text
- Increase compression level (use 9 for static content)
- Check if content is already compressed
- Increase min_response_size_bytes

### Issue 5: Slow Response Times

**Symptoms**: Response times increase after enabling compression

**Diagnosis**:
1. Check compression time
   - Brotli: ~200ms per MB
   - Gzip: ~50ms per MB
   - Deflate: ~30ms per MB

2. Check response sizes
   - Large responses take longer to compress

3. Check compression level
   - Higher levels = slower compression

**Solution**:
- Reduce compression level (use 3-6)
- Switch to faster algorithm (gzip or deflate)
- Increase min_response_size_bytes
- Disable compression for large files

### Issue 6: Client Decompression Errors

**Symptoms**: Client receives corrupted data or decompression errors

**Diagnosis**:
1. Check Content-Encoding header
   ```bash
   curl -I http://localhost:8080/file | grep Content-Encoding
   ```

2. Check compression algorithm
   - Verify algorithm matches Content-Encoding

3. Check for double compression
   - Response shouldn't be compressed twice

**Solution**:
- Verify Content-Encoding header is correct
- Check compression algorithm matches header
- Ensure response isn't already compressed
- Test with curl: `curl --compressed http://localhost:8080/file`

## Performance Tuning Checklist

- [ ] Compression enabled for text content
- [ ] Compression disabled for binary content
- [ ] Size thresholds set appropriately
- [ ] Compression level tuned for workload
- [ ] Gzip used as default algorithm
- [ ] Vary header includes Accept-Encoding
- [ ] Cache hit rate monitored
- [ ] CPU usage within acceptable range
- [ ] Compression ratio verified
- [ ] Response times acceptable
- [ ] Client decompression working
- [ ] Per-bucket configuration optimized

## Quick Reference

### Enable Compression (Recommended)
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 6
  min_response_size_bytes: 1024
  max_response_size_bytes: 104857600
```

### Disable Compression
```yaml
compression:
  enabled: false
```

### Aggressive Compression
```yaml
compression:
  enabled: true
  default_algorithm: "brotli"
  compression_level: 9
  min_response_size_bytes: 256
```

### Fast Compression
```yaml
compression:
  enabled: true
  default_algorithm: "gzip"
  compression_level: 3
  min_response_size_bytes: 2048
```

