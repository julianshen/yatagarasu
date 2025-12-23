//! Image optimization metrics and observability (Phase 50.7)
//!
//! This module implements:
//! - Image processing metrics tracking
//! - Transformation counters
//! - Performance monitoring

use super::params::OutputFormat;
use std::time::Duration;

/// Image processing metrics for a single operation
#[derive(Debug, Clone)]
pub struct ImageProcessingMetrics {
    /// Original image size in bytes
    pub original_size: usize,
    /// Processed image size in bytes
    pub processed_size: usize,
    /// Original dimensions (width, height)
    pub original_dimensions: (u32, u32),
    /// Processed dimensions (width, height)
    pub processed_dimensions: (u32, u32),
    /// Output format used
    pub output_format: OutputFormat,
    /// Time taken to process
    pub processing_time: Duration,
    /// Whether this was a cache hit
    pub cache_hit: bool,
    /// Transformations applied
    pub transformations: Vec<TransformationType>,
}

/// Types of transformations that can be applied
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformationType {
    Resize,
    Crop,
    SmartCrop,
    Rotate,
    Flip,
    FormatConversion,
    QualityAdjustment,
    ExifRotation,
}

impl TransformationType {
    /// Get the metric label for this transformation type
    pub fn as_label(&self) -> &'static str {
        match self {
            TransformationType::Resize => "resize",
            TransformationType::Crop => "crop",
            TransformationType::SmartCrop => "smart_crop",
            TransformationType::Rotate => "rotate",
            TransformationType::Flip => "flip",
            TransformationType::FormatConversion => "format_conversion",
            TransformationType::QualityAdjustment => "quality_adjustment",
            TransformationType::ExifRotation => "exif_rotation",
        }
    }
}

impl Default for ImageProcessingMetrics {
    fn default() -> Self {
        Self {
            original_size: 0,
            processed_size: 0,
            original_dimensions: (0, 0),
            processed_dimensions: (0, 0),
            output_format: OutputFormat::Jpeg,
            processing_time: Duration::ZERO,
            cache_hit: false,
            transformations: Vec::new(),
        }
    }
}

impl ImageProcessingMetrics {
    /// Create a new builder for ImageProcessingMetrics
    pub fn builder() -> ImageProcessingMetricsBuilder {
        ImageProcessingMetricsBuilder::default()
    }

    /// Calculate compression ratio (processed / original)
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            self.processed_size as f64 / self.original_size as f64
        }
    }

    /// Calculate bytes saved by processing
    pub fn bytes_saved(&self) -> i64 {
        self.original_size as i64 - self.processed_size as i64
    }

    /// Calculate percentage saved (can be negative if image grew)
    pub fn percentage_saved(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            (self.bytes_saved() as f64 / self.original_size as f64) * 100.0
        }
    }

    /// Get processing throughput (bytes/second)
    pub fn throughput_bytes_per_sec(&self) -> f64 {
        if self.processing_time.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.original_size as f64 / self.processing_time.as_secs_f64()
        }
    }

    /// Check if the image was resized
    pub fn was_resized(&self) -> bool {
        self.original_dimensions != self.processed_dimensions
    }

    /// Get the resize ratio (processed pixels / original pixels)
    pub fn resize_ratio(&self) -> f64 {
        let original_pixels = self.original_dimensions.0 as u64 * self.original_dimensions.1 as u64;
        let processed_pixels =
            self.processed_dimensions.0 as u64 * self.processed_dimensions.1 as u64;
        if original_pixels == 0 {
            0.0
        } else {
            processed_pixels as f64 / original_pixels as f64
        }
    }
}

/// Builder for ImageProcessingMetrics
#[derive(Debug, Clone, Default)]
pub struct ImageProcessingMetricsBuilder {
    original_size: usize,
    processed_size: usize,
    original_dimensions: (u32, u32),
    processed_dimensions: (u32, u32),
    output_format: Option<OutputFormat>,
    processing_time: Duration,
    cache_hit: bool,
    transformations: Vec<TransformationType>,
}

impl ImageProcessingMetricsBuilder {
    /// Set original size
    pub fn original_size(mut self, size: usize) -> Self {
        self.original_size = size;
        self
    }

    /// Set processed size
    pub fn processed_size(mut self, size: usize) -> Self {
        self.processed_size = size;
        self
    }

    /// Set original dimensions
    pub fn original_dimensions(mut self, width: u32, height: u32) -> Self {
        self.original_dimensions = (width, height);
        self
    }

    /// Set processed dimensions
    pub fn processed_dimensions(mut self, width: u32, height: u32) -> Self {
        self.processed_dimensions = (width, height);
        self
    }

    /// Set output format
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    /// Set processing time
    pub fn processing_time(mut self, time: Duration) -> Self {
        self.processing_time = time;
        self
    }

    /// Set cache hit status
    pub fn cache_hit(mut self, hit: bool) -> Self {
        self.cache_hit = hit;
        self
    }

    /// Add a transformation
    pub fn transformation(mut self, t: TransformationType) -> Self {
        self.transformations.push(t);
        self
    }

    /// Set all transformations
    pub fn transformations(mut self, ts: Vec<TransformationType>) -> Self {
        self.transformations = ts;
        self
    }

    /// Build the metrics
    pub fn build(self) -> ImageProcessingMetrics {
        ImageProcessingMetrics {
            original_size: self.original_size,
            processed_size: self.processed_size,
            original_dimensions: self.original_dimensions,
            processed_dimensions: self.processed_dimensions,
            output_format: self.output_format.unwrap_or(OutputFormat::Jpeg),
            processing_time: self.processing_time,
            cache_hit: self.cache_hit,
            transformations: self.transformations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_processing_metrics_builder() {
        let metrics = ImageProcessingMetrics::builder()
            .original_size(10000)
            .processed_size(5000)
            .original_dimensions(1920, 1080)
            .processed_dimensions(800, 600)
            .output_format(OutputFormat::WebP)
            .processing_time(Duration::from_millis(50))
            .cache_hit(false)
            .transformation(TransformationType::Resize)
            .transformation(TransformationType::FormatConversion)
            .build();
        assert_eq!(metrics.original_size, 10000);
        assert_eq!(metrics.processed_size, 5000);
        assert_eq!(metrics.original_dimensions, (1920, 1080));
        assert_eq!(metrics.processed_dimensions, (800, 600));
        assert!(!metrics.cache_hit);
        assert_eq!(metrics.transformations.len(), 2);
    }

    #[test]
    fn test_compression_ratio() {
        let metrics = ImageProcessingMetrics::builder()
            .original_size(10000)
            .processed_size(5000)
            .build();
        assert_eq!(metrics.compression_ratio(), 0.5);
    }

    #[test]
    fn test_bytes_saved() {
        let metrics = ImageProcessingMetrics::builder()
            .original_size(10000)
            .processed_size(3000)
            .build();
        assert_eq!(metrics.bytes_saved(), 7000);
    }

    #[test]
    fn test_bytes_saved_negative() {
        // Image grew (e.g., converting to lossless format)
        let metrics = ImageProcessingMetrics::builder()
            .original_size(5000)
            .processed_size(8000)
            .output_format(OutputFormat::Png)
            .build();
        assert_eq!(metrics.bytes_saved(), -3000);
    }

    #[test]
    fn test_percentage_saved() {
        let metrics = ImageProcessingMetrics::builder()
            .original_size(10000)
            .processed_size(5000)
            .build();
        assert_eq!(metrics.percentage_saved(), 50.0);
    }

    #[test]
    fn test_throughput_bytes_per_sec() {
        let metrics = ImageProcessingMetrics::builder()
            .original_size(10000)
            .processing_time(Duration::from_secs(1))
            .build();
        assert_eq!(metrics.throughput_bytes_per_sec(), 10000.0);
    }

    #[test]
    fn test_was_resized() {
        let resized = ImageProcessingMetrics::builder()
            .original_dimensions(1920, 1080)
            .processed_dimensions(800, 600)
            .build();
        assert!(resized.was_resized());

        let not_resized = ImageProcessingMetrics::builder()
            .original_dimensions(800, 600)
            .processed_dimensions(800, 600)
            .build();
        assert!(!not_resized.was_resized());
    }

    #[test]
    fn test_resize_ratio() {
        let metrics = ImageProcessingMetrics::builder()
            .original_dimensions(1000, 1000) // 1M pixels
            .processed_dimensions(500, 500) // 250K pixels
            .build();
        assert_eq!(metrics.resize_ratio(), 0.25);
    }

    #[test]
    fn test_transformation_type_labels() {
        assert_eq!(TransformationType::Resize.as_label(), "resize");
        assert_eq!(TransformationType::Crop.as_label(), "crop");
        assert_eq!(TransformationType::SmartCrop.as_label(), "smart_crop");
        assert_eq!(TransformationType::Rotate.as_label(), "rotate");
        assert_eq!(TransformationType::Flip.as_label(), "flip");
        assert_eq!(
            TransformationType::FormatConversion.as_label(),
            "format_conversion"
        );
        assert_eq!(
            TransformationType::QualityAdjustment.as_label(),
            "quality_adjustment"
        );
        assert_eq!(TransformationType::ExifRotation.as_label(), "exif_rotation");
    }

    #[test]
    fn test_cache_hit_tracking() {
        let cache_hit = ImageProcessingMetrics::builder().cache_hit(true).build();
        assert!(cache_hit.cache_hit);

        let cache_miss = ImageProcessingMetrics::builder().cache_hit(false).build();
        assert!(!cache_miss.cache_hit);
    }

    #[test]
    fn test_default_metrics() {
        let metrics = ImageProcessingMetrics::default();
        assert_eq!(metrics.original_size, 0);
        assert_eq!(metrics.processed_size, 0);
        assert_eq!(metrics.original_dimensions, (0, 0));
        assert_eq!(metrics.processed_dimensions, (0, 0));
        assert_eq!(metrics.output_format, OutputFormat::Jpeg);
        assert!(!metrics.cache_hit);
        assert!(metrics.transformations.is_empty());
    }
}
