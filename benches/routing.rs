use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use yatagarasu::config::{BucketConfig, Config, S3Config, ServerConfig};
use yatagarasu::router::Router;

/// Benchmark routing with single bucket
fn bench_routing_single_bucket(c: &mut Criterion) {
    // Setup: Create config with single bucket
    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout: 30,
            max_concurrent_requests: 1000,
            rate_limit: None,
            security_limits: Default::default(),
        },
        buckets: vec![BucketConfig {
            name: "test-bucket".to_string(),
            path_prefix: "/test".to_string(),
            s3: S3Config {
                endpoint: Some("https://s3.amazonaws.com".to_string()),
                region: "us-east-1".to_string(),
                bucket: "test-bucket".to_string(),
                access_key: "test-key".to_string(),
                secret_key: "test-secret".to_string(),
                timeout: 20,
                connection_pool_size: 50,
                rate_limit: None,
                circuit_breaker: None,
                retry: None,
                replicas: None,
            },
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
        }],
        jwt: None,
        cache: None,
        audit_log: None,
        observability: Default::default(),
        generation: 0,
    };

    let router = Router::new(config.buckets);

    c.bench_function("routing_single_bucket_match", |b| {
        b.iter(|| router.route(black_box("/test/file.txt")))
    });
}

/// Benchmark routing with multiple buckets (needs to check multiple prefixes)
fn bench_routing_multiple_buckets(c: &mut Criterion) {
    // Setup: Create config with 10 buckets
    let buckets: Vec<BucketConfig> = (0..10)
        .map(|i| BucketConfig {
            name: format!("bucket-{}", i),
            path_prefix: format!("/bucket-{}", i),
            s3: S3Config {
                endpoint: Some("https://s3.amazonaws.com".to_string()),
                region: "us-east-1".to_string(),
                bucket: format!("bucket-{}", i),
                access_key: "test-key".to_string(),
                secret_key: "test-secret".to_string(),
                timeout: 20,
                connection_pool_size: 50,
                rate_limit: None,
                circuit_breaker: None,
                retry: None,
                replicas: None,
            },
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
        })
        .collect();

    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout: 30,
            max_concurrent_requests: 1000,
            rate_limit: None,
            security_limits: Default::default(),
        },
        buckets,
        jwt: None,
        cache: None,
        audit_log: None,
        observability: Default::default(),
        generation: 0,
    };

    let router = Router::new(config.buckets);

    let mut group = c.benchmark_group("routing_multiple_buckets");

    // Test routing to first bucket (best case)
    group.bench_function("first_bucket", |b| {
        b.iter(|| router.route(black_box("/bucket-0/file.txt")))
    });

    // Test routing to middle bucket
    group.bench_function("middle_bucket", |b| {
        b.iter(|| router.route(black_box("/bucket-5/file.txt")))
    });

    // Test routing to last bucket (worst case)
    group.bench_function("last_bucket", |b| {
        b.iter(|| router.route(black_box("/bucket-9/file.txt")))
    });

    // Test routing with no match (checks all buckets, returns None)
    group.bench_function("no_match", |b| {
        b.iter(|| router.route(black_box("/nonexistent/file.txt")))
    });

    group.finish();
}

/// Benchmark routing with different path lengths
fn bench_routing_path_lengths(c: &mut Criterion) {
    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout: 30,
            max_concurrent_requests: 1000,
            rate_limit: None,
            security_limits: Default::default(),
        },
        buckets: vec![BucketConfig {
            name: "test-bucket".to_string(),
            path_prefix: "/test".to_string(),
            s3: S3Config {
                endpoint: Some("https://s3.amazonaws.com".to_string()),
                region: "us-east-1".to_string(),
                bucket: "test-bucket".to_string(),
                access_key: "test-key".to_string(),
                secret_key: "test-secret".to_string(),
                timeout: 20,
                connection_pool_size: 50,
                rate_limit: None,
                circuit_breaker: None,
                retry: None,
                replicas: None,
            },
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
        }],
        jwt: None,
        cache: None,
        audit_log: None,
        observability: Default::default(),
        generation: 0,
    };

    let router = Router::new(config.buckets);

    let mut group = c.benchmark_group("routing_path_lengths");

    // Short path
    group.bench_function("short_path", |b| {
        b.iter(|| router.route(black_box("/test/a.txt")))
    });

    // Medium path
    group.bench_function("medium_path", |b| {
        b.iter(|| router.route(black_box("/test/dir/subdir/file.txt")))
    });

    // Long path
    group.bench_function("long_path", |b| {
        b.iter(|| {
            router.route(black_box(
                "/test/very/long/path/with/many/segments/to/the/file.txt",
            ))
        })
    });

    group.finish();
}

/// Benchmark S3 key extraction
fn bench_s3_key_extraction(c: &mut Criterion) {
    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout: 30,
            max_concurrent_requests: 1000,
            rate_limit: None,
            security_limits: Default::default(),
        },
        buckets: vec![BucketConfig {
            name: "test-bucket".to_string(),
            path_prefix: "/test".to_string(),
            s3: S3Config {
                endpoint: Some("https://s3.amazonaws.com".to_string()),
                region: "us-east-1".to_string(),
                bucket: "test-bucket".to_string(),
                access_key: "test-key".to_string(),
                secret_key: "test-secret".to_string(),
                timeout: 20,
                connection_pool_size: 50,
                rate_limit: None,
                circuit_breaker: None,
                retry: None,
                replicas: None,
            },
            auth: None,
            cache: None,
            authorization: None,
            ip_filter: Default::default(),
        }],
        jwt: None,
        cache: None,
        audit_log: None,
        observability: Default::default(),
        generation: 0,
    };

    let router = Router::new(config.buckets);

    let mut group = c.benchmark_group("s3_key_extraction");

    // Short key
    group.bench_function("short_key", |b| {
        b.iter(|| router.extract_s3_key(black_box("/test/file.txt")))
    });

    // Medium key with subdirectories
    group.bench_function("medium_key", |b| {
        b.iter(|| router.extract_s3_key(black_box("/test/dir/subdir/file.txt")))
    });

    // Long key with many path segments
    group.bench_function("long_key", |b| {
        b.iter(|| {
            router.extract_s3_key(black_box(
                "/test/path/to/deeply/nested/directory/structure/file.txt",
            ))
        })
    });

    group.finish();
}

/// Benchmark routing with longest prefix matching
fn bench_routing_longest_prefix(c: &mut Criterion) {
    // Setup: Create config with overlapping prefixes
    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout: 30,
            max_concurrent_requests: 1000,
            rate_limit: None,
            security_limits: Default::default(),
        },
        buckets: vec![
            BucketConfig {
                name: "bucket-short".to_string(),
                path_prefix: "/api".to_string(),
                s3: S3Config {
                    endpoint: Some("https://s3.amazonaws.com".to_string()),
                    region: "us-east-1".to_string(),
                    bucket: "bucket-short".to_string(),
                    access_key: "test-key".to_string(),
                    secret_key: "test-secret".to_string(),
                    timeout: 20,
                    connection_pool_size: 50,
                    rate_limit: None,
                    circuit_breaker: None,
                    retry: None,
                    replicas: None,
                },
                auth: None,
                cache: None,
                authorization: None,
                ip_filter: Default::default(),
            },
            BucketConfig {
                name: "bucket-medium".to_string(),
                path_prefix: "/api/v1".to_string(),
                s3: S3Config {
                    endpoint: Some("https://s3.amazonaws.com".to_string()),
                    region: "us-east-1".to_string(),
                    bucket: "bucket-medium".to_string(),
                    access_key: "test-key".to_string(),
                    secret_key: "test-secret".to_string(),
                    timeout: 20,
                    connection_pool_size: 50,
                    rate_limit: None,
                    circuit_breaker: None,
                    retry: None,
                    replicas: None,
                },
                auth: None,
                cache: None,
                authorization: None,
                ip_filter: Default::default(),
            },
            BucketConfig {
                name: "bucket-long".to_string(),
                path_prefix: "/api/v1/data".to_string(),
                s3: S3Config {
                    endpoint: Some("https://s3.amazonaws.com".to_string()),
                    region: "us-east-1".to_string(),
                    bucket: "bucket-long".to_string(),
                    access_key: "test-key".to_string(),
                    secret_key: "test-secret".to_string(),
                    timeout: 20,
                    connection_pool_size: 50,
                    rate_limit: None,
                    circuit_breaker: None,
                    retry: None,
                    replicas: None,
                },
                auth: None,
                cache: None,
                authorization: None,
                ip_filter: Default::default(),
            },
        ],
        jwt: None,
        cache: None,
        audit_log: None,
        observability: Default::default(),
        generation: 0,
    };

    let router = Router::new(config.buckets);

    let mut group = c.benchmark_group("routing_longest_prefix");

    // Should match bucket-short (/api)
    group.bench_function("match_short_prefix", |b| {
        b.iter(|| router.route(black_box("/api/file.txt")))
    });

    // Should match bucket-medium (/api/v1) - needs to check if longer prefix matches
    group.bench_function("match_medium_prefix", |b| {
        b.iter(|| router.route(black_box("/api/v1/file.txt")))
    });

    // Should match bucket-long (/api/v1/data) - needs to check all three
    group.bench_function("match_long_prefix", |b| {
        b.iter(|| router.route(black_box("/api/v1/data/file.txt")))
    });

    group.finish();
}

/// Benchmark routing with many buckets (stress test)
fn bench_routing_many_buckets(c: &mut Criterion) {
    let bucket_counts = [5, 10, 50, 100];

    let mut group = c.benchmark_group("routing_many_buckets");

    for &count in &bucket_counts {
        let buckets: Vec<BucketConfig> = (0..count)
            .map(|i| BucketConfig {
                name: format!("bucket-{}", i),
                path_prefix: format!("/bucket-{}", i),
                s3: S3Config {
                    endpoint: Some("https://s3.amazonaws.com".to_string()),
                    region: "us-east-1".to_string(),
                    bucket: format!("bucket-{}", i),
                    access_key: "test-key".to_string(),
                    secret_key: "test-secret".to_string(),
                    timeout: 20,
                    connection_pool_size: 50,
                    rate_limit: None,
                    circuit_breaker: None,
                    retry: None,
                    replicas: None,
                },
                auth: None,
                cache: None,
                authorization: None,
                ip_filter: Default::default(),
            })
            .collect();

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1".to_string(),
                port: 8080,
                request_timeout: 30,
                max_concurrent_requests: 1000,
                rate_limit: None,
                security_limits: Default::default(),
            },
            buckets,
            jwt: None,
            cache: None,
            audit_log: None,
            observability: Default::default(),
            generation: 0,
        };

        let router = Router::new(config.buckets);

        // Benchmark routing to the last bucket (worst case - checks all buckets)
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_buckets_worst_case", count)),
            &count,
            |b, _| {
                let path = format!("/bucket-{}/file.txt", count - 1);
                b.iter(|| router.route(black_box(&path)))
            },
        );

        // Benchmark routing with no match (checks all buckets, returns None)
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_buckets_no_match", count)),
            &count,
            |b, _| b.iter(|| router.route(black_box("/nonexistent/file.txt"))),
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_routing_single_bucket,
    bench_routing_multiple_buckets,
    bench_routing_path_lengths,
    bench_s3_key_extraction,
    bench_routing_longest_prefix,
    bench_routing_many_buckets,
);
criterion_main!(benches);
