// S3 client module

use crate::config::S3Config;

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
}
