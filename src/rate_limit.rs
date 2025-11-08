//! Rate Limiting with Token Bucket Algorithm
//!
//! Prevents abuse and resource exhaustion by limiting request rates at three levels:
//! - **Global**: Limit total requests per second across all buckets
//! - **Per-Bucket**: Limit requests per bucket
//! - **Per-IP**: Limit requests per client IP address
//!
//! Uses the `governor` crate's token bucket algorithm with these characteristics:
//! - Sliding window (smoother rate limiting than fixed windows)
//! - Fast (lock-free atomic operations)
//! - Memory efficient (in-memory state)
//!
//! ## Rate Limiting Strategy
//!
//! Limits are checked in this order (fail fast):
//! 1. Global limit (if enabled)
//! 2. Per-IP limit (if enabled)
//! 3. Per-bucket limit (if configured for the bucket)
//!
//! If any limit is exceeded, return 429 Too Many Requests immediately.
//!
//! ## Configuration Example
//!
//! ```yaml
//! server:
//!   rate_limit:
//!     enabled: true
//!     global:
//!       requests_per_second: 1000
//!     per_ip:
//!       requests_per_second: 10
//!
//! buckets:
//!   - name: products
//!     s3:
//!       rate_limit:
//!         requests_per_second: 100
//! ```

use governor::{clock::DefaultClock, state::InMemoryState, Quota, RateLimiter};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiter manager handling global, per-bucket, and per-IP limits
pub struct RateLimitManager {
    /// Global rate limiter (all requests)
    global: Option<Arc<RateLimiter<governor::state::NotKeyed, InMemoryState, DefaultClock>>>,
    /// Per-bucket rate limiters (keyed by bucket name)
    #[allow(clippy::type_complexity)]
    buckets: Arc<
        RwLock<
            HashMap<
                String,
                Arc<RateLimiter<governor::state::NotKeyed, InMemoryState, DefaultClock>>,
            >,
        >,
    >,
    /// Per-IP rate limiters (keyed by IP address)
    #[allow(clippy::type_complexity)]
    ips: Arc<
        RwLock<
            HashMap<
                IpAddr,
                Arc<RateLimiter<governor::state::NotKeyed, InMemoryState, DefaultClock>>,
            >,
        >,
    >,
    /// Per-IP rate limit config (requests per second)
    per_ip_rps: Option<NonZeroU32>,
}

impl RateLimitManager {
    /// Create a new rate limit manager with optional global and per-IP limits
    ///
    /// # Arguments
    /// * `global_rps` - Global requests per second limit (None = disabled)
    /// * `per_ip_rps` - Per-IP requests per second limit (None = disabled)
    pub fn new(global_rps: Option<u32>, per_ip_rps: Option<u32>) -> Self {
        let global = global_rps.and_then(|rps| {
            NonZeroU32::new(rps).map(|nz| Arc::new(RateLimiter::direct(Quota::per_second(nz))))
        });

        let per_ip_rps = per_ip_rps.and_then(NonZeroU32::new);

        Self {
            global,
            buckets: Arc::new(RwLock::new(HashMap::new())),
            ips: Arc::new(RwLock::new(HashMap::new())),
            per_ip_rps,
        }
    }

    /// Add a per-bucket rate limiter
    ///
    /// # Arguments
    /// * `bucket_name` - Name of the bucket
    /// * `requests_per_second` - Rate limit for this bucket
    pub fn add_bucket_limiter(&self, bucket_name: String, requests_per_second: u32) {
        if let Some(nz) = NonZeroU32::new(requests_per_second) {
            let limiter = Arc::new(RateLimiter::direct(Quota::per_second(nz)));
            self.buckets.write().insert(bucket_name, limiter);
        }
    }

    /// Check if a request should be allowed (global limit)
    ///
    /// Returns true if allowed, false if rate limit exceeded
    pub fn check_global(&self) -> bool {
        if let Some(ref limiter) = self.global {
            limiter.check().is_ok()
        } else {
            true // No global limit configured
        }
    }

    /// Check if a request should be allowed for a specific bucket
    ///
    /// Returns true if allowed, false if rate limit exceeded
    pub fn check_bucket(&self, bucket_name: &str) -> bool {
        let limiters = self.buckets.read();
        if let Some(limiter) = limiters.get(bucket_name) {
            limiter.check().is_ok()
        } else {
            true // No bucket-specific limit configured
        }
    }

    /// Check if a request should be allowed for a specific IP address
    ///
    /// Returns true if allowed, false if rate limit exceeded
    pub fn check_ip(&self, ip: IpAddr) -> bool {
        if self.per_ip_rps.is_none() {
            return true; // No per-IP limit configured
        }

        // Fast path: check if limiter exists
        {
            let limiters = self.ips.read();
            if let Some(limiter) = limiters.get(&ip) {
                return limiter.check().is_ok();
            }
        }

        // Slow path: create limiter for new IP
        let mut limiters = self.ips.write();
        let limiter = limiters.entry(ip).or_insert_with(|| {
            let rps = self.per_ip_rps.unwrap(); // Safe: checked above
            Arc::new(RateLimiter::direct(Quota::per_second(rps)))
        });
        limiter.check().is_ok()
    }

    /// Check all rate limits for a request
    ///
    /// Returns Ok(()) if allowed, Err(RateLimitError) with which limit was hit
    pub fn check_all(
        &self,
        bucket_name: &str,
        client_ip: Option<IpAddr>,
    ) -> Result<(), RateLimitError> {
        // Check global limit first
        if !self.check_global() {
            return Err(RateLimitError::Global);
        }

        // Check per-IP limit
        if let Some(ip) = client_ip {
            if !self.check_ip(ip) {
                return Err(RateLimitError::PerIp(ip));
            }
        }

        // Check per-bucket limit
        if !self.check_bucket(bucket_name) {
            return Err(RateLimitError::PerBucket(bucket_name.to_string()));
        }

        Ok(())
    }

    /// Get count of tracked IPs (for metrics/monitoring)
    pub fn tracked_ip_count(&self) -> usize {
        self.ips.read().len()
    }

    /// Get count of tracked buckets (for metrics/monitoring)
    pub fn tracked_bucket_count(&self) -> usize {
        self.buckets.read().len()
    }

    /// Clean up IP limiters that haven't been used recently
    /// (prevents unbounded memory growth for per-IP tracking)
    ///
    /// This should be called periodically (e.g., every 5 minutes)
    pub fn cleanup_stale_ips(&self, max_ips: usize) {
        let mut ips = self.ips.write();
        if ips.len() > max_ips {
            // Simple strategy: clear all if too many
            // In production, could use LRU or timestamp-based eviction
            tracing::info!(
                ip_count = ips.len(),
                max_ips = max_ips,
                "Cleaning up per-IP rate limiters"
            );
            ips.clear();
        }
    }
}

/// Error type indicating which rate limit was exceeded
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitError {
    /// Global rate limit exceeded
    Global,
    /// Per-IP rate limit exceeded
    PerIp(IpAddr),
    /// Per-bucket rate limit exceeded
    PerBucket(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::Global => write!(f, "Global rate limit exceeded"),
            RateLimitError::PerIp(ip) => write!(f, "Rate limit exceeded for IP: {}", ip),
            RateLimitError::PerBucket(bucket) => {
                write!(f, "Rate limit exceeded for bucket: {}", bucket)
            }
        }
    }
}

impl std::error::Error for RateLimitError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_no_limits_allows_all_requests() {
        let manager = RateLimitManager::new(None, None);

        // Without any limits configured, all requests should be allowed
        for _ in 0..1000 {
            assert!(manager.check_global());
            assert!(manager.check_bucket("test-bucket"));
            assert!(manager.check_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        }
    }

    #[test]
    fn test_global_rate_limit_enforced() {
        let manager = RateLimitManager::new(Some(5), None); // 5 requests per second

        // First 5 requests should succeed
        for i in 0..5 {
            assert!(
                manager.check_global(),
                "Request {} should be allowed",
                i + 1
            );
        }

        // 6th request should be rate limited
        assert!(
            !manager.check_global(),
            "6th request should be rate limited"
        );
    }

    #[test]
    fn test_per_bucket_rate_limit_enforced() {
        let manager = RateLimitManager::new(None, None);
        manager.add_bucket_limiter("products".to_string(), 3); // 3 requests per second

        // First 3 requests should succeed
        for i in 0..3 {
            assert!(
                manager.check_bucket("products"),
                "Request {} should be allowed",
                i + 1
            );
        }

        // 4th request should be rate limited
        assert!(
            !manager.check_bucket("products"),
            "4th request should be rate limited"
        );

        // Other buckets should not be affected
        assert!(manager.check_bucket("other-bucket"));
    }

    #[test]
    fn test_per_ip_rate_limit_enforced() {
        let manager = RateLimitManager::new(None, Some(2)); // 2 requests per second per IP
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        // First 2 requests should succeed
        for i in 0..2 {
            assert!(manager.check_ip(ip), "Request {} should be allowed", i + 1);
        }

        // 3rd request should be rate limited
        assert!(!manager.check_ip(ip), "3rd request should be rate limited");

        // Different IP should not be affected
        let other_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101));
        assert!(manager.check_ip(other_ip));
    }

    #[test]
    fn test_check_all_enforces_all_limits() {
        let manager = RateLimitManager::new(Some(10), Some(5));
        manager.add_bucket_limiter("products".to_string(), 3);

        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        // First 3 requests should succeed (bucket limit is lowest)
        for _ in 0..3 {
            assert!(manager.check_all("products", Some(ip)).is_ok());
        }

        // 4th request should fail due to bucket limit
        let result = manager.check_all("products", Some(ip));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RateLimitError::PerBucket("products".to_string())
        );
    }

    #[test]
    fn test_rate_limit_refills_over_time() {
        let manager = RateLimitManager::new(Some(10), None); // 10 requests per second

        // Consume all tokens
        for _ in 0..10 {
            assert!(manager.check_global());
        }

        // Next request should fail
        assert!(!manager.check_global());

        // Wait for token bucket to refill (>100ms for 1 token at 10/sec)
        thread::sleep(Duration::from_millis(150));

        // Should have at least 1 token now
        assert!(manager.check_global(), "Token bucket should have refilled");
    }

    #[test]
    fn test_zero_rate_disables_limiter() {
        let manager = RateLimitManager::new(Some(0), Some(0));

        // Zero rate should disable rate limiting (treated as None)
        for _ in 0..100 {
            assert!(manager.check_global());
        }
    }

    #[test]
    fn test_tracked_counts() {
        let manager = RateLimitManager::new(None, Some(10));
        manager.add_bucket_limiter("bucket1".to_string(), 10);
        manager.add_bucket_limiter("bucket2".to_string(), 20);

        assert_eq!(manager.tracked_bucket_count(), 2);

        // Trigger IP tracking
        let ip1 = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        manager.check_ip(ip1);
        manager.check_ip(ip2);

        assert_eq!(manager.tracked_ip_count(), 2);
    }

    #[test]
    fn test_cleanup_stale_ips() {
        let manager = RateLimitManager::new(None, Some(10));

        // Add 100 IPs
        for i in 0..100 {
            let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, i));
            manager.check_ip(ip);
        }

        assert_eq!(manager.tracked_ip_count(), 100);

        // Cleanup with max 50 should clear all
        manager.cleanup_stale_ips(50);
        assert_eq!(manager.tracked_ip_count(), 0);
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err1 = RateLimitError::Global;
        assert_eq!(err1.to_string(), "Global rate limit exceeded");

        let err2 = RateLimitError::PerIp(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
        assert_eq!(err2.to_string(), "Rate limit exceeded for IP: 1.2.3.4");

        let err3 = RateLimitError::PerBucket("products".to_string());
        assert_eq!(err3.to_string(), "Rate limit exceeded for bucket: products");
    }
}
