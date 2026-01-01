//! Upstream request handling for the proxy.
//!
//! This module provides S3 request preparation, signing, and replica selection.
//! It extracts upstream-related logic from the main proxy module for better
//! organization and testability.
//!
//! # Design
//!
//! Functions return data structures instead of modifying request headers directly.
//! This avoids borrow checker issues and keeps request preparation testable.
//! The caller handles applying headers to the upstream request.

use std::collections::HashMap;
use std::time::Duration;

use pingora_core::upstreams::peer::HttpPeer;

use crate::config::{BucketConfig, S3Config};
use crate::replica_set::{ReplicaEntry, ReplicaSet};
use crate::s3::{build_get_object_request, build_head_object_request, S3Request};

// ============================================================================
// Result Types
// ============================================================================

/// S3 credentials extracted from configuration.
#[derive(Debug, Clone)]
pub struct S3Credentials {
    /// S3 bucket name.
    pub bucket: String,
    /// AWS region.
    pub region: String,
    /// Access key ID.
    pub access_key: String,
    /// Secret access key.
    pub secret_key: String,
    /// Custom endpoint (for MinIO, etc.). None for AWS S3.
    pub endpoint: Option<String>,
    /// Connection timeout in seconds.
    pub timeout: u64,
}

impl S3Credentials {
    /// Create credentials from S3Config.
    pub fn from_s3_config(config: &S3Config) -> Self {
        Self {
            bucket: config.bucket.clone(),
            region: config.region.clone(),
            access_key: config.access_key.clone(),
            secret_key: config.secret_key.clone(),
            endpoint: config.endpoint.clone(),
            timeout: config.timeout,
        }
    }

    /// Check if this is a custom endpoint (MinIO, etc.).
    pub fn is_custom_endpoint(&self) -> bool {
        self.endpoint.is_some()
    }

    /// Get the host for AWS Signature V4 signing.
    /// For custom endpoints, returns hostname without port.
    /// For AWS S3, returns bucket.s3.region.amazonaws.com format.
    pub fn host_for_signing(&self) -> String {
        if let Some(ref custom_endpoint) = self.endpoint {
            // For custom endpoints (MinIO), use hostname WITHOUT port
            // Filter out empty strings to handle malformed URLs like "http://:9000"
            let host = custom_endpoint
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split(':')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("localhost");
            host.to_string()
        } else {
            // For AWS S3, use standard format
            format!("{}.s3.{}.amazonaws.com", self.bucket, self.region)
        }
    }

    /// Get the URI path format for S3 requests.
    /// For custom endpoints (MinIO), returns /bucket/key format.
    /// For AWS S3, returns /key format (bucket is in Host header).
    pub fn uri_path(&self, s3_key: &str) -> String {
        if self.endpoint.is_some() {
            // MinIO path-style: /bucket/key
            format!("/{}/{}", self.bucket, s3_key)
        } else {
            // AWS virtual-hosted style: /key (bucket is in Host header)
            format!("/{}", s3_key)
        }
    }
}

/// Result of replica selection.
#[derive(Debug, Clone)]
pub enum ReplicaSelection {
    /// A healthy replica was selected.
    Selected {
        /// The selected replica's name.
        replica_name: String,
        /// Credentials from the selected replica.
        credentials: S3Credentials,
    },
    /// No healthy replicas available.
    AllUnhealthy,
    /// No replica set configured for this bucket.
    NotConfigured,
}

/// Upstream peer configuration ready to create HttpPeer.
#[derive(Debug, Clone)]
pub struct UpstreamPeerConfig {
    /// Endpoint hostname or IP.
    pub endpoint: String,
    /// Port number.
    pub port: u16,
    /// Whether to use TLS.
    pub use_tls: bool,
    /// Connection timeout.
    pub connection_timeout: Duration,
    /// Read timeout.
    pub read_timeout: Duration,
    /// Write timeout.
    pub write_timeout: Duration,
}

impl UpstreamPeerConfig {
    /// Build an HttpPeer from this configuration.
    pub fn build_peer(self) -> Box<HttpPeer> {
        let mut peer = Box::new(HttpPeer::new(
            (self.endpoint.clone(), self.port),
            self.use_tls,
            self.endpoint,
        ));

        peer.options.connection_timeout = Some(self.connection_timeout);
        peer.options.read_timeout = Some(self.read_timeout);
        peer.options.write_timeout = Some(self.write_timeout);

        peer
    }
}

/// Signed headers ready to apply to upstream request.
#[derive(Debug, Clone)]
pub struct SignedRequest {
    /// Headers to add to the upstream request.
    pub headers: HashMap<String, String>,
    /// Host header value.
    pub host: String,
    /// URI path for the request.
    pub uri: String,
}

// ============================================================================
// Credential Extraction
// ============================================================================

/// Get S3 credentials from bucket config or selected replica.
///
/// # Arguments
///
/// * `bucket_config` - The bucket configuration.
/// * `replica_sets` - Map of bucket name to replica set.
/// * `replica_name` - Optional name of selected replica.
///
/// # Returns
///
/// S3 credentials to use for signing and connection.
pub fn get_s3_credentials(
    bucket_config: &BucketConfig,
    replica_sets: &HashMap<String, ReplicaSet>,
    replica_name: Option<&str>,
) -> S3Credentials {
    // If replica is selected, try to use its credentials
    if let Some(name) = replica_name {
        if let Some(replica_set) = replica_sets.get(&bucket_config.name) {
            if let Some(replica) = replica_set.replicas.iter().find(|r| r.name == name) {
                return S3Credentials::from_s3_config(&replica.client.config);
            }
        }
    }

    // Fall back to bucket config
    S3Credentials::from_s3_config(&bucket_config.s3)
}

// ============================================================================
// Replica Selection
// ============================================================================

/// Select a healthy replica from the replica set.
///
/// Returns the first replica whose circuit breaker allows requests.
/// Replicas are checked in priority order.
///
/// # Arguments
///
/// * `replica_set` - The set of replicas to select from.
///
/// # Returns
///
/// * `Some(&ReplicaEntry)` - A healthy replica.
/// * `None` - All replicas are unhealthy.
pub fn select_healthy_replica(replica_set: &ReplicaSet) -> Option<&ReplicaEntry> {
    replica_set
        .replicas
        .iter()
        .find(|r| r.circuit_breaker.should_allow_request())
}

/// Try to select a replica for a bucket.
///
/// # Arguments
///
/// * `bucket_name` - Name of the bucket.
/// * `replica_sets` - Map of bucket name to replica set.
///
/// # Returns
///
/// A `ReplicaSelection` indicating the result.
pub fn select_replica(
    bucket_name: &str,
    replica_sets: &HashMap<String, ReplicaSet>,
) -> ReplicaSelection {
    let Some(replica_set) = replica_sets.get(bucket_name) else {
        return ReplicaSelection::NotConfigured;
    };

    match select_healthy_replica(replica_set) {
        Some(replica) => ReplicaSelection::Selected {
            replica_name: replica.name.clone(),
            credentials: S3Credentials::from_s3_config(&replica.client.config),
        },
        None => ReplicaSelection::AllUnhealthy,
    }
}

// ============================================================================
// S3 Request Building
// ============================================================================

/// Build an S3 request for GET or HEAD operations.
///
/// # Arguments
///
/// * `credentials` - S3 credentials for the request.
/// * `s3_key` - The S3 object key (path).
/// * `method` - HTTP method ("GET" or "HEAD").
///
/// # Returns
///
/// An S3Request ready for signing.
pub fn build_s3_request(credentials: &S3Credentials, s3_key: &str, method: &str) -> S3Request {
    match method.to_uppercase().as_str() {
        "HEAD" => build_head_object_request(&credentials.bucket, s3_key, &credentials.region),
        _ => build_get_object_request(&credentials.bucket, s3_key, &credentials.region),
    }
}

/// Sign an S3 request and return headers ready to apply.
///
/// # Arguments
///
/// * `s3_request` - The S3 request to sign.
/// * `credentials` - S3 credentials for signing.
///
/// # Returns
///
/// A `SignedRequest` with headers, host, and URI.
pub fn sign_s3_request(s3_request: &S3Request, credentials: &S3Credentials) -> SignedRequest {
    let host = credentials.host_for_signing();

    // Get signed headers with the appropriate host
    let headers = if credentials.is_custom_endpoint() {
        s3_request.get_signed_headers_with_host(
            &credentials.access_key,
            &credentials.secret_key,
            &host,
        )
    } else {
        s3_request.get_signed_headers(&credentials.access_key, &credentials.secret_key)
    };

    // Build URI path
    // TODO: Review during integration - S3Request signs with path-style URI (/{bucket}/{key})
    // but for AWS virtual-hosted style, the actual request URI should be /{key}.
    // The existing S3 module uses path-style signing which works with MinIO and
    // AWS S3 path-style endpoints. Verify this works correctly in production.
    let uri = credentials.uri_path(&s3_request.key);

    SignedRequest { headers, host, uri }
}

// ============================================================================
// Upstream Peer Building
// ============================================================================

/// Parse endpoint string into host, port, and TLS flag.
///
/// # Arguments
///
/// * `endpoint` - Optional custom endpoint URL.
/// * `bucket` - S3 bucket name (for AWS S3 host).
/// * `region` - AWS region (for AWS S3 host).
///
/// # Returns
///
/// Tuple of (host, port, use_tls).
pub fn parse_endpoint(endpoint: Option<&str>, bucket: &str, region: &str) -> (String, u16, bool) {
    if let Some(custom_endpoint) = endpoint {
        let endpoint_str = custom_endpoint
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let use_tls = custom_endpoint.starts_with("https://");

        // Extract just the host:port part (strip any path after the first /)
        let host_port_part = endpoint_str.split('/').next().unwrap_or(endpoint_str);

        let (host, port) = if let Some((h, p)) = host_port_part.split_once(':') {
            // Handle empty host (e.g., "http://:9000")
            let host = if h.is_empty() { "localhost" } else { h };
            (
                host.to_string(),
                p.parse().unwrap_or(if use_tls { 443 } else { 80 }),
            )
        } else {
            // Handle empty host (e.g., "http://")
            let host = if host_port_part.is_empty() {
                "localhost"
            } else {
                host_port_part
            };
            (host.to_string(), if use_tls { 443 } else { 80 })
        };

        (host, port, use_tls)
    } else {
        // AWS S3 standard endpoint
        let host = format!("{}.s3.{}.amazonaws.com", bucket, region);
        (host, 443, true)
    }
}

/// Build upstream peer configuration from credentials.
///
/// # Arguments
///
/// * `credentials` - S3 credentials with endpoint and timeout info.
///
/// # Returns
///
/// An `UpstreamPeerConfig` ready to build an HttpPeer.
pub fn build_upstream_peer_config(credentials: &S3Credentials) -> UpstreamPeerConfig {
    let (endpoint, port, use_tls) = parse_endpoint(
        credentials.endpoint.as_deref(),
        &credentials.bucket,
        &credentials.region,
    );

    let timeout = Duration::from_secs(credentials.timeout);

    UpstreamPeerConfig {
        endpoint,
        port,
        use_tls,
        connection_timeout: timeout,
        read_timeout: timeout,
        write_timeout: timeout,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_s3_config() -> S3Config {
        S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test-access-key".to_string(),
            secret_key: "test-secret-key".to_string(),
            endpoint: None,
            timeout: 30,
            ..Default::default()
        }
    }

    fn test_minio_config() -> S3Config {
        S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "minio-access-key".to_string(),
            secret_key: "minio-secret-key".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            timeout: 30,
            ..Default::default()
        }
    }

    #[test]
    fn test_upstream_module_exists() {
        // Phase 37.5 structural verification test
        let _ = S3Credentials::from_s3_config(&test_s3_config());
        let _ = ReplicaSelection::NotConfigured;
        let _ = UpstreamPeerConfig {
            endpoint: "test".to_string(),
            port: 443,
            use_tls: true,
            connection_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        };
    }

    #[test]
    fn test_s3_credentials_from_config() {
        let config = test_s3_config();
        let creds = S3Credentials::from_s3_config(&config);

        assert_eq!(creds.bucket, "test-bucket");
        assert_eq!(creds.region, "us-east-1");
        assert_eq!(creds.access_key, "test-access-key");
        assert!(!creds.is_custom_endpoint());
    }

    #[test]
    fn test_s3_credentials_custom_endpoint() {
        let config = test_minio_config();
        let creds = S3Credentials::from_s3_config(&config);

        assert!(creds.is_custom_endpoint());
        assert_eq!(creds.endpoint, Some("http://localhost:9000".to_string()));
    }

    #[test]
    fn test_host_for_signing_aws() {
        let creds = S3Credentials::from_s3_config(&test_s3_config());
        let host = creds.host_for_signing();

        assert_eq!(host, "test-bucket.s3.us-east-1.amazonaws.com");
    }

    #[test]
    fn test_host_for_signing_minio() {
        let creds = S3Credentials::from_s3_config(&test_minio_config());
        let host = creds.host_for_signing();

        // Should be hostname WITHOUT port
        assert_eq!(host, "localhost");
    }

    #[test]
    fn test_uri_path_aws() {
        let creds = S3Credentials::from_s3_config(&test_s3_config());
        let uri = creds.uri_path("path/to/file.jpg");

        // AWS uses virtual-hosted style: /key
        assert_eq!(uri, "/path/to/file.jpg");
    }

    #[test]
    fn test_uri_path_minio() {
        let creds = S3Credentials::from_s3_config(&test_minio_config());
        let uri = creds.uri_path("path/to/file.jpg");

        // MinIO uses path-style: /bucket/key
        assert_eq!(uri, "/test-bucket/path/to/file.jpg");
    }

    #[test]
    fn test_build_s3_request_get() {
        let creds = S3Credentials::from_s3_config(&test_s3_config());
        let request = build_s3_request(&creds, "file.txt", "GET");

        assert_eq!(request.method, "GET");
        assert_eq!(request.bucket, "test-bucket");
        assert_eq!(request.key, "file.txt");
    }

    #[test]
    fn test_build_s3_request_head() {
        let creds = S3Credentials::from_s3_config(&test_s3_config());
        let request = build_s3_request(&creds, "file.txt", "HEAD");

        assert_eq!(request.method, "HEAD");
    }

    #[test]
    fn test_sign_s3_request() {
        let creds = S3Credentials::from_s3_config(&test_s3_config());
        let request = build_s3_request(&creds, "file.txt", "GET");
        let signed = sign_s3_request(&request, &creds);

        // Should have Authorization header
        assert!(signed.headers.contains_key("authorization"));
        // Should have host header
        assert!(signed.headers.contains_key("host"));
        // Should have x-amz-date header
        assert!(signed.headers.contains_key("x-amz-date"));
        // URI should be correct for AWS
        assert_eq!(signed.uri, "/file.txt");
    }

    #[test]
    fn test_sign_s3_request_minio() {
        let creds = S3Credentials::from_s3_config(&test_minio_config());
        let request = build_s3_request(&creds, "file.txt", "GET");
        let signed = sign_s3_request(&request, &creds);

        // URI should include bucket for MinIO
        assert_eq!(signed.uri, "/test-bucket/file.txt");
        // Host should be localhost
        assert_eq!(signed.host, "localhost");
    }

    #[test]
    fn test_parse_endpoint_aws() {
        let (host, port, tls) = parse_endpoint(None, "my-bucket", "eu-west-1");

        assert_eq!(host, "my-bucket.s3.eu-west-1.amazonaws.com");
        assert_eq!(port, 443);
        assert!(tls);
    }

    #[test]
    fn test_parse_endpoint_minio_http() {
        let (host, port, tls) =
            parse_endpoint(Some("http://minio.local:9000"), "bucket", "us-east-1");

        assert_eq!(host, "minio.local");
        assert_eq!(port, 9000);
        assert!(!tls);
    }

    #[test]
    fn test_parse_endpoint_minio_https() {
        let (host, port, tls) =
            parse_endpoint(Some("https://minio.local:9001"), "bucket", "us-east-1");

        assert_eq!(host, "minio.local");
        assert_eq!(port, 9001);
        assert!(tls);
    }

    #[test]
    fn test_build_upstream_peer_config() {
        let creds = S3Credentials::from_s3_config(&test_s3_config());
        let config = build_upstream_peer_config(&creds);

        assert_eq!(config.endpoint, "test-bucket.s3.us-east-1.amazonaws.com");
        assert_eq!(config.port, 443);
        assert!(config.use_tls);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_build_upstream_peer_config_minio() {
        let creds = S3Credentials::from_s3_config(&test_minio_config());
        let config = build_upstream_peer_config(&creds);

        assert_eq!(config.endpoint, "localhost");
        assert_eq!(config.port, 9000);
        assert!(!config.use_tls);
    }

    #[test]
    fn test_select_replica_not_configured() {
        let replica_sets: HashMap<String, ReplicaSet> = HashMap::new();
        let result = select_replica("unknown-bucket", &replica_sets);

        assert!(matches!(result, ReplicaSelection::NotConfigured));
    }

    #[test]
    fn test_get_s3_credentials_fallback() {
        let bucket_config = BucketConfig {
            name: "test".to_string(),
            path_prefix: "/test".to_string(),
            s3: test_s3_config(),
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
            watermark: None,
        };
        let replica_sets: HashMap<String, ReplicaSet> = HashMap::new();

        let creds = get_s3_credentials(&bucket_config, &replica_sets, None);

        assert_eq!(creds.bucket, "test-bucket");
        assert_eq!(creds.access_key, "test-access-key");
    }

    // ========== Edge case tests for malformed endpoints ==========

    #[test]
    fn test_host_for_signing_empty_host_defaults_to_localhost() {
        // Malformed URL like "http://:9000" should default to "localhost"
        let creds = S3Credentials {
            bucket: "test".to_string(),
            region: "us-east-1".to_string(),
            access_key: "key".to_string(),
            secret_key: "secret".to_string(),
            endpoint: Some("http://:9000".to_string()),
            timeout: 30,
        };

        assert_eq!(creds.host_for_signing(), "localhost");
    }

    #[test]
    fn test_host_for_signing_empty_endpoint_defaults_to_localhost() {
        // Completely empty endpoint after scheme should default to "localhost"
        let creds = S3Credentials {
            bucket: "test".to_string(),
            region: "us-east-1".to_string(),
            access_key: "key".to_string(),
            secret_key: "secret".to_string(),
            endpoint: Some("http://".to_string()),
            timeout: 30,
        };

        assert_eq!(creds.host_for_signing(), "localhost");
    }

    #[test]
    fn test_parse_endpoint_with_trailing_slash() {
        // Endpoints with trailing slash should parse correctly
        let (host, port, tls) =
            parse_endpoint(Some("http://localhost:9000/"), "bucket", "us-east-1");

        assert_eq!(host, "localhost");
        assert_eq!(port, 9000);
        assert!(!tls);
    }

    #[test]
    fn test_parse_endpoint_with_path() {
        // Endpoints with path should ignore the path and use host:port only
        let (host, port, tls) = parse_endpoint(
            Some("http://minio.local:9000/some/path"),
            "bucket",
            "us-east-1",
        );

        assert_eq!(host, "minio.local");
        assert_eq!(port, 9000);
        assert!(!tls);
    }

    #[test]
    fn test_parse_endpoint_empty_host_defaults_to_localhost() {
        // Malformed URL with empty host should default to localhost
        let (host, port, tls) = parse_endpoint(Some("http://:9000"), "bucket", "us-east-1");

        assert_eq!(host, "localhost");
        assert_eq!(port, 9000);
        assert!(!tls);
    }

    #[test]
    fn test_parse_endpoint_completely_empty() {
        // Completely empty endpoint (just scheme) should default to localhost
        let (host, port, tls) = parse_endpoint(Some("http://"), "bucket", "us-east-1");

        assert_eq!(host, "localhost");
        assert_eq!(port, 80);
        assert!(!tls);
    }
}
