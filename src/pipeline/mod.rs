// Request pipeline module - handles request context and middleware chain
// Phase 13: Request Pipeline Integration

use crate::auth::Claims;
use crate::config::BucketConfig;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Request context that holds all information about an HTTP request
/// as it flows through the middleware pipeline
#[derive(Debug, Clone)]
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
