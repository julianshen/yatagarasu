// Integration tests entry point
// These tests require Docker and are marked with #[ignore]
// Run with: cargo test --test integration_tests -- --ignored

#[allow(unused)]
#[allow(clippy::all)]
mod integration {
    mod cache_e2e_test;
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
    mod range_requests_test;
    mod rate_limit_test;
    mod replica_set_test;
    mod retry_test;
    mod security_test;
    mod server_basic_test;
    mod streaming_test;
    pub mod test_harness;
    mod timeout_test; // Test utilities for starting/stopping proxy
}
