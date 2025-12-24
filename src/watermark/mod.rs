//! Watermark module for applying text and image watermarks to images.
//!
//! This module provides server-side enforced watermarking for images served
//! through the S3 proxy. Watermarks are configured per-bucket with path pattern
//! matching rules.
//!
//! # Features
//!
//! - **Text watermarks** with template variable support (JWT claims, IP, headers)
//! - **Image watermarks** from S3 buckets or external URLs (with caching)
//! - **11 positioning modes**: 9-grid, tiled, and diagonal band
//! - **Per-bucket configuration** with glob pattern path matching
//!
//! # Configuration Example
//!
//! ```yaml
//! buckets:
//!   - name: products
//!     watermark:
//!       enabled: true
//!       rules:
//!         - pattern: "/products/previews/*"
//!           watermarks:
//!             - type: text
//!               text: "Copyright {{jwt.org}}"
//!               position: bottom-right
//!             - type: image
//!               source: "s3://assets/logo.png"
//!               position: top-left
//! ```
//!
//! # Template Variables
//!
//! Text watermarks support the following template variables:
//! - `{{jwt.sub}}`, `{{jwt.iss}}`, `{{jwt.<custom>}}` - JWT claims
//! - `{{ip}}` - Client IP address
//! - `{{header.X-Name}}` - Request headers
//! - `{{date}}`, `{{datetime}}`, `{{timestamp}}` - Time values
//! - `{{path}}`, `{{bucket}}` - Request context

pub mod config;
pub mod error;
pub mod template;

// Re-export main types for convenience
pub use config::{
    BucketWatermarkConfig, ImageWatermarkConfig, TextWatermarkConfig, WatermarkDefinition,
    WatermarkPosition, WatermarkRule,
};
pub use error::WatermarkError;
pub use template::{resolve_template, template_hash, TemplateContext};
