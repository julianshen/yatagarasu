// Router module

use crate::config::BucketConfig;

pub struct Router {
    buckets: Vec<BucketConfig>,
}

impl Router {
    pub fn new(buckets: Vec<BucketConfig>) -> Self {
        Router { buckets }
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
    /// This is an O(n) linear search through buckets.
    /// Use `route()` for path-based lookups which is the primary use case.
    pub fn get_bucket_by_name(&self, name: &str) -> Option<&BucketConfig> {
        self.buckets.iter().find(|b| b.name == name)
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
