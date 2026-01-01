//! Routing and authorization orchestration for the proxy.
//!
//! This module provides helper functions that coordinate routing and
//! authorization checks. Each function returns a structured result that
//! the caller can use to generate the appropriate HTTP response.
//!
//! # Design
//!
//! Functions return result types instead of writing directly to session.
//! This avoids borrow checker issues and keeps the logic testable.
//! The caller is responsible for writing HTTP responses based on results.
//!
//! # Check Order
//!
//! 1. Route to bucket (find matching bucket config)
//! 2. Rate limiting (if enabled)
//! 3. Circuit breaker (if configured)
//! 4. JWT authentication (if required)
//! 5. OPA authorization (if configured)
//! 6. OpenFGA authorization (if configured)

use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use crate::auth::{authenticate_request, AuthError, Claims};
use crate::circuit_breaker::{CircuitBreaker, CircuitState};
use crate::config::{BucketConfig, JwtConfig};
use crate::metrics::Metrics;
use crate::opa::{
    AuthorizationDecision as OpaAuthorizationDecision, FailMode as OpaFailMode, OpaCache, OpaInput,
    SharedOpaClient,
};
use crate::openfga::{
    extract_user_id, http_method_to_relation,
    AuthorizationDecision as OpenFgaAuthorizationDecision, FailMode as OpenFgaFailMode,
    OpenFgaClient,
};
use crate::rate_limit::{RateLimitError, RateLimitManager};

/// Result of rate limit check.
#[derive(Debug)]
pub enum RateLimitResult {
    /// Request is allowed to proceed
    Allowed,
    /// Request is rate limited
    Limited {
        bucket_name: String,
        error: RateLimitError,
    },
}

/// Result of circuit breaker check.
#[derive(Debug, Clone)]
pub enum CircuitBreakerResult {
    /// Request is allowed (circuit closed or half-open)
    Allowed {
        /// True if in half-open state (testing recovery)
        is_half_open: bool,
    },
    /// Request is rejected (circuit open)
    Rejected {
        bucket_name: String,
        state: CircuitState,
    },
}

/// Result of JWT authentication.
#[derive(Debug)]
pub enum AuthenticationResult {
    /// Successfully authenticated with claims
    Authenticated(Claims),
    /// Token was missing (401)
    MissingToken,
    /// Token was invalid or claims check failed (403)
    InvalidToken,
    /// Authentication not required (public bucket)
    NotRequired,
}

/// Result of OPA authorization.
#[derive(Debug)]
pub enum OpaResult {
    /// Request is allowed
    Allowed,
    /// Request is allowed due to fail-open mode (with warning)
    AllowedFailOpen { error: String },
    /// Request is denied
    Denied,
    /// OPA not configured for this bucket
    NotConfigured,
}

/// Result of OpenFGA authorization.
#[derive(Debug)]
pub enum OpenFgaResult {
    /// Request is allowed
    Allowed,
    /// Request is allowed due to fail-open mode (with warning)
    AllowedFailOpen { error: String },
    /// Request is denied
    Denied,
    /// No user ID found in claims
    NoUserId { fail_mode: OpenFgaFailMode },
    /// OpenFGA not configured for this bucket
    NotConfigured,
}

// ============================================================================
// Rate Limiting
// ============================================================================

/// Check rate limits for a request.
///
/// Returns `RateLimitResult::Allowed` if all limits pass.
/// Returns `RateLimitResult::Limited` with error details if any limit is exceeded.
pub fn check_rate_limits(
    rate_limit_manager: Option<&RateLimitManager>,
    bucket_name: &str,
    client_ip: Option<IpAddr>,
) -> RateLimitResult {
    let Some(manager) = rate_limit_manager else {
        return RateLimitResult::Allowed;
    };

    match manager.check_all(bucket_name, client_ip) {
        Ok(()) => RateLimitResult::Allowed,
        Err(error) => RateLimitResult::Limited {
            bucket_name: bucket_name.to_string(),
            error,
        },
    }
}

// ============================================================================
// Circuit Breaker
// ============================================================================

/// Check circuit breaker state for a bucket.
///
/// Returns `CircuitBreakerResult::Allowed` if request should proceed.
/// Returns `CircuitBreakerResult::Rejected` if circuit is open.
///
/// Note: Caller should call `circuit_breaker.start_half_open_request()` if
/// the result is `Allowed { is_half_open: true }`.
pub fn check_circuit_breaker(
    circuit_breakers: &HashMap<String, Arc<CircuitBreaker>>,
    bucket_name: &str,
) -> CircuitBreakerResult {
    let Some(circuit_breaker) = circuit_breakers.get(bucket_name) else {
        // No circuit breaker configured - allow request
        return CircuitBreakerResult::Allowed {
            is_half_open: false,
        };
    };

    if circuit_breaker.should_allow_request() {
        CircuitBreakerResult::Allowed {
            is_half_open: circuit_breaker.state() == CircuitState::HalfOpen,
        }
    } else {
        CircuitBreakerResult::Rejected {
            bucket_name: bucket_name.to_string(),
            state: circuit_breaker.state(),
        }
    }
}

// ============================================================================
// JWT Authentication
// ============================================================================

/// Authenticate a request using JWT.
///
/// Returns authentication result based on bucket config and JWT validation.
pub fn authenticate_jwt(
    bucket_config: &BucketConfig,
    jwt_config: Option<&JwtConfig>,
    headers: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
) -> AuthenticationResult {
    // Check if auth is required for this bucket
    let Some(auth_config) = &bucket_config.auth else {
        return AuthenticationResult::NotRequired;
    };

    if !auth_config.enabled {
        return AuthenticationResult::NotRequired;
    }

    // Auth is required - check for JWT config
    let Some(jwt_config) = jwt_config else {
        // Auth required but no JWT config - treat as missing token
        return AuthenticationResult::MissingToken;
    };

    // Perform authentication
    match authenticate_request(headers, query_params, jwt_config) {
        Ok(claims) => AuthenticationResult::Authenticated(claims),
        Err(AuthError::MissingToken) => AuthenticationResult::MissingToken,
        Err(_) => AuthenticationResult::InvalidToken,
    }
}

// ============================================================================
// OPA Authorization
// ============================================================================

/// Build OPA input from request context.
pub fn build_opa_input(
    claims: Option<&Claims>,
    bucket_name: &str,
    path: &str,
    method: &str,
    client_ip: Option<String>,
) -> OpaInput {
    let jwt_claims = claims
        .map(|c| {
            serde_json::to_value(c).unwrap_or_else(|e| {
                tracing::warn!("Failed to serialize JWT claims for OPA input: {}", e);
                serde_json::json!({})
            })
        })
        .unwrap_or_else(|| serde_json::json!({}));

    OpaInput::new(
        jwt_claims,
        bucket_name.to_string(),
        path.to_string(),
        method.to_string(),
        client_ip,
    )
}

/// Authorize a request using OPA.
///
/// Checks cache first, then calls OPA if needed.
/// Returns the authorization decision.
pub async fn authorize_with_opa(
    opa_clients: &HashMap<String, SharedOpaClient>,
    opa_cache: Option<&Arc<OpaCache>>,
    bucket_config: &BucketConfig,
    opa_input: &OpaInput,
) -> OpaResult {
    let Some(opa_client) = opa_clients.get(&bucket_config.name) else {
        return OpaResult::NotConfigured;
    };

    // Get fail mode from config
    let fail_mode = bucket_config
        .authorization
        .as_ref()
        .and_then(|a| a.opa_fail_mode.as_ref())
        .map(|s| OpaFailMode::from_str(s).unwrap_or_default())
        .unwrap_or_default();

    // Check cache first
    let cache_key = opa_input.cache_key();
    let cached_decision = if let Some(cache) = opa_cache {
        cache.get(&cache_key).await
    } else {
        None
    };

    let decision = if let Some(allowed) = cached_decision {
        // Cache hit
        OpaAuthorizationDecision::from_opa_result(Ok(allowed), fail_mode)
    } else {
        // Cache miss - call OPA
        let eval_result = opa_client.evaluate(opa_input).await;
        let decision = OpaAuthorizationDecision::from_opa_result(eval_result.clone(), fail_mode);

        // Cache the result on success
        if let (Ok(allowed), Some(cache)) = (eval_result, opa_cache) {
            cache.put(cache_key, allowed).await;
        }

        decision
    };

    if decision.is_allowed() {
        if decision.is_fail_open_allow() {
            OpaResult::AllowedFailOpen {
                error: decision
                    .error()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Unknown OPA error".to_string()),
            }
        } else {
            OpaResult::Allowed
        }
    } else {
        OpaResult::Denied
    }
}

// ============================================================================
// OpenFGA Authorization
// ============================================================================

// Note: extract_user_id, build_openfga_object, and http_method_to_relation
// are imported from crate::openfga and re-exported for convenience

/// Authorize a request using OpenFGA.
///
/// Returns the authorization decision.
pub async fn authorize_with_openfga(
    openfga_clients: &HashMap<String, Arc<OpenFgaClient>>,
    bucket_config: &BucketConfig,
    claims: Option<&Claims>,
    object: &str,
    method: &str,
) -> OpenFgaResult {
    let Some(openfga_client) = openfga_clients.get(&bucket_config.name) else {
        return OpenFgaResult::NotConfigured;
    };

    // Get fail mode from config
    let fail_mode = bucket_config
        .authorization
        .as_ref()
        .and_then(|a| a.openfga_fail_mode.as_ref())
        .map(|s| OpenFgaFailMode::from_str(s).unwrap_or_default())
        .unwrap_or_default();

    // Extract user ID from claims
    let jwt_claims = claims
        .map(|c| {
            serde_json::to_value(c).unwrap_or_else(|e| {
                tracing::warn!("Failed to serialize JWT claims for OpenFGA authorization: {}", e);
                serde_json::json!({})
            })
        })
        .unwrap_or_else(|| serde_json::json!({}));

    let user_claim = bucket_config
        .authorization
        .as_ref()
        .and_then(|a| a.openfga_user_claim.as_deref());

    let Some(user_id) = extract_user_id(&jwt_claims, user_claim) else {
        return OpenFgaResult::NoUserId { fail_mode };
    };

    // Map HTTP method to relation
    let relation = http_method_to_relation(method);

    // Perform authorization check
    let check_result = openfga_client
        .check(&user_id, relation.as_str(), object)
        .await;
    let decision = OpenFgaAuthorizationDecision::from_check_result(check_result, fail_mode);

    if decision.is_allowed() {
        if decision.is_fail_open_allow() {
            OpenFgaResult::AllowedFailOpen {
                error: decision.error().unwrap_or_default().to_string(),
            }
        } else {
            OpenFgaResult::Allowed
        }
    } else {
        OpenFgaResult::Denied
    }
}

// ============================================================================
// Response Builders
// ============================================================================

/// Build JSON error response body for rate limiting.
pub fn build_rate_limit_error_body(error: &RateLimitError) -> String {
    serde_json::json!({
        "error": "Too Many Requests",
        "message": error.to_string(),
        "status": 429
    })
    .to_string()
}

/// Build JSON error response body for circuit breaker rejection.
pub fn build_circuit_breaker_error_body(bucket_name: &str) -> String {
    serde_json::json!({
        "error": "Service Temporarily Unavailable",
        "message": "S3 backend is experiencing issues. Circuit breaker is open.",
        "bucket": bucket_name,
        "status": 503
    })
    .to_string()
}

// ============================================================================
// Metrics Updates
// ============================================================================

/// Update metrics for rate limit exceeded.
pub fn record_rate_limit_exceeded(metrics: &Metrics, bucket_name: &str) {
    metrics.increment_rate_limit_exceeded(bucket_name);
    metrics.increment_status_count(429);
}

/// Update metrics for circuit breaker rejection.
pub fn record_circuit_breaker_rejected(metrics: &Metrics) {
    metrics.increment_status_count(503);
}

/// Update metrics for successful authentication.
pub fn record_auth_success(metrics: &Metrics) {
    metrics.increment_auth_success();
}

/// Update metrics for authentication failure.
pub fn record_auth_failure(metrics: &Metrics, error_type: &str, status: u16) {
    metrics.increment_auth_failure();
    metrics.increment_auth_error(error_type);
    metrics.increment_status_count(status);
}

/// Update metrics for authentication bypassed (public bucket).
pub fn record_auth_bypassed(metrics: &Metrics) {
    metrics.increment_auth_bypassed();
}

/// Update metrics for authorization denied.
pub fn record_authorization_denied(metrics: &Metrics) {
    metrics.increment_status_count(403);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_breaker::CircuitBreakerConfig;

    // ========== Rate Limit Tests ==========

    #[test]
    fn test_check_rate_limits_no_manager() {
        let result = check_rate_limits(None, "test-bucket", None);
        assert!(matches!(result, RateLimitResult::Allowed));
    }

    // ========== Circuit Breaker Tests ==========

    #[test]
    fn test_check_circuit_breaker_not_configured() {
        let breakers: HashMap<String, Arc<CircuitBreaker>> = HashMap::new();
        let result = check_circuit_breaker(&breakers, "test-bucket");
        assert!(matches!(
            result,
            CircuitBreakerResult::Allowed {
                is_half_open: false
            }
        ));
    }

    #[test]
    fn test_check_circuit_breaker_closed() {
        let mut breakers = HashMap::new();
        let cb = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        breakers.insert("test-bucket".to_string(), cb);

        let result = check_circuit_breaker(&breakers, "test-bucket");
        assert!(matches!(
            result,
            CircuitBreakerResult::Allowed {
                is_half_open: false
            }
        ));
    }

    #[test]
    fn test_check_circuit_breaker_open() {
        let mut breakers = HashMap::new();
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = Arc::new(CircuitBreaker::new(config));
        cb.record_failure(); // Open the circuit
        breakers.insert("test-bucket".to_string(), cb);

        let result = check_circuit_breaker(&breakers, "test-bucket");
        assert!(matches!(result, CircuitBreakerResult::Rejected { .. }));
    }

    // ========== JWT Authentication Tests ==========

    #[test]
    fn test_authenticate_jwt_not_required() {
        let bucket_config = BucketConfig {
            name: "test".to_string(),
            path_prefix: "/test".to_string(),
            s3: Default::default(),
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
            watermark: None,
        };

        let result = authenticate_jwt(&bucket_config, None, &HashMap::new(), &HashMap::new());
        assert!(matches!(result, AuthenticationResult::NotRequired));
    }

    // ========== OPA Helper Tests ==========

    #[test]
    fn test_build_opa_input() {
        let input = build_opa_input(
            None,
            "products",
            "/products/image.jpg",
            "GET",
            Some("1.2.3.4".to_string()),
        );

        assert_eq!(input.bucket(), "products");
        assert_eq!(input.path(), "/products/image.jpg");
        assert_eq!(input.method(), "GET");
    }

    // ========== OpenFGA Helper Tests ==========
    // Note: extract_user_id, build_openfga_object, http_method_to_relation
    // are imported from crate::openfga. Tests for those functions exist there.

    #[test]
    fn test_openfga_helpers_accessible() {
        // Verify the openfga helper functions are accessible via this module
        let claims = serde_json::json!({"sub": "alice"});
        let user_id = extract_user_id(&claims, None);
        // extract_user_id adds "user:" prefix per OpenFGA convention
        assert_eq!(user_id, Some("user:alice".to_string()));

        // build_openfga_object is tested in openfga module

        let relation = http_method_to_relation("GET");
        assert_eq!(relation.as_str(), "viewer");
    }

    // ========== Response Builder Tests ==========

    #[test]
    fn test_build_circuit_breaker_error_body() {
        let body = build_circuit_breaker_error_body("products");
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["error"], "Service Temporarily Unavailable");
        assert_eq!(parsed["bucket"], "products");
        assert_eq!(parsed["status"], 503);
    }

    // ========== Structural Verification Tests ==========

    #[test]
    fn test_routing_auth_module_exists() {
        // Phase 37.3 structural verification test
        // Verify the module exports the expected types and functions
        let _ = check_rate_limits
            as fn(Option<&RateLimitManager>, &str, Option<IpAddr>) -> RateLimitResult;
        let _ = check_circuit_breaker
            as fn(&HashMap<String, Arc<CircuitBreaker>>, &str) -> CircuitBreakerResult;
        let _ = authenticate_jwt
            as fn(
                &BucketConfig,
                Option<&JwtConfig>,
                &HashMap<String, String>,
                &HashMap<String, String>,
            ) -> AuthenticationResult;
        let _ =
            build_opa_input as fn(Option<&Claims>, &str, &str, &str, Option<String>) -> OpaInput;
    }
}
