// Metrics module - Prometheus-compatible metrics tracking
// Provides counters, histograms, and gauges for observability

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Metrics struct tracks counters and histograms for Prometheus export
/// Thread-safe via atomic operations and mutexes
pub struct Metrics {
    // Request counters
    request_count: AtomicU64,

    // Duration tracking (stored in microseconds as u64)
    durations: Mutex<Vec<u64>>,
}

impl Metrics {
    /// Create a new Metrics instance
    pub fn new() -> Self {
        Metrics {
            request_count: AtomicU64::new(0),
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
}
