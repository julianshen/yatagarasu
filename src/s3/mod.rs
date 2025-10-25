// S3 client module

use crate::config::S3Config;

#[derive(Debug)]
pub struct S3Client {
    #[allow(dead_code)]
    config: S3Config,
}

pub fn create_s3_client(config: &S3Config) -> Result<S3Client, String> {
    // Validate credentials are not empty
    if config.access_key.is_empty() {
        return Err("S3 access key cannot be empty".to_string());
    }
    if config.secret_key.is_empty() {
        return Err("S3 secret key cannot be empty".to_string());
    }
    if config.region.is_empty() {
        return Err("S3 region cannot be empty".to_string());
    }
    if config.bucket.is_empty() {
        return Err("S3 bucket name cannot be empty".to_string());
    }

    Ok(S3Client {
        config: config.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_s3_client_with_valid_credentials() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result = create_s3_client(&config);

        assert!(
            result.is_ok(),
            "Expected S3 client creation to succeed with valid credentials"
        );

        let client = result.unwrap();
        assert_eq!(client.config.bucket, "test-bucket");
        assert_eq!(client.config.region, "us-east-1");
        assert_eq!(client.config.access_key, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(
            client.config.secret_key,
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
    }

    #[test]
    fn test_can_create_s3_client_with_region_configuration() {
        // Test with us-east-1
        let config1 = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result1 = create_s3_client(&config1);
        assert!(result1.is_ok(), "Should create client with us-east-1");
        assert_eq!(result1.unwrap().config.region, "us-east-1");

        // Test with us-west-2
        let config2 = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-west-2".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result2 = create_s3_client(&config2);
        assert!(result2.is_ok(), "Should create client with us-west-2");
        assert_eq!(result2.unwrap().config.region, "us-west-2");

        // Test with eu-west-1
        let config3 = S3Config {
            bucket: "test-bucket".to_string(),
            region: "eu-west-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result3 = create_s3_client(&config3);
        assert!(result3.is_ok(), "Should create client with eu-west-1");
        assert_eq!(result3.unwrap().config.region, "eu-west-1");

        // Test with ap-southeast-1
        let config4 = S3Config {
            bucket: "test-bucket".to_string(),
            region: "ap-southeast-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result4 = create_s3_client(&config4);
        assert!(result4.is_ok(), "Should create client with ap-southeast-1");
        assert_eq!(result4.unwrap().config.region, "ap-southeast-1");
    }

    #[test]
    fn test_can_create_s3_client_with_custom_endpoint() {
        // Test with MinIO endpoint
        let minio_config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
        };

        let result = create_s3_client(&minio_config);
        assert!(result.is_ok(), "Should create client with MinIO endpoint");

        let client = result.unwrap();
        assert_eq!(
            client.config.endpoint,
            Some("http://localhost:9000".to_string()),
            "Endpoint should be stored correctly"
        );

        // Test with LocalStack endpoint
        let localstack_config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            endpoint: Some("http://localhost:4566".to_string()),
        };

        let result2 = create_s3_client(&localstack_config);
        assert!(
            result2.is_ok(),
            "Should create client with LocalStack endpoint"
        );

        let client2 = result2.unwrap();
        assert_eq!(
            client2.config.endpoint,
            Some("http://localhost:4566".to_string()),
            "LocalStack endpoint should be stored correctly"
        );

        // Test with HTTPS endpoint
        let https_config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            endpoint: Some("https://s3-compatible.example.com".to_string()),
        };

        let result3 = create_s3_client(&https_config);
        assert!(
            result3.is_ok(),
            "Should create client with HTTPS custom endpoint"
        );

        let client3 = result3.unwrap();
        assert_eq!(
            client3.config.endpoint,
            Some("https://s3-compatible.example.com".to_string()),
            "HTTPS endpoint should be stored correctly"
        );
    }

    #[test]
    fn test_client_creation_fails_with_empty_credentials() {
        // Test with empty access_key
        let config_empty_access_key = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result1 = create_s3_client(&config_empty_access_key);
        assert!(result1.is_err(), "Should fail with empty access_key");
        assert!(
            result1.unwrap_err().contains("access key"),
            "Error message should mention access key"
        );

        // Test with empty secret_key
        let config_empty_secret_key = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "".to_string(),
            endpoint: None,
        };

        let result2 = create_s3_client(&config_empty_secret_key);
        assert!(result2.is_err(), "Should fail with empty secret_key");
        assert!(
            result2.unwrap_err().contains("secret key"),
            "Error message should mention secret key"
        );

        // Test with empty region
        let config_empty_region = S3Config {
            bucket: "test-bucket".to_string(),
            region: "".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result3 = create_s3_client(&config_empty_region);
        assert!(result3.is_err(), "Should fail with empty region");
        assert!(
            result3.unwrap_err().contains("region"),
            "Error message should mention region"
        );

        // Test with empty bucket
        let config_empty_bucket = S3Config {
            bucket: "".to_string(),
            region: "us-east-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: None,
        };

        let result4 = create_s3_client(&config_empty_bucket);
        assert!(result4.is_err(), "Should fail with empty bucket");
        assert!(
            result4.unwrap_err().contains("bucket"),
            "Error message should mention bucket"
        );

        // Test with all empty credentials
        let config_all_empty = S3Config {
            bucket: "".to_string(),
            region: "".to_string(),
            access_key: "".to_string(),
            secret_key: "".to_string(),
            endpoint: None,
        };

        let result5 = create_s3_client(&config_all_empty);
        assert!(result5.is_err(), "Should fail with all empty credentials");
    }

    #[test]
    fn test_can_create_multiple_independent_s3_clients() {
        // Create config for products bucket
        let products_config = S3Config {
            bucket: "products-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "AKIAPRODUCTS1234567".to_string(),
            secret_key: "ProductsSecretKey123456789".to_string(),
            endpoint: None,
        };

        // Create config for users bucket
        let users_config = S3Config {
            bucket: "users-bucket".to_string(),
            region: "us-west-2".to_string(),
            access_key: "AKIAUSERS7654321ABC".to_string(),
            secret_key: "UsersSecretKeyXYZ987654321".to_string(),
            endpoint: None,
        };

        // Create config for images bucket with custom endpoint (MinIO)
        let images_config = S3Config {
            bucket: "images-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
        };

        // Create all three clients
        let products_client =
            create_s3_client(&products_config).expect("Should create products client");
        let users_client = create_s3_client(&users_config).expect("Should create users client");
        let images_client = create_s3_client(&images_config).expect("Should create images client");

        // Verify products client has correct configuration
        assert_eq!(products_client.config.bucket, "products-bucket");
        assert_eq!(products_client.config.region, "us-east-1");
        assert_eq!(products_client.config.access_key, "AKIAPRODUCTS1234567");
        assert_eq!(
            products_client.config.secret_key,
            "ProductsSecretKey123456789"
        );
        assert_eq!(products_client.config.endpoint, None);

        // Verify users client has correct configuration
        assert_eq!(users_client.config.bucket, "users-bucket");
        assert_eq!(users_client.config.region, "us-west-2");
        assert_eq!(users_client.config.access_key, "AKIAUSERS7654321ABC");
        assert_eq!(users_client.config.secret_key, "UsersSecretKeyXYZ987654321");
        assert_eq!(users_client.config.endpoint, None);

        // Verify images client has correct configuration
        assert_eq!(images_client.config.bucket, "images-bucket");
        assert_eq!(images_client.config.region, "us-east-1");
        assert_eq!(images_client.config.access_key, "minioadmin");
        assert_eq!(images_client.config.secret_key, "minioadmin");
        assert_eq!(
            images_client.config.endpoint,
            Some("http://localhost:9000".to_string())
        );

        // Verify credentials are truly independent (changing one doesn't affect others)
        // This is verified by the fact that each client maintains its own config
        assert_ne!(
            products_client.config.access_key, users_client.config.access_key,
            "Each client should have independent credentials"
        );
        assert_ne!(
            users_client.config.region, products_client.config.region,
            "Each client should have independent regions"
        );
    }
}
