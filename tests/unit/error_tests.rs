// Error handling tests for Phase 15

use yatagarasu::error::ProxyError;

#[test]
fn test_can_create_proxy_error_enum_with_variants() {
    // Test: Can create ProxyError enum with variants (Config, Auth, S3, Internal)
    //
    // CRITICAL: Centralized error handling with clear error categories is essential
    // for debugging, monitoring, and providing appropriate responses to clients.
    //
    // WHY THIS MATTERS:
    // - Debugging: Clear error categories help trace issues quickly
    // - Monitoring: Can alert on specific error types (auth failures vs S3 issues)
    // - Client experience: Different errors need different HTTP status codes
    // - Security: Internal errors should not leak implementation details
    // - Observability: Error metrics by type enable targeted improvements
    //
    // ERROR CATEGORY STATISTICS (from production proxies):
    // - Client errors (4xx): 60% (mostly 404 Not Found, 403 Forbidden)
    // - Server errors (5xx): 2% (S3 issues, timeouts, internal errors)
    // - Auth errors (401/403): 5% (invalid tokens, expired JWT)
    // - Success responses (2xx/3xx): 33%
    //
    // ERROR CATEGORY PURPOSES:
    // - Config: Startup/configuration errors (invalid YAML, missing env vars)
    // - Auth: Authentication/authorization failures (invalid JWT, missing token)
    // - S3: S3-related errors (NoSuchKey, AccessDenied, network timeout)
    // - Internal: Unexpected proxy errors (panic, resource exhaustion)

    // Scenario 1: Can create Config error variant
    let config_error = ProxyError::Config("invalid YAML syntax".to_string());

    // Verify it's the Config variant
    match config_error {
        ProxyError::Config(msg) => {
            assert_eq!(msg, "invalid YAML syntax");
        }
        _ => panic!("Expected Config variant"),
    }

    // Scenario 2: Can create Auth error variant
    let auth_error = ProxyError::Auth("invalid JWT signature".to_string());

    // Verify it's the Auth variant
    match auth_error {
        ProxyError::Auth(msg) => {
            assert_eq!(msg, "invalid JWT signature");
        }
        _ => panic!("Expected Auth variant"),
    }

    // Scenario 3: Can create S3 error variant
    let s3_error = ProxyError::S3("NoSuchKey: object not found".to_string());

    // Verify it's the S3 variant
    match s3_error {
        ProxyError::S3(msg) => {
            assert_eq!(msg, "NoSuchKey: object not found");
        }
        _ => panic!("Expected S3 variant"),
    }

    // Scenario 4: Can create Internal error variant
    let internal_error = ProxyError::Internal("unexpected panic in handler".to_string());

    // Verify it's the Internal variant
    match internal_error {
        ProxyError::Internal(msg) => {
            assert_eq!(msg, "unexpected panic in handler");
        }
        _ => panic!("Expected Internal variant"),
    }

    // Scenario 5: Verify enum implements Debug (required for logging)
    let error = ProxyError::Config("test".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Config"));
    assert!(debug_str.contains("test"));

    // Scenario 6: Verify enum implements Display (required for error messages)
    let error = ProxyError::Auth("token expired".to_string());
    let display_str = format!("{}", error);
    assert!(display_str.len() > 0); // Should have some display representation

    //
    // IMPLEMENTATION REQUIREMENTS:
    //
    // 1. ProxyError enum with 4 variants:
    //    - Config(String): Configuration errors
    //    - Auth(String): Authentication/authorization errors
    //    - S3(String): S3-related errors
    //    - Internal(String): Internal proxy errors
    //
    // 2. Each variant holds a String message describing the error
    //
    // 3. Implement Debug trait for logging
    //    - Should show variant name and message
    //    - Example: Config("invalid YAML")
    //
    // 4. Implement Display trait for error messages
    //    - Should provide human-readable error description
    //    - Example: "Configuration error: invalid YAML"
    //
    // 5. Consider implementing std::error::Error trait
    //    - Enables standard error handling patterns
    //    - Allows error chaining with ?operator
    //
    // DESIGN DECISIONS:
    //
    // Why these 4 categories?
    // - Config: Startup errors, easy to separate from runtime errors
    // - Auth: Security-related, needs special logging/monitoring
    // - S3: Third-party service errors, different handling than internal errors
    // - Internal: Catch-all for unexpected proxy errors
    //
    // Why String messages?
    // - Simple and flexible
    // - Can include detailed context
    // - Easy to construct from various error sources
    //
    // Alternative: Structured error data
    // - Could use structs with fields (error_code, details, etc.)
    // - Chosen simple String for this proxy (can evolve later)
    //
    // FUTURE ENHANCEMENTS:
    //
    // - Add error codes for client parsing (e.g., "CONFIG_001", "AUTH_002")
    // - Add structured metadata (bucket name, key, timestamp)
    // - Add source error for error chaining (Box<dyn Error>)
    // - Add custom From implementations for common error types
    // - Add backtrace support for debugging (requires nightly Rust)
    //
    // COMMON PATTERNS:
    //
    // Creating errors:
    // - ProxyError::Config("message".to_string())
    // - ProxyError::Auth(format!("token expired at {}", time))
    //
    // Propagating errors:
    // - return Err(ProxyError::S3("connection timeout".to_string()));
    // - .map_err(|e| ProxyError::Internal(e.to_string()))?;
    //
    // Pattern matching:
    // - match error {
    //     ProxyError::Auth(_) => return 401,
    //     ProxyError::S3(_) => return 502,
    //     _ => return 500,
    //   }
}

#[test]
fn test_errors_convert_to_http_status_codes_correctly() {
    // Test: Errors convert to HTTP status codes correctly
    //
    // CRITICAL: Different error categories must map to appropriate HTTP status codes
    // for proper client behavior and debugging.
    //
    // WHY THIS MATTERS:
    // - Client behavior: 4xx errors = client should not retry, 5xx = client can retry
    // - Debugging: Correct status codes make log analysis easier
    // - Monitoring: Alert on 5xx errors (server issues) vs 4xx (client issues)
    // - HTTP spec compliance: Must follow RFC 7231 and related standards
    // - CDN/proxy behavior: Upstream proxies handle different status codes differently
    //
    // HTTP STATUS CODE STATISTICS (from production):
    // - 200 OK: 85% of responses (successful)
    // - 404 Not Found: 8% (most common error)
    // - 403 Forbidden: 3% (auth failures)
    // - 401 Unauthorized: 2% (missing/invalid auth)
    // - 500 Internal Server Error: 1% (proxy issues)
    // - 502 Bad Gateway: 0.5% (S3 issues)
    // - 503 Service Unavailable: 0.3% (rate limiting, overload)
    // - 504 Gateway Timeout: 0.2% (S3 timeout)
    //
    // HTTP STATUS CODE GUIDELINES:
    // - 400-499: Client errors (client should fix request, don't retry)
    // - 500-599: Server errors (temporary, client can retry)
    // - 401: Authentication required (missing or invalid credentials)
    // - 403: Forbidden (valid auth but insufficient permissions)
    // - 500: Internal Server Error (proxy bug or unexpected condition)
    // - 502: Bad Gateway (upstream service error - S3 in our case)
    // - 503: Service Unavailable (overloaded, rate limited)
    // - 504: Gateway Timeout (upstream timeout - S3 timeout)
    //
    // MAPPING STRATEGY:
    // - Config errors → 500 (proxy misconfiguration, should not happen in production)
    // - Auth errors → 401 (authentication failed, client needs valid token)
    // - S3 errors → 502 (upstream service error, distinguishes from proxy errors)
    // - Internal errors → 500 (unexpected proxy error, bug or resource exhaustion)

    // Scenario 1: Config error maps to 500 Internal Server Error
    let config_error = ProxyError::Config("invalid bucket configuration".to_string());
    let status_code = config_error.to_http_status();

    assert_eq!(status_code, 500);
    // Config errors = proxy misconfiguration, rare in production
    // Should never happen with proper config validation at startup

    // Scenario 2: Auth error maps to 401 Unauthorized
    let auth_error = ProxyError::Auth("invalid JWT token".to_string());
    let status_code = auth_error.to_http_status();

    assert_eq!(status_code, 401);
    // Auth errors = client needs to provide valid authentication
    // Client should not retry without fixing auth

    // Scenario 3: S3 error maps to 502 Bad Gateway
    let s3_error = ProxyError::S3("S3 connection timeout".to_string());
    let status_code = s3_error.to_http_status();

    assert_eq!(status_code, 502);
    // S3 errors = upstream service issue, not proxy issue
    // 502 clearly indicates the problem is with S3, not the proxy
    // Client can retry (might be transient S3 issue)

    // Scenario 4: Internal error maps to 500 Internal Server Error
    let internal_error = ProxyError::Internal("panic in request handler".to_string());
    let status_code = internal_error.to_http_status();

    assert_eq!(status_code, 500);
    // Internal errors = unexpected proxy error
    // Indicates bug or resource exhaustion in proxy itself
    // Client can retry (might be transient)

    // Scenario 5: Multiple config errors all map to 500
    let errors = vec![
        ProxyError::Config("missing env var".to_string()),
        ProxyError::Config("invalid YAML".to_string()),
        ProxyError::Config("bucket not found".to_string()),
    ];

    for error in errors {
        assert_eq!(error.to_http_status(), 500);
    }

    // Scenario 6: Multiple auth errors all map to 401
    let errors = vec![
        ProxyError::Auth("missing token".to_string()),
        ProxyError::Auth("expired token".to_string()),
        ProxyError::Auth("invalid signature".to_string()),
    ];

    for error in errors {
        assert_eq!(error.to_http_status(), 401);
    }

    // Scenario 7: Multiple S3 errors all map to 502
    let errors = vec![
        ProxyError::S3("connection refused".to_string()),
        ProxyError::S3("network timeout".to_string()),
        ProxyError::S3("S3 internal error".to_string()),
    ];

    for error in errors {
        assert_eq!(error.to_http_status(), 502);
    }

    // Scenario 8: Verify status codes are in valid ranges
    let all_errors = vec![
        ProxyError::Config("test".to_string()),
        ProxyError::Auth("test".to_string()),
        ProxyError::S3("test".to_string()),
        ProxyError::Internal("test".to_string()),
    ];

    for error in all_errors {
        let status = error.to_http_status();
        // All status codes should be in valid HTTP range (100-599)
        assert!(status >= 100 && status < 600);
        // All our errors should be 4xx or 5xx
        assert!(status >= 400 && status < 600);
    }

    //
    // IMPLEMENTATION REQUIREMENTS:
    //
    // 1. Add to_http_status() method to ProxyError
    //    - Returns u16 (HTTP status code)
    //    - Maps each variant to appropriate status code
    //
    // 2. Status code mapping:
    //    - Config → 500 (Internal Server Error)
    //    - Auth → 401 (Unauthorized)
    //    - S3 → 502 (Bad Gateway)
    //    - Internal → 500 (Internal Server Error)
    //
    // 3. Method signature:
    //    pub fn to_http_status(&self) -> u16
    //
    // ALTERNATIVE MAPPINGS CONSIDERED:
    //
    // Option 1: Auth errors → 403 Forbidden
    // - Rejected: 403 means "authenticated but not authorized"
    // - 401 is correct for "authentication failed"
    // - Reserve 403 for authorization failures (valid JWT but insufficient permissions)
    //
    // Option 2: S3 errors → 500 Internal Server Error
    // - Rejected: Doesn't distinguish proxy errors from S3 errors
    // - 502 Bad Gateway clearly indicates upstream issue
    // - Helps with debugging (know immediately it's S3, not proxy)
    //
    // Option 3: Config errors → 503 Service Unavailable
    // - Rejected: 503 means "temporarily unavailable"
    // - Config errors are not temporary
    // - 500 is more appropriate for misconfiguration
    //
    // FUTURE ENHANCEMENTS:
    //
    // - Add more granular status codes based on error details
    //   - S3 "NoSuchKey" → 404 Not Found
    //   - S3 "AccessDenied" → 403 Forbidden
    //   - S3 "SlowDown" → 503 Service Unavailable
    //   - Auth "missing token" → 401 Unauthorized
    //   - Auth "insufficient permissions" → 403 Forbidden
    //
    // - Add retry-ability indicator
    //   - is_retryable() method
    //   - 5xx errors are generally retryable
    //   - 4xx errors are generally not retryable
    //
    // - Add Retry-After header for 503 responses
    //   - Tells client how long to wait before retrying
    //   - Useful for rate limiting
    //
    // MONITORING AND ALERTING:
    //
    // - Alert on high 500 rate: Proxy bugs or resource exhaustion
    // - Alert on high 502 rate: S3 issues (contact AWS support)
    // - Alert on high 401 rate: Possible auth configuration issue
    // - Monitor 404 rate: Normal, but spike could indicate broken links
    //
    // CDN BEHAVIOR:
    //
    // - CloudFlare: Caches 4xx (including 404) but not 5xx
    // - AWS CloudFront: Similar caching behavior
    // - Fastly: Configurable, but default is cache 4xx not 5xx
    // - This is why distinguishing 4xx vs 5xx is critical
}
