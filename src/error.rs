// Error types module

use std::fmt;

/// Centralized error type for the proxy
///
/// Categorizes errors into 4 main types for better debugging,
/// monitoring, and appropriate HTTP status code mapping.
#[derive(Debug, Clone)]
pub enum ProxyError {
    /// Configuration errors (invalid YAML, missing env vars, etc.)
    Config(String),

    /// Authentication/authorization failures (invalid JWT, missing token, etc.)
    Auth(String),

    /// S3-related errors (NoSuchKey, AccessDenied, network timeout, etc.)
    S3(String),

    /// Internal proxy errors (panic, resource exhaustion, unexpected errors)
    Internal(String),
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::Config(msg) => write!(f, "Configuration error: {}", msg),
            ProxyError::Auth(msg) => write!(f, "Authentication error: {}", msg),
            ProxyError::S3(msg) => write!(f, "S3 error: {}", msg),
            ProxyError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ProxyError {}

impl ProxyError {
    /// Convert error to appropriate HTTP status code
    ///
    /// Maps error variants to HTTP status codes following RFC 7231:
    /// - Config errors → 500 (Internal Server Error - proxy misconfiguration)
    /// - Auth errors → 401 (Unauthorized - authentication failed)
    /// - S3 errors → 502 (Bad Gateway - upstream service error)
    /// - Internal errors → 500 (Internal Server Error - unexpected proxy error)
    pub fn to_http_status(&self) -> u16 {
        match self {
            ProxyError::Config(_) => 500,    // Internal Server Error
            ProxyError::Auth(_) => 401,      // Unauthorized
            ProxyError::S3(_) => 502,        // Bad Gateway
            ProxyError::Internal(_) => 500,  // Internal Server Error
        }
    }

    /// Convert error to JSON response string
    ///
    /// Produces consistent JSON error response with fields:
    /// - error: Error category ("config", "auth", "s3", "internal")
    /// - message: Human-readable error message
    /// - status: HTTP status code
    /// - request_id: Optional request ID for tracing
    ///
    /// Example output:
    /// ```json
    /// {
    ///   "error": "auth",
    ///   "message": "Authentication error: invalid token",
    ///   "status": 401,
    ///   "request_id": "550e8400-e29b-41d4-a716-446655440000"
    /// }
    /// ```
    pub fn to_json_response(&self, request_id: Option<String>) -> String {
        use serde_json::json;

        let error_type = match self {
            ProxyError::Config(_) => "config",
            ProxyError::Auth(_) => "auth",
            ProxyError::S3(_) => "s3",
            ProxyError::Internal(_) => "internal",
        };

        let mut response = json!({
            "error": error_type,
            "message": self.to_string(),
            "status": self.to_http_status(),
        });

        // Add request_id if provided
        if let Some(id) = request_id {
            response["request_id"] = json!(id);
        }

        // Use to_string() to get compact JSON (no pretty printing)
        response.to_string()
    }
}
