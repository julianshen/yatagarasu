// Integration tests entry point
// These tests require Docker and are marked with #[ignore]
// Run with: cargo test --test integration_tests -- --ignored

#[allow(unused)]
#[allow(clippy::all)]
mod integration {
    mod e2e_localstack_test;
    mod jwt_auth_test;
    mod multibucket_test;
    mod range_requests_test;
    mod server_basic_test;
}
