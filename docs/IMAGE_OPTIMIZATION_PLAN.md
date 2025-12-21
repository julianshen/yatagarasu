# Image Optimization Implementation Plan

**Status**: In Progress (Foundation Complete)
**Phases**: 50.1 - 50.8
**Estimated Effort**: 10-12 weeks
**Dependencies**: Compression feature (merged)
**Last Updated**: December 2025

---

## Phase Overview

| Phase | Name | Focus | Tests | Status |
|-------|------|-------|-------|--------|
| 50.0 | Foundation | Error types, params, encoder abstraction | 47 | ✅ Complete |
| 50.1 | Enhanced Encoders | mozjpeg, oxipng, ravif integration | 25+ | ⏳ Pending |
| 50.2 | Advanced Resize & Crop | Smart crop, gravity, DPR | 20+ | ⏳ Pending |
| 50.3 | Transformations | Rotate, flip, blur, sharpen | 15+ | ⏳ Pending |
| 50.4 | Auto-Format | Accept header negotiation | 11 | ✅ Complete |
| 50.5 | URL Signing & Security | HMAC, image bomb protection | 13 | ✅ Complete |
| 50.6 | Cache Integration | Variant caching, purge | 15+ | 🔄 Partial |
| 50.7 | Metrics & Observability | Prometheus, logging | 10+ | ⏳ Pending |
| 50.8 | Testing & Documentation | Integration tests, docs | 20+ | ⏳ Pending |

**Current Total**: 71 test cases passing
**Target**: 140+ test cases

---

## Phase 50.0: Foundation ✅ COMPLETE

### Objective
Establish core module structure with error handling, parameter parsing, encoder abstraction, and basic processor.

### Completed Tasks

#### 50.0.1 Error Types ✅
- [x] Create `ImageError` enum with all variants
- [x] Implement HTTP status code mapping
- [x] Add error constructors with meaningful messages
- [x] **Tests**: 10 passing

#### 50.0.2 Parameter Parsing ✅
- [x] Define `ImageParams` struct
- [x] Parse from query parameters (`w`, `h`, `q`, `fmt`, `fit`, etc.)
- [x] Parse from path-based options (`w:800,h:600,q:80`)
- [x] Support `Dimension` type (pixels and percentage)
- [x] Support all `FitMode` variants
- [x] Support `Gravity` options
- [x] Backward-compatible `from_params()` method
- [x] **Tests**: 14 passing

#### 50.0.3 Encoder Abstraction ✅
- [x] Create `ImageEncoder` trait
- [x] Create `EncoderFactory` with factory pattern
- [x] Implement `JpegEncoder` (image crate)
- [x] Implement `PngEncoder` (image crate)
- [x] Implement `WebPEncoder` (lossless only)
- [x] Add `AvifEncoder` placeholder
- [x] Define `EncoderQuality` and `EncodedImage` types
- [x] **Tests**: 13 passing

#### 50.0.4 Basic Processor ✅
- [x] Decode image from bytes
- [x] Calculate target dimensions with DPR
- [x] Resize using `fast_image_resize` with Lanczos3
- [x] Encode to target format
- [x] Return `ProcessedImage` with metadata
- [x] **Tests**: 10 passing

### Module Structure (Actual)
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

## Phase 50.1: Enhanced Encoders

### Objective
Replace basic `image` crate encoders with optimized alternatives for better compression and quality.

### Tasks

#### 50.1.1 MozJPEG Integration
- [ ] Add `mozjpeg` crate dependency
- [ ] Implement `MozJpegEncoder` struct
- [ ] Support quality (1-100)
- [ ] Support progressive encoding
- [ ] Support chroma subsampling (4:4:4, 4:2:2, 4:2:0)
- [ ] Benchmark vs image crate JPEG

#### 50.1.2 Oxipng Integration
- [ ] Add `oxipng` crate dependency
- [ ] Implement `OxipngEncoder` struct
- [ ] Support compression levels (0-6)
- [ ] Support metadata stripping
- [ ] Support alpha optimization
- [ ] Benchmark vs image crate PNG

#### 50.1.3 WebP Encoder Enhancement
- [ ] Improve `webp` crate usage
- [ ] Support lossless mode
- [ ] Support near-lossless mode
- [ ] Support alpha quality

#### 50.1.4 AVIF/ravif Integration
- [ ] Add `ravif` crate dependency
- [ ] Implement `RavifEncoder` struct
- [ ] Support quality (1-100)
- [ ] Support speed (1-10)
- [ ] Handle slow encoding gracefully (timeout)

#### 50.1.5 Encoder Configuration
- [ ] Create `EncoderConfig` struct per format
- [ ] Add encoder selection to `ImageConfig`
- [ ] Implement encoder factory pattern

### Test Cases

```
[ ] test_mozjpeg_encodes_valid_jpeg
[ ] test_mozjpeg_quality_affects_size
[ ] test_mozjpeg_progressive_encoding
[ ] test_mozjpeg_chroma_subsampling
[ ] test_oxipng_encodes_valid_png
[ ] test_oxipng_compression_levels
[ ] test_oxipng_strips_metadata
[ ] test_oxipng_alpha_optimization
[ ] test_webp_lossy_encoding
[ ] test_webp_lossless_encoding
[ ] test_webp_near_lossless_encoding
[ ] test_ravif_encodes_valid_avif
[ ] test_ravif_quality_affects_size
[ ] test_ravif_speed_affects_time
[ ] test_encoder_config_defaults
[ ] test_encoder_config_validation
[ ] test_encoder_factory_returns_correct_encoder
[ ] test_encoder_fallback_on_error
[ ] test_mozjpeg_vs_image_compression_ratio
[ ] test_oxipng_vs_image_compression_ratio
[ ] test_encoder_roundtrip_preserves_quality
```

### Dependencies
```toml
mozjpeg = "0.10"
oxipng = "9.0"
ravif = "0.11"
```

---

## Phase 50.2: Advanced Resize & Crop

### Objective
Implement smart cropping, gravity-based positioning, and DPR support.

### Tasks

#### 50.2.1 Enhanced Resize
- [ ] Add DPR (device pixel ratio) support
- [ ] Add percentage-based dimensions (`50p`)
- [ ] Add `enlarge` option (allow upscaling)
- [ ] Improve aspect ratio calculation

#### 50.2.2 Crop Positioning
- [ ] Implement gravity system (center, north, south, east, west, ne, nw, se, sw)
- [ ] Implement manual crop offset (cx, cy)
- [ ] Implement crop dimensions (cw, ch)
- [ ] Add focal point support (fp-x, fp-y)

#### 50.2.3 Smart Crop (Basic)
- [ ] Implement edge detection based crop
- [ ] Implement entropy-based crop (focus on detailed areas)
- [ ] Add `crop:smart` parameter
- [ ] Add `crop:attention` (alias for smart)

#### 50.2.4 Fit Mode Enhancements
- [ ] Implement `fit:pad` (add padding to maintain ratio)
- [ ] Implement background color for padding
- [ ] Improve `fit:inside` and `fit:outside` accuracy

### Test Cases

```
[ ] test_resize_with_dpr_2x
[ ] test_resize_with_dpr_3x
[ ] test_resize_percentage_width
[ ] test_resize_percentage_height
[ ] test_resize_enlarge_disabled_by_default
[ ] test_resize_enlarge_when_enabled
[ ] test_crop_gravity_center
[ ] test_crop_gravity_north
[ ] test_crop_gravity_southeast
[ ] test_crop_manual_offset
[ ] test_crop_manual_dimensions
[ ] test_crop_focal_point
[ ] test_smart_crop_detects_subject
[ ] test_entropy_crop_favors_detail
[ ] test_fit_pad_adds_background
[ ] test_fit_pad_custom_color
[ ] test_fit_inside_never_exceeds
[ ] test_fit_outside_covers_fully
```

---

## Phase 50.3: Transformations

### Objective
Add rotation, flip, blur, sharpen, and basic color adjustments.

### Tasks

#### 50.3.1 Rotation
- [ ] Implement 90° rotation
- [ ] Implement 180° rotation
- [ ] Implement 270° rotation
- [ ] Handle arbitrary rotation (optional)
- [ ] Auto-rotate based on EXIF (optional)

#### 50.3.2 Flip
- [ ] Implement horizontal flip
- [ ] Implement vertical flip
- [ ] Combine flip with rotation

#### 50.3.3 Filters
- [ ] Implement Gaussian blur (sigma parameter)
- [ ] Implement unsharp mask / sharpen
- [ ] Clamp parameter ranges for safety

#### 50.3.4 Color Adjustments (Basic)
- [ ] Implement brightness adjustment
- [ ] Implement contrast adjustment
- [ ] Implement saturation adjustment (optional)

### Test Cases

```
[ ] test_rotate_90_clockwise
[ ] test_rotate_180
[ ] test_rotate_270_clockwise
[ ] test_flip_horizontal
[ ] test_flip_vertical
[ ] test_flip_both_equals_rotate_180
[ ] test_blur_sigma_0_no_change
[ ] test_blur_sigma_5_visible_blur
[ ] test_sharpen_increases_edges
[ ] test_brightness_increase
[ ] test_brightness_decrease
[ ] test_contrast_increase
[ ] test_combined_transformations_order
[ ] test_rotation_preserves_dimensions_correctly
[ ] test_filter_parameters_clamped
```

---

## Phase 50.4: Auto-Format Selection ✅ COMPLETE

### Objective
Automatically select optimal output format based on Accept header and content.

### Completed Tasks

#### 50.4.1 Accept Header Parsing ✅
- [x] Parse Accept header for image types
- [x] Extract quality values (q=0.9)
- [x] Handle wildcards (image/*)
- [x] Build preference list sorted by quality

#### 50.4.2 Format Selection Logic ✅
- [x] Implement format selection algorithm
- [x] Consider source format (preserve transparency)
- [x] Consider browser support (AVIF > WebP > JPEG)
- [x] Apply format preferences from config

#### 50.4.3 Configuration ✅
- [x] Add `auto_format.enabled` config
- [x] Add `auto_format.prefer_avif` config
- [x] Add `auto_format.prefer_webp` config
- [x] Add `auto_format.min_savings_percent` config

#### 50.4.4 Response Headers ✅
- [x] Add `Vary: Accept` header via `vary_header()` function
- [x] Content-Type based on output format
- [ ] Debug header with format decision (optional - deferred)

### Completed Test Cases (11 tests)

```
[x] test_parse_accept_header_simple
[x] test_parse_accept_header_with_quality
[x] test_parse_accept_header_sorted_by_quality
[x] test_select_format_avif_preferred
[x] test_select_format_webp_fallback
[x] test_select_format_preserve_transparency
[x] test_select_format_no_accept_header
[x] test_select_format_disabled
[x] test_is_format_acceptable
[x] test_is_format_acceptable_wildcard
[x] test_format_supports_transparency
```

### Implementation
**File**: `src/image_optimizer/format.rs`

---

## Phase 50.5: URL Signing & Security ✅ COMPLETE

### Objective
Implement HMAC-SHA256 URL signing and image bomb protection.

### Completed Tasks

#### 50.5.1 URL Signing ✅
- [x] Implement HMAC-SHA256 signature generation
- [x] Implement signature validation with constant-time comparison
- [x] Support optional salt
- [x] Add `signing_required` config option
- [x] Base64url encoding (URL-safe, no padding)

#### 50.5.2 Path-based URL Support ✅
- [x] Parse options from path (`w:800,h:600,q:80`)
- [x] Query parameter parsing also supported
- [ ] `/_img/{sig}/{options}/{url}` route pattern (proxy integration pending)

#### 50.5.3 Image Bomb Protection ✅
- [x] Validate max_source_width (default: 10,000)
- [x] Validate max_source_height (default: 10,000)
- [x] Check pixel count limit (default: 100MP)
- [x] Check file size limit (default: 50MB)
- [x] Return appropriate `ImageError` variants
- [ ] Processing timeout (deferred to metrics phase)

#### 50.5.4 Source Validation ✅
- [x] Implement allowed sources list
- [x] Implement blocked sources list
- [x] Glob pattern matching (prefix/suffix wildcards)
- [x] Validate before processing

### Completed Test Cases (13 tests)

```
[x] test_generate_signature
[x] test_validate_signature_success
[x] test_validate_signature_failure
[x] test_validate_signature_not_required
[x] test_validate_dimensions_ok
[x] test_validate_dimensions_width_exceeded
[x] test_validate_dimensions_pixels_exceeded
[x] test_validate_file_size_ok
[x] test_validate_file_size_exceeded
[x] test_validate_source_allowed
[x] test_validate_source_not_allowed
[x] test_validate_source_blocked
[x] test_glob_match
[x] test_constant_time_compare
```

### Implementation
**File**: `src/image_optimizer/security.rs`

### Security Configuration
```rust
pub struct SecurityConfig {
    pub signing_required: bool,
    pub signing_key: Option<Vec<u8>>,
    pub signing_salt: Option<Vec<u8>>,
    pub max_source_width: u32,        // Default: 10,000
    pub max_source_height: u32,       // Default: 10,000
    pub max_source_pixels: u64,       // Default: 100,000,000
    pub max_source_file_size: usize,  // Default: 50MB
    pub allowed_sources: Vec<String>,
    pub blocked_sources: Vec<String>,
}
```

---

## Phase 50.6: Cache Integration 🔄 PARTIAL

### Objective
Integrate image optimization with existing cache layer.

### Tasks

#### 50.6.1 Cache Key Generation 🔄
- [x] Add `variant` field to `CacheKey` struct
- [ ] Generate deterministic cache keys from params
- [ ] Include format in cache key
- [ ] Include quality in cache key
- [ ] Handle auto-format cache variants

#### 50.6.2 Cache Storage
- [ ] Store optimized images in cache
- [ ] Retrieve from cache before processing
- [ ] Respect cache TTL from source
- [ ] Add image-specific cache headers

#### 50.6.3 Cache Invalidation
- [ ] Purge by source URL (all variants)
- [ ] Purge by specific variant
- [ ] Integration with existing purge API

#### 50.6.4 Cache Headers
- [ ] Set appropriate Cache-Control
- [ ] Set ETag based on content hash
- [ ] Handle conditional requests (If-None-Match)

### Completed Work
**File**: `src/cache/entry.rs`
- Added `variant: Option<String>` field to `CacheKey` struct

### Test Cases

```
[ ] test_cache_key_deterministic
[ ] test_cache_key_includes_params
[ ] test_cache_key_different_for_formats
[ ] test_cache_hit_skips_processing
[ ] test_cache_miss_triggers_processing
[ ] test_cache_stores_after_processing
[ ] test_cache_purge_by_source
[ ] test_cache_purge_specific_variant
[ ] test_cache_control_header_set
[ ] test_etag_header_set
[ ] test_conditional_request_304
[ ] test_auto_format_caches_separately
```

---

## Phase 50.7: Metrics & Observability

### Objective
Add Prometheus metrics and structured logging for image operations.

### Tasks

#### 50.7.1 Prometheus Metrics
- [ ] Processing duration histogram
- [ ] Transformation counters (by type)
- [ ] Error counters (by type)
- [ ] Bytes saved gauge
- [ ] Compression ratio histogram
- [ ] Cache hit/miss counters

#### 50.7.2 Logging
- [ ] Structured log for each operation
- [ ] Include source, dimensions, format, duration
- [ ] Include compression ratio
- [ ] Log errors with context

#### 50.7.3 Debug Headers (Optional)
- [ ] Add `X-Image-Processing-Time` header
- [ ] Add `X-Image-Original-Size` header
- [ ] Add `X-Image-Format-Selected` header

### Test Cases

```
[ ] test_metrics_duration_recorded
[ ] test_metrics_transformation_counted
[ ] test_metrics_error_counted
[ ] test_metrics_bytes_saved_calculated
[ ] test_metrics_compression_ratio_recorded
[ ] test_metrics_cache_hit_counted
[ ] test_log_contains_required_fields
[ ] test_debug_headers_present_when_enabled
```

---

## Phase 50.8: Testing & Documentation

### Objective
Comprehensive integration tests and user documentation.

### Tasks

#### 50.8.1 Integration Tests
- [ ] End-to-end resize test
- [ ] End-to-end format conversion test
- [ ] End-to-end signed URL test
- [ ] Load test with concurrent requests
- [ ] Memory usage test with large images

#### 50.8.2 Benchmark Suite
- [ ] Benchmark resize performance
- [ ] Benchmark format conversion
- [ ] Benchmark encoder comparison
- [ ] Benchmark cache hit vs miss

#### 50.8.3 Documentation
- [ ] Update README with image optimization
- [ ] Create IMAGE_OPTIMIZATION.md user guide
- [ ] Create configuration reference
- [ ] Add examples for common use cases

#### 50.8.4 Migration Guide
- [ ] Document upgrade path from current implementation
- [ ] List breaking changes (if any)
- [ ] Provide example configurations

### Test Cases

```
[ ] test_e2e_resize_jpeg_to_webp
[ ] test_e2e_signed_url_flow
[ ] test_e2e_auto_format_selection
[ ] test_e2e_cache_integration
[ ] test_e2e_error_handling
[ ] test_concurrent_processing
[ ] test_memory_usage_large_image
[ ] test_memory_usage_many_small_images
[ ] bench_resize_1mp_image
[ ] bench_mozjpeg_vs_image_crate
[ ] bench_cache_hit_latency
```

---

## Module Structure

### Current Implementation (Flat Structure)
```
src/image_optimizer/
├── mod.rs          # Module root, public API exports
├── config.rs       # ImageConfig configuration
├── error.rs        # ImageError enum with HTTP status mapping
├── params.rs       # ImageParams parsing (query and path-based)
├── processor.rs    # Main processing pipeline (decode → resize → encode)
├── encoder.rs      # ImageEncoder trait, factory, JPEG/PNG/WebP/AVIF encoders
├── format.rs       # Auto-format selection from Accept header
└── security.rs     # URL signing, image bomb protection, source validation
```

### Planned Expansion (Phase 50.1+)
```
src/image_optimizer/
├── ... (existing modules)
├── encoders/
│   ├── mod.rs              # Enhanced encoder module
│   ├── mozjpeg.rs          # MozJPEG encoder
│   ├── oxipng.rs           # Oxipng encoder
│   └── ravif.rs            # AVIF encoder
├── operations/
│   ├── mod.rs              # Operation trait
│   ├── crop.rs             # Crop operations
│   ├── transform.rs        # Rotate, flip
│   └── filters.rs          # Blur, sharpen
├── cache.rs                # Cache integration
└── metrics.rs              # Prometheus metrics
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| AVIF encoding too slow | Medium | Medium | Configurable speed/quality tradeoff, timeout |
| Memory pressure with large images | Medium | High | Strict limits, streaming where possible |
| mozjpeg build complexity | Low | Medium | Fallback to image crate encoder |
| Smart crop accuracy | Medium | Low | Provide manual crop as fallback |
| Cache bloat from variants | Medium | Medium | LRU eviction, variant limits |

---

## Success Criteria

1. ✅ All 6 core operations working (resize, crop, rotate, flip, format, quality)
2. ✅ Enhanced encoders (mozjpeg, oxipng, ravif) integrated
3. ✅ Auto-format selection from Accept header
4. ✅ URL signing for security
5. ✅ Image bomb protection
6. ✅ Cache integration with variant storage
7. ✅ Prometheus metrics
8. ✅ >90% test coverage
9. ✅ Performance targets met
10. ✅ Documentation complete

---

## Next Steps

### Completed ✅
1. ✅ Foundation complete (Phase 50.0)
2. ✅ Auto-format selection (Phase 50.4)
3. ✅ URL signing & security (Phase 50.5)
4. ✅ CacheKey.variant field added (Phase 50.6 partial)

### Next Up
1. **Phase 50.1**: Add enhanced encoders (mozjpeg, oxipng, ravif)
2. **Phase 50.2**: Smart crop and advanced resize operations
3. **Phase 50.3**: Rotation, flip, blur, sharpen
4. **Phase 50.6**: Complete cache integration
5. **Phase 50.7**: Prometheus metrics and structured logging

### TDD Workflow
- Follow Red → Green → Refactor cycle
- Mark tests complete as implemented
- Commit with [BEHAVIORAL]/[STRUCTURAL] prefixes

---

**Ready to continue? Say "go" to begin Phase 50.1 (Enhanced Encoders)!**
