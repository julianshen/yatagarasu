//! Resource monitoring and exhaustion prevention
//!
//! This module tracks system resource usage (file descriptors, memory)
//! and implements graceful degradation when approaching system limits.
//!
//! Graceful Degradation Strategy:
//! - 80% capacity: Log warning
//! - 90% capacity: Disable metrics collection (reduce overhead)
//! - 95% capacity: Return 503 for new requests (prevent crash)
//! - < 80% capacity: Resume normal operation

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Resource monitor that tracks system resource usage
#[derive(Clone)]
pub struct ResourceMonitor {
    /// Current file descriptor count
    fd_count: Arc<AtomicU64>,
    /// Maximum file descriptors allowed
    fd_limit: Arc<AtomicU64>,
    /// Current memory usage in bytes
    memory_usage: Arc<AtomicU64>,
    /// Maximum memory allowed in bytes
    memory_limit: Arc<AtomicU64>,
    /// Whether metrics collection is enabled (disabled under pressure)
    metrics_enabled: Arc<AtomicBool>,
}

/// Resource usage levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLevel {
    /// Normal operation (< 80%)
    Normal,
    /// Warning level (80-90%)
    Warning,
    /// Critical level (90-95%) - metrics disabled
    Critical,
    /// Exhausted (>= 95%) - reject new requests
    Exhausted,
}

/// Detect file descriptor limit from the operating system
///
/// Returns the soft limit for RLIMIT_NOFILE (number of open files).
/// Falls back to 10240 if detection fails or limit is unlimited.
#[allow(clippy::unnecessary_cast)] // rlim_t is u64 on 64-bit Linux, cast needed for portability
pub fn detect_fd_limit() -> u64 {
    #[cfg(unix)]
    {
        // Use libc to get rlimit on Unix systems
        unsafe {
            let mut rlim = std::mem::zeroed::<libc::rlimit>();
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) == 0 {
                // Check if soft limit is unlimited (RLIM_INFINITY)
                if rlim.rlim_cur == libc::RLIM_INFINITY {
                    // Use hard limit if it's reasonable, otherwise use default
                    if rlim.rlim_max != libc::RLIM_INFINITY && rlim.rlim_max > 0 {
                        let limit = rlim.rlim_max as u64;
                        tracing::debug!(
                            limit = limit,
                            "FD soft limit is unlimited, using hard limit"
                        );
                        return limit;
                    } else {
                        tracing::debug!("FD limit is unlimited, using default 10240");
                        return 10240;
                    }
                }

                // Use soft limit (can be changed by process)
                let limit = rlim.rlim_cur as u64;
                tracing::debug!(limit = limit, "Detected file descriptor soft limit");
                return limit;
            } else {
                tracing::warn!("Failed to detect FD limit via getrlimit, using default 10240");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tracing::warn!("FD limit detection not supported on this platform, using default 10240");
    }

    10240 // Default fallback (10K file descriptors)
}

/// Detect memory limit from the operating system
///
/// Returns total system memory. Falls back to 1GB if detection fails.
pub fn detect_memory_limit() -> u64 {
    #[cfg(target_os = "linux")]
    {
        // On Linux, read from /proc/meminfo
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    // Format: "MemTotal:       16384000 kB"
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            let bytes = kb * 1024;
                            tracing::debug!(
                                bytes = bytes,
                                "Detected system memory from /proc/meminfo"
                            );
                            return bytes;
                        }
                    }
                }
            }
        }
        tracing::warn!("Failed to read /proc/meminfo, using default 1GB");
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use sysctl to get hw.memsize
        unsafe {
            let mut size: u64 = 0;
            let mut len = std::mem::size_of::<u64>();
            let name = b"hw.memsize\0".as_ptr() as *const i8;

            if libc::sysctlbyname(
                name,
                &mut size as *mut u64 as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            ) == 0
            {
                tracing::debug!(bytes = size, "Detected system memory from sysctl");
                return size;
            } else {
                tracing::warn!("Failed to detect memory via sysctl, using default 1GB");
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        tracing::warn!("Memory detection not supported on this platform, using default 1GB");
    }

    1024 * 1024 * 1024 // 1GB default fallback
}

impl ResourceMonitor {
    /// Create a new resource monitor with system limits
    pub fn new(fd_limit: u64, memory_limit: u64) -> Self {
        Self {
            fd_count: Arc::new(AtomicU64::new(0)),
            fd_limit: Arc::new(AtomicU64::new(fd_limit)),
            memory_usage: Arc::new(AtomicU64::new(0)),
            memory_limit: Arc::new(AtomicU64::new(memory_limit)),
            metrics_enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Create a new resource monitor with auto-detected system limits
    pub fn new_auto_detect() -> Self {
        let fd_limit = detect_fd_limit();
        let memory_limit = detect_memory_limit();

        tracing::info!(
            fd_limit = fd_limit,
            memory_limit = memory_limit,
            "Auto-detected system resource limits"
        );

        Self::new(fd_limit, memory_limit)
    }

    /// Update current file descriptor count
    pub fn update_fd_count(&self, count: u64) {
        self.fd_count.store(count, Ordering::Relaxed);
        self.check_and_apply_degradation();
    }

    /// Update current memory usage
    pub fn update_memory_usage(&self, usage: u64) {
        self.memory_usage.store(usage, Ordering::Relaxed);
        self.check_and_apply_degradation();
    }

    /// Get current file descriptor usage percentage (0-100)
    pub fn fd_usage_percent(&self) -> f64 {
        let count = self.fd_count.load(Ordering::Relaxed) as f64;
        let limit = self.fd_limit.load(Ordering::Relaxed) as f64;
        if limit == 0.0 {
            return 0.0;
        }
        (count / limit) * 100.0
    }

    /// Get current memory usage percentage (0-100)
    pub fn memory_usage_percent(&self) -> f64 {
        let usage = self.memory_usage.load(Ordering::Relaxed) as f64;
        let limit = self.memory_limit.load(Ordering::Relaxed) as f64;
        if limit == 0.0 {
            return 0.0;
        }
        (usage / limit) * 100.0
    }

    /// Get overall resource level (worst of fd and memory)
    pub fn resource_level(&self) -> ResourceLevel {
        let fd_percent = self.fd_usage_percent();
        let mem_percent = self.memory_usage_percent();
        let max_percent = fd_percent.max(mem_percent);

        if max_percent >= 95.0 {
            ResourceLevel::Exhausted
        } else if max_percent >= 90.0 {
            ResourceLevel::Critical
        } else if max_percent >= 80.0 {
            ResourceLevel::Warning
        } else {
            ResourceLevel::Normal
        }
    }

    /// Check if new requests should be accepted
    pub fn should_accept_request(&self) -> bool {
        // Reject requests at 95%+ usage
        self.resource_level() != ResourceLevel::Exhausted
    }

    /// Check if metrics collection is enabled
    pub fn metrics_enabled(&self) -> bool {
        self.metrics_enabled.load(Ordering::Relaxed)
    }

    /// Check resource usage and apply graceful degradation
    fn check_and_apply_degradation(&self) {
        let level = self.resource_level();

        match level {
            ResourceLevel::Normal => {
                // Re-enable metrics if disabled
                let was_disabled = !self.metrics_enabled.swap(true, Ordering::Relaxed);
                if was_disabled {
                    tracing::info!("Resources recovered, re-enabling metrics collection");
                }
            }
            ResourceLevel::Warning => {
                // Just log warning
                tracing::warn!(
                    fd_percent = self.fd_usage_percent(),
                    mem_percent = self.memory_usage_percent(),
                    "Resource usage at warning level (80-90%)"
                );
            }
            ResourceLevel::Critical => {
                // Disable metrics to reduce overhead
                let was_enabled = self.metrics_enabled.swap(false, Ordering::Relaxed);
                if was_enabled {
                    tracing::warn!(
                        fd_percent = self.fd_usage_percent(),
                        mem_percent = self.memory_usage_percent(),
                        "Resource usage critical (90-95%), disabling metrics collection"
                    );
                }
            }
            ResourceLevel::Exhausted => {
                // Ensure metrics are disabled (may have jumped from Normal->Exhausted)
                self.metrics_enabled.store(false, Ordering::Relaxed);

                tracing::error!(
                    fd_percent = self.fd_usage_percent(),
                    mem_percent = self.memory_usage_percent(),
                    "Resource exhaustion (>= 95%), rejecting new requests"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_resource_monitor() {
        let monitor = ResourceMonitor::new(1024, 1024 * 1024 * 1024);

        // Initial state should be normal
        assert_eq!(monitor.resource_level(), ResourceLevel::Normal);
        assert!(monitor.should_accept_request());
        assert!(monitor.metrics_enabled());
    }

    #[test]
    fn test_fd_usage_percent_calculation() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        monitor.update_fd_count(0);
        assert_eq!(monitor.fd_usage_percent(), 0.0);

        monitor.update_fd_count(500);
        assert_eq!(monitor.fd_usage_percent(), 50.0);

        monitor.update_fd_count(1000);
        assert_eq!(monitor.fd_usage_percent(), 100.0);
    }

    #[test]
    fn test_memory_usage_percent_calculation() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        monitor.update_memory_usage(0);
        assert_eq!(monitor.memory_usage_percent(), 0.0);

        monitor.update_memory_usage(512 * 1024);
        assert_eq!(monitor.memory_usage_percent(), 50.0);

        monitor.update_memory_usage(1024 * 1024);
        assert_eq!(monitor.memory_usage_percent(), 100.0);
    }

    #[test]
    fn test_resource_level_normal_below_80_percent() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // 70% usage - should be normal
        monitor.update_fd_count(700);
        assert_eq!(monitor.resource_level(), ResourceLevel::Normal);
        assert!(monitor.should_accept_request());
        assert!(monitor.metrics_enabled());
    }

    #[test]
    fn test_resource_level_warning_at_80_percent() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // 85% usage - should be warning
        monitor.update_fd_count(850);
        assert_eq!(monitor.resource_level(), ResourceLevel::Warning);
        assert!(monitor.should_accept_request());
        assert!(monitor.metrics_enabled()); // Metrics still enabled at warning level
    }

    #[test]
    fn test_resource_level_critical_at_90_percent() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // 92% usage - should be critical
        monitor.update_fd_count(920);
        assert_eq!(monitor.resource_level(), ResourceLevel::Critical);
        assert!(monitor.should_accept_request()); // Still accepting requests
        assert!(!monitor.metrics_enabled()); // Metrics disabled at critical level
    }

    #[test]
    fn test_resource_level_exhausted_at_95_percent() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // 96% usage - should be exhausted
        monitor.update_fd_count(960);
        assert_eq!(monitor.resource_level(), ResourceLevel::Exhausted);
        assert!(!monitor.should_accept_request()); // Rejecting new requests
        assert!(!monitor.metrics_enabled());
    }

    #[test]
    fn test_automatic_recovery_when_resources_freed() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // Go to critical level (92%)
        monitor.update_fd_count(920);
        assert_eq!(monitor.resource_level(), ResourceLevel::Critical);
        assert!(!monitor.metrics_enabled());

        // Free resources back to normal (70%)
        monitor.update_fd_count(700);
        assert_eq!(monitor.resource_level(), ResourceLevel::Normal);
        assert!(monitor.metrics_enabled()); // Metrics re-enabled
        assert!(monitor.should_accept_request());
    }

    #[test]
    fn test_uses_worst_resource_for_level() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // FD at 70%, memory at 92% - should use memory (critical)
        monitor.update_fd_count(700);
        monitor.update_memory_usage((1024 * 1024 * 92) / 100);

        assert_eq!(monitor.resource_level(), ResourceLevel::Critical);
        assert!(!monitor.metrics_enabled());
    }

    #[test]
    fn test_file_descriptor_limit_reached_returns_503() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // At 94% - still accepting
        monitor.update_fd_count(940);
        assert!(monitor.should_accept_request());

        // At 95% - should reject
        monitor.update_fd_count(950);
        assert!(!monitor.should_accept_request());
    }

    #[test]
    fn test_memory_limit_reached_returns_503() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // At 94% - still accepting
        monitor.update_memory_usage((1024 * 1024 * 94) / 100);
        assert!(monitor.should_accept_request());

        // At 96% - should reject
        monitor.update_memory_usage((1024 * 1024 * 96) / 100);
        assert!(!monitor.should_accept_request());
    }

    #[test]
    fn test_graceful_degradation_disables_metrics_at_90_percent() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // Start normal
        assert!(monitor.metrics_enabled());

        // Go to 91% - metrics should disable
        monitor.update_fd_count(910);
        assert!(!monitor.metrics_enabled());
    }

    #[test]
    fn test_metrics_re_enabled_when_dropping_below_80_percent() {
        let monitor = ResourceMonitor::new(1000, 1024 * 1024);

        // Go to critical (metrics disabled)
        monitor.update_fd_count(910);
        assert!(!monitor.metrics_enabled());

        // Drop to 75%
        monitor.update_fd_count(750);
        assert!(monitor.metrics_enabled());
    }

    #[test]
    fn test_system_fd_limit_detected() {
        let fd_limit = detect_fd_limit();

        // Should return a reasonable value (at least 256, typically 1024+)
        assert!(
            fd_limit >= 256,
            "FD limit should be at least 256, got {}",
            fd_limit
        );

        // Should be less than a million (sanity check)
        assert!(
            fd_limit < 10_000_000,
            "FD limit seems unreasonably high: {}",
            fd_limit
        );
    }

    #[test]
    fn test_system_memory_limit_detected() {
        let mem_limit = detect_memory_limit();

        // Should return at least 512MB
        assert!(
            mem_limit >= 512 * 1024 * 1024,
            "Memory limit should be at least 512MB, got {} bytes",
            mem_limit
        );

        // Should be less than 1TB (sanity check)
        assert!(
            mem_limit < 1024u64 * 1024 * 1024 * 1024,
            "Memory limit seems unreasonably high: {} bytes",
            mem_limit
        );
    }

    #[test]
    fn test_new_with_auto_detection() {
        let monitor = ResourceMonitor::new_auto_detect();

        // Verify it created successfully
        assert_eq!(monitor.resource_level(), ResourceLevel::Normal);
        assert!(monitor.should_accept_request());
        assert!(monitor.metrics_enabled());

        // Verify limits are reasonable (from auto-detection)
        let fd_percent = monitor.fd_usage_percent();
        let mem_percent = monitor.memory_usage_percent();

        // Initial usage should be low
        assert!(
            fd_percent < 50.0,
            "Initial FD usage should be low, got {}%",
            fd_percent
        );
        assert!(
            mem_percent < 50.0,
            "Initial memory usage should be low, got {}%",
            mem_percent
        );
    }
}
