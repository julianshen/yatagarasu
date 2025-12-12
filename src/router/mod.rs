//! Request Router Module
//!
//! Routes incoming requests to the appropriate S3 bucket based on URL path prefixes.
//! Uses longest-prefix matching to select the most specific bucket configuration.
//!
//! # Routing Algorithm
//!
//! 1. Normalize the request path (collapse duplicate slashes like `//`)
//! 2. Find all buckets whose `path_prefix` matches the start of the path
//! 3. Select the bucket with the longest matching prefix (most specific)
//! 4. Strip the matched prefix from the path for the upstream S3 request
//!
//! # Example
//!
//! Given buckets configured with prefixes:
//! - `/api/v1/` → bucket-a
//! - `/api/` → bucket-b
//! - `/` → bucket-default
//!
//! Request paths are routed as:
//! - `/api/v1/users` → bucket-a (stripped path: `users`)
//! - `/api/docs` → bucket-b (stripped path: `docs`)
//! - `/images/logo.png` → bucket-default (stripped path: `images/logo.png`)
//!
//! # Performance
//!
//! - O(n) routing where n = number of configured buckets
//! - O(1) bucket lookup by name via HashMap index

use crate::config::BucketConfig;
use std::collections::HashMap;

pub struct Router {
    buckets: Vec<BucketConfig>,
    /// Index for O(1) bucket lookup by name
    bucket_by_name: HashMap<String, usize>,
}

impl Router {
    pub fn new(buckets: Vec<BucketConfig>) -> Self {
        let bucket_by_name: HashMap<String, usize> = buckets
            .iter()
            .enumerate()
            .map(|(idx, bucket)| (bucket.name.clone(), idx))
            .collect();

        Router {
            buckets,
            bucket_by_name,
        }
    }

    pub fn route(&self, path: &str) -> Option<&BucketConfig> {
        let normalized_path = Self::normalize_path(path);
        self.buckets
            .iter()
            .filter(|bucket| normalized_path.starts_with(&bucket.path_prefix))
            .max_by_key(|bucket| bucket.path_prefix.len())
    }

    pub fn extract_s3_key(&self, path: &str) -> Option<String> {
        let normalized_path = Self::normalize_path(path);
        let bucket = self.route(path)?;

        // Remove the prefix from the path
        let key = normalized_path.strip_prefix(&bucket.path_prefix)?;

        // Remove leading slash if present
        let key = key.strip_prefix('/').unwrap_or(key);

        Some(key.to_string())
    }

    /// Get a bucket configuration by name
    ///
    /// This is an O(1) lookup using a HashMap index.
    pub fn get_bucket_by_name(&self, name: &str) -> Option<&BucketConfig> {
        self.bucket_by_name.get(name).map(|&idx| &self.buckets[idx])
    }

    fn normalize_path(path: &str) -> String {
        let mut result = String::new();
        let mut prev_was_slash = false;

        for ch in path.chars() {
            if ch == '/' {
                if !prev_was_slash {
                    result.push(ch);
                    prev_was_slash = true;
                }
            } else {
                result.push(ch);
                prev_was_slash = false;
            }
        }

        result
    }
}
