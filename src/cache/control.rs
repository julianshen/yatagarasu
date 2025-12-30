//! Cache-Control header parsing for RFC 7234 compliance.
//!
//! Parses Cache-Control headers from S3 responses to determine:
//! - Whether a response should be cached
//! - How long the cached response should be considered fresh
//! - Whether revalidation is required before serving stale content
//!
//! # RFC 7234 Compliance
//!
//! This module implements the caching semantics defined in RFC 7234:
//! - `no-store`: Response MUST NOT be stored in any cache
//! - `no-cache`: Response can be stored but MUST be revalidated before use
//! - `private`: Response MUST NOT be stored in shared caches (like this proxy)
//! - `max-age`: Response freshness lifetime in seconds
//! - `s-maxage`: Shared cache specific max-age (overrides max-age)
//! - `must-revalidate`: Stale responses MUST be revalidated before use
//!
//! # Example
//!
//! ```rust
//! use yatagarasu::cache::CacheControl;
//!
//! let cc = CacheControl::parse("max-age=3600, must-revalidate");
//! assert_eq!(cc.max_age, Some(std::time::Duration::from_secs(3600)));
//! assert!(cc.must_revalidate);
//! assert!(cc.is_cacheable_by_shared_cache());
//! ```

use std::time::Duration;

/// Parsed Cache-Control header directives.
///
/// Represents the caching directives from an HTTP Cache-Control header,
/// parsed into a structured format for making caching decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheControl {
    /// Maximum age for cached response (max-age directive)
    pub max_age: Option<Duration>,

    /// Shared cache maximum age (s-maxage directive)
    /// Takes precedence over max-age for shared caches like this proxy
    pub s_maxage: Option<Duration>,

    /// Response must not be stored in any cache (no-store directive)
    pub no_store: bool,

    /// Response must be revalidated before use (no-cache directive)
    pub no_cache: bool,

    /// Response is intended for a single user and must not be stored
    /// by shared caches (private directive)
    pub private: bool,

    /// Cache must revalidate stale responses before use (must-revalidate directive)
    pub must_revalidate: bool,

    /// Proxy must revalidate stale responses (proxy-revalidate directive)
    pub proxy_revalidate: bool,

    /// Response may be served stale if origin is unavailable (stale-if-error directive)
    pub stale_if_error: Option<Duration>,

    /// Response may be served stale while revalidating (stale-while-revalidate directive)
    pub stale_while_revalidate: Option<Duration>,

    /// Cache can transform the response (e.g., compress) unless no-transform is set
    pub no_transform: bool,

    /// Response can be cached by any cache (public directive)
    pub public: bool,

    /// Indicates immutable content that never changes (immutable directive)
    pub immutable: bool,
}

impl CacheControl {
    /// Parse a Cache-Control header value into structured directives.
    ///
    /// Handles comma-separated directives, with optional values for directives
    /// like `max-age=3600`. Unknown directives are ignored.
    ///
    /// # Arguments
    /// * `header_value` - The value of the Cache-Control header
    ///
    /// # Returns
    /// A `CacheControl` struct with parsed directives
    ///
    /// # Example
    /// ```rust
    /// use yatagarasu::cache::CacheControl;
    ///
    /// let cc = CacheControl::parse("max-age=3600, no-cache, must-revalidate");
    /// assert_eq!(cc.max_age, Some(std::time::Duration::from_secs(3600)));
    /// assert!(cc.no_cache);
    /// assert!(cc.must_revalidate);
    /// ```
    pub fn parse(header_value: &str) -> Self {
        let mut result = Self::default();

        // Split on commas and process each directive
        for directive in header_value.split(',') {
            let directive = directive.trim().to_lowercase();
            if directive.is_empty() {
                continue;
            }

            // Check for directives with values (e.g., max-age=3600)
            if let Some((name, value)) = directive.split_once('=') {
                let name = name.trim();
                let value = value.trim().trim_matches('"');

                match name {
                    "max-age" => {
                        if let Ok(secs) = value.parse::<u64>() {
                            result.max_age = Some(Duration::from_secs(secs));
                        }
                    }
                    "s-maxage" => {
                        if let Ok(secs) = value.parse::<u64>() {
                            result.s_maxage = Some(Duration::from_secs(secs));
                        }
                    }
                    "stale-while-revalidate" => {
                        if let Ok(secs) = value.parse::<u64>() {
                            result.stale_while_revalidate = Some(Duration::from_secs(secs));
                        }
                    }
                    "stale-if-error" => {
                        if let Ok(secs) = value.parse::<u64>() {
                            result.stale_if_error = Some(Duration::from_secs(secs));
                        }
                    }
                    _ => {
                        // Unknown directive with value, ignore
                    }
                }
            } else {
                // Boolean directives
                match directive.as_str() {
                    "no-store" => result.no_store = true,
                    "no-cache" => result.no_cache = true,
                    "private" => result.private = true,
                    "public" => result.public = true,
                    "must-revalidate" => result.must_revalidate = true,
                    "proxy-revalidate" => result.proxy_revalidate = true,
                    "no-transform" => result.no_transform = true,
                    "immutable" => result.immutable = true,
                    _ => {
                        // Unknown directive, ignore
                    }
                }
            }
        }

        result
    }

    /// Check if this response can be cached by a shared cache (like this proxy).
    ///
    /// Returns `false` if any of these conditions are met:
    /// - `no-store` directive is present
    /// - `private` directive is present (response is for single user only)
    ///
    /// Note: `no-cache` does NOT prevent storage, it only requires revalidation.
    ///
    /// # Example
    /// ```rust
    /// use yatagarasu::cache::CacheControl;
    ///
    /// let cc = CacheControl::parse("private, max-age=3600");
    /// assert!(!cc.is_cacheable_by_shared_cache());
    ///
    /// let cc = CacheControl::parse("public, max-age=3600");
    /// assert!(cc.is_cacheable_by_shared_cache());
    /// ```
    pub fn is_cacheable_by_shared_cache(&self) -> bool {
        !self.no_store && !self.private
    }

    /// Check if the response should be stored in cache.
    ///
    /// This is a stricter check than `is_cacheable_by_shared_cache` that also
    /// considers `max-age=0` as non-cacheable (since it's immediately stale).
    ///
    /// # Returns
    /// `false` if the response should not be stored:
    /// - `no-store` is present
    /// - `private` is present
    /// - `max-age=0` with no `stale-while-revalidate`
    pub fn should_store(&self) -> bool {
        if !self.is_cacheable_by_shared_cache() {
            return false;
        }

        // max-age=0 without stale-while-revalidate means immediately stale
        // and not useful to cache
        if let Some(max_age) = self.effective_max_age() {
            if max_age.is_zero() && self.stale_while_revalidate.is_none() {
                return false;
            }
        }

        true
    }

    /// Get the effective TTL for this response in a shared cache.
    ///
    /// For shared caches, `s-maxage` takes precedence over `max-age`.
    /// If neither is present, returns `None` (caller should use a default TTL).
    ///
    /// # Arguments
    /// * `default_ttl` - TTL to use if no max-age directives are present
    ///
    /// # Returns
    /// The effective TTL for caching this response
    ///
    /// # Example
    /// ```rust
    /// use yatagarasu::cache::CacheControl;
    /// use std::time::Duration;
    ///
    /// // s-maxage takes precedence for shared caches
    /// let cc = CacheControl::parse("max-age=3600, s-maxage=7200");
    /// let default = Duration::from_secs(300);
    /// assert_eq!(cc.effective_ttl(default), Duration::from_secs(7200));
    ///
    /// // Falls back to max-age if no s-maxage
    /// let cc = CacheControl::parse("max-age=3600");
    /// assert_eq!(cc.effective_ttl(default), Duration::from_secs(3600));
    ///
    /// // Uses default if no directives
    /// let cc = CacheControl::parse("");
    /// assert_eq!(cc.effective_ttl(default), Duration::from_secs(300));
    /// ```
    pub fn effective_ttl(&self, default_ttl: Duration) -> Duration {
        self.effective_max_age().unwrap_or(default_ttl)
    }

    /// Get the effective max-age for this response.
    ///
    /// For shared caches, `s-maxage` takes precedence over `max-age`.
    fn effective_max_age(&self) -> Option<Duration> {
        self.s_maxage.or(self.max_age)
    }

    /// Check if the cache must revalidate before serving stale content.
    ///
    /// Returns `true` if:
    /// - `must-revalidate` is present, OR
    /// - `proxy-revalidate` is present (for shared caches), OR
    /// - `no-cache` is present
    pub fn requires_revalidation(&self) -> bool {
        self.must_revalidate || self.proxy_revalidate || self.no_cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_max_age() {
        let cc = CacheControl::parse("max-age=3600");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
        assert!(cc.is_cacheable_by_shared_cache());
    }

    #[test]
    fn test_parse_s_maxage() {
        let cc = CacheControl::parse("s-maxage=7200");
        assert_eq!(cc.s_maxage, Some(Duration::from_secs(7200)));
    }

    #[test]
    fn test_parse_s_maxage_takes_precedence() {
        let cc = CacheControl::parse("max-age=3600, s-maxage=7200");
        assert_eq!(
            cc.effective_ttl(Duration::from_secs(60)),
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn test_parse_no_cache() {
        let cc = CacheControl::parse("no-cache");
        assert!(cc.no_cache);
        // no-cache allows storage but requires revalidation
        assert!(cc.is_cacheable_by_shared_cache());
        assert!(cc.requires_revalidation());
    }

    #[test]
    fn test_parse_no_store() {
        let cc = CacheControl::parse("no-store");
        assert!(cc.no_store);
        assert!(!cc.is_cacheable_by_shared_cache());
    }

    #[test]
    fn test_parse_private() {
        let cc = CacheControl::parse("private");
        assert!(cc.private);
        assert!(!cc.is_cacheable_by_shared_cache());
    }

    #[test]
    fn test_parse_must_revalidate() {
        let cc = CacheControl::parse("must-revalidate");
        assert!(cc.must_revalidate);
        assert!(cc.requires_revalidation());
    }

    #[test]
    fn test_parse_multiple_directives() {
        let cc = CacheControl::parse("max-age=3600, no-cache, must-revalidate");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
        assert!(cc.no_cache);
        assert!(cc.must_revalidate);
    }

    #[test]
    fn test_parse_with_whitespace() {
        let cc = CacheControl::parse("  max-age=3600 ,  no-cache  ");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
        assert!(cc.no_cache);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let cc = CacheControl::parse("Max-Age=3600, No-Cache, MUST-REVALIDATE");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
        assert!(cc.no_cache);
        assert!(cc.must_revalidate);
    }

    #[test]
    fn test_parse_empty_string() {
        let cc = CacheControl::parse("");
        assert_eq!(cc.max_age, None);
        assert!(!cc.no_store);
        assert!(!cc.no_cache);
        assert!(!cc.private);
    }

    #[test]
    fn test_parse_invalid_max_age() {
        let cc = CacheControl::parse("max-age=invalid");
        assert_eq!(cc.max_age, None);
    }

    #[test]
    fn test_parse_stale_while_revalidate() {
        let cc = CacheControl::parse("max-age=3600, stale-while-revalidate=60");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
        assert_eq!(cc.stale_while_revalidate, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_parse_public() {
        let cc = CacheControl::parse("public, max-age=3600");
        assert!(cc.public);
        assert!(cc.is_cacheable_by_shared_cache());
    }

    #[test]
    fn test_parse_immutable() {
        let cc = CacheControl::parse("max-age=31536000, immutable");
        assert!(cc.immutable);
        assert_eq!(cc.max_age, Some(Duration::from_secs(31536000)));
    }

    #[test]
    fn test_effective_ttl_uses_default() {
        let cc = CacheControl::parse("");
        let default = Duration::from_secs(300);
        assert_eq!(cc.effective_ttl(default), default);
    }

    #[test]
    fn test_should_store_false_for_no_store() {
        let cc = CacheControl::parse("no-store");
        assert!(!cc.should_store());
    }

    #[test]
    fn test_should_store_false_for_private() {
        let cc = CacheControl::parse("private, max-age=3600");
        assert!(!cc.should_store());
    }

    #[test]
    fn test_should_store_false_for_max_age_zero() {
        let cc = CacheControl::parse("max-age=0");
        assert!(!cc.should_store());
    }

    #[test]
    fn test_should_store_true_for_max_age_zero_with_stale_while_revalidate() {
        let cc = CacheControl::parse("max-age=0, stale-while-revalidate=60");
        // This pattern is used for "always revalidate but serve stale while doing so"
        assert!(cc.should_store());
    }

    #[test]
    fn test_should_store_true_for_no_cache() {
        // no-cache allows storage, just requires revalidation before use
        let cc = CacheControl::parse("no-cache, max-age=3600");
        assert!(cc.should_store());
        assert!(cc.requires_revalidation());
    }

    #[test]
    fn test_proxy_revalidate() {
        let cc = CacheControl::parse("proxy-revalidate");
        assert!(cc.proxy_revalidate);
        assert!(cc.requires_revalidation());
    }

    #[test]
    fn test_parse_quoted_value() {
        // Some implementations quote directive values
        let cc = CacheControl::parse("max-age=\"3600\"");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_unknown_directive_ignored() {
        let cc = CacheControl::parse("max-age=3600, unknown-directive, foo=bar");
        assert_eq!(cc.max_age, Some(Duration::from_secs(3600)));
        // Should not panic or error, just ignore unknown directives
    }
}
