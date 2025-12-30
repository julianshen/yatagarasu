//! Watermark image fetcher with caching.
//!
//! This module fetches watermark images from S3 buckets or HTTPS URLs
//! and caches them in memory for efficient reuse.
//!
//! # Supported Sources
//!
//! - `s3://bucket/key` - Fetch from S3 bucket
//! - `https://example.com/image.png` - Fetch from HTTPS URL
//!
//! # Caching
//!
//! Fetched images are cached in memory as pre-decoded RGBA images.
//! The cache uses LRU eviction with configurable TTL.
//!
//! # Example
//!
//! ```ignore
//! use yatagarasu::watermark::image_fetcher::{ImageFetcher, ImageFetcherConfig};
//!
//! let config = ImageFetcherConfig::default();
//! let fetcher = ImageFetcher::new(config);
//!
//! // Fetch and cache a watermark image
//! let image = fetcher.fetch("s3://assets/logo.png", &s3_client).await?;
//! ```

use super::WatermarkError;
use aws_sdk_s3::Client as S3Client;
use image::{DynamicImage, ImageFormat};
use moka::future::Cache;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

/// Configuration for the image fetcher.
#[derive(Debug, Clone)]
pub struct ImageFetcherConfig {
    /// Maximum number of cached images.
    pub max_cache_entries: u64,
    /// Time-to-live for cached images.
    pub cache_ttl: Duration,
}

impl Default for ImageFetcherConfig {
    fn default() -> Self {
        Self {
            max_cache_entries: 100,
            cache_ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Parsed source location for watermark images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    /// S3 bucket source: bucket name and object key.
    S3 { bucket: String, key: String },
    /// HTTPS URL source.
    Https(String),
}

impl ImageSource {
    /// Parse a source string into an ImageSource.
    ///
    /// Supports:
    /// - `s3://bucket/path/to/key` - S3 bucket reference
    /// - `https://example.com/path/to/image.png` - HTTPS URL
    ///
    /// # Errors
    ///
    /// Returns error if the source format is invalid or uses unsupported protocol.
    pub fn parse(source: &str) -> Result<Self, WatermarkError> {
        if let Some(rest) = source.strip_prefix("s3://") {
            // Parse s3://bucket/key format
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
                return Err(WatermarkError::FetchError(format!(
                    "Invalid S3 source format: {source}. Expected s3://bucket/key"
                )));
            }
            Ok(ImageSource::S3 {
                bucket: parts[0].to_string(),
                key: parts[1].to_string(),
            })
        } else if source.starts_with("https://") {
            Ok(ImageSource::Https(source.to_string()))
        } else {
            Err(WatermarkError::FetchError(format!(
                "Unsupported source protocol: {source}. Use s3:// or https://"
            )))
        }
    }

    /// Get a cache key for this source.
    pub fn cache_key(&self) -> String {
        match self {
            ImageSource::S3 { bucket, key } => format!("s3://{bucket}/{key}"),
            ImageSource::Https(url) => url.clone(),
        }
    }
}

/// Cached watermark image with metadata.
#[derive(Clone)]
pub struct CachedImage {
    /// The decoded RGBA image.
    pub image: Arc<DynamicImage>,
}

impl std::fmt::Debug for CachedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedImage")
            .field("dimensions", &(self.image.width(), self.image.height()))
            .finish()
    }
}

impl CachedImage {
    /// Create a new cached image.
    pub fn new(image: DynamicImage) -> Self {
        Self {
            image: Arc::new(image),
        }
    }
}

/// Fetcher for watermark images with built-in caching.
#[derive(Clone)]
pub struct ImageFetcher {
    cache: Cache<String, CachedImage>,
    http_client: reqwest::Client,
}

impl ImageFetcher {
    /// Create a new image fetcher with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `WatermarkError::ConfigError` if the HTTP client cannot be created
    /// (e.g., TLS configuration issues, system resource exhaustion).
    pub fn new(config: ImageFetcherConfig) -> Result<Self, WatermarkError> {
        let cache = Cache::builder()
            .max_capacity(config.max_cache_entries)
            .time_to_live(config.cache_ttl)
            .build();

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                WatermarkError::ConfigError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { cache, http_client })
    }

    /// Fetch an image from the given source.
    ///
    /// Images are cached after first fetch. Subsequent calls with the same
    /// source will return the cached image until TTL expires.
    ///
    /// # Arguments
    ///
    /// * `source` - Source string (s3://bucket/key or https://...)
    /// * `s3_client` - Optional S3 client for fetching from S3. Required for s3:// sources.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Source format is invalid
    /// - S3 client not provided for s3:// source
    /// - Network or S3 fetch fails
    /// - Image decoding fails
    pub async fn fetch(
        &self,
        source: &str,
        s3_client: Option<&S3Client>,
    ) -> Result<CachedImage, WatermarkError> {
        let parsed = ImageSource::parse(source)?;
        let cache_key = parsed.cache_key();

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        // Fetch from source
        let image = match &parsed {
            ImageSource::S3 { bucket, key } => {
                let client = s3_client.ok_or_else(|| {
                    WatermarkError::FetchError("S3 client required for s3:// sources".to_string())
                })?;
                self.fetch_from_s3(client, bucket, key).await?
            }
            ImageSource::Https(url) => self.fetch_from_https(url).await?,
        };

        let cached = CachedImage::new(image);

        // Store in cache
        self.cache.insert(cache_key, cached.clone()).await;

        Ok(cached)
    }

    /// Fetch image from S3 bucket.
    async fn fetch_from_s3(
        &self,
        client: &S3Client,
        bucket: &str,
        key: &str,
    ) -> Result<DynamicImage, WatermarkError> {
        let response = client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| WatermarkError::FetchError(format!("S3 fetch failed: {e}")))?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| WatermarkError::FetchError(format!("Failed to read S3 body: {e}")))?;

        let data = bytes.into_bytes();

        // Detect format from bytes or key extension
        let format = detect_image_format(&data, key)?;

        image::load(Cursor::new(data), format)
            .map_err(|e| WatermarkError::FetchError(format!("Failed to decode image: {e}")))
    }

    /// Fetch image from HTTPS URL.
    async fn fetch_from_https(&self, url: &str) -> Result<DynamicImage, WatermarkError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| WatermarkError::FetchError(format!("HTTP fetch failed: {e}")))?;

        if !response.status().is_success() {
            return Err(WatermarkError::FetchError(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| WatermarkError::FetchError(format!("Failed to read HTTP body: {e}")))?;

        // Detect format from bytes or URL extension
        let format = detect_image_format(&bytes, url)?;

        image::load(Cursor::new(bytes), format)
            .map_err(|e| WatermarkError::FetchError(format!("Failed to decode image: {e}")))
    }

    /// Get the number of cached images.
    pub fn cache_size(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Clear all cached images.
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    /// Check if an image is cached.
    pub async fn is_cached(&self, source: &str) -> bool {
        if let Ok(parsed) = ImageSource::parse(source) {
            self.cache.get(&parsed.cache_key()).await.is_some()
        } else {
            false
        }
    }
}

/// Detect image format from bytes or filename extension.
fn detect_image_format(data: &[u8], path: &str) -> Result<ImageFormat, WatermarkError> {
    // Try to detect from magic bytes first
    if let Ok(format) = image::guess_format(data) {
        return Ok(format);
    }

    // Fall back to extension
    let ext = path
        .rsplit('.')
        .next()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "png" => Ok(ImageFormat::Png),
        "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
        "gif" => Ok(ImageFormat::Gif),
        "webp" => Ok(ImageFormat::WebP),
        _ => Err(WatermarkError::FetchError(format!(
            "Unsupported image format: {ext}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test: Parse s3://bucket/key format
    #[test]
    fn test_parse_s3_source() {
        let source = ImageSource::parse("s3://my-bucket/path/to/logo.png").unwrap();
        assert_eq!(
            source,
            ImageSource::S3 {
                bucket: "my-bucket".to_string(),
                key: "path/to/logo.png".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_s3_source_simple_key() {
        let source = ImageSource::parse("s3://bucket/logo.png").unwrap();
        assert_eq!(
            source,
            ImageSource::S3 {
                bucket: "bucket".to_string(),
                key: "logo.png".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_s3_source_deep_path() {
        let source = ImageSource::parse("s3://assets/watermarks/v2/logo.png").unwrap();
        assert_eq!(
            source,
            ImageSource::S3 {
                bucket: "assets".to_string(),
                key: "watermarks/v2/logo.png".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_s3_invalid_no_key() {
        let result = ImageSource::parse("s3://bucket/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_s3_invalid_empty_bucket() {
        let result = ImageSource::parse("s3:///key");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_s3_invalid_no_slash() {
        let result = ImageSource::parse("s3://bucket");
        assert!(result.is_err());
    }

    // Test: Parse https:// URL
    #[test]
    fn test_parse_https_source() {
        let source = ImageSource::parse("https://example.com/logo.png").unwrap();
        assert_eq!(
            source,
            ImageSource::Https("https://example.com/logo.png".to_string())
        );
    }

    #[test]
    fn test_parse_https_with_query_params() {
        let source = ImageSource::parse("https://cdn.example.com/img/logo.png?v=2").unwrap();
        assert_eq!(
            source,
            ImageSource::Https("https://cdn.example.com/img/logo.png?v=2".to_string())
        );
    }

    // Test: Reject http:// (security)
    #[test]
    fn test_parse_http_rejected() {
        let result = ImageSource::parse("http://example.com/logo.png");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported source protocol"));
    }

    // Test: Reject unknown protocols
    #[test]
    fn test_parse_unknown_protocol_rejected() {
        let result = ImageSource::parse("ftp://example.com/logo.png");
        assert!(result.is_err());

        let result = ImageSource::parse("file:///path/to/logo.png");
        assert!(result.is_err());
    }

    // Test: Cache key generation
    #[test]
    fn test_cache_key_s3() {
        let source = ImageSource::S3 {
            bucket: "my-bucket".to_string(),
            key: "path/to/logo.png".to_string(),
        };
        assert_eq!(source.cache_key(), "s3://my-bucket/path/to/logo.png");
    }

    #[test]
    fn test_cache_key_https() {
        let source = ImageSource::Https("https://example.com/logo.png".to_string());
        assert_eq!(source.cache_key(), "https://example.com/logo.png");
    }

    // Test: Image format detection
    #[test]
    fn test_detect_format_from_extension() {
        assert!(matches!(
            detect_image_format(&[], "logo.png"),
            Ok(ImageFormat::Png)
        ));
        assert!(matches!(
            detect_image_format(&[], "photo.jpg"),
            Ok(ImageFormat::Jpeg)
        ));
        assert!(matches!(
            detect_image_format(&[], "photo.jpeg"),
            Ok(ImageFormat::Jpeg)
        ));
        assert!(matches!(
            detect_image_format(&[], "anim.gif"),
            Ok(ImageFormat::Gif)
        ));
        assert!(matches!(
            detect_image_format(&[], "modern.webp"),
            Ok(ImageFormat::WebP)
        ));
    }

    #[test]
    fn test_detect_format_unknown_extension() {
        let result = detect_image_format(&[], "file.bmp");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_format_from_png_magic_bytes() {
        // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(matches!(
            detect_image_format(&png_bytes, "noext"),
            Ok(ImageFormat::Png)
        ));
    }

    #[test]
    fn test_detect_format_from_jpeg_magic_bytes() {
        // JPEG magic bytes: FF D8 FF
        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert!(matches!(
            detect_image_format(&jpeg_bytes, "noext"),
            Ok(ImageFormat::Jpeg)
        ));
    }

    // Test: Fetcher configuration
    #[test]
    fn test_fetcher_config_default() {
        let config = ImageFetcherConfig::default();
        assert_eq!(config.max_cache_entries, 100);
        assert_eq!(config.cache_ttl, Duration::from_secs(3600));
    }

    // Test: Fetcher creation
    #[test]
    fn test_fetcher_creation() {
        let config = ImageFetcherConfig {
            max_cache_entries: 50,
            cache_ttl: Duration::from_secs(1800),
        };
        let fetcher = ImageFetcher::new(config).expect("should create fetcher");
        assert_eq!(fetcher.cache_size(), 0);
    }

    // Integration tests would require mocking S3 and HTTP servers
    // These are marked for integration testing

    #[tokio::test]
    async fn test_cache_clear() {
        let fetcher =
            ImageFetcher::new(ImageFetcherConfig::default()).expect("should create fetcher");

        // Cache should start empty
        assert_eq!(fetcher.cache_size(), 0);

        // Clear should work on empty cache
        fetcher.clear_cache().await;
        assert_eq!(fetcher.cache_size(), 0);
    }

    #[tokio::test]
    async fn test_is_cached_returns_false_for_uncached() {
        let fetcher =
            ImageFetcher::new(ImageFetcherConfig::default()).expect("should create fetcher");

        assert!(!fetcher.is_cached("s3://bucket/logo.png").await);
        assert!(!fetcher.is_cached("https://example.com/logo.png").await);
    }

    #[tokio::test]
    async fn test_fetch_requires_s3_client_for_s3_source() {
        let fetcher =
            ImageFetcher::new(ImageFetcherConfig::default()).expect("should create fetcher");

        // Trying to fetch S3 source without client should error
        let result = fetcher.fetch("s3://bucket/logo.png", None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("S3 client required"));
    }
}
