// Unit tests extracted from implementation files for better readability
// This file acts as the entry point for all unit tests in tests/unit/

mod unit {
    mod router_tests;
    mod config_tests;
    mod proxy_tests;
    mod s3_tests;
    mod auth_tests;
    mod server_tests;  // Phase 12: Pingora Server Setup
    mod pipeline_tests;  // Phase 13: Request Pipeline Integration
}
