//! Special endpoint handlers for the proxy.
//!
//! This module provides response generators for built-in endpoints:
//! - `/health` - Health check endpoint
//! - `/ready` - Readiness check with backend status
//! - `/metrics` - Prometheus metrics export
//!
//! # Design
//!
//! Functions return `EndpointResponse` instead of writing directly to session.
//! This avoids borrow checker issues and keeps response generation testable.
//! The caller handles writing the response to the session.

use std::collections::HashMap;
use std::time::Instant;

use crate::circuit_breaker::CircuitState;
use crate::config::BucketConfig;
use crate::metrics::Metrics;
use crate::replica_set::ReplicaSet;

/// Response from a special endpoint handler.
#[derive(Debug, Clone)]
pub struct EndpointResponse {
    /// HTTP status code
    pub status: u16,
    /// Content-Type header value
    pub content_type: &'static str,
    /// Response body
    pub body: String,
}

impl EndpointResponse {
    /// Create a JSON response with the given status and body.
    pub fn json(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "application/json",
            body,
        }
    }

    /// Create a plain text response (for Prometheus metrics).
    pub fn prometheus(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/plain; version=0.0.4",
            body,
        }
    }
}

/// Generate response for /health endpoint.
///
/// Returns health status with uptime and version information.
pub fn handle_health(start_time: Instant) -> EndpointResponse {
    let uptime_seconds = start_time.elapsed().as_secs();
    let version = env!("CARGO_PKG_VERSION");

    let body = serde_json::json!({
        "status": "healthy",
        "uptime_seconds": uptime_seconds,
        "version": version
    })
    .to_string();

    EndpointResponse::json(200, body)
}

/// Generate response for /ready endpoint.
///
/// Checks health of all S3 backends via circuit breaker state.
/// Returns per-replica health status for each bucket.
pub fn handle_ready(
    buckets: &[BucketConfig],
    replica_sets: &HashMap<String, ReplicaSet>,
    metrics: &Metrics,
) -> EndpointResponse {
    let mut backends_health = serde_json::Map::new();
    let mut all_healthy = true;

    for bucket_config in buckets {
        if let Some(replica_set) = replica_sets.get(&bucket_config.name) {
            // Check health of each replica via circuit breaker state
            let mut replicas_health = serde_json::Map::new();
            let mut bucket_has_healthy_replica = false;

            for replica in &replica_set.replicas {
                let is_healthy = replica.circuit_breaker.state() == CircuitState::Closed;

                if is_healthy {
                    bucket_has_healthy_replica = true;
                }

                replicas_health.insert(
                    replica.name.clone(),
                    serde_json::Value::String(if is_healthy {
                        "healthy".to_string()
                    } else {
                        "unhealthy".to_string()
                    }),
                );
            }

            // Determine overall bucket status
            let bucket_status = if bucket_has_healthy_replica {
                if replicas_health.values().all(|v| v == "healthy") {
                    "ready"
                } else {
                    "degraded" // Some replicas unhealthy but at least one healthy
                }
            } else {
                all_healthy = false;
                "unavailable" // All replicas unhealthy
            };

            // Record backend health in metrics
            metrics.set_backend_health(&bucket_config.name, bucket_has_healthy_replica);

            // Build bucket health object
            let mut bucket_health = serde_json::Map::new();
            bucket_health.insert(
                "status".to_string(),
                serde_json::Value::String(bucket_status.to_string()),
            );
            bucket_health.insert(
                "replicas".to_string(),
                serde_json::Value::Object(replicas_health),
            );

            backends_health.insert(
                bucket_config.name.clone(),
                serde_json::Value::Object(bucket_health),
            );
        } else {
            // No ReplicaSet found (shouldn't happen with proper config)
            tracing::warn!(
                bucket = %bucket_config.name,
                "No ReplicaSet found for bucket, reporting as unavailable"
            );
            all_healthy = false;

            let mut bucket_health = serde_json::Map::new();
            bucket_health.insert(
                "status".to_string(),
                serde_json::Value::String("unavailable".to_string()),
            );
            backends_health.insert(
                bucket_config.name.clone(),
                serde_json::Value::Object(bucket_health),
            );
        }
    }

    let status_code = if all_healthy { 200 } else { 503 };
    let body = serde_json::json!({
        "status": if all_healthy { "ready" } else { "unavailable" },
        "backends": backends_health
    })
    .to_string();

    EndpointResponse::json(status_code, body)
}

/// Generate response for /metrics endpoint.
///
/// Returns Prometheus-formatted metrics including circuit breaker states.
pub fn handle_metrics(metrics: &Metrics, circuit_breaker_metrics: String) -> EndpointResponse {
    let mut output = metrics.export_prometheus();
    output.push_str(&circuit_breaker_metrics);

    EndpointResponse::prometheus(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_response_json() {
        let response = EndpointResponse::json(200, r#"{"status":"ok"}"#.to_string());
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json");
        assert_eq!(response.body, r#"{"status":"ok"}"#);
    }

    #[test]
    fn test_endpoint_response_prometheus() {
        let response = EndpointResponse::prometheus("metric_name 42".to_string());
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/plain; version=0.0.4");
        assert_eq!(response.body, "metric_name 42");
    }

    #[test]
    fn test_handle_health() {
        let start_time = Instant::now();
        let response = handle_health(start_time);

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json");

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "healthy");
        assert!(parsed["uptime_seconds"].is_u64());
        assert!(parsed["version"].is_string());
    }

    #[test]
    fn test_handle_metrics() {
        let metrics = Metrics::new();
        let circuit_breaker_metrics = "cb_state{bucket=\"test\"} 0\n".to_string();

        let response = handle_metrics(&metrics, circuit_breaker_metrics);

        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/plain; version=0.0.4");
        assert!(response.body.contains("cb_state"));
    }

    #[test]
    fn test_special_endpoints_module_exists() {
        // Phase 37.2 structural verification test
        // Verify the module exports the expected types and functions
        let _ = EndpointResponse::json(200, String::new());
        let _ = handle_health as fn(Instant) -> EndpointResponse;
        let _ = handle_metrics as fn(&Metrics, String) -> EndpointResponse;
    }
}
