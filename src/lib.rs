// Yatagarasu S3 Proxy Library
// Module declarations will be added as we implement them

pub mod admin; // Phase 1 (v1.3): Admin API
pub mod audit; // Phase 33: Audit Logging
pub mod auth;
pub mod cache;
pub mod circuit_breaker; // Phase 21: Circuit Breaker Pattern
pub mod compression; // Phase 40: Request/Response Compression
pub mod config;
pub mod constants; // Centralized default values
pub mod error;
pub mod logging;
pub mod metrics; // Phase 18: Prometheus Metrics
pub mod observability; // Phase 34: Enhanced Observability
pub mod opa; // Phase 32: OPA Integration
pub mod openfga; // Phase 48: OpenFGA Integration
pub mod pipeline; // Phase 13: Request Pipeline Integration
pub mod proxy;
pub mod rate_limit; // Phase 21: Rate Limiting
pub mod reload; // Phase 19: Configuration Hot Reload
pub mod replica_set; // Phase 23: High Availability Bucket Replication
pub mod request_coalescing; // Phase 38: Request Coalescing
pub mod resources; // Phase 21: Resource Monitoring & Exhaustion Prevention
pub mod retry; // Phase 21: Retry Logic with Exponential Backoff
pub mod router;
pub mod s3;
pub mod security; // Phase 21: Security Validations (request size, headers, path traversal)
pub mod server; // Phase 12: Pingora Server Setup // Phase 15: Error Handling & Logging
