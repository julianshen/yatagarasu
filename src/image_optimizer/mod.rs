//! Image optimization module
//!
//! Provides comprehensive image processing with:
//! - Resize, crop, and format conversion
//! - URL signing and security validation
//! - Auto-format selection based on Accept header
//! - Multiple encoder support (JPEG, PNG, WebP, AVIF)
//!
//! # URL Formats
//!
//! Two URL formats are supported:
//!
//! ## Query Parameters
//! ```text
//! /bucket/image.jpg?w=800&h=600&q=80&fmt=webp
//! ```
//!
//! ## Path-Based Options
//! ```text
//! /bucket/w:800,h:600,q:80,f:webp/image.jpg
//! ```
//!
//! # Security
//!
//! When signing is enabled, URLs must include a valid HMAC-SHA256 signature:
//! ```text
//! /bucket/{signature}/w:800/image.jpg
//! ```

// Core modules
pub mod config;
pub mod encoder;
pub mod error;
pub mod format;
pub mod metrics;
pub mod params;
pub mod processor;
pub mod security;

// Re-export commonly used types
pub use config::ImageConfig;
pub use encoder::{EncodedImage, EncoderFactory, EncoderQuality, ImageEncoder};
pub use error::ImageError;
pub use format::{select_format, vary_header, AutoFormatConfig};
pub use metrics::{ImageProcessingMetrics, ImageProcessingMetricsBuilder, TransformationType};
pub use params::{Dimension, FitMode, Gravity, ImageParams, OutputFormat};
pub use processor::{process_image, process_image_internal, ProcessedImage};
pub use security::{
    generate_signature, validate_dimensions, validate_file_size, validate_signature,
    validate_source, SecurityConfig,
};
