use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use yatagarasu::auth::authenticate_request;
use yatagarasu::config::{JwtConfig, TokenSource};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    role: String,
}

/// Benchmark JWT extraction from Authorization header
fn bench_jwt_extraction_bearer_header(c: &mut Criterion) {
    // Setup: Create valid JWT token
    let secret = "benchmark-secret-key-12345";
    let claims = Claims {
        sub: "user123".to_string(),
        exp: 9999999999, // Far future
        role: "admin".to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create token");

    // Setup: Create headers with Bearer token
    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {}", token));

    // Setup: Create JWT config
    let jwt_config = JwtConfig {
        enabled: true,
        secret: secret.to_string(),
        algorithm: "HS256".to_string(),
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![],
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_extraction_bearer_header", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

/// Benchmark JWT extraction from query parameter
fn bench_jwt_extraction_query_param(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims {
        sub: "user123".to_string(),
        exp: 9999999999,
        role: "admin".to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create token");

    let headers = HashMap::new();
    let mut query_params = HashMap::new();
    query_params.insert("token".to_string(), token);

    let jwt_config = JwtConfig {
        enabled: true,
        secret: secret.to_string(),
        algorithm: "HS256".to_string(),
        token_sources: vec![TokenSource {
            source_type: "query_parameter".to_string(),
            name: Some("token".to_string()),
            prefix: None,
        }],
        claims: vec![],
    };

    c.bench_function("jwt_extraction_query_param", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

/// Benchmark JWT extraction from custom header
fn bench_jwt_extraction_custom_header(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims {
        sub: "user123".to_string(),
        exp: 9999999999,
        role: "admin".to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create token");

    let mut headers = HashMap::new();
    headers.insert("x-auth-token".to_string(), token);

    let jwt_config = JwtConfig {
        enabled: true,
        secret: secret.to_string(),
        algorithm: "HS256".to_string(),
        token_sources: vec![TokenSource {
            source_type: "header".to_string(),
            name: Some("x-auth-token".to_string()),
            prefix: None,
        }],
        claims: vec![],
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_extraction_custom_header", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

/// Benchmark JWT validation with different algorithms
fn bench_jwt_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("jwt_algorithms");

    for algorithm in ["HS256", "HS384", "HS512"].iter() {
        let secret = "benchmark-secret-key-12345";
        let claims = Claims {
            sub: "user123".to_string(),
            exp: 9999999999,
            role: "admin".to_string(),
        };

        let alg = match *algorithm {
            "HS256" => Algorithm::HS256,
            "HS384" => Algorithm::HS384,
            "HS512" => Algorithm::HS512,
            _ => Algorithm::HS256,
        };

        let token = encode(
            &Header::new(alg),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("Failed to create token");

        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), format!("Bearer {}", token));

        let jwt_config = JwtConfig {
            enabled: true,
            secret: secret.to_string(),
            algorithm: algorithm.to_string(),
            token_sources: vec![TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            }],
            claims: vec![],
        };

        let query_params = HashMap::new();

        group.bench_with_input(
            BenchmarkId::from_parameter(algorithm),
            algorithm,
            |b, _alg| {
                b.iter(|| {
                    authenticate_request(
                        black_box(&headers),
                        black_box(&query_params),
                        black_box(&jwt_config),
                    )
                })
            },
        );
    }

    group.finish();
}

/// Benchmark JWT validation with claims verification
fn bench_jwt_with_claims_validation(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims {
        sub: "user123".to_string(),
        exp: 9999999999,
        role: "admin".to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create token");

    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {}", token));

    let jwt_config = JwtConfig {
        enabled: true,
        secret: secret.to_string(),
        algorithm: "HS256".to_string(),
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![yatagarasu::config::ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: serde_json::Value::String("admin".to_string()),
        }],
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_with_claims_validation", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

/// Benchmark JWT validation with multiple token sources (fallback logic)
fn bench_jwt_multiple_sources(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims {
        sub: "user123".to_string(),
        exp: 9999999999,
        role: "admin".to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create token");

    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {}", token));

    // Configure multiple token sources (will check Bearer header first, then others)
    let jwt_config = JwtConfig {
        enabled: true,
        secret: secret.to_string(),
        algorithm: "HS256".to_string(),
        token_sources: vec![
            TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            },
            TokenSource {
                source_type: "query_parameter".to_string(),
                name: Some("token".to_string()),
                prefix: None,
            },
            TokenSource {
                source_type: "header".to_string(),
                name: Some("x-auth-token".to_string()),
                prefix: None,
            },
        ],
        claims: vec![],
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_multiple_sources", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_jwt_extraction_bearer_header,
    bench_jwt_extraction_query_param,
    bench_jwt_extraction_custom_header,
    bench_jwt_algorithms,
    bench_jwt_with_claims_validation,
    bench_jwt_multiple_sources,
);
criterion_main!(benches);
