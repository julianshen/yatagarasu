// Request pipeline module - handles request context and middleware chain
// Phase 13: Request Pipeline Integration

/// Request context that holds all information about an HTTP request
/// as it flows through the middleware pipeline
#[derive(Debug, Clone)]
pub struct RequestContext {
    method: String,
    path: String,
}

impl RequestContext {
    /// Create a new RequestContext from HTTP request information
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
        }
    }

    /// Get the HTTP method
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        &self.path
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
