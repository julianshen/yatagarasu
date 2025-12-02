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
