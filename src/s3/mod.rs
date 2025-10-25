// S3 client module

use crate::config::S3Config;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

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

// AWS Signature v4 implementation
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub struct SigningParams<'a> {
    pub method: &'a str,
    pub uri: &'a str,
    pub query_string: &'a str,
    pub headers: &'a std::collections::HashMap<String, String>,
    pub payload: &'a [u8],
    pub access_key: &'a str,
    pub secret_key: &'a str,
    pub region: &'a str,
    pub service: &'a str,
    pub date: &'a str,     // Format: YYYYMMDD
    pub datetime: &'a str, // Format: YYYYMMDDTHHMMSSZ
}

fn create_canonical_request(params: &SigningParams) -> String {
    let payload_hash = sha256_hex(params.payload);

    // Sort headers by lowercase key
    let mut sorted_headers: Vec<(&String, &String)> = params.headers.iter().collect();
    sorted_headers.sort_by_key(|(k, _)| k.to_lowercase());

    let canonical_headers = sorted_headers
        .iter()
        .map(|(k, v)| format!("{}:{}", k.to_lowercase(), v.trim()))
        .collect::<Vec<_>>()
        .join("\n");

    let signed_headers = sorted_headers
        .iter()
        .map(|(k, _)| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");

    format!(
        "{}\n{}\n{}\n{}\n\n{}\n{}",
        params.method,
        params.uri,
        params.query_string,
        canonical_headers,
        signed_headers,
        payload_hash
    )
}

fn create_string_to_sign(params: &SigningParams) -> String {
    let canonical_request = create_canonical_request(params);
    let canonical_request_hash = sha256_hex(canonical_request.as_bytes());

    let credential_scope = format!(
        "{}/{}/{}/aws4_request",
        params.date, params.region, params.service
    );

    format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        params.datetime, credential_scope, canonical_request_hash
    )
}

fn derive_signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

pub fn sign_request(params: &SigningParams) -> String {
    // Step 1 & 2: Create string to sign (includes canonical request generation)
    let string_to_sign = create_string_to_sign(params);

    // Calculate signed_headers for Authorization header
    let mut sorted_headers: Vec<(&String, &String)> = params.headers.iter().collect();
    sorted_headers.sort_by_key(|(k, _)| k.to_lowercase());
    let signed_headers = sorted_headers
        .iter()
        .map(|(k, _)| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");

    // Calculate credential scope for Authorization header
    let credential_scope = format!(
        "{}/{}/{}/aws4_request",
        params.date, params.region, params.service
    );

    // Step 3: Calculate signing key
    let k_signing = derive_signing_key(
        params.secret_key,
        params.date,
        params.region,
        params.service,
    );

    // Step 4: Calculate signature
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    // Step 5: Create Authorization header
    format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        params.access_key, credential_scope, signed_headers, signature
    )
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

    #[test]
    fn test_generates_valid_aws_signature_v4_for_get_request() {
        use std::collections::HashMap;

        // Test parameters (based on AWS Signature v4 test suite)
        let method = "GET";
        let uri = "/test-bucket/test-key.txt";
        let query_string = "";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";
        let service = "s3";
        let date = "20130524";
        let datetime = "20130524T000000Z";

        // Headers required for AWS Signature v4
        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "test-bucket.s3.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), datetime.to_string());
        headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));

        // Empty payload for GET request
        let payload = b"";

        // Generate signature
        let params = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers,
            payload,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime,
        };

        let authorization = sign_request(&params);

        // Verify Authorization header format
        assert!(
            authorization.starts_with("AWS4-HMAC-SHA256"),
            "Authorization header should start with AWS4-HMAC-SHA256"
        );
        assert!(
            authorization.contains("Credential="),
            "Authorization header should contain Credential"
        );
        assert!(
            authorization.contains("SignedHeaders="),
            "Authorization header should contain SignedHeaders"
        );
        assert!(
            authorization.contains("Signature="),
            "Authorization header should contain Signature"
        );

        // Verify credential scope is included
        assert!(
            authorization.contains(&format!("{}/{}/{}/aws4_request", date, region, service)),
            "Authorization header should contain correct credential scope"
        );

        // Verify access key is included
        assert!(
            authorization.contains(access_key),
            "Authorization header should contain access key"
        );

        // Verify signed headers are included
        assert!(
            authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"),
            "Authorization header should contain correct signed headers"
        );

        // Verify signature is a valid hex string (64 characters for SHA256)
        let signature_part = authorization
            .split("Signature=")
            .nth(1)
            .expect("Should have Signature part");
        assert_eq!(
            signature_part.len(),
            64,
            "Signature should be 64 hex characters"
        );
        assert!(
            signature_part.chars().all(|c| c.is_ascii_hexdigit()),
            "Signature should only contain hex characters"
        );
    }

    #[test]
    fn test_signature_includes_all_required_headers() {
        use std::collections::HashMap;

        let method = "GET";
        let uri = "/bucket/key";
        let query_string = "";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";
        let service = "s3";
        let date = "20130524";
        let datetime = "20130524T000000Z";

        // Create headers with multiple required headers
        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers.insert("x-amz-date".to_string(), datetime.to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            sha256_hex(b"").to_string(),
        );
        headers.insert("x-amz-security-token".to_string(), "test-token".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());

        let payload = b"";

        let params = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers,
            payload,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime,
        };

        let authorization = sign_request(&params);

        // Extract SignedHeaders from Authorization header
        let signed_headers_part = authorization
            .split("SignedHeaders=")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .expect("Should have SignedHeaders");

        // Verify all headers are included in SignedHeaders (sorted alphabetically, lowercase)
        assert!(
            signed_headers_part.contains("content-type"),
            "SignedHeaders should include content-type"
        );
        assert!(
            signed_headers_part.contains("host"),
            "SignedHeaders should include host"
        );
        assert!(
            signed_headers_part.contains("x-amz-content-sha256"),
            "SignedHeaders should include x-amz-content-sha256"
        );
        assert!(
            signed_headers_part.contains("x-amz-date"),
            "SignedHeaders should include x-amz-date"
        );
        assert!(
            signed_headers_part.contains("x-amz-security-token"),
            "SignedHeaders should include x-amz-security-token"
        );

        // Verify headers are in alphabetical order and semicolon-separated
        assert_eq!(
            signed_headers_part,
            "content-type;host;x-amz-content-sha256;x-amz-date;x-amz-security-token",
            "SignedHeaders should be alphabetically sorted and semicolon-separated"
        );

        // Verify that changing the headers changes the signature
        let mut headers2 = headers.clone();
        headers2.remove("x-amz-security-token");

        let params2 = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers2,
            payload,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime,
        };

        let authorization2 = sign_request(&params2);

        assert_ne!(
            authorization, authorization2,
            "Signature should change when headers change"
        );
    }

    #[test]
    fn test_signature_includes_authorization_header_with_correct_format() {
        use std::collections::HashMap;

        let method = "GET";
        let uri = "/bucket/key";
        let query_string = "";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";
        let service = "s3";
        let date = "20130524";
        let datetime = "20130524T000000Z";

        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers.insert("x-amz-date".to_string(), datetime.to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            sha256_hex(b"").to_string(),
        );

        let payload = b"";

        let params = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers,
            payload,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime,
        };

        let authorization = sign_request(&params);

        // Verify format: AWS4-HMAC-SHA256 Credential=..., SignedHeaders=..., Signature=...

        // 1. Must start with AWS4-HMAC-SHA256
        assert!(
            authorization.starts_with("AWS4-HMAC-SHA256 "),
            "Authorization header must start with 'AWS4-HMAC-SHA256 '"
        );

        // 2. Must contain Credential= with access key and credential scope
        assert!(
            authorization.contains("Credential="),
            "Authorization header must contain 'Credential='"
        );

        let expected_credential_scope = format!("{}/{}/{}/aws4_request", date, region, service);
        assert!(
            authorization.contains(&format!("Credential={}/{}", access_key, expected_credential_scope)),
            "Credential must be in format 'Credential=<access_key>/<date>/<region>/<service>/aws4_request'"
        );

        // 3. Must contain SignedHeaders=
        assert!(
            authorization.contains("SignedHeaders="),
            "Authorization header must contain 'SignedHeaders='"
        );

        // 4. Must contain Signature=
        assert!(
            authorization.contains("Signature="),
            "Authorization header must contain 'Signature='"
        );

        // 5. Verify the order: Credential, SignedHeaders, Signature
        let credential_pos = authorization.find("Credential=").unwrap();
        let signed_headers_pos = authorization.find("SignedHeaders=").unwrap();
        let signature_pos = authorization.find("Signature=").unwrap();

        assert!(
            credential_pos < signed_headers_pos,
            "Credential must come before SignedHeaders"
        );
        assert!(
            signed_headers_pos < signature_pos,
            "SignedHeaders must come before Signature"
        );

        // 6. Verify components are comma-separated
        assert!(
            authorization.contains(", SignedHeaders="),
            "Components must be separated by ', '"
        );
        assert!(
            authorization.contains(", Signature="),
            "Components must be separated by ', '"
        );

        // 7. Verify complete format with regex-like check
        let parts: Vec<&str> = authorization.split(' ').collect();
        assert_eq!(
            parts[0], "AWS4-HMAC-SHA256",
            "First part must be 'AWS4-HMAC-SHA256'"
        );

        // Remaining parts should be "Credential=..., SignedHeaders=..., Signature=..."
        let components = parts[1..].join(" ");
        assert!(
            components.starts_with("Credential="),
            "Second part must start with 'Credential='"
        );
    }

    #[test]
    fn test_signature_includes_x_amz_date_header() {
        use std::collections::HashMap;

        let method = "GET";
        let uri = "/bucket/key";
        let query_string = "";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";
        let service = "s3";
        let date = "20130524";
        let datetime1 = "20130524T000000Z";
        let datetime2 = "20130524T120000Z";

        // Create first signature with datetime1
        let mut headers1 = HashMap::new();
        headers1.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers1.insert("x-amz-date".to_string(), datetime1.to_string());
        headers1.insert(
            "x-amz-content-sha256".to_string(),
            sha256_hex(b"").to_string(),
        );

        let payload = b"";

        let params1 = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers1,
            payload,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime: datetime1,
        };

        let authorization1 = sign_request(&params1);

        // Verify x-amz-date is in SignedHeaders
        assert!(
            authorization1.contains("x-amz-date"),
            "SignedHeaders must include x-amz-date"
        );

        // Create second signature with datetime2 (different time)
        let mut headers2 = HashMap::new();
        headers2.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers2.insert("x-amz-date".to_string(), datetime2.to_string());
        headers2.insert(
            "x-amz-content-sha256".to_string(),
            sha256_hex(b"").to_string(),
        );

        let params2 = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers2,
            payload,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime: datetime2,
        };

        let authorization2 = sign_request(&params2);

        // Verify that changing x-amz-date value changes the signature
        assert_ne!(
            authorization1, authorization2,
            "Signature must change when x-amz-date header value changes"
        );

        // Extract signatures to verify they're different
        let sig1 = authorization1
            .split("Signature=")
            .nth(1)
            .expect("Should have Signature");
        let sig2 = authorization2
            .split("Signature=")
            .nth(1)
            .expect("Should have Signature");

        assert_ne!(
            sig1, sig2,
            "Signature value must be different when x-amz-date is different"
        );
    }

    #[test]
    fn test_signature_includes_x_amz_content_sha256_header() {
        use std::collections::HashMap;

        let method = "GET";
        let uri = "/bucket/key";
        let query_string = "";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let region = "us-east-1";
        let service = "s3";
        let date = "20130524";
        let datetime = "20130524T000000Z";

        // Create first signature with empty payload hash
        let payload1 = b"";
        let payload1_hash = sha256_hex(payload1);

        let mut headers1 = HashMap::new();
        headers1.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers1.insert("x-amz-date".to_string(), datetime.to_string());
        headers1.insert("x-amz-content-sha256".to_string(), payload1_hash.clone());

        let params1 = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers1,
            payload: payload1,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime,
        };

        let authorization1 = sign_request(&params1);

        // Verify x-amz-content-sha256 is in SignedHeaders
        assert!(
            authorization1.contains("x-amz-content-sha256"),
            "SignedHeaders must include x-amz-content-sha256"
        );

        // Create second signature with different payload hash
        let payload2 = b"test-payload";
        let payload2_hash = sha256_hex(payload2);

        let mut headers2 = HashMap::new();
        headers2.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers2.insert("x-amz-date".to_string(), datetime.to_string());
        headers2.insert("x-amz-content-sha256".to_string(), payload2_hash.clone());

        let params2 = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers2,
            payload: payload2,
            access_key,
            secret_key,
            region,
            service,
            date,
            datetime,
        };

        let authorization2 = sign_request(&params2);

        // Verify that changing x-amz-content-sha256 value changes the signature
        assert_ne!(
            authorization1, authorization2,
            "Signature must change when x-amz-content-sha256 header value changes"
        );

        // Extract signatures to verify they're different
        let sig1 = authorization1
            .split("Signature=")
            .nth(1)
            .expect("Should have Signature");
        let sig2 = authorization2
            .split("Signature=")
            .nth(1)
            .expect("Should have Signature");

        assert_ne!(
            sig1, sig2,
            "Signature value must be different when x-amz-content-sha256 is different"
        );

        // Verify that the payload hash values are actually different
        assert_ne!(
            payload1_hash, payload2_hash,
            "Payload hashes should be different for different payloads"
        );
    }

    #[test]
    fn test_canonical_request_is_generated_correctly() {
        use std::collections::HashMap;

        let method = "GET";
        let uri = "/test-bucket/test-key.txt";
        let query_string = "";

        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "test-bucket.s3.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), "20130524T000000Z".to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            sha256_hex(b"").to_string(),
        );

        let payload = b"";

        let params = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers,
            payload,
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20130524",
            datetime: "20130524T000000Z",
        };

        let canonical_request = create_canonical_request(&params);

        // Verify format: METHOD\nURI\nQUERY_STRING\nCANONICAL_HEADERS\n\nSIGNED_HEADERS\nPAYLOAD_HASH
        let lines: Vec<&str> = canonical_request.split('\n').collect();

        // Line 0: HTTP method
        assert_eq!(lines[0], "GET", "First line should be HTTP method");

        // Line 1: Canonical URI
        assert_eq!(
            lines[1], "/test-bucket/test-key.txt",
            "Second line should be canonical URI"
        );

        // Line 2: Canonical query string (empty in this test)
        assert_eq!(lines[2], "", "Third line should be canonical query string");

        // Lines 3+: Canonical headers (sorted, lowercase keys, trimmed values)
        // Should include: host, x-amz-content-sha256, x-amz-date (alphabetically)
        assert!(
            canonical_request.contains("host:test-bucket.s3.amazonaws.com\n"),
            "Canonical request should include host header"
        );
        assert!(
            canonical_request.contains("x-amz-content-sha256:"),
            "Canonical request should include x-amz-content-sha256 header"
        );
        assert!(
            canonical_request.contains("x-amz-date:20130524T000000Z\n"),
            "Canonical request should include x-amz-date header"
        );

        // Verify signed headers list (second to last line, separated by empty line)
        assert!(
            canonical_request.contains("host;x-amz-content-sha256;x-amz-date"),
            "Canonical request should contain signed headers list"
        );

        // Verify payload hash (last line)
        let payload_hash = sha256_hex(b"");
        assert!(
            canonical_request.ends_with(&payload_hash),
            "Canonical request should end with payload hash"
        );

        // Verify headers are sorted alphabetically (case-insensitive)
        let host_pos = canonical_request.find("host:").unwrap();
        let sha256_pos = canonical_request.find("x-amz-content-sha256:").unwrap();
        let date_pos = canonical_request.find("x-amz-date:").unwrap();

        assert!(
            host_pos < sha256_pos && sha256_pos < date_pos,
            "Headers should be sorted alphabetically"
        );
    }

    #[test]
    fn test_string_to_sign_is_generated_correctly() {
        use std::collections::HashMap;

        let method = "GET";
        let uri = "/test-bucket/test-key.txt";
        let query_string = "";
        let region = "us-east-1";
        let service = "s3";
        let date = "20130524";
        let datetime = "20130524T000000Z";

        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "test-bucket.s3.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), datetime.to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            sha256_hex(b"").to_string(),
        );

        let payload = b"";

        let params = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers,
            payload,
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region,
            service,
            date,
            datetime,
        };

        let string_to_sign = create_string_to_sign(&params);

        // Verify format: AWS4-HMAC-SHA256\n<datetime>\n<credential_scope>\n<canonical_request_hash>
        let lines: Vec<&str> = string_to_sign.split('\n').collect();

        // Line 0: Algorithm identifier
        assert_eq!(
            lines[0], "AWS4-HMAC-SHA256",
            "First line should be algorithm identifier"
        );

        // Line 1: Datetime
        assert_eq!(
            lines[1], datetime,
            "Second line should be datetime in format YYYYMMDDTHHMMSSZ"
        );

        // Line 2: Credential scope
        let expected_credential_scope = format!("{}/{}/{}/aws4_request", date, region, service);
        assert_eq!(
            lines[2], expected_credential_scope,
            "Third line should be credential scope in format date/region/service/aws4_request"
        );

        // Line 3: Canonical request hash (SHA256 hex, 64 characters)
        assert_eq!(
            lines[3].len(),
            64,
            "Fourth line should be canonical request hash (64 hex characters)"
        );
        assert!(
            lines[3].chars().all(|c| c.is_ascii_hexdigit()),
            "Canonical request hash should only contain hex characters"
        );

        // Verify that changing the canonical request changes the string to sign
        let mut headers2 = headers.clone();
        headers2.insert("x-custom-header".to_string(), "value".to_string());

        let params2 = SigningParams {
            method,
            uri,
            query_string,
            headers: &headers2,
            payload,
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region,
            service,
            date,
            datetime,
        };

        let string_to_sign2 = create_string_to_sign(&params2);

        assert_ne!(
            string_to_sign, string_to_sign2,
            "String to sign should change when canonical request changes"
        );

        // Verify only the canonical request hash line is different
        let lines2: Vec<&str> = string_to_sign2.split('\n').collect();
        assert_eq!(lines[0], lines2[0], "Algorithm should be the same");
        assert_eq!(lines[1], lines2[1], "Datetime should be the same");
        assert_eq!(lines[2], lines2[2], "Credential scope should be the same");
        assert_ne!(
            lines[3], lines2[3],
            "Canonical request hash should be different"
        );
    }

    #[test]
    fn test_signing_key_derivation_works_correctly() {
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let date = "20130524";
        let region = "us-east-1";
        let service = "s3";

        let signing_key = derive_signing_key(secret_key, date, region, service);

        // Verify signing key is not empty
        assert!(!signing_key.is_empty(), "Signing key should not be empty");

        // Verify signing key is 32 bytes (HMAC-SHA256 output)
        assert_eq!(
            signing_key.len(),
            32,
            "Signing key should be 32 bytes (HMAC-SHA256 output)"
        );

        // Verify signing key changes with different secret keys
        let signing_key2 = derive_signing_key("different-secret-key", date, region, service);
        assert_ne!(
            signing_key, signing_key2,
            "Signing key should change with different secret key"
        );

        // Verify signing key changes with different dates
        let signing_key3 = derive_signing_key(secret_key, "20130525", region, service);
        assert_ne!(
            signing_key, signing_key3,
            "Signing key should change with different date"
        );

        // Verify signing key changes with different regions
        let signing_key4 = derive_signing_key(secret_key, date, "us-west-2", service);
        assert_ne!(
            signing_key, signing_key4,
            "Signing key should change with different region"
        );

        // Verify signing key changes with different services
        let signing_key5 = derive_signing_key(secret_key, date, region, "ec2");
        assert_ne!(
            signing_key, signing_key5,
            "Signing key should change with different service"
        );

        // Verify signing key is deterministic (same inputs = same output)
        let signing_key6 = derive_signing_key(secret_key, date, region, service);
        assert_eq!(
            signing_key, signing_key6,
            "Signing key should be deterministic"
        );
    }
}
