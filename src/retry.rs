//! Retry Logic with Exponential Backoff
//!
//! Handles transient failures from S3 backends by automatically retrying failed requests
//! with exponential backoff delays between attempts.
//!
//! ## Retriable vs Non-Retriable Errors
//!
//! **Retriable Errors** (will be retried):
//! - 500 Internal Server Error - Temporary S3 backend issue
//! - 502 Bad Gateway - Upstream connection issue
//! - 503 Service Unavailable - S3 temporarily overloaded
//! - 504 Gateway Timeout - S3 request timed out
//!
//! **Non-Retriable Errors** (fail immediately):
//! - 400 Bad Request - Invalid request format
//! - 403 Forbidden - Permission denied
//! - 404 Not Found - Object doesn't exist
//! - 416 Range Not Satisfiable - Invalid range
//! - 2xx Success - Request succeeded
//!
//! ## Exponential Backoff
//!
//! Delays between retries grow exponentially to avoid overwhelming the backend:
//! - Attempt 1: No delay (immediate)
//! - Attempt 2: 100ms delay
//! - Attempt 3: 200ms delay (2x)
//! - Attempt 4: 400ms delay (2x)
//! - Capped at max_backoff_ms to prevent excessive delays
//!
//! ## Configuration Example
//!
//! ```yaml
//! buckets:
//!   - name: products
//!     s3:
//!       retry:
//!         max_attempts: 3
//!         initial_backoff_ms: 100
//!         max_backoff_ms: 1000
//! ```

use std::time::Duration;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (including initial attempt)
    pub max_attempts: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds (cap for exponential growth)
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 1000,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_attempts: u32, initial_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_backoff_ms,
            max_backoff_ms,
        }
    }

    /// Check if an HTTP status code should be retried
    pub fn is_retriable_status(&self, status_code: u16) -> bool {
        matches!(status_code, 500 | 502 | 503 | 504)
    }

    /// Calculate backoff delay for a given attempt number (0-indexed)
    ///
    /// # Arguments
    /// * `attempt` - The attempt number (0 = first attempt, 1 = first retry, etc.)
    ///
    /// # Returns
    /// Duration to wait before the next attempt (0 for first attempt)
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            // First attempt: no delay
            return Duration::from_millis(0);
        }

        // Exponential backoff: initial_backoff * 2^(attempt-1)
        let backoff_ms = self
            .initial_backoff_ms
            .saturating_mul(2u64.saturating_pow(attempt - 1))
            .min(self.max_backoff_ms);

        Duration::from_millis(backoff_ms)
    }

    /// Check if we should retry given the current attempt number and status code
    ///
    /// # Arguments
    /// * `attempt` - Current attempt number (0-indexed)
    /// * `status_code` - HTTP status code from the response
    ///
    /// # Returns
    /// true if we should retry, false otherwise
    pub fn should_retry(&self, attempt: u32, status_code: u16) -> bool {
        // Don't retry if we've exhausted attempts
        if attempt >= self.max_attempts - 1 {
            return false;
        }

        // Only retry if the status code is retriable
        self.is_retriable_status(status_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_backoff_ms, 100);
        assert_eq!(policy.max_backoff_ms, 1000);
    }

    #[test]
    fn test_retriable_status_codes() {
        let policy = RetryPolicy::default();

        // Retriable errors
        assert!(policy.is_retriable_status(500), "500 should be retriable");
        assert!(policy.is_retriable_status(502), "502 should be retriable");
        assert!(policy.is_retriable_status(503), "503 should be retriable");
        assert!(policy.is_retriable_status(504), "504 should be retriable");

        // Non-retriable errors
        assert!(
            !policy.is_retriable_status(400),
            "400 should not be retriable"
        );
        assert!(
            !policy.is_retriable_status(403),
            "403 should not be retriable"
        );
        assert!(
            !policy.is_retriable_status(404),
            "404 should not be retriable"
        );
        assert!(
            !policy.is_retriable_status(416),
            "416 should not be retriable"
        );

        // Success codes
        assert!(
            !policy.is_retriable_status(200),
            "200 should not be retriable"
        );
        assert!(
            !policy.is_retriable_status(204),
            "204 should not be retriable"
        );
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let policy = RetryPolicy::new(5, 100, 1000);

        // Attempt 0 (first attempt): no delay
        assert_eq!(policy.backoff_duration(0), Duration::from_millis(0));

        // Attempt 1 (first retry): 100ms
        assert_eq!(policy.backoff_duration(1), Duration::from_millis(100));

        // Attempt 2 (second retry): 200ms (100 * 2^1)
        assert_eq!(policy.backoff_duration(2), Duration::from_millis(200));

        // Attempt 3 (third retry): 400ms (100 * 2^2)
        assert_eq!(policy.backoff_duration(3), Duration::from_millis(400));

        // Attempt 4 (fourth retry): 800ms (100 * 2^3)
        assert_eq!(policy.backoff_duration(4), Duration::from_millis(800));

        // Attempt 5 would be 1600ms, but capped at 1000ms
        assert_eq!(policy.backoff_duration(5), Duration::from_millis(1000));
    }

    #[test]
    fn test_should_retry_logic() {
        let policy = RetryPolicy::new(3, 100, 1000);

        // Attempt 0, retriable error: should retry
        assert!(
            policy.should_retry(0, 500),
            "Should retry attempt 0 with 500"
        );

        // Attempt 1, retriable error: should retry
        assert!(
            policy.should_retry(1, 503),
            "Should retry attempt 1 with 503"
        );

        // Attempt 2 (last attempt), retriable error: should NOT retry (exhausted)
        assert!(
            !policy.should_retry(2, 500),
            "Should not retry attempt 2 (max attempts reached)"
        );

        // Attempt 0, non-retriable error: should NOT retry
        assert!(!policy.should_retry(0, 404), "Should not retry 404 error");

        // Attempt 0, success: should NOT retry
        assert!(!policy.should_retry(0, 200), "Should not retry 200 success");
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let policy = RetryPolicy::new(10, 50, 500);

        // Large attempt number should be capped at max_backoff_ms
        assert_eq!(policy.backoff_duration(10), Duration::from_millis(500));
        assert_eq!(policy.backoff_duration(100), Duration::from_millis(500));
    }

    #[test]
    fn test_no_retry_after_max_attempts() {
        let policy = RetryPolicy::new(2, 100, 1000); // Only 2 attempts total

        // Attempt 0: can retry
        assert!(policy.should_retry(0, 500));

        // Attempt 1 (last): cannot retry
        assert!(!policy.should_retry(1, 500));
    }

    #[test]
    fn test_backoff_with_zero_initial() {
        let policy = RetryPolicy::new(3, 0, 1000);

        // All attempts should have 0 backoff if initial is 0
        assert_eq!(policy.backoff_duration(0), Duration::from_millis(0));
        assert_eq!(policy.backoff_duration(1), Duration::from_millis(0));
        assert_eq!(policy.backoff_duration(2), Duration::from_millis(0));
    }

    #[test]
    fn test_saturating_mul_prevents_overflow() {
        let policy = RetryPolicy::new(100, u64::MAX, u64::MAX);

        // Should not panic even with max values
        let duration = policy.backoff_duration(50);
        assert_eq!(duration, Duration::from_millis(u64::MAX));
    }
}
