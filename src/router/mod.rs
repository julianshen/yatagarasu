// Router module

use crate::config::BucketConfig;

pub struct Router {
    #[allow(dead_code)]
    buckets: Vec<BucketConfig>,
}

impl Router {
    pub fn new(buckets: Vec<BucketConfig>) -> Self {
        Router { buckets }
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
}
