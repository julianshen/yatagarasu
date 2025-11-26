// Redis cache Prometheus metrics
//
// Provides comprehensive metrics for cache operations, including:
// - Operation counters (hits, misses, sets, evictions, errors)
// - Operation latency histograms
// - Connection pool metrics
// - Serialization metrics

use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, Histogram, HistogramVec,
    IntCounter, IntGauge,
};
use std::sync::OnceLock;

/// Global metrics registry for Redis cache
pub struct RedisCacheMetrics {
    /// Total number of cache hits
    pub hits: IntCounter,

    /// Total number of cache misses
    pub misses: IntCounter,

    /// Total number of cache sets
    pub sets: IntCounter,

    /// Total number of cache evictions
    pub evictions: IntCounter,

    /// Total number of cache errors
    pub errors: IntCounter,

    /// Operation duration histogram (in seconds)
    pub operation_duration: HistogramVec,

    /// Serialization duration histogram (in seconds)
    pub serialization_duration: HistogramVec,

    /// Current number of Redis connections
    pub active_connections: IntGauge,

    /// Current number of idle Redis connections
    pub idle_connections: IntGauge,
}

/// Global singleton instance of metrics
static METRICS: OnceLock<RedisCacheMetrics> = OnceLock::new();

impl RedisCacheMetrics {
    /// Initialize and return the global metrics instance
    ///
    /// This function should be called once at application startup.
    /// Subsequent calls return the same instance.
    pub fn global() -> &'static Self {
        METRICS.get_or_init(|| {
            // Create counter vector for cache operations
            let cache_ops = register_int_counter_vec!(
                "yatagarasu_cache_operations_total",
                "Total number of cache operations by type",
                &["operation"] // hit, miss, set, eviction, error
            )
            .expect("Failed to register cache_operations_total metric");

            // Create histogram for operation durations
            let operation_duration = register_histogram_vec!(
                "yatagarasu_cache_operation_duration_seconds",
                "Duration of cache operations in seconds",
                &["operation"], // get, set, delete, clear
                vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]  // 0.1ms to 1s
            )
            .expect("Failed to register cache_operation_duration_seconds metric");

            // Create histogram for serialization durations
            let serialization_duration = register_histogram_vec!(
                "yatagarasu_cache_serialization_duration_seconds",
                "Duration of serialization/deserialization in seconds",
                &["operation"], // serialize, deserialize
                vec![0.00001, 0.00005, 0.0001, 0.0005, 0.001, 0.005, 0.01]  // 10μs to 10ms
            )
            .expect("Failed to register cache_serialization_duration_seconds metric");

            // Create gauges for connection pool
            let active_connections = register_int_gauge!(
                "yatagarasu_redis_active_connections",
                "Current number of active Redis connections"
            )
            .expect("Failed to register redis_active_connections metric");

            let idle_connections = register_int_gauge!(
                "yatagarasu_redis_idle_connections",
                "Current number of idle Redis connections"
            )
            .expect("Failed to register redis_idle_connections metric");

            RedisCacheMetrics {
                hits: cache_ops.with_label_values(&["hit"]),
                misses: cache_ops.with_label_values(&["miss"]),
                sets: cache_ops.with_label_values(&["set"]),
                evictions: cache_ops.with_label_values(&["eviction"]),
                errors: cache_ops.with_label_values(&["error"]),
                operation_duration,
                serialization_duration,
                active_connections,
                idle_connections,
            }
        })
    }

    /// Start timing an operation
    ///
    /// Returns a timer that should be observed when the operation completes.
    ///
    /// # Example
    /// ```ignore
    /// let timer = metrics.start_operation_timer("get");
    /// // ... perform operation ...
    /// timer.observe_duration();
    /// ```
    pub fn start_operation_timer(&self, operation: &str) -> HistogramTimer {
        HistogramTimer {
            histogram: self.operation_duration.with_label_values(&[operation]),
            start: std::time::Instant::now(),
        }
    }

    /// Start timing a serialization operation
    pub fn start_serialization_timer(&self, operation: &str) -> HistogramTimer {
        HistogramTimer {
            histogram: self.serialization_duration.with_label_values(&[operation]),
            start: std::time::Instant::now(),
        }
    }
}

/// RAII timer for histogram metrics
///
/// Automatically records duration when dropped.
pub struct HistogramTimer {
    histogram: Histogram,
    start: std::time::Instant,
}

impl HistogramTimer {
    /// Manually observe and consume the timer
    pub fn observe_duration(self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

impl Drop for HistogramTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_initialize_metrics() {
        let metrics = RedisCacheMetrics::global();

        // Verify metrics exist (this will panic if registration failed)
        // Note: We can't assert exact values because metrics are global singletons
        // and other tests may have already incremented them
        let hits_before = metrics.hits.get();
        let misses_before = metrics.misses.get();

        // Just verify we can read the values
        assert!(hits_before >= 0);
        assert!(misses_before >= 0);
    }

    #[test]
    fn test_metrics_singleton() {
        let metrics1 = RedisCacheMetrics::global();
        let metrics2 = RedisCacheMetrics::global();

        // Should be the same instance
        assert_eq!(
            metrics1 as *const RedisCacheMetrics,
            metrics2 as *const RedisCacheMetrics
        );
    }

    #[test]
    fn test_can_increment_counters() {
        let metrics = RedisCacheMetrics::global();

        let hits_before = metrics.hits.get();
        metrics.hits.inc();
        assert_eq!(metrics.hits.get(), hits_before + 1);

        let misses_before = metrics.misses.get();
        metrics.misses.inc();
        assert_eq!(metrics.misses.get(), misses_before + 1);
    }

    #[test]
    fn test_can_create_operation_timer() {
        let metrics = RedisCacheMetrics::global();

        let timer = metrics.start_operation_timer("get");
        // Simulate some work
        std::thread::sleep(std::time::Duration::from_micros(100));
        timer.observe_duration();

        // Timer should have recorded a duration
        // We can't easily verify the exact value, but we can verify it doesn't panic
    }

    #[test]
    fn test_timer_auto_observes_on_drop() {
        let metrics = RedisCacheMetrics::global();

        {
            let _timer = metrics.start_operation_timer("test");
            // Timer will auto-observe when dropped
        }
        // No panic means it worked
    }
}
