//! Proxy utility functions.
//!
//! This module contains helper functions for request processing:
//! - Header extraction from Pingora requests
//! - Query parameter parsing
//! - Client IP detection (X-Forwarded-For aware)
//! - Circuit breaker metrics export

use std::collections::HashMap;
use std::sync::Arc;

use pingora_http::RequestHeader;
use pingora_proxy::Session;

use crate::circuit_breaker::CircuitBreaker;

/// Extract headers from Pingora RequestHeader into HashMap.
///
/// Converts all headers to string key-value pairs. Headers with non-UTF8
/// values are skipped.
pub fn extract_headers(req: &RequestHeader) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for (name, value) in req.headers.iter() {
        if let Ok(value_str) = value.to_str() {
            headers.insert(name.to_string(), value_str.to_string());
        }
    }
    headers
}

/// Extract query parameters from URI.
///
/// Parses the query string from the request URI and returns key-value pairs.
/// Values are URL-decoded.
pub fn extract_query_params(req: &RequestHeader) -> HashMap<String, String> {
    let mut params = HashMap::new();
    if let Some(query) = req.uri.query() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    key.to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }
    }
    params
}

/// Extract client IP address from session (X-Forwarded-For aware).
///
/// Checks X-Forwarded-For header first (for proxies/load balancers),
/// then falls back to direct connection IP from session.
///
/// # X-Forwarded-For handling
///
/// The header can contain multiple IPs: `"client, proxy1, proxy2"`.
/// The first IP is the original client, which is what we return.
pub fn get_client_ip(session: &Session) -> String {
    // Check X-Forwarded-For header first (common in reverse proxy setups)
    if let Some(forwarded_for) = session
        .req_header()
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        // X-Forwarded-For can contain multiple IPs: "client, proxy1, proxy2"
        // The first IP is the original client
        if let Some(client_ip) = forwarded_for.split(',').next() {
            return client_ip.trim().to_string();
        }
    }

    // Fall back to direct connection IP
    session
        .client_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Export circuit breaker metrics for Prometheus.
///
/// Generates Prometheus-compatible metrics text for all circuit breakers:
/// - `circuit_breaker_state` - Current state (0=closed, 1=open, 2=half-open)
/// - `circuit_breaker_failures` - Consecutive failure count
/// - `circuit_breaker_successes` - Success count in half-open state
pub fn export_circuit_breaker_metrics(
    circuit_breakers: &HashMap<String, Arc<CircuitBreaker>>,
) -> String {
    let mut output = String::new();

    // Circuit breaker state metric (gauge: 0=closed, 1=open, 2=half-open)
    output.push_str(
        "\n# HELP circuit_breaker_state Circuit breaker state per bucket (0=closed, 1=open, 2=half-open)\n",
    );
    output.push_str("# TYPE circuit_breaker_state gauge\n");

    for (bucket_name, circuit_breaker) in circuit_breakers.iter() {
        let state_value = circuit_breaker.state() as u8;
        output.push_str(&format!(
            "circuit_breaker_state{{bucket=\"{}\"}} {}\n",
            bucket_name, state_value
        ));
    }

    // Circuit breaker failure count metric (gauge)
    output.push_str("\n# HELP circuit_breaker_failures Current consecutive failure count\n");
    output.push_str("# TYPE circuit_breaker_failures gauge\n");

    for (bucket_name, circuit_breaker) in circuit_breakers.iter() {
        output.push_str(&format!(
            "circuit_breaker_failures{{bucket=\"{}\"}} {}\n",
            bucket_name,
            circuit_breaker.failure_count()
        ));
    }

    // Circuit breaker success count in half-open state (gauge)
    output.push_str("\n# HELP circuit_breaker_successes Success count in half-open state\n");
    output.push_str("# TYPE circuit_breaker_successes gauge\n");

    for (bucket_name, circuit_breaker) in circuit_breakers.iter() {
        output.push_str(&format!(
            "circuit_breaker_successes{{bucket=\"{}\"}} {}\n",
            bucket_name,
            circuit_breaker.success_count()
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_headers_empty() {
        let req = RequestHeader::build("GET", b"/", None).unwrap();
        let headers = extract_headers(&req);
        // Only host header might be present from build
        assert!(headers.len() <= 1);
    }

    #[test]
    fn test_extract_query_params_empty() {
        let req = RequestHeader::build("GET", b"/path", None).unwrap();
        let params = extract_query_params(&req);
        assert!(params.is_empty());
    }

    #[test]
    fn test_extract_query_params_single() {
        let req = RequestHeader::build("GET", b"/path?key=value", None).unwrap();
        let params = extract_query_params(&req);
        assert_eq!(params.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_extract_query_params_multiple() {
        let req = RequestHeader::build("GET", b"/path?a=1&b=2&c=3", None).unwrap();
        let params = extract_query_params(&req);
        assert_eq!(params.get("a"), Some(&"1".to_string()));
        assert_eq!(params.get("b"), Some(&"2".to_string()));
        assert_eq!(params.get("c"), Some(&"3".to_string()));
    }

    #[test]
    fn test_extract_query_params_url_encoded() {
        let req = RequestHeader::build("GET", b"/path?name=hello%20world", None).unwrap();
        let params = extract_query_params(&req);
        assert_eq!(params.get("name"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_export_circuit_breaker_metrics_empty() {
        let circuit_breakers = HashMap::new();
        let metrics = export_circuit_breaker_metrics(&circuit_breakers);
        assert!(metrics.contains("circuit_breaker_state"));
        assert!(metrics.contains("circuit_breaker_failures"));
        assert!(metrics.contains("circuit_breaker_successes"));
    }
}
