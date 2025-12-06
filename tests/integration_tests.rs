// Integration tests entry point
// These tests require Docker and are marked with #[ignore]
// Run with: cargo test --test integration_tests -- --ignored

#[allow(unused)]
#[allow(clippy::all)]
mod integration {
    mod audit_s3_export_test; // Phase 33.6: S3 Export for Audit Logs
    mod cache_e2e_test;
    mod chaos_test; // Phase 37: Chaos Engineering Tests
    mod circuit_breaker_test;
    mod concurrency_test;
    mod e2e_localstack_test;
    mod error_scenarios_test;
    // mod health_test; // TODO: Fix API mismatch with test_harness
    mod hot_reload_test;
    mod jwt_auth_test;
    // mod logging_test; // TODO: Fix API mismatch with test_harness
    mod metrics_test;
    mod multibucket_test;
    mod opa_test; // Phase 32: OPA Integration
    mod openfga_e2e_test; // Phase 49: OpenFGA E2E Tests (HTTP → Proxy → OpenFGA → S3)
    mod openfga_test; // Phase 49: OpenFGA Integration
    mod range_requests_test;
    mod rate_limit_test;
    mod redis_auth_test; // Phase 53.2: Redis Advanced Configuration Tests
    mod replica_set_test;
    mod retry_test;
    mod security_test;
    mod server_basic_test;
    mod streaming_test;
    pub mod test_harness;
    mod timeout_test; // Test utilities for starting/stopping proxy
}
