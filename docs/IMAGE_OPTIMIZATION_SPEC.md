# Image Optimization Feature Specification

**Version**: 1.1
**Status**: In Progress
**Target Release**: v1.6.0
**Author**: Yatagarasu Team
**Date**: December 2025
**Last Updated**: December 2025

---

## Executive Summary

Enhance Yatagarasu's image optimization capabilities to provide a production-ready, high-performance image processing pipeline inspired by imgproxy and imageproxy. The feature will support on-the-fly image transformations, format conversion, and optimization with security, caching, and observability.

---

## Current State Analysis

### Implementation Status (`src/image_optimizer/`)

| Feature               | Status      | Module           | Notes                                                           |
| --------------------- | ----------- | ---------------- | --------------------------------------------------------------- |
| Basic resize          | ✅ Complete | `processor.rs`   | Uses `fast_image_resize` with Lanczos3                          |
| Format conversion     | ✅ Complete | `encoder.rs`     | JPEG, PNG, WebP, AVIF (placeholder)                             |
| Quality control       | ✅ Complete | `params.rs`      | Via `q` parameter (1-100)                                       |
| Fit modes             | ✅ Complete | `params.rs`      | Cover, Contain, Fill, Inside, Outside, Pad                      |
| Query params          | ✅ Complete | `params.rs`      | `w`, `h`, `q`, `fmt`, `fit`, `dpr`, `g`, `r`, `blur`, `sharpen` |
| Path-based params     | ✅ Complete | `params.rs`      | `/w:800,h:600,q:80,f:webp/` format                              |
| DPR support           | ✅ Complete | `processor.rs`   | Device pixel ratio 1-4                                          |
| Percentage dimensions | ✅ Complete | `params.rs`      | `50p` or `50%` syntax                                           |
| Config                | ✅ Complete | `config.rs`      | max_width, max_height, default_quality                          |
| Error types           | ✅ Complete | `error.rs`       | Structured errors with HTTP status mapping                      |
| URL signing           | ✅ Complete | `security.rs`    | HMAC-SHA256 signature generation/validation                     |
| Image bomb protection | ✅ Complete | `security.rs`    | Dimension/pixel limits                                          |
| Source validation     | ✅ Complete | `security.rs`    | Allowlist/blocklist with glob patterns                          |
| Auto-format           | ✅ Complete | `format.rs`      | Accept header parsing, format selection                         |
| Encoder abstraction   | ✅ Complete | `encoder.rs`     | Trait-based with factory pattern                                |
| Cache variant support | ✅ Complete | `cache/entry.rs` | `CacheKey.variant` field added                                  |

### Remaining Work

| Feature                                    | Priority | Status                                            |
| ------------------------------------------ | -------- | ------------------------------------------------- |
| Smart/content-aware crop                   | Medium   | ✅ Complete (entropy-based)                       |
| Advanced encoders (mozjpeg, oxipng, ravif) | Medium   | ✅ Complete                                       |
| Rotation/flip implementation               | Medium   | ✅ Complete                                       |
| Blur/sharpen implementation                | Low      | ❌ Pending                                        |
| Watermarking                               | Low      | ❌ Pending                                        |
| Metrics & observability                    | Medium   | ✅ Complete (struct defined, integration pending) |
| Full cache integration                     | Medium   | ✅ Complete (variant field in CacheKey)           |

---

## Feature Requirements

### 1. URL Format

Support two URL patterns:

#### Pattern A: Query Parameters (Implemented)

```
GET /bucket/path/image.jpg?w=800&h=600&q=80&fmt=webp
```

#### Pattern B: Path-based (Implemented)

```
GET /bucket/w:800,h:600,q:80,f:webp/image.jpg
```

**Options Format**: Comma-separated key-value pairs

```
w:800,h:600,q:80,f:webp,fit:cover
```

### 2. Transformation Operations

#### 2.1 Resize Operations (Implemented)

| Parameter | Description                         | Example                                | Status |
| --------- | ----------------------------------- | -------------------------------------- | ------ |
| `w`       | Target width (pixels or percentage) | `w:800`, `w:50p`                       | ✅     |
| `h`       | Target height                       | `h:600`                                | ✅     |
| `fit`     | Fit mode                            | `fit:cover`, `fit:contain`, `fit:fill` | ✅     |
| `dpr`     | Device pixel ratio (1-4)            | `dpr:2`                                | ✅     |
| `enlarge` | Allow upscaling                     | `enlarge:1`                            | ✅     |

#### 2.2 Crop Operations (Complete)

| Parameter  | Description     | Example                          | Status                            |
| ---------- | --------------- | -------------------------------- | --------------------------------- |
| `gravity`  | Crop position   | `g:center`, `g:north`, `g:smart` | ✅                                |
| `cx`, `cy` | Crop offset     | `cx:100,cy:50`                   | ✅                                |
| `cw`, `ch` | Crop dimensions | `cw:400,ch:300`                  | ✅                                |
| `crop`     | Smart crop      | `crop:smart`                     | ✅ (via `g:smart`, entropy-based) |

#### 2.3 Format & Quality (Implemented)

| Parameter     | Description             | Example                               | Status    |
| ------------- | ----------------------- | ------------------------------------- | --------- |
| `f` / `fmt`   | Output format           | `f:webp`, `f:avif`, `f:jpeg`, `f:png` | ✅        |
| `q`           | Quality (1-100)         | `q:80`                                | ✅        |
| `auto`        | Auto-format from Accept | Automatic                             | ✅        |
| `strip`       | Strip metadata          | `strip:1`                             | ✅ Parsed |
| `progressive` | Progressive encoding    | `progressive:1`                       | ✅ Parsed |

#### 2.4 Effects

| Parameter      | Description           | Example                  | Status                             |
| -------------- | --------------------- | ------------------------ | ---------------------------------- |
| `r` / `rotate` | Rotation (degrees)    | `r:90`, `r:180`, `r:270` | ✅ Complete                        |
| `flip`         | Flip direction        | `flip:h`, `flip:v`       | ✅ Complete                        |
| `blur`         | Gaussian blur (sigma) | `blur:5`                 | ✅ Parsed (implementation pending) |
| `sharpen`      | Sharpen (sigma)       | `sharpen:1.5`            | ✅ Parsed (implementation pending) |

### 3. Security Features (Implemented)

#### 3.1 URL Signing (HMAC-SHA256)

```
signature = HMAC-SHA256(secret_key, salt + options + "/" + source_url)
```

**Implementation**: `src/image_optimizer/security.rs`

- `generate_signature()` - creates URL signature
- `validate_signature()` - verifies signature with constant-time comparison
- Optional salt support
- Configurable `signing_required` flag

#### 3.2 Image Bomb Protection

**Implementation**: `src/image_optimizer/security.rs`

- `validate_dimensions()` - checks width, height, and total pixels
- `validate_file_size()` - checks input file size

```rust
pub struct SecurityConfig {
    pub signing_required: bool,
    pub signing_key: Option<Vec<u8>>,
    pub signing_salt: Option<Vec<u8>>,
    pub max_source_width: u32,        // Default: 10,000
    pub max_source_height: u32,       // Default: 10,000
    pub max_source_pixels: u64,       // Default: 100,000,000 (100MP)
    pub max_source_file_size: usize,  // Default: 50MB
    pub allowed_sources: Vec<String>,
    pub blocked_sources: Vec<String>,
}
```

#### 3.3 Source Validation

**Implementation**: `src/image_optimizer/security.rs`

- `validate_source()` - checks against allowed/blocked lists
- Glob pattern support (`bucket/*`, `*.example.com`)

### 4. Auto-Format Selection (Implemented)

**Implementation**: `src/image_optimizer/format.rs`

Based on `Accept` header with fallback chain:

```
Accept: image/avif,image/webp,image/jpeg
```

Selection priority:

1. AVIF (if supported and `prefer_avif` enabled)
2. WebP (if supported and `prefer_webp` enabled)
3. Original format (JPEG/PNG)

**Configuration**:

```rust
pub struct AutoFormatConfig {
    pub enabled: bool,
    pub prefer_avif: bool,
    pub prefer_webp: bool,
    pub min_savings_percent: u8,
}
```

**Functions**:

- `select_format()` - chooses optimal format based on Accept header
- `vary_header()` - returns "Accept" for Vary header

### 5. Encoder Architecture (Implemented)

**Implementation**: `src/image_optimizer/encoder.rs`

#### Trait-Based Design

```rust
pub trait ImageEncoder: Send + Sync {
    fn format(&self) -> OutputFormat;
    fn encode(&self, data: &[u8], width: u32, height: u32, quality: EncoderQuality)
        -> Result<EncodedImage, ImageError>;
    fn supports_transparency(&self) -> bool;
}
```

#### Factory Pattern

```rust
pub struct EncoderFactory;

impl EncoderFactory {
    pub fn create(format: OutputFormat) -> Box<dyn ImageEncoder>;
}
```

#### Current Encoders

| Format | Encoder       | Notes                                  |
| ------ | ------------- | -------------------------------------- |
| JPEG   | `JpegEncoder` | Uses `image` crate                     |
| PNG    | `PngEncoder`  | Uses `image` crate                     |
| WebP   | `WebPEncoder` | Lossless only (image crate limitation) |
| AVIF   | `AvifEncoder` | Placeholder, returns error             |

### 6. Error Handling (Implemented)

**Implementation**: `src/image_optimizer/error.rs`

```rust
pub enum ImageError {
    // Decoding errors
    UnsupportedFormat { format: String },
    DecodeFailed { message: String },
    CorruptedImage { message: String },

    // Processing errors
    ResizeFailed { message: String },
    EncodeFailed { format: String, message: String },
    ProcessingTimeout { timeout_ms: u64 },

    // Security errors
    InvalidSignature,
    SourceNotAllowed { source: String },
    ImageBombDetected { width, height, pixels, max_pixels },
    FileTooLarge { size, max_size },

    // Parameter errors
    InvalidParameter { param: String, message: String },
    InvalidDimensions { width, height, reason: String },
    InvalidQuality { quality: u8 },
}

impl ImageError {
    pub fn to_http_status(&self) -> u16 {
        match self {
            Self::UnsupportedFormat { .. } => 415,
            Self::InvalidSignature | Self::SourceNotAllowed { .. } => 403,
            Self::FileTooLarge { .. } => 413,
            Self::ProcessingTimeout { .. } => 504,
            Self::DecodeFailed { .. } | Self::ImageBombDetected { .. } |
            Self::InvalidParameter { .. } | Self::InvalidDimensions { .. } |
            Self::InvalidQuality { .. } => 400,
            _ => 500,
        }
    }
}
```

---

## Module Structure (Current)

```
src/image_optimizer/
├── mod.rs          # Module root, public API exports
├── config.rs       # ImageConfig configuration
├── error.rs        # ImageError enum with HTTP status mapping
├── params.rs       # ImageParams parsing (query and path-based)
├── processor.rs    # Main processing pipeline
├── encoder.rs      # Encoder trait, factory, implementations
├── format.rs       # Auto-format selection from Accept header
└── security.rs     # URL signing, image bomb protection, source validation
```

---

## API Summary

### Public Types

```rust
// Core types
pub use error::ImageError;
pub use params::{Dimension, FitMode, Gravity, ImageParams, OutputFormat};
pub use config::ImageConfig;

// Encoder types
pub use encoder::{EncodedImage, EncoderFactory, EncoderQuality, ImageEncoder};

// Security types
pub use security::SecurityConfig;

// Format selection
pub use format::{AutoFormatConfig, select_format, vary_header};

// Processing
pub use processor::{process_image, process_image_internal, ProcessedImage};
```

### Key Functions

```rust
// Parse parameters from query map
pub fn ImageParams::from_params(params: &HashMap<String, String>) -> Option<Self>;
pub fn ImageParams::from_query(params: &HashMap<String, String>) -> Option<Result<Self, ImageError>>;
pub fn ImageParams::from_path_options(options: &str) -> Result<Self, ImageError>;

// Process image
pub fn process_image(data: &[u8], params: ImageParams) -> Result<(Vec<u8>, String), String>;

// Security
pub fn generate_signature(options: &str, source_url: &str, config: &SecurityConfig) -> Option<String>;
pub fn validate_signature(sig: &str, options: &str, source_url: &str, config: &SecurityConfig) -> Result<(), ImageError>;
pub fn validate_dimensions(width: u32, height: u32, config: &SecurityConfig) -> Result<(), ImageError>;
pub fn validate_source(source: &str, config: &SecurityConfig) -> Result<(), ImageError>;

// Format selection
pub fn select_format(accept: Option<&str>, source: OutputFormat, has_transparency: bool, config: &AutoFormatConfig) -> OutputFormat;
```

---

## Test Coverage

| Module         | Tests  | Status  |
| -------------- | ------ | ------- |
| `error.rs`     | 10     | ✅ Pass |
| `params.rs`    | 14     | ✅ Pass |
| `processor.rs` | 10     | ✅ Pass |
| `encoder.rs`   | 13     | ✅ Pass |
| `format.rs`    | 11     | ✅ Pass |
| `security.rs`  | 13     | ✅ Pass |
| **Total**      | **71** | ✅ Pass |

---

## Next Steps

1. ~~**Phase 50.1**: Integrate optimized encoders (mozjpeg, oxipng, ravif)~~ ✅ Complete
2. ~~**Phase 50.2**: Implement smart crop and advanced resize operations~~ ✅ Complete
3. **Phase 50.3**: Implement blur and sharpen effects (rotation/flip done)
4. ~~**Phase 50.6**: Full cache integration with variant storage~~ ✅ Complete
5. **Phase 50.7**: Wire Prometheus metrics to global registry (struct defined)

---

## Performance Targets

| Metric                       | Target     | Notes                  |
| ---------------------------- | ---------- | ---------------------- |
| Resize 1000x1000 → 500x500   | <50ms      | JPEG output            |
| Format convert (JPEG → WebP) | <100ms     | 1MP image              |
| AVIF encoding                | <500ms     | 1MP image, speed=6     |
| Throughput                   | >100 req/s | Per CPU core           |
| Memory per image             | <50MB      | Peak during processing |
| Cache hit response           | <10ms      | P95                    |

---

## Compatibility

- Maintains backward compatibility with existing query parameter API
- New path-based API works alongside query parameters
- `from_params()` method preserved for legacy code
- No breaking changes to existing configuration
