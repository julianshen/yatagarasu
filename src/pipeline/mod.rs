// Request pipeline module - handles request context and middleware chain
// Phase 13: Request Pipeline Integration

use std::collections::HashMap;
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
}

impl RequestContext {
    /// Create a new RequestContext from HTTP request information
    /// Automatically generates a unique request ID (UUID v4)
    pub fn new(method: String, path: String) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            headers: HashMap::new(),
            query_params: HashMap::new(),
        }
    }

    /// Create a new RequestContext with headers
    /// Automatically generates a unique request ID (UUID v4)
    pub fn with_headers(method: String, path: String, headers: HashMap<String, String>) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            headers,
            query_params: HashMap::new(),
        }
    }

    /// Create a new RequestContext with query parameters
    /// Automatically generates a unique request ID (UUID v4)
    pub fn with_query_params(method: String, path: String, query_params: HashMap<String, String>) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            method,
            path,
            headers: HashMap::new(),
            query_params,
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

    /// Get the request path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the request headers
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// Get the query parameters
    pub fn query_params(&self) -> &HashMap<String, String> {
        &self.query_params
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
