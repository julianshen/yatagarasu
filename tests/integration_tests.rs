// Integration tests entry point
// These tests require Docker and are marked with #[ignore]
// Run with: cargo test --test integration_tests -- --ignored

#[allow(unused)]
#[allow(clippy::all)]
mod integration {
    mod admin_auth_test; // Phase 65.1: Admin JWT Authentication
    mod audit_log_test;
    mod audit_s3_export_test; // Phase 33.6: S3 Export for Audit Logs
    mod backend_failure_test; // Phase 59: Backend Failure Handling
    mod cache; // Cache integration tests (memory, redis, tiered)
    mod cache_e2e_test;
    mod cache_metrics_test; // Phase 65.2: Enhanced Cache Metrics
    mod cache_write_through_test; // Phase 65.3: Cache Write-Through Tests
    mod chaos_test; // Phase 37: Chaos Engineering Tests
    mod circuit_breaker_test;
    mod concurrency_test;
    mod e2e_localstack_test;
    mod error_scenarios_test;
    // mod health_test; // TODO: Fix API mismatch with test_harness
    mod hot_reload_load_test; // Phase 60: Hot Reload Under Load
    mod hot_reload_test;
    mod image_optimization_test; // Phase 50.8: Image Optimization E2E Tests
    mod jwt_auth_test;
    mod k8s_scaling_test; // Phase 64: Kubernetes Deployment Testing
    mod layer_failure_test; // Phase 54.2: Layer Failure Recovery Tests
                            // mod logging_test; // TODO: Fix API mismatch with test_harness
    mod metrics_test;
    mod multi_instance_test; // Phase 63: Multi-Instance Testing
    mod multibucket_test;
    mod opa_test; // Phase 32: OPA Integration
    mod openfga_e2e_test; // Phase 49: OpenFGA E2E Tests (HTTP → Proxy → OpenFGA → S3)
    mod openfga_test; // Phase 49: OpenFGA Integration
    mod range_requests_test;
    mod rate_limit_test;
    mod redis_auth_test; // Phase 53.2: Redis Advanced Configuration Tests
    mod replica_failover_test; // Phase 59.3: Replica Failover Integration Tests
    mod replica_set_test;
    mod request_coalescing_test; // Phase 38: Request Coalescing
    mod retry_test;
    mod security_test;
    mod server_basic_test;
    mod streaming_test;
    pub mod test_harness;
    mod timeout_test; // Test utilities for starting/stopping proxy
}
