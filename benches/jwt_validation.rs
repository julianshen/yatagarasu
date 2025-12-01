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

/// Claims struct with 5 verifiable fields for benchmark
#[derive(Debug, Serialize, Deserialize)]
struct Claims5 {
    sub: String,
    exp: usize,
    role: String,
    department: String,
    level: String,
    region: String,
    active: bool,
}

/// Claims struct with 10 verifiable fields for benchmark
#[derive(Debug, Serialize, Deserialize)]
struct Claims10 {
    sub: String,
    exp: usize,
    role: String,
    department: String,
    level: String,
    region: String,
    active: bool,
    tier: String,
    team: String,
    project: String,
    clearance: String,
}

/// Nested profile object for benchmark
#[derive(Debug, Serialize, Deserialize)]
struct NestedProfile {
    name: String,
    email: String,
    department: String,
}

/// Nested permissions object for benchmark
#[derive(Debug, Serialize, Deserialize)]
struct NestedPermissions {
    roles: Vec<String>,
    level: u8,
    admin: bool,
}

/// Claims struct with nested objects for benchmark
#[derive(Debug, Serialize, Deserialize)]
struct ClaimsNested {
    sub: String,
    exp: usize,
    profile: NestedProfile,
    permissions: NestedPermissions,
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "query_parameter".to_string(),
            name: Some("token".to_string()),
            prefix: None,
        }],
        claims: vec![],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "header".to_string(),
            name: Some("x-auth-token".to_string()),
            prefix: None,
        }],
        claims: vec![],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
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
            rsa_public_key_path: None,
            ecdsa_public_key_path: None,
            token_sources: vec![TokenSource {
                source_type: "bearer".to_string(),
                name: None,
                prefix: None,
            }],
            claims: vec![],
            keys: vec![],
            jwks_url: None,
            jwks_refresh_interval_secs: None,
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
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
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
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

/// Benchmark JWT validation with 5 claims verification
fn bench_jwt_5_claims_validation(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims5 {
        sub: "user123".to_string(),
        exp: 9999999999,
        role: "admin".to_string(),
        department: "engineering".to_string(),
        level: "senior".to_string(),
        region: "us-west".to_string(),
        active: true,
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![
            yatagarasu::config::ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("admin".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "department".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("engineering".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("senior".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "region".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("us-west".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(true),
            },
        ],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_5_claims_validation", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

/// Benchmark JWT validation with 10 claims verification
fn bench_jwt_10_claims_validation(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims10 {
        sub: "user123".to_string(),
        exp: 9999999999,
        role: "admin".to_string(),
        department: "engineering".to_string(),
        level: "senior".to_string(),
        region: "us-west".to_string(),
        active: true,
        tier: "platinum".to_string(),
        team: "platform".to_string(),
        project: "yatagarasu".to_string(),
        clearance: "top-secret".to_string(),
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![
            yatagarasu::config::ClaimRule {
                claim: "role".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("admin".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "department".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("engineering".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "level".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("senior".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "region".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("us-west".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "active".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::Bool(true),
            },
            yatagarasu::config::ClaimRule {
                claim: "tier".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("platinum".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "team".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("platform".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "project".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("yatagarasu".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "clearance".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("top-secret".to_string()),
            },
            yatagarasu::config::ClaimRule {
                claim: "sub".to_string(),
                operator: "equals".to_string(),
                value: serde_json::Value::String("user123".to_string()),
            },
        ],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_10_claims_validation", |b| {
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
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
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
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

/// Benchmark JWT validation with nested objects in claims
fn bench_jwt_nested_claims(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = ClaimsNested {
        sub: "user123".to_string(),
        exp: 9999999999,
        profile: NestedProfile {
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            department: "engineering".to_string(),
        },
        permissions: NestedPermissions {
            roles: vec![
                "admin".to_string(),
                "developer".to_string(),
                "reviewer".to_string(),
            ],
            level: 5,
            admin: true,
        },
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .expect("Failed to create token");

    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {}", token));

    // Note: The current claim validation doesn't support nested object paths,
    // so we benchmark the parsing overhead without claim rules
    let jwt_config = JwtConfig {
        enabled: true,
        secret: secret.to_string(),
        algorithm: "HS256".to_string(),
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_nested_claims", |b| {
        b.iter(|| {
            authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            )
        })
    });
}

/// Benchmark expired token detection
/// This measures how quickly the system detects and rejects an expired JWT token
fn bench_jwt_expired_token(c: &mut Criterion) {
    let secret = "benchmark-secret-key-12345";
    let claims = Claims {
        sub: "user123".to_string(),
        exp: 1, // Expired: January 1, 1970 00:00:01 UTC
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
        rsa_public_key_path: None,
        ecdsa_public_key_path: None,
        token_sources: vec![TokenSource {
            source_type: "bearer".to_string(),
            name: None,
            prefix: None,
        }],
        claims: vec![],
        keys: vec![],
        jwks_url: None,
        jwks_refresh_interval_secs: None,
    };

    let query_params = HashMap::new();

    c.bench_function("jwt_expired_token", |b| {
        b.iter(|| {
            // This should return an error due to expired token
            let result = authenticate_request(
                black_box(&headers),
                black_box(&query_params),
                black_box(&jwt_config),
            );
            // We expect this to fail, but we still benchmark the detection time
            black_box(result)
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
    bench_jwt_5_claims_validation,
    bench_jwt_10_claims_validation,
    bench_jwt_multiple_sources,
    bench_jwt_nested_claims,
    bench_jwt_expired_token,
);
criterion_main!(benches);
