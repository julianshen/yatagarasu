// Constants module - centralized default values for configuration
//
// This module defines all default values used throughout the codebase.
// Using constants instead of magic numbers improves maintainability
// and makes it easier to understand and modify defaults.

// =============================================================================
// Server defaults
// =============================================================================

/// Default request timeout in seconds
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default maximum concurrent requests
pub const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 1000;

/// Default number of worker threads
pub const DEFAULT_THREADS: usize = 4;

// =============================================================================
// S3 defaults
// =============================================================================

/// Default S3 operation timeout in seconds
pub const DEFAULT_S3_TIMEOUT_SECS: u64 = 20;

/// Default connection pool size per S3 bucket
pub const DEFAULT_CONNECTION_POOL_SIZE: usize = 50;

// =============================================================================
// Security defaults
// =============================================================================

/// Default maximum request body size (10 MB)
pub const DEFAULT_MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Default maximum header size (64 KB)
pub const DEFAULT_MAX_HEADER_SIZE: usize = 64 * 1024;

/// Default maximum URI length (8 KB)
pub const DEFAULT_MAX_URI_LENGTH: usize = 8192;

// =============================================================================
// Cache defaults
// =============================================================================

/// Default maximum item size in megabytes
pub const DEFAULT_MAX_ITEM_SIZE_MB: u64 = 10;

/// Default maximum cache size in megabytes
pub const DEFAULT_MAX_CACHE_SIZE_MB: u64 = 1024;

/// Default TTL in seconds
pub const DEFAULT_TTL_SECONDS: u64 = 3600;

// =============================================================================
// Circuit breaker defaults
// =============================================================================

/// Default number of failures before circuit opens
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;

/// Default number of successes to close circuit
pub const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;

/// Default circuit breaker timeout in seconds
pub const DEFAULT_CB_TIMEOUT_SECS: u64 = 60;

/// Default maximum requests allowed in half-open state
pub const DEFAULT_HALF_OPEN_MAX_REQUESTS: u32 = 3;

// =============================================================================
// Retry defaults
// =============================================================================

/// Default maximum retry attempts
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;

/// Default initial backoff in milliseconds
pub const DEFAULT_INITIAL_BACKOFF_MS: u64 = 100;

/// Default maximum backoff in milliseconds
pub const DEFAULT_MAX_BACKOFF_MS: u64 = 1000;

// =============================================================================
// Audit defaults
// =============================================================================

/// Default maximum audit file size in megabytes
pub const DEFAULT_MAX_FILE_SIZE_MB: u64 = 50;

/// Default maximum backup audit files
pub const DEFAULT_MAX_BACKUP_FILES: u32 = 5;

/// Default audit buffer size (1 MB)
pub const DEFAULT_AUDIT_BUFFER_SIZE: usize = 1024 * 1024;

/// Default export interval in seconds
pub const DEFAULT_EXPORT_INTERVAL_SECS: u64 = 60;

// =============================================================================
// OPA defaults
// =============================================================================

/// Default OPA request timeout in milliseconds
pub const DEFAULT_OPA_TIMEOUT_MS: u64 = 100;

/// Default OPA cache TTL in seconds
pub const DEFAULT_OPA_CACHE_TTL_SECS: u64 = 60;

// =============================================================================
// OpenFGA defaults
// =============================================================================

/// Default OpenFGA request timeout in milliseconds
pub const DEFAULT_OPENFGA_TIMEOUT_MS: u64 = 100;

/// Default OpenFGA cache TTL in seconds
pub const DEFAULT_OPENFGA_CACHE_TTL_SECS: u64 = 60;
