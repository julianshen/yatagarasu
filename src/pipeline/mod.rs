// Request pipeline module - handles request context and middleware chain
// Phase 13: Request Pipeline Integration

use crate::audit::RequestContext as AuditRequestContext;
use crate::auth::Claims;
use crate::config::BucketConfig;
use crate::request_coalescing::StreamLeader;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Request context that holds all information about an HTTP request
/// as it flows through the middleware pipeline
#[derive(Debug)]
pub struct RequestContext {
    request_id: String,
    method: String,
    path: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    timestamp: u64,
    bucket_config: Option<BucketConfig>,
    claims: Option<Claims>,
    /// Selected replica name for Phase 23 HA bucket replication
    replica_name: Option<String>,
    /// Phase 30: Response buffering for cache population
    /// Buffer for accumulating response chunks from S3
    response_buffer: Option<Vec<u8>>,
    /// Content-Type from S3 response headers
    response_content_type: Option<String>,
    /// ETag from S3 response headers
    response_etag: Option<String>,
    /// Last-Modified from S3 response headers (for If-Modified-Since support)
    response_last_modified: Option<String>,
    /// Whether to cache this response (based on size, range requests, etc.)
    should_cache_response: bool,
    /// Total response size accumulated so far
    total_response_size: usize,
    /// Retry attempt counter (0-indexed: 0 = first attempt, 1 = first retry)
    retry_attempt: u32,
    /// Audit context
    pub audit: AuditRequestContext,
    /// Image optimization parameters (Phase: Image Optimization)
    image_params: Option<crate::image_optimizer::ImageParams>,
    /// Whether the current response is being optimized (Phase: Image Optimization)
    optimizing_image: bool,
    /// Streaming coalescer leader handle
    /// If Some, this request is the leader and must broadcast data to followers
    streaming_leader: Option<StreamLeader>,
}

impl RequestContext {
    /// Create a new RequestContext from HTTP request information
    /// Automatically generates a unique request ID (UUID v4) and captures current timestamp
    pub fn new(method: String, path: String) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            bucket_config: None,
            claims: None,
            replica_name: None,
            response_buffer: None,
            response_content_type: None,
            response_etag: None,
            response_last_modified: None,
            should_cache_response: false,
            total_response_size: 0,
            retry_attempt: 0,
            audit: AuditRequestContext::new(),
            image_params: None,
            optimizing_image: false,
            streaming_leader: None,
        }
    }

    /// Create a new RequestContext with headers
    /// Automatically generates a unique request ID (UUID v4) and captures current timestamp
    pub fn with_headers(method: String, path: String, headers: HashMap<String, String>) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            headers,
            query_params: HashMap::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            bucket_config: None,
            claims: None,
            replica_name: None,
            response_buffer: None,
            response_content_type: None,
            response_etag: None,
            response_last_modified: None,
            should_cache_response: false,
            total_response_size: 0,
            retry_attempt: 0,
            audit: AuditRequestContext::new(),
            image_params: None,
            optimizing_image: false,
            streaming_leader: None,
        }
    }

    /// Create a new RequestContext with query parameters
    /// Automatically generates a unique request ID (UUID v4) and captures current timestamp
    pub fn with_query_params(
        method: String,
        path: String,
        query_params: HashMap<String, String>,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            headers: HashMap::new(),
            query_params,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            bucket_config: None,
            claims: None,
            replica_name: None,
            response_buffer: None,
            response_content_type: None,
            response_etag: None,
            response_last_modified: None,
            should_cache_response: false,
            total_response_size: 0,
            retry_attempt: 0,
            audit: AuditRequestContext::new(),
            image_params: None,
            optimizing_image: false,
            streaming_leader: None,
        }
    }

    /// Get the unique request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get the HTTP method
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Set the HTTP method
    pub fn set_method(&mut self, method: String) {
        self.method = method;
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Set the request path
    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    /// Get the request headers
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Set the request headers
    pub fn set_headers(&mut self, headers: HashMap<String, String>) {
        self.headers = headers;
    }

    /// Get the query parameters
    pub fn query_params(&self) -> &HashMap<String, String> {
        &self.query_params
    }

    /// Set the query parameters
    pub fn set_query_params(&mut self, query_params: HashMap<String, String>) {
        self.query_params = query_params;
    }

    /// Get the request timestamp (Unix epoch seconds)
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Set the bucket configuration for this request
    pub fn set_bucket_config(&mut self, bucket_config: BucketConfig) {
        self.bucket_config = Some(bucket_config);
    }

    /// Get the bucket configuration for this request
    pub fn bucket_config(&self) -> Option<&BucketConfig> {
        self.bucket_config.as_ref()
    }

    /// Set the JWT claims for this request
    pub fn set_claims(&mut self, claims: Claims) {
        self.claims = Some(claims);
    }

    /// Get the JWT claims for this request
    pub fn claims(&self) -> Option<&Claims> {
        self.claims.as_ref()
    }

    /// Set the replica name that is serving this request (Phase 23: HA bucket replication)
    pub fn set_replica_name(&mut self, replica_name: String) {
        self.replica_name = Some(replica_name);
    }

    /// Get the replica name that is serving this request (Phase 23: HA bucket replication)
    pub fn replica_name(&self) -> Option<&str> {
        self.replica_name.as_deref()
    }

    /// Enable response buffering for cache population (Phase 30)
    pub fn enable_response_buffering(&mut self) {
        self.response_buffer = Some(Vec::new());
        self.should_cache_response = true;
        self.total_response_size = 0;
    }

    /// Disable response buffering (e.g., for range requests or large files)
    pub fn disable_response_buffering(&mut self) {
        self.response_buffer = None;
        self.should_cache_response = false;
    }

    /// Check if response buffering is enabled
    pub fn is_response_buffering_enabled(&self) -> bool {
        self.response_buffer.is_some()
    }

    /// Append data to response buffer
    pub fn append_response_chunk(&mut self, chunk: &[u8]) {
        if let Some(buffer) = &mut self.response_buffer {
            buffer.extend_from_slice(chunk);
            self.total_response_size += chunk.len();
        }
    }

    /// Get the buffered response data
    pub fn take_response_buffer(&mut self) -> Option<Vec<u8>> {
        self.response_buffer.take()
    }

    /// Set response Content-Type from upstream headers
    pub fn set_response_content_type(&mut self, content_type: String) {
        self.response_content_type = Some(content_type);
    }

    /// Get response Content-Type
    pub fn response_content_type(&self) -> Option<&str> {
        self.response_content_type.as_deref()
    }

    /// Set response ETag from upstream headers
    pub fn set_response_etag(&mut self, etag: String) {
        self.response_etag = Some(etag);
    }

    /// Get response ETag
    pub fn response_etag(&self) -> Option<&str> {
        self.response_etag.as_deref()
    }

    /// Set response Last-Modified from upstream headers (for If-Modified-Since)
    pub fn set_response_last_modified(&mut self, last_modified: String) {
        self.response_last_modified = Some(last_modified);
    }

    /// Get response Last-Modified
    pub fn response_last_modified(&self) -> Option<&str> {
        self.response_last_modified.as_deref()
    }

    /// Check if this response should be cached
    pub fn should_cache_response(&self) -> bool {
        self.should_cache_response
    }

    /// Get total response size accumulated so far
    pub fn total_response_size(&self) -> usize {
        self.total_response_size
    }

    /// Get current retry attempt number (0-indexed)
    pub fn retry_attempt(&self) -> u32 {
        self.retry_attempt
    }

    /// Increment retry attempt counter and return the new value
    pub fn increment_retry_attempt(&mut self) -> u32 {
        self.retry_attempt += 1;
        self.retry_attempt
    }

    /// Get a mutable reference to the audit context
    pub fn audit(&mut self) -> &mut AuditRequestContext {
        &mut self.audit
    }

    /// Set image optimization parameters
    pub fn set_image_params(&mut self, params: crate::image_optimizer::ImageParams) {
        self.image_params = Some(params);
    }

    /// Get image optimization parameters
    pub fn image_params(&self) -> Option<&crate::image_optimizer::ImageParams> {
        self.image_params.as_ref()
    }

    /// Set whether the response is being optimized
    pub fn set_optimizing_image(&mut self, optimizing: bool) {
        self.optimizing_image = optimizing;
    }

    /// Check if the response is being optimized
    pub fn is_optimizing_image(&self) -> bool {
        self.optimizing_image
    }

    /// Set the streaming leader handle (Streaming Coalescing)
    pub fn set_streaming_leader(&mut self, leader: StreamLeader) {
        self.streaming_leader = Some(leader);
    }

    /// Take the streaming leader handle, leaving None in its place
    pub fn take_streaming_leader(&mut self) -> Option<StreamLeader> {
        self.streaming_leader.take()
    }

    /// Get a reference to the streaming leader handle
    pub fn streaming_leader(&self) -> Option<&StreamLeader> {
        self.streaming_leader.as_ref()
    }
}

// Manual Clone implementation because StreamLeader cannot implement Clone
// (it has RAII semantics for cleanup). When cloning, streaming_leader is set to None.
impl Clone for RequestContext {
    fn clone(&self) -> Self {
        Self {
            request_id: self.request_id.clone(),
            method: self.method.clone(),
            path: self.path.clone(),
            headers: self.headers.clone(),
            query_params: self.query_params.clone(),
            timestamp: self.timestamp,
            bucket_config: self.bucket_config.clone(),
            claims: self.claims.clone(),
            replica_name: self.replica_name.clone(),
            response_buffer: self.response_buffer.clone(),
            response_content_type: self.response_content_type.clone(),
            response_etag: self.response_etag.clone(),
            response_last_modified: self.response_last_modified.clone(),
            should_cache_response: self.should_cache_response,
            total_response_size: self.total_response_size,
            retry_attempt: self.retry_attempt,
            audit: self.audit.clone(),
            image_params: self.image_params.clone(),
            optimizing_image: self.optimizing_image,
            streaming_leader: None, // Cannot clone - RAII handle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new("GET".to_string(), "/test".to_string());
        assert_eq!(ctx.method(), "GET");
        assert_eq!(ctx.path(), "/test");
    }
}
