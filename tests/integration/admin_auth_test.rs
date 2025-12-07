// Admin Authentication Tests (Phase 65.1)
// Tests for admin JWT authentication on cache management endpoints

use serde_json::json;

/// Phase 65.1: Test admin claim verification logic
/// This is a unit-level test for the verify_admin_claims function
#[test]
fn test_admin_claims_verification_with_matching_claims() {
    use yatagarasu::auth::verify_admin_claims;
    use yatagarasu::auth::Claims;
    use yatagarasu::config::ClaimRule;

    // Create claims with admin role
    let mut custom = serde_json::Map::new();
    custom.insert("role".to_string(), json!("admin"));
    custom.insert("department".to_string(), json!("ops"));

    let claims = Claims {
        sub: Some("admin-user".to_string()),
        exp: Some(u64::MAX),
        iat: None,
        nbf: None,
        iss: None,
        custom,
    };

    // Admin rule: role == "admin"
    let admin_rules = vec![ClaimRule {
        claim: "role".to_string(),
        operator: "equals".to_string(),
        value: json!("admin"),
    }];

    // Should pass - admin claim matches
    assert!(
        verify_admin_claims(&claims, &admin_rules),
        "Admin claims should verify when role=admin"
    );
}

#[test]
fn test_admin_claims_verification_without_matching_claims() {
    use yatagarasu::auth::verify_admin_claims;
    use yatagarasu::auth::Claims;
    use yatagarasu::config::ClaimRule;

    // Create claims with regular user role (not admin)
    let mut custom = serde_json::Map::new();
    custom.insert("role".to_string(), json!("user"));

    let claims = Claims {
        sub: Some("regular-user".to_string()),
        exp: Some(u64::MAX),
        iat: None,
        nbf: None,
        iss: None,
        custom,
    };

    // Admin rule: role == "admin"
    let admin_rules = vec![ClaimRule {
        claim: "role".to_string(),
        operator: "equals".to_string(),
        value: json!("admin"),
    }];

    // Should fail - user role doesn't match admin requirement
    assert!(
        !verify_admin_claims(&claims, &admin_rules),
        "Admin claims should NOT verify when role=user"
    );
}

#[test]
fn test_admin_claims_verification_with_empty_rules() {
    use yatagarasu::auth::verify_admin_claims;
    use yatagarasu::auth::Claims;
    use yatagarasu::config::ClaimRule;

    // Create claims with any role
    let mut custom = serde_json::Map::new();
    custom.insert("role".to_string(), json!("guest"));

    let claims = Claims {
        sub: Some("guest-user".to_string()),
        exp: Some(u64::MAX),
        iat: None,
        nbf: None,
        iss: None,
        custom,
    };

    // No admin rules configured (empty array)
    let admin_rules: Vec<ClaimRule> = vec![];

    // Should pass - no admin restrictions when rules are empty
    assert!(
        verify_admin_claims(&claims, &admin_rules),
        "Admin claims should verify when no rules are configured"
    );
}

#[test]
fn test_admin_claims_verification_with_multiple_rules() {
    use yatagarasu::auth::verify_admin_claims;
    use yatagarasu::auth::Claims;
    use yatagarasu::config::ClaimRule;

    // Create claims with admin role AND ops department
    let mut custom = serde_json::Map::new();
    custom.insert("role".to_string(), json!("admin"));
    custom.insert("department".to_string(), json!("ops"));

    let claims = Claims {
        sub: Some("ops-admin".to_string()),
        exp: Some(u64::MAX),
        iat: None,
        nbf: None,
        iss: None,
        custom,
    };

    // Multiple admin rules: role == "admin" AND department == "ops"
    let admin_rules = vec![
        ClaimRule {
            claim: "role".to_string(),
            operator: "equals".to_string(),
            value: json!("admin"),
        },
        ClaimRule {
            claim: "department".to_string(),
            operator: "equals".to_string(),
            value: json!("ops"),
        },
    ];

    // Should pass - both claims match
    assert!(
        verify_admin_claims(&claims, &admin_rules),
        "Admin claims should verify when all rules match"
    );

    // Test partial match (should fail - all rules must match)
    let mut custom_partial = serde_json::Map::new();
    custom_partial.insert("role".to_string(), json!("admin"));
    custom_partial.insert("department".to_string(), json!("engineering")); // Wrong department

    let claims_partial = Claims {
        sub: Some("eng-admin".to_string()),
        exp: Some(u64::MAX),
        iat: None,
        nbf: None,
        iss: None,
        custom: custom_partial,
    };

    assert!(
        !verify_admin_claims(&claims_partial, &admin_rules),
        "Admin claims should NOT verify when only some rules match"
    );
}

#[test]
fn test_admin_claims_verification_with_missing_claim() {
    use yatagarasu::auth::verify_admin_claims;
    use yatagarasu::auth::Claims;
    use yatagarasu::config::ClaimRule;

    // Create claims WITHOUT the required admin role
    let custom = serde_json::Map::new(); // Empty custom claims

    let claims = Claims {
        sub: Some("user-without-role".to_string()),
        exp: Some(u64::MAX),
        iat: None,
        nbf: None,
        iss: None,
        custom,
    };

    // Admin rule: role == "admin"
    let admin_rules = vec![ClaimRule {
        claim: "role".to_string(),
        operator: "equals".to_string(),
        value: json!("admin"),
    }];

    // Should fail - claim is missing entirely
    assert!(
        !verify_admin_claims(&claims, &admin_rules),
        "Admin claims should NOT verify when required claim is missing"
    );
}

/// Test config parsing for admin_claims field
#[test]
fn test_config_admin_claims_parsing() {
    use yatagarasu::config::Config;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

jwt:
  enabled: true
  secret: "test-secret-key-for-jwt-signing"
  algorithm: HS256
  claims:
    - claim: "scope"
      operator: "equals"
      value: "read"
  admin_claims:
    - claim: "role"
      operator: "equals"
      value: "admin"
    - claim: "department"
      operator: "equals"
      value: "ops"

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      endpoint: "http://localhost:9000"
      region: "us-east-1"
      bucket: "test"
      credentials:
        access_key: "test"
        secret_key: "test"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse config");

    assert!(config.jwt.is_some(), "JWT config should be present");
    let jwt_config = config.jwt.unwrap();

    assert_eq!(
        jwt_config.admin_claims.len(),
        2,
        "Should have 2 admin claim rules"
    );

    // First rule: role == "admin"
    let rule1 = &jwt_config.admin_claims[0];
    assert_eq!(rule1.claim, "role");
    assert_eq!(rule1.operator, "equals");
    assert_eq!(rule1.value, json!("admin"));

    // Second rule: department == "ops"
    let rule2 = &jwt_config.admin_claims[1];
    assert_eq!(rule2.claim, "department");
    assert_eq!(rule2.operator, "equals");
    assert_eq!(rule2.value, json!("ops"));
}

/// Test config with empty admin_claims (defaults to empty vec)
#[test]
fn test_config_admin_claims_defaults_to_empty() {
    use yatagarasu::config::Config;

    let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

jwt:
  enabled: true
  secret: "test-secret-key-for-jwt-signing"
  algorithm: HS256

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      endpoint: "http://localhost:9000"
      region: "us-east-1"
      bucket: "test"
      credentials:
        access_key: "test"
        secret_key: "test"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse config");

    assert!(config.jwt.is_some(), "JWT config should be present");
    let jwt_config = config.jwt.unwrap();

    // admin_claims should default to empty vec (no admin restrictions)
    assert!(
        jwt_config.admin_claims.is_empty(),
        "admin_claims should default to empty when not specified"
    );
}

/// Integration test description for E2E testing (requires running server)
/// These tests would require a running proxy instance with proper JWT configuration
///
/// Test scenarios:
/// 1. POST /admin/cache/purge with valid JWT + admin claim → 200 OK
/// 2. POST /admin/cache/purge with valid JWT but no admin claim → 403 Forbidden
/// 3. POST /admin/cache/purge with invalid/missing JWT → 401 Unauthorized
/// 4. POST /admin/cache/purge/:bucket/:path with valid JWT + admin claim → 200 OK
/// 5. GET /admin/cache/stats with valid JWT (no admin required) → 200 OK
#[test]
fn test_admin_auth_e2e_scenario_documentation() {
    // This test documents the expected behavior for E2E testing
    // Actual E2E tests would need a running proxy with MinIO

    let expected_scenarios = vec![
        ("POST /admin/cache/purge", "valid JWT + admin claim", 200),
        ("POST /admin/cache/purge", "valid JWT, no admin claim", 403),
        ("POST /admin/cache/purge", "invalid/missing JWT", 401),
        (
            "POST /admin/cache/purge/bucket/path",
            "valid JWT + admin claim",
            200,
        ),
        (
            "POST /admin/cache/purge/bucket/path",
            "valid JWT, no admin claim",
            403,
        ),
        (
            "GET /admin/cache/stats",
            "valid JWT (no admin required)",
            200,
        ),
        (
            "GET /admin/cache/stats/bucket",
            "valid JWT (no admin required)",
            200,
        ),
    ];

    assert_eq!(
        expected_scenarios.len(),
        7,
        "Should have 7 documented scenarios"
    );
}
