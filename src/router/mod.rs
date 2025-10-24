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
            .find(|bucket| normalized_path.starts_with(&bucket.path_prefix))
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

    #[test]
    fn test_router_matches_path_with_trailing_segments() {
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

        let result = router.route("/products/item.txt");

        assert!(
            result.is_some(),
            "Expected to match /products/item.txt path"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");
        assert_eq!(matched_bucket.path_prefix, "/products");
    }

    #[test]
    fn test_router_returns_none_for_unmapped_path() {
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

        let result = router.route("/unmapped");

        assert!(result.is_none(), "Expected no match for /unmapped path");
    }

    #[test]
    fn test_router_returns_correct_bucket_for_first_matching_prefix() {
        let bucket1 = BucketConfig {
            name: "images".to_string(),
            path_prefix: "/images".to_string(),
            s3: S3Config {
                bucket: "my-images-bucket".to_string(),
                region: "us-east-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
            },
        };
        let bucket2 = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "my-products-bucket".to_string(),
                region: "us-west-2".to_string(),
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
        let router = Router::new(buckets);

        let result = router.route("/products/item.txt");

        assert!(result.is_some(), "Expected to match /products/item.txt");
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");
        assert_eq!(matched_bucket.path_prefix, "/products");
    }

    #[test]
    fn test_router_handles_path_without_leading_slash() {
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

        let result = router.route("products");

        assert!(
            result.is_none(),
            "Expected to reject path without leading slash"
        );
    }

    #[test]
    fn test_normalizes_paths_with_double_slashes() {
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

        // Path with double slashes in the middle should be normalized and match
        let result = router.route("/products//item.txt");
        assert!(
            result.is_some(),
            "Expected to normalize and match /products//item.txt"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");

        // Path with double slashes at the beginning should be normalized and match
        let result2 = router.route("//products/item.txt");
        assert!(
            result2.is_some(),
            "Expected to normalize and match //products/item.txt"
        );
        let matched_bucket2 = result2.unwrap();
        assert_eq!(matched_bucket2.name, "products");
    }

    #[test]
    fn test_normalizes_paths_with_trailing_slash() {
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

        // Path with single trailing slash should match and be normalized
        let result = router.route("/products/");
        assert!(
            result.is_some(),
            "Expected to match /products/ (with trailing slash)"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");

        // Path with multiple trailing slashes should be normalized
        let result2 = router.route("/products/item.txt///");
        assert!(
            result2.is_some(),
            "Expected to normalize and match /products/item.txt///"
        );
        let matched_bucket2 = result2.unwrap();
        assert_eq!(matched_bucket2.name, "products");
    }

    #[test]
    fn test_handles_url_encoded_paths_correctly() {
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

        // URL-encoded space (%20) should be decoded
        let result = router.route("/products/my%20item.txt");
        assert!(
            result.is_some(),
            "Expected to decode and match /products/my%20item.txt"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");

        // URL-encoded special characters should be decoded
        let result2 = router.route("/products/item%2Btest.txt");
        assert!(
            result2.is_some(),
            "Expected to decode and match /products/item%2Btest.txt"
        );
        let matched_bucket2 = result2.unwrap();
        assert_eq!(matched_bucket2.name, "products");

        // URL-encoded forward slash (%2F) should be decoded (but not used for path separation)
        let result3 = router.route("/products/folder%2Fitem.txt");
        assert!(
            result3.is_some(),
            "Expected to decode and match /products/folder%2Fitem.txt"
        );
        let matched_bucket3 = result3.unwrap();
        assert_eq!(matched_bucket3.name, "products");
    }

    #[test]
    fn test_handles_special_characters_in_paths() {
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

        // Hyphen and underscore
        let result = router.route("/products/my-file_name.txt");
        assert!(
            result.is_some(),
            "Expected to match path with hyphen and underscore"
        );
        assert_eq!(result.unwrap().name, "products");

        // Dots in filename
        let result2 = router.route("/products/file.backup.txt");
        assert!(
            result2.is_some(),
            "Expected to match path with multiple dots"
        );
        assert_eq!(result2.unwrap().name, "products");

        // Tilde
        let result3 = router.route("/products/~backup/file.txt");
        assert!(result3.is_some(), "Expected to match path with tilde");
        assert_eq!(result3.unwrap().name, "products");

        // Parentheses and brackets
        let result4 = router.route("/products/file(1)[copy].txt");
        assert!(
            result4.is_some(),
            "Expected to match path with parentheses and brackets"
        );
        assert_eq!(result4.unwrap().name, "products");

        // At symbol and plus
        let result5 = router.route("/products/user@email+tag.txt");
        assert!(
            result5.is_some(),
            "Expected to match path with @ and + symbols"
        );
        assert_eq!(result5.unwrap().name, "products");
    }

    #[test]
    fn test_preserves_case_sensitivity_in_paths() {
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

        // Exact case match should succeed
        let result = router.route("/products/item.txt");
        assert!(
            result.is_some(),
            "Expected to match path with exact case /products"
        );
        assert_eq!(result.unwrap().name, "products");

        // Different case should not match
        let result2 = router.route("/Products/item.txt");
        assert!(
            result2.is_none(),
            "Expected to NOT match path with different case /Products"
        );

        let result3 = router.route("/PRODUCTS/item.txt");
        assert!(
            result3.is_none(),
            "Expected to NOT match path with different case /PRODUCTS"
        );

        // Case sensitivity should apply to the entire path
        let result4 = router.route("/products/Item.txt");
        assert!(
            result4.is_some(),
            "Expected to match prefix but preserve case in filename"
        );
        assert_eq!(result4.unwrap().name, "products");
    }
}
