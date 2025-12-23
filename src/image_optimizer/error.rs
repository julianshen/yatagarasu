//! Image optimization error types
//!
//! Provides structured error handling with HTTP status mapping,
//! consistent with the compression module pattern.

use std::fmt;

/// Errors that can occur during image optimization operations
#[derive(Debug, Clone)]
pub enum ImageError {
    // === Decoding Errors ===
    /// Image format is not supported
    UnsupportedFormat { format: String },
    /// Failed to decode image data
    DecodeFailed { message: String },
    /// Image data is corrupted or invalid
    CorruptedImage { message: String },

    // === Processing Errors ===
    /// Resize operation failed
    ResizeFailed { message: String },
    /// Encoding to output format failed
    EncodeFailed { format: String, message: String },
    /// Processing took too long
    ProcessingTimeout { timeout_ms: u64 },

    // === Security Errors ===
    /// URL signature is invalid or missing
    InvalidSignature,
    /// Source URL is not in allowed list
    SourceNotAllowed { source: String },
    /// Image dimensions exceed safety limits (image bomb protection)
    ImageBombDetected {
        width: u32,
        height: u32,
        pixels: u64,
        max_pixels: u64,
    },
    /// Input file size exceeds limit
    FileTooLarge { size: usize, max_size: usize },

    // === Parameter Errors ===
    /// Invalid transformation parameter
    InvalidParameter { param: String, message: String },
    /// Requested dimensions are invalid
    InvalidDimensions {
        width: u32,
        height: u32,
        reason: String,
    },
    /// Quality value out of range
    InvalidQuality { quality: u8 },
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Decoding errors
            ImageError::UnsupportedFormat { format } => {
                write!(f, "Unsupported image format: {}", format)
            }
            ImageError::DecodeFailed { message } => {
                write!(f, "Failed to decode image: {}", message)
            }
            ImageError::CorruptedImage { message } => {
                write!(f, "Corrupted image data: {}", message)
            }

            // Processing errors
            ImageError::ResizeFailed { message } => {
                write!(f, "Resize failed: {}", message)
            }
            ImageError::EncodeFailed { format, message } => {
                write!(f, "Failed to encode to {}: {}", format, message)
            }
            ImageError::ProcessingTimeout { timeout_ms } => {
                write!(f, "Processing timeout after {}ms", timeout_ms)
            }

            // Security errors
            ImageError::InvalidSignature => {
                write!(f, "Invalid or missing URL signature")
            }
            ImageError::SourceNotAllowed { source } => {
                write!(f, "Source not allowed: {}", source)
            }
            ImageError::ImageBombDetected {
                width,
                height,
                pixels,
                max_pixels,
            } => {
                write!(
                    f,
                    "Image dimensions {}x{} ({} pixels) exceed limit of {} pixels",
                    width, height, pixels, max_pixels
                )
            }
            ImageError::FileTooLarge { size, max_size } => {
                write!(
                    f,
                    "File size {} bytes exceeds maximum {} bytes",
                    size, max_size
                )
            }

            // Parameter errors
            ImageError::InvalidParameter { param, message } => {
                write!(f, "Invalid parameter '{}': {}", param, message)
            }
            ImageError::InvalidDimensions {
                width,
                height,
                reason,
            } => {
                write!(f, "Invalid dimensions {}x{}: {}", width, height, reason)
            }
            ImageError::InvalidQuality { quality } => {
                write!(f, "Invalid quality {}: must be 1-100", quality)
            }
        }
    }
}

impl std::error::Error for ImageError {}

impl ImageError {
    /// Maps image errors to HTTP status codes
    ///
    /// Status mapping:
    /// - UnsupportedFormat → 415 (Unsupported Media Type)
    /// - DecodeFailed, CorruptedImage → 400 (Bad Request)
    /// - ResizeFailed, EncodeFailed → 500 (Internal Server Error)
    /// - ProcessingTimeout → 504 (Gateway Timeout)
    /// - InvalidSignature, SourceNotAllowed → 403 (Forbidden)
    /// - ImageBombDetected, InvalidParameter, InvalidDimensions, InvalidQuality → 400
    /// - FileTooLarge → 413 (Payload Too Large)
    pub fn to_http_status(&self) -> u16 {
        match self {
            // 415 Unsupported Media Type
            ImageError::UnsupportedFormat { .. } => 415,

            // 400 Bad Request
            ImageError::DecodeFailed { .. }
            | ImageError::CorruptedImage { .. }
            | ImageError::ImageBombDetected { .. }
            | ImageError::InvalidParameter { .. }
            | ImageError::InvalidDimensions { .. }
            | ImageError::InvalidQuality { .. } => 400,

            // 403 Forbidden
            ImageError::InvalidSignature | ImageError::SourceNotAllowed { .. } => 403,

            // 413 Payload Too Large
            ImageError::FileTooLarge { .. } => 413,

            // 500 Internal Server Error
            ImageError::ResizeFailed { .. } | ImageError::EncodeFailed { .. } => 500,

            // 504 Gateway Timeout
            ImageError::ProcessingTimeout { .. } => 504,
        }
    }

    /// Helper constructors for common error patterns
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        ImageError::UnsupportedFormat {
            format: format.into(),
        }
    }

    pub fn decode_failed(message: impl Into<String>) -> Self {
        ImageError::DecodeFailed {
            message: message.into(),
        }
    }

    pub fn resize_failed(message: impl Into<String>) -> Self {
        ImageError::ResizeFailed {
            message: message.into(),
        }
    }

    pub fn encode_failed(format: impl Into<String>, message: impl Into<String>) -> Self {
        ImageError::EncodeFailed {
            format: format.into(),
            message: message.into(),
        }
    }

    pub fn invalid_param(param: impl Into<String>, message: impl Into<String>) -> Self {
        ImageError::InvalidParameter {
            param: param.into(),
            message: message.into(),
        }
    }

    pub fn image_bomb(width: u32, height: u32, max_pixels: u64) -> Self {
        ImageError::ImageBombDetected {
            width,
            height,
            pixels: width as u64 * height as u64,
            max_pixels,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsupported_format_display() {
        let err = ImageError::unsupported_format("tga");
        assert_eq!(err.to_string(), "Unsupported image format: tga");
        assert_eq!(err.to_http_status(), 415);
    }

    #[test]
    fn test_decode_failed_display() {
        let err = ImageError::decode_failed("invalid header");
        assert_eq!(err.to_string(), "Failed to decode image: invalid header");
        assert_eq!(err.to_http_status(), 400);
    }

    #[test]
    fn test_resize_failed_display() {
        let err = ImageError::resize_failed("out of memory");
        assert_eq!(err.to_string(), "Resize failed: out of memory");
        assert_eq!(err.to_http_status(), 500);
    }

    #[test]
    fn test_encode_failed_display() {
        let err = ImageError::encode_failed("webp", "encoder error");
        assert_eq!(err.to_string(), "Failed to encode to webp: encoder error");
        assert_eq!(err.to_http_status(), 500);
    }

    #[test]
    fn test_invalid_signature_display() {
        let err = ImageError::InvalidSignature;
        assert_eq!(err.to_string(), "Invalid or missing URL signature");
        assert_eq!(err.to_http_status(), 403);
    }

    #[test]
    fn test_image_bomb_display() {
        let err = ImageError::image_bomb(10000, 10000, 50_000_000);
        assert!(err.to_string().contains("100000000 pixels"));
        assert_eq!(err.to_http_status(), 400);
    }

    #[test]
    fn test_file_too_large_display() {
        let err = ImageError::FileTooLarge {
            size: 100_000_000,
            max_size: 50_000_000,
        };
        assert!(err.to_string().contains("100000000 bytes"));
        assert_eq!(err.to_http_status(), 413);
    }

    #[test]
    fn test_processing_timeout_display() {
        let err = ImageError::ProcessingTimeout { timeout_ms: 30000 };
        assert_eq!(err.to_string(), "Processing timeout after 30000ms");
        assert_eq!(err.to_http_status(), 504);
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ImageError>();
    }
}
