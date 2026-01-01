//! Response handler module for the proxy.
//!
//! This module provides response processing, caching, and optimization helpers.
//! It extracts response-related logic from the main proxy module for better
//! organization and testability.
//!
//! # Design
//!
//! Functions return data structures instead of modifying session directly.
//! This avoids borrow checker issues and keeps response handling testable.
//! The caller handles writing responses and updating metrics.

use std::time::Duration;

use bytes::Bytes;
use pingora_http::ResponseHeader;

use crate::cache::{CacheControl, CacheEntry, CacheKey};
use crate::image_optimizer::params::ImageParams;

// ============================================================================
// Constants
// ============================================================================

/// Maximum size for cacheable responses (10MB).
pub const MAX_CACHE_SIZE: usize = 10 * 1024 * 1024;

/// Default cache TTL when no Cache-Control header is present (1 hour).
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600);

// ============================================================================
// Result Types
// ============================================================================

/// Headers captured from upstream response for caching.
#[derive(Debug, Clone, Default)]
pub struct CapturedHeaders {
    /// Content-Type header value.
    pub content_type: Option<String>,
    /// ETag header value (cleaned of quotes).
    pub etag: Option<String>,
    /// Last-Modified header value.
    pub last_modified: Option<String>,
    /// Cache-Control header value.
    pub cache_control: Option<String>,
}

impl CapturedHeaders {
    /// Create new empty captured headers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the parsed Cache-Control directives.
    pub fn cache_control_parsed(&self) -> CacheControl {
        self.cache_control
            .as_ref()
            .map(|s| CacheControl::parse(s))
            .unwrap_or_default()
    }

    /// Check if the response should be cached based on Cache-Control.
    pub fn should_cache(&self) -> bool {
        self.cache_control_parsed().should_store()
    }

    /// Get the effective TTL for caching.
    pub fn effective_ttl(&self, default: Duration) -> Duration {
        self.cache_control_parsed().effective_ttl(default)
    }
}

/// Result of buffering a response chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferResult {
    /// Chunk was successfully buffered.
    Buffered {
        /// New total buffer size after adding chunk.
        total_size: usize,
    },
    /// Response exceeded max size, buffering disabled.
    ExceededMaxSize {
        /// Size that would have been reached.
        attempted_size: usize,
    },
    /// No chunk to buffer (empty body).
    Empty,
}

/// Result of cache population.
#[derive(Debug, Clone)]
pub enum CachePopulationResult {
    /// Response was cached successfully.
    Cached {
        /// The cache key used.
        key: CacheKey,
        /// TTL applied.
        ttl: Duration,
    },
    /// Caching was skipped due to Cache-Control directives.
    SkippedByDirective {
        /// The Cache-Control header value.
        cache_control: String,
    },
    /// Caching was skipped because cache is not configured.
    NotConfigured,
}

/// Result of image optimization.
#[derive(Debug, Clone)]
pub enum OptimizationResult {
    /// Image was optimized successfully.
    Optimized {
        /// The optimized image data.
        data: Vec<u8>,
        /// The content type of the optimized image.
        content_type: String,
    },
    /// Optimization failed, original data should be used.
    Failed {
        /// The error message.
        message: String,
    },
    /// No optimization was requested.
    NotRequested,
}

// ============================================================================
// Header Capture Functions
// ============================================================================

/// Capture response headers for caching from an upstream response.
///
/// Extracts Content-Type, ETag, Last-Modified, and Cache-Control headers.
/// ETag values are cleaned of surrounding quotes.
///
/// # Arguments
///
/// * `response` - The upstream response headers.
///
/// # Returns
///
/// A `CapturedHeaders` struct with all captured values.
pub fn capture_response_headers(response: &ResponseHeader) -> CapturedHeaders {
    let mut captured = CapturedHeaders::new();

    // Capture Content-Type
    if let Some(content_type) = response
        .headers
        .get("content-type")
        .or_else(|| response.headers.get("Content-Type"))
    {
        if let Ok(ct_str) = content_type.to_str() {
            captured.content_type = Some(ct_str.to_string());
        }
    }

    // Capture ETag (clean quotes)
    if let Some(etag) = response
        .headers
        .get("etag")
        .or_else(|| response.headers.get("ETag"))
    {
        if let Ok(etag_str) = etag.to_str() {
            captured.etag = Some(etag_str.trim_matches('"').to_string());
        }
    }

    // Capture Last-Modified
    if let Some(last_modified) = response
        .headers
        .get("last-modified")
        .or_else(|| response.headers.get("Last-Modified"))
    {
        if let Ok(lm_str) = last_modified.to_str() {
            captured.last_modified = Some(lm_str.to_string());
        }
    }

    // Capture Cache-Control
    if let Some(cache_control) = response
        .headers
        .get("cache-control")
        .or_else(|| response.headers.get("Cache-Control"))
    {
        if let Ok(cc_str) = cache_control.to_str() {
            captured.cache_control = Some(cc_str.to_string());
        }
    }

    captured
}

// ============================================================================
// Response Buffering Functions
// ============================================================================

/// Buffer a response chunk, checking against max cache size.
///
/// # Arguments
///
/// * `buffer` - The buffer to append to.
/// * `chunk` - The chunk to buffer.
/// * `max_size` - Maximum allowed buffer size.
///
/// # Returns
///
/// A `BufferResult` indicating success or failure.
pub fn buffer_response_chunk(buffer: &mut Vec<u8>, chunk: &[u8], max_size: usize) -> BufferResult {
    if chunk.is_empty() {
        return BufferResult::Empty;
    }

    let new_size = buffer.len() + chunk.len();

    if new_size <= max_size {
        buffer.extend_from_slice(chunk);
        BufferResult::Buffered {
            total_size: new_size,
        }
    } else {
        BufferResult::ExceededMaxSize {
            attempted_size: new_size,
        }
    }
}

// ============================================================================
// Cache Population Functions
// ============================================================================

/// Build a cache key for the response.
///
/// # Arguments
///
/// * `bucket` - The bucket name.
/// * `object_key` - The S3 object key.
/// * `variant` - Optional variant string (for optimized images).
///
/// # Returns
///
/// A `CacheKey` ready for cache operations.
pub fn build_cache_key(bucket: &str, object_key: &str, variant: Option<String>) -> CacheKey {
    CacheKey {
        bucket: bucket.to_string(),
        object_key: object_key.to_string(),
        etag: None,
        variant,
    }
}

/// Build a cache entry from captured response data.
///
/// # Arguments
///
/// * `data` - The response body data.
/// * `headers` - The captured response headers.
/// * `ttl` - The TTL for the cache entry.
///
/// # Returns
///
/// A `CacheEntry` ready to be stored.
pub fn build_cache_entry(data: Bytes, headers: &CapturedHeaders, ttl: Duration) -> CacheEntry {
    CacheEntry::new(
        data,
        headers
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string()),
        headers.etag.clone().unwrap_or_default(),
        headers.last_modified.clone(),
        Some(ttl),
    )
}

/// Check if a response should be cached based on headers.
///
/// # Arguments
///
/// * `headers` - The captured response headers.
///
/// # Returns
///
/// `true` if the response should be cached, `false` otherwise.
pub fn should_cache_response(headers: &CapturedHeaders) -> bool {
    headers.should_cache()
}

/// Get the effective TTL for a response.
///
/// # Arguments
///
/// * `headers` - The captured response headers.
///
/// # Returns
///
/// The effective TTL duration.
pub fn get_effective_ttl(headers: &CapturedHeaders) -> Duration {
    headers.effective_ttl(DEFAULT_CACHE_TTL)
}

// ============================================================================
// Image Optimization Functions
// ============================================================================

/// Process image optimization on buffered data.
///
/// # Arguments
///
/// * `data` - The original image data.
/// * `params` - The image optimization parameters.
///
/// # Returns
///
/// An `OptimizationResult` with the optimized data or error.
pub fn process_image_optimization(data: &[u8], params: &ImageParams) -> OptimizationResult {
    match crate::image_optimizer::processor::process_image(data, params.clone()) {
        Ok((optimized_data, content_type)) => OptimizationResult::Optimized {
            data: optimized_data,
            content_type,
        },
        Err(e) => OptimizationResult::Failed {
            message: e.to_string(),
        },
    }
}

/// Build a cache variant key from image parameters.
///
/// # Arguments
///
/// * `params` - The image optimization parameters.
///
/// # Returns
///
/// A string representing the variant key for caching.
pub fn build_variant_key(params: &ImageParams) -> String {
    params.to_cache_key()
}

/// Check if content type is an optimizable image type.
///
/// # Arguments
///
/// * `content_type` - The content type to check.
///
/// # Returns
///
/// `true` if the content type is an image that can be optimized.
pub fn is_optimizable_image(content_type: &str) -> bool {
    let ct_lower = content_type.to_lowercase();
    ct_lower.starts_with("image/")
        && (ct_lower.contains("jpeg")
            || ct_lower.contains("jpg")
            || ct_lower.contains("png")
            || ct_lower.contains("webp")
            || ct_lower.contains("gif")
            || ct_lower.contains("avif"))
}

// ============================================================================
// Streaming Coalescing Helpers
// ============================================================================

/// Result of broadcasting a chunk to followers.
#[derive(Debug, Clone)]
pub enum BroadcastResult {
    /// Chunk was broadcast successfully.
    Sent,
    /// No followers to broadcast to.
    NoFollowers,
    /// Broadcasting failed.
    Failed { message: String },
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_handler_module_exists() {
        // Phase 37.6 structural verification test
        let _ = CapturedHeaders::new();
        let _ = BufferResult::Empty;
        let _ = CachePopulationResult::NotConfigured;
        let _ = OptimizationResult::NotRequested;
    }

    #[test]
    fn test_captured_headers_default() {
        let headers = CapturedHeaders::new();

        assert!(headers.content_type.is_none());
        assert!(headers.etag.is_none());
        assert!(headers.last_modified.is_none());
        assert!(headers.cache_control.is_none());
    }

    #[test]
    fn test_captured_headers_should_cache_no_directive() {
        let headers = CapturedHeaders::new();

        // No Cache-Control means cacheable by default
        assert!(headers.should_cache());
    }

    #[test]
    fn test_captured_headers_should_cache_with_no_store() {
        let mut headers = CapturedHeaders::new();
        headers.cache_control = Some("no-store".to_string());

        assert!(!headers.should_cache());
    }

    #[test]
    fn test_captured_headers_should_cache_with_no_cache() {
        let mut headers = CapturedHeaders::new();
        headers.cache_control = Some("no-cache".to_string());

        // no-cache means must revalidate, but can still store
        assert!(headers.should_cache());
    }

    #[test]
    fn test_captured_headers_effective_ttl_from_max_age() {
        let mut headers = CapturedHeaders::new();
        headers.cache_control = Some("max-age=3600".to_string());

        let ttl = headers.effective_ttl(Duration::from_secs(600));
        assert_eq!(ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_captured_headers_effective_ttl_default() {
        let headers = CapturedHeaders::new();

        let ttl = headers.effective_ttl(Duration::from_secs(600));
        assert_eq!(ttl, Duration::from_secs(600));
    }

    #[test]
    fn test_buffer_response_chunk_success() {
        let mut buffer = Vec::new();
        let chunk = b"hello world";

        let result = buffer_response_chunk(&mut buffer, chunk, 100);

        assert_eq!(
            result,
            BufferResult::Buffered {
                total_size: chunk.len()
            }
        );
        assert_eq!(buffer, chunk);
    }

    #[test]
    fn test_buffer_response_chunk_exceeds_max() {
        let mut buffer = vec![0u8; 50];
        let chunk = b"hello world";

        let result = buffer_response_chunk(&mut buffer, chunk, 55);

        assert_eq!(
            result,
            BufferResult::ExceededMaxSize {
                attempted_size: 50 + chunk.len()
            }
        );
        // Buffer unchanged on failure
        assert_eq!(buffer.len(), 50);
    }

    #[test]
    fn test_buffer_response_chunk_empty() {
        let mut buffer = Vec::new();

        let result = buffer_response_chunk(&mut buffer, &[], 100);

        assert_eq!(result, BufferResult::Empty);
    }

    #[test]
    fn test_build_cache_key_without_variant() {
        let key = build_cache_key("my-bucket", "path/to/file.jpg", None);

        assert_eq!(key.bucket, "my-bucket");
        assert_eq!(key.object_key, "path/to/file.jpg");
        assert!(key.variant.is_none());
    }

    #[test]
    fn test_build_cache_key_with_variant() {
        let key = build_cache_key("my-bucket", "image.jpg", Some("w=100&h=100".to_string()));

        assert_eq!(key.bucket, "my-bucket");
        assert_eq!(key.object_key, "image.jpg");
        assert_eq!(key.variant, Some("w=100&h=100".to_string()));
    }

    #[test]
    fn test_build_cache_entry() {
        let mut headers = CapturedHeaders::new();
        headers.content_type = Some("image/jpeg".to_string());
        headers.etag = Some("abc123".to_string());
        headers.last_modified = Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string());

        let entry = build_cache_entry(
            Bytes::from("test data"),
            &headers,
            Duration::from_secs(3600),
        );

        assert_eq!(entry.content_type, "image/jpeg");
        assert_eq!(entry.etag, "abc123");
        assert_eq!(
            entry.last_modified,
            Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
        );
    }

    #[test]
    fn test_build_cache_entry_defaults() {
        let headers = CapturedHeaders::new();

        let entry = build_cache_entry(Bytes::from("data"), &headers, Duration::from_secs(600));

        assert_eq!(entry.content_type, "application/octet-stream");
        assert_eq!(entry.etag, "");
        assert!(entry.last_modified.is_none());
    }

    #[test]
    fn test_is_optimizable_image_jpeg() {
        assert!(is_optimizable_image("image/jpeg"));
        assert!(is_optimizable_image("image/jpg"));
        assert!(is_optimizable_image("Image/JPEG")); // case insensitive
    }

    #[test]
    fn test_is_optimizable_image_png() {
        assert!(is_optimizable_image("image/png"));
    }

    #[test]
    fn test_is_optimizable_image_webp() {
        assert!(is_optimizable_image("image/webp"));
    }

    #[test]
    fn test_is_optimizable_image_gif() {
        assert!(is_optimizable_image("image/gif"));
    }

    #[test]
    fn test_is_optimizable_image_avif() {
        assert!(is_optimizable_image("image/avif"));
    }

    #[test]
    fn test_is_optimizable_image_not_image() {
        assert!(!is_optimizable_image("text/html"));
        assert!(!is_optimizable_image("application/json"));
        assert!(!is_optimizable_image("video/mp4"));
    }

    #[test]
    fn test_is_optimizable_image_unsupported_format() {
        assert!(!is_optimizable_image("image/svg+xml"));
        assert!(!is_optimizable_image("image/bmp"));
        assert!(!is_optimizable_image("image/tiff"));
    }

    #[test]
    fn test_get_effective_ttl() {
        let mut headers = CapturedHeaders::new();
        headers.cache_control = Some("max-age=7200".to_string());

        let ttl = get_effective_ttl(&headers);
        assert_eq!(ttl, Duration::from_secs(7200));
    }

    #[test]
    fn test_get_effective_ttl_uses_default() {
        let headers = CapturedHeaders::new();

        let ttl = get_effective_ttl(&headers);
        assert_eq!(ttl, DEFAULT_CACHE_TTL);
    }

    #[test]
    fn test_should_cache_response() {
        let headers = CapturedHeaders::new();
        assert!(should_cache_response(&headers));

        let mut headers_no_store = CapturedHeaders::new();
        headers_no_store.cache_control = Some("no-store".to_string());
        assert!(!should_cache_response(&headers_no_store));
    }
}
