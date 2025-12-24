//! Watermark error types.
//!
//! Defines errors that can occur during watermark processing.

use std::fmt;

/// Errors that can occur during watermark processing.
#[derive(Debug)]
pub enum WatermarkError {
    /// Failed to fetch watermark image from source
    FetchError(String),

    /// Failed to decode watermark image
    DecodeError(String),

    /// Failed to render text watermark
    RenderError(String),

    /// Invalid configuration
    ConfigError(String),

    /// Template resolution failed
    TemplateError(String),

    /// Failed to composite watermark onto image
    CompositeError(String),
}

impl fmt::Display for WatermarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchError(msg) => write!(f, "Failed to fetch watermark: {}", msg),
            Self::DecodeError(msg) => write!(f, "Failed to decode watermark image: {}", msg),
            Self::RenderError(msg) => write!(f, "Failed to render text watermark: {}", msg),
            Self::ConfigError(msg) => write!(f, "Watermark configuration error: {}", msg),
            Self::TemplateError(msg) => write!(f, "Template resolution failed: {}", msg),
            Self::CompositeError(msg) => write!(f, "Failed to composite watermark: {}", msg),
        }
    }
}

impl std::error::Error for WatermarkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = WatermarkError::FetchError("connection timeout".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to fetch watermark: connection timeout"
        );

        let err = WatermarkError::DecodeError("invalid PNG".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to decode watermark image: invalid PNG"
        );

        let err = WatermarkError::RenderError("font not found".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to render text watermark: font not found"
        );

        let err = WatermarkError::ConfigError("invalid opacity".to_string());
        assert_eq!(
            err.to_string(),
            "Watermark configuration error: invalid opacity"
        );

        let err = WatermarkError::TemplateError("unknown variable".to_string());
        assert_eq!(
            err.to_string(),
            "Template resolution failed: unknown variable"
        );

        let err = WatermarkError::CompositeError("image too small".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to composite watermark: image too small"
        );
    }

    #[test]
    fn test_error_debug() {
        let err = WatermarkError::FetchError("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("FetchError"));
        assert!(debug_str.contains("test"));
    }
}
