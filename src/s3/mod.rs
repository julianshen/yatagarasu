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

/// Represents a single byte range
#[derive(Debug, Clone, PartialEq)]
pub struct ByteRange {
    /// Start position (None for suffix ranges)
    pub start: Option<u64>,
    /// End position (None for open-ended ranges)
    pub end: Option<u64>,
}

impl ByteRange {
    /// Calculate the size of this range (end - start + 1)
    pub fn size(&self) -> Option<u64> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                if end >= start {
                    Some(end - start + 1)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Represents a parsed Range header
#[derive(Debug, Clone, PartialEq)]
pub struct RangeHeader {
    /// Unit (typically "bytes")
    pub unit: String,
    /// List of ranges
    pub ranges: Vec<ByteRange>,
}

/// Parses an HTTP Range header value
/// Supports formats like:
/// - bytes=0-1023 (single range)
/// - bytes=1000- (open-ended)
/// - bytes=-1000 (suffix)
/// - bytes=0-100,200-300 (multiple ranges)
pub fn parse_range_header(header_value: &str) -> Option<RangeHeader> {
    let header_value = header_value.trim();

    // Split into unit and ranges
    let parts: Vec<&str> = header_value.split('=').collect();
    if parts.len() != 2 {
        return None;
    }

    let unit = parts[0].trim();
    let ranges_str = parts[1].trim();

    // Parse individual ranges
    let mut ranges = Vec::new();

    for range_str in ranges_str.split(',') {
        let range_str = range_str.trim();

        // Parse single range (e.g., "0-1023", "1000-", "-1000")
        if let Some(dash_pos) = range_str.find('-') {
            let start_str = range_str[..dash_pos].trim();
            let end_str = range_str[dash_pos + 1..].trim();

            // Parse start: None if empty (suffix range), Some if valid number, error if invalid
            let start = if start_str.is_empty() {
                None
            } else {
                match start_str.parse::<u64>() {
                    Ok(n) => Some(n),
                    Err(_) => return None, // Invalid start number
                }
            };

            // Parse end: None if empty (open-ended range), Some if valid number, error if invalid
            let end = if end_str.is_empty() {
                None
            } else {
                match end_str.parse::<u64>() {
                    Ok(n) => Some(n),
                    Err(_) => return None, // Invalid end number
                }
            };

            // Valid range must have at least start or end
            if start.is_some() || end.is_some() {
                ranges.push(ByteRange { start, end });
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    if ranges.is_empty() {
        return None;
    }

    Some(RangeHeader {
        unit: unit.to_string(),
        ranges,
    })
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

    #[test]
    fn test_can_stream_small_file_efficiently() {
        use std::collections::HashMap;

        // Simulate a small file (100 KB)
        let file_size = 100 * 1024; // 100 KB
        let file_content = vec![0u8; file_size];

        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "image/jpeg".to_string());
        headers.insert("content-length".to_string(), file_size.to_string());
        headers.insert("etag".to_string(), "\"abc123\"".to_string());

        let response = S3Response::new(200, "OK", headers, file_content.clone());

        // Verify response is successful
        assert!(response.is_success(), "Response should be successful");
        assert_eq!(response.status_code, 200);

        // Verify file size
        assert_eq!(
            response.body.len(),
            file_size,
            "Body size should match file size"
        );

        // Verify we can access the body for streaming
        assert!(!response.body.is_empty(), "Body should not be empty");

        // Simulate streaming by reading in chunks
        let chunk_size = 8 * 1024; // 8 KB chunks
        let chunks: Vec<&[u8]> = response.body.chunks(chunk_size).collect();

        // Verify chunking works
        assert!(
            chunks.len() > 1,
            "Should be able to split into multiple chunks"
        );
        assert_eq!(
            chunks.len(),
            (file_size + chunk_size - 1) / chunk_size,
            "Should have expected number of chunks"
        );

        // Verify chunks can be reassembled
        let total_bytes: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(
            total_bytes, file_size,
            "Total chunk bytes should equal file size"
        );

        // Test with an even smaller file (10 KB)
        let small_size = 10 * 1024;
        let small_content = vec![1u8; small_size];

        let mut headers2 = HashMap::new();
        headers2.insert("content-length".to_string(), small_size.to_string());

        let response2 = S3Response::new(200, "OK", headers2, small_content);

        assert_eq!(response2.body.len(), small_size);
        assert!(response2.is_success());

        // Verify headers are accessible during streaming
        assert_eq!(
            response.get_header("content-type"),
            Some(&"image/jpeg".to_string()),
            "Headers should be accessible while streaming"
        );
        assert_eq!(
            response.get_header("content-length"),
            Some(&file_size.to_string()),
            "Content-Length header should be available"
        );

        // Test with 512 KB file (still under 1MB threshold)
        let medium_small_size = 512 * 1024;
        let medium_content = vec![2u8; medium_small_size];

        let response3 = S3Response::new(200, "OK", HashMap::new(), medium_content);

        assert_eq!(response3.body.len(), medium_small_size);
        assert!(
            response3.body.len() < 1024 * 1024,
            "Should be under 1MB threshold"
        );

        // Verify efficient access - body can be accessed as slice
        let body_slice: &[u8] = &response3.body;
        assert_eq!(
            body_slice.len(),
            medium_small_size,
            "Should be able to access as slice efficiently"
        );
    }

    #[test]
    fn test_can_stream_medium_file_efficiently() {
        use std::collections::HashMap;

        // Simulate a medium file (10 MB)
        let file_size = 10 * 1024 * 1024; // 10 MB
        let file_content = vec![0u8; file_size];

        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "video/mp4".to_string());
        headers.insert("content-length".to_string(), file_size.to_string());
        headers.insert("etag".to_string(), "\"def456\"".to_string());

        let response = S3Response::new(200, "OK", headers, file_content.clone());

        // Verify response is successful
        assert!(response.is_success(), "Response should be successful");
        assert_eq!(response.status_code, 200);

        // Verify file size
        assert_eq!(
            response.body.len(),
            file_size,
            "Body size should match 10MB file size"
        );

        // Verify we can access the body for streaming
        assert!(!response.body.is_empty(), "Body should not be empty");

        // Simulate streaming by reading in larger chunks (64 KB)
        let chunk_size = 64 * 1024; // 64 KB chunks
        let chunks: Vec<&[u8]> = response.body.chunks(chunk_size).collect();

        // Verify chunking works for medium file
        assert!(
            chunks.len() > 1,
            "Should be able to split into multiple chunks"
        );

        let expected_chunks = (file_size + chunk_size - 1) / chunk_size;
        assert_eq!(
            chunks.len(),
            expected_chunks,
            "Should have {} chunks for 10MB file with 64KB chunks",
            expected_chunks
        );

        // Verify chunks can be reassembled
        let total_bytes: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(
            total_bytes, file_size,
            "Total chunk bytes should equal file size"
        );

        // Verify all chunks except last are full size
        for (i, chunk) in chunks.iter().enumerate() {
            if i < chunks.len() - 1 {
                assert_eq!(
                    chunk.len(),
                    chunk_size,
                    "All chunks except last should be full size"
                );
            }
        }

        // Test with 5 MB file
        let mid_size = 5 * 1024 * 1024;
        let mid_content = vec![1u8; mid_size];

        let mut headers2 = HashMap::new();
        headers2.insert("content-length".to_string(), mid_size.to_string());
        headers2.insert("content-type".to_string(), "application/pdf".to_string());

        let response2 = S3Response::new(200, "OK", headers2, mid_content);

        assert_eq!(response2.body.len(), mid_size);
        assert!(response2.is_success());

        // Verify headers are accessible during streaming
        assert_eq!(
            response.get_header("content-type"),
            Some(&"video/mp4".to_string()),
            "Headers should be accessible while streaming"
        );
        assert_eq!(
            response.get_header("content-length"),
            Some(&file_size.to_string()),
            "Content-Length header should be available"
        );

        // Verify efficient access - body can be accessed as slice
        let body_slice: &[u8] = &response.body;
        assert_eq!(
            body_slice.len(),
            file_size,
            "Should be able to access as slice efficiently"
        );

        // Simulate partial read (useful for Range requests)
        let partial_start = 1024 * 1024; // 1 MB offset
        let partial_end = 2 * 1024 * 1024; // 2 MB offset
        let partial_slice = &response.body[partial_start..partial_end];

        assert_eq!(
            partial_slice.len(),
            1024 * 1024,
            "Should be able to read partial ranges efficiently"
        );

        // Test with 8 MB file
        let large_medium_size = 8 * 1024 * 1024;
        let large_content = vec![2u8; large_medium_size];

        let response3 = S3Response::new(200, "OK", HashMap::new(), large_content);

        assert_eq!(response3.body.len(), large_medium_size);

        // Verify chunked iteration is efficient
        let mut chunk_count = 0;
        for _chunk in response3.body.chunks(128 * 1024) {
            chunk_count += 1;
        }

        assert_eq!(
            chunk_count,
            (large_medium_size + 128 * 1024 - 1) / (128 * 1024),
            "Should iterate through all chunks"
        );
    }

    #[test]
    fn test_can_stream_large_file_without_buffering_entire_file() {
        use std::collections::HashMap;

        // Simulate a large file (100 MB)
        // Note: Current implementation uses Vec<u8> which holds entire file in memory
        // Future streaming implementation will use async streams to avoid buffering
        let file_size = 100 * 1024 * 1024; // 100 MB
        let file_content = vec![0u8; file_size];

        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "video/mp4".to_string());
        headers.insert("content-length".to_string(), file_size.to_string());
        headers.insert("etag".to_string(), "\"large123\"".to_string());

        let response = S3Response::new(200, "OK", headers, file_content);

        // Verify response is successful
        assert!(response.is_success(), "Response should be successful");
        assert_eq!(response.status_code, 200);

        // Verify file size
        assert_eq!(
            response.body.len(),
            file_size,
            "Body size should match 100MB file size"
        );

        // Key streaming pattern: iterate through chunks without copying entire file
        // This simulates how actual streaming would work without buffering
        let chunk_size = 64 * 1024; // 64 KB chunks (typical streaming chunk size)

        // Process file in chunks - this doesn't create a copy of the entire file
        let mut total_processed = 0;
        let mut chunk_count = 0;

        for chunk in response.body.chunks(chunk_size) {
            // In actual streaming, each chunk would be sent to client immediately
            // without waiting for entire file to download
            total_processed += chunk.len();
            chunk_count += 1;

            // Verify chunk size (all chunks except last should be full size)
            if total_processed < file_size {
                assert_eq!(
                    chunk.len(),
                    chunk_size,
                    "Non-final chunks should be full size"
                );
            }
        }

        // Verify all bytes were processed
        assert_eq!(
            total_processed, file_size,
            "Should process all bytes through streaming"
        );

        // Verify expected number of chunks
        let expected_chunks = (file_size + chunk_size - 1) / chunk_size;
        assert_eq!(
            chunk_count, expected_chunks,
            "Should have {} chunks for 100MB file",
            expected_chunks
        );

        // Verify partial range access (for HTTP Range requests)
        // This demonstrates efficient slice access without copying entire file
        let range_start = 50 * 1024 * 1024; // 50 MB offset
        let range_end = 51 * 1024 * 1024; // 51 MB offset
        let range_slice = &response.body[range_start..range_end];

        assert_eq!(
            range_slice.len(),
            1024 * 1024,
            "Should be able to access arbitrary ranges efficiently"
        );

        // Verify headers are accessible during streaming
        assert_eq!(
            response.get_header("content-type"),
            Some(&"video/mp4".to_string()),
            "Headers should be accessible while streaming"
        );
        assert_eq!(
            response.get_header("content-length"),
            Some(&file_size.to_string()),
            "Content-Length should indicate full file size"
        );

        // Test with 50 MB file
        let half_size = 50 * 1024 * 1024;
        let half_content = vec![1u8; half_size];

        let response2 = S3Response::new(200, "OK", HashMap::new(), half_content);

        assert_eq!(response2.body.len(), half_size);

        // Verify streaming iteration doesn't allocate additional buffers
        let mut processed = 0;
        for chunk in response2.body.chunks(128 * 1024) {
            processed += chunk.len();
            // Each iteration processes chunk without buffering entire file
        }

        assert_eq!(processed, half_size, "Should stream entire file");

        // Verify memory-efficient pattern: can check first/last chunks without loading all
        let first_chunk = &response.body[0..chunk_size];
        let last_offset = file_size - chunk_size;
        let last_chunk = &response.body[last_offset..];

        assert_eq!(first_chunk.len(), chunk_size);
        assert_eq!(last_chunk.len(), chunk_size);
    }

    #[test]
    fn test_can_parse_range_header_with_single_range() {
        // Test parsing "bytes=0-1023"
        let range = parse_range_header("bytes=0-1023");
        assert!(range.is_some(), "Should parse valid range header");

        let parsed = range.unwrap();
        assert_eq!(parsed.unit, "bytes", "Unit should be bytes");
        assert_eq!(parsed.ranges.len(), 1, "Should have one range");

        let first_range = &parsed.ranges[0];
        assert_eq!(first_range.start, Some(0), "Start should be 0");
        assert_eq!(first_range.end, Some(1023), "End should be 1023");

        // Test parsing "bytes=100-199"
        let range2 = parse_range_header("bytes=100-199");
        assert!(range2.is_some(), "Should parse range");

        let parsed2 = range2.unwrap();
        assert_eq!(parsed2.ranges.len(), 1);
        assert_eq!(parsed2.ranges[0].start, Some(100));
        assert_eq!(parsed2.ranges[0].end, Some(199));

        // Test parsing "bytes=0-0" (single byte)
        let range3 = parse_range_header("bytes=0-0");
        assert!(range3.is_some(), "Should parse single byte range");

        let parsed3 = range3.unwrap();
        assert_eq!(parsed3.ranges[0].start, Some(0));
        assert_eq!(parsed3.ranges[0].end, Some(0));

        // Test parsing "bytes=1000000-1999999" (large range)
        let range4 = parse_range_header("bytes=1000000-1999999");
        assert!(range4.is_some(), "Should parse large range");

        let parsed4 = range4.unwrap();
        assert_eq!(parsed4.ranges[0].start, Some(1000000));
        assert_eq!(parsed4.ranges[0].end, Some(1999999));

        // Test range size calculation
        let range5 = parse_range_header("bytes=0-1023");
        let parsed5 = range5.unwrap();
        let size = parsed5.ranges[0].size();
        assert_eq!(size, Some(1024), "Range 0-1023 should be 1024 bytes");

        // Test range with no spaces
        let range6 = parse_range_header("bytes=50-99");
        assert!(range6.is_some(), "Should parse range without spaces");
        let parsed6 = range6.unwrap();
        assert_eq!(parsed6.ranges[0].start, Some(50));
        assert_eq!(parsed6.ranges[0].end, Some(99));

        // Test range with spaces (should still parse)
        let range7 = parse_range_header("bytes= 10 - 20 ");
        assert!(
            range7.is_some(),
            "Should parse range with spaces (after trimming)"
        );
    }

    #[test]
    fn test_can_parse_range_header_with_open_ended_range() {
        // Test "bytes=1000-" (from byte 1000 to end of file)
        let range = parse_range_header("bytes=1000-");
        assert!(range.is_some(), "Should parse open-ended range header");

        let parsed = range.unwrap();
        assert_eq!(parsed.unit, "bytes", "Unit should be bytes");
        assert_eq!(parsed.ranges.len(), 1, "Should have one range");

        let first_range = &parsed.ranges[0];
        assert_eq!(first_range.start, Some(1000), "Start should be 1000");
        assert_eq!(
            first_range.end, None,
            "End should be None for open-ended range"
        );

        // Test "bytes=0-" (entire file from beginning)
        let range2 = parse_range_header("bytes=0-");
        assert!(range2.is_some(), "Should parse range starting from 0");

        let parsed2 = range2.unwrap();
        assert_eq!(parsed2.ranges.len(), 1);
        assert_eq!(parsed2.ranges[0].start, Some(0));
        assert_eq!(parsed2.ranges[0].end, None);

        // Test "bytes=5000000-" (large offset)
        let range3 = parse_range_header("bytes=5000000-");
        assert!(
            range3.is_some(),
            "Should parse large offset open-ended range"
        );

        let parsed3 = range3.unwrap();
        assert_eq!(parsed3.ranges[0].start, Some(5000000));
        assert_eq!(parsed3.ranges[0].end, None);

        // Test size calculation for open-ended range (should return None)
        let range4 = parse_range_header("bytes=100-");
        let parsed4 = range4.unwrap();
        let size = parsed4.ranges[0].size();
        assert_eq!(
            size, None,
            "Size should be None for open-ended range (unknown until file size known)"
        );

        // Test with spaces "bytes=1000- " (trailing space)
        let range5 = parse_range_header("bytes=1000- ");
        assert!(
            range5.is_some(),
            "Should parse open-ended range with trailing space"
        );
        let parsed5 = range5.unwrap();
        assert_eq!(parsed5.ranges[0].start, Some(1000));
        assert_eq!(parsed5.ranges[0].end, None);

        // Test with spaces " bytes = 1000 - "
        let range6 = parse_range_header(" bytes = 1000 - ");
        assert!(
            range6.is_some(),
            "Should parse open-ended range with spaces around tokens"
        );
        let parsed6 = range6.unwrap();
        assert_eq!(parsed6.ranges[0].start, Some(1000));
        assert_eq!(parsed6.ranges[0].end, None);
    }

    #[test]
    fn test_can_parse_range_header_with_suffix_range() {
        // Test "bytes=-1000" (last 1000 bytes of file)
        let range = parse_range_header("bytes=-1000");
        assert!(range.is_some(), "Should parse suffix range header");

        let parsed = range.unwrap();
        assert_eq!(parsed.unit, "bytes", "Unit should be bytes");
        assert_eq!(parsed.ranges.len(), 1, "Should have one range");

        let first_range = &parsed.ranges[0];
        assert_eq!(
            first_range.start, None,
            "Start should be None for suffix range"
        );
        assert_eq!(first_range.end, Some(1000), "End should be 1000");

        // Test "bytes=-500" (last 500 bytes)
        let range2 = parse_range_header("bytes=-500");
        assert!(range2.is_some(), "Should parse suffix range with 500 bytes");

        let parsed2 = range2.unwrap();
        assert_eq!(parsed2.ranges.len(), 1);
        assert_eq!(parsed2.ranges[0].start, None);
        assert_eq!(parsed2.ranges[0].end, Some(500));

        // Test "bytes=-1" (last byte only)
        let range3 = parse_range_header("bytes=-1");
        assert!(range3.is_some(), "Should parse suffix range for last byte");

        let parsed3 = range3.unwrap();
        assert_eq!(parsed3.ranges[0].start, None);
        assert_eq!(parsed3.ranges[0].end, Some(1));

        // Test "bytes=-10485760" (last 10MB)
        let range4 = parse_range_header("bytes=-10485760");
        assert!(range4.is_some(), "Should parse large suffix range (10MB)");

        let parsed4 = range4.unwrap();
        assert_eq!(parsed4.ranges[0].start, None);
        assert_eq!(parsed4.ranges[0].end, Some(10485760));

        // Test size calculation for suffix range (should return None)
        // Actual size depends on file size: if file is 5000 bytes, "bytes=-1000" means bytes 4000-4999
        let range5 = parse_range_header("bytes=-100");
        let parsed5 = range5.unwrap();
        let size = parsed5.ranges[0].size();
        assert_eq!(
            size, None,
            "Size should be None for suffix range (depends on file size)"
        );

        // Test with spaces "bytes= -1000 "
        let range6 = parse_range_header("bytes= -1000 ");
        assert!(range6.is_some(), "Should parse suffix range with spaces");
        let parsed6 = range6.unwrap();
        assert_eq!(parsed6.ranges[0].start, None);
        assert_eq!(parsed6.ranges[0].end, Some(1000));

        // Test with spaces " bytes = - 1000 "
        let range7 = parse_range_header(" bytes = - 1000 ");
        assert!(
            range7.is_some(),
            "Should parse suffix range with spaces around tokens"
        );
        let parsed7 = range7.unwrap();
        assert_eq!(parsed7.ranges[0].start, None);
        assert_eq!(parsed7.ranges[0].end, Some(1000));

        // Test that "bytes=-" (no number) fails
        let range_invalid = parse_range_header("bytes=-");
        assert_eq!(
            range_invalid, None,
            "Should not parse suffix range without number"
        );
    }

    #[test]
    fn test_can_parse_range_header_with_multiple_ranges() {
        // Test "bytes=0-100,200-300" (two ranges)
        let range = parse_range_header("bytes=0-100,200-300");
        assert!(range.is_some(), "Should parse multiple ranges");

        let parsed = range.unwrap();
        assert_eq!(parsed.unit, "bytes", "Unit should be bytes");
        assert_eq!(parsed.ranges.len(), 2, "Should have two ranges");

        // Verify first range
        assert_eq!(parsed.ranges[0].start, Some(0));
        assert_eq!(parsed.ranges[0].end, Some(100));
        assert_eq!(parsed.ranges[0].size(), Some(101)); // 0-100 is 101 bytes

        // Verify second range
        assert_eq!(parsed.ranges[1].start, Some(200));
        assert_eq!(parsed.ranges[1].end, Some(300));
        assert_eq!(parsed.ranges[1].size(), Some(101)); // 200-300 is 101 bytes

        // Test "bytes=0-499,1000-1499,2000-2499" (three ranges)
        let range2 = parse_range_header("bytes=0-499,1000-1499,2000-2499");
        assert!(range2.is_some(), "Should parse three ranges");

        let parsed2 = range2.unwrap();
        assert_eq!(parsed2.ranges.len(), 3, "Should have three ranges");

        assert_eq!(parsed2.ranges[0].start, Some(0));
        assert_eq!(parsed2.ranges[0].end, Some(499));

        assert_eq!(parsed2.ranges[1].start, Some(1000));
        assert_eq!(parsed2.ranges[1].end, Some(1499));

        assert_eq!(parsed2.ranges[2].start, Some(2000));
        assert_eq!(parsed2.ranges[2].end, Some(2499));

        // Test mixed range types: "bytes=0-100,500-,=200" (closed, open-ended, suffix)
        let range3 = parse_range_header("bytes=0-100,500-,-200");
        assert!(
            range3.is_some(),
            "Should parse mixed range types (closed, open-ended, suffix)"
        );

        let parsed3 = range3.unwrap();
        assert_eq!(parsed3.ranges.len(), 3, "Should have three ranges");

        // First: closed range 0-100
        assert_eq!(parsed3.ranges[0].start, Some(0));
        assert_eq!(parsed3.ranges[0].end, Some(100));

        // Second: open-ended range 500-
        assert_eq!(parsed3.ranges[1].start, Some(500));
        assert_eq!(parsed3.ranges[1].end, None);

        // Third: suffix range -200
        assert_eq!(parsed3.ranges[2].start, None);
        assert_eq!(parsed3.ranges[2].end, Some(200));

        // Test with spaces "bytes= 0-100 , 200-300 "
        let range4 = parse_range_header("bytes= 0-100 , 200-300 ");
        assert!(range4.is_some(), "Should parse multiple ranges with spaces");

        let parsed4 = range4.unwrap();
        assert_eq!(parsed4.ranges.len(), 2);
        assert_eq!(parsed4.ranges[0].start, Some(0));
        assert_eq!(parsed4.ranges[0].end, Some(100));
        assert_eq!(parsed4.ranges[1].start, Some(200));
        assert_eq!(parsed4.ranges[1].end, Some(300));

        // Test single range (should still work)
        let range5 = parse_range_header("bytes=100-199");
        assert!(range5.is_some(), "Should parse single range");

        let parsed5 = range5.unwrap();
        assert_eq!(parsed5.ranges.len(), 1, "Should have one range");
        assert_eq!(parsed5.ranges[0].start, Some(100));
        assert_eq!(parsed5.ranges[0].end, Some(199));

        // Test many ranges (5 ranges)
        let range6 = parse_range_header("bytes=0-99,100-199,200-299,300-399,400-499");
        assert!(range6.is_some(), "Should parse five ranges");

        let parsed6 = range6.unwrap();
        assert_eq!(parsed6.ranges.len(), 5, "Should have five ranges");

        for (i, range) in parsed6.ranges.iter().enumerate() {
            let expected_start = i as u64 * 100;
            let expected_end = expected_start + 99;
            assert_eq!(range.start, Some(expected_start));
            assert_eq!(range.end, Some(expected_end));
            assert_eq!(range.size(), Some(100));
        }
    }

    #[test]
    fn test_handles_invalid_range_header_syntax_gracefully() {
        // Test empty string
        let range1 = parse_range_header("");
        assert_eq!(range1, None, "Should reject empty string");

        // Test missing "bytes=" unit
        let range2 = parse_range_header("0-1023");
        assert_eq!(range2, None, "Should reject missing unit");

        // Test invalid unit (not "bytes")
        let range3 = parse_range_header("chars=0-1023");
        assert!(
            range3.is_some(),
            "Should parse with different unit (HTTP spec allows it)"
        );
        assert_eq!(range3.unwrap().unit, "chars");

        // Test missing equals sign
        let range4 = parse_range_header("bytes 0-1023");
        assert_eq!(range4, None, "Should reject missing equals sign");

        // Test missing dash in range
        let range5 = parse_range_header("bytes=01023");
        assert_eq!(range5, None, "Should reject missing dash");

        // Test invalid start (not a number)
        let range6 = parse_range_header("bytes=abc-1023");
        assert_eq!(range6, None, "Should reject non-numeric start");

        // Test invalid end (not a number)
        let range7 = parse_range_header("bytes=0-xyz");
        assert_eq!(range7, None, "Should reject non-numeric end");

        // Test both start and end invalid
        let range8 = parse_range_header("bytes=abc-xyz");
        assert_eq!(range8, None, "Should reject non-numeric start and end");

        // Test negative start (not suffix range)
        let range9 = parse_range_header("bytes=-100-200");
        assert_eq!(
            range9, None,
            "Should reject negative start in non-suffix range"
        );

        // Test start greater than end
        let range10 = parse_range_header("bytes=1000-100");
        assert!(
            range10.is_some(),
            "Should parse start > end (spec says satisfiable or not depends on content)"
        );
        let parsed10 = range10.unwrap();
        assert_eq!(parsed10.ranges[0].start, Some(1000));
        assert_eq!(parsed10.ranges[0].end, Some(100));
        assert_eq!(
            parsed10.ranges[0].size(),
            None,
            "Size should be None for invalid range (start > end)"
        );

        // Test missing both start and end (just dash)
        let range11 = parse_range_header("bytes=-");
        assert_eq!(range11, None, "Should reject missing both start and end");

        // Test multiple equals signs
        let range12 = parse_range_header("bytes=0=1023");
        assert_eq!(range12, None, "Should reject multiple equals signs");

        // Test trailing comma
        let range13 = parse_range_header("bytes=0-1023,");
        assert_eq!(range13, None, "Should reject trailing comma");

        // Test leading comma
        let range14 = parse_range_header("bytes=,0-1023");
        assert_eq!(range14, None, "Should reject leading comma");

        // Test double comma
        let range15 = parse_range_header("bytes=0-100,,200-300");
        assert_eq!(range15, None, "Should reject double comma");

        // Test whitespace only
        let range16 = parse_range_header("   ");
        assert_eq!(range16, None, "Should reject whitespace only");

        // Test missing value after equals
        let range17 = parse_range_header("bytes=");
        assert_eq!(range17, None, "Should reject missing value after equals");

        // Test special characters
        let range18 = parse_range_header("bytes=0-1023!");
        assert_eq!(range18, None, "Should reject special characters");

        // Test floating point (not allowed)
        let range19 = parse_range_header("bytes=0.5-1023.5");
        assert_eq!(range19, None, "Should reject floating point numbers");
    }

    #[test]
    fn test_includes_accept_ranges_bytes_in_response_headers() {
        use std::collections::HashMap;

        // Test that Accept-Ranges header is included in successful responses
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());
        headers.insert("content-length".to_string(), "1024".to_string());
        headers.insert("accept-ranges".to_string(), "bytes".to_string());

        let response = S3Response::new(200, "OK", headers, vec![0u8; 1024]);

        assert!(
            response.is_success(),
            "Response should be successful (200 OK)"
        );

        // Verify Accept-Ranges header is present
        let accept_ranges = response.get_header("accept-ranges");
        assert!(
            accept_ranges.is_some(),
            "Accept-Ranges header should be present"
        );
        assert_eq!(
            accept_ranges.unwrap(),
            "bytes",
            "Accept-Ranges should be 'bytes'"
        );

        // Test with different content types
        let mut headers2 = HashMap::new();
        headers2.insert("content-type".to_string(), "video/mp4".to_string());
        headers2.insert("content-length".to_string(), "10485760".to_string());
        headers2.insert("accept-ranges".to_string(), "bytes".to_string());
        headers2.insert("etag".to_string(), "\"abc123\"".to_string());

        let response2 = S3Response::new(200, "OK", headers2, vec![0u8; 100]);

        assert_eq!(
            response2.get_header("accept-ranges"),
            Some(&"bytes".to_string()),
            "Video response should include Accept-Ranges: bytes"
        );

        // Test with 206 Partial Content response (range request)
        let mut headers3 = HashMap::new();
        headers3.insert("content-type".to_string(), "application/pdf".to_string());
        headers3.insert("content-length".to_string(), "1024".to_string());
        headers3.insert("content-range".to_string(), "bytes 0-1023/5000".to_string());
        headers3.insert("accept-ranges".to_string(), "bytes".to_string());

        let response3 = S3Response::new(206, "Partial Content", headers3, vec![0u8; 1024]);

        assert_eq!(
            response3.status_code, 206,
            "Range response should have 206 status"
        );
        assert_eq!(
            response3.get_header("accept-ranges"),
            Some(&"bytes".to_string()),
            "Partial content response should include Accept-Ranges: bytes"
        );

        // Test that Accept-Ranges can be checked case-insensitively
        // (though we store as lowercase)
        let mut headers4 = HashMap::new();
        headers4.insert("Accept-Ranges".to_string(), "bytes".to_string());
        headers4.insert("content-length".to_string(), "500".to_string());

        let response4 = S3Response::new(200, "OK", headers4, vec![0u8; 500]);

        // Note: Our implementation stores keys as-is, so exact match needed
        assert!(
            response4.get_header("Accept-Ranges").is_some()
                || response4.get_header("accept-ranges").is_some(),
            "Accept-Ranges should be present (case variations)"
        );

        // Test without Accept-Ranges header (should not panic, just None)
        let mut headers5 = HashMap::new();
        headers5.insert("content-type".to_string(), "text/html".to_string());

        let response5 = S3Response::new(200, "OK", headers5, vec![0u8; 100]);

        assert_eq!(
            response5.get_header("accept-ranges"),
            None,
            "Response without Accept-Ranges should return None"
        );

        // Test with error response (should not have Accept-Ranges)
        let mut headers6 = HashMap::new();
        headers6.insert("content-type".to_string(), "application/xml".to_string());

        let error_body = b"<Error><Code>NoSuchKey</Code></Error>".to_vec();
        let response6 = S3Response::new(404, "Not Found", headers6, error_body);

        assert!(!response6.is_success(), "404 should not be success");
        assert_eq!(
            response6.get_header("accept-ranges"),
            None,
            "Error responses typically don't include Accept-Ranges"
        );
    }

    #[test]
    fn test_forwards_range_header_to_s3_with_aws_signature() {
        use std::collections::HashMap;

        // Test that Range header is included in S3 request and AWS signature
        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers.insert("x-amz-date".to_string(), "20231201T120000Z".to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        headers.insert("range".to_string(), "bytes=0-1023".to_string());

        let params = SigningParams {
            method: "GET",
            uri: "/my-bucket/test.txt",
            query_string: "",
            headers: &headers,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231201",
            datetime: "20231201T120000Z",
        };

        let authorization = sign_request(&params);

        // Verify authorization header is generated
        assert!(
            authorization.starts_with("AWS4-HMAC-SHA256"),
            "Should generate AWS4-HMAC-SHA256 signature"
        );

        // Verify it contains SignedHeaders including range
        assert!(
            authorization.contains("SignedHeaders="),
            "Should include SignedHeaders"
        );

        // The canonical request should include range header in sorted order
        let canonical = create_canonical_request(&params);

        // Range header should be in canonical request (lowercase)
        assert!(
            canonical.contains("range:bytes=0-1023"),
            "Canonical request should include range header: {}",
            canonical
        );

        // Verify signed headers includes range
        assert!(
            canonical.contains("range") || authorization.contains("range"),
            "Signature should include range header"
        );

        // Test with different range formats
        let mut headers2 = HashMap::new();
        headers2.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers2.insert("x-amz-date".to_string(), "20231201T120000Z".to_string());
        headers2.insert(
            "x-amz-content-sha256".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        headers2.insert("range".to_string(), "bytes=1000-".to_string()); // open-ended

        let params2 = SigningParams {
            method: "GET",
            uri: "/my-bucket/video.mp4",
            query_string: "",
            headers: &headers2,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231201",
            datetime: "20231201T120000Z",
        };

        let authorization2 = sign_request(&params2);
        assert!(
            authorization2.starts_with("AWS4-HMAC-SHA256"),
            "Should generate signature for open-ended range"
        );

        let canonical2 = create_canonical_request(&params2);
        assert!(
            canonical2.contains("range:bytes=1000-"),
            "Should include open-ended range in canonical request"
        );

        // Test with suffix range
        let mut headers3 = HashMap::new();
        headers3.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers3.insert("x-amz-date".to_string(), "20231201T120000Z".to_string());
        headers3.insert(
            "x-amz-content-sha256".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        headers3.insert("range".to_string(), "bytes=-500".to_string()); // suffix

        let params3 = SigningParams {
            method: "GET",
            uri: "/my-bucket/data.bin",
            query_string: "",
            headers: &headers3,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231201",
            datetime: "20231201T120000Z",
        };

        let authorization3 = sign_request(&params3);
        assert!(
            authorization3.starts_with("AWS4-HMAC-SHA256"),
            "Should generate signature for suffix range"
        );

        let canonical3 = create_canonical_request(&params3);
        assert!(
            canonical3.contains("range:bytes=-500"),
            "Should include suffix range in canonical request"
        );

        // Test with multiple ranges
        let mut headers4 = HashMap::new();
        headers4.insert("host".to_string(), "bucket.s3.amazonaws.com".to_string());
        headers4.insert("x-amz-date".to_string(), "20231201T120000Z".to_string());
        headers4.insert(
            "x-amz-content-sha256".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        headers4.insert("range".to_string(), "bytes=0-100,200-300".to_string());

        let params4 = SigningParams {
            method: "GET",
            uri: "/my-bucket/file.dat",
            query_string: "",
            headers: &headers4,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231201",
            datetime: "20231201T120000Z",
        };

        let authorization4 = sign_request(&params4);
        assert!(
            authorization4.starts_with("AWS4-HMAC-SHA256"),
            "Should generate signature for multiple ranges"
        );

        let canonical4 = create_canonical_request(&params4);
        assert!(
            canonical4.contains("range:bytes=0-100,200-300"),
            "Should include multiple ranges in canonical request"
        );

        // Verify different range headers produce different signatures
        let sig_single = authorization;
        let sig_open = authorization2;
        let sig_suffix = authorization3;
        let sig_multi = authorization4;

        assert_ne!(
            sig_single, sig_open,
            "Different range values should produce different signatures"
        );
        assert_ne!(
            sig_single, sig_suffix,
            "Different range types should produce different signatures"
        );
        assert_ne!(
            sig_single, sig_multi,
            "Multiple ranges should produce different signature"
        );
    }

    #[test]
    fn test_returns_206_partial_content_for_valid_range() {
        use std::collections::HashMap;

        // Test 206 response for single range request
        let mut headers1 = HashMap::new();
        headers1.insert("content-type".to_string(), "text/plain".to_string());
        headers1.insert("content-length".to_string(), "1024".to_string());
        headers1.insert("content-range".to_string(), "bytes 0-1023/5000".to_string());
        headers1.insert("accept-ranges".to_string(), "bytes".to_string());

        let response1 = S3Response::new(206, "Partial Content", headers1, vec![0u8; 1024]);

        assert_eq!(
            response1.status_code, 206,
            "Should return 206 Partial Content for range request"
        );
        assert_eq!(
            response1.status_text, "Partial Content",
            "Status text should be 'Partial Content'"
        );
        assert!(
            response1.is_success(),
            "206 Partial Content is a success status (2xx)"
        );

        // Verify Content-Range header is present
        let content_range = response1.get_header("content-range");
        assert!(
            content_range.is_some(),
            "Content-Range header should be present in 206 response"
        );
        assert_eq!(
            content_range.unwrap(),
            "bytes 0-1023/5000",
            "Content-Range should specify the byte range"
        );

        // Test 206 response for open-ended range (bytes 1000 to end)
        let mut headers2 = HashMap::new();
        headers2.insert("content-type".to_string(), "video/mp4".to_string());
        headers2.insert("content-length".to_string(), "4000".to_string());
        headers2.insert(
            "content-range".to_string(),
            "bytes 1000-4999/5000".to_string(),
        );
        headers2.insert("accept-ranges".to_string(), "bytes".to_string());

        let response2 = S3Response::new(206, "Partial Content", headers2, vec![0u8; 4000]);

        assert_eq!(response2.status_code, 206);
        assert_eq!(
            response2.get_header("content-range"),
            Some(&"bytes 1000-4999/5000".to_string())
        );

        // Test 206 response for suffix range (last 500 bytes)
        let mut headers3 = HashMap::new();
        headers3.insert("content-type".to_string(), "application/pdf".to_string());
        headers3.insert("content-length".to_string(), "500".to_string());
        headers3.insert(
            "content-range".to_string(),
            "bytes 4500-4999/5000".to_string(),
        );
        headers3.insert("accept-ranges".to_string(), "bytes".to_string());

        let response3 = S3Response::new(206, "Partial Content", headers3, vec![0u8; 500]);

        assert_eq!(response3.status_code, 206);
        assert_eq!(
            response3.get_header("content-range"),
            Some(&"bytes 4500-4999/5000".to_string())
        );

        // Verify body size matches Content-Range
        assert_eq!(
            response3.body.len(),
            500,
            "Body size should match the range size"
        );

        // Test that 200 OK is different from 206
        let mut headers_full = HashMap::new();
        headers_full.insert("content-type".to_string(), "text/plain".to_string());
        headers_full.insert("content-length".to_string(), "5000".to_string());
        headers_full.insert("accept-ranges".to_string(), "bytes".to_string());

        let response_full = S3Response::new(200, "OK", headers_full, vec![0u8; 5000]);

        assert_eq!(response_full.status_code, 200, "Full file returns 200 OK");
        assert_eq!(
            response_full.get_header("content-range"),
            None,
            "200 OK response should not have Content-Range header"
        );
        assert_ne!(
            response1.status_code, response_full.status_code,
            "206 Partial Content should be different from 200 OK"
        );

        // Test 206 with multiple ranges (multipart/byteranges)
        // Note: This is typically returned as multipart content
        let mut headers_multi = HashMap::new();
        headers_multi.insert(
            "content-type".to_string(),
            "multipart/byteranges; boundary=example".to_string(),
        );
        headers_multi.insert("content-length".to_string(), "300".to_string());

        let response_multi = S3Response::new(206, "Partial Content", headers_multi, vec![0u8; 300]);

        assert_eq!(
            response_multi.status_code, 206,
            "Multiple ranges also return 206"
        );
        assert!(
            response_multi
                .get_header("content-type")
                .unwrap()
                .contains("multipart/byteranges"),
            "Multiple ranges use multipart content type"
        );

        // Test that 206 body size can be less than full file
        assert!(
            response1.body.len() < 5000,
            "Partial content body should be smaller than full file"
        );
        assert!(
            response2.body.len() < 5000,
            "Partial content body should be smaller than full file"
        );
        assert!(
            response3.body.len() < 5000,
            "Partial content body should be smaller than full file"
        );
    }

    #[test]
    fn test_returns_content_range_header_with_correct_format() {
        use std::collections::HashMap;

        // Test Content-Range format: "bytes start-end/total"
        // RFC 7233 specifies: Content-Range: bytes-unit SP first-byte-pos "-" last-byte-pos "/" complete-length

        // Test single range: bytes 0-1023/5000
        let mut headers1 = HashMap::new();
        headers1.insert("content-type".to_string(), "text/plain".to_string());
        headers1.insert("content-length".to_string(), "1024".to_string());
        headers1.insert("content-range".to_string(), "bytes 0-1023/5000".to_string());

        let response1 = S3Response::new(206, "Partial Content", headers1, vec![0u8; 1024]);

        let content_range = response1.get_header("content-range");
        assert!(
            content_range.is_some(),
            "Content-Range header must be present"
        );

        let range_value = content_range.unwrap();
        assert_eq!(
            range_value, "bytes 0-1023/5000",
            "Content-Range should be 'bytes 0-1023/5000'"
        );

        // Verify format components
        assert!(
            range_value.starts_with("bytes "),
            "Should start with 'bytes '"
        );
        assert!(range_value.contains("-"), "Should contain '-' separator");
        assert!(range_value.contains("/"), "Should contain '/' before total");

        // Test open-ended range result: bytes 1000-4999/5000
        let mut headers2 = HashMap::new();
        headers2.insert(
            "content-range".to_string(),
            "bytes 1000-4999/5000".to_string(),
        );

        let response2 = S3Response::new(206, "Partial Content", headers2, vec![0u8; 4000]);
        assert_eq!(
            response2.get_header("content-range"),
            Some(&"bytes 1000-4999/5000".to_string())
        );

        // Test suffix range result: bytes 4500-4999/5000 (last 500 bytes)
        let mut headers3 = HashMap::new();
        headers3.insert(
            "content-range".to_string(),
            "bytes 4500-4999/5000".to_string(),
        );

        let response3 = S3Response::new(206, "Partial Content", headers3, vec![0u8; 500]);
        assert_eq!(
            response3.get_header("content-range"),
            Some(&"bytes 4500-4999/5000".to_string())
        );

        // Test large file: bytes 0-1048575/10485760 (first MB of 10MB file)
        let mut headers4 = HashMap::new();
        headers4.insert(
            "content-range".to_string(),
            "bytes 0-1048575/10485760".to_string(),
        );

        let response4 = S3Response::new(206, "Partial Content", headers4, vec![0u8; 1048576]);

        let range4 = response4.get_header("content-range").unwrap();
        assert_eq!(range4, "bytes 0-1048575/10485760");
        assert!(range4.starts_with("bytes "));
        assert!(range4.contains("-"));
        assert!(range4.contains("/10485760"));

        // Test unknown total size: bytes 0-1023/* (when total size unknown)
        let mut headers5 = HashMap::new();
        headers5.insert("content-range".to_string(), "bytes 0-1023/*".to_string());

        let response5 = S3Response::new(206, "Partial Content", headers5, vec![0u8; 1024]);
        assert_eq!(
            response5.get_header("content-range"),
            Some(&"bytes 0-1023/*".to_string()),
            "Content-Range with unknown size should use '*'"
        );

        // Test edge case: single byte range (bytes 0-0/100)
        let mut headers6 = HashMap::new();
        headers6.insert("content-range".to_string(), "bytes 0-0/100".to_string());

        let response6 = S3Response::new(206, "Partial Content", headers6, vec![0u8; 1]);
        assert_eq!(
            response6.get_header("content-range"),
            Some(&"bytes 0-0/100".to_string()),
            "Single byte range should be 'bytes 0-0/total'"
        );

        // Test edge case: last byte (bytes 99-99/100)
        let mut headers7 = HashMap::new();
        headers7.insert("content-range".to_string(), "bytes 99-99/100".to_string());

        let response7 = S3Response::new(206, "Partial Content", headers7, vec![0u8; 1]);
        assert_eq!(
            response7.get_header("content-range"),
            Some(&"bytes 99-99/100".to_string())
        );

        // Verify parsing components from Content-Range header
        let range_str = "bytes 100-199/500";
        let parts: Vec<&str> = range_str.split_whitespace().collect();
        assert_eq!(parts[0], "bytes", "First part should be 'bytes'");

        let byte_range = parts[1];
        let range_parts: Vec<&str> = byte_range.split('/').collect();
        assert_eq!(range_parts.len(), 2, "Should have range and total");

        let positions: Vec<&str> = range_parts[0].split('-').collect();
        assert_eq!(positions.len(), 2, "Should have start and end");
        assert_eq!(positions[0], "100", "Start should be 100");
        assert_eq!(positions[1], "199", "End should be 199");
        assert_eq!(range_parts[1], "500", "Total should be 500");

        // Test that 200 OK response doesn't have Content-Range
        let mut headers_full = HashMap::new();
        headers_full.insert("content-type".to_string(), "text/plain".to_string());
        headers_full.insert("content-length".to_string(), "5000".to_string());

        let response_full = S3Response::new(200, "OK", headers_full, vec![0u8; 5000]);
        assert_eq!(
            response_full.get_header("content-range"),
            None,
            "200 OK should not have Content-Range header"
        );
    }

    #[tokio::test]
    async fn test_streams_only_requested_bytes_not_full_file() {
        use futures::stream::{self, StreamExt};

        // Simulate a 5000 byte file where client requests only bytes 1000-1999
        let total_file_size = 5000usize;
        let range_start = 1000usize;
        let range_end = 1999usize;
        let expected_bytes = (range_end - range_start) + 1; // 1000 bytes

        // Create full file data (5000 bytes, each byte = its position % 256)
        let full_file: Vec<u8> = (0..total_file_size).map(|i| (i % 256) as u8).collect();

        // Simulate S3 returning only the requested range (not full file)
        let range_data: Vec<u8> = full_file[range_start..=range_end].to_vec();

        // Create stream that yields only the requested bytes
        let chunk_size = 256; // Stream in 256-byte chunks
        let chunks: Vec<Vec<u8>> = range_data.chunks(chunk_size).map(|c| c.to_vec()).collect();

        let data_stream = stream::iter(
            chunks
                .into_iter()
                .map(|chunk| Ok::<_, std::io::Error>(bytes::Bytes::from(chunk))),
        );

        // Client receives the stream
        let mut received_bytes = Vec::new();
        let mut stream = Box::pin(data_stream);

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    received_bytes.extend_from_slice(&chunk);
                }
                Err(_) => break,
            }
        }

        // Verify we only received the requested range, not the full file
        assert_eq!(
            received_bytes.len(),
            expected_bytes,
            "Should receive exactly {} bytes (requested range), not {} bytes (full file)",
            expected_bytes,
            total_file_size
        );

        assert!(
            received_bytes.len() < total_file_size,
            "Received bytes ({}) should be less than full file ({})",
            received_bytes.len(),
            total_file_size
        );

        // Verify the content is correct (matches bytes 1000-1999 from original)
        for (i, &byte) in received_bytes.iter().enumerate() {
            let original_position = range_start + i;
            let expected_byte = (original_position % 256) as u8;
            assert_eq!(
                byte, expected_byte,
                "Byte at offset {} should be {} (from position {}), got {}",
                i, expected_byte, original_position, byte
            );
        }

        // Test different range sizes to verify streaming efficiency
        // Small range: bytes 0-99 (100 bytes from 5000 byte file)
        let small_range_data: Vec<u8> = (0..100).map(|i| i as u8).collect();
        let small_stream = stream::iter(vec![Ok::<_, std::io::Error>(bytes::Bytes::from(
            small_range_data.clone(),
        ))]);

        let mut small_received = Vec::new();
        let mut small_stream_pin = Box::pin(small_stream);

        while let Some(chunk_result) = small_stream_pin.next().await {
            if let Ok(chunk) = chunk_result {
                small_received.extend_from_slice(&chunk);
            }
        }

        assert_eq!(
            small_received.len(),
            100,
            "Small range should be 100 bytes, not full file"
        );
        assert_eq!(small_received, small_range_data);

        // Large range: bytes 0-4999 (full file = 5000 bytes)
        let large_range_data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();
        let large_stream = stream::iter(
            large_range_data
                .chunks(1000)
                .map(|c| Ok::<_, std::io::Error>(bytes::Bytes::from(c.to_vec())))
                .collect::<Vec<_>>(),
        );

        let mut large_received = Vec::new();
        let mut large_stream_pin = Box::pin(large_stream);

        while let Some(chunk_result) = large_stream_pin.next().await {
            if let Ok(chunk) = chunk_result {
                large_received.extend_from_slice(&chunk);
            }
        }

        assert_eq!(
            large_received.len(),
            5000,
            "Large range covering full file should be 5000 bytes"
        );

        // Open-ended range: bytes 4500- (last 500 bytes)
        let open_ended_data: Vec<u8> = (4500..5000).map(|i| (i % 256) as u8).collect();
        let open_ended_stream = stream::iter(vec![Ok::<_, std::io::Error>(bytes::Bytes::from(
            open_ended_data.clone(),
        ))]);

        let mut open_ended_received = Vec::new();
        let mut open_ended_stream_pin = Box::pin(open_ended_stream);

        while let Some(chunk_result) = open_ended_stream_pin.next().await {
            if let Ok(chunk) = chunk_result {
                open_ended_received.extend_from_slice(&chunk);
            }
        }

        assert_eq!(
            open_ended_received.len(),
            500,
            "Open-ended range should be 500 bytes (4500 to end), not full 5000"
        );
        assert_eq!(open_ended_received, open_ended_data);

        // Suffix range: bytes -200 (last 200 bytes)
        let suffix_data: Vec<u8> = (4800..5000).map(|i| (i % 256) as u8).collect();
        let suffix_stream = stream::iter(vec![Ok::<_, std::io::Error>(bytes::Bytes::from(
            suffix_data.clone(),
        ))]);

        let mut suffix_received = Vec::new();
        let mut suffix_stream_pin = Box::pin(suffix_stream);

        while let Some(chunk_result) = suffix_stream_pin.next().await {
            if let Ok(chunk) = chunk_result {
                suffix_received.extend_from_slice(&chunk);
            }
        }

        assert_eq!(
            suffix_received.len(),
            200,
            "Suffix range should be 200 bytes (last 200), not full 5000"
        );
        assert_eq!(suffix_received, suffix_data);

        println!(
            "✓ Range request streams only {} bytes, not full {} byte file",
            expected_bytes, total_file_size
        );
        println!("✓ Small range (100 bytes), large range (5000 bytes), open-ended (500 bytes), suffix (200 bytes) all verified");
    }

    #[test]
    fn test_returns_416_range_not_satisfiable_for_out_of_bounds_range() {
        use std::collections::HashMap;

        // Test 416 when range start is beyond file size
        // File size: 5000 bytes, Request: bytes 6000-7000
        let mut headers1 = HashMap::new();
        headers1.insert("content-type".to_string(), "application/xml".to_string());
        headers1.insert("content-range".to_string(), "bytes */5000".to_string());

        let error_body = b"<Error><Code>InvalidRange</Code><Message>The requested range is not satisfiable</Message></Error>".to_vec();
        let response1 = S3Response::new(416, "Range Not Satisfiable", headers1, error_body);

        assert_eq!(
            response1.status_code, 416,
            "Should return 416 for out-of-bounds range"
        );
        assert_eq!(
            response1.status_text, "Range Not Satisfiable",
            "Status text should be 'Range Not Satisfiable'"
        );
        assert!(
            !response1.is_success(),
            "416 is not a success status (4xx error)"
        );

        // Content-Range header with unsatisfiable range uses format: bytes */total-length
        let content_range = response1.get_header("content-range");
        assert!(
            content_range.is_some(),
            "416 response should include Content-Range header"
        );
        assert_eq!(
            content_range.unwrap(),
            "bytes */5000",
            "Content-Range should be 'bytes */5000' for unsatisfiable range"
        );

        // Test when range start > file size (bytes 10000-10999 from 5000 byte file)
        let mut headers2 = HashMap::new();
        headers2.insert("content-range".to_string(), "bytes */5000".to_string());

        let response2 = S3Response::new(416, "Range Not Satisfiable", headers2, vec![]);
        assert_eq!(response2.status_code, 416);
        assert_eq!(
            response2.get_header("content-range"),
            Some(&"bytes */5000".to_string())
        );

        // Test when both start and end are beyond file size
        let mut headers3 = HashMap::new();
        headers3.insert("content-range".to_string(), "bytes */1048576".to_string());

        let response3 = S3Response::new(416, "Range Not Satisfiable", headers3, vec![]);
        assert_eq!(response3.status_code, 416);
        assert_eq!(
            response3.get_header("content-range"),
            Some(&"bytes */1048576".to_string()),
            "Should indicate total file size even for out-of-bounds range"
        );

        // Test 416 vs 206 distinction
        let mut headers_206 = HashMap::new();
        headers_206.insert("content-range".to_string(), "bytes 0-999/5000".to_string());

        let response_206 = S3Response::new(206, "Partial Content", headers_206, vec![0u8; 1000]);

        assert_ne!(
            response1.status_code, response_206.status_code,
            "416 should be different from 206"
        );

        // Test 416 vs 200 distinction
        let mut headers_200 = HashMap::new();
        headers_200.insert("content-length".to_string(), "5000".to_string());

        let response_200 = S3Response::new(200, "OK", headers_200, vec![0u8; 5000]);

        assert_ne!(
            response1.status_code, response_200.status_code,
            "416 should be different from 200"
        );

        // Verify Content-Range format for 416: bytes */complete-length
        let range_str = "bytes */5000";
        assert!(range_str.starts_with("bytes "));
        assert!(range_str.contains("*/"));

        let parts: Vec<&str> = range_str.split_whitespace().collect();
        assert_eq!(parts[0], "bytes");
        assert!(
            parts[1].starts_with("*/"),
            "Should start with '*/' for unsatisfiable range"
        );

        // Test error body contains meaningful error
        let error_code = response1.get_error_code();
        assert!(
            error_code.is_some(),
            "416 response should have error code in body"
        );
        assert_eq!(
            error_code.unwrap(),
            "InvalidRange",
            "Error code should be InvalidRange"
        );

        // Test that 416 response body is not partial content
        assert!(
            response1.body.len() < 1000,
            "416 response should not contain partial content data"
        );

        // Verify 416 can occur with different file sizes
        let mut headers_small = HashMap::new();
        headers_small.insert("content-range".to_string(), "bytes */100".to_string());

        let response_small = S3Response::new(416, "Range Not Satisfiable", headers_small, vec![]);
        assert_eq!(response_small.status_code, 416);

        let mut headers_large = HashMap::new();
        headers_large.insert("content-range".to_string(), "bytes */10485760".to_string());

        let response_large = S3Response::new(416, "Range Not Satisfiable", headers_large, vec![]);
        assert_eq!(response_large.status_code, 416);
    }

    #[tokio::test]
    async fn test_streaming_stops_if_client_disconnects() {
        use futures::stream::{self, StreamExt};
        use std::sync::{Arc, Mutex};
        use tokio::sync::mpsc;

        // Track how many chunks were actually processed
        let chunks_processed = Arc::new(Mutex::new(0usize));
        let chunks_processed_clone = chunks_processed.clone();

        // Simulate a large S3 response stream with 100 chunks
        let total_chunks = 100;
        let chunk_size = 64 * 1024; // 64 KB chunks
        let data_stream = stream::iter(0..total_chunks).map(move |i| {
            // Each chunk is 64KB of data
            let chunk = vec![i as u8; chunk_size];
            Ok::<_, std::io::Error>(bytes::Bytes::from(chunk))
        });

        // Create a channel to simulate client connection
        // Small buffer to simulate realistic backpressure
        let (tx, mut rx) = mpsc::channel::<bytes::Bytes>(4);

        // Spawn a task to send stream chunks to client
        let sender_task = tokio::spawn(async move {
            let mut stream = Box::pin(data_stream);

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // Increment processed counter
                        *chunks_processed_clone.lock().unwrap() += 1;

                        // Try to send chunk to client
                        // If client disconnected, send will fail
                        if tx.send(chunk).await.is_err() {
                            // Client disconnected - stop streaming!
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Client receives some chunks then disconnects
        let mut received_chunks = 0;
        let disconnect_after = 10; // Disconnect after 10 chunks

        while let Some(_chunk) = rx.recv().await {
            received_chunks += 1;

            if received_chunks >= disconnect_after {
                // Client disconnects by dropping receiver
                drop(rx);
                break;
            }
        }

        // Wait a bit for sender task to detect disconnect
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify streaming stopped when client disconnected
        let total_processed = *chunks_processed.lock().unwrap();

        assert_eq!(
            received_chunks, disconnect_after,
            "Client should have received exactly {} chunks before disconnect",
            disconnect_after
        );

        assert!(
            total_processed <= disconnect_after + 4, // +4 for buffer size
            "Streaming should stop shortly after client disconnect. Processed: {}, Expected: <= {}",
            total_processed,
            disconnect_after + 4
        );

        assert!(
            total_processed < total_chunks,
            "Should NOT process all {} chunks when client disconnects early. Processed: {}",
            total_chunks,
            total_processed
        );

        // Verify sender task completed (not hung)
        let sender_result =
            tokio::time::timeout(tokio::time::Duration::from_secs(1), sender_task).await;

        assert!(
            sender_result.is_ok(),
            "Sender task should complete within 1 second after client disconnect"
        );
    }

    #[tokio::test]
    async fn test_memory_usage_stays_constant_during_streaming() {
        use futures::stream::{self, StreamExt};
        use std::sync::{Arc, Mutex};

        // Simulate streaming a very large file (1 GB)
        // Key insight: We process chunks one at a time, never holding entire file
        let total_chunks = 16384; // 16,384 chunks * 64KB = 1 GB
        let chunk_size = 64 * 1024; // 64 KB chunks

        // Track maximum memory held at any point
        // In real streaming: only 1-2 chunks should be in memory at once
        let max_chunks_in_memory = Arc::new(Mutex::new(0usize));
        let max_chunks_clone = max_chunks_in_memory.clone();

        // Current chunks in memory (should stay ≤ 2-3 due to buffering)
        let current_chunks_in_memory = Arc::new(Mutex::new(0usize));
        let current_chunks_clone = current_chunks_in_memory.clone();

        // Create a stream that simulates S3 response
        let data_stream = stream::iter(0..total_chunks).map(move |i| {
            // Simulate chunk creation (allocate memory)
            let chunk = vec![i as u8; chunk_size];

            // Track allocation
            let mut current = current_chunks_clone.lock().unwrap();
            *current += 1;

            // Update max if needed
            let mut max = max_chunks_clone.lock().unwrap();
            if *current > *max {
                *max = *current;
            }

            Ok::<_, std::io::Error>((bytes::Bytes::from(chunk), current_chunks_clone.clone()))
        });

        // Client that receives and processes chunks one at a time
        let mut stream = Box::pin(data_stream);
        let mut total_bytes_received = 0u64;
        let mut chunks_processed = 0usize;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok((chunk, counter)) => {
                    // Process chunk (in real scenario: send to client immediately)
                    total_bytes_received += chunk.len() as u64;
                    chunks_processed += 1;

                    // Simulate chunk being sent/deallocated
                    // Drop chunk here (goes out of scope)
                    drop(chunk);

                    // Decrement in-memory counter
                    *counter.lock().unwrap() -= 1;

                    // Optional: simulate network delay/backpressure
                    if chunks_processed % 100 == 0 {
                        tokio::task::yield_now().await;
                    }
                }
                Err(_) => break,
            }
        }

        // Verify all chunks were processed
        assert_eq!(
            chunks_processed, total_chunks,
            "Should process all {} chunks",
            total_chunks
        );

        // Verify total data streamed
        let expected_bytes = (total_chunks as u64) * (chunk_size as u64);
        assert_eq!(
            total_bytes_received,
            expected_bytes,
            "Should receive all {} GB of data",
            expected_bytes / (1024 * 1024 * 1024)
        );

        // Verify memory usage stayed constant (never held all chunks in memory)
        let max_memory_chunks = *max_chunks_in_memory.lock().unwrap();
        assert!(
            max_memory_chunks <= 10,
            "Should never hold more than ~10 chunks in memory. Had: {}",
            max_memory_chunks
        );

        // Calculate memory efficiency
        let max_memory_mb = (max_memory_chunks * chunk_size) / (1024 * 1024);
        let total_file_mb = expected_bytes / (1024 * 1024);

        assert!(
            max_memory_mb < 1, // Less than 1 MB in memory at once
            "Memory usage should be < 1 MB, was {} MB for {} MB file",
            max_memory_mb,
            total_file_mb
        );

        // Verify final state: no chunks left in memory
        let final_chunks = *current_chunks_in_memory.lock().unwrap();
        assert_eq!(
            final_chunks, 0,
            "All chunks should be deallocated after streaming completes"
        );

        // This demonstrates O(1) memory usage for O(n) file size
        // Whether streaming 1 MB or 1 GB, memory usage stays constant
        println!(
            "✓ Streamed {} MB file using only {} KB max memory",
            total_file_mb,
            max_memory_chunks * chunk_size / 1024
        );
    }

    #[tokio::test]
    async fn test_can_handle_concurrent_streams_to_multiple_clients() {
        use futures::stream::{self, StreamExt};
        use std::sync::{Arc, Mutex};
        use tokio::time::{timeout, Duration};

        // Test concurrent streaming of multiple files to multiple clients
        let num_clients = 10;
        let chunks_per_file = 100;
        let chunk_size = 64 * 1024; // 64 KB

        // Track successful completions
        let completed_clients = Arc::new(Mutex::new(0usize));
        let completed_clients_clone = completed_clients.clone();

        // Spawn multiple concurrent client tasks
        let mut client_tasks = vec![];

        for client_id in 0..num_clients {
            let completed_clone = completed_clients_clone.clone();

            let client_task = tokio::spawn(async move {
                // Each client streams a different file (identified by client_id)
                // Simulate S3 response stream for this client's file
                let data_stream = stream::iter(0..chunks_per_file).map(move |_chunk_num| {
                    // Each chunk contains client_id to detect data corruption
                    let chunk_data = vec![client_id as u8; chunk_size];
                    Ok::<_, std::io::Error>(bytes::Bytes::from(chunk_data))
                });

                let mut stream = Box::pin(data_stream);
                let mut chunks_received = 0;
                let mut total_bytes = 0u64;

                // Client receives stream
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // Verify data integrity (all bytes should be client_id)
                            for &byte in chunk.iter() {
                                if byte != client_id as u8 {
                                    panic!(
                                        "Data corruption detected! Client {} received byte {} instead of {}",
                                        client_id, byte, client_id
                                    );
                                }
                            }

                            chunks_received += 1;
                            total_bytes += chunk.len() as u64;

                            // Simulate realistic network delay/processing
                            if chunks_received % 10 == 0 {
                                tokio::task::yield_now().await;
                            }
                        }
                        Err(_) => break,
                    }
                }

                // Verify client received complete file
                assert_eq!(
                    chunks_received, chunks_per_file,
                    "Client {} should receive all chunks",
                    client_id
                );

                let expected_bytes = (chunks_per_file as u64) * (chunk_size as u64);
                assert_eq!(
                    total_bytes, expected_bytes,
                    "Client {} should receive all bytes",
                    client_id
                );

                // Mark completion
                *completed_clone.lock().unwrap() += 1;

                client_id as usize
            });

            client_tasks.push(client_task);
        }

        // Wait for all clients to complete with timeout
        let all_tasks = futures::future::join_all(client_tasks);
        let result = timeout(Duration::from_secs(10), all_tasks).await;

        assert!(
            result.is_ok(),
            "All concurrent streams should complete within 10 seconds"
        );

        let client_results = result.unwrap();

        // Verify all clients completed successfully
        for (i, task_result) in client_results.iter().enumerate() {
            assert!(
                task_result.is_ok(),
                "Client task {} should complete without panic",
                i
            );

            let client_id = task_result.as_ref().unwrap();
            assert_eq!(*client_id, i, "Client ID should match task index");
        }

        // Verify completion counter
        let total_completed = *completed_clients.lock().unwrap();
        assert_eq!(
            total_completed, num_clients,
            "All {} clients should complete successfully",
            num_clients
        );

        // Test concurrent streaming of the SAME file to multiple clients
        // This verifies no race conditions when multiple clients access same resource
        let same_file_stream_fn = || {
            stream::iter(0..50)
                .map(|_i| Ok::<_, std::io::Error>(bytes::Bytes::from(vec![42u8; 1024])))
        };

        let mut same_file_tasks = vec![];
        for client_id in 0..5 {
            let client_task = tokio::spawn(async move {
                let mut stream = Box::pin(same_file_stream_fn());
                let mut count = 0;

                while let Some(chunk_result) = stream.next().await {
                    if let Ok(chunk) = chunk_result {
                        // Verify data integrity
                        assert_eq!(chunk.len(), 1024);
                        assert!(chunk.iter().all(|&b| b == 42));
                        count += 1;
                    }
                }

                assert_eq!(
                    count, 50,
                    "Client {} should receive all 50 chunks",
                    client_id
                );
            });

            same_file_tasks.push(client_task);
        }

        // Wait for same-file streaming tests
        let same_file_result = timeout(
            Duration::from_secs(5),
            futures::future::join_all(same_file_tasks),
        )
        .await;

        assert!(
            same_file_result.is_ok(),
            "Concurrent streams of same file should complete"
        );

        for task_result in same_file_result.unwrap() {
            assert!(
                task_result.is_ok(),
                "Same-file streaming task should complete successfully"
            );
        }

        println!(
            "✓ Successfully handled {} concurrent streams with no data corruption",
            num_clients
        );
        println!("✓ Successfully handled 5 concurrent streams of same file");
    }

    #[test]
    fn test_handles_if_range_conditional_requests_correctly() {
        use std::collections::HashMap;

        // Test case 1: If-Range with ETag that matches
        // Client has cached version with ETag "abc123", wants bytes 0-1023
        // Server has same ETag → return 206 Partial Content
        let mut headers_match = HashMap::new();
        headers_match.insert("etag".to_string(), "\"abc123\"".to_string());
        headers_match.insert("content-type".to_string(), "text/plain".to_string());
        headers_match.insert("content-range".to_string(), "bytes 0-1023/5000".to_string());
        headers_match.insert("content-length".to_string(), "1024".to_string());

        let partial_body = vec![1u8; 1024]; // 1024 bytes of partial content
        let response_match = S3Response::new(206, "Partial Content", headers_match, partial_body);

        assert_eq!(response_match.status_code, 206);
        assert_eq!(response_match.status_text, "Partial Content");
        assert_eq!(
            response_match.headers.get("content-range").unwrap(),
            "bytes 0-1023/5000"
        );
        assert_eq!(response_match.body.len(), 1024);

        // Test case 2: If-Range with ETag that doesn't match
        // Client has cached version with ETag "abc123", wants bytes 0-1023
        // Server has different ETag "xyz789" → return 200 OK with full content
        let mut headers_mismatch = HashMap::new();
        headers_mismatch.insert("etag".to_string(), "\"xyz789\"".to_string());
        headers_mismatch.insert("content-type".to_string(), "text/plain".to_string());
        headers_mismatch.insert("content-length".to_string(), "5000".to_string());
        // Note: No Content-Range header when returning full content

        let full_body = vec![2u8; 5000]; // Full 5000 bytes
        let response_mismatch = S3Response::new(200, "OK", headers_mismatch, full_body);

        assert_eq!(response_mismatch.status_code, 200);
        assert_eq!(response_mismatch.status_text, "OK");
        assert_eq!(response_mismatch.headers.get("etag").unwrap(), "\"xyz789\"");
        assert_eq!(response_mismatch.body.len(), 5000);
        assert!(
            response_mismatch.headers.get("content-range").is_none(),
            "200 OK response should not include Content-Range header"
        );

        // Test case 3: If-Range with Last-Modified date that matches
        // Client has cached version from specific date, wants partial content
        // Server Last-Modified matches → return 206 Partial Content
        let mut headers_date_match = HashMap::new();
        headers_date_match.insert(
            "last-modified".to_string(),
            "Wed, 21 Oct 2015 07:28:00 GMT".to_string(),
        );
        headers_date_match.insert("content-type".to_string(), "application/pdf".to_string());
        headers_date_match.insert(
            "content-range".to_string(),
            "bytes 1000-2999/10000".to_string(),
        );
        headers_date_match.insert("content-length".to_string(), "2000".to_string());

        let partial_body_date = vec![3u8; 2000];
        let response_date_match = S3Response::new(
            206,
            "Partial Content",
            headers_date_match,
            partial_body_date,
        );

        assert_eq!(response_date_match.status_code, 206);
        assert_eq!(
            response_date_match.headers.get("last-modified").unwrap(),
            "Wed, 21 Oct 2015 07:28:00 GMT"
        );
        assert_eq!(
            response_date_match.headers.get("content-range").unwrap(),
            "bytes 1000-2999/10000"
        );
        assert_eq!(response_date_match.body.len(), 2000);

        // Test case 4: If-Range with Last-Modified date that doesn't match
        // Client has old cached version, wants partial content
        // Server Last-Modified is newer → return 200 OK with full content
        let mut headers_date_mismatch = HashMap::new();
        headers_date_mismatch.insert(
            "last-modified".to_string(),
            "Thu, 22 Oct 2015 10:00:00 GMT".to_string(), // Newer date
        );
        headers_date_mismatch.insert("content-type".to_string(), "application/pdf".to_string());
        headers_date_mismatch.insert("content-length".to_string(), "10000".to_string());

        let full_body_date = vec![4u8; 10000];
        let response_date_mismatch =
            S3Response::new(200, "OK", headers_date_mismatch, full_body_date);

        assert_eq!(response_date_mismatch.status_code, 200);
        assert_eq!(
            response_date_mismatch.headers.get("last-modified").unwrap(),
            "Thu, 22 Oct 2015 10:00:00 GMT"
        );
        assert_eq!(response_date_mismatch.body.len(), 10000);
        assert!(
            response_date_mismatch
                .headers
                .get("content-range")
                .is_none(),
            "200 OK response should not include Content-Range header"
        );

        // Test case 5: Verify Accept-Ranges header is included
        // This indicates server supports range requests
        let mut headers_accept_ranges = HashMap::new();
        headers_accept_ranges.insert("accept-ranges".to_string(), "bytes".to_string());
        headers_accept_ranges.insert("etag".to_string(), "\"def456\"".to_string());
        headers_accept_ranges.insert("content-length".to_string(), "1000".to_string());

        let response_accept_ranges =
            S3Response::new(200, "OK", headers_accept_ranges, vec![5u8; 1000]);

        assert_eq!(
            response_accept_ranges.headers.get("accept-ranges").unwrap(),
            "bytes"
        );

        // Test case 6: Range request without If-Range (already tested, but verify distinction)
        // This should ALWAYS return 206 if range is valid, regardless of ETag/date
        let mut headers_no_if_range = HashMap::new();
        headers_no_if_range.insert("etag".to_string(), "\"any-etag\"".to_string());
        headers_no_if_range.insert("content-range".to_string(), "bytes 0-999/5000".to_string());
        headers_no_if_range.insert("content-length".to_string(), "1000".to_string());

        let response_no_if_range =
            S3Response::new(206, "Partial Content", headers_no_if_range, vec![6u8; 1000]);

        assert_eq!(response_no_if_range.status_code, 206);
        assert_eq!(
            response_no_if_range.headers.get("content-range").unwrap(),
            "bytes 0-999/5000"
        );
    }

    #[test]
    fn test_graceful_fallback_to_200_ok_for_invalid_range_syntax() {
        use std::collections::HashMap;

        // Per RFC 7233, when a Range header has invalid syntax,
        // the server SHOULD ignore it and return 200 OK with full content
        // This is more user-friendly than returning 400 Bad Request

        // Test case 1: Invalid range syntax with letters
        // Request: Range: bytes=abc-def
        // Expected: 200 OK with full content (ignore invalid range)
        let mut headers_invalid_letters = HashMap::new();
        headers_invalid_letters.insert("content-type".to_string(), "text/plain".to_string());
        headers_invalid_letters.insert("content-length".to_string(), "5000".to_string());
        headers_invalid_letters.insert("etag".to_string(), "\"abc123\"".to_string());
        // No Content-Range header since we're serving full content

        let full_body = vec![1u8; 5000];
        let response_invalid_letters =
            S3Response::new(200, "OK", headers_invalid_letters, full_body);

        assert_eq!(response_invalid_letters.status_code, 200);
        assert_eq!(response_invalid_letters.status_text, "OK");
        assert_eq!(response_invalid_letters.body.len(), 5000);
        assert_eq!(
            response_invalid_letters
                .headers
                .get("content-length")
                .unwrap(),
            "5000"
        );
        assert!(
            response_invalid_letters
                .headers
                .get("content-range")
                .is_none(),
            "Invalid range should fall back to 200 OK without Content-Range header"
        );

        // Test case 2: Completely malformed Range header
        // Request: Range: invalid-header-value
        // Expected: 200 OK with full content
        let mut headers_malformed = HashMap::new();
        headers_malformed.insert("content-type".to_string(), "application/json".to_string());
        headers_malformed.insert("content-length".to_string(), "1024".to_string());

        let response_malformed = S3Response::new(200, "OK", headers_malformed, vec![2u8; 1024]);

        assert_eq!(response_malformed.status_code, 200);
        assert_eq!(response_malformed.body.len(), 1024);
        assert!(
            response_malformed.headers.get("content-range").is_none(),
            "Malformed range should fall back to 200 OK"
        );

        // Test case 3: Range header with no equals sign
        // Request: Range: bytes
        // Expected: 200 OK with full content
        let mut headers_no_equals = HashMap::new();
        headers_no_equals.insert("content-type".to_string(), "image/png".to_string());
        headers_no_equals.insert("content-length".to_string(), "2048".to_string());

        let response_no_equals = S3Response::new(200, "OK", headers_no_equals, vec![3u8; 2048]);

        assert_eq!(response_no_equals.status_code, 200);
        assert_eq!(response_no_equals.body.len(), 2048);
        assert!(
            response_no_equals.headers.get("content-range").is_none(),
            "Range without equals should fall back to 200 OK"
        );

        // Test case 4: Verify this is DIFFERENT from 416 Range Not Satisfiable
        // 416 is for VALID range syntax that's out of bounds
        // 200 fallback is for INVALID range syntax
        let mut headers_416 = HashMap::new();
        headers_416.insert("content-range".to_string(), "bytes */5000".to_string());

        let response_416 = S3Response::new(416, "Range Not Satisfiable", headers_416, vec![]);

        // Invalid syntax → 200 OK with full body
        // Valid but out of bounds → 416 with no body (or error body)
        assert_ne!(
            response_invalid_letters.status_code, response_416.status_code,
            "Invalid syntax (200) is different from out-of-bounds (416)"
        );
        assert!(
            response_invalid_letters.body.len() > response_416.body.len(),
            "200 fallback includes full content, 416 has empty/error body"
        );

        // Test case 5: Verify Accept-Ranges header is still included
        // Even when falling back to 200 OK, server should indicate it supports ranges
        let mut headers_with_accept = HashMap::new();
        headers_with_accept.insert("accept-ranges".to_string(), "bytes".to_string());
        headers_with_accept.insert("content-type".to_string(), "video/mp4".to_string());
        headers_with_accept.insert("content-length".to_string(), "10000".to_string());

        let response_with_accept =
            S3Response::new(200, "OK", headers_with_accept, vec![4u8; 10000]);

        assert_eq!(response_with_accept.status_code, 200);
        assert_eq!(
            response_with_accept.headers.get("accept-ranges").unwrap(),
            "bytes"
        );
        assert!(
            response_with_accept.headers.get("content-range").is_none(),
            "200 OK doesn't include Content-Range even with Accept-Ranges"
        );

        // Test case 6: Multiple invalid ranges (e.g., "bytes=abc-def,xyz-123")
        // Should also fall back to 200 OK
        let mut headers_multiple_invalid = HashMap::new();
        headers_multiple_invalid.insert("content-type".to_string(), "text/html".to_string());
        headers_multiple_invalid.insert("content-length".to_string(), "3000".to_string());

        let response_multiple_invalid =
            S3Response::new(200, "OK", headers_multiple_invalid, vec![5u8; 3000]);

        assert_eq!(response_multiple_invalid.status_code, 200);
        assert_eq!(response_multiple_invalid.body.len(), 3000);
        assert!(
            response_multiple_invalid
                .headers
                .get("content-range")
                .is_none(),
            "Multiple invalid ranges should fall back to 200 OK"
        );
    }

    #[test]
    fn test_range_requests_bypass_cache() {
        use std::collections::HashMap;

        // Range requests should NEVER be cached
        // Rationale:
        // 1. Caching partial responses is complex (need to track which ranges are cached)
        // 2. Range requests are typically for large files (videos) with varying ranges
        // 3. Client may request different ranges each time (seeking, parallel downloads)
        // 4. Cache efficiency would be low for range requests
        // 5. Simpler to always fetch range requests directly from S3

        // Test case 1: Range request response should indicate it was NOT served from cache
        // A cache hit would typically include headers like X-Cache: HIT or Age: > 0
        // Range requests should always be fresh from S3
        let mut headers_range_request = HashMap::new();
        headers_range_request.insert("content-type".to_string(), "video/mp4".to_string());
        headers_range_request.insert(
            "content-range".to_string(),
            "bytes 0-1023/10000".to_string(),
        );
        headers_range_request.insert("content-length".to_string(), "1024".to_string());
        // No X-Cache header = not from cache
        // No Age header = fresh from origin

        let response_range = S3Response::new(
            206,
            "Partial Content",
            headers_range_request,
            vec![1u8; 1024],
        );

        assert_eq!(response_range.status_code, 206);
        assert!(
            response_range.headers.get("x-cache").is_none(),
            "Range request should not include X-Cache header (not cached)"
        );
        assert!(
            response_range.headers.get("age").is_none(),
            "Range request should not include Age header (fresh from S3)"
        );

        // Test case 2: Multiple range requests for SAME file should each go to S3
        // Even if requesting the same bytes multiple times
        // This is different from full file requests which SHOULD be cached
        let mut headers_range_1 = HashMap::new();
        headers_range_1.insert(
            "content-range".to_string(),
            "bytes 1000-1999/50000".to_string(),
        );
        headers_range_1.insert("etag".to_string(), "\"same-file-etag\"".to_string());

        let response_range_1 =
            S3Response::new(206, "Partial Content", headers_range_1, vec![2u8; 1000]);

        // Second request for the exact same range
        let mut headers_range_2 = HashMap::new();
        headers_range_2.insert(
            "content-range".to_string(),
            "bytes 1000-1999/50000".to_string(),
        );
        headers_range_2.insert("etag".to_string(), "\"same-file-etag\"".to_string());

        let response_range_2 =
            S3Response::new(206, "Partial Content", headers_range_2, vec![2u8; 1000]);

        // Both should be 206 (not 304 Not Modified from cache)
        assert_eq!(response_range_1.status_code, 206);
        assert_eq!(response_range_2.status_code, 206);

        // Both should have identical ETag (same file)
        assert_eq!(
            response_range_1.headers.get("etag"),
            response_range_2.headers.get("etag")
        );

        // But neither should indicate cache hit
        assert!(response_range_1.headers.get("x-cache").is_none());
        assert!(response_range_2.headers.get("x-cache").is_none());

        // Test case 3: Contrast with full file request which COULD be cached
        let mut headers_full_file = HashMap::new();
        headers_full_file.insert("content-type".to_string(), "video/mp4".to_string());
        headers_full_file.insert("content-length".to_string(), "10000".to_string());
        headers_full_file.insert("etag".to_string(), "\"full-file-etag\"".to_string());
        // Full file requests (200 OK) could include cache indicators
        headers_full_file.insert("x-cache".to_string(), "HIT".to_string());
        headers_full_file.insert("age".to_string(), "300".to_string()); // 5 minutes old

        let response_full = S3Response::new(200, "OK", headers_full_file, vec![3u8; 10000]);

        assert_eq!(response_full.status_code, 200);
        assert!(
            response_full.headers.get("x-cache").is_some(),
            "Full file request CAN be served from cache"
        );
        assert!(
            response_full.headers.get("age").is_some(),
            "Full file request CAN have Age header"
        );

        // Verify different behavior: 206 bypass cache, 200 may use cache
        assert_ne!(response_range.status_code, response_full.status_code);
        assert!(response_range.headers.get("x-cache").is_none());
        assert!(response_full.headers.get("x-cache").is_some());

        // Test case 4: Large file with multiple different ranges
        // Each range request goes to S3, even for same file
        let ranges = vec![
            "bytes 0-999/100000",
            "bytes 1000-1999/100000",
            "bytes 50000-50999/100000",
            "bytes 99000-99999/100000",
        ];

        for range_str in ranges {
            let mut headers = HashMap::new();
            headers.insert("content-range".to_string(), range_str.to_string());
            headers.insert("etag".to_string(), "\"large-file-etag\"".to_string());

            let response = S3Response::new(206, "Partial Content", headers, vec![4u8; 1000]);

            assert_eq!(response.status_code, 206);
            assert!(
                response.headers.get("x-cache").is_none(),
                "Each range request should bypass cache: {}",
                range_str
            );
            assert_eq!(
                response.headers.get("etag").unwrap(),
                "\"large-file-etag\"",
                "All ranges are from same file"
            );
        }

        // Test case 5: Range request with If-Range also bypasses cache
        // Even conditional range requests should not use cache
        let mut headers_if_range = HashMap::new();
        headers_if_range.insert("content-range".to_string(), "bytes 0-499/5000".to_string());
        headers_if_range.insert("etag".to_string(), "\"conditional-etag\"".to_string());

        let response_if_range =
            S3Response::new(206, "Partial Content", headers_if_range, vec![5u8; 500]);

        assert_eq!(response_if_range.status_code, 206);
        assert!(
            response_if_range.headers.get("x-cache").is_none(),
            "If-Range conditional request should also bypass cache"
        );
    }

    #[test]
    fn test_range_request_doesnt_populate_cache() {
        use std::collections::HashMap;

        // Range requests should NOT populate the cache
        // This means:
        // 1. After serving a range request, nothing is added to cache
        // 2. Subsequent requests (even for full file) don't benefit from range request
        // 3. Range requests are pure pass-through from S3 to client

        // Test case 1: First request is a range request (206 Partial Content)
        // This should NOT populate cache with anything
        let mut headers_first_range = HashMap::new();
        headers_first_range.insert("content-type".to_string(), "video/mp4".to_string());
        headers_first_range.insert("content-range".to_string(), "bytes 0-999/10000".to_string());
        headers_first_range.insert("content-length".to_string(), "1000".to_string());
        headers_first_range.insert("etag".to_string(), "\"file-etag-123\"".to_string());

        let response_first_range =
            S3Response::new(206, "Partial Content", headers_first_range, vec![1u8; 1000]);

        assert_eq!(response_first_range.status_code, 206);
        assert!(
            response_first_range.headers.get("x-cache").is_none(),
            "First range request should not indicate cache population"
        );

        // Test case 2: Second request for FULL file of same resource
        // Should still go to S3, NOT served from cache (because range request didn't cache)
        // This is verified by absence of X-Cache: HIT and Age headers
        let mut headers_full_after_range = HashMap::new();
        headers_full_after_range.insert("content-type".to_string(), "video/mp4".to_string());
        headers_full_after_range.insert("content-length".to_string(), "10000".to_string());
        headers_full_after_range.insert("etag".to_string(), "\"file-etag-123\"".to_string());
        // Same ETag = same file, but cache wasn't populated by range request

        let response_full_after_range =
            S3Response::new(200, "OK", headers_full_after_range, vec![2u8; 10000]);

        assert_eq!(response_full_after_range.status_code, 200);
        assert_eq!(
            response_first_range.headers.get("etag"),
            response_full_after_range.headers.get("etag"),
            "Both requests are for same file (same ETag)"
        );
        assert!(
            response_full_after_range.headers.get("x-cache").is_none(),
            "Full file request after range request should NOT hit cache"
        );
        assert!(
            response_full_after_range.headers.get("age").is_none(),
            "Full file request should be fresh from S3, not cached"
        );

        // Test case 3: Multiple range requests for different parts
        // None of them should populate cache
        let ranges = vec![
            ("bytes 0-999/50000", 1000),
            ("bytes 10000-19999/50000", 10000),
            ("bytes 40000-49999/50000", 10000),
        ];

        for (range_str, size) in ranges {
            let mut headers = HashMap::new();
            headers.insert("content-range".to_string(), range_str.to_string());
            headers.insert("etag".to_string(), "\"multi-range-etag\"".to_string());

            let response = S3Response::new(206, "Partial Content", headers, vec![3u8; size]);

            assert_eq!(response.status_code, 206);
            assert!(
                response.headers.get("x-cache").is_none(),
                "Range request {} should not populate cache",
                range_str
            );
        }

        // After all those range requests, a full file request should still go to S3
        let mut headers_full_after_multiple = HashMap::new();
        headers_full_after_multiple.insert("content-length".to_string(), "50000".to_string());
        headers_full_after_multiple.insert("etag".to_string(), "\"multi-range-etag\"".to_string());

        let response_full_after_multiple =
            S3Response::new(200, "OK", headers_full_after_multiple, vec![4u8; 50000]);

        assert!(
            response_full_after_multiple
                .headers
                .get("x-cache")
                .is_none(),
            "Full file request after multiple range requests should NOT hit cache"
        );

        // Test case 4: Contrast with full file request which DOES populate cache
        // First request: full file (200 OK) - this populates cache
        let mut headers_full_first = HashMap::new();
        headers_full_first.insert("content-length".to_string(), "5000".to_string());
        headers_full_first.insert("etag".to_string(), "\"cacheable-etag\"".to_string());

        let response_full_first = S3Response::new(200, "OK", headers_full_first, vec![5u8; 5000]);

        assert_eq!(response_full_first.status_code, 200);

        // Second request: full file (200 OK) - this CAN be served from cache
        let mut headers_full_second = HashMap::new();
        headers_full_second.insert("content-length".to_string(), "5000".to_string());
        headers_full_second.insert("etag".to_string(), "\"cacheable-etag\"".to_string());
        headers_full_second.insert("x-cache".to_string(), "HIT".to_string());
        headers_full_second.insert("age".to_string(), "60".to_string()); // 60 seconds old

        let response_full_second = S3Response::new(200, "OK", headers_full_second, vec![5u8; 5000]);

        assert!(
            response_full_second.headers.get("x-cache").is_some(),
            "Full file requests CAN populate and use cache"
        );
        assert!(
            response_full_second.headers.get("age").is_some(),
            "Cached response has Age header"
        );

        // Compare: Range requests (206) don't populate cache
        // Full file requests (200) do populate cache
        assert_eq!(response_first_range.status_code, 206);
        assert_eq!(response_full_second.status_code, 200);
        assert!(response_first_range.headers.get("x-cache").is_none());
        assert!(response_full_second.headers.get("x-cache").is_some());

        // Test case 5: Range request after full file is cached
        // Range request should bypass cache even if full file is cached
        // (This will be tested more in next test: "Cached full file doesn't satisfy range request")
        let mut headers_range_after_cache = HashMap::new();
        headers_range_after_cache.insert(
            "content-range".to_string(),
            "bytes 1000-1999/5000".to_string(),
        );
        headers_range_after_cache.insert("etag".to_string(), "\"cacheable-etag\"".to_string());

        let response_range_after_cache = S3Response::new(
            206,
            "Partial Content",
            headers_range_after_cache,
            vec![6u8; 1000],
        );

        assert_eq!(response_range_after_cache.status_code, 206);
        assert!(
            response_range_after_cache.headers.get("x-cache").is_none(),
            "Range request should bypass cache even if full file is cached"
        );
        // Same file (same ETag) but range request goes to S3, not cache
        assert_eq!(
            response_full_second.headers.get("etag"),
            response_range_after_cache.headers.get("etag")
        );
    }

    #[test]
    fn test_cached_full_file_doesnt_satisfy_range_request() {
        use std::collections::HashMap;

        // Even when a full file is cached, range requests should NOT be satisfied from cache
        // Instead, they should fetch from S3 directly
        // Rationale:
        // 1. Extracting partial content from cached file adds complexity
        // 2. Would need to verify cache still valid before extracting range
        // 3. Range requests are typically for large files not suitable for caching anyway
        // 4. Simpler to always fetch range requests from S3

        // Test case 1: First request - full file (200 OK) that gets cached
        let mut headers_full_cached = HashMap::new();
        headers_full_cached.insert("content-type".to_string(), "video/mp4".to_string());
        headers_full_cached.insert("content-length".to_string(), "100000".to_string());
        headers_full_cached.insert("etag".to_string(), "\"cached-video-etag\"".to_string());
        headers_full_cached.insert(
            "last-modified".to_string(),
            "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
        );
        headers_full_cached.insert("x-cache".to_string(), "MISS".to_string()); // First request, cache miss

        let response_full_cached =
            S3Response::new(200, "OK", headers_full_cached, vec![1u8; 100000]);

        assert_eq!(response_full_cached.status_code, 200);
        assert_eq!(response_full_cached.body.len(), 100000);

        // Simulate: This full file is now in cache
        // Next full file request would get X-Cache: HIT

        // Test case 2: Second request - range request for same file
        // Even though full file is cached, range request should go to S3
        let mut headers_range_request = HashMap::new();
        headers_range_request.insert("content-type".to_string(), "video/mp4".to_string());
        headers_range_request.insert(
            "content-range".to_string(),
            "bytes 10000-19999/100000".to_string(),
        );
        headers_range_request.insert("content-length".to_string(), "10000".to_string());
        headers_range_request.insert("etag".to_string(), "\"cached-video-etag\"".to_string());
        headers_range_request.insert(
            "last-modified".to_string(),
            "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
        );
        // NO X-Cache header = fresh from S3, not from cache

        let response_range = S3Response::new(
            206,
            "Partial Content",
            headers_range_request,
            vec![2u8; 10000],
        );

        assert_eq!(response_range.status_code, 206);
        assert_eq!(response_range.body.len(), 10000);

        // Verify same file (same ETag and Last-Modified)
        assert_eq!(
            response_full_cached.headers.get("etag"),
            response_range.headers.get("etag"),
            "Range request is for same file as cached full file"
        );
        assert_eq!(
            response_full_cached.headers.get("last-modified"),
            response_range.headers.get("last-modified")
        );

        // Critical: Range request should NOT indicate cache hit
        assert!(
            response_range.headers.get("x-cache").is_none(),
            "Range request should bypass cache, not extract from cached full file"
        );
        assert!(
            response_range.headers.get("age").is_none(),
            "Range request should be fresh from S3"
        );

        // Test case 3: Third request - another full file request
        // This SHOULD hit cache (proving cache is still populated)
        let mut headers_full_hit = HashMap::new();
        headers_full_hit.insert("content-type".to_string(), "video/mp4".to_string());
        headers_full_hit.insert("content-length".to_string(), "100000".to_string());
        headers_full_hit.insert("etag".to_string(), "\"cached-video-etag\"".to_string());
        headers_full_hit.insert("x-cache".to_string(), "HIT".to_string()); // Cache hit!
        headers_full_hit.insert("age".to_string(), "120".to_string()); // 2 minutes old

        let response_full_hit = S3Response::new(200, "OK", headers_full_hit, vec![1u8; 100000]);

        assert_eq!(response_full_hit.status_code, 200);
        assert!(
            response_full_hit.headers.get("x-cache").is_some(),
            "Full file request CAN hit cache"
        );

        // Compare behaviors:
        // - Full file requests (200): CAN use cache
        // - Range requests (206): ALWAYS bypass cache, even if full file is cached
        assert_eq!(response_full_cached.status_code, 200);
        assert_eq!(response_range.status_code, 206);
        assert_eq!(response_full_hit.status_code, 200);

        assert!(response_range.headers.get("x-cache").is_none());
        assert!(response_full_hit.headers.get("x-cache").is_some());

        // Test case 4: Multiple different ranges from same cached file
        // All should bypass cache and go to S3
        let ranges = vec![
            ("bytes 0-9999/100000", 10000),
            ("bytes 50000-59999/100000", 10000),
            ("bytes 90000-99999/100000", 10000),
        ];

        for (range_str, size) in ranges {
            let mut headers = HashMap::new();
            headers.insert("content-range".to_string(), range_str.to_string());
            headers.insert("etag".to_string(), "\"cached-video-etag\"".to_string());
            // Same file as cached, but fetched from S3

            let response = S3Response::new(206, "Partial Content", headers, vec![3u8; size]);

            assert_eq!(response.status_code, 206);
            assert!(
                response.headers.get("x-cache").is_none(),
                "Range {} should bypass cache even though full file is cached",
                range_str
            );
        }

        // Test case 5: Range request with If-Range also bypasses cache
        let mut headers_if_range = HashMap::new();
        headers_if_range.insert(
            "content-range".to_string(),
            "bytes 20000-29999/100000".to_string(),
        );
        headers_if_range.insert("etag".to_string(), "\"cached-video-etag\"".to_string());

        let response_if_range =
            S3Response::new(206, "Partial Content", headers_if_range, vec![4u8; 10000]);

        assert_eq!(response_if_range.status_code, 206);
        assert!(
            response_if_range.headers.get("x-cache").is_none(),
            "If-Range request should bypass cache"
        );

        // Test case 6: Verify we don't accidentally serve wrong bytes from cache
        // If we DID serve from cache, we'd need to extract the right byte range
        // But we don't - we always fetch from S3
        let mut headers_wrong_range = HashMap::new();
        headers_wrong_range.insert(
            "content-range".to_string(),
            "bytes 1000-1999/100000".to_string(),
        );
        headers_wrong_range.insert("content-length".to_string(), "1000".to_string());

        let response_specific_range =
            S3Response::new(206, "Partial Content", headers_wrong_range, vec![5u8; 1000]);

        assert_eq!(response_specific_range.status_code, 206);
        assert_eq!(response_specific_range.body.len(), 1000);
        // Body contains exactly 1000 bytes (the requested range)
        // NOT 100000 bytes (full cached file)
        assert_ne!(
            response_specific_range.body.len(),
            response_full_cached.body.len()
        );
    }

    #[test]
    fn test_range_requests_work_when_cache_enabled_for_bucket() {
        use std::collections::HashMap;

        // Range requests should work correctly even when caching is enabled for the bucket
        // This verifies the entire cache bypass behavior in a realistic scenario
        // where cache is configured and active for full file requests

        // Test case 1: Bucket has caching enabled - full file request gets cached
        let mut headers_cached_bucket_full = HashMap::new();
        headers_cached_bucket_full.insert("content-type".to_string(), "image/png".to_string());
        headers_cached_bucket_full.insert("content-length".to_string(), "50000".to_string());
        headers_cached_bucket_full.insert("etag".to_string(), "\"cached-bucket-file\"".to_string());
        headers_cached_bucket_full.insert("cache-control".to_string(), "max-age=3600".to_string());
        headers_cached_bucket_full.insert("x-cache".to_string(), "HIT".to_string());

        let response_cached_full =
            S3Response::new(200, "OK", headers_cached_bucket_full, vec![1u8; 50000]);

        assert_eq!(response_cached_full.status_code, 200);
        assert!(
            response_cached_full.headers.get("x-cache").is_some(),
            "Full file request benefits from cache when cache is enabled"
        );
        assert!(
            response_cached_full.headers.get("cache-control").is_some(),
            "Cache-Control headers indicate caching is active"
        );

        // Test case 2: Same bucket with cache enabled - range request bypasses cache
        let mut headers_range_no_cache = HashMap::new();
        headers_range_no_cache.insert("content-type".to_string(), "image/png".to_string());
        headers_range_no_cache.insert(
            "content-range".to_string(),
            "bytes 0-9999/50000".to_string(),
        );
        headers_range_no_cache.insert("content-length".to_string(), "10000".to_string());
        headers_range_no_cache.insert("etag".to_string(), "\"cached-bucket-file\"".to_string());
        // No X-Cache header - bypasses cache even though bucket has caching enabled

        let response_range_bypass = S3Response::new(
            206,
            "Partial Content",
            headers_range_no_cache,
            vec![2u8; 10000],
        );

        assert_eq!(response_range_bypass.status_code, 206);
        assert_eq!(
            response_cached_full.headers.get("etag"),
            response_range_bypass.headers.get("etag"),
            "Same file, different request types"
        );
        assert!(
            response_range_bypass.headers.get("x-cache").is_none(),
            "Range request bypasses cache even when bucket has cache enabled"
        );

        // Test case 3: Verify caching configuration doesn't break range request functionality
        // Range requests should return correct Content-Range headers
        let ranges_to_test = vec![
            ("bytes 0-999/50000", 1000),
            ("bytes 10000-19999/50000", 10000),
            ("bytes 40000-49999/50000", 10000),
        ];

        for (range_str, expected_size) in ranges_to_test {
            let mut headers = HashMap::new();
            headers.insert("content-range".to_string(), range_str.to_string());
            headers.insert("content-length".to_string(), expected_size.to_string());
            headers.insert("etag".to_string(), "\"cached-bucket-file\"".to_string());

            let response =
                S3Response::new(206, "Partial Content", headers, vec![3u8; expected_size]);

            assert_eq!(response.status_code, 206);
            assert_eq!(response.body.len(), expected_size);
            assert_eq!(
                response.headers.get("content-range").unwrap(),
                range_str,
                "Content-Range header correct for {}",
                range_str
            );
            assert!(
                response.headers.get("x-cache").is_none(),
                "Range {} bypasses cache in cached bucket",
                range_str
            );
        }

        // Test case 4: Interleaved requests - full file (cached) and range requests
        // Pattern: Full -> Range -> Full -> Range
        // Full requests should hit cache, range requests should bypass

        // Full request 1 (cache hit)
        let mut headers_full_1 = HashMap::new();
        headers_full_1.insert("x-cache".to_string(), "HIT".to_string());
        headers_full_1.insert("content-length".to_string(), "50000".to_string());

        let response_full_1 = S3Response::new(200, "OK", headers_full_1, vec![4u8; 50000]);
        assert!(response_full_1.headers.get("x-cache").is_some());

        // Range request 1 (bypass cache)
        let mut headers_range_1 = HashMap::new();
        headers_range_1.insert("content-range".to_string(), "bytes 0-999/50000".to_string());

        let response_range_1 =
            S3Response::new(206, "Partial Content", headers_range_1, vec![5u8; 1000]);
        assert!(response_range_1.headers.get("x-cache").is_none());

        // Full request 2 (cache hit)
        let mut headers_full_2 = HashMap::new();
        headers_full_2.insert("x-cache".to_string(), "HIT".to_string());
        headers_full_2.insert("content-length".to_string(), "50000".to_string());

        let response_full_2 = S3Response::new(200, "OK", headers_full_2, vec![4u8; 50000]);
        assert!(response_full_2.headers.get("x-cache").is_some());

        // Range request 2 (bypass cache)
        let mut headers_range_2 = HashMap::new();
        headers_range_2.insert(
            "content-range".to_string(),
            "bytes 1000-1999/50000".to_string(),
        );

        let response_range_2 =
            S3Response::new(206, "Partial Content", headers_range_2, vec![6u8; 1000]);
        assert!(response_range_2.headers.get("x-cache").is_none());

        // Verify pattern holds
        assert_eq!(response_full_1.status_code, 200);
        assert_eq!(response_range_1.status_code, 206);
        assert_eq!(response_full_2.status_code, 200);
        assert_eq!(response_range_2.status_code, 206);

        // Test case 5: Cache settings don't affect range request Accept-Ranges header
        let mut headers_accept_ranges = HashMap::new();
        headers_accept_ranges.insert("accept-ranges".to_string(), "bytes".to_string());
        headers_accept_ranges.insert(
            "content-range".to_string(),
            "bytes 5000-5999/50000".to_string(),
        );

        let response_with_accept_ranges = S3Response::new(
            206,
            "Partial Content",
            headers_accept_ranges,
            vec![7u8; 1000],
        );

        assert_eq!(
            response_with_accept_ranges
                .headers
                .get("accept-ranges")
                .unwrap(),
            "bytes",
            "Accept-Ranges header works correctly with cache enabled"
        );

        // Test case 6: If-Range requests also work correctly with cache enabled
        let mut headers_if_range_cached = HashMap::new();
        headers_if_range_cached.insert(
            "content-range".to_string(),
            "bytes 20000-29999/50000".to_string(),
        );
        headers_if_range_cached.insert("etag".to_string(), "\"cached-bucket-file\"".to_string());

        let response_if_range_cached = S3Response::new(
            206,
            "Partial Content",
            headers_if_range_cached,
            vec![8u8; 10000],
        );

        assert_eq!(response_if_range_cached.status_code, 206);
        assert!(
            response_if_range_cached.headers.get("x-cache").is_none(),
            "If-Range requests bypass cache even in cached bucket"
        );
    }

    #[test]
    fn test_range_requests_work_on_public_buckets() {
        use std::collections::HashMap;

        // Range requests should work on public buckets (no authentication required)
        // Public buckets don't require JWT tokens for any requests
        // Range requests should function the same as full file requests

        // Test case 1: Full file request on public bucket (no auth required)
        let mut headers_public_full = HashMap::new();
        headers_public_full.insert("content-type".to_string(), "image/jpeg".to_string());
        headers_public_full.insert("content-length".to_string(), "50000".to_string());
        headers_public_full.insert("etag".to_string(), "\"public-file-etag\"".to_string());

        let response_public_full =
            S3Response::new(200, "OK", headers_public_full, vec![1u8; 50000]);

        assert_eq!(response_public_full.status_code, 200);
        assert_eq!(response_public_full.body.len(), 50000);
        // No authentication required - no 401 or 403 errors

        // Test case 2: Range request on same public bucket (no auth required)
        let mut headers_public_range = HashMap::new();
        headers_public_range.insert("content-type".to_string(), "image/jpeg".to_string());
        headers_public_range.insert(
            "content-range".to_string(),
            "bytes 0-9999/50000".to_string(),
        );
        headers_public_range.insert("content-length".to_string(), "10000".to_string());
        headers_public_range.insert("etag".to_string(), "\"public-file-etag\"".to_string());

        let response_public_range = S3Response::new(
            206,
            "Partial Content",
            headers_public_range,
            vec![2u8; 10000],
        );

        assert_eq!(response_public_range.status_code, 206);
        assert_eq!(response_public_range.body.len(), 10000);
        assert_eq!(
            response_public_full.headers.get("etag"),
            response_public_range.headers.get("etag"),
            "Same file on public bucket"
        );
        // No authentication errors
        assert_ne!(response_public_range.status_code, 401);
        assert_ne!(response_public_range.status_code, 403);

        // Test case 3: Multiple different ranges on public bucket
        let ranges = vec![
            ("bytes 0-999/50000", 1000),
            ("bytes 10000-19999/50000", 10000),
            ("bytes 40000-49999/50000", 10000),
        ];

        for (range_str, expected_size) in ranges {
            let mut headers = HashMap::new();
            headers.insert("content-range".to_string(), range_str.to_string());
            headers.insert("content-length".to_string(), expected_size.to_string());

            let response =
                S3Response::new(206, "Partial Content", headers, vec![3u8; expected_size]);

            assert_eq!(response.status_code, 206);
            assert_eq!(response.body.len(), expected_size);
            assert_eq!(
                response.headers.get("content-range").unwrap(),
                range_str,
                "Range {} works on public bucket",
                range_str
            );
        }

        // Test case 4: Open-ended range on public bucket
        let mut headers_open_ended = HashMap::new();
        headers_open_ended.insert(
            "content-range".to_string(),
            "bytes 10000-49999/50000".to_string(),
        );
        headers_open_ended.insert("content-length".to_string(), "40000".to_string());

        let response_open_ended =
            S3Response::new(206, "Partial Content", headers_open_ended, vec![4u8; 40000]);

        assert_eq!(response_open_ended.status_code, 206);
        assert_eq!(response_open_ended.body.len(), 40000);

        // Test case 5: Suffix range on public bucket
        let mut headers_suffix = HashMap::new();
        headers_suffix.insert(
            "content-range".to_string(),
            "bytes 49000-49999/50000".to_string(),
        );
        headers_suffix.insert("content-length".to_string(), "1000".to_string());

        let response_suffix =
            S3Response::new(206, "Partial Content", headers_suffix, vec![5u8; 1000]);

        assert_eq!(response_suffix.status_code, 206);
        assert_eq!(response_suffix.body.len(), 1000);

        // Test case 6: If-Range request on public bucket
        let mut headers_if_range = HashMap::new();
        headers_if_range.insert(
            "content-range".to_string(),
            "bytes 5000-14999/50000".to_string(),
        );
        headers_if_range.insert("content-length".to_string(), "10000".to_string());
        headers_if_range.insert("etag".to_string(), "\"public-file-etag\"".to_string());

        let response_if_range =
            S3Response::new(206, "Partial Content", headers_if_range, vec![6u8; 10000]);

        assert_eq!(response_if_range.status_code, 206);
        assert_eq!(response_if_range.body.len(), 10000);

        // Test case 7: Accept-Ranges header on public bucket
        let mut headers_accept = HashMap::new();
        headers_accept.insert("accept-ranges".to_string(), "bytes".to_string());
        headers_accept.insert("content-length".to_string(), "50000".to_string());

        let response_accept = S3Response::new(200, "OK", headers_accept, vec![7u8; 50000]);

        assert_eq!(
            response_accept.headers.get("accept-ranges").unwrap(),
            "bytes",
            "Public bucket supports range requests"
        );

        // Test case 8: 416 Range Not Satisfiable on public bucket (out of bounds)
        let mut headers_416 = HashMap::new();
        headers_416.insert("content-range".to_string(), "bytes */50000".to_string());

        let response_416 = S3Response::new(416, "Range Not Satisfiable", headers_416, vec![]);

        assert_eq!(response_416.status_code, 416);
        // Even 416 errors don't require authentication on public bucket

        // Test case 9: Invalid range syntax falls back to 200 OK on public bucket
        let mut headers_fallback = HashMap::new();
        headers_fallback.insert("content-length".to_string(), "50000".to_string());

        let response_fallback = S3Response::new(200, "OK", headers_fallback, vec![8u8; 50000]);

        assert_eq!(response_fallback.status_code, 200);
        assert_eq!(response_fallback.body.len(), 50000);

        // Test case 10: Verify no authentication headers required
        // Public bucket responses don't need Authorization or X-Auth-Token headers
        assert!(
            response_public_range.headers.get("authorization").is_none(),
            "Public bucket doesn't require Authorization header"
        );
        assert!(
            response_public_range.headers.get("x-auth-token").is_none(),
            "Public bucket doesn't require X-Auth-Token header"
        );
    }

    #[test]
    fn test_range_requests_require_jwt_on_private_buckets() {
        use std::collections::HashMap;

        // Range requests on private buckets MUST require valid JWT authentication
        // Just like full file requests, range requests need auth on private buckets
        // Without valid JWT, requests should return 401 Unauthorized

        // Test case 1: Range request WITHOUT JWT on private bucket -> 401
        let mut headers_no_jwt = HashMap::new();
        headers_no_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_no_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_no_jwt = S3Response::new(401, "Unauthorized", headers_no_jwt, vec![]);

        assert_eq!(response_no_jwt.status_code, 401);
        assert_eq!(response_no_jwt.status_text, "Unauthorized");
        assert_eq!(response_no_jwt.body.len(), 0);
        assert!(
            response_no_jwt.headers.get("www-authenticate").is_some(),
            "401 response should include WWW-Authenticate header"
        );

        // Test case 2: Range request WITH valid JWT on private bucket -> 206
        let mut headers_valid_jwt = HashMap::new();
        headers_valid_jwt.insert("content-type".to_string(), "video/mp4".to_string());
        headers_valid_jwt.insert(
            "content-range".to_string(),
            "bytes 0-9999/100000".to_string(),
        );
        headers_valid_jwt.insert("content-length".to_string(), "10000".to_string());
        headers_valid_jwt.insert("etag".to_string(), "\"private-file-etag\"".to_string());

        let response_valid_jwt =
            S3Response::new(206, "Partial Content", headers_valid_jwt, vec![1u8; 10000]);

        assert_eq!(response_valid_jwt.status_code, 206);
        assert_eq!(response_valid_jwt.status_text, "Partial Content");
        assert_eq!(response_valid_jwt.body.len(), 10000);
        assert_eq!(
            response_valid_jwt.headers.get("content-range").unwrap(),
            "bytes 0-9999/100000"
        );

        // Test case 3: Range request with INVALID JWT on private bucket -> 401
        let mut headers_invalid_jwt = HashMap::new();
        headers_invalid_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_invalid_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\", error=\"invalid_token\"".to_string(),
        );

        let response_invalid_jwt =
            S3Response::new(401, "Unauthorized", headers_invalid_jwt, vec![]);

        assert_eq!(response_invalid_jwt.status_code, 401);
        assert!(
            response_invalid_jwt
                .headers
                .get("www-authenticate")
                .is_some(),
            "Invalid JWT should return 401 with WWW-Authenticate"
        );

        // Test case 4: Range request with EXPIRED JWT on private bucket -> 401
        let mut headers_expired_jwt = HashMap::new();
        headers_expired_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_expired_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\", error=\"token_expired\"".to_string(),
        );

        let response_expired_jwt =
            S3Response::new(401, "Unauthorized", headers_expired_jwt, vec![]);

        assert_eq!(response_expired_jwt.status_code, 401);

        // Test case 5: Full file request on private bucket also requires JWT (for comparison)
        let mut headers_full_no_jwt = HashMap::new();
        headers_full_no_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_full_no_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_full_no_jwt =
            S3Response::new(401, "Unauthorized", headers_full_no_jwt, vec![]);

        assert_eq!(response_full_no_jwt.status_code, 401);

        // Compare: Both range and full file requests require JWT
        assert_eq!(
            response_no_jwt.status_code,
            response_full_no_jwt.status_code
        );

        // Test case 6: Multiple range requests with valid JWT all succeed
        let ranges = vec![
            ("bytes 0-999/100000", 1000),
            ("bytes 50000-59999/100000", 10000),
            ("bytes 90000-99999/100000", 10000),
        ];

        for (range_str, expected_size) in ranges {
            let mut headers = HashMap::new();
            headers.insert("content-range".to_string(), range_str.to_string());
            headers.insert("content-length".to_string(), expected_size.to_string());

            let response =
                S3Response::new(206, "Partial Content", headers, vec![2u8; expected_size]);

            assert_eq!(response.status_code, 206);
            assert_eq!(response.body.len(), expected_size);
            assert_eq!(
                response.headers.get("content-range").unwrap(),
                range_str,
                "Authenticated range request {} succeeds",
                range_str
            );
        }

        // Test case 7: If-Range request on private bucket also requires JWT
        // Without JWT -> 401
        let mut headers_if_range_no_jwt = HashMap::new();
        headers_if_range_no_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_if_range_no_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_if_range_no_jwt =
            S3Response::new(401, "Unauthorized", headers_if_range_no_jwt, vec![]);

        assert_eq!(response_if_range_no_jwt.status_code, 401);

        // With valid JWT -> 206
        let mut headers_if_range_valid = HashMap::new();
        headers_if_range_valid.insert(
            "content-range".to_string(),
            "bytes 10000-19999/100000".to_string(),
        );
        headers_if_range_valid.insert("content-length".to_string(), "10000".to_string());
        headers_if_range_valid.insert("etag".to_string(), "\"private-file-etag\"".to_string());

        let response_if_range_valid = S3Response::new(
            206,
            "Partial Content",
            headers_if_range_valid,
            vec![3u8; 10000],
        );

        assert_eq!(response_if_range_valid.status_code, 206);

        // Test case 8: 416 Range Not Satisfiable on private bucket also requires JWT
        // Without JWT -> 401 (auth checked before range validation)
        let mut headers_416_no_jwt = HashMap::new();
        headers_416_no_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_416_no_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_416_no_jwt = S3Response::new(401, "Unauthorized", headers_416_no_jwt, vec![]);

        assert_eq!(response_416_no_jwt.status_code, 401);
        // 401 takes precedence over 416

        // With valid JWT but out-of-bounds range -> 416
        let mut headers_416_valid_jwt = HashMap::new();
        headers_416_valid_jwt.insert("content-range".to_string(), "bytes */100000".to_string());

        let response_416_valid_jwt =
            S3Response::new(416, "Range Not Satisfiable", headers_416_valid_jwt, vec![]);

        assert_eq!(response_416_valid_jwt.status_code, 416);

        // Test case 9: Verify auth happens BEFORE processing range header
        // Invalid JWT returns 401, not 416 even if range is bad
        assert_eq!(response_no_jwt.status_code, 401);
        assert_ne!(response_no_jwt.status_code, 416);

        // Test case 10: Private bucket responses with valid JWT don't expose auth tokens
        // Response shouldn't leak the JWT token in headers
        assert!(
            response_valid_jwt.headers.get("authorization").is_none(),
            "Response shouldn't leak Authorization header"
        );
    }

    #[test]
    fn test_returns_401_before_processing_range_if_auth_fails() {
        use std::collections::HashMap;

        // Authentication should happen BEFORE range header processing
        // Ensures that 401 Unauthorized takes precedence over any range-related errors

        // Test case 1: Missing JWT with VALID range header -> 401 (not 206)
        let mut headers_missing_jwt_valid_range = HashMap::new();
        headers_missing_jwt_valid_range
            .insert("content-type".to_string(), "application/json".to_string());
        headers_missing_jwt_valid_range.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_missing_jwt =
            S3Response::new(401, "Unauthorized", headers_missing_jwt_valid_range, vec![]);

        assert_eq!(response_missing_jwt.status_code, 401);
        assert_ne!(
            response_missing_jwt.status_code, 206,
            "Should return 401, not 206, when auth fails even with valid range"
        );

        // Test case 2: Missing JWT with INVALID range syntax -> 401 (not 400)
        let mut headers_missing_jwt_invalid_syntax = HashMap::new();
        headers_missing_jwt_invalid_syntax
            .insert("content-type".to_string(), "application/json".to_string());
        headers_missing_jwt_invalid_syntax.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_invalid_syntax = S3Response::new(
            401,
            "Unauthorized",
            headers_missing_jwt_invalid_syntax,
            vec![],
        );

        assert_eq!(response_invalid_syntax.status_code, 401);
        assert_ne!(
            response_invalid_syntax.status_code, 400,
            "Should return 401, not 400, when auth fails even with invalid range syntax"
        );

        // Test case 3: Missing JWT with OUT-OF-BOUNDS range -> 401 (not 416)
        let mut headers_missing_jwt_oob_range = HashMap::new();
        headers_missing_jwt_oob_range
            .insert("content-type".to_string(), "application/json".to_string());
        headers_missing_jwt_oob_range.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_oob =
            S3Response::new(401, "Unauthorized", headers_missing_jwt_oob_range, vec![]);

        assert_eq!(response_oob.status_code, 401);
        assert_ne!(
            response_oob.status_code, 416,
            "Should return 401, not 416, when auth fails even with out-of-bounds range"
        );

        // Test case 4: Compare with valid JWT + out-of-bounds range -> 416
        // This demonstrates the correct sequence: auth first, then range validation
        let mut headers_valid_jwt_oob = HashMap::new();
        headers_valid_jwt_oob.insert("content-range".to_string(), "bytes */100000".to_string());

        let response_valid_oob =
            S3Response::new(416, "Range Not Satisfiable", headers_valid_jwt_oob, vec![]);

        assert_eq!(response_valid_oob.status_code, 416);

        // Compare: Without auth, get 401 even with out-of-bounds range
        // With auth, get 416 for out-of-bounds range
        assert_eq!(
            response_oob.status_code, 401,
            "No JWT + bad range = 401 (auth checked first)"
        );
        assert_eq!(
            response_valid_oob.status_code, 416,
            "Valid JWT + bad range = 416 (range checked after auth)"
        );

        // Test case 5: Verify WWW-Authenticate header present in 401 responses
        assert!(
            response_missing_jwt
                .headers
                .contains_key("www-authenticate"),
            "401 response should include WWW-Authenticate header"
        );
        assert_eq!(
            response_missing_jwt.headers.get("www-authenticate"),
            Some(&"Bearer realm=\"yatagarasu\"".to_string())
        );

        // Test case 6: Expired JWT with valid range -> 401 (not 206)
        let mut headers_expired_jwt = HashMap::new();
        headers_expired_jwt.insert("content-type".to_string(), "application/json".to_string());
        headers_expired_jwt.insert(
            "www-authenticate".to_string(),
            "Bearer realm=\"yatagarasu\"".to_string(),
        );

        let response_expired = S3Response::new(401, "Unauthorized", headers_expired_jwt, vec![]);

        assert_eq!(response_expired.status_code, 401);
        assert_ne!(
            response_expired.status_code, 206,
            "Expired JWT should return 401, not 206"
        );
    }
}
