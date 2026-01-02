//! Template variable substitution for text watermarks.
//!
//! This module provides template resolution for text watermarks, replacing
//! variables like `{{jwt.sub}}`, `{{ip}}`, `{{date}}` with actual values
//! from the request context.
//!
//! # Supported Variables
//!
//! - `{{ip}}` - Client IP address (X-Forwarded-For aware)
//! - `{{jwt.sub}}`, `{{jwt.iss}}`, `{{jwt.<claim>}}` - JWT claims
//! - `{{header.X-Name}}` - Request header values
//! - `{{path}}` - Request path
//! - `{{bucket}}` - Bucket name
//! - `{{date}}` - Current date (YYYY-MM-DD)
//! - `{{datetime}}` - ISO 8601 datetime
//! - `{{timestamp}}` - Unix timestamp
//!
//! # Example
//!
//! ```ignore
//! use yatagarasu::watermark::template::{TemplateContext, resolve_template};
//!
//! let mut context = TemplateContext::new();
//! context.set_ip("192.168.1.100");
//! context.set_jwt_claim("sub", "user123");
//!
//! let result = resolve_template("User: {{jwt.sub}} from {{ip}}", &context);
//! assert_eq!(result, "User: user123 from 192.168.1.100");
//! ```

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Regex pattern for matching template variables: {{variable}}
static TEMPLATE_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Gets the compiled template pattern regex.
///
/// # Safety
/// The regex pattern `r"\{\{([^}]+)\}\}"` is a compile-time constant that is
/// guaranteed to be valid. The `.expect()` can never panic in practice.
/// This is verified by the `test_template_regex_is_valid` test.
fn get_template_pattern() -> &'static Regex {
    TEMPLATE_PATTERN.get_or_init(|| {
        // SAFETY: This is a compile-time constant regex pattern.
        // If this panics, it's a developer error that should fail at test time.
        Regex::new(r"\{\{([^}]+)\}\}").expect("Invalid template regex - this is a compile-time bug")
    })
}

/// Context containing all available values for template substitution.
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    /// Client IP address
    ip: Option<String>,
    /// JWT claims (key -> value)
    jwt_claims: HashMap<String, String>,
    /// Request headers (name -> value)
    headers: HashMap<String, String>,
    /// Request path
    path: Option<String>,
    /// Bucket name
    bucket: Option<String>,
    /// Custom timestamp for testing (if None, uses current time)
    #[cfg(test)]
    test_timestamp: Option<i64>,
}

impl TemplateContext {
    /// Creates a new empty template context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the client IP address.
    pub fn set_ip(&mut self, ip: impl Into<String>) {
        self.ip = Some(ip.into());
    }

    /// Sets a JWT claim value.
    pub fn set_jwt_claim(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.jwt_claims.insert(key.into(), value.into());
    }

    /// Sets a request header value.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    /// Sets the request path.
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.path = Some(path.into());
    }

    /// Sets the bucket name.
    pub fn set_bucket(&mut self, bucket: impl Into<String>) {
        self.bucket = Some(bucket.into());
    }

    /// Gets the IP address.
    pub fn ip(&self) -> Option<&str> {
        self.ip.as_deref()
    }

    /// Gets a JWT claim by key.
    pub fn jwt_claim(&self, key: &str) -> Option<&str> {
        self.jwt_claims.get(key).map(|s| s.as_str())
    }

    /// Gets a header by name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Gets the request path.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Gets the bucket name.
    pub fn bucket(&self) -> Option<&str> {
        self.bucket.as_deref()
    }

    /// Gets the current timestamp (or test timestamp if set).
    #[cfg(test)]
    fn current_timestamp(&self) -> i64 {
        self.test_timestamp
            .unwrap_or_else(|| chrono::Utc::now().timestamp())
    }

    /// Gets the current timestamp.
    #[cfg(not(test))]
    fn current_timestamp(&self) -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// Sets a test timestamp for deterministic testing.
    #[cfg(test)]
    pub fn set_test_timestamp(&mut self, timestamp: i64) {
        self.test_timestamp = Some(timestamp);
    }
}

/// Resolves all template variables in the given text.
///
/// Variables are specified using `{{variable}}` syntax. Unknown variables
/// or variables with missing values are replaced with an empty string.
///
/// # Arguments
///
/// * `template` - The template string containing variables
/// * `context` - The context containing variable values
///
/// # Returns
///
/// The resolved string with all variables replaced.
pub fn resolve_template(template: &str, context: &TemplateContext) -> String {
    get_template_pattern()
        .replace_all(template, |caps: &regex::Captures| {
            let var_name = &caps[1];
            resolve_variable(var_name, context)
        })
        .into_owned()
}

/// Resolves a single variable name to its value.
fn resolve_variable(var_name: &str, context: &TemplateContext) -> String {
    // Handle JWT claims: jwt.sub, jwt.iss, jwt.<custom>
    if let Some(claim_name) = var_name.strip_prefix("jwt.") {
        return context.jwt_claim(claim_name).unwrap_or("").to_string();
    }

    // Handle headers: header.X-Name
    if let Some(header_name) = var_name.strip_prefix("header.") {
        return context.header(header_name).unwrap_or("").to_string();
    }

    // Handle other variables
    match var_name {
        "ip" => context.ip().unwrap_or("").to_string(),
        "path" => context.path().unwrap_or("").to_string(),
        "bucket" => context.bucket().unwrap_or("").to_string(),
        "date" => format_date(context.current_timestamp()),
        "datetime" => format_datetime(context.current_timestamp()),
        "timestamp" => context.current_timestamp().to_string(),
        _ => String::new(), // Unknown variable -> empty string
    }
}

/// Formats a timestamp as a date string (YYYY-MM-DD).
fn format_date(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// Formats a timestamp as an ISO 8601 datetime string.
fn format_datetime(timestamp: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_default()
}

/// Generates a hash of the resolved template for cache key purposes.
///
/// This hash can be used to create unique cache keys that account for
/// the dynamic content of watermarks.
pub fn template_hash(template: &str, context: &TemplateContext) -> u64 {
    use std::hash::{Hash, Hasher};
    let resolved = resolve_template(template, context);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    resolved.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the template regex pattern is valid.
    /// This test ensures the `.expect()` in `get_template_pattern()` will never panic.
    #[test]
    fn test_template_regex_is_valid() {
        // This test verifies the compile-time constant regex pattern is valid.
        // If this test passes, the .expect() in get_template_pattern() is safe.
        let pattern = get_template_pattern();
        assert!(pattern.is_match("{{variable}}"));
        assert!(pattern.is_match("{{jwt.sub}}"));
        assert!(!pattern.is_match("plain text"));
    }

    // Test: resolve_template replaces {{ip}}
    #[test]
    fn test_resolve_template_replaces_ip() {
        let mut context = TemplateContext::new();
        context.set_ip("192.168.1.100");

        let result = resolve_template("Client IP: {{ip}}", &context);
        assert_eq!(result, "Client IP: 192.168.1.100");
    }

    #[test]
    fn test_resolve_template_replaces_ip_multiple_occurrences() {
        let mut context = TemplateContext::new();
        context.set_ip("10.0.0.1");

        let result = resolve_template("From {{ip}} to {{ip}}", &context);
        assert_eq!(result, "From 10.0.0.1 to 10.0.0.1");
    }

    // Test: resolve_template replaces {{jwt.sub}}
    #[test]
    fn test_resolve_template_replaces_jwt_sub() {
        let mut context = TemplateContext::new();
        context.set_jwt_claim("sub", "user123");

        let result = resolve_template("User: {{jwt.sub}}", &context);
        assert_eq!(result, "User: user123");
    }

    #[test]
    fn test_resolve_template_replaces_jwt_iss() {
        let mut context = TemplateContext::new();
        context.set_jwt_claim("iss", "auth.example.com");

        let result = resolve_template("Issuer: {{jwt.iss}}", &context);
        assert_eq!(result, "Issuer: auth.example.com");
    }

    // Test: resolve_template replaces {{jwt.custom_claim}}
    #[test]
    fn test_resolve_template_replaces_jwt_custom_claim() {
        let mut context = TemplateContext::new();
        context.set_jwt_claim("org", "Acme Inc");
        context.set_jwt_claim("department", "Engineering");

        let result = resolve_template("{{jwt.org}} - {{jwt.department}}", &context);
        assert_eq!(result, "Acme Inc - Engineering");
    }

    // Test: resolve_template replaces {{header.X-Name}}
    #[test]
    fn test_resolve_template_replaces_header() {
        let mut context = TemplateContext::new();
        context.set_header("X-User-Id", "12345");

        let result = resolve_template("User ID: {{header.X-User-Id}}", &context);
        assert_eq!(result, "User ID: 12345");
    }

    #[test]
    fn test_resolve_template_replaces_multiple_headers() {
        let mut context = TemplateContext::new();
        context.set_header("X-Request-Id", "req-abc");
        context.set_header("X-Trace-Id", "trace-xyz");

        let result = resolve_template(
            "Request: {{header.X-Request-Id}}, Trace: {{header.X-Trace-Id}}",
            &context,
        );
        assert_eq!(result, "Request: req-abc, Trace: trace-xyz");
    }

    // Test: resolve_template handles missing values gracefully
    #[test]
    fn test_resolve_template_missing_ip_returns_empty() {
        let context = TemplateContext::new();

        let result = resolve_template("IP: {{ip}}", &context);
        assert_eq!(result, "IP: ");
    }

    #[test]
    fn test_resolve_template_missing_jwt_claim_returns_empty() {
        let context = TemplateContext::new();

        let result = resolve_template("User: {{jwt.sub}}", &context);
        assert_eq!(result, "User: ");
    }

    #[test]
    fn test_resolve_template_missing_header_returns_empty() {
        let context = TemplateContext::new();

        let result = resolve_template("Header: {{header.X-Missing}}", &context);
        assert_eq!(result, "Header: ");
    }

    #[test]
    fn test_resolve_template_unknown_variable_returns_empty() {
        let context = TemplateContext::new();

        let result = resolve_template("Unknown: {{unknown_var}}", &context);
        assert_eq!(result, "Unknown: ");
    }

    // Test: resolve_template handles {{date}}, {{datetime}}, {{timestamp}}
    #[test]
    fn test_resolve_template_replaces_date() {
        let mut context = TemplateContext::new();
        // 2025-12-24 00:00:00 UTC
        context.set_test_timestamp(1766534400);

        let result = resolve_template("Date: {{date}}", &context);
        assert_eq!(result, "Date: 2025-12-24");
    }

    #[test]
    fn test_resolve_template_replaces_datetime() {
        let mut context = TemplateContext::new();
        // 2025-12-24 10:30:00 UTC
        context.set_test_timestamp(1766572200);

        let result = resolve_template("DateTime: {{datetime}}", &context);
        assert_eq!(result, "DateTime: 2025-12-24T10:30:00Z");
    }

    #[test]
    fn test_resolve_template_replaces_timestamp() {
        let mut context = TemplateContext::new();
        context.set_test_timestamp(1766534400);

        let result = resolve_template("Timestamp: {{timestamp}}", &context);
        assert_eq!(result, "Timestamp: 1766534400");
    }

    // Test: resolve_template handles path and bucket
    #[test]
    fn test_resolve_template_replaces_path() {
        let mut context = TemplateContext::new();
        context.set_path("/products/item.jpg");

        let result = resolve_template("Path: {{path}}", &context);
        assert_eq!(result, "Path: /products/item.jpg");
    }

    #[test]
    fn test_resolve_template_replaces_bucket() {
        let mut context = TemplateContext::new();
        context.set_bucket("products");

        let result = resolve_template("Bucket: {{bucket}}", &context);
        assert_eq!(result, "Bucket: products");
    }

    // Test: Template hash generation for cache key
    #[test]
    fn test_template_hash_same_content_same_hash() {
        let mut context = TemplateContext::new();
        context.set_jwt_claim("sub", "user123");

        let hash1 = template_hash("User: {{jwt.sub}}", &context);
        let hash2 = template_hash("User: {{jwt.sub}}", &context);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_template_hash_different_content_different_hash() {
        let mut context1 = TemplateContext::new();
        context1.set_jwt_claim("sub", "user123");

        let mut context2 = TemplateContext::new();
        context2.set_jwt_claim("sub", "user456");

        let hash1 = template_hash("User: {{jwt.sub}}", &context1);
        let hash2 = template_hash("User: {{jwt.sub}}", &context2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_template_hash_static_template_consistent() {
        let context = TemplateContext::new();

        let hash1 = template_hash("CONFIDENTIAL", &context);
        let hash2 = template_hash("CONFIDENTIAL", &context);

        assert_eq!(hash1, hash2);
    }

    // Test: Complex template with multiple variable types
    #[test]
    fn test_resolve_template_complex_template() {
        let mut context = TemplateContext::new();
        context.set_ip("192.168.1.100");
        context.set_jwt_claim("sub", "alice");
        context.set_jwt_claim("org", "Acme");
        context.set_header("X-Request-Id", "req-123");
        context.set_path("/products/image.jpg");
        context.set_bucket("products");
        context.set_test_timestamp(1766534400); // 2025-12-24 00:00:00 UTC

        let template = "{{jwt.sub}}@{{jwt.org}} from {{ip}} - {{path}} in {{bucket}} on {{date}}";
        let result = resolve_template(template, &context);

        assert_eq!(
            result,
            "alice@Acme from 192.168.1.100 - /products/image.jpg in products on 2025-12-24"
        );
    }

    // Test: Template with no variables returns as-is
    #[test]
    fn test_resolve_template_no_variables() {
        let context = TemplateContext::new();

        let result = resolve_template("Static text without variables", &context);
        assert_eq!(result, "Static text without variables");
    }

    // Test: Empty template returns empty string
    #[test]
    fn test_resolve_template_empty() {
        let context = TemplateContext::new();

        let result = resolve_template("", &context);
        assert_eq!(result, "");
    }

    // Test: Template with adjacent variables
    #[test]
    fn test_resolve_template_adjacent_variables() {
        let mut context = TemplateContext::new();
        context.set_jwt_claim("first", "Hello");
        context.set_jwt_claim("second", "World");

        let result = resolve_template("{{jwt.first}}{{jwt.second}}", &context);
        assert_eq!(result, "HelloWorld");
    }

    // Test: Template with special characters in values
    #[test]
    fn test_resolve_template_special_chars_in_values() {
        let mut context = TemplateContext::new();
        context.set_jwt_claim("name", "O'Brien & Co.");
        context.set_ip("::1");

        let result = resolve_template("Name: {{jwt.name}}, IP: {{ip}}", &context);
        assert_eq!(result, "Name: O'Brien & Co., IP: ::1");
    }
}
