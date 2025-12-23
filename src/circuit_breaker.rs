//! Circuit Breaker Pattern Implementation
//!
//! Prevents cascading failures by failing fast when S3 backends become unhealthy.
//!
//! State Machine:
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Too many failures, reject requests immediately (503)
//! - **Half-Open**: After timeout, allow test requests
//!   - Success → Closed
//!   - Failure → Open
//!
//! Configuration:
//! - `failure_threshold`: Number of consecutive failures to open circuit
//! - `success_threshold`: Number of successes in half-open to close circuit
//! - `timeout_seconds`: How long to wait before trying again (open → half-open)
//! - `half_open_max_requests`: Max concurrent test requests in half-open state

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Get current time as milliseconds since UNIX epoch (lock-free timestamp)
#[inline]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed = 0,
    /// Too many failures - reject requests immediately
    Open = 1,
    /// Testing if backend recovered - allow limited requests
    HalfOpen = 2,
}

impl From<u8> for CircuitState {
    fn from(value: u8) -> Self {
        match value {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed, // Default to closed for invalid values
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to open circuit
    pub failure_threshold: u32,
    /// Number of successes in half-open to close circuit
    pub success_threshold: u32,
    /// How long to wait before trying again (open → half-open)
    pub timeout_duration: Duration,
    /// Max concurrent test requests in half-open state
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

/// Circuit breaker for preventing cascading failures
///
/// Uses lock-free atomics for all operations, including timestamp tracking.
/// State transitions use Acquire/Release ordering for proper synchronization.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Current circuit state (uses Acquire on load, Release on store)
    state: Arc<AtomicU8>,
    /// Consecutive failure count
    failure_count: Arc<AtomicU64>,
    /// Success count in half-open state
    success_count: Arc<AtomicU64>,
    /// Number of requests currently being tested in half-open state
    half_open_requests: Arc<AtomicU64>,
    /// Last state transition time as milliseconds since UNIX epoch (lock-free)
    last_transition_ms: Arc<AtomicU64>,
    /// Configuration
    config: Arc<CircuitBreakerConfig>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(AtomicU8::new(CircuitState::Closed as u8)),
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            half_open_requests: Arc::new(AtomicU64::new(0)),
            last_transition_ms: Arc::new(AtomicU64::new(now_ms())),
            config: Arc::new(config),
        }
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        self.state.load(Ordering::Acquire).into()
    }

    /// Check if a request should be allowed through the circuit
    pub fn should_allow_request(&self) -> bool {
        let current_state = self.state();

        match current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed to transition to half-open (lock-free)
                let last_ms = self.last_transition_ms.load(Ordering::Acquire);
                let elapsed_ms = now_ms().saturating_sub(last_ms);
                let timeout_ms = self.config.timeout_duration.as_millis() as u64;

                if elapsed_ms >= timeout_ms {
                    tracing::info!("Circuit breaker timeout elapsed, transitioning to half-open");
                    self.transition_to_half_open();
                    true // Allow the first test request
                } else {
                    false // Still in open state, reject request
                }
            }
            CircuitState::HalfOpen => {
                // Only allow limited concurrent requests in half-open
                let current = self.half_open_requests.load(Ordering::Relaxed);
                current < self.config.half_open_max_requests as u64
            }
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let current_state = self.state();

        match current_state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                // Decrement half-open request count
                self.half_open_requests.fetch_sub(1, Ordering::Relaxed);

                // Increment success count
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

                tracing::debug!(
                    successes = successes,
                    threshold = self.config.success_threshold,
                    "Circuit breaker success in half-open state"
                );

                // If we've had enough successes, close the circuit
                if successes >= self.config.success_threshold as u64 {
                    tracing::info!("Circuit breaker closing after successful recovery");
                    self.transition_to_closed();
                }
            }
            CircuitState::Open => {
                // Success in open state shouldn't happen (requests are rejected)
                // But if it does, we can ignore it
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let current_state = self.state();

        match current_state {
            CircuitState::Closed => {
                // Increment failure count
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

                tracing::warn!(
                    failures = failures,
                    threshold = self.config.failure_threshold,
                    "Circuit breaker failure in closed state"
                );

                // If we've exceeded the failure threshold, open the circuit
                if failures >= self.config.failure_threshold as u64 {
                    tracing::error!("Circuit breaker opening due to consecutive failures");
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Decrement half-open request count
                self.half_open_requests.fetch_sub(1, Ordering::Relaxed);

                // Any failure in half-open immediately reopens the circuit
                tracing::warn!("Circuit breaker reopening due to failure in half-open state");
                self.transition_to_open();
            }
            CircuitState::Open => {
                // Failure in open state is expected (no requests should get through)
            }
        }
    }

    /// Increment half-open request count (called when request starts)
    pub fn start_half_open_request(&self) {
        if self.state() == CircuitState::HalfOpen {
            self.half_open_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get current failure count
    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Get current success count (in half-open state)
    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Transition to closed state
    ///
    /// Uses Release ordering to ensure counter resets are visible before state change.
    fn transition_to_closed(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
        self.last_transition_ms.store(now_ms(), Ordering::Relaxed);
        // Release ensures all above writes are visible before state change
        self.state
            .store(CircuitState::Closed as u8, Ordering::Release);
    }

    /// Transition to open state
    ///
    /// Uses Release ordering to ensure counter resets are visible before state change.
    fn transition_to_open(&self) {
        self.success_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
        self.last_transition_ms.store(now_ms(), Ordering::Relaxed);
        // Release ensures all above writes are visible before state change
        self.state
            .store(CircuitState::Open as u8, Ordering::Release);
    }

    /// Transition to half-open state
    ///
    /// Uses Release ordering to ensure counter resets are visible before state change.
    fn transition_to_half_open(&self) {
        self.success_count.store(0, Ordering::Relaxed);
        self.half_open_requests.store(0, Ordering::Relaxed);
        self.last_transition_ms.store(now_ms(), Ordering::Relaxed);
        // Release ensures all above writes are visible before state change
        self.state
            .store(CircuitState::HalfOpen as u8, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_circuit_starts_in_closed_state() {
        let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.should_allow_request());
    }

    #[test]
    fn test_circuit_opens_after_failure_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Record 2 failures - should stay closed
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.should_allow_request());

        // Record 3rd failure - should open
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.should_allow_request());
    }

    #[test]
    fn test_circuit_resets_failure_count_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Record 2 failures
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count(), 2);

        // Record success - should reset count
        breaker.record_success();
        assert_eq!(breaker.failure_count(), 0);
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_transitions_to_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_duration: Duration::from_millis(100),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.should_allow_request());

        // Wait for timeout
        thread::sleep(Duration::from_millis(150));

        // Should transition to half-open and allow request
        assert!(breaker.should_allow_request());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_closes_after_success_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            timeout_duration: Duration::from_millis(10),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait and transition to half-open
        thread::sleep(Duration::from_millis(20));
        assert!(breaker.should_allow_request());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record first success - should stay half-open
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record second success - should close
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.should_allow_request());
    }

    #[test]
    fn test_half_open_reopens_on_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_duration: Duration::from_millis(10),
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait and transition to half-open
        thread::sleep(Duration::from_millis(20));
        assert!(breaker.should_allow_request());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Record failure - should reopen immediately
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.should_allow_request());
    }

    #[test]
    fn test_half_open_limits_concurrent_requests() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            timeout_duration: Duration::from_millis(10),
            half_open_max_requests: 3,
            ..Default::default()
        };
        let breaker = CircuitBreaker::new(config);

        // Open the circuit
        breaker.record_failure();

        // Wait and transition to half-open
        thread::sleep(Duration::from_millis(20));
        breaker.should_allow_request(); // Transition to half-open

        // Simulate 3 concurrent requests
        breaker.start_half_open_request();
        breaker.start_half_open_request();
        breaker.start_half_open_request();

        // 4th request should be rejected
        assert!(!breaker.should_allow_request());

        // After requests complete, should allow more
        breaker.record_success(); // Decrements half_open_requests
        assert!(breaker.should_allow_request());
    }

    #[test]
    fn test_circuit_state_numeric_values() {
        assert_eq!(CircuitState::Closed as u8, 0);
        assert_eq!(CircuitState::Open as u8, 1);
        assert_eq!(CircuitState::HalfOpen as u8, 2);
    }

    #[test]
    fn test_circuit_state_from_u8() {
        assert_eq!(CircuitState::from(0), CircuitState::Closed);
        assert_eq!(CircuitState::from(1), CircuitState::Open);
        assert_eq!(CircuitState::from(2), CircuitState::HalfOpen);
        assert_eq!(CircuitState::from(99), CircuitState::Closed); // Invalid defaults to Closed
    }

    #[test]
    fn test_default_config_values() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.timeout_duration, Duration::from_secs(60));
        assert_eq!(config.half_open_max_requests, 3);
    }
}
