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

/// Represents an S3 GET/HEAD request
#[derive(Debug)]
pub struct S3Request {
    pub method: String,
    pub bucket: String,
    pub key: String,
    pub region: String,
}

impl S3Request {
    /// Returns the URL path for the S3 request (path-style: /bucket/key)
    pub fn get_url(&self) -> String {
        format!("/{}/{}", self.bucket, self.key)
    }

    /// Returns signed headers for the S3 request including Authorization header
    pub fn get_signed_headers(
        &self,
        access_key: &str,
        secret_key: &str,
    ) -> std::collections::HashMap<String, String> {
        use std::collections::HashMap;

        // Generate timestamp (hardcoded for now, will use actual time later)
        let datetime = "20130524T000000Z";
        let date = "20130524";

        // Build headers
        let mut headers = HashMap::new();
        let host = format!("{}.s3.{}.amazonaws.com", self.bucket, self.region);
        headers.insert("host".to_string(), host);
        headers.insert("x-amz-date".to_string(), datetime.to_string());
        headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));

        // Create signing params
        let params = SigningParams {
            method: &self.method,
            uri: &self.get_url(),
            query_string: "",
            headers: &headers,
            payload: b"",
            access_key,
            secret_key,
            region: &self.region,
            service: "s3",
            date,
            datetime,
        };

        // Generate Authorization header
        let authorization = sign_request(&params);
        headers.insert("authorization".to_string(), authorization);

        headers
    }
}

/// Builds a GET object request for S3
pub fn build_get_object_request(bucket: &str, key: &str, region: &str) -> S3Request {
    S3Request {
        method: "GET".to_string(),
        bucket: bucket.to_string(),
        key: key.to_string(),
        region: region.to_string(),
    }
}

/// Builds a HEAD object request for S3
pub fn build_head_object_request(bucket: &str, key: &str, region: &str) -> S3Request {
    S3Request {
        method: "HEAD".to_string(),
        bucket: bucket.to_string(),
        key: key.to_string(),
        region: region.to_string(),
    }
}

/// Represents an S3 response
#[derive(Debug)]
pub struct S3Response {
    pub status_code: u16,
    pub status_text: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
}

impl S3Response {
    /// Creates a new S3Response
    pub fn new(
        status_code: u16,
        status_text: &str,
        headers: std::collections::HashMap<String, String>,
        body: Vec<u8>,
    ) -> Self {
        S3Response {
            status_code,
            status_text: status_text.to_string(),
            headers,
            body,
        }
    }

    /// Returns true if the response indicates success (2xx status code)
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    /// Gets a header value by name
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    /// Extracts the error code from S3 XML error response
    pub fn get_error_code(&self) -> Option<String> {
        let body_str = String::from_utf8(self.body.clone()).ok()?;

        // Find <Code> tag and extract its content
        let start_tag = "<Code>";
        let end_tag = "</Code>";

        let start_pos = body_str.find(start_tag)?;
        let content_start = start_pos + start_tag.len();
        let content_end = body_str[content_start..].find(end_tag)?;

        Some(body_str[content_start..content_start + content_end].to_string())
    }

    /// Extracts the error message from S3 XML error response
    pub fn get_error_message(&self) -> Option<String> {
        let body_str = String::from_utf8(self.body.clone()).ok()?;

        // Find <Message> tag and extract its content
        let start_tag = "<Message>";
        let end_tag = "</Message>";

        let start_pos = body_str.find(start_tag)?;
        let content_start = start_pos + start_tag.len();
        let content_end = body_str[content_start..].find(end_tag)?;

        Some(body_str[content_start..content_start + content_end].to_string())
    }
}

/// Maps S3 error code to appropriate HTTP status code
pub fn map_s3_error_to_status(error_code: &str) -> u16 {
    match error_code {
        // 404 - Not Found
        "NoSuchKey" | "NoSuchBucket" | "NoSuchUpload" | "NoSuchVersion" => 404,

        // 403 - Forbidden
        "AccessDenied"
        | "InvalidAccessKeyId"
        | "SignatureDoesNotMatch"
        | "AccountProblem"
        | "InvalidSecurity" => 403,

        // 400 - Bad Request
        "InvalidArgument"
        | "InvalidBucketName"
        | "InvalidRange"
        | "MalformedXML"
        | "InvalidDigest"
        | "InvalidRequest"
        | "InvalidURI"
        | "KeyTooLongError"
        | "MalformedACLError"
        | "MalformedPOSTRequest"
        | "MetadataTooLarge"
        | "MissingContentLength"
        | "MissingRequestBodyError"
        | "TooManyBuckets"
        | "InvalidPart"
        | "InvalidPartOrder" => 400,

        // 409 - Conflict
        "BucketAlreadyExists"
        | "BucketNotEmpty"
        | "BucketAlreadyOwnedByYou"
        | "OperationAborted" => 409,

        // 412 - Precondition Failed
        "PreconditionFailed" => 412,

        // 416 - Range Not Satisfiable
        "InvalidRange416" => 416,

        // 503 - Service Unavailable
        "SlowDown" | "ServiceUnavailable" => 503,

        // 500 - Internal Server Error
        "InternalError" => 500,

        // Default to 500 for unknown errors
        _ => 500,
    }
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

    #[test]
    fn test_can_build_get_object_request_with_key() {
        let bucket = "test-bucket";
        let key = "test-key.txt";
        let region = "us-east-1";

        let request = build_get_object_request(bucket, key, region);

        // Verify the request has correct method
        assert_eq!(request.method, "GET", "Request method should be GET");

        // Verify the request includes bucket in path or host
        let request_str = format!("{:?}", request);
        assert!(
            request_str.contains(bucket),
            "Request should include bucket name"
        );

        // Verify the request includes key in path
        assert!(
            request_str.contains(key),
            "Request should include object key"
        );
    }

    #[test]
    fn test_get_request_includes_correct_bucket_and_key_in_url() {
        let bucket = "my-bucket";
        let key = "folder/file.txt";
        let region = "us-east-1";

        let request = build_get_object_request(bucket, key, region);
        let url = request.get_url();

        // Verify URL contains bucket name
        assert!(
            url.contains(bucket),
            "URL should contain bucket name: {}",
            url
        );

        // Verify URL contains key (path-style: /bucket/key)
        assert!(url.contains(key), "URL should contain object key: {}", url);

        // Verify path-style URL format: /bucket/key
        let expected_path = format!("/{}/{}", bucket, key);
        assert!(
            url.contains(&expected_path) || url.contains(&format!("{}.s3", bucket)),
            "URL should use either path-style (/bucket/key) or virtual-hosted-style (bucket.s3...): {}",
            url
        );

        // Test with simple key (no slash)
        let request2 = build_get_object_request("test-bucket", "simple.txt", "us-west-2");
        let url2 = request2.get_url();
        assert!(
            url2.contains("test-bucket"),
            "URL should contain bucket: {}",
            url2
        );
        assert!(
            url2.contains("simple.txt"),
            "URL should contain key: {}",
            url2
        );
    }

    #[test]
    fn test_get_request_includes_proper_aws_signature_headers() {
        let bucket = "test-bucket";
        let key = "test-key.txt";
        let region = "us-east-1";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

        let request = build_get_object_request(bucket, key, region);
        let headers = request.get_signed_headers(access_key, secret_key);

        // Verify x-amz-date header is present
        assert!(
            headers.contains_key("x-amz-date"),
            "Request should include x-amz-date header"
        );

        // Verify x-amz-date is in correct format (YYYYMMDDTHHMMSSZ)
        let date_header = headers.get("x-amz-date").unwrap();
        assert_eq!(
            date_header.len(),
            16,
            "x-amz-date should be 16 characters (YYYYMMDDTHHMMSSZ)"
        );
        assert!(date_header.ends_with('Z'), "x-amz-date should end with Z");

        // Verify x-amz-content-sha256 header is present
        assert!(
            headers.contains_key("x-amz-content-sha256"),
            "Request should include x-amz-content-sha256 header"
        );

        // Verify x-amz-content-sha256 is a valid SHA256 hex (64 characters)
        let content_sha_header = headers.get("x-amz-content-sha256").unwrap();
        assert_eq!(
            content_sha_header.len(),
            64,
            "x-amz-content-sha256 should be 64 hex characters"
        );
        assert!(
            content_sha_header.chars().all(|c| c.is_ascii_hexdigit()),
            "x-amz-content-sha256 should only contain hex characters"
        );

        // Verify Authorization header is present
        assert!(
            headers.contains_key("authorization"),
            "Request should include Authorization header"
        );

        // Verify Authorization header has correct format
        let auth_header = headers.get("authorization").unwrap();
        assert!(
            auth_header.starts_with("AWS4-HMAC-SHA256"),
            "Authorization header should start with AWS4-HMAC-SHA256"
        );
        assert!(
            auth_header.contains("Credential="),
            "Authorization header should contain Credential="
        );
        assert!(
            auth_header.contains("SignedHeaders="),
            "Authorization header should contain SignedHeaders="
        );
        assert!(
            auth_header.contains("Signature="),
            "Authorization header should contain Signature="
        );

        // Verify host header is present
        assert!(
            headers.contains_key("host"),
            "Request should include host header"
        );

        // Verify host header includes bucket and region
        let host_header = headers.get("host").unwrap();
        assert!(
            host_header.contains(bucket) || host_header.contains("s3"),
            "Host header should include bucket or s3: {}",
            host_header
        );
    }

    #[test]
    fn test_get_request_handles_s3_keys_with_special_characters() {
        let bucket = "test-bucket";
        let region = "us-east-1";

        // Test key with spaces
        let key_with_spaces = "folder/my file.txt";
        let request1 = build_get_object_request(bucket, key_with_spaces, region);
        assert_eq!(request1.key, key_with_spaces);
        let url1 = request1.get_url();
        assert!(
            url1.contains(key_with_spaces),
            "URL should contain key with spaces: {}",
            url1
        );

        // Test key with hyphens and underscores
        let key_with_symbols = "my-folder/my_file-v2.txt";
        let request2 = build_get_object_request(bucket, key_with_symbols, region);
        assert_eq!(request2.key, key_with_symbols);
        let url2 = request2.get_url();
        assert!(
            url2.contains(key_with_symbols),
            "URL should contain key with hyphens/underscores: {}",
            url2
        );

        // Test key with dots
        let key_with_dots = "folder/file.backup.2023.txt";
        let request3 = build_get_object_request(bucket, key_with_dots, region);
        assert_eq!(request3.key, key_with_dots);
        let url3 = request3.get_url();
        assert!(
            url3.contains(key_with_dots),
            "URL should contain key with dots: {}",
            url3
        );

        // Test key with parentheses
        let key_with_parens = "folder/file(1).txt";
        let request4 = build_get_object_request(bucket, key_with_parens, region);
        assert_eq!(request4.key, key_with_parens);
        let url4 = request4.get_url();
        assert!(
            url4.contains(key_with_parens),
            "URL should contain key with parentheses: {}",
            url4
        );
    }

    #[test]
    fn test_get_request_handles_s3_keys_with_url_unsafe_characters() {
        let bucket = "test-bucket";
        let region = "us-east-1";

        // Test key with percent sign
        let key_with_percent = "folder/file%20name.txt";
        let request1 = build_get_object_request(bucket, key_with_percent, region);
        assert_eq!(request1.key, key_with_percent);
        assert!(
            request1.get_url().contains(key_with_percent),
            "URL should preserve percent sign in key"
        );

        // Test key with hash/pound sign
        let key_with_hash = "folder/file#1.txt";
        let request2 = build_get_object_request(bucket, key_with_hash, region);
        assert_eq!(request2.key, key_with_hash);
        assert!(
            request2.get_url().contains(key_with_hash),
            "URL should preserve hash sign in key"
        );

        // Test key with ampersand
        let key_with_ampersand = "folder/file&data.txt";
        let request3 = build_get_object_request(bucket, key_with_ampersand, region);
        assert_eq!(request3.key, key_with_ampersand);
        assert!(
            request3.get_url().contains(key_with_ampersand),
            "URL should preserve ampersand in key"
        );

        // Test key with plus sign
        let key_with_plus = "folder/file+v2.txt";
        let request4 = build_get_object_request(bucket, key_with_plus, region);
        assert_eq!(request4.key, key_with_plus);
        assert!(
            request4.get_url().contains(key_with_plus),
            "URL should preserve plus sign in key"
        );

        // Test key with equals sign
        let key_with_equals = "folder/file=copy.txt";
        let request5 = build_get_object_request(bucket, key_with_equals, region);
        assert_eq!(request5.key, key_with_equals);
        assert!(
            request5.get_url().contains(key_with_equals),
            "URL should preserve equals sign in key"
        );

        // Test key with question mark
        let key_with_question = "folder/file?.txt";
        let request6 = build_get_object_request(bucket, key_with_question, region);
        assert_eq!(request6.key, key_with_question);
        assert!(
            request6.get_url().contains(key_with_question),
            "URL should preserve question mark in key"
        );
    }

    #[test]
    fn test_get_request_preserves_original_path_structure() {
        let bucket = "test-bucket";
        let region = "us-east-1";

        // Test deeply nested path
        let nested_key = "level1/level2/level3/level4/file.txt";
        let request1 = build_get_object_request(bucket, nested_key, region);
        assert_eq!(request1.key, nested_key, "Key should be preserved exactly");
        let url1 = request1.get_url();
        assert!(
            url1.contains(nested_key),
            "URL should preserve nested path structure: {}",
            url1
        );
        // Verify all path levels are present
        assert!(url1.contains("level1/level2/level3/level4/file.txt"));

        // Test path with trailing slash (folder marker)
        let folder_key = "folder/subfolder/";
        let request2 = build_get_object_request(bucket, folder_key, region);
        assert_eq!(request2.key, folder_key, "Folder key should be preserved");
        let url2 = request2.get_url();
        assert!(
            url2.ends_with("/"),
            "URL should preserve trailing slash: {}",
            url2
        );

        // Test single-level path
        let single_level = "document.pdf";
        let request3 = build_get_object_request(bucket, single_level, region);
        assert_eq!(request3.key, single_level);
        let url3 = request3.get_url();
        assert!(
            url3.contains(single_level),
            "URL should preserve single-level path: {}",
            url3
        );

        // Test path with multiple slashes (edge case)
        let multiple_slashes = "folder//subfolder/file.txt";
        let request4 = build_get_object_request(bucket, multiple_slashes, region);
        assert_eq!(
            request4.key, multiple_slashes,
            "Key with multiple slashes should be preserved exactly"
        );
        let url4 = request4.get_url();
        assert!(
            url4.contains(multiple_slashes),
            "URL should preserve multiple slashes: {}",
            url4
        );

        // Test path starting with slash (edge case)
        let leading_slash = "/folder/file.txt";
        let request5 = build_get_object_request(bucket, leading_slash, region);
        assert_eq!(
            request5.key, leading_slash,
            "Key with leading slash should be preserved"
        );
    }

    #[test]
    fn test_can_build_head_object_request_with_key() {
        let bucket = "test-bucket";
        let key = "test-key.txt";
        let region = "us-east-1";

        let request = build_head_object_request(bucket, key, region);

        // Verify the request has correct method
        assert_eq!(request.method, "HEAD", "Request method should be HEAD");

        // Verify the request includes bucket
        assert_eq!(request.bucket, bucket);

        // Verify the request includes key
        assert_eq!(request.key, key);

        // Verify the request includes region
        assert_eq!(request.region, region);
    }

    #[test]
    fn test_head_request_includes_correct_http_method() {
        let bucket = "my-bucket";
        let key = "documents/report.pdf";
        let region = "us-west-2";

        let request = build_head_object_request(bucket, key, region);

        // Verify method is exactly "HEAD" (not "head" or "Head")
        assert_eq!(
            request.method, "HEAD",
            "HEAD request must use uppercase HEAD method"
        );

        // Verify method is not GET
        assert_ne!(
            request.method, "GET",
            "HEAD request should not use GET method"
        );

        // Test with different keys to ensure method is always HEAD
        let request2 = build_head_object_request("another-bucket", "file.txt", "eu-west-1");
        assert_eq!(
            request2.method, "HEAD",
            "Method should always be HEAD regardless of parameters"
        );

        let request3 = build_head_object_request("bucket3", "path/to/object", "ap-south-1");
        assert_eq!(request3.method, "HEAD");
    }

    #[test]
    fn test_head_request_includes_same_headers_as_get() {
        let bucket = "test-bucket";
        let key = "documents/file.pdf";
        let region = "us-east-1";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

        // Build GET request and get headers
        let get_request = build_get_object_request(bucket, key, region);
        let get_headers = get_request.get_signed_headers(access_key, secret_key);

        // Build HEAD request and get headers
        let head_request = build_head_object_request(bucket, key, region);
        let head_headers = head_request.get_signed_headers(access_key, secret_key);

        // Verify both have the same header keys
        let get_keys: std::collections::HashSet<_> = get_headers.keys().collect();
        let head_keys: std::collections::HashSet<_> = head_headers.keys().collect();
        assert_eq!(
            get_keys, head_keys,
            "HEAD and GET requests should have the same header keys"
        );

        // Verify both include required AWS headers
        assert!(
            head_headers.contains_key("host"),
            "HEAD request should include host header"
        );
        assert!(
            head_headers.contains_key("x-amz-date"),
            "HEAD request should include x-amz-date header"
        );
        assert!(
            head_headers.contains_key("x-amz-content-sha256"),
            "HEAD request should include x-amz-content-sha256 header"
        );
        assert!(
            head_headers.contains_key("authorization"),
            "HEAD request should include authorization header"
        );

        // Verify host header is the same (independent of method)
        assert_eq!(
            get_headers.get("host"),
            head_headers.get("host"),
            "Host header should be identical for GET and HEAD"
        );

        // Verify x-amz-content-sha256 is the same (empty payload for both)
        assert_eq!(
            get_headers.get("x-amz-content-sha256"),
            head_headers.get("x-amz-content-sha256"),
            "Content SHA256 should be identical for GET and HEAD"
        );

        // Note: x-amz-date might differ due to timestamp generation
        // Note: Authorization signature will differ because method is different
    }

    #[test]
    fn test_head_request_returns_object_metadata_without_body() {
        // This test documents the expected behavior of HEAD requests:
        // HEAD requests should return the same headers as GET (metadata)
        // but with no response body, as per HTTP specification.

        let bucket = "metadata-bucket";
        let key = "large-file.bin";
        let region = "us-east-1";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

        let head_request = build_head_object_request(bucket, key, region);

        // Verify method is HEAD (which per HTTP spec means no response body)
        assert_eq!(
            head_request.method, "HEAD",
            "HEAD method indicates metadata-only request (no body)"
        );

        // Verify request structure is identical to GET except for method
        let get_request = build_get_object_request(bucket, key, region);
        assert_eq!(head_request.bucket, get_request.bucket);
        assert_eq!(head_request.key, get_request.key);
        assert_eq!(head_request.region, get_request.region);

        // Verify HEAD request includes all necessary headers for authentication
        let headers = head_request.get_signed_headers(access_key, secret_key);
        assert!(
            headers.contains_key("authorization"),
            "HEAD request must include authorization for metadata access"
        );

        // The key difference: HEAD method tells S3 to return only headers
        // S3 will respond with Content-Length, Content-Type, ETag, etc.
        // but the response body will be empty (0 bytes transferred)
        assert_eq!(
            head_request.method, "HEAD",
            "HEAD method ensures response body is omitted per HTTP spec"
        );

        // Verify URL is the same as GET (points to same resource)
        assert_eq!(
            head_request.get_url(),
            get_request.get_url(),
            "HEAD and GET should request the same resource URL"
        );
    }

    #[test]
    fn test_parses_200_ok_response_from_s3() {
        use std::collections::HashMap;

        // Simulate a 200 OK response from S3
        let status_code = 200;
        let status_text = "OK";
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());
        headers.insert("content-length".to_string(), "13".to_string());
        headers.insert("etag".to_string(), "\"abc123\"".to_string());

        let body = b"Hello, World!";

        let response = S3Response::new(status_code, status_text, headers, body.to_vec());

        // Verify status code is parsed correctly
        assert_eq!(response.status_code, 200, "Status code should be 200");

        // Verify status text is parsed correctly
        assert_eq!(response.status_text, "OK", "Status text should be OK");

        // Verify response is successful
        assert!(
            response.is_success(),
            "200 OK response should be considered successful"
        );

        // Verify body is preserved
        assert_eq!(response.body, body, "Response body should be preserved");
    }

    #[test]
    fn test_extracts_content_type_header_from_s3_response() {
        use std::collections::HashMap;

        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("content-length".to_string(), "100".to_string());

        let response = S3Response::new(200, "OK", headers, vec![]);

        // Test extracting content-type header
        let content_type = response.get_header("content-type");
        assert_eq!(
            content_type,
            Some(&"application/json".to_string()),
            "Should extract content-type header"
        );

        // Test with different content types
        let mut headers2 = HashMap::new();
        headers2.insert(
            "content-type".to_string(),
            "text/html; charset=utf-8".to_string(),
        );
        let response2 = S3Response::new(200, "OK", headers2, vec![]);
        assert_eq!(
            response2.get_header("content-type"),
            Some(&"text/html; charset=utf-8".to_string())
        );

        // Test with image content type
        let mut headers3 = HashMap::new();
        headers3.insert("content-type".to_string(), "image/png".to_string());
        let response3 = S3Response::new(200, "OK", headers3, vec![]);
        assert_eq!(
            response3.get_header("content-type"),
            Some(&"image/png".to_string())
        );

        // Test missing content-type header
        let headers4 = HashMap::new();
        let response4 = S3Response::new(200, "OK", headers4, vec![]);
        assert_eq!(
            response4.get_header("content-type"),
            None,
            "Should return None for missing header"
        );

        // Test case-insensitive header lookup
        let mut headers5 = HashMap::new();
        headers5.insert("Content-Type".to_string(), "text/plain".to_string());
        let response5 = S3Response::new(200, "OK", headers5, vec![]);
        assert!(
            response5.get_header("content-type").is_some()
                || response5.get_header("Content-Type").is_some(),
            "Should handle header case variations"
        );
    }

    #[test]
    fn test_extracts_content_length_header_from_s3_response() {
        use std::collections::HashMap;

        // Test with small file
        let mut headers = HashMap::new();
        headers.insert("content-length".to_string(), "1024".to_string());
        headers.insert("content-type".to_string(), "text/plain".to_string());

        let response = S3Response::new(200, "OK", headers, vec![]);

        let content_length = response.get_header("content-length");
        assert_eq!(
            content_length,
            Some(&"1024".to_string()),
            "Should extract content-length header"
        );

        // Test with large file
        let mut headers2 = HashMap::new();
        headers2.insert("content-length".to_string(), "104857600".to_string()); // 100MB
        let response2 = S3Response::new(200, "OK", headers2, vec![]);
        assert_eq!(
            response2.get_header("content-length"),
            Some(&"104857600".to_string())
        );

        // Test with zero-length file
        let mut headers3 = HashMap::new();
        headers3.insert("content-length".to_string(), "0".to_string());
        let response3 = S3Response::new(200, "OK", headers3, vec![]);
        assert_eq!(
            response3.get_header("content-length"),
            Some(&"0".to_string())
        );

        // Test missing content-length header
        let headers4 = HashMap::new();
        let response4 = S3Response::new(200, "OK", headers4, vec![]);
        assert_eq!(
            response4.get_header("content-length"),
            None,
            "Should return None for missing header"
        );

        // Test parsing content-length value as number
        let mut headers5 = HashMap::new();
        headers5.insert("content-length".to_string(), "2048".to_string());
        let response5 = S3Response::new(200, "OK", headers5, vec![]);
        if let Some(length_str) = response5.get_header("content-length") {
            let length: u64 = length_str.parse().expect("Should parse as number");
            assert_eq!(length, 2048, "Content-length should be parseable as u64");
        } else {
            panic!("Content-length header should be present");
        }
    }

    #[test]
    fn test_extracts_etag_header_from_s3_response() {
        use std::collections::HashMap;

        // Test with standard ETag (MD5 hash)
        let mut headers = HashMap::new();
        headers.insert(
            "etag".to_string(),
            "\"5d41402abc4b2a76b9719d911017c592\"".to_string(),
        );
        headers.insert("content-type".to_string(), "text/plain".to_string());

        let response = S3Response::new(200, "OK", headers, vec![]);

        let etag = response.get_header("etag");
        assert_eq!(
            etag,
            Some(&"\"5d41402abc4b2a76b9719d911017c592\"".to_string()),
            "Should extract ETag header with quotes"
        );

        // Test with multipart upload ETag (includes part count)
        let mut headers2 = HashMap::new();
        headers2.insert("etag".to_string(), "\"abc123-5\"".to_string());
        let response2 = S3Response::new(200, "OK", headers2, vec![]);
        assert_eq!(
            response2.get_header("etag"),
            Some(&"\"abc123-5\"".to_string()),
            "Should extract multipart ETag"
        );

        // Test with weak ETag (W/ prefix)
        let mut headers3 = HashMap::new();
        headers3.insert("etag".to_string(), "W/\"abc123\"".to_string());
        let response3 = S3Response::new(200, "OK", headers3, vec![]);
        assert_eq!(
            response3.get_header("etag"),
            Some(&"W/\"abc123\"".to_string()),
            "Should extract weak ETag"
        );

        // Test missing ETag header
        let headers4 = HashMap::new();
        let response4 = S3Response::new(200, "OK", headers4, vec![]);
        assert_eq!(
            response4.get_header("etag"),
            None,
            "Should return None for missing ETag"
        );

        // Test ETag without quotes (edge case)
        let mut headers5 = HashMap::new();
        headers5.insert("etag".to_string(), "abc123".to_string());
        let response5 = S3Response::new(200, "OK", headers5, vec![]);
        assert_eq!(
            response5.get_header("etag"),
            Some(&"abc123".to_string()),
            "Should handle ETag without quotes"
        );

        // Test that ETag is preserved exactly as received
        let mut headers6 = HashMap::new();
        headers6.insert(
            "etag".to_string(),
            "\"d41d8cd98f00b204e9800998ecf8427e\"".to_string(),
        );
        let response6 = S3Response::new(200, "OK", headers6, vec![]);
        let etag_value = response6.get_header("etag").unwrap();
        assert!(
            etag_value.starts_with('"') && etag_value.ends_with('"'),
            "ETag should preserve surrounding quotes"
        );
    }

    #[test]
    fn test_extracts_last_modified_header_from_s3_response() {
        use std::collections::HashMap;

        // Test with standard Last-Modified format (HTTP date)
        let mut headers = HashMap::new();
        headers.insert(
            "last-modified".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );
        headers.insert("content-type".to_string(), "text/plain".to_string());

        let response = S3Response::new(200, "OK", headers, vec![]);

        let last_modified = response.get_header("last-modified");
        assert_eq!(
            last_modified,
            Some(&"Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            "Should extract Last-Modified header in HTTP date format"
        );

        // Test with different date
        let mut headers2 = HashMap::new();
        headers2.insert(
            "last-modified".to_string(),
            "Fri, 01 Jan 2021 00:00:00 GMT".to_string(),
        );
        let response2 = S3Response::new(200, "OK", headers2, vec![]);
        assert_eq!(
            response2.get_header("last-modified"),
            Some(&"Fri, 01 Jan 2021 00:00:00 GMT".to_string())
        );

        // Test with recent date
        let mut headers3 = HashMap::new();
        headers3.insert(
            "last-modified".to_string(),
            "Mon, 15 May 2023 14:30:45 GMT".to_string(),
        );
        let response3 = S3Response::new(200, "OK", headers3, vec![]);
        assert_eq!(
            response3.get_header("last-modified"),
            Some(&"Mon, 15 May 2023 14:30:45 GMT".to_string())
        );

        // Test missing Last-Modified header
        let headers4 = HashMap::new();
        let response4 = S3Response::new(200, "OK", headers4, vec![]);
        assert_eq!(
            response4.get_header("last-modified"),
            None,
            "Should return None for missing Last-Modified"
        );

        // Test that Last-Modified is preserved exactly as received
        let mut headers5 = HashMap::new();
        headers5.insert(
            "last-modified".to_string(),
            "Tue, 25 Dec 2024 12:00:00 GMT".to_string(),
        );
        let response5 = S3Response::new(200, "OK", headers5, vec![]);
        let last_mod_value = response5.get_header("last-modified").unwrap();
        assert!(
            last_mod_value.ends_with("GMT"),
            "Last-Modified should end with GMT"
        );
        assert!(
            last_mod_value.contains(','),
            "Last-Modified should contain comma after day name"
        );
    }

    #[test]
    fn test_preserves_custom_s3_metadata_headers() {
        use std::collections::HashMap;

        // Test with single custom metadata header
        let mut headers = HashMap::new();
        headers.insert("x-amz-meta-author".to_string(), "John Doe".to_string());
        headers.insert("content-type".to_string(), "image/jpeg".to_string());

        let response = S3Response::new(200, "OK", headers, vec![]);

        assert_eq!(
            response.get_header("x-amz-meta-author"),
            Some(&"John Doe".to_string()),
            "Should preserve custom x-amz-meta-author header"
        );

        // Test with multiple custom metadata headers
        let mut headers2 = HashMap::new();
        headers2.insert("x-amz-meta-author".to_string(), "Jane Smith".to_string());
        headers2.insert("x-amz-meta-project".to_string(), "yatagarasu".to_string());
        headers2.insert(
            "x-amz-meta-environment".to_string(),
            "production".to_string(),
        );
        headers2.insert("x-amz-meta-version".to_string(), "1.0.0".to_string());
        headers2.insert("content-type".to_string(), "application/json".to_string());

        let response2 = S3Response::new(200, "OK", headers2, vec![]);

        assert_eq!(
            response2.get_header("x-amz-meta-author"),
            Some(&"Jane Smith".to_string())
        );
        assert_eq!(
            response2.get_header("x-amz-meta-project"),
            Some(&"yatagarasu".to_string())
        );
        assert_eq!(
            response2.get_header("x-amz-meta-environment"),
            Some(&"production".to_string())
        );
        assert_eq!(
            response2.get_header("x-amz-meta-version"),
            Some(&"1.0.0".to_string())
        );

        // Test with custom metadata containing special characters
        let mut headers3 = HashMap::new();
        headers3.insert(
            "x-amz-meta-description".to_string(),
            "User uploaded image, processed on 2024-01-15".to_string(),
        );
        headers3.insert(
            "x-amz-meta-tags".to_string(),
            "landscape,mountains,photography".to_string(),
        );

        let response3 = S3Response::new(200, "OK", headers3, vec![]);

        assert_eq!(
            response3.get_header("x-amz-meta-description"),
            Some(&"User uploaded image, processed on 2024-01-15".to_string())
        );
        assert_eq!(
            response3.get_header("x-amz-meta-tags"),
            Some(&"landscape,mountains,photography".to_string())
        );

        // Test that non-existent metadata header returns None
        let headers4 = HashMap::new();
        let response4 = S3Response::new(200, "OK", headers4, vec![]);
        assert_eq!(
            response4.get_header("x-amz-meta-nonexistent"),
            None,
            "Should return None for missing metadata header"
        );

        // Test that metadata values are preserved exactly as received
        let mut headers5 = HashMap::new();
        headers5.insert(
            "x-amz-meta-data".to_string(),
            "  spaces and\ttabs  ".to_string(),
        );
        let response5 = S3Response::new(200, "OK", headers5, vec![]);
        assert_eq!(
            response5.get_header("x-amz-meta-data"),
            Some(&"  spaces and\ttabs  ".to_string()),
            "Should preserve exact value including whitespace"
        );
    }

    #[test]
    fn test_streams_response_body_to_client() {
        use std::collections::HashMap;

        // Test with text content
        let text_body = b"Hello, World!".to_vec();
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());
        headers.insert("content-length".to_string(), "13".to_string());

        let response = S3Response::new(200, "OK", headers, text_body.clone());

        assert_eq!(
            response.body, text_body,
            "Should provide access to text body"
        );
        assert_eq!(response.body.len(), 13, "Body length should be 13 bytes");

        // Test with binary content (simulated image)
        let binary_body = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]; // JPEG header
        let mut headers2 = HashMap::new();
        headers2.insert("content-type".to_string(), "image/jpeg".to_string());

        let response2 = S3Response::new(200, "OK", headers2, binary_body.clone());

        assert_eq!(
            response2.body, binary_body,
            "Should provide access to binary body"
        );
        assert_eq!(
            response2.body[0], 0xFF,
            "First byte should be preserved correctly"
        );

        // Test with large body (simulated streaming)
        let large_body = vec![0u8; 10 * 1024 * 1024]; // 10MB
        let mut headers3 = HashMap::new();
        headers3.insert("content-length".to_string(), (10 * 1024 * 1024).to_string());

        let response3 = S3Response::new(200, "OK", headers3, large_body.clone());

        assert_eq!(
            response3.body.len(),
            10 * 1024 * 1024,
            "Should handle large body for streaming"
        );

        // Test with empty body (HEAD request)
        let empty_body = vec![];
        let headers4 = HashMap::new();

        let response4 = S3Response::new(200, "OK", headers4, empty_body.clone());

        assert_eq!(response4.body.len(), 0, "Should handle empty body");
        assert!(response4.body.is_empty(), "Empty body should be empty");

        // Test with JSON body
        let json_body = br#"{"name":"test","value":123}"#.to_vec();
        let mut headers5 = HashMap::new();
        headers5.insert("content-type".to_string(), "application/json".to_string());

        let response5 = S3Response::new(200, "OK", headers5, json_body.clone());

        assert_eq!(response5.body, json_body, "Should preserve JSON body");

        // Verify body can be accessed as bytes for streaming
        let body_bytes: &[u8] = &response5.body;
        assert_eq!(
            body_bytes.len(),
            json_body.len(),
            "Body bytes should match length"
        );

        // Test chunked streaming simulation
        let content = b"This is a test file for streaming in chunks".to_vec();
        let response6 = S3Response::new(200, "OK", HashMap::new(), content.clone());

        // Simulate reading in chunks
        let chunk_size = 10;
        let chunks: Vec<&[u8]> = response6.body.chunks(chunk_size).collect();

        assert!(
            chunks.len() > 1,
            "Should be able to split body into chunks for streaming"
        );

        let reconstructed: Vec<u8> = chunks
            .iter()
            .flat_map(|&chunk| chunk.iter())
            .copied()
            .collect();
        assert_eq!(
            reconstructed, content,
            "Chunks should reconstruct original content"
        );
    }

    #[test]
    fn test_handles_404_not_found_from_s3() {
        use std::collections::HashMap;

        // Test basic 404 response
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/xml".to_string());
        headers.insert("x-amz-request-id".to_string(), "ABC123".to_string());

        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>NoSuchKey</Code>
    <Message>The specified key does not exist.</Message>
    <Key>nonexistent/file.txt</Key>
    <RequestId>ABC123</RequestId>
</Error>"#
            .to_vec();

        let response = S3Response::new(404, "Not Found", headers, error_body.clone());

        assert_eq!(response.status_code, 404, "Status code should be 404");
        assert_eq!(
            response.status_text, "Not Found",
            "Status text should be 'Not Found'"
        );
        assert!(!response.is_success(), "404 should not be success");
        assert_eq!(
            response.get_header("content-type"),
            Some(&"application/xml".to_string()),
            "Should preserve content-type header"
        );
        assert!(!response.body.is_empty(), "Should have error body");

        // Test 404 with minimal headers
        let headers2 = HashMap::new();
        let response2 = S3Response::new(404, "Not Found", headers2, vec![]);

        assert_eq!(response2.status_code, 404);
        assert!(!response2.is_success());
        assert_eq!(response2.body.len(), 0, "Empty body should be allowed");

        // Test 404 with custom metadata headers (should still be preserved)
        let mut headers3 = HashMap::new();
        headers3.insert("x-amz-request-id".to_string(), "DEF456GHI789".to_string());
        headers3.insert("x-amz-id-2".to_string(), "extended-request-id".to_string());

        let response3 = S3Response::new(404, "Not Found", headers3, vec![]);

        assert_eq!(
            response3.get_header("x-amz-request-id"),
            Some(&"DEF456GHI789".to_string()),
            "Should preserve request ID header"
        );
        assert_eq!(
            response3.get_header("x-amz-id-2"),
            Some(&"extended-request-id".to_string()),
            "Should preserve extended request ID"
        );

        // Verify status code is accessible for error handling
        assert!(
            response.status_code >= 400 && response.status_code < 500,
            "404 is a client error (4xx)"
        );

        // Test that error body can be parsed
        let body_str = String::from_utf8(response.body.clone()).unwrap();
        assert!(
            body_str.contains("NoSuchKey"),
            "Error body should contain error code"
        );
        assert!(
            body_str.contains("The specified key does not exist"),
            "Error body should contain error message"
        );
    }

    #[test]
    fn test_handles_403_forbidden_from_s3() {
        use std::collections::HashMap;

        // Test basic 403 response for access denied
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/xml".to_string());
        headers.insert("x-amz-request-id".to_string(), "XYZ789".to_string());

        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>AccessDenied</Code>
    <Message>Access Denied</Message>
    <RequestId>XYZ789</RequestId>
    <HostId>host-id-string</HostId>
</Error>"#
            .to_vec();

        let response = S3Response::new(403, "Forbidden", headers, error_body.clone());

        assert_eq!(response.status_code, 403, "Status code should be 403");
        assert_eq!(
            response.status_text, "Forbidden",
            "Status text should be 'Forbidden'"
        );
        assert!(!response.is_success(), "403 should not be success");
        assert!(!response.body.is_empty(), "Should have error body");

        // Verify it's a client error
        assert!(
            response.status_code >= 400 && response.status_code < 500,
            "403 is a client error (4xx)"
        );

        // Test that error body can be parsed
        let body_str = String::from_utf8(response.body.clone()).unwrap();
        assert!(
            body_str.contains("AccessDenied"),
            "Error body should contain AccessDenied code"
        );
        assert!(
            body_str.contains("Access Denied"),
            "Error body should contain error message"
        );

        // Test 403 with different error code (e.g., InvalidAccessKeyId)
        let error_body2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InvalidAccessKeyId</Code>
    <Message>The AWS Access Key Id you provided does not exist in our records.</Message>
    <AWSAccessKeyId>INVALIDKEY</AWSAccessKeyId>
</Error>"#
            .to_vec();

        let mut headers2 = HashMap::new();
        headers2.insert("content-type".to_string(), "application/xml".to_string());

        let response2 = S3Response::new(403, "Forbidden", headers2, error_body2.clone());

        assert_eq!(response2.status_code, 403);
        assert!(!response2.is_success());

        let body_str2 = String::from_utf8(response2.body).unwrap();
        assert!(
            body_str2.contains("InvalidAccessKeyId"),
            "Should handle InvalidAccessKeyId error"
        );

        // Test 403 with SignatureDoesNotMatch
        let error_body3 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>SignatureDoesNotMatch</Code>
    <Message>The request signature we calculated does not match the signature you provided.</Message>
</Error>"#
            .to_vec();

        let response3 = S3Response::new(403, "Forbidden", HashMap::new(), error_body3.clone());

        assert_eq!(response3.status_code, 403);
        let body_str3 = String::from_utf8(response3.body).unwrap();
        assert!(
            body_str3.contains("SignatureDoesNotMatch"),
            "Should handle signature mismatch errors"
        );

        // Test 403 with minimal response (no body)
        let response4 = S3Response::new(403, "Forbidden", HashMap::new(), vec![]);

        assert_eq!(response4.status_code, 403);
        assert!(!response4.is_success());
        assert!(
            response4.body.is_empty(),
            "Should handle 403 with empty body"
        );
    }

    #[test]
    fn test_handles_400_bad_request_from_s3() {
        use std::collections::HashMap;

        // Test basic 400 response for invalid request
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/xml".to_string());
        headers.insert("x-amz-request-id".to_string(), "REQ123".to_string());

        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InvalidArgument</Code>
    <Message>Invalid Argument</Message>
    <ArgumentName>marker</ArgumentName>
    <ArgumentValue>invalid-value</ArgumentValue>
    <RequestId>REQ123</RequestId>
</Error>"#
            .to_vec();

        let response = S3Response::new(400, "Bad Request", headers, error_body.clone());

        assert_eq!(response.status_code, 400, "Status code should be 400");
        assert_eq!(
            response.status_text, "Bad Request",
            "Status text should be 'Bad Request'"
        );
        assert!(!response.is_success(), "400 should not be success");
        assert!(!response.body.is_empty(), "Should have error body");

        // Verify it's a client error
        assert!(
            response.status_code >= 400 && response.status_code < 500,
            "400 is a client error (4xx)"
        );

        // Test that error body can be parsed
        let body_str = String::from_utf8(response.body.clone()).unwrap();
        assert!(
            body_str.contains("InvalidArgument"),
            "Error body should contain InvalidArgument code"
        );
        assert!(
            body_str.contains("Invalid Argument"),
            "Error body should contain error message"
        );

        // Test 400 with InvalidBucketName
        let error_body2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InvalidBucketName</Code>
    <Message>The specified bucket is not valid.</Message>
    <BucketName>Invalid_Bucket_Name</BucketName>
</Error>"#
            .to_vec();

        let response2 = S3Response::new(400, "Bad Request", HashMap::new(), error_body2.clone());

        assert_eq!(response2.status_code, 400);
        assert!(!response2.is_success());

        let body_str2 = String::from_utf8(response2.body).unwrap();
        assert!(
            body_str2.contains("InvalidBucketName"),
            "Should handle InvalidBucketName error"
        );

        // Test 400 with MalformedXML
        let error_body3 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>MalformedXML</Code>
    <Message>The XML you provided was not well-formed or did not validate against our published schema.</Message>
</Error>"#
            .to_vec();

        let response3 = S3Response::new(400, "Bad Request", HashMap::new(), error_body3.clone());

        assert_eq!(response3.status_code, 400);
        let body_str3 = String::from_utf8(response3.body).unwrap();
        assert!(
            body_str3.contains("MalformedXML"),
            "Should handle malformed XML errors"
        );

        // Test 400 with InvalidRange (for Range requests)
        let error_body4 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InvalidRange</Code>
    <Message>The requested range is not satisfiable</Message>
    <RangeRequested>bytes=1000-2000</RangeRequested>
    <ActualObjectSize>500</ActualObjectSize>
</Error>"#
            .to_vec();

        let mut headers4 = HashMap::new();
        headers4.insert("content-range".to_string(), "bytes */500".to_string());

        let response4 = S3Response::new(400, "Bad Request", headers4, error_body4.clone());

        assert_eq!(response4.status_code, 400);
        assert_eq!(
            response4.get_header("content-range"),
            Some(&"bytes */500".to_string()),
            "Should preserve content-range header"
        );
        let body_str4 = String::from_utf8(response4.body).unwrap();
        assert!(
            body_str4.contains("InvalidRange"),
            "Should handle invalid range errors"
        );

        // Test 400 with empty body
        let response5 = S3Response::new(400, "Bad Request", HashMap::new(), vec![]);

        assert_eq!(response5.status_code, 400);
        assert!(!response5.is_success());
        assert!(
            response5.body.is_empty(),
            "Should handle 400 with empty body"
        );
    }

    #[test]
    fn test_handles_500_internal_server_error_from_s3() {
        use std::collections::HashMap;

        // Test basic 500 response
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/xml".to_string());
        headers.insert("x-amz-request-id".to_string(), "ERR500".to_string());

        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InternalError</Code>
    <Message>We encountered an internal error. Please try again.</Message>
    <RequestId>ERR500</RequestId>
</Error>"#
            .to_vec();

        let response = S3Response::new(500, "Internal Server Error", headers, error_body.clone());

        assert_eq!(response.status_code, 500, "Status code should be 500");
        assert_eq!(
            response.status_text, "Internal Server Error",
            "Status text should be 'Internal Server Error'"
        );
        assert!(!response.is_success(), "500 should not be success");
        assert!(!response.body.is_empty(), "Should have error body");

        // Verify it's a server error
        assert!(
            response.status_code >= 500 && response.status_code < 600,
            "500 is a server error (5xx)"
        );

        // Test that error body can be parsed
        let body_str = String::from_utf8(response.body.clone()).unwrap();
        assert!(
            body_str.contains("InternalError"),
            "Error body should contain InternalError code"
        );
        assert!(
            body_str.contains("We encountered an internal error"),
            "Error body should contain error message"
        );

        // Test 500 with minimal headers
        let headers2 = HashMap::new();
        let response2 = S3Response::new(500, "Internal Server Error", headers2, vec![]);

        assert_eq!(response2.status_code, 500);
        assert!(!response2.is_success());
        assert_eq!(response2.body.len(), 0, "Empty body should be allowed");

        // Test 500 with request ID header preserved
        let mut headers3 = HashMap::new();
        headers3.insert(
            "x-amz-request-id".to_string(),
            "500-ERROR-ID-123".to_string(),
        );
        headers3.insert("x-amz-id-2".to_string(), "extended-id".to_string());

        let response3 = S3Response::new(500, "Internal Server Error", headers3, vec![]);

        assert_eq!(
            response3.get_header("x-amz-request-id"),
            Some(&"500-ERROR-ID-123".to_string()),
            "Should preserve request ID for debugging"
        );
        assert_eq!(
            response3.get_header("x-amz-id-2"),
            Some(&"extended-id".to_string()),
            "Should preserve extended request ID for AWS support"
        );

        // Test that 500 errors should be retryable (implementation detail)
        // Unlike 4xx errors, 5xx errors are typically transient
        assert!(
            response.status_code >= 500,
            "Server errors (5xx) are typically retryable"
        );
        assert!(
            response.status_code < 400 || response.status_code >= 500,
            "500 is not a client error"
        );
    }

    #[test]
    fn test_handles_503_service_unavailable_from_s3() {
        use std::collections::HashMap;

        // Test 503 with SlowDown error (rate limiting)
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/xml".to_string());
        headers.insert("x-amz-request-id".to_string(), "SLOWDOWN123".to_string());
        headers.insert("retry-after".to_string(), "5".to_string());

        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>SlowDown</Code>
    <Message>Please reduce your request rate.</Message>
    <RequestId>SLOWDOWN123</RequestId>
</Error>"#
            .to_vec();

        let response = S3Response::new(503, "Service Unavailable", headers, error_body.clone());

        assert_eq!(response.status_code, 503, "Status code should be 503");
        assert_eq!(
            response.status_text, "Service Unavailable",
            "Status text should be 'Service Unavailable'"
        );
        assert!(!response.is_success(), "503 should not be success");
        assert!(!response.body.is_empty(), "Should have error body");

        // Verify it's a server error
        assert!(
            response.status_code >= 500 && response.status_code < 600,
            "503 is a server error (5xx)"
        );

        // Test that error body can be parsed
        let body_str = String::from_utf8(response.body.clone()).unwrap();
        assert!(
            body_str.contains("SlowDown"),
            "Error body should contain SlowDown code"
        );
        assert!(
            body_str.contains("Please reduce your request rate"),
            "Error body should contain rate limiting message"
        );

        // Verify Retry-After header is preserved
        assert_eq!(
            response.get_header("retry-after"),
            Some(&"5".to_string()),
            "Should preserve Retry-After header for backoff"
        );

        // Test 503 with ServiceUnavailable error
        let error_body2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>ServiceUnavailable</Code>
    <Message>Service is temporarily unavailable. Please retry.</Message>
</Error>"#
            .to_vec();

        let mut headers2 = HashMap::new();
        headers2.insert("content-type".to_string(), "application/xml".to_string());

        let response2 = S3Response::new(503, "Service Unavailable", headers2, error_body2.clone());

        assert_eq!(response2.status_code, 503);
        assert!(!response2.is_success());

        let body_str2 = String::from_utf8(response2.body).unwrap();
        assert!(
            body_str2.contains("ServiceUnavailable"),
            "Should handle ServiceUnavailable error"
        );

        // Test 503 with minimal response (no body)
        let response3 = S3Response::new(503, "Service Unavailable", HashMap::new(), vec![]);

        assert_eq!(response3.status_code, 503);
        assert!(!response3.is_success());
        assert!(
            response3.body.is_empty(),
            "Should handle 503 with empty body"
        );

        // Test 503 with request ID preserved
        let mut headers4 = HashMap::new();
        headers4.insert(
            "x-amz-request-id".to_string(),
            "503-UNAVAIL-456".to_string(),
        );
        headers4.insert("x-amz-id-2".to_string(), "extended-id".to_string());

        let response4 = S3Response::new(503, "Service Unavailable", headers4, vec![]);

        assert_eq!(
            response4.get_header("x-amz-request-id"),
            Some(&"503-UNAVAIL-456".to_string()),
            "Should preserve request ID for debugging"
        );
        assert_eq!(
            response4.get_header("x-amz-id-2"),
            Some(&"extended-id".to_string()),
            "Should preserve extended request ID"
        );

        // Verify 503 is retryable with exponential backoff
        assert!(
            response.status_code >= 500,
            "Server errors (5xx) should be retried with backoff"
        );
    }

    #[test]
    fn test_parses_s3_xml_error_response_body() {
        use std::collections::HashMap;

        // Test parsing complete S3 error response
        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>NoSuchKey</Code>
    <Message>The specified key does not exist.</Message>
    <Key>path/to/nonexistent.txt</Key>
    <RequestId>ABC123DEF456</RequestId>
    <HostId>host-id-string-here</HostId>
</Error>"#
            .to_vec();

        let response = S3Response::new(404, "Not Found", HashMap::new(), error_body.clone());

        // Convert body to string for parsing
        let body_str = String::from_utf8(response.body.clone()).unwrap();

        // Verify XML structure is present
        assert!(
            body_str.contains("<?xml version=\"1.0\""),
            "Should contain XML declaration"
        );
        assert!(
            body_str.contains("<Error>"),
            "Should contain Error root element"
        );
        assert!(
            body_str.contains("</Error>"),
            "Should have closing Error tag"
        );

        // Verify error code is extractable
        assert!(
            body_str.contains("<Code>NoSuchKey</Code>"),
            "Should contain error code"
        );

        // Verify error message is extractable
        assert!(
            body_str.contains("<Message>The specified key does not exist.</Message>"),
            "Should contain error message"
        );

        // Verify additional fields are present
        assert!(
            body_str.contains("<Key>path/to/nonexistent.txt</Key>"),
            "Should contain Key field"
        );
        assert!(
            body_str.contains("<RequestId>ABC123DEF456</RequestId>"),
            "Should contain RequestId"
        );
        assert!(
            body_str.contains("<HostId>host-id-string-here</HostId>"),
            "Should contain HostId"
        );

        // Test parsing AccessDenied error
        let error_body2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>AccessDenied</Code>
    <Message>Access Denied</Message>
    <RequestId>XYZ789</RequestId>
</Error>"#
            .to_vec();

        let response2 = S3Response::new(403, "Forbidden", HashMap::new(), error_body2.clone());
        let body_str2 = String::from_utf8(response2.body).unwrap();

        assert!(body_str2.contains("<Code>AccessDenied</Code>"));
        assert!(body_str2.contains("<Message>Access Denied</Message>"));

        // Test parsing error with special characters in message
        let error_body3 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InvalidArgument</Code>
    <Message>Invalid Argument: marker must be a valid token &amp; not empty</Message>
    <ArgumentName>marker</ArgumentName>
    <ArgumentValue></ArgumentValue>
</Error>"#
            .to_vec();

        let response3 = S3Response::new(400, "Bad Request", HashMap::new(), error_body3.clone());
        let body_str3 = String::from_utf8(response3.body).unwrap();

        assert!(
            body_str3.contains("<Code>InvalidArgument</Code>"),
            "Should handle error codes"
        );
        assert!(body_str3.contains("&amp;"), "Should preserve XML entities");

        // Test malformed/minimal XML
        let error_body4 = b"<Error><Code>InternalError</Code></Error>".to_vec();

        let response4 = S3Response::new(500, "Internal Server Error", HashMap::new(), error_body4);
        let body_str4 = String::from_utf8(response4.body).unwrap();

        assert!(
            body_str4.contains("<Code>InternalError</Code>"),
            "Should handle minimal XML"
        );

        // Test empty error body
        let response5 = S3Response::new(500, "Internal Server Error", HashMap::new(), vec![]);

        assert!(response5.body.is_empty(), "Should handle empty error body");

        // Test non-XML error body
        let response6 = S3Response::new(
            500,
            "Internal Server Error",
            HashMap::new(),
            b"Internal Server Error".to_vec(),
        );

        let body_str6 = String::from_utf8(response6.body).unwrap();
        assert_eq!(
            body_str6, "Internal Server Error",
            "Should handle non-XML error body"
        );
    }

    #[test]
    fn test_extracts_error_code_and_message_from_s3_error_response() {
        use std::collections::HashMap;

        // Test extracting error code and message from NoSuchKey error
        let error_body = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>NoSuchKey</Code>
    <Message>The specified key does not exist.</Message>
    <Key>path/to/nonexistent.txt</Key>
    <RequestId>ABC123</RequestId>
</Error>"#
            .to_vec();

        let response = S3Response::new(404, "Not Found", HashMap::new(), error_body);

        assert_eq!(
            response.get_error_code(),
            Some("NoSuchKey".to_string()),
            "Should extract error code"
        );
        assert_eq!(
            response.get_error_message(),
            Some("The specified key does not exist.".to_string()),
            "Should extract error message"
        );

        // Test extracting from AccessDenied error
        let error_body2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>AccessDenied</Code>
    <Message>Access Denied</Message>
</Error>"#
            .to_vec();

        let response2 = S3Response::new(403, "Forbidden", HashMap::new(), error_body2);

        assert_eq!(response2.get_error_code(), Some("AccessDenied".to_string()));
        assert_eq!(
            response2.get_error_message(),
            Some("Access Denied".to_string())
        );

        // Test extracting from SlowDown error
        let error_body3 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>SlowDown</Code>
    <Message>Please reduce your request rate.</Message>
</Error>"#
            .to_vec();

        let response3 = S3Response::new(503, "Service Unavailable", HashMap::new(), error_body3);

        assert_eq!(response3.get_error_code(), Some("SlowDown".to_string()));
        assert_eq!(
            response3.get_error_message(),
            Some("Please reduce your request rate.".to_string())
        );

        // Test with minimal XML (only code)
        let error_body4 = b"<Error><Code>InternalError</Code></Error>".to_vec();

        let response4 = S3Response::new(500, "Internal Server Error", HashMap::new(), error_body4);

        assert_eq!(
            response4.get_error_code(),
            Some("InternalError".to_string())
        );
        assert_eq!(
            response4.get_error_message(),
            None,
            "Should return None when Message tag missing"
        );

        // Test with empty body
        let response5 = S3Response::new(500, "Internal Server Error", HashMap::new(), vec![]);

        assert_eq!(
            response5.get_error_code(),
            None,
            "Should return None for empty body"
        );
        assert_eq!(
            response5.get_error_message(),
            None,
            "Should return None for empty body"
        );

        // Test with non-XML body
        let response6 = S3Response::new(
            500,
            "Internal Server Error",
            HashMap::new(),
            b"Internal Server Error".to_vec(),
        );

        assert_eq!(
            response6.get_error_code(),
            None,
            "Should return None for non-XML body"
        );
        assert_eq!(
            response6.get_error_message(),
            None,
            "Should return None for non-XML body"
        );

        // Test error message with special characters
        let error_body7 = br#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
    <Code>InvalidArgument</Code>
    <Message>Invalid Argument: value must be &gt; 0</Message>
</Error>"#
            .to_vec();

        let response7 = S3Response::new(400, "Bad Request", HashMap::new(), error_body7);

        assert_eq!(
            response7.get_error_code(),
            Some("InvalidArgument".to_string())
        );
        assert_eq!(
            response7.get_error_message(),
            Some("Invalid Argument: value must be &gt; 0".to_string()),
            "Should preserve XML entities in message"
        );
    }

    #[test]
    fn test_maps_s3_errors_to_appropriate_http_status_codes() {
        // Test 404 errors
        assert_eq!(
            map_s3_error_to_status("NoSuchKey"),
            404,
            "NoSuchKey should map to 404"
        );
        assert_eq!(
            map_s3_error_to_status("NoSuchBucket"),
            404,
            "NoSuchBucket should map to 404"
        );

        // Test 403 errors
        assert_eq!(
            map_s3_error_to_status("AccessDenied"),
            403,
            "AccessDenied should map to 403"
        );
        assert_eq!(
            map_s3_error_to_status("InvalidAccessKeyId"),
            403,
            "InvalidAccessKeyId should map to 403"
        );
        assert_eq!(
            map_s3_error_to_status("SignatureDoesNotMatch"),
            403,
            "SignatureDoesNotMatch should map to 403"
        );

        // Test 400 errors
        assert_eq!(
            map_s3_error_to_status("InvalidArgument"),
            400,
            "InvalidArgument should map to 400"
        );
        assert_eq!(
            map_s3_error_to_status("InvalidBucketName"),
            400,
            "InvalidBucketName should map to 400"
        );
        assert_eq!(
            map_s3_error_to_status("InvalidRange"),
            400,
            "InvalidRange should map to 400"
        );
        assert_eq!(
            map_s3_error_to_status("MalformedXML"),
            400,
            "MalformedXML should map to 400"
        );

        // Test 409 errors
        assert_eq!(
            map_s3_error_to_status("BucketAlreadyExists"),
            409,
            "BucketAlreadyExists should map to 409"
        );
        assert_eq!(
            map_s3_error_to_status("BucketNotEmpty"),
            409,
            "BucketNotEmpty should map to 409"
        );

        // Test 500 errors
        assert_eq!(
            map_s3_error_to_status("InternalError"),
            500,
            "InternalError should map to 500"
        );

        // Test 503 errors
        assert_eq!(
            map_s3_error_to_status("SlowDown"),
            503,
            "SlowDown should map to 503"
        );
        assert_eq!(
            map_s3_error_to_status("ServiceUnavailable"),
            503,
            "ServiceUnavailable should map to 503"
        );

        // Test unknown error code (should default to 500)
        assert_eq!(
            map_s3_error_to_status("UnknownError"),
            500,
            "Unknown errors should default to 500"
        );
        assert_eq!(
            map_s3_error_to_status(""),
            500,
            "Empty error code should default to 500"
        );
    }
}
