// S3 client module

use crate::config::S3Config;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{config::Region, Client as AwsS3Client};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Metadata for an S3 object
#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: i64,
    pub etag: String,
    pub last_modified: String,
}

/// Result of a LIST operation
#[derive(Debug, Clone)]
pub struct ListResult {
    pub objects: Vec<ObjectMeta>,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
    pub common_prefixes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct S3Client {
    pub config: S3Config,
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

impl S3Client {
    pub async fn create_aws_client(&self) -> AwsS3Client {
        let creds = Credentials::new(
            self.config.access_key.clone(),
            self.config.secret_key.clone(),
            None,
            None,
            "static",
        );

        let region = Region::new(self.config.region.clone());

        let mut config_builder = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .region(region)
            .credentials_provider(creds);

        if let Some(endpoint) = &self.config.endpoint {
            config_builder = config_builder.endpoint_url(endpoint.clone());
            config_builder = config_builder.force_path_style(true);
        }

        AwsS3Client::from_conf(config_builder.build())
    }

    /// List objects in the bucket (ListObjectsV2)
    pub async fn list_objects(
        &self,
        prefix: Option<&str>,
        continuation_token: Option<&str>,
        max_keys: Option<i32>,
    ) -> Result<ListResult, String> {
        let client = self.create_aws_client().await;

        let mut req = client.list_objects_v2().bucket(&self.config.bucket);

        if let Some(p) = prefix {
            req = req.prefix(p);
        }

        if let Some(token) = continuation_token {
            req = req.continuation_token(token);
        }

        if let Some(max) = max_keys {
            req = req.max_keys(max);
        }

        match req.send().await {
            Ok(output) => {
                let objects = output
                    .contents()
                    .iter()
                    .map(|o| ObjectMeta {
                        key: o.key().unwrap_or("").to_string(),
                        size: o.size().unwrap_or(0),
                        etag: o.e_tag().unwrap_or("").to_string(),
                        last_modified: o.last_modified().map(|d| d.to_string()).unwrap_or_default(),
                    })
                    .collect();

                let common_prefixes = output
                    .common_prefixes()
                    .iter()
                    .map(|p| p.prefix().unwrap_or("").to_string())
                    .collect();

                Ok(ListResult {
                    objects,
                    is_truncated: output.is_truncated().unwrap_or(false),
                    next_continuation_token: output
                        .next_continuation_token()
                        .map(|s| s.to_string()),
                    common_prefixes,
                })
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

// AWS Signature v4 implementation
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub fn sha256_hex(data: &[u8]) -> String {
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

pub fn create_canonical_request(params: &SigningParams) -> String {
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

pub fn create_string_to_sign(params: &SigningParams) -> String {
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

pub fn derive_signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
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
        let host = format!("{}.s3.{}.amazonaws.com", self.bucket, self.region);
        self.get_signed_headers_with_host(access_key, secret_key, &host)
    }

    /// Returns signed headers with a custom host header (for MinIO/custom S3 endpoints)
    pub fn get_signed_headers_with_host(
        &self,
        access_key: &str,
        secret_key: &str,
        host: &str,
    ) -> std::collections::HashMap<String, String> {
        use std::collections::HashMap;

        // Generate current timestamp for AWS Signature V4
        let now = chrono::Utc::now();
        let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();

        // Build headers
        let mut headers = HashMap::new();
        headers.insert("host".to_string(), host.to_string());
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
            date: &date,
            datetime: &datetime,
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
