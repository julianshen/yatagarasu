// Request/response logging module
// Phase 34: Enhanced Observability

use crate::observability::config::RequestLoggingConfig;
use regex::Regex;
use std::collections::HashSet;

/// Request/response logger with filtering and redaction
#[derive(Debug, Clone)]
pub struct RequestLogger {
    config: RequestLoggingConfig,
    include_patterns: Vec<Regex>,
    exclude_patterns: Vec<Regex>,
    status_codes: HashSet<u16>,
    redact_headers_lower: HashSet<String>,
}

impl RequestLogger {
    /// Create a new request logger
    pub fn new(config: RequestLoggingConfig) -> Self {
        let include_patterns = config
            .include_paths
            .iter()
            .filter_map(|p| Self::glob_to_regex(p).ok())
            .collect();

        let exclude_patterns = config
            .exclude_paths
            .iter()
            .filter_map(|p| Self::glob_to_regex(p).ok())
            .collect();

        let status_codes: HashSet<u16> = config.status_codes.iter().copied().collect();

        let redact_headers_lower: HashSet<String> = config
            .redact_headers
            .iter()
            .map(|h| h.to_lowercase())
            .collect();

        Self {
            config,
            include_patterns,
            exclude_patterns,
            status_codes,
            redact_headers_lower,
        }
    }

    /// Convert glob pattern to regex
    fn glob_to_regex(pattern: &str) -> Result<Regex, regex::Error> {
        let regex_pattern = pattern
            .replace('.', r"\.")
            .replace('*', ".*")
            .replace('?', ".");
        Regex::new(&format!("^{}$", regex_pattern))
    }

    /// Check if request logging is enabled
    pub fn should_log_requests(&self) -> bool {
        self.config.log_requests
    }

    /// Check if response logging is enabled
    pub fn should_log_responses(&self) -> bool {
        self.config.log_responses
    }

    /// Check if a path should be logged based on include/exclude patterns
    pub fn should_log_path(&self, path: &str) -> bool {
        // If exclude patterns exist and path matches any, don't log
        if !self.exclude_patterns.is_empty() {
            for pattern in &self.exclude_patterns {
                if pattern.is_match(path) {
                    return false;
                }
            }
        }

        // If no include patterns, log everything not excluded
        if self.include_patterns.is_empty() {
            return true;
        }

        // If include patterns exist, path must match at least one
        for pattern in &self.include_patterns {
            if pattern.is_match(path) {
                return true;
            }
        }

        false
    }

    /// Check if a status code should be logged
    pub fn should_log_status(&self, status: u16) -> bool {
        // If no status codes configured, log all
        if self.status_codes.is_empty() {
            return true;
        }

        self.status_codes.contains(&status)
    }

    /// Check if a header should be redacted
    pub fn should_redact_header(&self, header_name: &str) -> bool {
        self.redact_headers_lower
            .contains(&header_name.to_lowercase())
    }

    /// Redact a header value if needed
    pub fn redact_header_value(&self, header_name: &str, value: &str) -> String {
        if self.should_redact_header(header_name) {
            "[REDACTED]".to_string()
        } else {
            value.to_string()
        }
    }

    /// Truncate body to max size
    pub fn truncate_body(&self, body: &[u8]) -> String {
        let max_size = self.config.max_body_size;
        if body.len() <= max_size {
            String::from_utf8_lossy(body).to_string()
        } else {
            let truncated = String::from_utf8_lossy(&body[..max_size]);
            format!("{}... [truncated, {} bytes total]", truncated, body.len())
        }
    }

    /// Log a request
    pub fn log_request(
        &self,
        method: &str,
        path: &str,
        headers: &[(String, String)],
        body: Option<&[u8]>,
        correlation_id: &str,
    ) {
        if !self.config.log_requests {
            return;
        }

        if !self.should_log_path(path) {
            return;
        }

        // Build headers string with redaction
        let headers_str: Vec<String> = headers
            .iter()
            .map(|(name, value)| {
                let redacted_value = self.redact_header_value(name, value);
                format!("{}: {}", name, redacted_value)
            })
            .collect();

        // Build body string if enabled
        let body_str = if self.config.log_request_body {
            body.map(|b| self.truncate_body(b))
        } else {
            None
        };

        tracing::info!(
            correlation_id = %correlation_id,
            method = %method,
            path = %path,
            headers = ?headers_str,
            body = ?body_str,
            "Incoming request"
        );
    }

    /// Log a response
    pub fn log_response(
        &self,
        status: u16,
        path: &str,
        headers: &[(String, String)],
        body: Option<&[u8]>,
        latency_ms: u64,
        correlation_id: &str,
    ) {
        if !self.config.log_responses {
            return;
        }

        if !self.should_log_path(path) {
            return;
        }

        if !self.should_log_status(status) {
            return;
        }

        // Build headers string with redaction
        let headers_str: Vec<String> = headers
            .iter()
            .map(|(name, value)| {
                let redacted_value = self.redact_header_value(name, value);
                format!("{}: {}", name, redacted_value)
            })
            .collect();

        // Build body string if enabled
        let body_str = if self.config.log_response_body {
            body.map(|b| self.truncate_body(b))
        } else {
            None
        };

        tracing::info!(
            correlation_id = %correlation_id,
            status = %status,
            path = %path,
            latency_ms = %latency_ms,
            headers = ?headers_str,
            body = ?body_str,
            "Outgoing response"
        );
    }
}

impl Default for RequestLogger {
    fn default() -> Self {
        Self::new(RequestLoggingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_logger_disabled_by_default() {
        let logger = RequestLogger::default();
        assert!(!logger.should_log_requests());
        assert!(!logger.should_log_responses());
    }

    #[test]
    fn test_request_logger_enabled() {
        let config = RequestLoggingConfig {
            log_requests: true,
            log_responses: true,
            ..Default::default()
        };
        let logger = RequestLogger::new(config);
        assert!(logger.should_log_requests());
        assert!(logger.should_log_responses());
    }

    #[test]
    fn test_path_filtering_no_patterns() {
        let config = RequestLoggingConfig {
            log_requests: true,
            include_paths: vec![],
            exclude_paths: vec![],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Should log everything when no patterns
        assert!(logger.should_log_path("/any/path"));
        assert!(logger.should_log_path("/health"));
        assert!(logger.should_log_path("/api/users"));
    }

    #[test]
    fn test_path_filtering_exclude_patterns() {
        let config = RequestLoggingConfig {
            log_requests: true,
            include_paths: vec![],
            exclude_paths: vec!["/health".to_string(), "/metrics".to_string()],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        assert!(!logger.should_log_path("/health"));
        assert!(!logger.should_log_path("/metrics"));
        assert!(logger.should_log_path("/api/users"));
        assert!(logger.should_log_path("/any/other/path"));
    }

    #[test]
    fn test_path_filtering_include_patterns() {
        let config = RequestLoggingConfig {
            log_requests: true,
            include_paths: vec!["/api/*".to_string(), "/public/*".to_string()],
            exclude_paths: vec![],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        assert!(logger.should_log_path("/api/users"));
        assert!(logger.should_log_path("/api/anything"));
        assert!(logger.should_log_path("/public/images/logo.png"));
        assert!(!logger.should_log_path("/health"));
        assert!(!logger.should_log_path("/other/path"));
    }

    #[test]
    fn test_path_filtering_combined() {
        let config = RequestLoggingConfig {
            log_requests: true,
            include_paths: vec!["/api/*".to_string()],
            exclude_paths: vec!["/api/internal/*".to_string()],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        assert!(logger.should_log_path("/api/users"));
        assert!(!logger.should_log_path("/api/internal/debug"));
        assert!(!logger.should_log_path("/health")); // Not in include
    }

    #[test]
    fn test_status_code_filtering_all() {
        let config = RequestLoggingConfig {
            log_responses: true,
            status_codes: vec![],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Empty status_codes means log all
        assert!(logger.should_log_status(200));
        assert!(logger.should_log_status(400));
        assert!(logger.should_log_status(500));
    }

    #[test]
    fn test_status_code_filtering_specific() {
        let config = RequestLoggingConfig {
            log_responses: true,
            status_codes: vec![400, 500, 502, 503],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        assert!(!logger.should_log_status(200));
        assert!(logger.should_log_status(400));
        assert!(logger.should_log_status(500));
        assert!(logger.should_log_status(502));
        assert!(!logger.should_log_status(201));
    }

    #[test]
    fn test_header_redaction() {
        let config = RequestLoggingConfig {
            redact_headers: vec!["Authorization".to_string(), "X-Api-Key".to_string()],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        assert!(logger.should_redact_header("Authorization"));
        assert!(logger.should_redact_header("authorization")); // Case insensitive
        assert!(logger.should_redact_header("AUTHORIZATION"));
        assert!(logger.should_redact_header("X-Api-Key"));
        assert!(logger.should_redact_header("x-api-key"));
        assert!(!logger.should_redact_header("Content-Type"));
    }

    #[test]
    fn test_redact_header_value() {
        let config = RequestLoggingConfig {
            redact_headers: vec!["Authorization".to_string()],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        assert_eq!(
            logger.redact_header_value("Authorization", "Bearer secret-token"),
            "[REDACTED]"
        );
        assert_eq!(
            logger.redact_header_value("Content-Type", "application/json"),
            "application/json"
        );
    }

    #[test]
    fn test_body_truncation() {
        let config = RequestLoggingConfig {
            max_body_size: 10,
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Short body - no truncation
        let short = b"hello";
        assert_eq!(logger.truncate_body(short), "hello");

        // Exact max size
        let exact = b"1234567890";
        assert_eq!(logger.truncate_body(exact), "1234567890");

        // Long body - truncated
        let long = b"this is a very long body that should be truncated";
        let truncated = logger.truncate_body(long);
        assert!(truncated.contains("this is a "));
        assert!(truncated.contains("[truncated"));
        assert!(truncated.contains("49 bytes total"));
    }

    #[test]
    fn test_glob_to_regex() {
        // Basic path
        let regex = RequestLogger::glob_to_regex("/health").unwrap();
        assert!(regex.is_match("/health"));
        assert!(!regex.is_match("/health/check"));

        // Wildcard
        let regex = RequestLogger::glob_to_regex("/api/*").unwrap();
        assert!(regex.is_match("/api/users"));
        assert!(regex.is_match("/api/anything"));
        assert!(!regex.is_match("/other/path"));

        // Question mark
        let regex = RequestLogger::glob_to_regex("/api/v?").unwrap();
        assert!(regex.is_match("/api/v1"));
        assert!(regex.is_match("/api/v2"));
        assert!(!regex.is_match("/api/v10"));
    }

    #[test]
    fn test_log_request_disabled() {
        let config = RequestLoggingConfig {
            log_requests: false,
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Should not panic even when disabled
        logger.log_request(
            "GET",
            "/test",
            &[("Content-Type".to_string(), "application/json".to_string())],
            None,
            "req-123",
        );
    }

    #[test]
    fn test_log_request_enabled() {
        let config = RequestLoggingConfig {
            log_requests: true,
            log_request_body: true,
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Should not panic
        logger.log_request(
            "POST",
            "/api/users",
            &[
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string()),
            ],
            Some(b"{\"name\": \"test\"}"),
            "req-456",
        );
    }

    #[test]
    fn test_log_response_disabled() {
        let config = RequestLoggingConfig {
            log_responses: false,
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Should not panic
        logger.log_response(200, "/test", &[], None, 100, "req-123");
    }

    #[test]
    fn test_log_response_enabled() {
        let config = RequestLoggingConfig {
            log_responses: true,
            log_response_body: true,
            status_codes: vec![200, 400, 500],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // Should log 200
        logger.log_response(
            200,
            "/api/users",
            &[("Content-Type".to_string(), "application/json".to_string())],
            Some(b"[{\"id\": 1}]"),
            50,
            "req-789",
        );

        // Should log 400
        logger.log_response(
            400,
            "/api/users",
            &[],
            Some(b"{\"error\": \"bad request\"}"),
            10,
            "req-790",
        );
    }

    #[test]
    fn test_log_response_filtered_by_status() {
        let config = RequestLoggingConfig {
            log_responses: true,
            status_codes: vec![400, 500],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // This should not log because 200 is not in the filter
        // We can't easily verify no logging, but it should not panic
        logger.log_response(200, "/api/users", &[], None, 50, "req-123");
    }

    #[test]
    fn test_log_response_filtered_by_path() {
        let config = RequestLoggingConfig {
            log_responses: true,
            exclude_paths: vec!["/health".to_string()],
            ..Default::default()
        };
        let logger = RequestLogger::new(config);

        // This should not log because path is excluded
        logger.log_response(200, "/health", &[], None, 5, "req-123");
    }
}
