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

/// Maximum number of per-IP rate limiters to track before cleanup
const DEFAULT_MAX_IP_LIMITERS: usize = 100_000;
/// Maximum number of per-user rate limiters to track before cleanup
const DEFAULT_MAX_USER_LIMITERS: usize = 50_000;

/// Rate limiter manager handling global, per-bucket, per-IP, and per-user limits
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
    /// Per-user rate limiters (keyed by user ID from JWT)
    #[allow(clippy::type_complexity)]
    users: Arc<
        RwLock<
            HashMap<
                String,
                Arc<RateLimiter<governor::state::NotKeyed, InMemoryState, DefaultClock>>,
            >,
        >,
    >,
    /// Per-IP rate limit config (requests per second)
    per_ip_rps: Option<NonZeroU32>,
    /// Per-user rate limit config (requests per second)
    per_user_rps: Option<NonZeroU32>,
    /// Maximum number of tracked IPs before cleanup
    max_ip_limiters: usize,
    /// Maximum number of tracked users before cleanup
    max_user_limiters: usize,
}

impl RateLimitManager {
    /// Create a new rate limit manager with optional global and per-IP limits
    ///
    /// # Arguments
    /// * `global_rps` - Global requests per second limit (None = disabled)
    /// * `per_ip_rps` - Per-IP requests per second limit (None = disabled)
    pub fn new(global_rps: Option<u32>, per_ip_rps: Option<u32>) -> Self {
        Self::with_user_limit(global_rps, per_ip_rps, None)
    }

    /// Create a new rate limit manager with optional global, per-IP, and per-user limits
    ///
    /// # Arguments
    /// * `global_rps` - Global requests per second limit (None = disabled)
    /// * `per_ip_rps` - Per-IP requests per second limit (None = disabled)
    /// * `per_user_rps` - Per-user requests per second limit (None = disabled)
    pub fn with_user_limit(
        global_rps: Option<u32>,
        per_ip_rps: Option<u32>,
        per_user_rps: Option<u32>,
    ) -> Self {
        let global = global_rps.and_then(|rps| {
            NonZeroU32::new(rps).map(|nz| Arc::new(RateLimiter::direct(Quota::per_second(nz))))
        });

        let per_ip_rps = per_ip_rps.and_then(NonZeroU32::new);
        let per_user_rps = per_user_rps.and_then(NonZeroU32::new);

        Self {
            global,
            buckets: Arc::new(RwLock::new(HashMap::new())),
            ips: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            per_ip_rps,
            per_user_rps,
            max_ip_limiters: DEFAULT_MAX_IP_LIMITERS,
            max_user_limiters: DEFAULT_MAX_USER_LIMITERS,
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
    ///
    /// If the number of tracked IPs exceeds `max_ip_limiters`, older entries
    /// are cleared to prevent unbounded memory growth under DDoS attacks.
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

        // Enforce max limiter count to prevent memory exhaustion
        if limiters.len() >= self.max_ip_limiters {
            tracing::warn!(
                ip_count = limiters.len(),
                max_ips = self.max_ip_limiters,
                "Per-IP rate limiter count exceeded max, clearing all to prevent memory exhaustion"
            );
            limiters.clear();
        }

        let limiter = limiters.entry(ip).or_insert_with(|| {
            let rps = self.per_ip_rps.unwrap(); // Safe: checked above
            Arc::new(RateLimiter::direct(Quota::per_second(rps)))
        });
        limiter.check().is_ok()
    }

    /// Check if a request should be allowed for a specific user (from JWT claims)
    ///
    /// Returns true if allowed, false if rate limit exceeded
    ///
    /// If the number of tracked users exceeds `max_user_limiters`, older entries
    /// are cleared to prevent unbounded memory growth.
    pub fn check_user(&self, user_id: &str) -> bool {
        if self.per_user_rps.is_none() {
            return true; // No per-user limit configured
        }

        // Fast path: check if limiter exists
        {
            let limiters = self.users.read();
            if let Some(limiter) = limiters.get(user_id) {
                return limiter.check().is_ok();
            }
        }

        // Slow path: create limiter for new user
        let mut limiters = self.users.write();

        // Enforce max limiter count to prevent memory exhaustion
        if limiters.len() >= self.max_user_limiters {
            tracing::warn!(
                user_count = limiters.len(),
                max_users = self.max_user_limiters,
                "Per-user rate limiter count exceeded max, clearing all to prevent memory exhaustion"
            );
            limiters.clear();
        }

        let limiter = limiters.entry(user_id.to_string()).or_insert_with(|| {
            let rps = self.per_user_rps.unwrap(); // Safe: checked above
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
        self.check_all_with_user(bucket_name, client_ip, None)
    }

    /// Check all rate limits for a request, including per-user limits
    ///
    /// Returns Ok(()) if allowed, Err(RateLimitError) with which limit was hit
    ///
    /// # Arguments
    /// * `bucket_name` - Name of the bucket being accessed
    /// * `client_ip` - Client IP address (for per-IP limits)
    /// * `user_id` - User ID from JWT claims (for per-user limits)
    pub fn check_all_with_user(
        &self,
        bucket_name: &str,
        client_ip: Option<IpAddr>,
        user_id: Option<&str>,
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

        // Check per-user limit (from JWT)
        if let Some(user) = user_id {
            if !self.check_user(user) {
                return Err(RateLimitError::PerUser(user.to_string()));
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

    /// Get count of tracked users (for metrics/monitoring)
    pub fn tracked_user_count(&self) -> usize {
        self.users.read().len()
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

    /// Clean up user limiters that haven't been used recently
    /// (prevents unbounded memory growth for per-user tracking)
    ///
    /// This should be called periodically (e.g., every 5 minutes)
    pub fn cleanup_stale_users(&self, max_users: usize) {
        let mut users = self.users.write();
        if users.len() > max_users {
            tracing::info!(
                user_count = users.len(),
                max_users = max_users,
                "Cleaning up per-user rate limiters"
            );
            users.clear();
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
    /// Per-user rate limit exceeded (user ID from JWT)
    PerUser(String),
    /// Per-bucket rate limit exceeded
    PerBucket(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::Global => write!(f, "Global rate limit exceeded"),
            RateLimitError::PerIp(ip) => write!(f, "Rate limit exceeded for IP: {}", ip),
            RateLimitError::PerUser(user) => write!(f, "Rate limit exceeded for user: {}", user),
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

        let err3 = RateLimitError::PerUser("user123".to_string());
        assert_eq!(err3.to_string(), "Rate limit exceeded for user: user123");

        let err4 = RateLimitError::PerBucket("products".to_string());
        assert_eq!(err4.to_string(), "Rate limit exceeded for bucket: products");
    }

    // ============================================================
    // Per-User Rate Limiting Tests (Phase 35)
    // ============================================================

    #[test]
    fn test_per_user_rate_limit_enforced() {
        let manager = RateLimitManager::with_user_limit(None, None, Some(3)); // 3 req/s per user

        // First 3 requests should succeed
        for i in 0..3 {
            assert!(
                manager.check_user("user-alice"),
                "Request {} should be allowed",
                i + 1
            );
        }

        // 4th request should be rate limited
        assert!(
            !manager.check_user("user-alice"),
            "4th request should be rate limited"
        );

        // Different user should not be affected
        assert!(manager.check_user("user-bob"));
    }

    #[test]
    fn test_per_user_limit_disabled_allows_all() {
        let manager = RateLimitManager::with_user_limit(None, None, None);

        // All requests should be allowed when per-user limit is not configured
        for _ in 0..100 {
            assert!(manager.check_user("any-user"));
        }
    }

    #[test]
    fn test_check_all_with_user_enforces_user_limit() {
        let manager = RateLimitManager::with_user_limit(Some(100), None, Some(2)); // 2 req/s per user
        manager.add_bucket_limiter("products".to_string(), 100);

        // First 2 requests should succeed
        for _ in 0..2 {
            assert!(manager
                .check_all_with_user("products", None, Some("user-charlie"))
                .is_ok());
        }

        // 3rd request should fail due to user limit
        let result = manager.check_all_with_user("products", None, Some("user-charlie"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RateLimitError::PerUser("user-charlie".to_string())
        );
    }

    #[test]
    fn test_check_all_with_user_no_user_bypasses_user_limit() {
        let manager = RateLimitManager::with_user_limit(None, None, Some(1));

        // Without user_id, per-user limit should not apply
        for _ in 0..10 {
            assert!(manager.check_all_with_user("bucket", None, None).is_ok());
        }
    }

    #[test]
    fn test_tracked_user_count() {
        let manager = RateLimitManager::with_user_limit(None, None, Some(10));

        assert_eq!(manager.tracked_user_count(), 0);

        manager.check_user("user1");
        assert_eq!(manager.tracked_user_count(), 1);

        manager.check_user("user2");
        assert_eq!(manager.tracked_user_count(), 2);

        manager.check_user("user1"); // Same user, no new entry
        assert_eq!(manager.tracked_user_count(), 2);
    }

    #[test]
    fn test_cleanup_stale_users() {
        let manager = RateLimitManager::with_user_limit(None, None, Some(100));

        // Add many users
        for i in 0..100 {
            manager.check_user(&format!("user-{}", i));
        }
        assert_eq!(manager.tracked_user_count(), 100);

        // Cleanup with max 50 should clear all
        manager.cleanup_stale_users(50);
        assert_eq!(manager.tracked_user_count(), 0);
    }

    #[test]
    fn test_user_rate_limit_with_ip_and_bucket() {
        // Test that all limits work together
        let manager = RateLimitManager::with_user_limit(Some(100), Some(100), Some(2));
        manager.add_bucket_limiter("api".to_string(), 100);

        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        // Per-user limit should be the bottleneck
        for _ in 0..2 {
            assert!(manager
                .check_all_with_user("api", Some(ip), Some("limited-user"))
                .is_ok());
        }

        // 3rd request should fail due to user limit
        let result = manager.check_all_with_user("api", Some(ip), Some("limited-user"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RateLimitError::PerUser("limited-user".to_string())
        );

        // Different user should still work
        assert!(manager
            .check_all_with_user("api", Some(ip), Some("other-user"))
            .is_ok());
    }
}
