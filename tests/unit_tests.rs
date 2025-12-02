// Unit tests extracted from implementation files for better readability
// This file acts as the entry point for all unit tests in tests/unit/

#[allow(unused)]
#[allow(clippy::all)]
mod unit {
    mod auth_tests;
    mod config_tests;
    mod error_tests; // Phase 15: Error Handling & Logging
    mod head_request_test; // HEAD request method support
    mod logging_tests;
    mod opa_tests; // Phase 32: OPA Integration
    mod openfga_tests; // Phase 48: OpenFGA Integration
    mod pipeline_tests; // Phase 13: Request Pipeline Integration
    mod proxy_tests;
    mod router_tests;
    mod s3_tests;
    mod server_tests; // Phase 12: Pingora Server Setup // Phase 15: Error Handling & Logging (Structured Logging)
}
