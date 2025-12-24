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

pub mod compositor;
pub mod config;
pub mod error;
pub mod image_fetcher;
pub mod position;
pub mod template;
pub mod text_renderer;

// Re-export main types for convenience
pub use compositor::{
    apply_watermark, apply_watermarks, create_diagonal_layers, create_positioned_layer,
    create_tiled_layers, Compositor, WatermarkLayer,
};
pub use config::{
    BucketWatermarkConfig, ImageWatermarkConfig, TextWatermarkConfig, WatermarkDefinition,
    WatermarkPosition, WatermarkRule,
};
pub use error::WatermarkError;
pub use image_fetcher::{CachedImage, ImageFetcher, ImageFetcherConfig, ImageSource};
pub use position::{
    calculate_diagonal_positions, calculate_position, calculate_tiled_positions, clamp_to_bounds,
    is_visible, ImageDimensions, PlacementPosition, WatermarkDimensions,
};
pub use template::{resolve_template, template_hash, TemplateContext};
pub use text_renderer::{measure_text, parse_hex_color, render_text, Color, TextRenderOptions};
