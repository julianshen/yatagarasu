//! OpenFGA client for Relationship-Based Access Control (ReBAC)
//!
//! This module provides an HTTP client for OpenFGA, enabling fine-grained
//! authorization checks based on relationships between users and objects.

use std::fmt;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Error type for OpenFGA operations
#[derive(Debug, Clone)]
pub enum Error {
    /// Invalid configuration (empty endpoint, store_id, etc.)
    InvalidConfig(String),
    /// Connection error (network failure, timeout, etc.)
    Connection(String),
    /// API error returned by OpenFGA server
    Api(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidConfig(msg) => write!(f, "OpenFGA config error: {}", msg),
            Error::Connection(msg) => write!(f, "OpenFGA connection error: {}", msg),
            Error::Api(msg) => write!(f, "OpenFGA API error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type for OpenFGA operations
pub type Result<T> = std::result::Result<T, Error>;

/// OpenFGA client for making authorization queries
#[derive(Debug, Clone)]
pub struct OpenFgaClient {
    endpoint: String,
    store_id: String,
    authorization_model_id: Option<String>,
    api_token: Option<String>,
    timeout: Duration,
    client: Client,
}

impl OpenFgaClient {
    /// Creates a new OpenFGA client
    ///
    /// # Arguments
    /// * `endpoint` - The OpenFGA server endpoint (e.g., "http://localhost:8080")
    /// * `store_id` - The OpenFGA store ID
    ///
    /// # Errors
    /// Returns an error if:
    /// - The endpoint is empty
    /// - The store_id is empty
    /// - Failed to create HTTP client
    pub fn new(endpoint: &str, store_id: &str) -> Result<Self> {
        if endpoint.is_empty() {
            return Err(Error::InvalidConfig(
                "OpenFGA endpoint cannot be empty".to_string(),
            ));
        }
        if store_id.is_empty() {
            return Err(Error::InvalidConfig(
                "OpenFGA store_id cannot be empty".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_millis(100))
            .build()
            .map_err(|e| Error::InvalidConfig(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            endpoint: endpoint.to_string(),
            store_id: store_id.to_string(),
            authorization_model_id: None,
            api_token: None,
            timeout: Duration::from_millis(100),
            client,
        })
    }

    /// Returns the endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the store ID
    pub fn store_id(&self) -> &str {
        &self.store_id
    }

    /// Returns the authorization model ID, if set
    pub fn authorization_model_id(&self) -> Option<&str> {
        self.authorization_model_id.as_deref()
    }

    /// Returns the API token, if set
    pub fn api_token(&self) -> Option<&str> {
        self.api_token.as_deref()
    }

    /// Returns the timeout duration
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Creates a new OpenFGA client builder
    pub fn builder(endpoint: &str, store_id: &str) -> OpenFgaClientBuilder {
        OpenFgaClientBuilder::new(endpoint, store_id)
    }
}

/// Builder for OpenFgaClient
#[derive(Debug, Clone)]
pub struct OpenFgaClientBuilder {
    endpoint: String,
    store_id: String,
    authorization_model_id: Option<String>,
    api_token: Option<String>,
    timeout_ms: u64,
}

impl OpenFgaClientBuilder {
    /// Creates a new builder with required fields
    pub fn new(endpoint: &str, store_id: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            store_id: store_id.to_string(),
            authorization_model_id: None,
            api_token: None,
            timeout_ms: 100,
        }
    }

    /// Sets the authorization model ID
    pub fn authorization_model_id(mut self, model_id: &str) -> Self {
        self.authorization_model_id = Some(model_id.to_string());
        self
    }

    /// Sets the API token for authentication
    pub fn api_token(mut self, token: &str) -> Self {
        self.api_token = Some(token.to_string());
        self
    }

    /// Sets the timeout in milliseconds
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = timeout;
        self
    }

    /// Builds the OpenFgaClient
    pub fn build(self) -> Result<OpenFgaClient> {
        if self.endpoint.is_empty() {
            return Err(Error::InvalidConfig(
                "OpenFGA endpoint cannot be empty".to_string(),
            ));
        }
        if self.store_id.is_empty() {
            return Err(Error::InvalidConfig(
                "OpenFGA store_id cannot be empty".to_string(),
            ));
        }

        let timeout = Duration::from_millis(self.timeout_ms);

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| Error::InvalidConfig(format!("Failed to create HTTP client: {}", e)))?;

        Ok(OpenFgaClient {
            endpoint: self.endpoint,
            store_id: self.store_id,
            authorization_model_id: self.authorization_model_id,
            api_token: self.api_token,
            timeout,
            client,
        })
    }
}

/// Tuple key for OpenFGA authorization check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TupleKey {
    /// The user identifier (e.g., "user:alice")
    pub user: String,
    /// The relation to check (e.g., "viewer", "editor", "owner")
    pub relation: String,
    /// The object identifier (e.g., "document:readme")
    pub object: String,
}

impl TupleKey {
    /// Creates a new tuple key
    pub fn new(user: &str, relation: &str, object: &str) -> Self {
        Self {
            user: user.to_string(),
            relation: relation.to_string(),
            object: object.to_string(),
        }
    }
}

/// Request body for OpenFGA Check API
#[derive(Debug, Serialize)]
struct CheckRequest {
    tuple_key: TupleKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_model_id: Option<String>,
}

/// Response from OpenFGA Check API
#[derive(Debug, Deserialize)]
struct CheckResponse {
    allowed: bool,
}

impl OpenFgaClient {
    /// Checks if a user has a specific relation to an object
    ///
    /// # Arguments
    /// * `user` - The user identifier (e.g., "user:alice")
    /// * `relation` - The relation to check (e.g., "viewer", "editor", "owner")
    /// * `object` - The object identifier (e.g., "document:readme")
    ///
    /// # Returns
    /// * `Ok(true)` - The user has the specified relation to the object
    /// * `Ok(false)` - The user does not have the specified relation
    /// * `Err(Error)` - An error occurred during the check
    ///
    /// # Errors
    /// Returns an error if:
    /// - Network connection fails
    /// - Request times out
    /// - OpenFGA server returns an error
    pub async fn check(&self, user: &str, relation: &str, object: &str) -> Result<bool> {
        let url = format!("{}/stores/{}/check", self.endpoint, self.store_id);

        let request = CheckRequest {
            tuple_key: TupleKey::new(user, relation, object),
            authorization_model_id: self.authorization_model_id.clone(),
        };

        let mut req = self.client.post(&url).json(&request);

        // Add Authorization header if API token is set
        if let Some(ref token) = self.api_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::Connection(format!("Request timed out: {}", e))
            } else if e.is_connect() {
                Error::Connection(format!("Failed to connect: {}", e))
            } else {
                Error::Connection(format!("HTTP request failed: {}", e))
            }
        })?;

        let status = response.status();

        if status.is_success() {
            let check_response: CheckResponse = response
                .json()
                .await
                .map_err(|e| Error::Api(format!("Failed to parse response: {}", e)))?;
            Ok(check_response.allowed)
        } else if status.as_u16() == 400 {
            // Bad Request - invalid tuple format
            let error_body = response.text().await.unwrap_or_default();
            Err(Error::Api(format!("Invalid request (400): {}", error_body)))
        } else if status.as_u16() == 404 {
            // Store not found
            Err(Error::Api(format!("Store '{}' not found", self.store_id)))
        } else {
            // Other API errors
            let error_body = response.text().await.unwrap_or_default();
            Err(Error::Api(format!(
                "OpenFGA API error ({}): {}",
                status.as_u16(),
                error_body
            )))
        }
    }
}

// Phase 49.2: Request Authorization Flow - Helper functions

/// Relation types for OpenFGA authorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// Read-only access (GET, HEAD)
    Viewer,
    /// Read-write access (PUT, POST)
    Editor,
    /// Full access including delete (DELETE)
    Owner,
}

impl Relation {
    /// Get the relation string for OpenFGA
    pub fn as_str(&self) -> &'static str {
        match self {
            Relation::Viewer => "viewer",
            Relation::Editor => "editor",
            Relation::Owner => "owner",
        }
    }
}

/// Extracts the user ID from JWT claims
///
/// # Arguments
/// * `claims` - JSON value containing JWT claims
/// * `claim_name` - Optional custom claim name (defaults to "sub")
///
/// # Returns
/// * `Some(user:{id})` - The user ID formatted for OpenFGA
/// * `None` - If the claim is not found or is not a string
///
/// # Examples
/// ```
/// use serde_json::json;
/// use yatagarasu::openfga::extract_user_id;
///
/// let claims = json!({"sub": "user123"});
/// let user_id = extract_user_id(&claims, None);
/// assert_eq!(user_id, Some("user:user123".to_string()));
/// ```
pub fn extract_user_id(claims: &serde_json::Value, claim_name: Option<&str>) -> Option<String> {
    let claim = claim_name.unwrap_or("sub");

    // Support nested claims using dot notation (e.g., "user.id")
    let value = if claim.contains('.') {
        let parts: Vec<&str> = claim.split('.').collect();
        let mut current = claims;
        for part in parts {
            current = current.get(part)?;
        }
        current
    } else {
        claims.get(claim)?
    };

    // Extract string value
    let id = value.as_str()?;
    Some(format!("user:{}", id))
}

/// Builds an OpenFGA object identifier from bucket and path
///
/// Object naming convention:
/// - `bucket:{bucket}` - For bucket root access
/// - `folder:{bucket}/{path}/` - For folder access (path ends with /)
/// - `file:{bucket}/{path}` - For file access
///
/// # Arguments
/// * `bucket` - The S3 bucket name
/// * `path` - The object path within the bucket
///
/// # Returns
/// The OpenFGA object identifier string
///
/// # Examples
/// ```
/// use yatagarasu::openfga::build_openfga_object;
///
/// assert_eq!(build_openfga_object("my-bucket", "docs/file.txt"), "file:my-bucket/docs/file.txt");
/// assert_eq!(build_openfga_object("my-bucket", "docs/"), "folder:my-bucket/docs/");
/// assert_eq!(build_openfga_object("my-bucket", ""), "bucket:my-bucket");
/// ```
pub fn build_openfga_object(bucket: &str, path: &str) -> String {
    // Normalize path: remove leading slash
    let normalized_path = path.trim_start_matches('/');

    if normalized_path.is_empty() {
        // Bucket root access
        format!("bucket:{}", bucket)
    } else if normalized_path.ends_with('/') {
        // Folder access
        format!("folder:{}/{}", bucket, normalized_path)
    } else {
        // File access
        format!("file:{}/{}", bucket, normalized_path)
    }
}

/// Converts an HTTP method to the required OpenFGA relation
///
/// Mapping:
/// - GET, HEAD → Viewer (read access)
/// - PUT, POST → Editor (write access)
/// - DELETE → Owner (delete access)
///
/// # Arguments
/// * `method` - The HTTP method string (case-insensitive)
///
/// # Returns
/// The corresponding Relation enum value
///
/// # Examples
/// ```
/// use yatagarasu::openfga::{http_method_to_relation, Relation};
///
/// assert_eq!(http_method_to_relation("GET"), Relation::Viewer);
/// assert_eq!(http_method_to_relation("PUT"), Relation::Editor);
/// assert_eq!(http_method_to_relation("DELETE"), Relation::Owner);
/// ```
pub fn http_method_to_relation(method: &str) -> Relation {
    match method.to_uppercase().as_str() {
        "GET" | "HEAD" => Relation::Viewer,
        "PUT" | "POST" => Relation::Editor,
        "DELETE" => Relation::Owner,
        // Default to viewer for unknown methods (most restrictive common case)
        _ => Relation::Viewer,
    }
}

// Phase 49.2: Authorization Decision Types

/// Fail mode for OpenFGA authorization
///
/// Determines behavior when OpenFGA is unreachable or returns an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailMode {
    /// Fail-open: Allow requests when OpenFGA is unavailable (less secure, higher availability)
    Open,
    /// Fail-closed: Deny requests when OpenFGA is unavailable (more secure, default)
    #[default]
    Closed,
}

impl std::str::FromStr for FailMode {
    type Err = std::convert::Infallible;

    /// Parse fail mode from string
    ///
    /// Returns Closed (deny) for unknown values as a secure default.
    /// This never fails - unknown values default to Closed.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "open" => FailMode::Open,
            _ => FailMode::Closed, // Default to closed for security
        })
    }
}

/// Result of an OpenFGA authorization decision
///
/// Captures the authorization outcome along with any error information
/// for logging and debugging purposes.
#[derive(Debug)]
pub struct AuthorizationDecision {
    /// Whether the request is allowed
    allowed: bool,
    /// Whether this decision was made due to fail-open mode
    fail_open_allow: bool,
    /// Error that occurred, if any
    error: Option<String>,
}

impl AuthorizationDecision {
    /// Create a new allowed decision
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            fail_open_allow: false,
            error: None,
        }
    }

    /// Create a new denied decision
    pub fn denied() -> Self {
        Self {
            allowed: false,
            fail_open_allow: false,
            error: None,
        }
    }

    /// Create a decision from an OpenFGA check result and fail mode
    ///
    /// # Arguments
    /// * `result` - The result from OpenFGA check (Ok(bool) or Err(Error))
    /// * `fail_mode` - The configured fail mode for handling errors
    ///
    /// # Returns
    /// An AuthorizationDecision based on the result and fail mode:
    /// - Ok(true) -> allowed
    /// - Ok(false) -> denied
    /// - Err(_) + FailMode::Open -> allowed with fail_open_allow=true
    /// - Err(_) + FailMode::Closed -> denied with error
    pub fn from_check_result(result: Result<bool>, fail_mode: FailMode) -> Self {
        match result {
            Ok(true) => Self::allowed(),
            Ok(false) => Self::denied(),
            Err(e) => match fail_mode {
                FailMode::Open => Self {
                    allowed: true,
                    fail_open_allow: true,
                    error: Some(e.to_string()),
                },
                FailMode::Closed => Self {
                    allowed: false,
                    fail_open_allow: false,
                    error: Some(e.to_string()),
                },
            },
        }
    }

    /// Check if the request is allowed
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Check if this was a fail-open allow
    pub fn is_fail_open_allow(&self) -> bool {
        self.fail_open_allow
    }

    /// Get the error message, if any
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Check if an error occurred
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creates_client_with_defaults() {
        let client = OpenFgaClientBuilder::new("http://localhost:8080", "01H0TEST")
            .build()
            .unwrap();

        assert_eq!(client.endpoint(), "http://localhost:8080");
        assert_eq!(client.store_id(), "01H0TEST");
        assert_eq!(client.authorization_model_id(), None);
        assert_eq!(client.api_token(), None);
        assert_eq!(client.timeout(), Duration::from_millis(100));
    }

    #[test]
    fn test_builder_with_all_options() {
        let client = OpenFgaClientBuilder::new("http://localhost:8080", "01H0TEST")
            .authorization_model_id("model123")
            .api_token("secret-token")
            .timeout_ms(500)
            .build()
            .unwrap();

        assert_eq!(client.authorization_model_id(), Some("model123"));
        assert_eq!(client.api_token(), Some("secret-token"));
        assert_eq!(client.timeout(), Duration::from_millis(500));
    }
}
