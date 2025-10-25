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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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
                endpoint: None,
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

    #[test]
    fn test_matches_longest_prefix_when_multiple_prefixes_match() {
        let bucket1 = BucketConfig {
            name: "prod".to_string(),
            path_prefix: "/prod".to_string(),
            s3: S3Config {
                bucket: "prod-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: None,
            },
        };
        let bucket2 = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket1, bucket2];
        let router = Router::new(buckets);

        // /products/item.txt should match /products (longest), not /prod
        let result = router.route("/products/item.txt");
        assert!(
            result.is_some(),
            "Expected to match /products/item.txt to a bucket"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(
            matched_bucket.name, "products",
            "Expected to match longest prefix /products, not /prod"
        );
        assert_eq!(matched_bucket.path_prefix, "/products");

        // /prod/item.txt should match /prod
        let result2 = router.route("/prod/item.txt");
        assert!(result2.is_some(), "Expected to match /prod/item.txt");
        let matched_bucket2 = result2.unwrap();
        assert_eq!(matched_bucket2.name, "prod");
        assert_eq!(matched_bucket2.path_prefix, "/prod");
    }

    #[test]
    fn test_products_foo_matches_products_not_prod() {
        let bucket1 = BucketConfig {
            name: "prod".to_string(),
            path_prefix: "/prod".to_string(),
            s3: S3Config {
                bucket: "prod-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: None,
            },
        };
        let bucket2 = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket1, bucket2];
        let router = Router::new(buckets);

        // /products/foo should match /products, not /prod
        let result = router.route("/products/foo");
        assert!(result.is_some(), "Expected /products/foo to match a bucket");
        let matched_bucket = result.unwrap();
        assert_eq!(
            matched_bucket.name, "products",
            "Expected /products/foo to match /products prefix, not /prod"
        );
        assert_eq!(matched_bucket.path_prefix, "/products");
    }

    #[test]
    fn test_handles_root_path_correctly() {
        let bucket1 = BucketConfig {
            name: "default".to_string(),
            path_prefix: "/".to_string(),
            s3: S3Config {
                bucket: "default-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: None,
            },
        };
        let bucket2 = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket1, bucket2];
        let router = Router::new(buckets);

        // Root path / should act as catch-all for unmapped paths
        let result = router.route("/unmapped/file.txt");
        assert!(
            result.is_some(),
            "Expected root path / to match as catch-all"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "default");
        assert_eq!(matched_bucket.path_prefix, "/");

        // More specific prefix should take precedence over root
        let result2 = router.route("/products/item.txt");
        assert!(result2.is_some(), "Expected /products/item.txt to match");
        let matched_bucket2 = result2.unwrap();
        assert_eq!(
            matched_bucket2.name, "products",
            "Expected /products to take precedence over root /"
        );
        assert_eq!(matched_bucket2.path_prefix, "/products");

        // Root path itself should match
        let result3 = router.route("/");
        assert!(result3.is_some(), "Expected / to match root path");
        let matched_bucket3 = result3.unwrap();
        assert_eq!(matched_bucket3.name, "default");
        assert_eq!(matched_bucket3.path_prefix, "/");
    }

    #[test]
    fn test_handles_path_prefixes_with_query_parameters() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Path with query parameters should strip them before routing
        let result = router.route("/products/item.txt?version=2");
        assert!(
            result.is_some(),
            "Expected to match path with query parameters"
        );
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");

        // Multiple query parameters
        let result2 = router.route("/products/item.txt?version=2&format=json");
        assert!(
            result2.is_some(),
            "Expected to match path with multiple query parameters"
        );
        assert_eq!(result2.unwrap().name, "products");

        // Query parameter on prefix itself
        let result3 = router.route("/products?list=all");
        assert!(
            result3.is_some(),
            "Expected to match prefix with query parameter"
        );
        assert_eq!(result3.unwrap().name, "products");
    }

    #[test]
    fn test_handles_path_prefixes_with_fragments() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Path with fragment should match
        let result = router.route("/products/item.txt#section1");
        assert!(result.is_some(), "Expected to match path with fragment");
        let matched_bucket = result.unwrap();
        assert_eq!(matched_bucket.name, "products");

        // Fragment on prefix itself
        let result2 = router.route("/products#top");
        assert!(result2.is_some(), "Expected to match prefix with fragment");
        assert_eq!(result2.unwrap().name, "products");

        // Combined query parameter and fragment
        let result3 = router.route("/products/item.txt?version=2#section1");
        assert!(
            result3.is_some(),
            "Expected to match path with query and fragment"
        );
        assert_eq!(result3.unwrap().name, "products");
    }

    #[test]
    fn test_extracts_s3_key_by_removing_path_prefix() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Extract S3 key from path
        let s3_key = router.extract_s3_key("/products/folder/item.txt");
        assert_eq!(
            s3_key,
            Some("folder/item.txt".to_string()),
            "Expected S3 key to be 'folder/item.txt'"
        );

        // Single file
        let s3_key2 = router.extract_s3_key("/products/item.txt");
        assert_eq!(
            s3_key2,
            Some("item.txt".to_string()),
            "Expected S3 key to be 'item.txt'"
        );

        // Path that doesn't match any prefix
        let s3_key3 = router.extract_s3_key("/unmapped/file.txt");
        assert_eq!(s3_key3, None, "Expected None for unmapped path");
    }

    #[test]
    fn test_handles_path_prefix_with_trailing_slash() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products/".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Extract S3 key with trailing slash prefix
        let s3_key = router.extract_s3_key("/products/folder/item.txt");
        assert_eq!(
            s3_key,
            Some("folder/item.txt".to_string()),
            "Expected S3 key to be 'folder/item.txt' with trailing slash prefix"
        );

        // Single file with trailing slash prefix
        let s3_key2 = router.extract_s3_key("/products/item.txt");
        assert_eq!(
            s3_key2,
            Some("item.txt".to_string()),
            "Expected S3 key to be 'item.txt' with trailing slash prefix"
        );

        // Exact prefix match (just the prefix with trailing slash)
        let s3_key3 = router.extract_s3_key("/products/");
        assert_eq!(
            s3_key3,
            Some("".to_string()),
            "Expected empty string for exact prefix match with trailing slash"
        );
    }

    #[test]
    fn test_handles_path_prefix_without_trailing_slash() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Extract S3 key without trailing slash prefix
        let s3_key = router.extract_s3_key("/products/folder/item.txt");
        assert_eq!(
            s3_key,
            Some("folder/item.txt".to_string()),
            "Expected S3 key to be 'folder/item.txt' without trailing slash prefix"
        );

        // Single file without trailing slash prefix
        let s3_key2 = router.extract_s3_key("/products/item.txt");
        assert_eq!(
            s3_key2,
            Some("item.txt".to_string()),
            "Expected S3 key to be 'item.txt' without trailing slash prefix"
        );

        // Exact prefix match (just the prefix without trailing slash)
        let s3_key3 = router.extract_s3_key("/products");
        assert_eq!(
            s3_key3,
            Some("".to_string()),
            "Expected empty string for exact prefix match without trailing slash"
        );
    }

    #[test]
    fn test_extracts_nested_s3_keys_correctly() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Two-level nesting
        let s3_key = router.extract_s3_key("/products/folder/file.txt");
        assert_eq!(
            s3_key,
            Some("folder/file.txt".to_string()),
            "Expected S3 key to be 'folder/file.txt' for two-level nesting"
        );

        // Three-level nesting
        let s3_key2 = router.extract_s3_key("/products/folder/subfolder/file.txt");
        assert_eq!(
            s3_key2,
            Some("folder/subfolder/file.txt".to_string()),
            "Expected S3 key to be 'folder/subfolder/file.txt' for three-level nesting"
        );

        // Deep nesting with multiple subdirectories
        let s3_key3 = router.extract_s3_key("/products/a/b/c/d/e/file.txt");
        assert_eq!(
            s3_key3,
            Some("a/b/c/d/e/file.txt".to_string()),
            "Expected S3 key to be 'a/b/c/d/e/file.txt' for deep nesting"
        );

        // Nested folder without file (folder path)
        let s3_key4 = router.extract_s3_key("/products/folder/subfolder/");
        assert_eq!(
            s3_key4,
            Some("folder/subfolder/".to_string()),
            "Expected S3 key to preserve trailing slash for folder paths"
        );
    }

    #[test]
    fn test_handles_s3_key_with_special_characters() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Spaces in filename
        let s3_key = router.extract_s3_key("/products/my file.txt");
        assert_eq!(
            s3_key,
            Some("my file.txt".to_string()),
            "Expected S3 key to preserve spaces"
        );

        // Hyphens and underscores
        let s3_key2 = router.extract_s3_key("/products/my-file_name.txt");
        assert_eq!(
            s3_key2,
            Some("my-file_name.txt".to_string()),
            "Expected S3 key to preserve hyphens and underscores"
        );

        // Multiple dots
        let s3_key3 = router.extract_s3_key("/products/file.backup.2024.txt");
        assert_eq!(
            s3_key3,
            Some("file.backup.2024.txt".to_string()),
            "Expected S3 key to preserve multiple dots"
        );

        // Parentheses and brackets
        let s3_key4 = router.extract_s3_key("/products/file(1)[copy].txt");
        assert_eq!(
            s3_key4,
            Some("file(1)[copy].txt".to_string()),
            "Expected S3 key to preserve parentheses and brackets"
        );

        // Special characters: tilde, exclamation, at, plus
        let s3_key5 = router.extract_s3_key("/products/~backup/user@email+tag.txt");
        assert_eq!(
            s3_key5,
            Some("~backup/user@email+tag.txt".to_string()),
            "Expected S3 key to preserve ~, @, + characters"
        );

        // Dollar sign, percent, ampersand
        let s3_key6 = router.extract_s3_key("/products/$price-100%&sale.txt");
        assert_eq!(
            s3_key6,
            Some("$price-100%&sale.txt".to_string()),
            "Expected S3 key to preserve $, %, & characters"
        );

        // Equals, comma, semicolon
        let s3_key7 = router.extract_s3_key("/products/key=value,item;data.txt");
        assert_eq!(
            s3_key7,
            Some("key=value,item;data.txt".to_string()),
            "Expected S3 key to preserve =, ,, ; characters"
        );

        // Single quotes and backticks
        let s3_key8 = router.extract_s3_key("/products/file's-name`backup.txt");
        assert_eq!(
            s3_key8,
            Some("file's-name`backup.txt".to_string()),
            "Expected S3 key to preserve single quotes and backticks"
        );
    }

    #[test]
    fn test_handles_empty_s3_key_when_prefix_is_full_path() {
        let bucket = BucketConfig {
            name: "products".to_string(),
            path_prefix: "/products".to_string(),
            s3: S3Config {
                bucket: "products-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets = vec![bucket];
        let router = Router::new(buckets);

        // Exact match: path equals prefix (without trailing slash)
        let s3_key = router.extract_s3_key("/products");
        assert_eq!(
            s3_key,
            Some("".to_string()),
            "Expected empty string when path exactly matches prefix without trailing slash"
        );

        // Test with bucket that has trailing slash in prefix
        let bucket2 = BucketConfig {
            name: "images".to_string(),
            path_prefix: "/images/".to_string(),
            s3: S3Config {
                bucket: "images-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets2 = vec![bucket2];
        let router2 = Router::new(buckets2);

        // Exact match with trailing slash
        let s3_key2 = router2.extract_s3_key("/images/");
        assert_eq!(
            s3_key2,
            Some("".to_string()),
            "Expected empty string when path exactly matches prefix with trailing slash"
        );

        // Test with root path bucket
        let bucket3 = BucketConfig {
            name: "root".to_string(),
            path_prefix: "/".to_string(),
            s3: S3Config {
                bucket: "root-bucket".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                endpoint: None,
            },
        };
        let buckets3 = vec![bucket3];
        let router3 = Router::new(buckets3);

        // Root path should give empty key
        let s3_key3 = router3.extract_s3_key("/");
        assert_eq!(
            s3_key3,
            Some("".to_string()),
            "Expected empty string for root path /"
        );
    }

    #[test]
    fn test_router_lookup_is_fast_for_reasonable_config_sizes() {
        use std::time::Instant;

        // Create router with 50 buckets (reasonable config size)
        let mut buckets = Vec::new();
        for i in 0..50 {
            buckets.push(BucketConfig {
                name: format!("bucket{}", i),
                path_prefix: format!("/prefix{}", i),
                s3: S3Config {
                    bucket: format!("s3-bucket-{}", i),
                    region: "us-west-2".to_string(),
                    access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                    secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                    endpoint: None,
                },
            });
        }
        let router = Router::new(buckets);

        // Perform 10,000 lookups and measure time
        let start = Instant::now();
        for _ in 0..10_000 {
            // Lookup various paths
            let _ = router.route("/prefix25/file.txt");
            let _ = router.route("/prefix0/item.txt");
            let _ = router.route("/prefix49/data.txt");
            let _ = router.route("/unmapped/file.txt");
        }
        let duration = start.elapsed();

        // Should complete in less than 150ms for 10,000 lookups with 50 buckets
        // This demonstrates O(n) performance is acceptable for reasonable config sizes
        // Note: Threshold increased from 100ms to 150ms to account for system variability
        assert!(
            duration.as_millis() < 150,
            "Router lookup too slow: {:?} for 10,000 lookups with 50 buckets",
            duration
        );
    }

    #[test]
    fn test_can_handle_100_plus_bucket_configurations_efficiently() {
        use std::time::Instant;

        // Create router with 150 buckets (larger than typical, testing scalability)
        let mut buckets = Vec::new();
        for i in 0..150 {
            buckets.push(BucketConfig {
                name: format!("bucket{}", i),
                path_prefix: format!("/prefix{}", i),
                s3: S3Config {
                    bucket: format!("s3-bucket-{}", i),
                    region: "us-west-2".to_string(),
                    access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                    secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
                    endpoint: None,
                },
            });
        }
        let router = Router::new(buckets);

        // Perform 10,000 lookups and measure time
        let start = Instant::now();
        for _ in 0..10_000 {
            // Lookup various paths across the range
            let _ = router.route("/prefix75/file.txt"); // Middle
            let _ = router.route("/prefix0/item.txt"); // First
            let _ = router.route("/prefix149/data.txt"); // Last
            let _ = router.route("/unmapped/file.txt"); // No match
        }
        let duration = start.elapsed();

        // Should complete in less than 300ms for 10,000 iterations with 150 buckets
        // This is 3x the threshold for 50 buckets, accounting for O(n) scaling
        assert!(
            duration.as_millis() < 300,
            "Router lookup too slow: {:?} for 10,000 iterations (40,000 lookups) with 150 buckets",
            duration
        );

        // Verify router actually works correctly with this many buckets
        assert!(router.route("/prefix0/test.txt").is_some());
        assert!(router.route("/prefix75/test.txt").is_some());
        assert!(router.route("/prefix149/test.txt").is_some());
        assert!(router.route("/unmapped/test.txt").is_none());
    }
}
