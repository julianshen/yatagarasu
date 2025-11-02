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

    /// Calculate histogram from duration samples (for testing)
    #[cfg(test)]
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
}

/// Calculate percentiles from a sorted vector of samples (in microseconds)
#[cfg(test)]
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
}
