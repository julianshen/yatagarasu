# Image Optimization Feature Specification

**Version**: 1.0
**Status**: Draft
**Target Release**: v1.6.0
**Author**: Yatagarasu Team
**Date**: December 2025

---

## Executive Summary

Enhance Yatagarasu's image optimization capabilities to provide a production-ready, high-performance image processing pipeline inspired by imgproxy and imageproxy. The feature will support on-the-fly image transformations, format conversion, and optimization with security, caching, and observability.

---

## Current State Analysis

### Existing Implementation (`src/image_optimizer/`)

| Feature | Status | Notes |
|---------|--------|-------|
| Basic resize | ✅ Exists | Uses `fast_image_resize` with Lanczos3 |
| Format conversion | ✅ Exists | JPEG, PNG, WebP, AVIF |
| Quality control | ✅ Exists | Via `q` parameter |
| Fit modes | ✅ Exists | Cover, Contain, Fill, Inside, Outside |
| Query params | ✅ Exists | `w`, `h`, `q`, `fmt`, `fit` |
| Config | ✅ Exists | max_width, max_height, default_quality |

### Gaps to Address

| Feature | Priority | Status |
|---------|----------|--------|
| URL signing (HMAC) | High | ❌ Missing |
| Smart/content-aware crop | High | ❌ Missing |
| Advanced encoders (mozjpeg, oxipng, ravif) | High | ❌ Missing |
| Auto-format (Accept header) | High | ❌ Missing |
| Crop positioning | Medium | ❌ Missing |
| Rotation/flip | Medium | ❌ Missing |
| Watermarking | Medium | ❌ Missing |
| Blur/sharpen filters | Low | ❌ Missing |
| Cache integration | High | ❌ Missing |
| Metrics & observability | High | ❌ Missing |
| Image bomb protection | High | ❌ Missing |
| Streaming processing | Medium | ❌ Missing |

---

## Feature Requirements

### 1. URL Format

Support two URL patterns:

#### Pattern A: Query Parameters (Current)
```
GET /bucket/path/image.jpg?w=800&h=600&q=80&fmt=webp
```

#### Pattern B: Path-based (imgproxy-style)
```
GET /_img/{signature}/{options}/{encoded_url}
GET /_img/insecure/{options}/{bucket}/{path}
```

**Options Format**: Comma-separated key-value pairs
```
w:800,h:600,q:80,f:webp,fit:cover,crop:smart
```

### 2. Transformation Operations

#### 2.1 Resize Operations

| Parameter | Description | Example |
|-----------|-------------|---------|
| `w` | Target width (pixels or percentage) | `w:800`, `w:50p` |
| `h` | Target height | `h:600` |
| `fit` | Fit mode | `fit:cover`, `fit:contain`, `fit:fill` |
| `dpr` | Device pixel ratio (1-4) | `dpr:2` |
| `enlarge` | Allow upscaling | `enlarge:1` |

#### 2.2 Crop Operations

| Parameter | Description | Example |
|-----------|-------------|---------|
| `crop` | Crop mode | `crop:smart`, `crop:attention`, `crop:entropy` |
| `gravity` | Crop position | `g:center`, `g:north`, `g:face` |
| `cx`, `cy` | Crop offset | `cx:100,cy:50` |
| `cw`, `ch` | Crop dimensions | `cw:400,ch:300` |

#### 2.3 Format & Quality

| Parameter | Description | Example |
|-----------|-------------|---------|
| `f` / `fmt` | Output format | `f:webp`, `f:avif`, `f:jpeg`, `f:png` |
| `q` | Quality (1-100) | `q:80` |
| `auto` | Auto-format from Accept | `auto:webp,avif` |
| `strip` | Strip metadata | `strip:1` |
| `progressive` | Progressive encoding | `progressive:1` |

#### 2.4 Effects

| Parameter | Description | Example |
|-----------|-------------|---------|
| `r` / `rotate` | Rotation (degrees) | `r:90`, `r:180`, `r:270` |
| `flip` | Flip direction | `flip:h`, `flip:v` |
| `blur` | Gaussian blur (sigma) | `blur:5` |
| `sharpen` | Sharpen (sigma) | `sharpen:1.5` |
| `brightness` | Brightness adjustment | `brightness:10` |
| `contrast` | Contrast adjustment | `contrast:1.2` |

#### 2.5 Watermark (Phase 2)

| Parameter | Description | Example |
|-----------|-------------|---------|
| `wm` | Watermark image URL | `wm:logo.png` |
| `wm_pos` | Watermark position | `wm_pos:se` (southeast) |
| `wm_opacity` | Watermark opacity | `wm_opacity:0.5` |
| `wm_scale` | Watermark scale | `wm_scale:0.1` |

### 3. Security Features

#### 3.1 URL Signing (HMAC-SHA256)

```
signature = HMAC-SHA256(secret_key, options + "/" + source_url)
URL = /_img/{signature}/{options}/{encoded_url}
```

**Configuration**:
```yaml
image_optimization:
  security:
    signing_required: true
    signing_key: "${IMAGE_SIGNING_KEY}"
    signing_salt: "${IMAGE_SIGNING_SALT}"  # Optional
```

#### 3.2 Image Bomb Protection

- Validate image dimensions before full decode
- Maximum pixel limit (e.g., 100 megapixels)
- Maximum file size before processing
- Timeout for processing operations

```yaml
image_optimization:
  limits:
    max_source_width: 10000
    max_source_height: 10000
    max_source_pixels: 100000000  # 100MP
    max_source_file_size: 52428800  # 50MB
    processing_timeout_ms: 30000
```

#### 3.3 Allowed Sources

```yaml
image_optimization:
  security:
    allowed_sources:
      - "s3://my-bucket/*"
      - "https://cdn.example.com/*"
    blocked_sources:
      - "*.internal.example.com"
```

### 4. Format Support

#### 4.1 Input Formats

| Format | Library | Notes |
|--------|---------|-------|
| JPEG | zune-jpeg | Fastest Rust decoder |
| PNG | zune-png | Fast decoder |
| WebP | webp | Google's library |
| AVIF | libavif | AV1-based |
| GIF | image | Static only (Phase 1) |
| TIFF | image | Full support |
| BMP | image | Full support |

#### 4.2 Output Formats

| Format | Encoder | Quality Range | Notes |
|--------|---------|---------------|-------|
| JPEG | mozjpeg | 1-100 | Best compression |
| PNG | oxipng | 0-6 (levels) | Lossless, multithreaded |
| WebP | webp | 1-100 | Good compression + transparency |
| AVIF | ravif | 1-100 | Best compression, slow encode |

### 5. Auto-Format Selection

Based on `Accept` header with fallback chain:

```
Accept: image/avif,image/webp,image/jpeg
```

Selection priority:
1. AVIF (if supported and beneficial)
2. WebP (if supported)
3. Original format (JPEG/PNG)

**Decision Factors**:
- Browser support (Accept header)
- Source format (preserve PNG transparency)
- File size benefit threshold (>10% savings)
- Processing time budget

### 6. Caching Strategy

#### 6.1 Cache Key Generation

```
cache_key = hash(source_url + options + format)
```

Example:
```
/bucket/image.jpg:w800_h600_q80_fwebp_fitcover
```

#### 6.2 Cache Headers

```http
Cache-Control: public, max-age=31536000
Vary: Accept
ETag: "abc123"
```

#### 6.3 Integration with Existing Cache

- Use existing cache layer (memory, Redis, disk)
- Store transformed variants separately
- Support cache purge by source URL (invalidate all variants)

### 7. Metrics & Observability

#### 7.1 Prometheus Metrics

```
# Processing time histogram
yatagarasu_image_processing_duration_seconds{operation="resize",format="webp"}

# Transformation counters
yatagarasu_image_transformations_total{operation="resize",status="success"}
yatagarasu_image_transformations_total{operation="resize",status="error"}

# Size reduction
yatagarasu_image_bytes_saved_total{format="webp"}
yatagarasu_image_compression_ratio{format="webp"}

# Cache performance
yatagarasu_image_cache_hits_total
yatagarasu_image_cache_misses_total
```

#### 7.2 Logging

```json
{
  "event": "image_processed",
  "source": "bucket/image.jpg",
  "operations": ["resize", "format_convert"],
  "input_size": 1048576,
  "output_size": 262144,
  "compression_ratio": 0.25,
  "duration_ms": 45,
  "output_format": "webp",
  "dimensions": {"width": 800, "height": 600}
}
```

---

## Configuration Schema

```yaml
image_optimization:
  # Global enable/disable
  enabled: true

  # Processing limits
  limits:
    max_width: 4096
    max_height: 4096
    max_source_width: 10000
    max_source_height: 10000
    max_source_pixels: 100000000
    max_source_file_size: 52428800
    processing_timeout_ms: 30000

  # Default settings
  defaults:
    quality: 80
    format: "auto"  # auto, jpeg, png, webp, avif
    fit: "cover"
    strip_metadata: true
    progressive: true

  # Encoder settings
  encoders:
    jpeg:
      encoder: "mozjpeg"  # mozjpeg, image
      quality: 80
      progressive: true
      chroma_subsample: "4:2:0"
    png:
      encoder: "oxipng"  # oxipng, image
      compression_level: 3  # 0-6
      strip_metadata: true
    webp:
      quality: 80
      lossless: false
      near_lossless: false
    avif:
      quality: 70
      speed: 6  # 1-10 (higher = faster, lower quality)

  # Security
  security:
    signing_required: false
    signing_key: "${IMAGE_SIGNING_KEY}"
    allowed_sources: []
    blocked_sources: []

  # Auto-format settings
  auto_format:
    enabled: true
    prefer_avif: true
    prefer_webp: true
    min_savings_percent: 10

  # Per-bucket overrides
  buckets:
    static-assets:
      enabled: true
      defaults:
        quality: 85
        format: "webp"
    user-uploads:
      enabled: true
      limits:
        max_width: 2048
        max_height: 2048
```

---

## API Design

### Public API

```rust
/// Image optimization result
pub struct OptimizedImage {
    pub data: Vec<u8>,
    pub content_type: String,
    pub width: u32,
    pub height: u32,
    pub original_size: usize,
    pub optimized_size: usize,
}

/// Parse image parameters from URL/query
pub fn parse_params(
    query: &HashMap<String, String>,
    path_options: Option<&str>,
) -> Result<ImageParams, ImageError>;

/// Process image with given parameters
pub async fn process_image(
    source: &[u8],
    params: &ImageParams,
    config: &ImageConfig,
) -> Result<OptimizedImage, ImageError>;

/// Validate URL signature
pub fn validate_signature(
    signature: &str,
    options: &str,
    source_url: &str,
    config: &SecurityConfig,
) -> Result<(), ImageError>;

/// Generate URL signature
pub fn generate_signature(
    options: &str,
    source_url: &str,
    config: &SecurityConfig,
) -> String;
```

### Error Types

```rust
pub enum ImageError {
    // Decoding errors
    UnsupportedFormat(String),
    DecodeFailed(String),
    CorruptedImage(String),

    // Processing errors
    ResizeFailed(String),
    EncodeFailed(String),
    ProcessingTimeout,

    // Security errors
    InvalidSignature,
    SourceNotAllowed(String),
    ImageBombDetected { width: u32, height: u32, pixels: u64 },
    FileTooLarge { size: usize, max: usize },

    // Configuration errors
    InvalidParameter(String),
    InvalidDimensions { width: u32, height: u32 },
}

impl ImageError {
    pub fn to_http_status(&self) -> u16 {
        match self {
            Self::UnsupportedFormat(_) => 415,
            Self::InvalidSignature => 403,
            Self::SourceNotAllowed(_) => 403,
            Self::ImageBombDetected { .. } => 400,
            Self::FileTooLarge { .. } => 413,
            Self::InvalidParameter(_) => 400,
            Self::ProcessingTimeout => 504,
            _ => 500,
        }
    }
}
```

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Resize 1000x1000 → 500x500 | <50ms | JPEG output |
| Format convert (JPEG → WebP) | <100ms | 1MP image |
| AVIF encoding | <500ms | 1MP image, speed=6 |
| Throughput | >100 req/s | Per CPU core |
| Memory per image | <50MB | Peak during processing |
| Cache hit response | <10ms | P95 |

---

## Dependencies

### Required Crates

```toml
# Core image processing
image = "0.24"
fast_image_resize = "2.7"
zune-image = "0.4"
zune-jpeg = "0.4"

# Optimized encoders
mozjpeg = "0.10"
oxipng = "9.0"
webp = "0.2"
ravif = "0.11"
libavif = { version = "0.14", optional = true }

# Smart crop (optional, Phase 2)
# smartcrop = "0.2"  # or implement basic edge detection
```

### Feature Flags

```toml
[features]
default = ["jpeg", "png", "webp"]
jpeg = ["mozjpeg", "zune-jpeg"]
png = ["oxipng"]
webp = ["webp"]
avif = ["ravif", "libavif"]
full = ["jpeg", "png", "webp", "avif"]
```

---

## Security Considerations

1. **Image Bomb Protection**: Validate dimensions before full decode
2. **URL Signing**: HMAC-SHA256 prevents unauthorized transformations
3. **Rate Limiting**: Use existing rate limiter for image endpoints
4. **Memory Limits**: Cap concurrent image processing
5. **Timeout Protection**: Kill long-running operations
6. **Source Validation**: Whitelist allowed image sources

---

## Compatibility

- Maintains backward compatibility with existing query parameter API
- New path-based API is opt-in
- Existing cache entries remain valid
- No breaking changes to configuration (additive only)

---

## Success Criteria

1. All 6 core operations working (resize, crop, rotate, flip, format, quality)
2. Auto-format selection based on Accept header
3. URL signing for security
4. Image bomb protection
5. Cache integration with variant storage
6. Metrics for monitoring
7. >90% test coverage
8. Performance targets met
9. Documentation complete

---

## Out of Scope (Future Phases)

- Animated GIF/WebP processing
- Video thumbnail generation
- Face detection for smart crop
- AI-based upscaling
- PDF/PSD preview generation
- SVG rasterization
