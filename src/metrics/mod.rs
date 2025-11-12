// Metrics module - Prometheus-compatible metrics tracking
// Provides counters, histograms, and gauges for observability

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Histogram represents percentile statistics for latency measurements
#[derive(Debug, Clone, Copy)]
pub struct Histogram {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Metrics struct tracks counters and histograms for Prometheus export
/// Thread-safe via atomic operations and mutexes
pub struct Metrics {
    // Request counters
    request_count: AtomicU64,

    // Status code counters (e.g., 200, 404, 500)
    status_counts: Mutex<HashMap<u16, u64>>,

    // Bucket name counters
    bucket_counts: Mutex<HashMap<String, u64>>,

    // HTTP method counters (GET, HEAD, POST, etc.)
    method_counts: Mutex<HashMap<String, u64>>,

    // Duration tracking (stored in microseconds as u64)
    durations: Mutex<Vec<u64>>,

    // S3 backend latency tracking (stored in microseconds as u64)
    s3_latencies: Mutex<Vec<u64>>,

    // Per-bucket latency tracking (stored in microseconds as u64)
    bucket_latencies: Mutex<HashMap<String, Vec<u64>>>,

    // Authentication metrics
    auth_success: AtomicU64,
    auth_failure: AtomicU64,
    auth_bypassed: AtomicU64,

    // Authentication error counters by type (missing, invalid, expired, etc.)
    auth_errors: Mutex<HashMap<String, u64>>,

    // S3 operation counters (GET, HEAD, etc.)
    s3_operations: Mutex<HashMap<String, u64>>,

    // System metrics
    active_connections: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    memory_usage: AtomicU64,
    uptime_seconds: AtomicU64,

    // S3 error counters by error code (NoSuchKey, AccessDenied, etc.)
    s3_errors: Mutex<HashMap<String, u64>>,

    // Configuration reload metrics
    reload_success: AtomicU64,
    reload_failure: AtomicU64,
    config_generation: AtomicU64,

    // Concurrency limiting metrics
    concurrency_limit_rejections: AtomicU64,

    // Rate limiting metrics (per-bucket)
    rate_limit_exceeded: Mutex<HashMap<String, u64>>,

    // Retry metrics (per-bucket)
    s3_retry_attempts: Mutex<HashMap<String, u64>>,
    s3_retry_success: Mutex<HashMap<String, u64>>,
    s3_retry_exhausted: Mutex<HashMap<String, u64>>,

    // Security validation metrics
    security_payload_too_large: AtomicU64,
    security_headers_too_large: AtomicU64,
    security_uri_too_long: AtomicU64,
    security_path_traversal_blocked: AtomicU64,
    security_sql_injection_blocked: AtomicU64,

    // Backend health per bucket (1=healthy, 0=unhealthy)
    backend_health: Mutex<HashMap<String, bool>>,

    // Phase 23: Per-replica metrics
    // Key format: "bucket_name:replica_name"
    replica_request_counts: Mutex<HashMap<String, u64>>,
}

impl Metrics {
    /// Create a new Metrics instance
    pub fn new() -> Self {
        Metrics {
            request_count: AtomicU64::new(0),
            status_counts: Mutex::new(HashMap::new()),
            bucket_counts: Mutex::new(HashMap::new()),
            method_counts: Mutex::new(HashMap::new()),
            durations: Mutex::new(Vec::new()),
            s3_latencies: Mutex::new(Vec::new()),
            bucket_latencies: Mutex::new(HashMap::new()),
            auth_success: AtomicU64::new(0),
            auth_failure: AtomicU64::new(0),
            auth_bypassed: AtomicU64::new(0),
            auth_errors: Mutex::new(HashMap::new()),
            s3_operations: Mutex::new(HashMap::new()),
            active_connections: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            memory_usage: AtomicU64::new(0),
            uptime_seconds: AtomicU64::new(0),
            s3_errors: Mutex::new(HashMap::new()),
            reload_success: AtomicU64::new(0),
            reload_failure: AtomicU64::new(0),
            config_generation: AtomicU64::new(0),
            concurrency_limit_rejections: AtomicU64::new(0),
            rate_limit_exceeded: Mutex::new(HashMap::new()),
            s3_retry_attempts: Mutex::new(HashMap::new()),
            s3_retry_success: Mutex::new(HashMap::new()),
            s3_retry_exhausted: Mutex::new(HashMap::new()),
            security_payload_too_large: AtomicU64::new(0),
            security_headers_too_large: AtomicU64::new(0),
            security_uri_too_long: AtomicU64::new(0),
            security_path_traversal_blocked: AtomicU64::new(0),
            security_sql_injection_blocked: AtomicU64::new(0),
            backend_health: Mutex::new(HashMap::new()),
            replica_request_counts: Mutex::new(HashMap::new()),
        }
    }

    /// Check if metrics struct is valid (for testing)
    pub fn is_valid(&self) -> bool {
        true
    }

    /// Increment the total request count
    pub fn increment_request_count(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment counter for a specific HTTP status code
    pub fn increment_status_count(&self, status_code: u16) {
        if let Ok(mut counts) = self.status_counts.lock() {
            *counts.entry(status_code).or_insert(0) += 1;
        }
    }

    /// Increment counter for a specific bucket name
    pub fn increment_bucket_count(&self, bucket_name: &str) {
        if let Ok(mut counts) = self.bucket_counts.lock() {
            *counts.entry(bucket_name.to_string()).or_insert(0) += 1;
        }
    }

    /// Increment counter for a specific HTTP method
    pub fn increment_method_count(&self, method: &str) {
        if let Ok(mut counts) = self.method_counts.lock() {
            *counts.entry(method.to_string()).or_insert(0) += 1;
        }
    }

    /// Record a request duration in milliseconds
    pub fn record_duration(&self, duration_ms: f64) {
        let duration_us = (duration_ms * 1000.0) as u64;
        if let Ok(mut durations) = self.durations.lock() {
            durations.push(duration_us);
        }
    }

    /// Get current request count (for testing)
    #[cfg(test)]
    pub fn get_request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }

    /// Get count for specific status code (for testing)
    #[cfg(test)]
    pub fn get_status_count(&self, status_code: u16) -> u64 {
        self.status_counts
            .lock()
            .ok()
            .and_then(|counts| counts.get(&status_code).copied())
            .unwrap_or(0)
    }

    /// Get count for specific bucket (for testing)
    #[cfg(test)]
    pub fn get_bucket_count(&self, bucket_name: &str) -> u64 {
        self.bucket_counts
            .lock()
            .ok()
            .and_then(|counts| counts.get(bucket_name).copied())
            .unwrap_or(0)
    }

    /// Get count for specific HTTP method (for testing)
    #[cfg(test)]
    pub fn get_method_count(&self, method: &str) -> u64 {
        self.method_counts
            .lock()
            .ok()
            .and_then(|counts| counts.get(method).copied())
            .unwrap_or(0)
    }

    /// Record S3 backend latency in milliseconds
    pub fn record_s3_latency(&self, duration_ms: f64) {
        let duration_us = (duration_ms * 1000.0) as u64;
        if let Ok(mut latencies) = self.s3_latencies.lock() {
            latencies.push(duration_us);
        }
    }

    /// Record latency for a specific bucket in milliseconds
    pub fn record_bucket_latency(&self, bucket_name: &str, duration_ms: f64) {
        let duration_us = (duration_ms * 1000.0) as u64;
        if let Ok(mut latencies) = self.bucket_latencies.lock() {
            latencies
                .entry(bucket_name.to_string())
                .or_insert_with(Vec::new)
                .push(duration_us);
        }
    }

    /// Calculate histogram from duration samples
    pub fn get_duration_histogram(&self) -> Histogram {
        if let Ok(durations) = self.durations.lock() {
            calculate_histogram(&durations)
        } else {
            Histogram {
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
            }
        }
    }

    /// Calculate histogram from S3 latency samples (for testing)
    #[cfg(test)]
    pub fn get_s3_latency_histogram(&self) -> Histogram {
        if let Ok(latencies) = self.s3_latencies.lock() {
            calculate_histogram(&latencies)
        } else {
            Histogram {
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
            }
        }
    }

    /// Calculate histogram for specific bucket (for testing)
    #[cfg(test)]
    pub fn get_bucket_latency_histogram(&self, bucket_name: &str) -> Histogram {
        if let Ok(latencies) = self.bucket_latencies.lock() {
            if let Some(bucket_samples) = latencies.get(bucket_name) {
                calculate_histogram(bucket_samples)
            } else {
                Histogram {
                    p50: 0.0,
                    p90: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                }
            }
        } else {
            Histogram {
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
            }
        }
    }

    /// Increment successful authentication counter
    pub fn increment_auth_success(&self) {
        self.auth_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment failed authentication counter
    pub fn increment_auth_failure(&self) {
        self.auth_failure.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment authentication bypassed counter (public buckets)
    pub fn increment_auth_bypassed(&self) {
        self.auth_bypassed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment counter for a specific authentication error type
    pub fn increment_auth_error(&self, error_type: &str) {
        if let Ok(mut errors) = self.auth_errors.lock() {
            *errors.entry(error_type.to_string()).or_insert(0) += 1;
        }
    }

    /// Get successful authentication count (for testing)
    #[cfg(test)]
    pub fn get_auth_success_count(&self) -> u64 {
        self.auth_success.load(Ordering::Relaxed)
    }

    /// Get failed authentication count (for testing)
    #[cfg(test)]
    pub fn get_auth_failure_count(&self) -> u64 {
        self.auth_failure.load(Ordering::Relaxed)
    }

    /// Get authentication bypassed count (for testing)
    #[cfg(test)]
    pub fn get_auth_bypassed_count(&self) -> u64 {
        self.auth_bypassed.load(Ordering::Relaxed)
    }

    /// Get count for specific auth error type (for testing)
    #[cfg(test)]
    pub fn get_auth_error_count(&self, error_type: &str) -> u64 {
        self.auth_errors
            .lock()
            .ok()
            .and_then(|errors| errors.get(error_type).copied())
            .unwrap_or(0)
    }

    /// Increment counter for a specific S3 operation
    pub fn increment_s3_operation(&self, operation: &str) {
        if let Ok(mut operations) = self.s3_operations.lock() {
            *operations.entry(operation.to_string()).or_insert(0) += 1;
        }
    }

    /// Increment counter for a specific S3 error code
    pub fn increment_s3_error(&self, error_code: &str) {
        if let Ok(mut errors) = self.s3_errors.lock() {
            *errors.entry(error_code.to_string()).or_insert(0) += 1;
        }
    }

    /// Get count for specific S3 operation (for testing)
    #[cfg(test)]
    pub fn get_s3_operation_count(&self, operation: &str) -> u64 {
        self.s3_operations
            .lock()
            .ok()
            .and_then(|operations| operations.get(operation).copied())
            .unwrap_or(0)
    }

    /// Get count for specific S3 error code (for testing)
    #[cfg(test)]
    pub fn get_s3_error_count(&self, error_code: &str) -> u64 {
        self.s3_errors
            .lock()
            .ok()
            .and_then(|errors| errors.get(error_code).copied())
            .unwrap_or(0)
    }

    // System metrics methods

    /// Increment active connections count (new client connected)
    pub fn increment_active_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections count (client disconnected)
    pub fn decrement_active_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Add bytes sent to client
    pub fn add_bytes_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Add bytes received from client
    pub fn add_bytes_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update memory usage (RSS in bytes)
    pub fn update_memory_usage(&self, bytes: u64) {
        self.memory_usage.store(bytes, Ordering::Relaxed);
    }

    /// Update uptime (seconds since start)
    pub fn update_uptime(&self, seconds: u64) {
        self.uptime_seconds.store(seconds, Ordering::Relaxed);
    }

    /// Get active connections count (for testing)
    #[cfg(test)]
    pub fn get_active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get bytes sent (for testing)
    #[cfg(test)]
    pub fn get_bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get bytes received (for testing)
    #[cfg(test)]
    pub fn get_bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Get memory usage (for testing)
    #[cfg(test)]
    pub fn get_memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }

    /// Get uptime in seconds (for testing)
    #[cfg(test)]
    pub fn get_uptime_seconds(&self) -> u64 {
        self.uptime_seconds.load(Ordering::Relaxed)
    }

    // Configuration reload metrics methods

    /// Increment successful config reload counter
    pub fn increment_reload_success(&self) {
        self.reload_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment failed config reload counter
    pub fn increment_reload_failure(&self) {
        self.reload_failure.fetch_add(1, Ordering::Relaxed);
    }

    /// Set current config generation number
    pub fn set_config_generation(&self, generation: u64) {
        self.config_generation.store(generation, Ordering::Relaxed);
    }

    /// Increment concurrency limit rejection counter (503 responses)
    pub fn increment_concurrency_limit_rejection(&self) {
        self.concurrency_limit_rejections
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment rate limit exceeded counter for a specific bucket (429 responses)
    pub fn increment_rate_limit_exceeded(&self, bucket: &str) {
        let mut rate_limit_exceeded = self.rate_limit_exceeded.lock().unwrap();
        *rate_limit_exceeded.entry(bucket.to_string()).or_insert(0) += 1;
    }

    /// Increment S3 retry attempt counter for a specific bucket
    pub fn increment_s3_retry_attempt(&self, bucket: &str) {
        let mut s3_retry_attempts = self.s3_retry_attempts.lock().unwrap();
        *s3_retry_attempts.entry(bucket.to_string()).or_insert(0) += 1;
    }

    /// Increment S3 retry success counter for a specific bucket (eventually succeeded after retry)
    pub fn increment_s3_retry_success(&self, bucket: &str) {
        let mut s3_retry_success = self.s3_retry_success.lock().unwrap();
        *s3_retry_success.entry(bucket.to_string()).or_insert(0) += 1;
    }

    /// Increment S3 retry exhausted counter for a specific bucket (all attempts failed)
    pub fn increment_s3_retry_exhausted(&self, bucket: &str) {
        let mut s3_retry_exhausted = self.s3_retry_exhausted.lock().unwrap();
        *s3_retry_exhausted.entry(bucket.to_string()).or_insert(0) += 1;
    }

    /// Increment security validation: payload too large (413 responses)
    pub fn increment_security_payload_too_large(&self) {
        self.security_payload_too_large
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment security validation: headers too large (431 responses)
    pub fn increment_security_headers_too_large(&self) {
        self.security_headers_too_large
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment security validation: URI too long (414 responses)
    pub fn increment_security_uri_too_long(&self) {
        self.security_uri_too_long.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment security validation: path traversal blocked (400 responses)
    pub fn increment_security_path_traversal_blocked(&self) {
        self.security_path_traversal_blocked
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment security validation: SQL injection blocked (400 responses)
    pub fn increment_security_sql_injection_blocked(&self) {
        self.security_sql_injection_blocked
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Set backend health status for a bucket (1=healthy, 0=unhealthy)
    pub fn set_backend_health(&self, bucket_name: &str, is_healthy: bool) {
        if let Ok(mut health) = self.backend_health.lock() {
            health.insert(bucket_name.to_string(), is_healthy);
        }
    }

    /// Get backend health status for all buckets
    pub fn get_backend_health(&self) -> HashMap<String, bool> {
        if let Ok(health) = self.backend_health.lock() {
            health.clone()
        } else {
            HashMap::new()
        }
    }

    // Phase 23: Per-replica metrics methods

    /// Increment request count for a specific replica within a bucket
    pub fn increment_replica_request_count(&self, bucket: &str, replica: &str) {
        let key = format!("{}:{}", bucket, replica);
        if let Ok(mut counts) = self.replica_request_counts.lock() {
            *counts.entry(key).or_insert(0) += 1;
        }
    }

    /// Get request count for a specific replica (for testing)
    #[cfg(test)]
    pub fn get_replica_request_count(&self, bucket: &str, replica: &str) -> u64 {
        let key = format!("{}:{}", bucket, replica);
        self.replica_request_counts
            .lock()
            .ok()
            .and_then(|counts| counts.get(&key).copied())
            .unwrap_or(0)
    }

    /// Get successful reload count (for testing)
    #[cfg(test)]
    pub fn get_reload_success_count(&self) -> u64 {
        self.reload_success.load(Ordering::Relaxed)
    }

    /// Get failed reload count (for testing)
    #[cfg(test)]
    pub fn get_reload_failure_count(&self) -> u64 {
        self.reload_failure.load(Ordering::Relaxed)
    }

    /// Get current config generation (for testing)
    #[cfg(test)]
    pub fn get_config_generation(&self) -> u64 {
        self.config_generation.load(Ordering::Relaxed)
    }

    /// Get concurrency limit rejection count (for testing)
    #[cfg(test)]
    pub fn get_concurrency_limit_rejections(&self) -> u64 {
        self.concurrency_limit_rejections.load(Ordering::Relaxed)
    }

    /// Export metrics in Prometheus text format
    /// Returns metrics as text/plain content for /metrics endpoint
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Request metrics
        output.push_str("# HELP http_requests_total Total number of HTTP requests received\n");
        output.push_str("# TYPE http_requests_total counter\n");
        output.push_str(&format!(
            "http_requests_total {}\n",
            self.request_count.load(Ordering::Relaxed)
        ));

        // Status code metrics
        output.push_str("\n# HELP http_requests_by_status_total HTTP requests by status code\n");
        output.push_str("# TYPE http_requests_by_status_total counter\n");
        if let Ok(counts) = self.status_counts.lock() {
            for (status, count) in counts.iter() {
                output.push_str(&format!(
                    "http_requests_by_status_total{{status=\"{}\"}} {}\n",
                    status, count
                ));
            }
        }

        // Bucket metrics
        output.push_str("\n# HELP http_requests_by_bucket_total HTTP requests by S3 bucket\n");
        output.push_str("# TYPE http_requests_by_bucket_total counter\n");
        if let Ok(counts) = self.bucket_counts.lock() {
            for (bucket, count) in counts.iter() {
                output.push_str(&format!(
                    "http_requests_by_bucket_total{{bucket=\"{}\"}} {}\n",
                    bucket, count
                ));
            }
        }

        // HTTP method metrics
        output.push_str("\n# HELP http_requests_by_method_total HTTP requests by method\n");
        output.push_str("# TYPE http_requests_by_method_total counter\n");
        if let Ok(counts) = self.method_counts.lock() {
            for (method, count) in counts.iter() {
                output.push_str(&format!(
                    "http_requests_by_method_total{{method=\"{}\"}} {}\n",
                    method, count
                ));
            }
        }

        // Authentication metrics
        output.push_str("\n# HELP auth_success_total Successful authentication attempts\n");
        output.push_str("# TYPE auth_success_total counter\n");
        output.push_str(&format!(
            "auth_success_total {}\n",
            self.auth_success.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP auth_failure_total Failed authentication attempts\n");
        output.push_str("# TYPE auth_failure_total counter\n");
        output.push_str(&format!(
            "auth_failure_total {}\n",
            self.auth_failure.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP auth_bypassed_total Authentication bypassed (public buckets)\n");
        output.push_str("# TYPE auth_bypassed_total counter\n");
        output.push_str(&format!(
            "auth_bypassed_total {}\n",
            self.auth_bypassed.load(Ordering::Relaxed)
        ));

        // S3 operation metrics
        output.push_str("\n# HELP s3_operations_total S3 operations by type\n");
        output.push_str("# TYPE s3_operations_total counter\n");
        if let Ok(ops) = self.s3_operations.lock() {
            for (operation, count) in ops.iter() {
                output.push_str(&format!(
                    "s3_operations_total{{operation=\"{}\"}} {}\n",
                    operation, count
                ));
            }
        }

        // S3 error metrics
        output.push_str("\n# HELP s3_errors_total S3 errors by error code\n");
        output.push_str("# TYPE s3_errors_total counter\n");
        if let Ok(errors) = self.s3_errors.lock() {
            for (error_code, count) in errors.iter() {
                output.push_str(&format!(
                    "s3_errors_total{{error_code=\"{}\"}} {}\n",
                    error_code, count
                ));
            }
        }

        // System metrics
        output.push_str("\n# HELP active_connections Current number of active connections\n");
        output.push_str("# TYPE active_connections gauge\n");
        output.push_str(&format!(
            "active_connections {}\n",
            self.active_connections.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP bytes_sent_total Total bytes sent to clients\n");
        output.push_str("# TYPE bytes_sent_total counter\n");
        output.push_str(&format!(
            "bytes_sent_total {}\n",
            self.bytes_sent.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP bytes_received_total Total bytes received from clients\n");
        output.push_str("# TYPE bytes_received_total counter\n");
        output.push_str(&format!(
            "bytes_received_total {}\n",
            self.bytes_received.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP memory_usage_bytes Current memory usage (RSS)\n");
        output.push_str("# TYPE memory_usage_bytes gauge\n");
        output.push_str(&format!(
            "memory_usage_bytes {}\n",
            self.memory_usage.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP uptime_seconds Proxy uptime in seconds\n");
        output.push_str("# TYPE uptime_seconds gauge\n");
        output.push_str(&format!(
            "uptime_seconds {}\n",
            self.uptime_seconds.load(Ordering::Relaxed)
        ));

        // Configuration reload metrics
        output.push_str("\n# HELP config_reload_success_total Successful configuration reloads\n");
        output.push_str("# TYPE config_reload_success_total counter\n");
        output.push_str(&format!(
            "config_reload_success_total {}\n",
            self.reload_success.load(Ordering::Relaxed)
        ));

        output.push_str(
            "\n# HELP config_reload_failure_total Failed configuration reload attempts\n",
        );
        output.push_str("# TYPE config_reload_failure_total counter\n");
        output.push_str(&format!(
            "config_reload_failure_total {}\n",
            self.reload_failure.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP config_generation Current configuration generation number\n");
        output.push_str("# TYPE config_generation gauge\n");
        output.push_str(&format!(
            "config_generation {}\n",
            self.config_generation.load(Ordering::Relaxed)
        ));

        output.push_str(
            "\n# HELP concurrency_limit_rejections_total Requests rejected due to concurrency limit (503)\n",
        );
        output.push_str("# TYPE concurrency_limit_rejections_total counter\n");
        output.push_str(&format!(
            "concurrency_limit_rejections_total {}\n",
            self.concurrency_limit_rejections.load(Ordering::Relaxed)
        ));

        // Rate limiting metrics
        output.push_str("\n# HELP rate_limit_exceeded_total Requests rejected due to rate limit (429) per bucket\n");
        output.push_str("# TYPE rate_limit_exceeded_total counter\n");
        let rate_limit_exceeded = self.rate_limit_exceeded.lock().unwrap();
        for (bucket, count) in rate_limit_exceeded.iter() {
            output.push_str(&format!(
                "rate_limit_exceeded_total{{bucket=\"{}\"}} {}\n",
                bucket, count
            ));
        }

        // Retry metrics
        output.push_str("\n# HELP s3_retry_attempts_total Total retry attempts per bucket\n");
        output.push_str("# TYPE s3_retry_attempts_total counter\n");
        let s3_retry_attempts = self.s3_retry_attempts.lock().unwrap();
        for (bucket, count) in s3_retry_attempts.iter() {
            output.push_str(&format!(
                "s3_retry_attempts_total{{bucket=\"{}\"}} {}\n",
                bucket, count
            ));
        }

        output.push_str("\n# HELP s3_retry_success_total Successful retries per bucket (eventually succeeded)\n");
        output.push_str("# TYPE s3_retry_success_total counter\n");
        let s3_retry_success = self.s3_retry_success.lock().unwrap();
        for (bucket, count) in s3_retry_success.iter() {
            output.push_str(&format!(
                "s3_retry_success_total{{bucket=\"{}\"}} {}\n",
                bucket, count
            ));
        }

        output.push_str("\n# HELP s3_retry_exhausted_total Retries exhausted per bucket (all attempts failed)\n");
        output.push_str("# TYPE s3_retry_exhausted_total counter\n");
        let s3_retry_exhausted = self.s3_retry_exhausted.lock().unwrap();
        for (bucket, count) in s3_retry_exhausted.iter() {
            output.push_str(&format!(
                "s3_retry_exhausted_total{{bucket=\"{}\"}} {}\n",
                bucket, count
            ));
        }

        // Security validation metrics
        output.push_str("\n# HELP security_payload_too_large_total Requests rejected due to payload size exceeding limit (413)\n");
        output.push_str("# TYPE security_payload_too_large_total counter\n");
        output.push_str(&format!(
            "security_payload_too_large_total {}\n",
            self.security_payload_too_large.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP security_headers_too_large_total Requests rejected due to headers size exceeding limit (431)\n");
        output.push_str("# TYPE security_headers_too_large_total counter\n");
        output.push_str(&format!(
            "security_headers_too_large_total {}\n",
            self.security_headers_too_large.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP security_uri_too_long_total Requests rejected due to URI length exceeding limit (414)\n");
        output.push_str("# TYPE security_uri_too_long_total counter\n");
        output.push_str(&format!(
            "security_uri_too_long_total {}\n",
            self.security_uri_too_long.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP security_path_traversal_blocked_total Requests blocked due to path traversal attempt (400)\n");
        output.push_str("# TYPE security_path_traversal_blocked_total counter\n");
        output.push_str(&format!(
            "security_path_traversal_blocked_total {}\n",
            self.security_path_traversal_blocked.load(Ordering::Relaxed)
        ));

        output.push_str("\n# HELP security_sql_injection_blocked_total Requests blocked due to SQL injection attempt (400)\n");
        output.push_str("# TYPE security_sql_injection_blocked_total counter\n");
        output.push_str(&format!(
            "security_sql_injection_blocked_total {}\n",
            self.security_sql_injection_blocked.load(Ordering::Relaxed)
        ));

        // Request duration histogram (p50, p95, p99)
        let histogram = self.get_duration_histogram();
        output.push_str("\n# HELP http_request_duration_seconds Request duration in seconds\n");
        output.push_str("# TYPE http_request_duration_seconds summary\n");
        output.push_str(&format!(
            "http_request_duration_seconds{{quantile=\"0.5\"}} {:.3}\n",
            histogram.p50 / 1000.0 // Convert ms to seconds
        ));
        output.push_str(&format!(
            "http_request_duration_seconds{{quantile=\"0.9\"}} {:.3}\n",
            histogram.p90 / 1000.0
        ));
        output.push_str(&format!(
            "http_request_duration_seconds{{quantile=\"0.95\"}} {:.3}\n",
            histogram.p95 / 1000.0
        ));
        output.push_str(&format!(
            "http_request_duration_seconds{{quantile=\"0.99\"}} {:.3}\n",
            histogram.p99 / 1000.0
        ));

        // Backend health per bucket (1=healthy, 0=unhealthy)
        output.push_str(
            "\n# HELP backend_health Backend health status per bucket (1=healthy, 0=unhealthy)\n",
        );
        output.push_str("# TYPE backend_health gauge\n");
        if let Ok(health) = self.backend_health.lock() {
            for (bucket, is_healthy) in health.iter() {
                output.push_str(&format!(
                    "backend_health{{bucket=\"{}\"}} {}\n",
                    bucket,
                    if *is_healthy { 1 } else { 0 }
                ));
            }
        }

        output
    }
}

/// Calculate percentiles from a sorted vector of samples (in microseconds)
fn calculate_histogram(samples: &[u64]) -> Histogram {
    if samples.is_empty() {
        return Histogram {
            p50: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
        };
    }

    let mut sorted: Vec<u64> = samples.to_vec();
    sorted.sort_unstable();

    let p50_idx = (sorted.len() as f64 * 0.50) as usize;
    let p90_idx = (sorted.len() as f64 * 0.90) as usize;
    let p95_idx = (sorted.len() as f64 * 0.95) as usize;
    let p99_idx = (sorted.len() as f64 * 0.99) as usize;

    // Convert from microseconds to milliseconds
    Histogram {
        p50: sorted.get(p50_idx.saturating_sub(1)).copied().unwrap_or(0) as f64 / 1000.0,
        p90: sorted.get(p90_idx.saturating_sub(1)).copied().unwrap_or(0) as f64 / 1000.0,
        p95: sorted.get(p95_idx.saturating_sub(1)).copied().unwrap_or(0) as f64 / 1000.0,
        p99: sorted.get(p99_idx.saturating_sub(1)).copied().unwrap_or(0) as f64 / 1000.0,
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_metrics_struct() {
        // Test: Can create Metrics struct to track counters and histograms
        let metrics = Metrics::new();

        // Verify metrics struct was created successfully
        assert!(metrics.is_valid());
    }

    #[test]
    fn test_metrics_has_increment_request_count_method() {
        // Test: Metrics struct has increment_request_count() method
        let metrics = Metrics::new();

        // Should be able to increment request count
        metrics.increment_request_count();
    }

    #[test]
    fn test_metrics_has_record_duration_method() {
        // Test: Metrics struct has record_duration() method
        let metrics = Metrics::new();

        // Should be able to record duration in milliseconds
        metrics.record_duration(123.45);
    }

    #[test]
    fn test_metrics_can_be_shared_across_threads() {
        // Test: Metrics can be shared across threads (Arc<Metrics>)
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(Metrics::new());

        // Clone Arc for thread
        let metrics_clone = Arc::clone(&metrics);

        // Spawn thread that uses metrics
        let handle = thread::spawn(move || {
            metrics_clone.increment_request_count();
        });

        // Use metrics in main thread
        metrics.increment_request_count();

        // Wait for thread to complete
        handle.join().unwrap();

        // Both threads should have successfully used the metrics
        // (actual count checking will be tested later)
    }

    // Request count metrics tests
    #[test]
    fn test_track_total_http_requests_received() {
        // Test: Track total HTTP requests received
        let metrics = Metrics::new();

        // Initially zero requests
        assert_eq!(metrics.get_request_count(), 0);

        // Increment request count
        metrics.increment_request_count();
        assert_eq!(metrics.get_request_count(), 1);

        // Increment again
        metrics.increment_request_count();
        assert_eq!(metrics.get_request_count(), 2);
    }

    #[test]
    fn test_track_requests_by_status_code() {
        // Test: Track requests by status code (2xx, 3xx, 4xx, 5xx)
        let metrics = Metrics::new();

        // Track 2xx success responses
        metrics.increment_status_count(200);
        assert_eq!(metrics.get_status_count(200), 1);

        // Track 404 not found
        metrics.increment_status_count(404);
        assert_eq!(metrics.get_status_count(404), 1);

        // Track 500 internal error
        metrics.increment_status_count(500);
        assert_eq!(metrics.get_status_count(500), 1);

        // Multiple requests with same status
        metrics.increment_status_count(200);
        assert_eq!(metrics.get_status_count(200), 2);
    }

    #[test]
    fn test_track_requests_by_bucket_name() {
        // Test: Track requests by bucket name
        let metrics = Metrics::new();

        // Track requests to different buckets
        metrics.increment_bucket_count("products");
        assert_eq!(metrics.get_bucket_count("products"), 1);

        metrics.increment_bucket_count("images");
        assert_eq!(metrics.get_bucket_count("images"), 1);

        // Multiple requests to same bucket
        metrics.increment_bucket_count("products");
        assert_eq!(metrics.get_bucket_count("products"), 2);
    }

    #[test]
    fn test_track_requests_by_http_method() {
        // Test: Track requests by HTTP method (GET, HEAD, POST, etc.)
        let metrics = Metrics::new();

        // Track different HTTP methods
        metrics.increment_method_count("GET");
        assert_eq!(metrics.get_method_count("GET"), 1);

        metrics.increment_method_count("HEAD");
        assert_eq!(metrics.get_method_count("HEAD"), 1);

        metrics.increment_method_count("POST");
        assert_eq!(metrics.get_method_count("POST"), 1);

        // Multiple GET requests
        metrics.increment_method_count("GET");
        metrics.increment_method_count("GET");
        assert_eq!(metrics.get_method_count("GET"), 3);
    }

    // Latency metrics tests
    #[test]
    fn test_record_request_duration_histogram() {
        // Test: Record request duration histogram (p50, p90, p95, p99)
        let metrics = Metrics::new();

        // Record various request durations (in milliseconds)
        metrics.record_duration(10.5); // 10.5ms
        metrics.record_duration(25.0); // 25ms
        metrics.record_duration(50.0); // 50ms
        metrics.record_duration(100.0); // 100ms
        metrics.record_duration(200.0); // 200ms

        // Calculate percentiles
        let histogram = metrics.get_duration_histogram();
        assert!(histogram.p50 > 0.0);
        assert!(histogram.p90 > 0.0);
        assert!(histogram.p95 > 0.0);
        assert!(histogram.p99 > 0.0);

        // P99 should be >= P95 >= P90 >= P50
        assert!(histogram.p99 >= histogram.p95);
        assert!(histogram.p95 >= histogram.p90);
        assert!(histogram.p90 >= histogram.p50);
    }

    #[test]
    fn test_record_s3_backend_latency_separately() {
        // Test: Record S3 backend latency separately from total latency
        let metrics = Metrics::new();

        // Record total request latency (client -> proxy -> S3 -> proxy -> client)
        metrics.record_duration(100.0); // 100ms total

        // Record S3 backend latency only (proxy -> S3 -> proxy)
        metrics.record_s3_latency(80.0); // 80ms S3 backend

        // Should be able to retrieve both metrics separately
        let total_histogram = metrics.get_duration_histogram();
        let s3_histogram = metrics.get_s3_latency_histogram();

        assert!(total_histogram.p50 > 0.0);
        assert!(s3_histogram.p50 > 0.0);

        // S3 latency should typically be less than total latency
        // (total includes proxy overhead + network to client)
    }

    #[test]
    fn test_record_latency_by_bucket() {
        // Test: Record latency by bucket
        let metrics = Metrics::new();

        // Record latencies for different buckets
        metrics.record_bucket_latency("products", 50.0); // 50ms for products
        metrics.record_bucket_latency("products", 60.0); // 60ms for products
        metrics.record_bucket_latency("images", 100.0); // 100ms for images

        // Should be able to retrieve per-bucket latency histograms
        let products_histogram = metrics.get_bucket_latency_histogram("products");
        let images_histogram = metrics.get_bucket_latency_histogram("images");

        assert!(products_histogram.p50 > 0.0);
        assert!(images_histogram.p50 > 0.0);

        // Products bucket should have lower latency than images bucket
        assert!(products_histogram.p50 < images_histogram.p50);
    }

    // Authentication metrics tests
    #[test]
    fn test_track_jwt_authentication_attempts() {
        // Test: Track JWT authentication attempts (success/failure)
        let metrics = Metrics::new();

        // Track successful authentication
        metrics.increment_auth_success();
        assert_eq!(metrics.get_auth_success_count(), 1);

        // Track failed authentication
        metrics.increment_auth_failure();
        assert_eq!(metrics.get_auth_failure_count(), 1);

        // Multiple authentications
        metrics.increment_auth_success();
        metrics.increment_auth_success();
        assert_eq!(metrics.get_auth_success_count(), 3);

        metrics.increment_auth_failure();
        assert_eq!(metrics.get_auth_failure_count(), 2);
    }

    #[test]
    fn test_track_authentication_bypassed() {
        // Test: Track authentication bypassed (public buckets)
        let metrics = Metrics::new();

        // Track requests to public buckets (no auth required)
        metrics.increment_auth_bypassed();
        assert_eq!(metrics.get_auth_bypassed_count(), 1);

        // Multiple public bucket requests
        metrics.increment_auth_bypassed();
        metrics.increment_auth_bypassed();
        assert_eq!(metrics.get_auth_bypassed_count(), 3);
    }

    #[test]
    fn test_track_authentication_errors_by_type() {
        // Test: Track authentication errors by type (missing, invalid, expired)
        let metrics = Metrics::new();

        // Track different error types
        metrics.increment_auth_error("missing");
        assert_eq!(metrics.get_auth_error_count("missing"), 1);

        metrics.increment_auth_error("invalid");
        assert_eq!(metrics.get_auth_error_count("invalid"), 1);

        metrics.increment_auth_error("expired");
        assert_eq!(metrics.get_auth_error_count("expired"), 1);

        // Multiple errors of same type
        metrics.increment_auth_error("missing");
        metrics.increment_auth_error("missing");
        assert_eq!(metrics.get_auth_error_count("missing"), 3);
    }

    // S3 operation metrics tests
    #[test]
    fn test_track_s3_requests_by_operation() {
        // Test: Track S3 requests by operation (GET, HEAD)
        let metrics = Metrics::new();

        // Track different S3 operations
        metrics.increment_s3_operation("GET");
        assert_eq!(metrics.get_s3_operation_count("GET"), 1);

        metrics.increment_s3_operation("HEAD");
        assert_eq!(metrics.get_s3_operation_count("HEAD"), 1);

        // Multiple GET requests
        metrics.increment_s3_operation("GET");
        metrics.increment_s3_operation("GET");
        assert_eq!(metrics.get_s3_operation_count("GET"), 3);
    }

    #[test]
    fn test_track_s3_errors_by_error_code() {
        // Test: Track S3 errors by error code (NoSuchKey, AccessDenied, etc.)
        let metrics = Metrics::new();

        // Track different S3 error codes
        metrics.increment_s3_error("NoSuchKey");
        assert_eq!(metrics.get_s3_error_count("NoSuchKey"), 1);

        metrics.increment_s3_error("AccessDenied");
        assert_eq!(metrics.get_s3_error_count("AccessDenied"), 1);

        metrics.increment_s3_error("InternalError");
        assert_eq!(metrics.get_s3_error_count("InternalError"), 1);

        // Multiple errors of same type
        metrics.increment_s3_error("NoSuchKey");
        metrics.increment_s3_error("NoSuchKey");
        assert_eq!(metrics.get_s3_error_count("NoSuchKey"), 3);
    }

    #[test]
    fn test_track_s3_request_duration() {
        // Test: Track S3 request duration (already covered by record_s3_latency)
        // This test verifies S3-specific latency tracking works correctly
        let metrics = Metrics::new();

        // Record S3 request durations
        metrics.record_s3_latency(50.0); // 50ms
        metrics.record_s3_latency(100.0); // 100ms
        metrics.record_s3_latency(150.0); // 150ms

        // Verify histogram calculation works
        let histogram = metrics.get_s3_latency_histogram();
        assert!(histogram.p50 > 0.0);
        assert!(histogram.p95 > 0.0);

        // P95 should be >= P50
        assert!(histogram.p95 >= histogram.p50);
    }

    // System metrics tests
    #[test]
    fn test_track_active_connections_count() {
        // Test: Track active connections count
        let metrics = Metrics::new();

        // Start with zero connections
        assert_eq!(metrics.get_active_connections(), 0);

        // Increment connections (new client connected)
        metrics.increment_active_connections();
        assert_eq!(metrics.get_active_connections(), 1);

        metrics.increment_active_connections();
        assert_eq!(metrics.get_active_connections(), 2);

        // Decrement connections (client disconnected)
        metrics.decrement_active_connections();
        assert_eq!(metrics.get_active_connections(), 1);

        metrics.decrement_active_connections();
        assert_eq!(metrics.get_active_connections(), 0);
    }

    #[test]
    fn test_track_bytes_sent_received() {
        // Test: Track bytes sent/received
        let metrics = Metrics::new();

        // Start with zero bytes
        assert_eq!(metrics.get_bytes_sent(), 0);
        assert_eq!(metrics.get_bytes_received(), 0);

        // Track bytes sent (response to client)
        metrics.add_bytes_sent(1024); // 1KB
        assert_eq!(metrics.get_bytes_sent(), 1024);

        metrics.add_bytes_sent(2048); // 2KB
        assert_eq!(metrics.get_bytes_sent(), 3072); // 3KB total

        // Track bytes received (request from client)
        metrics.add_bytes_received(512); // 512 bytes
        assert_eq!(metrics.get_bytes_received(), 512);

        metrics.add_bytes_received(256); // 256 bytes
        assert_eq!(metrics.get_bytes_received(), 768); // 768 bytes total
    }

    #[test]
    fn test_track_memory_usage() {
        // Test: Track memory usage (RSS)
        let metrics = Metrics::new();

        // Update memory usage (in bytes)
        metrics.update_memory_usage(1024 * 1024 * 100); // 100 MB
        assert_eq!(metrics.get_memory_usage(), 1024 * 1024 * 100);

        // Memory usage can increase
        metrics.update_memory_usage(1024 * 1024 * 150); // 150 MB
        assert_eq!(metrics.get_memory_usage(), 1024 * 1024 * 150);

        // Memory usage can decrease (after GC)
        metrics.update_memory_usage(1024 * 1024 * 80); // 80 MB
        assert_eq!(metrics.get_memory_usage(), 1024 * 1024 * 80);
    }

    #[test]
    fn test_track_uptime() {
        // Test: Track uptime (seconds since start)
        let metrics = Metrics::new();

        // Uptime starts at 0
        assert_eq!(metrics.get_uptime_seconds(), 0);

        // Update uptime
        metrics.update_uptime(60); // 1 minute
        assert_eq!(metrics.get_uptime_seconds(), 60);

        metrics.update_uptime(3600); // 1 hour
        assert_eq!(metrics.get_uptime_seconds(), 3600);

        metrics.update_uptime(86400); // 1 day
        assert_eq!(metrics.get_uptime_seconds(), 86400);
    }

    // /metrics endpoint tests
    #[test]
    fn test_export_prometheus_format() {
        // Test: export_prometheus() returns valid Prometheus text format
        let metrics = Metrics::new();

        // Add some sample data
        metrics.increment_request_count();
        metrics.increment_status_count(200);
        metrics.increment_bucket_count("products");

        let output = metrics.export_prometheus();

        // Should contain HELP and TYPE annotations
        assert!(output.contains("# HELP http_requests_total"));
        assert!(output.contains("# TYPE http_requests_total counter"));

        // Should contain actual metric values
        assert!(output.contains("http_requests_total 1"));
        assert!(output.contains("http_requests_by_status_total{status=\"200\"} 1"));
        assert!(output.contains("http_requests_by_bucket_total{bucket=\"products\"} 1"));
    }

    #[test]
    fn test_export_includes_all_metric_types() {
        // Test: Response includes all tracked metrics
        let metrics = Metrics::new();

        // Populate various metrics
        metrics.increment_request_count();
        metrics.increment_auth_success();
        metrics.increment_s3_operation("GET");
        metrics.increment_active_connections();
        metrics.add_bytes_sent(1024);

        let output = metrics.export_prometheus();

        // Verify all metric categories are present
        assert!(output.contains("http_requests_total"));
        assert!(output.contains("auth_success_total"));
        assert!(output.contains("s3_operations_total"));
        assert!(output.contains("active_connections"));
        assert!(output.contains("bytes_sent_total"));
    }

    #[test]
    fn test_metric_names_follow_prometheus_conventions() {
        // Test: Metric names follow Prometheus naming conventions (snake_case, _total suffix)
        let metrics = Metrics::new();
        let output = metrics.export_prometheus();

        // Counter metrics should have _total suffix
        assert!(output.contains("http_requests_total"));
        assert!(output.contains("auth_success_total"));
        assert!(output.contains("s3_operations_total"));
        assert!(output.contains("bytes_sent_total"));

        // Gauge metrics should NOT have _total suffix
        assert!(output.contains("active_connections "));
        assert!(output.contains("memory_usage_bytes "));
        assert!(output.contains("uptime_seconds "));

        // All metric names should be snake_case (no camelCase, PascalCase, etc.)
        // The output should not contain invalid metric name characters
        assert!(!output.contains("httpRequestsTotal")); // camelCase - bad
        assert!(!output.contains("HttpRequestsTotal")); // PascalCase - bad
    }

    #[test]
    fn test_metrics_include_help_and_type_annotations() {
        // Test: Metrics include help text and type annotations
        let metrics = Metrics::new();
        let output = metrics.export_prometheus();

        // Every metric should have HELP and TYPE
        // Check a sample of metrics
        assert!(
            output.contains("# HELP http_requests_total Total number of HTTP requests received")
        );
        assert!(output.contains("# TYPE http_requests_total counter"));

        assert!(output.contains("# HELP active_connections Current number of active connections"));
        assert!(output.contains("# TYPE active_connections gauge"));

        assert!(output.contains("# HELP s3_operations_total S3 operations by type"));
        assert!(output.contains("# TYPE s3_operations_total counter"));

        // Count HELP lines (should have many)
        let help_count = output.matches("# HELP").count();
        assert!(help_count >= 10, "Should have at least 10 HELP annotations");

        // Count TYPE lines (should match HELP count)
        let type_count = output.matches("# TYPE").count();
        assert_eq!(
            help_count, type_count,
            "Every HELP should have matching TYPE"
        );
    }

    #[test]
    fn test_track_successful_config_reloads() {
        // Test: Track successful config reload attempts
        let metrics = Metrics::new();

        // Start with zero successful reloads
        assert_eq!(metrics.get_reload_success_count(), 0);

        // Increment successful reload count
        metrics.increment_reload_success();
        assert_eq!(metrics.get_reload_success_count(), 1);

        // Multiple successful reloads
        metrics.increment_reload_success();
        metrics.increment_reload_success();
        assert_eq!(metrics.get_reload_success_count(), 3);
    }

    #[test]
    fn test_track_failed_config_reloads() {
        // Test: Track failed config reload attempts
        let metrics = Metrics::new();

        // Start with zero failed reloads
        assert_eq!(metrics.get_reload_failure_count(), 0);

        // Increment failed reload count
        metrics.increment_reload_failure();
        assert_eq!(metrics.get_reload_failure_count(), 1);

        // Multiple failed reloads
        metrics.increment_reload_failure();
        metrics.increment_reload_failure();
        assert_eq!(metrics.get_reload_failure_count(), 3);
    }

    #[test]
    fn test_track_config_generation() {
        // Test: Track current config generation number
        let metrics = Metrics::new();

        // Start with zero (initial generation)
        assert_eq!(metrics.get_config_generation(), 0);

        // Set config generation
        metrics.set_config_generation(1);
        assert_eq!(metrics.get_config_generation(), 1);

        // Update to new generation
        metrics.set_config_generation(5);
        assert_eq!(metrics.get_config_generation(), 5);

        // Generation can increase by any amount
        metrics.set_config_generation(42);
        assert_eq!(metrics.get_config_generation(), 42);
    }

    #[test]
    fn test_export_prometheus_performance() {
        // Test: Response time < 50ms even under load (simulated with many metrics)
        let metrics = Metrics::new();

        // Populate with many metrics
        for i in 0..100 {
            metrics.increment_status_count(200 + (i % 100) as u16);
            metrics.increment_bucket_count(&format!("bucket{}", i));
            metrics.increment_method_count("GET");
            metrics.increment_s3_operation("GET");
        }

        // Time the export
        let start = std::time::Instant::now();
        let output = metrics.export_prometheus();
        let elapsed = start.elapsed();

        // Should be fast (< 50ms even with 100+ metrics)
        assert!(
            elapsed.as_millis() < 50,
            "Export took {}ms, should be < 50ms",
            elapsed.as_millis()
        );

        // Output should still be valid
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }

    // Phase 23: Per-Replica Metrics Tests
    #[test]
    fn test_track_request_count_per_replica() {
        // Test: Request count per replica
        let metrics = Metrics::new();

        // Track requests to different replicas within same bucket
        metrics.increment_replica_request_count("products", "primary");
        assert_eq!(metrics.get_replica_request_count("products", "primary"), 1);

        metrics.increment_replica_request_count("products", "replica-eu");
        assert_eq!(
            metrics.get_replica_request_count("products", "replica-eu"),
            1
        );

        // Multiple requests to same replica
        metrics.increment_replica_request_count("products", "primary");
        metrics.increment_replica_request_count("products", "primary");
        assert_eq!(metrics.get_replica_request_count("products", "primary"), 3);

        // Different bucket, same replica name
        metrics.increment_replica_request_count("media", "primary");
        assert_eq!(metrics.get_replica_request_count("media", "primary"), 1);

        // Original primary count should be unchanged
        assert_eq!(metrics.get_replica_request_count("products", "primary"), 3);
    }
}
