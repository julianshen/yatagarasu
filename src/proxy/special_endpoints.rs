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
                if replicas_health
                    .values()
                    .all(|v| v.as_str() == Some("healthy"))
                {
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
    use crate::circuit_breaker::CircuitBreakerConfig;
    use crate::config::S3Config;
    use crate::replica_set::ReplicaEntry;
    use crate::s3::S3Client;

    /// Helper to create a minimal S3Config for testing
    fn test_s3_config() -> S3Config {
        S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test-key".to_string(),
            secret_key: "test-secret".to_string(),
            ..Default::default()
        }
    }

    /// Helper to create a minimal BucketConfig for testing
    fn test_bucket_config(name: &str) -> BucketConfig {
        BucketConfig {
            name: name.to_string(),
            path_prefix: format!("/{}", name),
            s3: test_s3_config(),
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
            watermark: None,
        }
    }

    /// Helper to create a ReplicaEntry with healthy (closed) circuit breaker
    fn healthy_replica(name: &str) -> ReplicaEntry {
        ReplicaEntry {
            name: name.to_string(),
            priority: 1,
            client: S3Client {
                config: test_s3_config(),
            },
            circuit_breaker: crate::circuit_breaker::CircuitBreaker::new(
                CircuitBreakerConfig::default(),
            ),
        }
    }

    /// Helper to create a ReplicaEntry with unhealthy (open) circuit breaker
    fn unhealthy_replica(name: &str) -> ReplicaEntry {
        let cb_config = CircuitBreakerConfig {
            failure_threshold: 1, // Open after 1 failure
            ..Default::default()
        };
        let cb = crate::circuit_breaker::CircuitBreaker::new(cb_config);
        // Record a failure to open the circuit
        cb.record_failure();

        ReplicaEntry {
            name: name.to_string(),
            priority: 1,
            client: S3Client {
                config: test_s3_config(),
            },
            circuit_breaker: cb,
        }
    }

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

    // ========== handle_ready tests ==========

    #[test]
    fn test_handle_ready_all_replicas_healthy() {
        // Setup: One bucket with two healthy replicas
        let buckets = vec![test_bucket_config("products")];
        let mut replica_sets = HashMap::new();
        replica_sets.insert(
            "products".to_string(),
            ReplicaSet {
                replicas: vec![healthy_replica("primary"), healthy_replica("secondary")],
            },
        );
        let metrics = Metrics::new();

        // Execute
        let response = handle_ready(&buckets, &replica_sets, &metrics);

        // Verify
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json");

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "ready");
        assert_eq!(parsed["backends"]["products"]["status"], "ready");
        assert_eq!(
            parsed["backends"]["products"]["replicas"]["primary"],
            "healthy"
        );
        assert_eq!(
            parsed["backends"]["products"]["replicas"]["secondary"],
            "healthy"
        );
    }

    #[test]
    fn test_handle_ready_some_replicas_unhealthy_returns_degraded() {
        // Setup: One bucket with one healthy and one unhealthy replica
        let buckets = vec![test_bucket_config("products")];
        let mut replica_sets = HashMap::new();
        replica_sets.insert(
            "products".to_string(),
            ReplicaSet {
                replicas: vec![healthy_replica("primary"), unhealthy_replica("secondary")],
            },
        );
        let metrics = Metrics::new();

        // Execute
        let response = handle_ready(&buckets, &replica_sets, &metrics);

        // Verify: Status 200 but bucket is "degraded"
        assert_eq!(response.status, 200);

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "ready"); // Overall still ready (at least one healthy)
        assert_eq!(parsed["backends"]["products"]["status"], "degraded");
        assert_eq!(
            parsed["backends"]["products"]["replicas"]["primary"],
            "healthy"
        );
        assert_eq!(
            parsed["backends"]["products"]["replicas"]["secondary"],
            "unhealthy"
        );
    }

    #[test]
    fn test_handle_ready_all_replicas_unhealthy_returns_unavailable() {
        // Setup: One bucket with all unhealthy replicas
        let buckets = vec![test_bucket_config("products")];
        let mut replica_sets = HashMap::new();
        replica_sets.insert(
            "products".to_string(),
            ReplicaSet {
                replicas: vec![unhealthy_replica("primary"), unhealthy_replica("secondary")],
            },
        );
        let metrics = Metrics::new();

        // Execute
        let response = handle_ready(&buckets, &replica_sets, &metrics);

        // Verify: Status 503 and bucket is "unavailable"
        assert_eq!(response.status, 503);

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "unavailable");
        assert_eq!(parsed["backends"]["products"]["status"], "unavailable");
        assert_eq!(
            parsed["backends"]["products"]["replicas"]["primary"],
            "unhealthy"
        );
        assert_eq!(
            parsed["backends"]["products"]["replicas"]["secondary"],
            "unhealthy"
        );
    }

    #[test]
    fn test_handle_ready_no_replica_set_returns_unavailable() {
        // Setup: Bucket exists in config but no ReplicaSet found
        let buckets = vec![test_bucket_config("products")];
        let replica_sets = HashMap::new(); // Empty - no replica sets
        let metrics = Metrics::new();

        // Execute
        let response = handle_ready(&buckets, &replica_sets, &metrics);

        // Verify: Status 503 and bucket is "unavailable"
        assert_eq!(response.status, 503);

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "unavailable");
        assert_eq!(parsed["backends"]["products"]["status"], "unavailable");
    }

    #[test]
    fn test_handle_ready_multiple_buckets_mixed_health() {
        // Setup: Two buckets - one healthy, one with all unhealthy replicas
        let buckets = vec![test_bucket_config("products"), test_bucket_config("images")];
        let mut replica_sets = HashMap::new();
        replica_sets.insert(
            "products".to_string(),
            ReplicaSet {
                replicas: vec![healthy_replica("primary")],
            },
        );
        replica_sets.insert(
            "images".to_string(),
            ReplicaSet {
                replicas: vec![unhealthy_replica("primary")],
            },
        );
        let metrics = Metrics::new();

        // Execute
        let response = handle_ready(&buckets, &replica_sets, &metrics);

        // Verify: Status 503 because one bucket is unavailable
        assert_eq!(response.status, 503);

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "unavailable");
        assert_eq!(parsed["backends"]["products"]["status"], "ready");
        assert_eq!(parsed["backends"]["images"]["status"], "unavailable");
    }

    #[test]
    fn test_handle_ready_empty_buckets_returns_ready() {
        // Setup: No buckets configured
        let buckets: Vec<BucketConfig> = vec![];
        let replica_sets = HashMap::new();
        let metrics = Metrics::new();

        // Execute
        let response = handle_ready(&buckets, &replica_sets, &metrics);

        // Verify: Status 200, all_healthy is true when there's nothing to check
        assert_eq!(response.status, 200);

        let parsed: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["status"], "ready");
    }
}
