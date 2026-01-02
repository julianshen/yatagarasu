//! Request filter orchestration module for the proxy.
//!
//! This module provides types and helpers for orchestrating the request
//! processing pipeline. It defines the stages of request processing and
//! the outcomes at each stage, enabling a simplified orchestration flow.
//!
//! # Design
//!
//! The request filter follows a 10-stage pipeline:
//! 1. Resource checks (concurrency, load)
//! 2. Security validation
//! 3. Special endpoint handling
//! 4. Routing to bucket
//! 5. Rate limiting
//! 6. Circuit breaker
//! 7. Authentication
//! 8. Authorization
//! 9. Cache lookup
//! 10. Upstream forwarding
//!
//! Each stage can either continue to the next stage or short-circuit
//! with a response (success or error).

use std::borrow::Cow;
use std::time::Duration;

// ============================================================================
// Request Processing Stages
// ============================================================================

/// Stages of request processing in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStage {
    /// Initial stage - check resource availability.
    ResourceCheck,
    /// Security validation (method, path, headers).
    SecurityValidation,
    /// Special endpoint handling (health, metrics).
    SpecialEndpoint,
    /// Route request to bucket.
    Routing,
    /// Rate limiting check.
    RateLimiting,
    /// Circuit breaker check.
    CircuitBreaker,
    /// JWT authentication.
    Authentication,
    /// OPA/OpenFGA authorization.
    Authorization,
    /// Cache lookup.
    CacheLookup,
    /// Forward to upstream.
    UpstreamForward,
}

impl RequestStage {
    /// All stages in pipeline order. Used for stage sequencing.
    const STAGES: &'static [Self] = &[
        Self::ResourceCheck,
        Self::SecurityValidation,
        Self::SpecialEndpoint,
        Self::Routing,
        Self::RateLimiting,
        Self::CircuitBreaker,
        Self::Authentication,
        Self::Authorization,
        Self::CacheLookup,
        Self::UpstreamForward,
    ];

    /// Get the next stage in the pipeline.
    pub fn next(&self) -> Option<RequestStage> {
        let current_pos = Self::STAGES.iter().position(|&s| s == *self)?;
        Self::STAGES.get(current_pos + 1).copied()
    }

    /// Get stage name for logging/metrics.
    pub fn name(&self) -> &'static str {
        match self {
            RequestStage::ResourceCheck => "resource_check",
            RequestStage::SecurityValidation => "security_validation",
            RequestStage::SpecialEndpoint => "special_endpoint",
            RequestStage::Routing => "routing",
            RequestStage::RateLimiting => "rate_limiting",
            RequestStage::CircuitBreaker => "circuit_breaker",
            RequestStage::Authentication => "authentication",
            RequestStage::Authorization => "authorization",
            RequestStage::CacheLookup => "cache_lookup",
            RequestStage::UpstreamForward => "upstream_forward",
        }
    }
}

// ============================================================================
// Stage Outcomes
// ============================================================================

/// Outcome of a request processing stage.
#[derive(Debug, Clone)]
pub enum StageOutcome {
    /// Continue to the next stage.
    Continue,
    /// Short-circuit with a response (request fully handled).
    Handled {
        /// HTTP status code of the response.
        status_code: u16,
    },
    /// Short-circuit with an error response.
    Error {
        /// HTTP status code for the error.
        status_code: u16,
        /// Error message. Uses Cow to avoid allocation for static strings.
        message: Cow<'static, str>,
    },
}

impl StageOutcome {
    /// Create a continue outcome.
    pub fn continue_processing() -> Self {
        StageOutcome::Continue
    }

    /// Create a handled outcome (successful response sent).
    pub fn handled(status_code: u16) -> Self {
        StageOutcome::Handled { status_code }
    }

    /// Create an error outcome with a static message (no allocation).
    pub fn error(status_code: u16, message: impl Into<Cow<'static, str>>) -> Self {
        StageOutcome::Error {
            status_code,
            message: message.into(),
        }
    }

    /// Check if processing should continue.
    pub fn should_continue(&self) -> bool {
        matches!(self, StageOutcome::Continue)
    }

    /// Check if request was handled (either success or error).
    pub fn is_handled(&self) -> bool {
        !self.should_continue()
    }
}

// ============================================================================
// Resource Check Results
// ============================================================================

/// Result of resource availability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceCheckResult {
    /// Resources available, continue processing.
    Available,
    /// Concurrency limit reached.
    ConcurrencyLimitReached,
    /// System under heavy load.
    ResourceExhausted,
}

impl ResourceCheckResult {
    /// Get suggested retry delay for resource exhaustion.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ResourceCheckResult::Available => None,
            ResourceCheckResult::ConcurrencyLimitReached => Some(Duration::from_secs(5)),
            ResourceCheckResult::ResourceExhausted => Some(Duration::from_secs(10)),
        }
    }

    /// Convert to stage outcome.
    pub fn as_outcome(&self) -> StageOutcome {
        match self {
            ResourceCheckResult::Available => StageOutcome::Continue,
            ResourceCheckResult::ConcurrencyLimitReached => {
                StageOutcome::error(503, "Server has reached maximum concurrent request limit")
            }
            ResourceCheckResult::ResourceExhausted => {
                StageOutcome::error(503, "Server is under heavy load")
            }
        }
    }
}

// ============================================================================
// Special Endpoint Results
// ============================================================================

/// Result of special endpoint handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialEndpointResult {
    /// Not a special endpoint, continue normal processing.
    NotSpecial,
    /// Health check endpoint.
    HealthCheck,
    /// Metrics endpoint.
    Metrics,
    /// Cache purge endpoint.
    CachePurge,
    /// Config reload endpoint.
    ConfigReload,
}

impl SpecialEndpointResult {
    /// Check if this is a special endpoint.
    pub fn is_special(&self) -> bool {
        !matches!(self, SpecialEndpointResult::NotSpecial)
    }
}

// ============================================================================
// Routing Results
// ============================================================================

/// Result of routing a request to a bucket.
#[derive(Debug, Clone)]
pub enum RoutingResult {
    /// Successfully routed to a bucket.
    Routed {
        /// Bucket name.
        bucket_name: String,
        /// Object key within the bucket.
        object_key: String,
    },
    /// No matching route found.
    NotFound,
}

impl RoutingResult {
    /// Create a routed result.
    pub fn routed(bucket_name: impl Into<String>, object_key: impl Into<String>) -> Self {
        RoutingResult::Routed {
            bucket_name: bucket_name.into(),
            object_key: object_key.into(),
        }
    }

    /// Create a not found result.
    pub fn not_found() -> Self {
        RoutingResult::NotFound
    }

    /// Check if routing was successful.
    pub fn is_routed(&self) -> bool {
        matches!(self, RoutingResult::Routed { .. })
    }

    /// Get bucket name if routed.
    pub fn bucket_name(&self) -> Option<&str> {
        match self {
            RoutingResult::Routed { bucket_name, .. } => Some(bucket_name),
            RoutingResult::NotFound => None,
        }
    }

    /// Get object key if routed.
    pub fn object_key(&self) -> Option<&str> {
        match self {
            RoutingResult::Routed { object_key, .. } => Some(object_key),
            RoutingResult::NotFound => None,
        }
    }
}

// ============================================================================
// Rate Limit Results
// ============================================================================

/// Result of rate limit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request allowed.
    Allowed,
    /// Request rate limited.
    Limited {
        /// Seconds until rate limit resets.
        retry_after: u64,
    },
}

impl RateLimitResult {
    /// Create an allowed result.
    pub fn allowed() -> Self {
        RateLimitResult::Allowed
    }

    /// Create a limited result.
    pub fn limited(retry_after: u64) -> Self {
        RateLimitResult::Limited { retry_after }
    }

    /// Check if request is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }

    /// Convert to stage outcome.
    pub fn as_outcome(&self) -> StageOutcome {
        match self {
            RateLimitResult::Allowed => StageOutcome::Continue,
            RateLimitResult::Limited { retry_after } => StageOutcome::error(
                429,
                format!("Rate limit exceeded. Retry after {} seconds", retry_after),
            ),
        }
    }
}

// ============================================================================
// Circuit Breaker Results
// ============================================================================

/// Result of circuit breaker check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerResult {
    /// Circuit closed, request allowed.
    Closed,
    /// Circuit open, request rejected.
    Open,
    /// Circuit half-open, limited requests allowed.
    HalfOpen,
}

impl CircuitBreakerResult {
    /// Check if request should proceed.
    pub fn should_proceed(&self) -> bool {
        matches!(
            self,
            CircuitBreakerResult::Closed | CircuitBreakerResult::HalfOpen
        )
    }

    /// Convert to stage outcome.
    pub fn as_outcome(&self) -> StageOutcome {
        match self {
            CircuitBreakerResult::Closed | CircuitBreakerResult::HalfOpen => StageOutcome::Continue,
            CircuitBreakerResult::Open => StageOutcome::error(
                503,
                "Service temporarily unavailable (circuit breaker open)",
            ),
        }
    }
}

// ============================================================================
// Authentication Results
// ============================================================================

/// Result of JWT authentication.
#[derive(Debug, Clone)]
pub enum AuthenticationResult {
    /// Successfully authenticated.
    Authenticated {
        /// Subject (user ID) from JWT.
        subject: Option<String>,
        /// Custom claims from JWT.
        claims: Vec<(String, String)>,
    },
    /// Authentication not required (public bucket).
    NotRequired,
    /// Authentication failed.
    Failed {
        /// Error message. Uses Cow to avoid allocation for static strings.
        reason: Cow<'static, str>,
    },
}

impl AuthenticationResult {
    /// Create an authenticated result.
    pub fn authenticated(subject: Option<String>, claims: Vec<(String, String)>) -> Self {
        AuthenticationResult::Authenticated { subject, claims }
    }

    /// Create a not required result.
    pub fn not_required() -> Self {
        AuthenticationResult::NotRequired
    }

    /// Create a failed result.
    pub fn failed(reason: impl Into<Cow<'static, str>>) -> Self {
        AuthenticationResult::Failed {
            reason: reason.into(),
        }
    }

    /// Check if authentication succeeded (or was not required).
    pub fn is_success(&self) -> bool {
        !matches!(self, AuthenticationResult::Failed { .. })
    }

    /// Convert to stage outcome.
    pub fn as_outcome(&self) -> StageOutcome {
        match self {
            AuthenticationResult::Authenticated { .. } | AuthenticationResult::NotRequired => {
                StageOutcome::Continue
            }
            AuthenticationResult::Failed { reason } => StageOutcome::error(401, reason.clone()),
        }
    }
}

// ============================================================================
// Authorization Results
// ============================================================================

/// Result of authorization check (OPA/OpenFGA).
#[derive(Debug, Clone)]
pub enum AuthorizationResult {
    /// Access allowed.
    Allowed,
    /// Access denied.
    Denied {
        /// Reason for denial. Uses Cow to avoid allocation for static strings.
        reason: Cow<'static, str>,
    },
    /// Authorization not configured.
    NotConfigured,
}

impl AuthorizationResult {
    /// Create an allowed result.
    pub fn allowed() -> Self {
        AuthorizationResult::Allowed
    }

    /// Create a denied result.
    pub fn denied(reason: impl Into<Cow<'static, str>>) -> Self {
        AuthorizationResult::Denied {
            reason: reason.into(),
        }
    }

    /// Create a not configured result.
    pub fn not_configured() -> Self {
        AuthorizationResult::NotConfigured
    }

    /// Check if access is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(
            self,
            AuthorizationResult::Allowed | AuthorizationResult::NotConfigured
        )
    }

    /// Convert to stage outcome.
    pub fn as_outcome(&self) -> StageOutcome {
        match self {
            AuthorizationResult::Allowed | AuthorizationResult::NotConfigured => {
                StageOutcome::Continue
            }
            AuthorizationResult::Denied { reason } => StageOutcome::error(403, reason.clone()),
        }
    }
}

// ============================================================================
// Cache Lookup Results
// ============================================================================

/// Result of cache lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLookupResult {
    /// Cache hit - serve from cache.
    Hit,
    /// Cache miss - fetch from upstream.
    Miss,
    /// Conditional request matched - return 304.
    NotModified,
    /// Joined streaming coalescer as follower.
    StreamingFollower,
    /// Cache lookup error - continue to upstream.
    Error,
}

impl CacheLookupResult {
    /// Check if request should continue to upstream.
    pub fn should_fetch_upstream(&self) -> bool {
        matches!(self, CacheLookupResult::Miss | CacheLookupResult::Error)
    }

    /// Check if response can be served immediately.
    pub fn can_serve_immediately(&self) -> bool {
        matches!(
            self,
            CacheLookupResult::Hit
                | CacheLookupResult::NotModified
                | CacheLookupResult::StreamingFollower
        )
    }
}

// ============================================================================
// Final Request Filter Outcome
// ============================================================================

/// Final outcome of request_filter processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestFilterOutcome {
    /// Request was fully handled (response sent).
    Handled,
    /// Continue to upstream (return false from request_filter).
    ContinueToUpstream,
}

impl RequestFilterOutcome {
    /// Convert to the boolean return value for request_filter.
    /// `true` = handled, `false` = continue to upstream.
    pub fn as_bool(&self) -> bool {
        matches!(self, RequestFilterOutcome::Handled)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a path is a special endpoint.
///
/// Special endpoints include:
/// - `/health`, `/healthz`, `/ready` - Health checks
/// - `/metrics` - Prometheus metrics
/// - `/admin/reload` - Configuration hot reload
/// - `/admin/cache/purge` - Cache purge (global and bucket-level)
pub fn is_special_endpoint(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/healthz" | "/ready" | "/metrics" | "/admin/reload"
    ) || path.starts_with("/admin/cache/purge")
}

/// Determine the special endpoint type from path.
pub fn classify_special_endpoint(path: &str) -> SpecialEndpointResult {
    match path {
        "/health" | "/healthz" | "/ready" => SpecialEndpointResult::HealthCheck,
        "/metrics" => SpecialEndpointResult::Metrics,
        "/admin/reload" => SpecialEndpointResult::ConfigReload,
        _ if path.starts_with("/admin/cache/purge") => SpecialEndpointResult::CachePurge,
        _ => SpecialEndpointResult::NotSpecial,
    }
}

/// Check if HTTP method is safe (GET/HEAD/OPTIONS).
/// These methods should not modify server state.
pub fn is_safe_method(method: &str) -> bool {
    method.eq_ignore_ascii_case("GET")
        || method.eq_ignore_ascii_case("HEAD")
        || method.eq_ignore_ascii_case("OPTIONS")
}

/// Check if request requires authentication based on bucket config.
pub fn requires_authentication(jwt_config: Option<&str>) -> bool {
    jwt_config.is_some()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Structural verification tests --

    #[test]
    fn test_request_filter_module_exists() {
        // Phase 37.9 structural verification test
        let _ = RequestStage::ResourceCheck;
        let _ = StageOutcome::Continue;
        let _ = ResourceCheckResult::Available;
        let _ = RoutingResult::NotFound;
    }

    #[test]
    fn test_request_filter_orchestration_types_available() {
        // Verify all orchestration types are accessible
        let _ = SpecialEndpointResult::NotSpecial;
        let _ = RateLimitResult::Allowed;
        let _ = CircuitBreakerResult::Closed;
        let _ = AuthenticationResult::NotRequired;
        let _ = AuthorizationResult::Allowed;
        let _ = CacheLookupResult::Miss;
        let _ = RequestFilterOutcome::ContinueToUpstream;
    }

    // -- Request stage tests --

    #[test]
    fn test_request_stage_sequence() {
        let mut stage = RequestStage::ResourceCheck;
        let mut count = 1;

        while let Some(next) = stage.next() {
            stage = next;
            count += 1;
        }

        // Should have 10 stages total
        assert_eq!(count, 10);
        assert_eq!(stage, RequestStage::UpstreamForward);
    }

    #[test]
    fn test_request_stage_names() {
        assert_eq!(RequestStage::ResourceCheck.name(), "resource_check");
        assert_eq!(
            RequestStage::SecurityValidation.name(),
            "security_validation"
        );
        assert_eq!(RequestStage::Authentication.name(), "authentication");
        assert_eq!(RequestStage::CacheLookup.name(), "cache_lookup");
    }

    // -- Stage outcome tests --

    #[test]
    fn test_stage_outcome_continue() {
        let outcome = StageOutcome::continue_processing();
        assert!(outcome.should_continue());
        assert!(!outcome.is_handled());
    }

    #[test]
    fn test_stage_outcome_handled() {
        let outcome = StageOutcome::handled(200);
        assert!(!outcome.should_continue());
        assert!(outcome.is_handled());
    }

    #[test]
    fn test_stage_outcome_error() {
        let outcome = StageOutcome::error(404, "Not found");
        assert!(!outcome.should_continue());
        assert!(outcome.is_handled());
    }

    // -- Resource check tests --

    #[test]
    fn test_resource_check_available() {
        let result = ResourceCheckResult::Available;
        assert!(result.retry_after().is_none());
        assert!(result.as_outcome().should_continue());
    }

    #[test]
    fn test_resource_check_concurrency_limit() {
        let result = ResourceCheckResult::ConcurrencyLimitReached;
        assert_eq!(result.retry_after(), Some(Duration::from_secs(5)));
        assert!(!result.as_outcome().should_continue());
    }

    #[test]
    fn test_resource_check_exhausted() {
        let result = ResourceCheckResult::ResourceExhausted;
        assert_eq!(result.retry_after(), Some(Duration::from_secs(10)));
        assert!(!result.as_outcome().should_continue());
    }

    // -- Routing tests --

    #[test]
    fn test_routing_result_routed() {
        let result = RoutingResult::routed("my-bucket", "path/to/file.jpg");
        assert!(result.is_routed());
        assert_eq!(result.bucket_name(), Some("my-bucket"));
        assert_eq!(result.object_key(), Some("path/to/file.jpg"));
    }

    #[test]
    fn test_routing_result_not_found() {
        let result = RoutingResult::not_found();
        assert!(!result.is_routed());
        assert!(result.bucket_name().is_none());
        assert!(result.object_key().is_none());
    }

    // -- Rate limit tests --

    #[test]
    fn test_rate_limit_allowed() {
        let result = RateLimitResult::allowed();
        assert!(result.is_allowed());
        assert!(result.as_outcome().should_continue());
    }

    #[test]
    fn test_rate_limit_limited() {
        let result = RateLimitResult::limited(60);
        assert!(!result.is_allowed());
        assert!(!result.as_outcome().should_continue());
    }

    // -- Circuit breaker tests --

    #[test]
    fn test_circuit_breaker_closed() {
        let result = CircuitBreakerResult::Closed;
        assert!(result.should_proceed());
        assert!(result.as_outcome().should_continue());
    }

    #[test]
    fn test_circuit_breaker_open() {
        let result = CircuitBreakerResult::Open;
        assert!(!result.should_proceed());
        assert!(!result.as_outcome().should_continue());
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let result = CircuitBreakerResult::HalfOpen;
        assert!(result.should_proceed());
        assert!(result.as_outcome().should_continue());
    }

    // -- Authentication tests --

    #[test]
    fn test_authentication_success() {
        let result = AuthenticationResult::authenticated(
            Some("user123".to_string()),
            vec![("role".to_string(), "admin".to_string())],
        );
        assert!(result.is_success());
        assert!(result.as_outcome().should_continue());
    }

    #[test]
    fn test_authentication_not_required() {
        let result = AuthenticationResult::not_required();
        assert!(result.is_success());
        assert!(result.as_outcome().should_continue());
    }

    #[test]
    fn test_authentication_failed() {
        let result = AuthenticationResult::failed("Invalid token");
        assert!(!result.is_success());
        assert!(!result.as_outcome().should_continue());
    }

    // -- Authorization tests --

    #[test]
    fn test_authorization_allowed() {
        let result = AuthorizationResult::allowed();
        assert!(result.is_allowed());
        assert!(result.as_outcome().should_continue());
    }

    #[test]
    fn test_authorization_denied() {
        let result = AuthorizationResult::denied("Access denied");
        assert!(!result.is_allowed());
        assert!(!result.as_outcome().should_continue());
    }

    #[test]
    fn test_authorization_not_configured() {
        let result = AuthorizationResult::not_configured();
        assert!(result.is_allowed());
        assert!(result.as_outcome().should_continue());
    }

    // -- Cache lookup tests --

    #[test]
    fn test_cache_lookup_hit() {
        let result = CacheLookupResult::Hit;
        assert!(result.can_serve_immediately());
        assert!(!result.should_fetch_upstream());
    }

    #[test]
    fn test_cache_lookup_miss() {
        let result = CacheLookupResult::Miss;
        assert!(!result.can_serve_immediately());
        assert!(result.should_fetch_upstream());
    }

    #[test]
    fn test_cache_lookup_not_modified() {
        let result = CacheLookupResult::NotModified;
        assert!(result.can_serve_immediately());
        assert!(!result.should_fetch_upstream());
    }

    // -- Request filter outcome tests --

    #[test]
    fn test_request_filter_outcome_handled() {
        let outcome = RequestFilterOutcome::Handled;
        assert!(outcome.as_bool());
    }

    #[test]
    fn test_request_filter_outcome_continue() {
        let outcome = RequestFilterOutcome::ContinueToUpstream;
        assert!(!outcome.as_bool());
    }

    // -- Helper function tests --

    #[test]
    fn test_is_special_endpoint() {
        // Health endpoints
        assert!(is_special_endpoint("/health"));
        assert!(is_special_endpoint("/healthz"));
        assert!(is_special_endpoint("/ready"));
        // Metrics endpoint
        assert!(is_special_endpoint("/metrics"));
        // Admin endpoints
        assert!(is_special_endpoint("/admin/reload"));
        assert!(is_special_endpoint("/admin/cache/purge"));
        assert!(is_special_endpoint("/admin/cache/purge/bucket"));
        assert!(is_special_endpoint(
            "/admin/cache/purge/bucket/path/to/file"
        ));
        // Non-special endpoints
        assert!(!is_special_endpoint("/bucket/file.jpg"));
        assert!(!is_special_endpoint("/api/v1/data"));
        assert!(!is_special_endpoint("/admin/other"));
    }

    #[test]
    fn test_classify_special_endpoint() {
        // Health checks
        assert_eq!(
            classify_special_endpoint("/health"),
            SpecialEndpointResult::HealthCheck
        );
        assert_eq!(
            classify_special_endpoint("/healthz"),
            SpecialEndpointResult::HealthCheck
        );
        assert_eq!(
            classify_special_endpoint("/ready"),
            SpecialEndpointResult::HealthCheck
        );
        // Metrics
        assert_eq!(
            classify_special_endpoint("/metrics"),
            SpecialEndpointResult::Metrics
        );
        // Cache purge (global and bucket-level)
        assert_eq!(
            classify_special_endpoint("/admin/cache/purge"),
            SpecialEndpointResult::CachePurge
        );
        assert_eq!(
            classify_special_endpoint("/admin/cache/purge/bucket"),
            SpecialEndpointResult::CachePurge
        );
        assert_eq!(
            classify_special_endpoint("/admin/cache/purge/bucket/path/to/file"),
            SpecialEndpointResult::CachePurge
        );
        // Config reload
        assert_eq!(
            classify_special_endpoint("/admin/reload"),
            SpecialEndpointResult::ConfigReload
        );
        // Non-special
        assert_eq!(
            classify_special_endpoint("/other"),
            SpecialEndpointResult::NotSpecial
        );
    }

    #[test]
    fn test_is_safe_method() {
        assert!(is_safe_method("GET"));
        assert!(is_safe_method("HEAD"));
        assert!(is_safe_method("OPTIONS"));
        assert!(is_safe_method("get")); // Case insensitive
        assert!(!is_safe_method("POST"));
        assert!(!is_safe_method("PUT"));
        assert!(!is_safe_method("DELETE"));
    }

    #[test]
    fn test_requires_authentication() {
        assert!(requires_authentication(Some("secret")));
        assert!(!requires_authentication(None));
    }
}
