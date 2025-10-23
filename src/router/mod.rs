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
        self.buckets
            .iter()
            .find(|bucket| path == bucket.path_prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BucketConfig, S3Config};

    #[test]
    fn test_can_create_router_with_empty_bucket_list() {
        let buckets: Vec<BucketConfig> = vec![];
        let _router = Router::new(buckets);

        // Router should be created successfully even with empty bucket list
        // (The fact that we reach this assertion means router was created successfully)
    }

    #[test]
    fn test_can_create_router_with_single_bucket_config() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "my-products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            },
        };
        let buckets = vec![bucket];
        let _router = Router::new(buckets);

        // Router should be created successfully with single bucket config
    }

    #[test]
    fn test_can_create_router_with_multiple_bucket_configs() {
        let bucket1 = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "my-products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
            },
        };
        let bucket2 = BucketConfig {
            name: "images".to_string(),
            path_prefix: "/images".to_string(),
            s3: S3Config {
                bucket: "my-images-bucket".to_string(),
                region: "us-east-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
            },
        };
        let bucket3 = BucketConfig {
            name: "documents".to_string(),
            path_prefix: "/documents".to_string(),
            s3: S3Config {
                bucket: "my-documents-bucket".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
            },
        };
        let buckets = vec![bucket1, bucket2, bucket3];
        let _router = Router::new(buckets);

        // Router should be created successfully with multiple bucket configs
    }

    #[test]
    fn test_router_matches_exact_path_prefix() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "my-products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        let result = router.route("/products");

        assert!(result.is_some(), "Expected to match /products path");
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");
        assert_eq!(matched_bucket.path_prefix, "/products");
    }
}
