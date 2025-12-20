// Error types module

use std::fmt;

/// Centralized error type for the proxy
///
/// Categorizes errors into 4 main types for better debugging,
/// monitoring, and appropriate HTTP status code mapping.
///
/// Each variant now includes structured context for better debugging:
/// - Config: configuration file/validation errors
/// - Auth: authentication/authorization failures with optional user/bucket context
/// - S3: S3-related errors with optional bucket/key context
/// - Internal: unexpected proxy errors with optional operation context
#[derive(Debug, Clone)]
pub enum ProxyError {
    /// Configuration errors (invalid YAML, missing env vars, etc.)
    ///
    /// Fields:
    /// - message: Human-readable error description
    /// - context: Optional additional context (e.g., "loading config.yaml")
    Config {
        message: String,
        context: Option<String>,
    },

    /// Authentication/authorization failures (invalid JWT, missing token, etc.)
    ///
    /// Fields:
    /// - message: Human-readable error description
    /// - bucket: Optional bucket name for context
    /// - user: Optional user ID for context
    Auth {
        message: String,
        bucket: Option<String>,
        user: Option<String>,
    },

    /// S3-related errors (NoSuchKey, AccessDenied, network timeout, etc.)
    ///
    /// Fields:
    /// - message: Human-readable error description
    /// - bucket: Optional bucket name
    /// - key: Optional S3 object key
    /// - operation: Optional operation type (GET, HEAD, LIST, etc.)
    S3 {
        message: String,
        bucket: Option<String>,
        key: Option<String>,
        operation: Option<String>,
    },

    /// Internal proxy errors (panic, resource exhaustion, unexpected errors)
    ///
    /// Fields:
    /// - message: Human-readable error description
    /// - operation: Optional operation being performed
    /// - details: Optional additional details for debugging
    Internal {
        message: String,
        operation: Option<String>,
        details: Option<String>,
    },
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::Config { message, context } => {
                write!(f, "Configuration error: {}", message)?;
                if let Some(ctx) = context {
                    write!(f, " ({})", ctx)?;
                }
                Ok(())
            }
            ProxyError::Auth {
                message,
                bucket,
                user,
            } => {
                write!(f, "Authentication error: {}", message)?;
                if let Some(b) = bucket {
                    write!(f, " [bucket: {}]", b)?;
                }
                if let Some(u) = user {
                    write!(f, " [user: {}]", u)?;
                }
                Ok(())
            }
            ProxyError::S3 {
                message,
                bucket,
                key,
                operation,
            } => {
                write!(f, "S3 error: {}", message)?;
                if let Some(b) = bucket {
                    write!(f, " [bucket: {}]", b)?;
                }
                if let Some(k) = key {
                    write!(f, " [key: {}]", k)?;
                }
                if let Some(op) = operation {
                    write!(f, " [op: {}]", op)?;
                }
                Ok(())
            }
            ProxyError::Internal {
                message,
                operation,
                details,
            } => {
                write!(f, "Internal error: {}", message)?;
                if let Some(op) = operation {
                    write!(f, " [operation: {}]", op)?;
                }
                if let Some(d) = details {
                    write!(f, " [details: {}]", d)?;
                }
                Ok(())
            }
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
            ProxyError::Config { .. } => 500,   // Internal Server Error
            ProxyError::Auth { .. } => 401,     // Unauthorized
            ProxyError::S3 { .. } => 502,       // Bad Gateway
            ProxyError::Internal { .. } => 500, // Internal Server Error
        }
    }

    /// Convert error to JSON response string
    ///
    /// Produces consistent JSON error response with fields:
    /// - error: Error category ("config", "auth", "s3", "internal")
    /// - message: Human-readable error message
    /// - status: HTTP status code
    /// - context: Optional context fields (bucket, key, user, operation, etc.)
    /// - request_id: Optional request ID for tracing
    ///
    /// Example output:
    /// ```json
    /// {
    ///   "error": "auth",
    ///   "message": "Authentication error: invalid token",
    ///   "status": 401,
    ///   "context": {
    ///     "bucket": "my-bucket",
    ///     "user": "user123"
    ///   },
    ///   "request_id": "550e8400-e29b-41d4-a716-446655440000"
    /// }
    /// ```
    pub fn to_json_response(&self, request_id: Option<String>) -> String {
        use serde_json::json;

        let (error_type, context) = match self {
            ProxyError::Config {
                message: _,
                context,
            } => ("config", json!({ "context": context })),
            ProxyError::Auth {
                message: _,
                bucket,
                user,
            } => {
                let mut ctx = serde_json::Map::new();
                if let Some(b) = bucket {
                    ctx.insert("bucket".to_string(), json!(b));
                }
                if let Some(u) = user {
                    ctx.insert("user".to_string(), json!(u));
                }
                ("auth", json!(ctx))
            }
            ProxyError::S3 {
                message: _,
                bucket,
                key,
                operation,
            } => {
                let mut ctx = serde_json::Map::new();
                if let Some(b) = bucket {
                    ctx.insert("bucket".to_string(), json!(b));
                }
                if let Some(k) = key {
                    ctx.insert("key".to_string(), json!(k));
                }
                if let Some(op) = operation {
                    ctx.insert("operation".to_string(), json!(op));
                }
                ("s3", json!(ctx))
            }
            ProxyError::Internal {
                message: _,
                operation,
                details,
            } => {
                let mut ctx = serde_json::Map::new();
                if let Some(op) = operation {
                    ctx.insert("operation".to_string(), json!(op));
                }
                if let Some(d) = details {
                    ctx.insert("details".to_string(), json!(d));
                }
                ("internal", json!(ctx))
            }
        };

        let mut response = json!({
            "error": error_type,
            "message": self.to_string(),
            "status": self.to_http_status(),
        });

        // Add context if not empty
        if !context.is_null() && context.as_object().is_some_and(|m| !m.is_empty()) {
            response["context"] = context;
        }

        // Add request_id if provided
        if let Some(id) = request_id {
            response["request_id"] = json!(id);
        }

        // Use to_string() to get compact JSON (no pretty printing)
        response.to_string()
    }

    // Helper constructors for easier error creation with context

    /// Create a Config error with optional context
    pub fn config(message: impl Into<String>) -> Self {
        ProxyError::Config {
            message: message.into(),
            context: None,
        }
    }

    /// Create a Config error with context
    pub fn config_with_context(message: impl Into<String>, context: impl Into<String>) -> Self {
        ProxyError::Config {
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create an Auth error with optional bucket and user context
    pub fn auth(message: impl Into<String>) -> Self {
        ProxyError::Auth {
            message: message.into(),
            bucket: None,
            user: None,
        }
    }

    /// Create an Auth error with bucket context
    pub fn auth_with_bucket(message: impl Into<String>, bucket: impl Into<String>) -> Self {
        ProxyError::Auth {
            message: message.into(),
            bucket: Some(bucket.into()),
            user: None,
        }
    }

    /// Create an Auth error with user context
    pub fn auth_with_user(message: impl Into<String>, user: impl Into<String>) -> Self {
        ProxyError::Auth {
            message: message.into(),
            bucket: None,
            user: Some(user.into()),
        }
    }

    /// Create an Auth error with both bucket and user context
    pub fn auth_with_context(
        message: impl Into<String>,
        bucket: impl Into<String>,
        user: impl Into<String>,
    ) -> Self {
        ProxyError::Auth {
            message: message.into(),
            bucket: Some(bucket.into()),
            user: Some(user.into()),
        }
    }

    /// Create an S3 error with optional context
    pub fn s3(message: impl Into<String>) -> Self {
        ProxyError::S3 {
            message: message.into(),
            bucket: None,
            key: None,
            operation: None,
        }
    }

    /// Create an S3 error with bucket context
    pub fn s3_with_bucket(message: impl Into<String>, bucket: impl Into<String>) -> Self {
        ProxyError::S3 {
            message: message.into(),
            bucket: Some(bucket.into()),
            key: None,
            operation: None,
        }
    }

    /// Create an S3 error with bucket and key context
    pub fn s3_with_key(
        message: impl Into<String>,
        bucket: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        ProxyError::S3 {
            message: message.into(),
            bucket: Some(bucket.into()),
            key: Some(key.into()),
            operation: None,
        }
    }

    /// Create an S3 error with full context
    pub fn s3_with_context(
        message: impl Into<String>,
        bucket: impl Into<String>,
        key: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        ProxyError::S3 {
            message: message.into(),
            bucket: Some(bucket.into()),
            key: Some(key.into()),
            operation: Some(operation.into()),
        }
    }

    /// Create an Internal error with optional context
    pub fn internal(message: impl Into<String>) -> Self {
        ProxyError::Internal {
            message: message.into(),
            operation: None,
            details: None,
        }
    }

    /// Create an Internal error with operation context
    pub fn internal_with_operation(
        message: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        ProxyError::Internal {
            message: message.into(),
            operation: Some(operation.into()),
            details: None,
        }
    }

    /// Create an Internal error with full context
    pub fn internal_with_context(
        message: impl Into<String>,
        operation: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        ProxyError::Internal {
            message: message.into(),
            operation: Some(operation.into()),
            details: Some(details.into()),
        }
    }
}
