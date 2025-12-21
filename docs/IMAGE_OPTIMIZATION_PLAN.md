# Image Optimization Implementation Plan

**Status**: Ready for Implementation
**Phases**: 50.1 - 50.8
**Estimated Effort**: 10-12 weeks
**Dependencies**: Compression feature (merged)

---

## Phase Overview

| Phase | Name | Focus | Tests | Duration |
|-------|------|-------|-------|----------|
| 50.1 | Enhanced Encoders | mozjpeg, oxipng, ravif integration | 25+ | 1.5 weeks |
| 50.2 | Advanced Resize & Crop | Smart crop, gravity, DPR | 20+ | 1 week |
| 50.3 | Transformations | Rotate, flip, blur, sharpen | 15+ | 1 week |
| 50.4 | Auto-Format | Accept header negotiation | 15+ | 1 week |
| 50.5 | URL Signing & Security | HMAC, image bomb protection | 20+ | 1.5 weeks |
| 50.6 | Cache Integration | Variant caching, purge | 15+ | 1 week |
| 50.7 | Metrics & Observability | Prometheus, logging | 10+ | 0.5 weeks |
| 50.8 | Testing & Documentation | Integration tests, docs | 20+ | 1.5 weeks |

**Total**: 140+ test cases

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

## Phase 50.4: Auto-Format Selection

### Objective
Automatically select optimal output format based on Accept header and content.

### Tasks

#### 50.4.1 Accept Header Parsing
- [ ] Parse Accept header for image types
- [ ] Extract quality values (q=0.9)
- [ ] Handle wildcards (image/*)
- [ ] Build preference list

#### 50.4.2 Format Selection Logic
- [ ] Implement format selection algorithm
- [ ] Consider source format (preserve transparency)
- [ ] Consider browser support
- [ ] Consider file size benefit threshold

#### 50.4.3 Configuration
- [ ] Add `auto_format.enabled` config
- [ ] Add `auto_format.prefer_avif` config
- [ ] Add `auto_format.prefer_webp` config
- [ ] Add `auto_format.min_savings_percent` config

#### 50.4.4 Response Headers
- [ ] Add `Vary: Accept` header
- [ ] Add `Content-Type` based on output
- [ ] Add debug header with format decision (optional)

### Test Cases

```
[ ] test_accept_header_parse_single
[ ] test_accept_header_parse_multiple
[ ] test_accept_header_parse_with_quality
[ ] test_accept_header_parse_wildcard
[ ] test_format_selection_prefers_avif
[ ] test_format_selection_falls_back_to_webp
[ ] test_format_selection_preserves_png_transparency
[ ] test_format_selection_respects_min_savings
[ ] test_format_selection_disabled
[ ] test_vary_header_present
[ ] test_content_type_matches_output
[ ] test_format_explicit_overrides_auto
```

---

## Phase 50.5: URL Signing & Security

### Objective
Implement HMAC-SHA256 URL signing and image bomb protection.

### Tasks

#### 50.5.1 URL Signing
- [ ] Implement HMAC-SHA256 signature generation
- [ ] Implement signature validation
- [ ] Support optional salt
- [ ] Add `signing_required` config option
- [ ] Generate signed URLs helper

#### 50.5.2 Path-based URL Support
- [ ] Implement `/_img/{sig}/{options}/{url}` pattern
- [ ] Parse options from path
- [ ] Decode source URL (base64 or plain)
- [ ] Route to image processor

#### 50.5.3 Image Bomb Protection
- [ ] Validate dimensions before full decode
- [ ] Check pixel count limit
- [ ] Check file size limit
- [ ] Implement processing timeout
- [ ] Return appropriate error responses

#### 50.5.4 Source Validation
- [ ] Implement allowed sources list
- [ ] Implement blocked sources list
- [ ] Glob pattern matching
- [ ] Validate before processing

### Test Cases

```
[ ] test_signature_generation
[ ] test_signature_validation_success
[ ] test_signature_validation_failure
[ ] test_signature_with_salt
[ ] test_signature_required_rejects_unsigned
[ ] test_signature_optional_allows_unsigned
[ ] test_path_url_parsing
[ ] test_path_options_parsing
[ ] test_image_bomb_width_exceeded
[ ] test_image_bomb_height_exceeded
[ ] test_image_bomb_pixels_exceeded
[ ] test_file_size_limit
[ ] test_processing_timeout
[ ] test_allowed_source_passes
[ ] test_blocked_source_rejected
[ ] test_source_glob_matching
[ ] test_error_returns_correct_status
```

---

## Phase 50.6: Cache Integration

### Objective
Integrate image optimization with existing cache layer.

### Tasks

#### 50.6.1 Cache Key Generation
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

```
src/image_optimizer/
├── mod.rs                  # Module root, public API
├── config.rs               # Configuration (enhance existing)
├── processor.rs            # Main processing pipeline (enhance existing)
├── params.rs               # Parameter parsing (extract from processor)
├── encoders/
│   ├── mod.rs              # Encoder trait and factory
│   ├── mozjpeg.rs          # MozJPEG encoder
│   ├── oxipng.rs           # Oxipng encoder
│   ├── webp.rs             # WebP encoder
│   ├── ravif.rs            # AVIF encoder
│   └── fallback.rs         # image crate fallback
├── operations/
│   ├── mod.rs              # Operation trait
│   ├── resize.rs           # Resize operations
│   ├── crop.rs             # Crop operations
│   ├── transform.rs        # Rotate, flip
│   └── filters.rs          # Blur, sharpen
├── security/
│   ├── mod.rs              # Security module root
│   ├── signing.rs          # URL signing
│   └── validation.rs       # Image bomb, source validation
├── format/
│   ├── mod.rs              # Format detection and selection
│   └── auto.rs             # Auto-format from Accept header
├── cache.rs                # Cache integration
├── metrics.rs              # Prometheus metrics
└── error.rs                # Error types
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

1. Review and approve spec
2. Add dependencies to Cargo.toml
3. Begin Phase 50.1 (Enhanced Encoders)
4. Follow TDD workflow: Red → Green → Refactor
5. Mark tests complete as implemented
6. Commit with [BEHAVIORAL]/[STRUCTURAL] prefixes

---

**Ready to start? Say "go" to begin Phase 50.1!**
