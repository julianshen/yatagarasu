//! JWKS Client for fetching and caching JSON Web Key Sets
//!
//! This module provides a client for fetching JWKS from remote endpoints
//! and caching them with configurable refresh intervals.

use super::jwks::{JwkError, JwkKey, Jwks};
use jsonwebtoken::DecodingKey;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Error type for JWKS client operations
#[derive(Debug)]
pub enum JwksClientError {
    /// Failed to fetch JWKS from URL
    FetchError(String),
    /// Failed to parse JWKS JSON response
    ParseError(String),
    /// No JWKS URL configured
    NotConfigured,
    /// Key not found in JWKS
    KeyNotFound(String),
    /// Failed to convert JWK to DecodingKey
    KeyConversionError(JwkError),
}

impl std::fmt::Display for JwksClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwksClientError::FetchError(msg) => {
                write!(f, "Failed to fetch JWKS: {}", msg)
            }
            JwksClientError::ParseError(msg) => {
                write!(f, "Failed to parse JWKS: {}", msg)
            }
            JwksClientError::NotConfigured => {
                write!(f, "JWKS URL not configured")
            }
            JwksClientError::KeyNotFound(kid) => {
                write!(f, "Key '{}' not found in JWKS", kid)
            }
            JwksClientError::KeyConversionError(e) => {
                write!(f, "Failed to convert JWK: {}", e)
            }
        }
    }
}

impl std::error::Error for JwksClientError {}

impl From<JwkError> for JwksClientError {
    fn from(e: JwkError) -> Self {
        JwksClientError::KeyConversionError(e)
    }
}

/// Cached JWKS with metadata
struct CachedJwks {
    jwks: Jwks,
    fetched_at: Instant,
}

/// JWKS Client configuration
#[derive(Debug, Clone)]
pub struct JwksClientConfig {
    /// URL to fetch JWKS from
    pub url: String,
    /// How often to refresh the JWKS (in seconds)
    pub refresh_interval_secs: u64,
    /// HTTP request timeout (in seconds)
    pub timeout_secs: u64,
}

impl Default for JwksClientConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            refresh_interval_secs: 3600, // 1 hour default
            timeout_secs: 30,
        }
    }
}

/// JWKS Client for fetching and caching JSON Web Key Sets
pub struct JwksClient {
    config: JwksClientConfig,
    cached: RwLock<Option<CachedJwks>>,
}

impl JwksClient {
    /// Create a new JWKS client with the given configuration
    pub fn new(config: JwksClientConfig) -> Self {
        Self {
            config,
            cached: RwLock::new(None),
        }
    }

    /// Create a JWKS client from a URL with default settings
    pub fn from_url(url: &str) -> Self {
        Self::new(JwksClientConfig {
            url: url.to_string(),
            ..Default::default()
        })
    }

    /// Check if the cached JWKS is still valid (not expired)
    pub fn is_cache_valid(&self) -> bool {
        let cached = self.cached.read();
        match &*cached {
            Some(c) => {
                let age = c.fetched_at.elapsed();
                age < Duration::from_secs(self.config.refresh_interval_secs)
            }
            None => false,
        }
    }

    /// Get the current cached JWKS, if any
    pub fn get_cached_jwks(&self) -> Option<Jwks> {
        let cached = self.cached.read();
        cached.as_ref().map(|c| c.jwks.clone())
    }

    /// Find a key in the cached JWKS by its Key ID
    pub fn find_key(&self, kid: &str) -> Option<JwkKey> {
        let cached = self.cached.read();
        cached
            .as_ref()
            .and_then(|c| c.jwks.find_key_by_kid(kid).cloned())
    }

    /// Get a DecodingKey for a specific key ID from the cached JWKS
    pub fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey, JwksClientError> {
        let key = self
            .find_key(kid)
            .ok_or_else(|| JwksClientError::KeyNotFound(kid.to_string()))?;

        key.to_decoding_key().map_err(JwksClientError::from)
    }

    /// Fetch JWKS from the configured URL and update the cache
    pub async fn fetch_and_cache(&self) -> Result<Jwks, JwksClientError> {
        if self.config.url.is_empty() {
            return Err(JwksClientError::NotConfigured);
        }

        tracing::debug!("Fetching JWKS from {}", self.config.url);

        let jwks = fetch_jwks(&self.config.url, self.config.timeout_secs).await?;

        // Update the cache
        {
            let mut cached = self.cached.write();
            *cached = Some(CachedJwks {
                jwks: jwks.clone(),
                fetched_at: Instant::now(),
            });
        }

        tracing::info!(
            "JWKS fetched and cached successfully ({} keys)",
            jwks.keys.len()
        );

        Ok(jwks)
    }

    /// Get JWKS, fetching if cache is expired or empty
    pub async fn get_jwks(&self) -> Result<Jwks, JwksClientError> {
        if self.is_cache_valid() {
            if let Some(jwks) = self.get_cached_jwks() {
                return Ok(jwks);
            }
        }

        self.fetch_and_cache().await
    }

    /// Force refresh the JWKS cache
    pub async fn refresh(&self) -> Result<Jwks, JwksClientError> {
        self.fetch_and_cache().await
    }
}

/// Thread-safe shared JWKS client
pub type SharedJwksClient = Arc<JwksClient>;

/// Fetch JWKS from a URL
async fn fetch_jwks(url: &str, timeout_secs: u64) -> Result<Jwks, JwksClientError> {
    use http_body_util::BodyExt;
    use hyper::body::Buf;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;

    // Parse the URL
    let uri: hyper::Uri = url
        .parse()
        .map_err(|e| JwksClientError::FetchError(format!("Invalid URL: {}", e)))?;

    // Determine if HTTPS
    let is_https = uri.scheme_str() == Some("https");

    // Create the HTTP client
    if is_https {
        // For HTTPS, we need TLS support
        // Note: In production, you'd want to use hyper-rustls or similar
        // For now, we'll return an error for HTTPS until TLS is configured
        return Err(JwksClientError::FetchError(
            "HTTPS not yet implemented - use HTTP for testing".to_string(),
        ));
    }

    let client: Client<_, http_body_util::Empty<bytes::Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();

    // Create the request
    let req = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri(&uri)
        .header("Accept", "application/json")
        .body(http_body_util::Empty::new())
        .map_err(|e| JwksClientError::FetchError(format!("Failed to build request: {}", e)))?;

    // Make the request with timeout
    let response = tokio::time::timeout(Duration::from_secs(timeout_secs), client.request(req))
        .await
        .map_err(|_| JwksClientError::FetchError("Request timed out".to_string()))?
        .map_err(|e| JwksClientError::FetchError(format!("Request failed: {}", e)))?;

    // Check status
    if !response.status().is_success() {
        return Err(JwksClientError::FetchError(format!(
            "HTTP {} response",
            response.status()
        )));
    }

    // Read the body
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|e| JwksClientError::FetchError(format!("Failed to read body: {}", e)))?
        .aggregate();

    // Parse the JSON
    let jwks: Jwks = serde_json::from_reader(body.reader())
        .map_err(|e| JwksClientError::ParseError(format!("Invalid JSON: {}", e)))?;

    Ok(jwks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwks_client_config_default() {
        let config = JwksClientConfig::default();
        assert_eq!(config.refresh_interval_secs, 3600);
        assert_eq!(config.timeout_secs, 30);
        assert!(config.url.is_empty());
    }

    #[test]
    fn test_jwks_client_from_url() {
        let client = JwksClient::from_url("http://example.com/.well-known/jwks.json");
        assert_eq!(
            client.config.url,
            "http://example.com/.well-known/jwks.json"
        );
    }

    #[test]
    fn test_jwks_client_cache_initially_invalid() {
        let client = JwksClient::from_url("http://example.com/.well-known/jwks.json");
        assert!(!client.is_cache_valid());
    }

    #[test]
    fn test_jwks_client_no_cached_jwks_initially() {
        let client = JwksClient::from_url("http://example.com/.well-known/jwks.json");
        assert!(client.get_cached_jwks().is_none());
    }

    #[test]
    fn test_jwks_client_key_not_found() {
        let client = JwksClient::from_url("http://example.com/.well-known/jwks.json");
        let result = client.get_decoding_key("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            JwksClientError::KeyNotFound(kid) => {
                assert_eq!(kid, "nonexistent");
            }
            other => panic!("Expected KeyNotFound error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_jwks_client_not_configured() {
        let client = JwksClient::new(JwksClientConfig::default());
        let result = client.fetch_and_cache().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            JwksClientError::NotConfigured => {}
            other => panic!("Expected NotConfigured error, got: {:?}", other),
        }
    }
}
