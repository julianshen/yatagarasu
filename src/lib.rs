// Yatagarasu S3 Proxy Library
// Module declarations will be added as we implement them

pub mod auth;
pub mod cache;
pub mod circuit_breaker; // Phase 21: Circuit Breaker Pattern
pub mod config;
pub mod error;
pub mod logging;
pub mod metrics; // Phase 18: Prometheus Metrics
pub mod pipeline; // Phase 13: Request Pipeline Integration
pub mod proxy;
pub mod rate_limit; // Phase 21: Rate Limiting
pub mod reload; // Phase 19: Configuration Hot Reload
pub mod resources; // Phase 21: Resource Monitoring & Exhaustion Prevention
pub mod router;
pub mod s3;
pub mod server; // Phase 12: Pingora Server Setup // Phase 15: Error Handling & Logging
