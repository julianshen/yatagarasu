// Proxy module - Pingora ProxyHttp implementation
// Implements the HTTP proxy logic for Yatagarasu S3 proxy

use async_trait::async_trait;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::{authenticate_request, AuthError};
use crate::config::Config;
use crate::pipeline::RequestContext;
use crate::router::Router;
use crate::s3::build_get_object_request;

/// YatagarasuProxy implements the Pingora ProxyHttp trait
/// Handles routing, authentication, and S3 proxying
pub struct YatagarasuProxy {
    config: Arc<Config>,
    router: Router,
}

impl YatagarasuProxy {
    /// Create a new YatagarasuProxy instance from configuration
    pub fn new(config: Config) -> Self {
        let router = Router::new(config.buckets.clone());
        Self {
            config: Arc::new(config),
            router,
        }
    }

    /// Extract headers from Pingora RequestHeader into HashMap
    fn extract_headers(req: &RequestHeader) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        for (name, value) in req.headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }
        headers
    }

    /// Extract query parameters from URI
    fn extract_query_params(req: &RequestHeader) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if let Some(query) = req.uri.query() {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    params.insert(
                        key.to_string(),
                        urlencoding::decode(value).unwrap_or_default().to_string(),
                    );
                }
            }
        }
        params
    }
}

#[async_trait]
impl ProxyHttp for YatagarasuProxy {
    type CTX = RequestContext;

    /// Create a new request context for each incoming request
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new("GET".to_string(), "/".to_string())
    }

    /// Determine the upstream S3 peer for this request
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // Get bucket config from context (set in request_filter)
        let bucket_config = ctx.bucket_config().ok_or_else(|| {
            pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                "No bucket config in context",
            )
        })?;

        // Build S3 endpoint
        let endpoint = format!(
            "{}.s3.{}.amazonaws.com",
            bucket_config.s3.bucket, bucket_config.s3.region
        );

        // Create HttpPeer for S3 endpoint - need to clone endpoint for SNI
        let peer = Box::new(HttpPeer::new(
            (endpoint.clone(), 443),
            true, // use_tls
            endpoint,
        ));

        Ok(peer)
    }

    /// Filter and process incoming requests (routing and authentication)
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // Extract request information
        let req = session.req_header();
        let path = req.uri.path().to_string();
        let method = req.method.to_string();

        // Update context with request details
        ctx.set_method(method);
        ctx.set_path(path.clone());
        ctx.set_headers(Self::extract_headers(req));
        ctx.set_query_params(Self::extract_query_params(req));

        // Route request to bucket
        let bucket_config = match self.router.route(&path) {
            Some(config) => config,
            None => {
                // No matching bucket found - return 404
                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "text/plain")?;
                session
                    .write_response_header(Box::new(header), true)
                    .await?;
                return Ok(true); // Short-circuit
            }
        };

        // Store bucket config in context
        ctx.set_bucket_config(bucket_config.clone());

        // Check if authentication is required
        if let Some(auth_config) = &bucket_config.auth {
            if auth_config.enabled {
                if let Some(jwt_config) = &self.config.jwt {
                    // Authenticate request
                    let headers = ctx.headers();
                    let query_params = ctx.query_params();

                    match authenticate_request(headers, query_params, jwt_config) {
                        Ok(claims) => {
                            ctx.set_claims(claims);
                        }
                        Err(AuthError::MissingToken) => {
                            // Return 401 Unauthorized
                            let mut header = ResponseHeader::build(401, None)?;
                            header.insert_header("Content-Type", "text/plain")?;
                            header.insert_header("WWW-Authenticate", "Bearer")?;
                            session
                                .write_response_header(Box::new(header), true)
                                .await?;
                            return Ok(true); // Short-circuit
                        }
                        Err(_) => {
                            // Return 403 Forbidden (invalid token or claims)
                            let mut header = ResponseHeader::build(403, None)?;
                            header.insert_header("Content-Type", "text/plain")?;
                            session
                                .write_response_header(Box::new(header), true)
                                .await?;
                            return Ok(true); // Short-circuit
                        }
                    }
                }
            }
        }

        Ok(false) // Continue to upstream
    }

    /// Modify upstream request headers (add AWS Signature v4)
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let bucket_config = ctx.bucket_config().ok_or_else(|| {
            pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                "No bucket config in context",
            )
        })?;

        // Extract S3 key from path
        let s3_key = self.router.extract_s3_key(ctx.path()).unwrap_or_default();

        // Build S3 request
        let s3_request =
            build_get_object_request(&bucket_config.s3.bucket, &s3_key, &bucket_config.s3.region);

        // Get signed headers
        let signed_headers = s3_request
            .get_signed_headers(&bucket_config.s3.access_key, &bucket_config.s3.secret_key);

        // Add signed headers to upstream request
        // Use append_header instead of insert_header to avoid lifetime issues
        for (name, value) in signed_headers {
            let header_name =
                http::header::HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
                    pingora_core::Error::explain(
                        pingora_core::ErrorType::InternalError,
                        format!("Invalid header name: {}", e),
                    )
                })?;
            let header_value = http::header::HeaderValue::from_str(&value).map_err(|e| {
                pingora_core::Error::explain(
                    pingora_core::ErrorType::InternalError,
                    format!("Invalid header value: {}", e),
                )
            })?;
            upstream_request
                .append_header(header_name, header_value)
                .map_err(|e| {
                    pingora_core::Error::explain(
                        pingora_core::ErrorType::InternalError,
                        format!("Failed to append header: {}", e),
                    )
                })?;
        }

        // Update Host header to S3 endpoint
        let host = format!(
            "{}.s3.{}.amazonaws.com",
            bucket_config.s3.bucket, bucket_config.s3.region
        );
        upstream_request.remove_header(&http::header::HOST);
        upstream_request
            .append_header(
                http::header::HOST,
                http::header::HeaderValue::from_str(&host).map_err(|e| {
                    pingora_core::Error::explain(
                        pingora_core::ErrorType::InternalError,
                        format!("Invalid host header: {}", e),
                    )
                })?,
            )
            .map_err(|e| {
                pingora_core::Error::explain(
                    pingora_core::ErrorType::InternalError,
                    format!("Failed to set Host header: {}", e),
                )
            })?;

        // Update URI to S3 key
        let uri = format!("/{}", s3_key);
        let parsed_uri = uri.parse().map_err(|e: http::uri::InvalidUri| {
            pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                format!("Invalid URI: {}", e),
            )
        })?;
        upstream_request.set_uri(parsed_uri);

        Ok(())
    }

    /// Log request completion for metrics and debugging
    async fn logging(
        &self,
        _session: &mut Session,
        _e: Option<&pingora_core::Error>,
        ctx: &mut Self::CTX,
    ) {
        // Log request completion with request ID for tracing
        tracing::info!(
            request_id = %ctx.request_id(),
            method = %ctx.method(),
            path = %ctx.path(),
            "Request completed"
        );
    }
}
