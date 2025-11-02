// Unit test for HEAD request support in proxy
// Tests that HEAD requests are properly handled and signed with correct method

use yatagarasu::s3::{build_get_object_request, build_head_object_request};

#[test]
fn test_build_head_request_uses_head_method() {
    // Test: build_head_object_request creates request with HEAD method
    let request = build_head_object_request("test-bucket", "path/to/file.txt", "us-east-1");

    assert_eq!(
        request.method, "HEAD",
        "HEAD request should have method=HEAD"
    );
    assert_eq!(request.bucket, "test-bucket");
    assert_eq!(request.key, "path/to/file.txt");
    assert_eq!(request.region, "us-east-1");
}

#[test]
fn test_build_get_request_uses_get_method() {
    // Test: build_get_object_request creates request with GET method
    let request = build_get_object_request("test-bucket", "path/to/file.txt", "us-east-1");

    assert_eq!(request.method, "GET", "GET request should have method=GET");
    assert_eq!(request.bucket, "test-bucket");
    assert_eq!(request.key, "path/to/file.txt");
    assert_eq!(request.region, "us-east-1");
}

#[test]
fn test_head_and_get_requests_generate_different_signatures() {
    // Test: HEAD and GET requests should generate different AWS SigV4 signatures
    // because the HTTP method is part of the canonical request

    let access_key = "AKIAIOSFODNN7EXAMPLE";
    let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

    let get_request = build_get_object_request("test-bucket", "test.txt", "us-east-1");
    let head_request = build_head_object_request("test-bucket", "test.txt", "us-east-1");

    let get_headers = get_request.get_signed_headers(access_key, secret_key);
    let head_headers = head_request.get_signed_headers(access_key, secret_key);

    // Extract Authorization headers
    let get_auth = get_headers
        .iter()
        .find(|(k, _)| k.as_str() == "authorization")
        .map(|(_, v)| v);
    let head_auth = head_headers
        .iter()
        .find(|(k, _)| k.as_str() == "authorization")
        .map(|(_, v)| v);

    assert!(
        get_auth.is_some(),
        "GET request should have Authorization header"
    );
    assert!(
        head_auth.is_some(),
        "HEAD request should have Authorization header"
    );

    // The signatures should be different because the method is different
    assert_ne!(
        get_auth.unwrap(),
        head_auth.unwrap(),
        "GET and HEAD requests should have different Authorization signatures"
    );
}
