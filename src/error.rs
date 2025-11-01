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
