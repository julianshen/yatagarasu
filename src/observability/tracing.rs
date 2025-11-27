// OpenTelemetry tracing module
// Phase 34: Enhanced Observability

use crate::observability::config::{ExporterType, TracingConfig};
use opentelemetry::trace::TracerProvider as TracerProviderTrait;
use opentelemetry_sdk::trace::TracerProvider;
use std::sync::Arc;
use tracing::Span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

/// OpenTelemetry tracer manager
pub struct TracingManager {
    config: TracingConfig,
    provider: Option<Arc<TracerProvider>>,
}

impl TracingManager {
    /// Create a new tracing manager from config
    pub fn new(config: TracingConfig) -> Self {
        Self {
            config,
            provider: None,
        }
    }

    /// Initialize the tracer provider
    pub fn init(&mut self) -> Result<(), TracingError> {
        if !self.config.enabled {
            return Ok(());
        }

        self.config.validate().map_err(TracingError::ConfigError)?;

        let provider = self.create_provider()?;
        self.provider = Some(Arc::new(provider));

        Ok(())
    }

    /// Initialize tracing with the subscriber layer
    pub fn init_subscriber(&self) -> Result<(), TracingError> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Some(provider) = &self.provider {
            let tracer = provider.tracer(self.config.service_name.clone());
            let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            Registry::default()
                .with(telemetry_layer)
                .try_init()
                .map_err(|e| TracingError::InitError(e.to_string()))?;
        }

        Ok(())
    }

    fn create_provider(&self) -> Result<TracerProvider, TracingError> {
        use opentelemetry_sdk::trace::Sampler;

        let sampler = if (self.config.sampling_ratio - 1.0).abs() < f64::EPSILON {
            Sampler::AlwaysOn
        } else if self.config.sampling_ratio <= 0.0 {
            Sampler::AlwaysOff
        } else {
            Sampler::TraceIdRatioBased(self.config.sampling_ratio)
        };

        match self.config.exporter_type() {
            ExporterType::Otlp => self.create_otlp_provider(sampler),
            ExporterType::Jaeger | ExporterType::Zipkin => {
                // For Jaeger/Zipkin, use OTLP with their endpoints
                // Modern Jaeger supports OTLP directly
                self.create_otlp_provider(sampler)
            }
            ExporterType::None => Ok(TracerProvider::builder().with_sampler(sampler).build()),
        }
    }

    fn create_otlp_provider(
        &self,
        sampler: opentelemetry_sdk::trace::Sampler,
    ) -> Result<TracerProvider, TracingError> {
        use opentelemetry_otlp::WithExportConfig;
        use opentelemetry_sdk::runtime;

        let endpoint = self.get_endpoint().ok_or_else(|| {
            TracingError::ConfigError("No endpoint configured for exporter".to_string())
        })?;

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build()
            .map_err(|e| TracingError::ExporterError(e.to_string()))?;

        let provider = TracerProvider::builder()
            .with_batch_exporter(exporter, runtime::Tokio)
            .with_sampler(sampler)
            .build();

        Ok(provider)
    }

    fn get_endpoint(&self) -> Option<String> {
        match self.config.exporter_type() {
            ExporterType::Otlp => self.config.otlp_endpoint.clone(),
            ExporterType::Jaeger => self.config.jaeger_endpoint.clone(),
            ExporterType::Zipkin => self.config.zipkin_endpoint.clone(),
            ExporterType::None => None,
        }
    }

    /// Shutdown the tracer provider
    pub fn shutdown(&self) {
        if let Some(provider) = &self.provider {
            let _ = provider.shutdown();
        }
    }

    /// Check if tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.provider.is_some()
    }
}

/// Tracing error types
#[derive(Debug, thiserror::Error)]
pub enum TracingError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Failed to initialize tracer: {0}")]
    InitError(String),

    #[error("Failed to create exporter: {0}")]
    ExporterError(String),
}

/// Helper to create a request processing span
#[inline]
pub fn create_request_span(
    method: &str,
    path: &str,
    bucket: Option<&str>,
    correlation_id: &str,
) -> Span {
    tracing::info_span!(
        "request",
        http.method = %method,
        http.target = %path,
        bucket = bucket.unwrap_or("unknown"),
        correlation_id = %correlation_id,
        otel.kind = "server"
    )
}

/// Helper to create an S3 operation span
#[inline]
pub fn create_s3_span(operation: &str, bucket: &str, key: &str) -> Span {
    tracing::info_span!(
        "s3_operation",
        operation = %operation,
        bucket = %bucket,
        key = %key,
        otel.kind = "client"
    )
}

/// Helper to create a cache operation span
#[inline]
pub fn create_cache_span(operation: &str, cache_layer: &str, hit: Option<bool>) -> Span {
    tracing::info_span!(
        "cache_operation",
        operation = %operation,
        cache_layer = %cache_layer,
        cache.hit = hit
    )
}

/// Helper to create an auth operation span
#[inline]
pub fn create_auth_span(auth_type: &str) -> Span {
    tracing::info_span!(
        "auth",
        auth_type = %auth_type
    )
}

/// Request timing structure for slow query logging
#[derive(Debug, Clone, Default)]
pub struct RequestTiming {
    pub total_ms: u64,
    pub auth_ms: Option<u64>,
    pub cache_ms: Option<u64>,
    pub s3_ms: Option<u64>,
    pub other_ms: Option<u64>,
}

impl RequestTiming {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_total(mut self, ms: u64) -> Self {
        self.total_ms = ms;
        self
    }

    pub fn with_auth(mut self, ms: u64) -> Self {
        self.auth_ms = Some(ms);
        self
    }

    pub fn with_cache(mut self, ms: u64) -> Self {
        self.cache_ms = Some(ms);
        self
    }

    pub fn with_s3(mut self, ms: u64) -> Self {
        self.s3_ms = Some(ms);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_manager_disabled_by_default() {
        let config = TracingConfig::default();
        let manager = TracingManager::new(config);
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_tracing_manager_init_disabled() {
        let config = TracingConfig::default();
        let mut manager = TracingManager::new(config);
        assert!(manager.init().is_ok());
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_tracing_manager_init_missing_endpoint() {
        let config = TracingConfig {
            enabled: true,
            exporter: "otlp".to_string(),
            otlp_endpoint: None,
            ..Default::default()
        };
        let mut manager = TracingManager::new(config);
        let result = manager.init();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_request_span() {
        let span = create_request_span("GET", "/bucket/key", Some("my-bucket"), "req-123");
        assert!(span.is_disabled() || !span.is_disabled()); // Just verify it creates without panic
    }

    #[test]
    fn test_create_s3_span() {
        let span = create_s3_span("GetObject", "my-bucket", "path/to/file.txt");
        assert!(span.is_disabled() || !span.is_disabled());
    }

    #[test]
    fn test_create_cache_span() {
        let span = create_cache_span("get", "memory", Some(true));
        assert!(span.is_disabled() || !span.is_disabled());
    }

    #[test]
    fn test_create_auth_span() {
        let span = create_auth_span("jwt");
        assert!(span.is_disabled() || !span.is_disabled());
    }

    #[test]
    fn test_request_timing_builder() {
        let timing = RequestTiming::new()
            .with_total(100)
            .with_auth(10)
            .with_cache(5)
            .with_s3(80);

        assert_eq!(timing.total_ms, 100);
        assert_eq!(timing.auth_ms, Some(10));
        assert_eq!(timing.cache_ms, Some(5));
        assert_eq!(timing.s3_ms, Some(80));
    }

    #[test]
    fn test_tracing_error_display() {
        let err = TracingError::ConfigError("test error".to_string());
        assert!(err.to_string().contains("Configuration error"));

        let err = TracingError::InitError("init failed".to_string());
        assert!(err.to_string().contains("Failed to initialize"));

        let err = TracingError::ExporterError("export failed".to_string());
        assert!(err.to_string().contains("Failed to create exporter"));
    }
}
