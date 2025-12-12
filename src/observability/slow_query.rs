//! Slow Query Logging Module
//!
//! Detects and logs requests that exceed a configurable time threshold,
//! helping identify performance bottlenecks and degraded upstream responses.
//!
//! # Features
//!
//! - **Configurable threshold**: Set minimum duration (in milliseconds) to trigger logging
//! - **Timing breakdown**: Optionally include detailed timing for each request phase
//! - **Configurable log level**: Output slow queries at warn, info, debug, or error level
//! - **Correlation ID tracking**: Links slow query logs to full request traces
//!
//! # Example
//!
//! ```yaml
//! observability:
//!   slow_query:
//!     enabled: true
//!     threshold_ms: 1000  # Log requests taking > 1 second
//!     log_level: warn
//!     include_breakdown: true
//! ```

use crate::observability::config::SlowQueryConfig;
use crate::observability::tracing::RequestTiming;
use std::time::Instant;

/// Slow query logger
#[derive(Debug, Clone)]
pub struct SlowQueryLogger {
    config: SlowQueryConfig,
}

impl SlowQueryLogger {
    /// Create a new slow query logger
    pub fn new(config: SlowQueryConfig) -> Self {
        Self { config }
    }

    /// Check if slow query logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the threshold in milliseconds
    pub fn threshold_ms(&self) -> u64 {
        self.config.threshold_ms
    }

    /// Log a slow query if it exceeds the threshold
    pub fn log_if_slow(
        &self,
        timing: &RequestTiming,
        method: &str,
        path: &str,
        correlation_id: &str,
    ) {
        if !self.config.enabled {
            return;
        }

        if timing.total_ms < self.config.threshold_ms {
            return;
        }

        self.log_slow_query(timing, method, path, correlation_id);
    }

    fn log_slow_query(
        &self,
        timing: &RequestTiming,
        method: &str,
        path: &str,
        correlation_id: &str,
    ) {
        let breakdown = if self.config.include_breakdown {
            self.format_breakdown(timing)
        } else {
            String::new()
        };

        match self.config.log_level.to_lowercase().as_str() {
            "error" => {
                tracing::error!(
                    correlation_id = %correlation_id,
                    method = %method,
                    path = %path,
                    total_ms = timing.total_ms,
                    threshold_ms = self.config.threshold_ms,
                    breakdown = %breakdown,
                    "Slow query detected"
                );
            }
            "info" => {
                tracing::info!(
                    correlation_id = %correlation_id,
                    method = %method,
                    path = %path,
                    total_ms = timing.total_ms,
                    threshold_ms = self.config.threshold_ms,
                    breakdown = %breakdown,
                    "Slow query detected"
                );
            }
            "debug" => {
                tracing::debug!(
                    correlation_id = %correlation_id,
                    method = %method,
                    path = %path,
                    total_ms = timing.total_ms,
                    threshold_ms = self.config.threshold_ms,
                    breakdown = %breakdown,
                    "Slow query detected"
                );
            }
            _ => {
                // Default to warn
                tracing::warn!(
                    correlation_id = %correlation_id,
                    method = %method,
                    path = %path,
                    total_ms = timing.total_ms,
                    threshold_ms = self.config.threshold_ms,
                    breakdown = %breakdown,
                    "Slow query detected"
                );
            }
        }
    }

    fn format_breakdown(&self, timing: &RequestTiming) -> String {
        let mut parts = Vec::new();

        if let Some(auth_ms) = timing.auth_ms {
            parts.push(format!("auth={}ms", auth_ms));
        }
        if let Some(cache_ms) = timing.cache_ms {
            parts.push(format!("cache={}ms", cache_ms));
        }
        if let Some(s3_ms) = timing.s3_ms {
            parts.push(format!("s3={}ms", s3_ms));
        }
        if let Some(other_ms) = timing.other_ms {
            parts.push(format!("other={}ms", other_ms));
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join(", ")
        }
    }
}

impl Default for SlowQueryLogger {
    fn default() -> Self {
        Self::new(SlowQueryConfig::default())
    }
}

/// Timer helper for measuring request phases
#[derive(Debug)]
pub struct PhaseTimer {
    start: Instant,
    auth_duration: Option<u64>,
    cache_duration: Option<u64>,
    s3_duration: Option<u64>,
    phase_start: Option<Instant>,
}

impl PhaseTimer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            auth_duration: None,
            cache_duration: None,
            s3_duration: None,
            phase_start: None,
        }
    }

    /// Start timing the auth phase
    pub fn start_auth(&mut self) {
        self.phase_start = Some(Instant::now());
    }

    /// End timing the auth phase
    pub fn end_auth(&mut self) {
        if let Some(start) = self.phase_start.take() {
            self.auth_duration = Some(start.elapsed().as_millis() as u64);
        }
    }

    /// Start timing the cache phase
    pub fn start_cache(&mut self) {
        self.phase_start = Some(Instant::now());
    }

    /// End timing the cache phase
    pub fn end_cache(&mut self) {
        if let Some(start) = self.phase_start.take() {
            self.cache_duration = Some(start.elapsed().as_millis() as u64);
        }
    }

    /// Start timing the S3 phase
    pub fn start_s3(&mut self) {
        self.phase_start = Some(Instant::now());
    }

    /// End timing the S3 phase
    pub fn end_s3(&mut self) {
        if let Some(start) = self.phase_start.take() {
            self.s3_duration = Some(start.elapsed().as_millis() as u64);
        }
    }

    /// Get the total elapsed time in milliseconds
    pub fn total_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Convert to RequestTiming
    pub fn to_timing(&self) -> RequestTiming {
        let total = self.total_ms();
        let known = self.auth_duration.unwrap_or(0)
            + self.cache_duration.unwrap_or(0)
            + self.s3_duration.unwrap_or(0);
        let other = total.saturating_sub(known);

        RequestTiming {
            total_ms: total,
            auth_ms: self.auth_duration,
            cache_ms: self.cache_duration,
            s3_ms: self.s3_duration,
            other_ms: if other > 0 { Some(other) } else { None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_slow_query_logger_disabled_by_default() {
        let logger = SlowQueryLogger::default();
        assert!(!logger.is_enabled());
    }

    #[test]
    fn test_slow_query_logger_threshold() {
        let config = SlowQueryConfig {
            enabled: true,
            threshold_ms: 500,
            ..Default::default()
        };
        let logger = SlowQueryLogger::new(config);
        assert!(logger.is_enabled());
        assert_eq!(logger.threshold_ms(), 500);
    }

    #[test]
    fn test_slow_query_logger_does_not_log_fast_queries() {
        let config = SlowQueryConfig {
            enabled: true,
            threshold_ms: 1000,
            ..Default::default()
        };
        let logger = SlowQueryLogger::new(config);

        let timing = RequestTiming {
            total_ms: 500, // Below threshold
            ..Default::default()
        };

        // Should not panic or log
        logger.log_if_slow(&timing, "GET", "/test", "req-123");
    }

    #[test]
    fn test_slow_query_logger_logs_slow_queries() {
        let config = SlowQueryConfig {
            enabled: true,
            threshold_ms: 100,
            include_breakdown: true,
            log_level: "warn".to_string(),
        };
        let logger = SlowQueryLogger::new(config);

        let timing = RequestTiming {
            total_ms: 200, // Above threshold
            auth_ms: Some(10),
            cache_ms: Some(20),
            s3_ms: Some(150),
            other_ms: Some(20),
        };

        // Should not panic (actual logging depends on subscriber)
        logger.log_if_slow(&timing, "GET", "/slow/path", "req-456");
    }

    #[test]
    fn test_slow_query_logger_disabled_does_not_log() {
        let config = SlowQueryConfig {
            enabled: false,
            threshold_ms: 100,
            ..Default::default()
        };
        let logger = SlowQueryLogger::new(config);

        let timing = RequestTiming {
            total_ms: 200, // Above threshold but disabled
            ..Default::default()
        };

        // Should not log when disabled
        logger.log_if_slow(&timing, "GET", "/test", "req-123");
    }

    #[test]
    fn test_format_breakdown() {
        let config = SlowQueryConfig {
            enabled: true,
            threshold_ms: 100,
            include_breakdown: true,
            ..Default::default()
        };
        let logger = SlowQueryLogger::new(config);

        let timing = RequestTiming {
            total_ms: 200,
            auth_ms: Some(10),
            cache_ms: Some(20),
            s3_ms: Some(150),
            other_ms: None,
        };

        let breakdown = logger.format_breakdown(&timing);
        assert!(breakdown.contains("auth=10ms"));
        assert!(breakdown.contains("cache=20ms"));
        assert!(breakdown.contains("s3=150ms"));
    }

    #[test]
    fn test_format_breakdown_empty() {
        let config = SlowQueryConfig::default();
        let logger = SlowQueryLogger::new(config);

        let timing = RequestTiming {
            total_ms: 200,
            auth_ms: None,
            cache_ms: None,
            s3_ms: None,
            other_ms: None,
        };

        let breakdown = logger.format_breakdown(&timing);
        assert!(breakdown.is_empty());
    }

    #[test]
    fn test_phase_timer_start() {
        let timer = PhaseTimer::start();
        // Verify timer can be read (total_ms returns u64, always valid)
        let _ = timer.total_ms();
    }

    #[test]
    fn test_phase_timer_auth_phase() {
        let mut timer = PhaseTimer::start();
        timer.start_auth();
        thread::sleep(Duration::from_millis(10));
        timer.end_auth();

        let timing = timer.to_timing();
        assert!(timing.auth_ms.is_some());
        assert!(timing.auth_ms.unwrap() >= 10);
    }

    #[test]
    fn test_phase_timer_cache_phase() {
        let mut timer = PhaseTimer::start();
        timer.start_cache();
        thread::sleep(Duration::from_millis(10));
        timer.end_cache();

        let timing = timer.to_timing();
        assert!(timing.cache_ms.is_some());
        assert!(timing.cache_ms.unwrap() >= 10);
    }

    #[test]
    fn test_phase_timer_s3_phase() {
        let mut timer = PhaseTimer::start();
        timer.start_s3();
        thread::sleep(Duration::from_millis(10));
        timer.end_s3();

        let timing = timer.to_timing();
        assert!(timing.s3_ms.is_some());
        assert!(timing.s3_ms.unwrap() >= 10);
    }

    #[test]
    fn test_phase_timer_all_phases() {
        let mut timer = PhaseTimer::start();

        timer.start_auth();
        thread::sleep(Duration::from_millis(5));
        timer.end_auth();

        timer.start_cache();
        thread::sleep(Duration::from_millis(5));
        timer.end_cache();

        timer.start_s3();
        thread::sleep(Duration::from_millis(5));
        timer.end_s3();

        let timing = timer.to_timing();
        assert!(timing.auth_ms.is_some());
        assert!(timing.cache_ms.is_some());
        assert!(timing.s3_ms.is_some());
        assert!(timing.total_ms >= 15);
    }

    #[test]
    fn test_phase_timer_calculates_other() {
        let mut timer = PhaseTimer::start();

        // Simulate some unmeasured overhead
        thread::sleep(Duration::from_millis(20));

        timer.start_s3();
        thread::sleep(Duration::from_millis(5));
        timer.end_s3();

        let timing = timer.to_timing();
        // Total should be more than just S3 time
        assert!(timing.total_ms > timing.s3_ms.unwrap_or(0));
        // Other should capture the difference
        assert!(timing.other_ms.is_some());
    }

    #[test]
    fn test_slow_query_log_levels() {
        // Test different log levels don't panic
        for level in &["error", "warn", "info", "debug", "unknown"] {
            let config = SlowQueryConfig {
                enabled: true,
                threshold_ms: 100,
                log_level: level.to_string(),
                include_breakdown: false,
            };
            let logger = SlowQueryLogger::new(config);

            let timing = RequestTiming {
                total_ms: 200,
                ..Default::default()
            };

            logger.log_if_slow(&timing, "GET", "/test", "req-123");
        }
    }

    #[test]
    fn test_slow_query_includes_correlation_id() {
        let config = SlowQueryConfig {
            enabled: true,
            threshold_ms: 100,
            ..Default::default()
        };
        let logger = SlowQueryLogger::new(config);

        let timing = RequestTiming {
            total_ms: 200,
            ..Default::default()
        };

        // The correlation_id is included in the log call
        // This test verifies the API accepts it
        logger.log_if_slow(&timing, "GET", "/test", "correlation-xyz-123");
    }
}
