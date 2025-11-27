// Observability configuration module
// Phase 34: Enhanced Observability

use serde::{Deserialize, Serialize};

/// Main observability configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// OpenTelemetry tracing configuration
    #[serde(default)]
    pub tracing: TracingConfig,

    /// Request/response logging configuration
    #[serde(default)]
    pub request_logging: RequestLoggingConfig,

    /// Slow query logging configuration
    #[serde(default)]
    pub slow_query: SlowQueryConfig,
}

/// OpenTelemetry tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable OpenTelemetry tracing
    #[serde(default)]
    pub enabled: bool,

    /// Exporter type: "otlp", "jaeger", "zipkin", or "none"
    #[serde(default = "default_exporter")]
    pub exporter: String,

    /// OTLP endpoint (e.g., "http://localhost:4317")
    #[serde(default)]
    pub otlp_endpoint: Option<String>,

    /// Jaeger endpoint (e.g., "http://localhost:14268/api/traces")
    #[serde(default)]
    pub jaeger_endpoint: Option<String>,

    /// Zipkin endpoint (e.g., "http://localhost:9411/api/v2/spans")
    #[serde(default)]
    pub zipkin_endpoint: Option<String>,

    /// Service name for traces (default: "yatagarasu")
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Sampling ratio (0.0 to 1.0, default: 1.0 = sample all)
    #[serde(default = "default_sampling_ratio")]
    pub sampling_ratio: f64,

    /// Whether to propagate trace context in headers
    #[serde(default = "default_true")]
    pub propagate_context: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            exporter: default_exporter(),
            otlp_endpoint: None,
            jaeger_endpoint: None,
            zipkin_endpoint: None,
            service_name: default_service_name(),
            sampling_ratio: default_sampling_ratio(),
            propagate_context: true,
        }
    }
}

/// Request/response logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLoggingConfig {
    /// Enable request logging
    #[serde(default)]
    pub log_requests: bool,

    /// Enable response logging
    #[serde(default)]
    pub log_responses: bool,

    /// Path patterns to include (glob patterns, empty = all)
    #[serde(default)]
    pub include_paths: Vec<String>,

    /// Path patterns to exclude (glob patterns)
    #[serde(default)]
    pub exclude_paths: Vec<String>,

    /// Status codes to log (empty = all)
    #[serde(default)]
    pub status_codes: Vec<u16>,

    /// Headers to redact in logs
    #[serde(default = "default_redact_headers")]
    pub redact_headers: Vec<String>,

    /// Log request body (up to max_body_size bytes)
    #[serde(default)]
    pub log_request_body: bool,

    /// Log response body (up to max_body_size bytes)
    #[serde(default)]
    pub log_response_body: bool,

    /// Maximum body size to log (default: 1KB)
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

impl Default for RequestLoggingConfig {
    fn default() -> Self {
        Self {
            log_requests: false,
            log_responses: false,
            include_paths: vec![],
            exclude_paths: vec![],
            status_codes: vec![],
            redact_headers: default_redact_headers(),
            log_request_body: false,
            log_response_body: false,
            max_body_size: default_max_body_size(),
        }
    }
}

/// Slow query logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryConfig {
    /// Enable slow query logging
    #[serde(default)]
    pub enabled: bool,

    /// Threshold in milliseconds (default: 1000ms = 1 second)
    #[serde(default = "default_slow_query_threshold_ms")]
    pub threshold_ms: u64,

    /// Include timing breakdown (auth, cache, s3)
    #[serde(default = "default_true")]
    pub include_breakdown: bool,

    /// Log level for slow queries (default: "warn")
    #[serde(default = "default_slow_query_log_level")]
    pub log_level: String,
}

impl Default for SlowQueryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_ms: default_slow_query_threshold_ms(),
            include_breakdown: true,
            log_level: default_slow_query_log_level(),
        }
    }
}

// Default value functions
fn default_exporter() -> String {
    "otlp".to_string()
}

fn default_service_name() -> String {
    "yatagarasu".to_string()
}

fn default_sampling_ratio() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_redact_headers() -> Vec<String> {
    vec![
        "authorization".to_string(),
        "x-api-key".to_string(),
        "cookie".to_string(),
        "set-cookie".to_string(),
        "x-amz-security-token".to_string(),
    ]
}

fn default_max_body_size() -> usize {
    1024 // 1KB
}

fn default_slow_query_threshold_ms() -> u64 {
    1000 // 1 second
}

fn default_slow_query_log_level() -> String {
    "warn".to_string()
}

/// Exporter type enum for validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExporterType {
    Otlp,
    Jaeger,
    Zipkin,
    None,
}

impl TracingConfig {
    /// Parse exporter type from string
    pub fn exporter_type(&self) -> ExporterType {
        match self.exporter.to_lowercase().as_str() {
            "otlp" => ExporterType::Otlp,
            "jaeger" => ExporterType::Jaeger,
            "zipkin" => ExporterType::Zipkin,
            "none" | "" => ExporterType::None,
            _ => ExporterType::None,
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        match self.exporter_type() {
            ExporterType::Otlp => {
                if self.otlp_endpoint.is_none() {
                    return Err("OTLP exporter requires otlp_endpoint".to_string());
                }
            }
            ExporterType::Jaeger => {
                if self.jaeger_endpoint.is_none() {
                    return Err("Jaeger exporter requires jaeger_endpoint".to_string());
                }
            }
            ExporterType::Zipkin => {
                if self.zipkin_endpoint.is_none() {
                    return Err("Zipkin exporter requires zipkin_endpoint".to_string());
                }
            }
            ExporterType::None => {}
        }

        if !(0.0..=1.0).contains(&self.sampling_ratio) {
            return Err(format!(
                "sampling_ratio must be between 0.0 and 1.0, got {}",
                self.sampling_ratio
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_empty_observability_config() {
        let config = ObservabilityConfig::default();
        assert!(!config.tracing.enabled);
        assert!(!config.request_logging.log_requests);
        assert!(!config.slow_query.enabled);
    }

    #[test]
    fn test_can_deserialize_minimal_observability_config() {
        let yaml = "{}";
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.tracing.enabled);
    }

    #[test]
    fn test_tracing_config_defaults() {
        let config = TracingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.exporter, "otlp");
        assert_eq!(config.service_name, "yatagarasu");
        assert!((config.sampling_ratio - 1.0).abs() < f64::EPSILON);
        assert!(config.propagate_context);
    }

    #[test]
    fn test_can_parse_tracing_config_with_otlp() {
        let yaml = r#"
tracing:
  enabled: true
  exporter: otlp
  otlp_endpoint: "http://localhost:4317"
  service_name: my-service
  sampling_ratio: 0.5
"#;
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.tracing.enabled);
        assert_eq!(config.tracing.exporter, "otlp");
        assert_eq!(
            config.tracing.otlp_endpoint,
            Some("http://localhost:4317".to_string())
        );
        assert_eq!(config.tracing.service_name, "my-service");
        assert!((config.tracing.sampling_ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_can_parse_tracing_config_with_jaeger() {
        let yaml = r#"
tracing:
  enabled: true
  exporter: jaeger
  jaeger_endpoint: "http://localhost:14268/api/traces"
"#;
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.tracing.enabled);
        assert_eq!(config.tracing.exporter, "jaeger");
        assert_eq!(
            config.tracing.jaeger_endpoint,
            Some("http://localhost:14268/api/traces".to_string())
        );
    }

    #[test]
    fn test_can_parse_tracing_config_with_zipkin() {
        let yaml = r#"
tracing:
  enabled: true
  exporter: zipkin
  zipkin_endpoint: "http://localhost:9411/api/v2/spans"
"#;
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.tracing.enabled);
        assert_eq!(config.tracing.exporter, "zipkin");
        assert_eq!(
            config.tracing.zipkin_endpoint,
            Some("http://localhost:9411/api/v2/spans".to_string())
        );
    }

    #[test]
    fn test_exporter_type_parsing() {
        let mut config = TracingConfig::default();

        config.exporter = "otlp".to_string();
        assert_eq!(config.exporter_type(), ExporterType::Otlp);

        config.exporter = "OTLP".to_string();
        assert_eq!(config.exporter_type(), ExporterType::Otlp);

        config.exporter = "jaeger".to_string();
        assert_eq!(config.exporter_type(), ExporterType::Jaeger);

        config.exporter = "zipkin".to_string();
        assert_eq!(config.exporter_type(), ExporterType::Zipkin);

        config.exporter = "none".to_string();
        assert_eq!(config.exporter_type(), ExporterType::None);

        config.exporter = "unknown".to_string();
        assert_eq!(config.exporter_type(), ExporterType::None);
    }

    #[test]
    fn test_tracing_config_validation_disabled() {
        let config = TracingConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tracing_config_validation_otlp_requires_endpoint() {
        let config = TracingConfig {
            enabled: true,
            exporter: "otlp".to_string(),
            otlp_endpoint: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("otlp_endpoint"));
    }

    #[test]
    fn test_tracing_config_validation_jaeger_requires_endpoint() {
        let config = TracingConfig {
            enabled: true,
            exporter: "jaeger".to_string(),
            jaeger_endpoint: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("jaeger_endpoint"));
    }

    #[test]
    fn test_tracing_config_validation_zipkin_requires_endpoint() {
        let config = TracingConfig {
            enabled: true,
            exporter: "zipkin".to_string(),
            zipkin_endpoint: None,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("zipkin_endpoint"));
    }

    #[test]
    fn test_tracing_config_validation_sampling_ratio() {
        let config = TracingConfig {
            enabled: true,
            exporter: "none".to_string(),
            sampling_ratio: 1.5,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sampling_ratio"));
    }

    #[test]
    fn test_request_logging_config_defaults() {
        let config = RequestLoggingConfig::default();
        assert!(!config.log_requests);
        assert!(!config.log_responses);
        assert!(config.include_paths.is_empty());
        assert!(config.exclude_paths.is_empty());
        assert!(config.status_codes.is_empty());
        assert!(config.redact_headers.contains(&"authorization".to_string()));
        assert_eq!(config.max_body_size, 1024);
    }

    #[test]
    fn test_can_parse_request_logging_config() {
        let yaml = r#"
request_logging:
  log_requests: true
  log_responses: true
  include_paths:
    - "/api/*"
    - "/public/*"
  exclude_paths:
    - "/health"
    - "/metrics"
  status_codes:
    - 400
    - 500
  redact_headers:
    - authorization
    - x-custom-secret
  log_request_body: true
  max_body_size: 4096
"#;
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.request_logging.log_requests);
        assert!(config.request_logging.log_responses);
        assert_eq!(config.request_logging.include_paths.len(), 2);
        assert_eq!(config.request_logging.exclude_paths.len(), 2);
        assert_eq!(config.request_logging.status_codes, vec![400, 500]);
        assert!(config.request_logging.log_request_body);
        assert_eq!(config.request_logging.max_body_size, 4096);
    }

    #[test]
    fn test_slow_query_config_defaults() {
        let config = SlowQueryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.threshold_ms, 1000);
        assert!(config.include_breakdown);
        assert_eq!(config.log_level, "warn");
    }

    #[test]
    fn test_can_parse_slow_query_config() {
        let yaml = r#"
slow_query:
  enabled: true
  threshold_ms: 500
  include_breakdown: true
  log_level: error
"#;
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.slow_query.enabled);
        assert_eq!(config.slow_query.threshold_ms, 500);
        assert!(config.slow_query.include_breakdown);
        assert_eq!(config.slow_query.log_level, "error");
    }

    #[test]
    fn test_can_parse_full_observability_config() {
        let yaml = r#"
tracing:
  enabled: true
  exporter: otlp
  otlp_endpoint: "http://jaeger:4317"
  service_name: my-proxy
  sampling_ratio: 0.1
request_logging:
  log_requests: true
  exclude_paths:
    - "/health"
slow_query:
  enabled: true
  threshold_ms: 2000
"#;
        let config: ObservabilityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.tracing.enabled);
        assert_eq!(config.tracing.exporter, "otlp");
        assert!(config.request_logging.log_requests);
        assert!(config.slow_query.enabled);
        assert_eq!(config.slow_query.threshold_ms, 2000);
    }

    #[test]
    fn test_default_redact_headers_include_sensitive() {
        let headers = default_redact_headers();
        assert!(headers.contains(&"authorization".to_string()));
        assert!(headers.contains(&"x-api-key".to_string()));
        assert!(headers.contains(&"cookie".to_string()));
        assert!(headers.contains(&"x-amz-security-token".to_string()));
    }
}
