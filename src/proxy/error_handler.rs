//! Error handler module for the proxy.
//!
//! This module provides error handling and retry logic helpers.
//! It extracts error-related logic from the main proxy module for better
//! organization and testability.
//!
//! # Design
//!
//! Functions return data structures describing error actions instead of
//! modifying Pingora error objects directly. This keeps error handling
//! testable and allows the caller to apply actions appropriately.

use crate::retry::RetryPolicy;

// ============================================================================
// Result Types
// ============================================================================

/// Action to take after an error occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorAction {
    /// Retry the request.
    Retry {
        /// Current attempt number (1-indexed, before retry).
        current_attempt: u32,
        /// Next attempt number (after retry).
        next_attempt: u32,
        /// Maximum allowed attempts.
        max_attempts: u32,
    },
    /// Fail the request without retrying.
    Fail {
        /// Reason for not retrying.
        reason: FailReason,
    },
}

/// Reason why a request cannot be retried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailReason {
    /// Maximum retry attempts have been exhausted.
    AttemptsExhausted {
        /// Number of attempts made.
        attempts: u32,
        /// Maximum allowed attempts.
        max_attempts: u32,
    },
    /// No retry policy is configured for this bucket.
    NoRetryPolicy,
    /// Client connection cannot be reused for retry.
    ClientNotReusable,
    /// Response buffer was truncated, cannot retry.
    BufferTruncated,
    /// Status code is not retriable according to policy.
    NotRetriableStatus {
        /// The status code.
        status: u16,
    },
}

/// Result of checking retry eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryEligibility {
    /// Retry is allowed.
    Eligible {
        /// Current attempt number.
        current_attempt: u32,
    },
    /// Retry is not allowed.
    NotEligible {
        /// Reason why retry is not allowed.
        reason: FailReason,
    },
}

/// Context for proxy error handling.
#[derive(Debug, Clone)]
pub struct ProxyErrorContext {
    /// Whether the client connection can be reused.
    pub client_reused: bool,
    /// Whether the response buffer was truncated.
    pub buffer_truncated: bool,
}

// ============================================================================
// Retry Policy Checking
// ============================================================================

/// Check if a retry should be attempted based on policy and current state.
///
/// This function uses 1-indexed attempts consistently:
/// - attempt=1 is the first attempt
/// - attempt=max_attempts is the last allowed attempt (no more retries)
///
/// # Arguments
///
/// * `retry_policy` - Optional retry policy for the bucket.
/// * `current_attempt` - Current attempt number (1-indexed).
/// * `status_code` - HTTP status code to check for retriability.
///
/// # Returns
///
/// A `RetryEligibility` indicating whether retry is allowed.
///
/// # Note
///
/// This uses `RetryPolicy::is_retriable_status()` which only considers
/// 500/502/503/504 as retriable. The broader `RETRIABLE_STATUS_CODES`
/// constant in this module (which also includes 408/429) is provided
/// for classification purposes but not used for policy-based retry decisions.
pub fn check_retry_eligibility(
    retry_policy: Option<&RetryPolicy>,
    current_attempt: u32,
    status_code: u16,
) -> RetryEligibility {
    let Some(policy) = retry_policy else {
        return RetryEligibility::NotEligible {
            reason: FailReason::NoRetryPolicy,
        };
    };

    // Check attempts first (1-indexed: attempt >= max_attempts means exhausted)
    if current_attempt >= policy.max_attempts {
        return RetryEligibility::NotEligible {
            reason: FailReason::AttemptsExhausted {
                attempts: current_attempt,
                max_attempts: policy.max_attempts,
            },
        };
    }

    // Check if status code is retriable according to policy
    if !policy.is_retriable_status(status_code) {
        return RetryEligibility::NotEligible {
            reason: FailReason::NotRetriableStatus {
                status: status_code,
            },
        };
    }

    RetryEligibility::Eligible { current_attempt }
}

/// Determine action for a connection failure.
///
/// Connection failures (fail_to_connect) are simpler than proxy errors
/// because we haven't started sending the response yet.
///
/// # Arguments
///
/// * `retry_policy` - Optional retry policy for the bucket.
/// * `current_attempt` - Current attempt number (1-indexed).
///
/// # Returns
///
/// An `ErrorAction` describing whether to retry or fail.
pub fn determine_connection_error_action(
    retry_policy: Option<&RetryPolicy>,
    current_attempt: u32,
) -> ErrorAction {
    // Use 502 as status code for connection failures
    let eligibility = check_retry_eligibility(retry_policy, current_attempt, 502);

    match eligibility {
        RetryEligibility::Eligible { current_attempt } => {
            let max_attempts = retry_policy.map(|p| p.max_attempts).unwrap_or(1);
            ErrorAction::Retry {
                current_attempt,
                next_attempt: current_attempt + 1,
                max_attempts,
            }
        }
        RetryEligibility::NotEligible { reason } => ErrorAction::Fail { reason },
    }
}

/// Determine action for a proxy error (error during data transfer).
///
/// Proxy errors are more complex because we need to check if the client
/// connection can be reused and if the buffer hasn't been truncated.
///
/// # Arguments
///
/// * `retry_policy` - Optional retry policy for the bucket.
/// * `current_attempt` - Current attempt number (1-indexed).
/// * `context` - Proxy error context with client state.
///
/// # Returns
///
/// An `ErrorAction` describing whether to retry or fail.
pub fn determine_proxy_error_action(
    retry_policy: Option<&RetryPolicy>,
    current_attempt: u32,
    context: &ProxyErrorContext,
) -> ErrorAction {
    // First check retry eligibility based on policy
    let eligibility = check_retry_eligibility(retry_policy, current_attempt, 502);

    match eligibility {
        RetryEligibility::Eligible { current_attempt } => {
            // Check client state allows retry
            if !context.client_reused {
                return ErrorAction::Fail {
                    reason: FailReason::ClientNotReusable,
                };
            }
            if context.buffer_truncated {
                return ErrorAction::Fail {
                    reason: FailReason::BufferTruncated,
                };
            }

            let max_attempts = retry_policy.map(|p| p.max_attempts).unwrap_or(1);
            ErrorAction::Retry {
                current_attempt,
                next_attempt: current_attempt + 1,
                max_attempts,
            }
        }
        RetryEligibility::NotEligible { reason } => ErrorAction::Fail { reason },
    }
}

/// Check if a request can be retried based on client state.
///
/// # Arguments
///
/// * `client_reused` - Whether the client connection can be reused.
/// * `buffer_truncated` - Whether the response buffer was truncated.
///
/// # Returns
///
/// `true` if the request can be retried, `false` otherwise.
pub fn can_retry_request(client_reused: bool, buffer_truncated: bool) -> bool {
    client_reused && !buffer_truncated
}

// ============================================================================
// Error Classification
// ============================================================================

/// HTTP status codes that are typically retriable.
pub const RETRIABLE_STATUS_CODES: &[u16] = &[
    408, // Request Timeout
    429, // Too Many Requests
    500, // Internal Server Error
    502, // Bad Gateway
    503, // Service Unavailable
    504, // Gateway Timeout
];

/// Check if a status code is considered retriable.
///
/// # Arguments
///
/// * `status_code` - HTTP status code to check.
///
/// # Returns
///
/// `true` if the status code is retriable, `false` otherwise.
pub fn is_retriable_status(status_code: u16) -> bool {
    RETRIABLE_STATUS_CODES.contains(&status_code)
}

/// Classify an error as transient or permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClassification {
    /// Transient error that may succeed on retry.
    Transient,
    /// Permanent error that will not succeed on retry.
    Permanent,
}

/// Classify an HTTP status code.
///
/// # Arguments
///
/// * `status_code` - HTTP status code to classify.
///
/// # Returns
///
/// Classification as transient or permanent.
pub fn classify_status(status_code: u16) -> ErrorClassification {
    if is_retriable_status(status_code) {
        ErrorClassification::Transient
    } else {
        ErrorClassification::Permanent
    }
}

// ============================================================================
// Backoff Calculation
// ============================================================================

/// Maximum exponent for backoff calculation to prevent overflow.
/// With this cap, the maximum multiplier is 2^10 = 1024.
const MAX_BACKOFF_EXPONENT: u32 = 10;

/// Calculate backoff delay for a retry attempt.
///
/// Uses exponential backoff: `initial_backoff_ms * 2^(attempt-1)`.
/// The result is capped at `max_backoff_ms`.
///
/// Note: This is a deterministic calculation without jitter.
/// For production use with multiple clients, consider adding
/// jitter on top of this base delay to prevent thundering herd.
///
/// # Arguments
///
/// * `attempt` - Current attempt number (1-indexed).
/// * `initial_backoff_ms` - Initial backoff delay in milliseconds.
/// * `max_backoff_ms` - Maximum backoff delay in milliseconds.
///
/// # Returns
///
/// Backoff delay in milliseconds.
pub fn calculate_backoff(attempt: u32, initial_backoff_ms: u64, max_backoff_ms: u64) -> u64 {
    // Exponential backoff: initial * 2^(attempt-1)
    // Cap exponent to prevent overflow
    let exponent = attempt.saturating_sub(1).min(MAX_BACKOFF_EXPONENT);
    let base_delay = initial_backoff_ms.saturating_mul(1u64 << exponent);

    // Cap at max backoff
    base_delay.min(max_backoff_ms)
}

/// Calculate backoff delay using a retry policy.
///
/// # Arguments
///
/// * `policy` - The retry policy.
/// * `attempt` - Current attempt number (1-indexed).
///
/// # Returns
///
/// Backoff delay in milliseconds.
pub fn calculate_policy_backoff(policy: &RetryPolicy, attempt: u32) -> u64 {
    calculate_backoff(attempt, policy.initial_backoff_ms, policy.max_backoff_ms)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_retry_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 1000,
        }
    }

    #[test]
    fn test_error_handler_module_exists() {
        // Phase 37.7 structural verification test
        let _ = ErrorAction::Fail {
            reason: FailReason::NoRetryPolicy,
        };
        let _ = RetryEligibility::Eligible { current_attempt: 1 };
        let _ = ErrorClassification::Transient;
    }

    // ========== Retry Eligibility Tests ==========

    #[test]
    fn test_check_retry_eligibility_no_policy() {
        let result = check_retry_eligibility(None, 1, 502);

        assert_eq!(
            result,
            RetryEligibility::NotEligible {
                reason: FailReason::NoRetryPolicy
            }
        );
    }

    #[test]
    fn test_check_retry_eligibility_first_attempt() {
        let policy = test_retry_policy();
        let result = check_retry_eligibility(Some(&policy), 1, 502);

        assert_eq!(result, RetryEligibility::Eligible { current_attempt: 1 });
    }

    #[test]
    fn test_check_retry_eligibility_attempts_exhausted() {
        let policy = test_retry_policy();
        let result = check_retry_eligibility(Some(&policy), 3, 502);

        assert_eq!(
            result,
            RetryEligibility::NotEligible {
                reason: FailReason::AttemptsExhausted {
                    attempts: 3,
                    max_attempts: 3,
                }
            }
        );
    }

    // ========== Connection Error Tests ==========

    #[test]
    fn test_determine_connection_error_action_retry() {
        let policy = test_retry_policy();
        let action = determine_connection_error_action(Some(&policy), 1);

        assert_eq!(
            action,
            ErrorAction::Retry {
                current_attempt: 1,
                next_attempt: 2,
                max_attempts: 3,
            }
        );
    }

    #[test]
    fn test_determine_connection_error_action_no_policy() {
        let action = determine_connection_error_action(None, 1);

        assert_eq!(
            action,
            ErrorAction::Fail {
                reason: FailReason::NoRetryPolicy
            }
        );
    }

    #[test]
    fn test_determine_connection_error_action_exhausted() {
        let policy = test_retry_policy();
        let action = determine_connection_error_action(Some(&policy), 3);

        assert_eq!(
            action,
            ErrorAction::Fail {
                reason: FailReason::AttemptsExhausted {
                    attempts: 3,
                    max_attempts: 3,
                }
            }
        );
    }

    // ========== Proxy Error Tests ==========

    #[test]
    fn test_determine_proxy_error_action_retry() {
        let policy = test_retry_policy();
        let context = ProxyErrorContext {
            client_reused: true,
            buffer_truncated: false,
        };
        let action = determine_proxy_error_action(Some(&policy), 1, &context);

        assert_eq!(
            action,
            ErrorAction::Retry {
                current_attempt: 1,
                next_attempt: 2,
                max_attempts: 3,
            }
        );
    }

    #[test]
    fn test_determine_proxy_error_action_client_not_reusable() {
        let policy = test_retry_policy();
        let context = ProxyErrorContext {
            client_reused: false,
            buffer_truncated: false,
        };
        let action = determine_proxy_error_action(Some(&policy), 1, &context);

        assert_eq!(
            action,
            ErrorAction::Fail {
                reason: FailReason::ClientNotReusable
            }
        );
    }

    #[test]
    fn test_determine_proxy_error_action_buffer_truncated() {
        let policy = test_retry_policy();
        let context = ProxyErrorContext {
            client_reused: true,
            buffer_truncated: true,
        };
        let action = determine_proxy_error_action(Some(&policy), 1, &context);

        assert_eq!(
            action,
            ErrorAction::Fail {
                reason: FailReason::BufferTruncated
            }
        );
    }

    // ========== Can Retry Tests ==========

    #[test]
    fn test_can_retry_request_all_conditions_met() {
        assert!(can_retry_request(true, false));
    }

    #[test]
    fn test_can_retry_request_client_not_reusable() {
        assert!(!can_retry_request(false, false));
    }

    #[test]
    fn test_can_retry_request_buffer_truncated() {
        assert!(!can_retry_request(true, true));
    }

    // ========== Status Code Tests ==========

    #[test]
    fn test_is_retriable_status_502() {
        assert!(is_retriable_status(502));
    }

    #[test]
    fn test_is_retriable_status_503() {
        assert!(is_retriable_status(503));
    }

    #[test]
    fn test_is_retriable_status_429() {
        assert!(is_retriable_status(429));
    }

    #[test]
    fn test_is_retriable_status_404_not_retriable() {
        assert!(!is_retriable_status(404));
    }

    #[test]
    fn test_is_retriable_status_200_not_retriable() {
        assert!(!is_retriable_status(200));
    }

    #[test]
    fn test_classify_status_transient() {
        assert_eq!(classify_status(502), ErrorClassification::Transient);
        assert_eq!(classify_status(503), ErrorClassification::Transient);
        assert_eq!(classify_status(429), ErrorClassification::Transient);
    }

    #[test]
    fn test_classify_status_permanent() {
        assert_eq!(classify_status(404), ErrorClassification::Permanent);
        assert_eq!(classify_status(403), ErrorClassification::Permanent);
        assert_eq!(classify_status(400), ErrorClassification::Permanent);
    }

    // ========== Backoff Tests ==========

    #[test]
    fn test_calculate_backoff_first_attempt() {
        let backoff = calculate_backoff(1, 100, 1000);
        assert_eq!(backoff, 100); // 100 * 2^0 = 100
    }

    #[test]
    fn test_calculate_backoff_second_attempt() {
        let backoff = calculate_backoff(2, 100, 1000);
        assert_eq!(backoff, 200); // 100 * 2^1 = 200
    }

    #[test]
    fn test_calculate_backoff_third_attempt() {
        let backoff = calculate_backoff(3, 100, 1000);
        assert_eq!(backoff, 400); // 100 * 2^2 = 400
    }

    #[test]
    fn test_calculate_backoff_capped_at_max() {
        let backoff = calculate_backoff(10, 100, 1000);
        assert_eq!(backoff, 1000); // Should be capped at max
    }

    #[test]
    fn test_calculate_policy_backoff() {
        let policy = test_retry_policy();
        let backoff = calculate_policy_backoff(&policy, 2);
        assert_eq!(backoff, 200); // 100 * 2^1 = 200
    }
}
