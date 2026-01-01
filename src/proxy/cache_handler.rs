//! Cache handler module for the proxy.
//!
//! This module provides cache lookup and streaming coalescing orchestration.
//! It extracts cache-related logic from the main proxy module for better
//! organization and testability.
//!
//! # Design
//!
//! Functions return structured result enums instead of writing directly to session.
//! This avoids borrow checker issues and keeps cache handling testable.
//! The caller handles writing responses to the session and updating metrics.

use bytes::Bytes;
use tokio::sync::broadcast;

use crate::cache::tiered::TieredCache;
use crate::cache::traits::Cache;
use crate::cache::{CacheEntry, CacheKey};
use crate::request_coalescing::{Coalescer, StreamingSlot, StreamMessage};

// ============================================================================
// Result Types
// ============================================================================

/// Result of a cache lookup operation.
#[derive(Debug)]
pub enum CacheLookup {
    /// Cache hit - entry found and valid.
    Hit {
        /// The cached entry.
        entry: CacheEntry,
    },
    /// Conditional request matched - client already has current version.
    /// Return 304 Not Modified.
    ConditionalNotModified {
        /// ETag to include in response (if available).
        etag: Option<String>,
        /// Last-Modified to include in response (if available).
        last_modified: Option<String>,
    },
    /// Cache miss - entry not found or expired.
    Miss,
    /// Cache lookup error - continue to upstream.
    Error {
        /// The error message for logging.
        message: String,
    },
    /// Joined streaming coalescer as follower.
    CoalescerFollower {
        /// Receiver for streaming data from leader.
        receiver: broadcast::Receiver<StreamMessage>,
    },
}

/// Result of conditional request validation.
#[derive(Debug, Clone)]
pub enum ConditionalResult {
    /// ETags match - return 304 with ETag.
    NotModifiedByEtag { etag: String },
    /// Last-Modified matches - return 304 with Last-Modified (and optionally ETag).
    NotModifiedByDate {
        last_modified: String,
        etag: Option<String>,
    },
    /// No match - serve full response.
    Modified,
}

/// Response data prepared from a cache hit.
#[derive(Debug, Clone)]
pub struct CacheHitResponse {
    /// HTTP status code (200 for full response, 304 for conditional).
    pub status: u16,
    /// Content-Type header value.
    pub content_type: String,
    /// ETag header value.
    pub etag: String,
    /// Last-Modified header value (if available).
    pub last_modified: Option<String>,
    /// Content-Length header value.
    pub content_length: usize,
    /// Response body (None for HEAD requests or 304 responses).
    pub body: Option<Bytes>,
}

/// Result of coalescer acquisition.
#[derive(Debug)]
pub enum CoalescerAcquisition {
    /// Became the leader - proceed to S3.
    Leader,
    /// Became a follower - stream from leader.
    Follower {
        /// Receiver for streaming data from leader.
        receiver: broadcast::Receiver<StreamMessage>,
    },
    /// No coalescer configured.
    NotConfigured,
}

// ============================================================================
// Cache Lookup Functions
// ============================================================================

/// Check if a request can be served from cache.
///
/// Performs cache lookup and conditional request validation.
/// Returns a `CacheLookup` result indicating how to proceed.
///
/// # Arguments
///
/// * `cache` - The tiered cache to query.
/// * `key` - The cache key to look up.
/// * `if_none_match` - Client's If-None-Match header (for ETag validation).
/// * `if_modified_since` - Client's If-Modified-Since header.
///
/// # Returns
///
/// * `CacheLookup::Hit` - Cache hit, entry is valid.
/// * `CacheLookup::ConditionalNotModified` - Client has current version.
/// * `CacheLookup::Miss` - Cache miss, fetch from upstream.
/// * `CacheLookup::Error` - Cache error, continue to upstream.
pub async fn check_cache_hit(
    cache: &TieredCache,
    key: &CacheKey,
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
) -> CacheLookup {
    match cache.get(key).await {
        Ok(Some(entry)) => {
            // Check conditional request headers
            match handle_conditional_request(&entry, if_none_match, if_modified_since) {
                ConditionalResult::NotModifiedByEtag { etag } => CacheLookup::ConditionalNotModified {
                    etag: Some(etag),
                    last_modified: None,
                },
                ConditionalResult::NotModifiedByDate { last_modified, etag } => {
                    CacheLookup::ConditionalNotModified {
                        etag,
                        last_modified: Some(last_modified),
                    }
                }
                ConditionalResult::Modified => CacheLookup::Hit { entry },
            }
        }
        Ok(None) => CacheLookup::Miss,
        Err(e) => CacheLookup::Error {
            message: e.to_string(),
        },
    }
}

/// Handle conditional request headers (If-None-Match, If-Modified-Since).
///
/// Checks if the client's cached version matches the server version.
/// If so, we can return 304 Not Modified instead of the full response.
///
/// # Arguments
///
/// * `entry` - The cached entry to validate against.
/// * `if_none_match` - Client's If-None-Match header (ETag comparison).
/// * `if_modified_since` - Client's If-Modified-Since header.
///
/// # Returns
///
/// * `ConditionalResult::EtagMatch` - ETag matches, return 304.
/// * `ConditionalResult::LastModifiedMatch` - Last-Modified matches, return 304.
/// * `ConditionalResult::NoMatch` - No match, serve full response.
pub fn handle_conditional_request(
    entry: &CacheEntry,
    if_none_match: Option<&str>,
    if_modified_since: Option<&str>,
) -> ConditionalResult {
    // Check ETag first (stronger validator)
    if let Some(client_etag) = if_none_match {
        if client_etag == entry.etag {
            return ConditionalResult::NotModifiedByEtag {
                etag: entry.etag.clone(),
            };
        }
    }

    // Check Last-Modified
    if let Some(client_modified_since) = if_modified_since {
        if let Some(ref last_modified) = entry.last_modified {
            if client_modified_since == last_modified {
                return ConditionalResult::NotModifiedByDate {
                    last_modified: last_modified.clone(),
                    etag: if !entry.etag.is_empty() {
                        Some(entry.etag.clone())
                    } else {
                        None
                    },
                };
            }
        }
    }

    ConditionalResult::Modified
}

/// Build a response from a cache entry.
///
/// Prepares response headers and body for serving from cache.
///
/// # Arguments
///
/// * `entry` - The cached entry to serve.
/// * `is_head_request` - Whether this is a HEAD request (no body).
///
/// # Returns
///
/// A `CacheHitResponse` with all data needed to write the response.
pub fn serve_from_cache(entry: &CacheEntry, is_head_request: bool) -> CacheHitResponse {
    CacheHitResponse {
        status: 200,
        content_type: entry.content_type.clone(),
        etag: entry.etag.clone(),
        last_modified: entry.last_modified.clone(),
        content_length: entry.data.len(),
        body: if is_head_request {
            None
        } else {
            Some(entry.data.clone())
        },
    }
}

/// Build a 304 Not Modified response.
///
/// # Arguments
///
/// * `etag` - ETag to include in response.
/// * `last_modified` - Last-Modified to include in response.
///
/// # Returns
///
/// A `CacheHitResponse` configured for 304 response.
pub fn build_not_modified_response(
    etag: Option<String>,
    last_modified: Option<String>,
) -> CacheHitResponse {
    CacheHitResponse {
        status: 304,
        content_type: String::new(), // Not needed for 304
        etag: etag.unwrap_or_default(),
        last_modified,
        content_length: 0,
        body: None,
    }
}

// ============================================================================
// Streaming Coalescing Functions
// ============================================================================

/// Try to join a streaming coalescer for the given cache key.
///
/// If a coalescer is configured and another request is already fetching
/// this object, join as a follower to receive streamed data.
///
/// # Arguments
///
/// * `coalescer` - The streaming coalescer (if configured).
/// * `cache_key` - The cache key to coalesce on.
///
/// # Returns
///
/// * `CoalescerAcquisition::Leader` - Became leader, proceed to S3.
/// * `CoalescerAcquisition::Follower` - Became follower, stream from leader.
/// * `CoalescerAcquisition::NotConfigured` - No coalescer configured.
pub fn join_streaming_coalescer(
    coalescer: Option<&Coalescer>,
    cache_key: &CacheKey,
) -> CoalescerAcquisition {
    let Some(Coalescer::Streaming(streaming_coalescer)) = coalescer else {
        return CoalescerAcquisition::NotConfigured;
    };

    match streaming_coalescer.acquire(cache_key) {
        StreamingSlot::Leader(_leader) => CoalescerAcquisition::Leader,
        StreamingSlot::Follower(receiver) => CoalescerAcquisition::Follower { receiver },
    }
}

/// Check if a streaming coalescer is configured.
pub fn has_streaming_coalescer(coalescer: Option<&Coalescer>) -> bool {
    matches!(coalescer, Some(Coalescer::Streaming(_)))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build a cache key from request components.
///
/// # Arguments
///
/// * `bucket` - The bucket name.
/// * `object_key` - The S3 object key (path).
/// * `variant` - Optional variant string (e.g., image processing params).
///
/// # Returns
///
/// A `CacheKey` ready for cache lookup.
pub fn build_cache_key(bucket: &str, object_key: &str, variant: Option<String>) -> CacheKey {
    CacheKey {
        bucket: bucket.to_string(),
        object_key: object_key.to_string(),
        etag: None,
        variant,
    }
}

/// Extract conditional request headers from a header map.
///
/// # Arguments
///
/// * `headers` - The request headers.
///
/// # Returns
///
/// A tuple of (If-None-Match, If-Modified-Since) values.
pub fn extract_conditional_headers(
    headers: &std::collections::HashMap<String, String>,
) -> (Option<String>, Option<String>) {
    let if_none_match = headers
        .get("If-None-Match")
        .or_else(|| headers.get("if-none-match"))
        .cloned();

    let if_modified_since = headers
        .get("If-Modified-Since")
        .or_else(|| headers.get("if-modified-since"))
        .cloned();

    (if_none_match, if_modified_since)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::time::Duration;

    /// Helper to create a test cache entry.
    fn test_cache_entry(etag: &str, last_modified: Option<&str>) -> CacheEntry {
        CacheEntry::new(
            Bytes::from("test data"),
            "text/plain".to_string(),
            etag.to_string(),
            last_modified.map(|s| s.to_string()),
            Some(Duration::from_secs(3600)),
        )
    }

    #[test]
    fn test_cache_handler_module_exists() {
        // Phase 37.4 structural verification test
        let _ = CacheLookup::Miss;
        let _ = ConditionalResult::Modified;
        let _ = CoalescerAcquisition::NotConfigured;
    }

    #[test]
    fn test_handle_conditional_request_etag_match() {
        let entry = test_cache_entry("abc123", None);
        let result = handle_conditional_request(&entry, Some("abc123"), None);

        match result {
            ConditionalResult::NotModifiedByEtag { etag } => assert_eq!(etag, "abc123"),
            _ => panic!("Expected NotModifiedByEtag"),
        }
    }

    #[test]
    fn test_handle_conditional_request_etag_no_match() {
        let entry = test_cache_entry("abc123", None);
        let result = handle_conditional_request(&entry, Some("different"), None);

        assert!(matches!(result, ConditionalResult::Modified));
    }

    #[test]
    fn test_handle_conditional_request_last_modified_match() {
        let entry = test_cache_entry("abc123", Some("Wed, 21 Oct 2015 07:28:00 GMT"));
        let result =
            handle_conditional_request(&entry, None, Some("Wed, 21 Oct 2015 07:28:00 GMT"));

        match result {
            ConditionalResult::NotModifiedByDate { last_modified, etag } => {
                assert_eq!(last_modified, "Wed, 21 Oct 2015 07:28:00 GMT");
                assert_eq!(etag, Some("abc123".to_string()));
            }
            _ => panic!("Expected NotModifiedByDate"),
        }
    }

    #[test]
    fn test_handle_conditional_request_no_headers() {
        let entry = test_cache_entry("abc123", Some("Wed, 21 Oct 2015 07:28:00 GMT"));
        let result = handle_conditional_request(&entry, None, None);

        assert!(matches!(result, ConditionalResult::Modified));
    }

    #[test]
    fn test_serve_from_cache_get_request() {
        let entry = test_cache_entry("abc123", Some("Wed, 21 Oct 2015 07:28:00 GMT"));
        let response = serve_from_cache(&entry, false);

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/plain");
        assert_eq!(response.etag, "abc123");
        assert_eq!(
            response.last_modified,
            Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
        );
        assert!(response.body.is_some());
    }

    #[test]
    fn test_serve_from_cache_head_request() {
        let entry = test_cache_entry("abc123", None);
        let response = serve_from_cache(&entry, true);

        assert_eq!(response.status, 200);
        assert!(response.body.is_none());
    }

    #[test]
    fn test_build_not_modified_response() {
        let response =
            build_not_modified_response(Some("abc123".to_string()), Some("last-mod".to_string()));

        assert_eq!(response.status, 304);
        assert_eq!(response.etag, "abc123");
        assert_eq!(response.last_modified, Some("last-mod".to_string()));
        assert!(response.body.is_none());
    }

    #[test]
    fn test_build_cache_key() {
        let key = build_cache_key("my-bucket", "path/to/file.jpg", None);

        assert_eq!(key.bucket, "my-bucket");
        assert_eq!(key.object_key, "path/to/file.jpg");
        assert_eq!(key.etag, None);
        assert_eq!(key.variant, None);
    }

    #[test]
    fn test_build_cache_key_with_variant() {
        let key = build_cache_key("my-bucket", "image.jpg", Some("w=100&h=100".to_string()));

        assert_eq!(key.bucket, "my-bucket");
        assert_eq!(key.object_key, "image.jpg");
        assert_eq!(key.variant, Some("w=100&h=100".to_string()));
    }

    #[test]
    fn test_extract_conditional_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("If-None-Match".to_string(), "abc123".to_string());
        headers.insert(
            "If-Modified-Since".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );

        let (etag, modified) = extract_conditional_headers(&headers);

        assert_eq!(etag, Some("abc123".to_string()));
        assert_eq!(
            modified,
            Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
        );
    }

    #[test]
    fn test_extract_conditional_headers_lowercase() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("if-none-match".to_string(), "abc123".to_string());
        headers.insert(
            "if-modified-since".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );

        let (etag, modified) = extract_conditional_headers(&headers);

        assert_eq!(etag, Some("abc123".to_string()));
        assert_eq!(
            modified,
            Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
        );
    }

    #[test]
    fn test_has_streaming_coalescer_none() {
        assert!(!has_streaming_coalescer(None));
    }

    #[test]
    fn test_join_streaming_coalescer_not_configured() {
        let result = join_streaming_coalescer(None, &build_cache_key("bucket", "key", None));

        assert!(matches!(result, CoalescerAcquisition::NotConfigured));
    }
}
