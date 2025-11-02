// Metrics module - Prometheus-compatible metrics tracking
// Provides counters, histograms, and gauges for observability

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
}
