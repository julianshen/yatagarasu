use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;

use yatagarasu::s3::{
    build_get_object_request, build_head_object_request, create_canonical_request,
    create_string_to_sign, derive_signing_key, sha256_hex, sign_request, SigningParams,
};

/// Benchmark complete S3 signature generation (GET request)
fn bench_s3_signature_get_request(c: &mut Criterion) {
    let bucket = "test-bucket";
    let key = "test-file.txt";
    let region = "us-east-1";
    let access_key = "AKIAIOSFODNN7EXAMPLE";
    let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

    c.bench_function("s3_signature_get_request", |b| {
        b.iter(|| {
            let request =
                build_get_object_request(black_box(bucket), black_box(key), black_box(region));
            request.get_signed_headers(black_box(access_key), black_box(secret_key))
        })
    });
}

/// Benchmark complete S3 signature generation (HEAD request)
fn bench_s3_signature_head_request(c: &mut Criterion) {
    let bucket = "test-bucket";
    let key = "test-file.txt";
    let region = "us-east-1";
    let access_key = "AKIAIOSFODNN7EXAMPLE";
    let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

    c.bench_function("s3_signature_head_request", |b| {
        b.iter(|| {
            let request =
                build_head_object_request(black_box(bucket), black_box(key), black_box(region));
            request.get_signed_headers(black_box(access_key), black_box(secret_key))
        })
    });
}

/// Benchmark signature generation with different key lengths
fn bench_s3_signature_key_lengths(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_signature_key_lengths");

    let keys = vec![
        ("short_key", "file.txt"),
        ("medium_key", "path/to/file.txt"),
        ("long_key", "very/long/path/to/deeply/nested/file.txt"),
    ];

    for (name, key) in keys {
        group.bench_function(name, |b| {
            let bucket = "test-bucket";
            let region = "us-east-1";
            let access_key = "AKIAIOSFODNN7EXAMPLE";
            let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

            b.iter(|| {
                let request =
                    build_get_object_request(black_box(bucket), black_box(key), black_box(region));
                request.get_signed_headers(black_box(access_key), black_box(secret_key))
            })
        });
    }

    group.finish();
}

/// Benchmark individual signature components
fn bench_s3_signature_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_signature_components");

    // Setup common parameters
    let method = "GET";
    let uri = "/test-bucket/test-file.txt";
    let query_string = "";
    let mut headers = HashMap::new();
    headers.insert(
        "host".to_string(),
        "test-bucket.s3.us-east-1.amazonaws.com".to_string(),
    );
    headers.insert("x-amz-date".to_string(), "20231115T120000Z".to_string());
    headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));
    let payload = b"";
    let access_key = "AKIAIOSFODNN7EXAMPLE";
    let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    let region = "us-east-1";
    let service = "s3";
    let date = "20231115";
    let datetime = "20231115T120000Z";

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

    // Benchmark canonical request creation
    group.bench_function("canonical_request", |b| {
        b.iter(|| create_canonical_request(black_box(&params)))
    });

    // Benchmark string to sign creation
    group.bench_function("string_to_sign", |b| {
        b.iter(|| create_string_to_sign(black_box(&params)))
    });

    // Benchmark signing key derivation
    group.bench_function("derive_signing_key", |b| {
        b.iter(|| {
            derive_signing_key(
                black_box(secret_key),
                black_box(date),
                black_box(region),
                black_box(service),
            )
        })
    });

    // Benchmark complete signature generation
    group.bench_function("sign_request", |b| {
        b.iter(|| sign_request(black_box(&params)))
    });

    // Benchmark SHA256 hash
    group.bench_function("sha256_hex", |b| {
        b.iter(|| sha256_hex(black_box(b"test payload data")))
    });

    group.finish();
}

/// Benchmark signature generation with different bucket names
fn bench_s3_signature_bucket_names(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_signature_bucket_names");

    let buckets = vec![
        ("short_bucket", "bucket"),
        ("medium_bucket", "my-application-bucket"),
        ("long_bucket", "very-long-bucket-name-for-testing-purposes"),
    ];

    for (name, bucket) in buckets {
        group.bench_function(name, |b| {
            let key = "test-file.txt";
            let region = "us-east-1";
            let access_key = "AKIAIOSFODNN7EXAMPLE";
            let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

            b.iter(|| {
                let request =
                    build_get_object_request(black_box(bucket), black_box(key), black_box(region));
                request.get_signed_headers(black_box(access_key), black_box(secret_key))
            })
        });
    }

    group.finish();
}

/// Benchmark signature generation with different regions
fn bench_s3_signature_regions(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_signature_regions");

    let regions = vec![
        ("us_east_1", "us-east-1"),
        ("eu_west_1", "eu-west-1"),
        ("ap_southeast_1", "ap-southeast-1"),
    ];

    for (name, region) in regions {
        group.bench_function(name, |b| {
            let bucket = "test-bucket";
            let key = "test-file.txt";
            let access_key = "AKIAIOSFODNN7EXAMPLE";
            let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

            b.iter(|| {
                let request =
                    build_get_object_request(black_box(bucket), black_box(key), black_box(region));
                request.get_signed_headers(black_box(access_key), black_box(secret_key))
            })
        });
    }

    group.finish();
}

/// Benchmark signature generation with different payload sizes (for POST/PUT)
fn bench_s3_signature_payload_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("s3_signature_payload_sizes");

    let payload_sizes = vec![
        ("empty", vec![]),
        ("1kb", vec![0u8; 1024]),
        ("10kb", vec![0u8; 10 * 1024]),
        ("100kb", vec![0u8; 100 * 1024]),
    ];

    for (name, payload) in payload_sizes {
        group.bench_with_input(BenchmarkId::from_parameter(name), &payload, |b, payload| {
            let method = "PUT";
            let uri = "/test-bucket/test-file.txt";
            let query_string = "";
            let mut headers = HashMap::new();
            headers.insert(
                "host".to_string(),
                "test-bucket.s3.us-east-1.amazonaws.com".to_string(),
            );
            headers.insert("x-amz-date".to_string(), "20231115T120000Z".to_string());
            headers.insert(
                "x-amz-content-sha256".to_string(),
                sha256_hex(payload.as_slice()),
            );
            let access_key = "AKIAIOSFODNN7EXAMPLE";
            let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
            let region = "us-east-1";
            let service = "s3";
            let date = "20231115";
            let datetime = "20231115T120000Z";

            b.iter(|| {
                let params = SigningParams {
                    method,
                    uri,
                    query_string,
                    headers: &headers,
                    payload: black_box(payload.as_slice()),
                    access_key,
                    secret_key,
                    region,
                    service,
                    date,
                    datetime,
                };
                sign_request(black_box(&params))
            })
        });
    }

    group.finish();
}

/// Benchmark concurrent signature generation
fn bench_s3_signature_concurrent(c: &mut Criterion) {
    c.bench_function("s3_signature_concurrent_10", |b| {
        let bucket = "test-bucket";
        let region = "us-east-1";
        let access_key = "AKIAIOSFODNN7EXAMPLE";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";

        b.iter(|| {
            // Simulate 10 concurrent signature generations
            for i in 0..10 {
                let key = format!("file-{}.txt", i);
                let request =
                    build_get_object_request(black_box(bucket), black_box(&key), black_box(region));
                let _ = request.get_signed_headers(black_box(access_key), black_box(secret_key));
            }
        })
    });
}

/// Benchmark date formatting (ISO 8601)
/// Tests chrono datetime formatting as used in AWS Signature V4
fn bench_date_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("date_formatting");

    // Benchmark datetime format (YYYYMMDDTHHMMSSZ)
    group.bench_function("datetime_iso8601", |b| {
        b.iter(|| {
            let now = chrono::Utc::now();
            black_box(now.format("%Y%m%dT%H%M%SZ").to_string())
        })
    });

    // Benchmark date only format (YYYYMMDD)
    group.bench_function("date_only", |b| {
        b.iter(|| {
            let now = chrono::Utc::now();
            black_box(now.format("%Y%m%d").to_string())
        })
    });

    // Benchmark both formats together (as used in signing)
    group.bench_function("datetime_and_date", |b| {
        b.iter(|| {
            let now = chrono::Utc::now();
            let datetime = now.format("%Y%m%dT%H%M%SZ").to_string();
            let date = now.format("%Y%m%d").to_string();
            black_box((datetime, date))
        })
    });

    group.finish();
}

/// Benchmark header canonicalization with varying header counts
/// Tests the header sorting and formatting as used in canonical request creation
fn bench_header_canonicalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_canonicalization");

    // 3 headers (minimum typical S3 request)
    group.bench_function("3_headers", |b| {
        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "bucket.s3.us-east-1.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), "20231115T120000Z".to_string());
        headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));

        let params = SigningParams {
            method: "GET",
            uri: "/bucket/key.txt",
            query_string: "",
            headers: &headers,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231115",
            datetime: "20231115T120000Z",
        };

        b.iter(|| create_canonical_request(black_box(&params)))
    });

    // 5 headers (typical S3 request with extra headers)
    group.bench_function("5_headers", |b| {
        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "bucket.s3.us-east-1.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), "20231115T120000Z".to_string());
        headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));
        headers.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        let params = SigningParams {
            method: "GET",
            uri: "/bucket/key.txt",
            query_string: "",
            headers: &headers,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231115",
            datetime: "20231115T120000Z",
        };

        b.iter(|| create_canonical_request(black_box(&params)))
    });

    // 10 headers (complex request)
    group.bench_function("10_headers", |b| {
        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "bucket.s3.us-east-1.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), "20231115T120000Z".to_string());
        headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));
        headers.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());
        headers.insert("x-amz-meta-author".to_string(), "test-author".to_string());
        headers.insert("x-amz-meta-version".to_string(), "1.0.0".to_string());
        headers.insert("x-amz-acl".to_string(), "private".to_string());
        headers.insert("content-encoding".to_string(), "gzip".to_string());
        headers.insert("accept-encoding".to_string(), "gzip, deflate".to_string());

        let params = SigningParams {
            method: "PUT",
            uri: "/bucket/key.txt",
            query_string: "",
            headers: &headers,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231115",
            datetime: "20231115T120000Z",
        };

        b.iter(|| create_canonical_request(black_box(&params)))
    });

    // 15 headers (many custom metadata headers)
    group.bench_function("15_headers", |b| {
        let mut headers = HashMap::new();
        headers.insert(
            "host".to_string(),
            "bucket.s3.us-east-1.amazonaws.com".to_string(),
        );
        headers.insert("x-amz-date".to_string(), "20231115T120000Z".to_string());
        headers.insert("x-amz-content-sha256".to_string(), sha256_hex(b""));
        headers.insert(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        );
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());
        headers.insert("x-amz-meta-author".to_string(), "test-author".to_string());
        headers.insert("x-amz-meta-version".to_string(), "1.0.0".to_string());
        headers.insert("x-amz-meta-project".to_string(), "yatagarasu".to_string());
        headers.insert(
            "x-amz-meta-environment".to_string(),
            "production".to_string(),
        );
        headers.insert("x-amz-meta-region".to_string(), "us-east-1".to_string());
        headers.insert("x-amz-acl".to_string(), "private".to_string());
        headers.insert("content-encoding".to_string(), "gzip".to_string());
        headers.insert("accept-encoding".to_string(), "gzip, deflate".to_string());
        headers.insert("x-amz-storage-class".to_string(), "STANDARD".to_string());
        headers.insert(
            "x-amz-server-side-encryption".to_string(),
            "AES256".to_string(),
        );

        let params = SigningParams {
            method: "PUT",
            uri: "/bucket/key.txt",
            query_string: "",
            headers: &headers,
            payload: b"",
            access_key: "AKIAIOSFODNN7EXAMPLE",
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "s3",
            date: "20231115",
            datetime: "20231115T120000Z",
        };

        b.iter(|| create_canonical_request(black_box(&params)))
    });

    group.finish();
}

/// Benchmark HMAC-SHA256 computation in isolation
fn bench_hmac_sha256(c: &mut Criterion) {
    let mut group = c.benchmark_group("hmac_sha256");

    // Small key, small data (typical signing scenario)
    group.bench_function("small_key_small_data", |b| {
        let key = b"AWS4wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let data = b"20231115";
        b.iter(|| {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice(black_box(key)).unwrap();
            mac.update(black_box(data));
            black_box(mac.finalize().into_bytes().to_vec())
        })
    });

    // 32-byte derived key (as used after first HMAC in signing key derivation)
    group.bench_function("derived_key_region", |b| {
        let key = vec![0u8; 32]; // Simulates derived key
        let data = b"us-east-1";
        b.iter(|| {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            let mut mac = HmacSha256::new_from_slice(black_box(&key)).unwrap();
            mac.update(black_box(data));
            black_box(mac.finalize().into_bytes().to_vec())
        })
    });

    // Full signing key derivation chain (4 HMAC operations)
    group.bench_function("full_signing_key_derivation", |b| {
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let date = "20231115";
        let region = "us-east-1";
        let service = "s3";
        b.iter(|| {
            derive_signing_key(
                black_box(secret_key),
                black_box(date),
                black_box(region),
                black_box(service),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_s3_signature_get_request,
    bench_s3_signature_head_request,
    bench_s3_signature_key_lengths,
    bench_s3_signature_components,
    bench_s3_signature_bucket_names,
    bench_s3_signature_regions,
    bench_s3_signature_payload_sizes,
    bench_s3_signature_concurrent,
    bench_date_formatting,
    bench_header_canonicalization,
    bench_hmac_sha256,
);
criterion_main!(benches);
