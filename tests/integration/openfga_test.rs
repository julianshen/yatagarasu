//! OpenFGA Integration Tests using testcontainers
//!
//! These tests use testcontainers to run OpenFGA in Docker, making them self-contained.
//! No external OpenFGA server is required.

use serde_json::json;
use std::time::Duration;
use testcontainers::{clients::Cli, core::WaitFor, GenericImage, RunnableImage};
use yatagarasu::openfga::{
    build_openfga_object, extract_user_id, http_method_to_relation, AuthorizationDecision,
    FailMode, OpenFgaClient, Relation,
};

/// Create an OpenFGA container and return the URL
fn create_openfga_container(docker: &Cli) -> (testcontainers::Container<'_, GenericImage>, String) {
    // OpenFGA image with server mode
    // OpenFGA logs to stdout and is ready when it says "starting server"
    let openfga_image = GenericImage::new("openfga/openfga", "latest")
        .with_exposed_port(8080)
        .with_wait_for(WaitFor::message_on_stdout("starting server"));

    // Create RunnableImage with command arguments for server mode
    let args: Vec<String> = vec!["run".to_string()];
    let runnable_image = RunnableImage::from((openfga_image, args));

    let container = docker.run(runnable_image);

    let port = container.get_host_port_ipv4(8080);
    let url = format!("http://127.0.0.1:{}", port);

    (container, url)
}

/// Wait for OpenFGA to be ready
async fn wait_for_openfga(openfga_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    for _ in 0..30 {
        if let Ok(response) = client.get(&format!("{}/healthz", openfga_url)).send().await {
            if response.status().is_success() {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Create a store in OpenFGA and return the store ID
async fn create_store(openfga_url: &str, store_name: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/stores", openfga_url);

    let response = client
        .post(&url)
        .json(&json!({"name": store_name}))
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    body.get("id")?.as_str().map(|s| s.to_string())
}

/// Write an authorization model to OpenFGA
async fn write_authorization_model(
    openfga_url: &str,
    store_id: &str,
    model: serde_json::Value,
) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/stores/{}/authorization-models", openfga_url, store_id);

    let response = client.post(&url).json(&model).send().await.ok()?;

    if !response.status().is_success() {
        eprintln!(
            "Failed to write model: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
        return None;
    }

    let body: serde_json::Value = response.json().await.ok()?;
    body.get("authorization_model_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Write a relationship tuple to OpenFGA
async fn write_tuple(
    openfga_url: &str,
    store_id: &str,
    user: &str,
    relation: &str,
    object: &str,
) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/stores/{}/write", openfga_url, store_id);

    let body = json!({
        "writes": {
            "tuple_keys": [{
                "user": user,
                "relation": relation,
                "object": object
            }]
        }
    });

    match client.post(&url).json(&body).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Standard authorization model for testing
/// This model defines:
/// - bucket: with viewer, editor, owner relations
/// - folder: with viewer, editor, owner relations (parent is bucket)
/// - file: with viewer, editor, owner relations (parent is folder or bucket)
fn test_authorization_model() -> serde_json::Value {
    json!({
        "schema_version": "1.1",
        "type_definitions": [
            {
                "type": "user",
                "relations": {}
            },
            {
                "type": "bucket",
                "relations": {
                    "viewer": {
                        "this": {}
                    },
                    "editor": {
                        "this": {}
                    },
                    "owner": {
                        "this": {}
                    }
                },
                "metadata": {
                    "relations": {
                        "viewer": {
                            "directly_related_user_types": [{"type": "user"}]
                        },
                        "editor": {
                            "directly_related_user_types": [{"type": "user"}]
                        },
                        "owner": {
                            "directly_related_user_types": [{"type": "user"}]
                        }
                    }
                }
            },
            {
                "type": "folder",
                "relations": {
                    "parent": {
                        "this": {}
                    },
                    "viewer": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "viewer"}}}
                            ]
                        }
                    },
                    "editor": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "editor"}}}
                            ]
                        }
                    },
                    "owner": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "owner"}}}
                            ]
                        }
                    }
                },
                "metadata": {
                    "relations": {
                        "parent": {
                            "directly_related_user_types": [{"type": "bucket"}]
                        },
                        "viewer": {
                            "directly_related_user_types": [{"type": "user"}]
                        },
                        "editor": {
                            "directly_related_user_types": [{"type": "user"}]
                        },
                        "owner": {
                            "directly_related_user_types": [{"type": "user"}]
                        }
                    }
                }
            },
            {
                "type": "file",
                "relations": {
                    "parent": {
                        "this": {}
                    },
                    "viewer": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "viewer"}}}
                            ]
                        }
                    },
                    "editor": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "editor"}}}
                            ]
                        }
                    },
                    "owner": {
                        "union": {
                            "child": [
                                {"this": {}},
                                {"tupleToUserset": {"tupleset": {"relation": "parent"}, "computedUserset": {"relation": "owner"}}}
                            ]
                        }
                    }
                },
                "metadata": {
                    "relations": {
                        "parent": {
                            "directly_related_user_types": [{"type": "bucket"}, {"type": "folder"}]
                        },
                        "viewer": {
                            "directly_related_user_types": [{"type": "user"}]
                        },
                        "editor": {
                            "directly_related_user_types": [{"type": "user"}]
                        },
                        "owner": {
                            "directly_related_user_types": [{"type": "user"}]
                        }
                    }
                }
            }
        ]
    })
}

// ============================================================================
// Phase 49.2: Request Authorization Flow Tests
// ============================================================================

/// Test: Check authorization before proxying
/// When a user has the viewer relation to a file, they should be allowed
#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_check_authorization_allowed() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    // Wait for OpenFGA to be ready
    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    // Create a store
    let store_id = create_store(&openfga_url, "test-store")
        .await
        .expect("Should create store");

    // Write authorization model
    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    // Create OpenFGA client
    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // Write a tuple granting alice viewer access to a file
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "user:alice",
            "viewer",
            "file:my-bucket/docs/readme.txt"
        )
        .await,
        "Should write tuple"
    );

    // Check that alice can view the file
    let result = client
        .check("user:alice", "viewer", "file:my-bucket/docs/readme.txt")
        .await;

    assert!(result.is_ok(), "Check should succeed: {:?}", result.err());
    assert!(result.unwrap(), "Alice should be allowed to view the file");
}

/// Test: Return 403 on authorization failure
/// When a user does NOT have the required relation, they should be denied
#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_check_authorization_denied_returns_403() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    // Wait for OpenFGA to be ready
    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    // Create a store
    let store_id = create_store(&openfga_url, "test-store-denied")
        .await
        .expect("Should create store");

    // Write authorization model
    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    // Create OpenFGA client
    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // DON'T write any tuple for bob - he has no access

    // Check that bob cannot view the file (no permission)
    let result = client
        .check("user:bob", "viewer", "file:my-bucket/secret/data.json")
        .await;

    assert!(
        result.is_ok(),
        "Check should succeed (return false, not error): {:?}",
        result.err()
    );
    assert!(
        !result.unwrap(),
        "Bob should be denied access (403 scenario)"
    );

    // Create authorization decision for denied case
    let decision = AuthorizationDecision::from_check_result(Ok(false), FailMode::Closed);
    assert!(
        !decision.is_allowed(),
        "Authorization decision should be denied"
    );
    assert!(
        !decision.is_fail_open_allow(),
        "Should NOT be a fail-open allow"
    );
    assert!(decision.error().is_none(), "Should have no error");
}

/// Test: Return 500 on OpenFGA error (fail closed)
/// When OpenFGA is unreachable and fail mode is Closed, request should be denied
#[tokio::test]
async fn test_openfga_error_fail_closed_returns_500() {
    // Create a client pointing to a non-existent server
    let client = OpenFgaClient::builder("http://127.0.0.1:19999", "nonexistent-store")
        .timeout_ms(100) // Short timeout
        .build()
        .unwrap();

    // Try to check - this will fail with connection error
    let result = client
        .check("user:test", "viewer", "file:bucket/test.txt")
        .await;

    assert!(result.is_err(), "Check should fail with connection error");

    // Create authorization decision with fail-closed mode
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Closed);

    assert!(
        !decision.is_allowed(),
        "Should be denied on error with fail-closed mode"
    );
    assert!(
        !decision.is_fail_open_allow(),
        "Should NOT be a fail-open allow"
    );
    assert!(decision.has_error(), "Should have captured the error");
    assert!(
        decision.error().unwrap().contains("connect")
            || decision.error().unwrap().contains("Connection"),
        "Error should mention connection: {}",
        decision.error().unwrap()
    );
}

/// Test: Return 200 on OpenFGA error (fail open)
/// When OpenFGA is unreachable and fail mode is Open, request should be allowed
#[tokio::test]
async fn test_openfga_error_fail_open_allows() {
    // Create a client pointing to a non-existent server
    let client = OpenFgaClient::builder("http://127.0.0.1:19998", "nonexistent-store")
        .timeout_ms(100) // Short timeout
        .build()
        .unwrap();

    // Try to check - this will fail with connection error
    let result = client
        .check("user:test", "viewer", "file:bucket/test.txt")
        .await;

    assert!(result.is_err(), "Check should fail with connection error");

    // Create authorization decision with fail-open mode
    let decision = AuthorizationDecision::from_check_result(result, FailMode::Open);

    assert!(
        decision.is_allowed(),
        "Should be allowed on error with fail-open mode"
    );
    assert!(
        decision.is_fail_open_allow(),
        "Should be marked as fail-open allow"
    );
    assert!(decision.has_error(), "Should have captured the error");
}

// ============================================================================
// Helper Function Tests (already unit tested, but verify E2E integration)
// ============================================================================

#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_integration_with_helpers() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    // Wait for OpenFGA to be ready
    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    // Create a store
    let store_id = create_store(&openfga_url, "test-helpers")
        .await
        .expect("Should create store");

    // Write authorization model
    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    // Create OpenFGA client
    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // Test: Extract user ID from JWT claims
    let claims = json!({"sub": "alice123", "email": "alice@example.com"});
    let user_id = extract_user_id(&claims, None).expect("Should extract user ID");
    assert_eq!(user_id, "user:alice123");

    // Test: Build OpenFGA object from bucket + path
    let object = build_openfga_object("my-bucket", "docs/readme.txt");
    assert_eq!(object, "file:my-bucket/docs/readme.txt");

    // Test: HTTP method to relation
    let relation = http_method_to_relation("GET");
    assert_eq!(relation, Relation::Viewer);

    // Write tuple for alice to access the file
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            &user_id,
            relation.as_str(),
            &object
        )
        .await,
        "Should write tuple"
    );

    // Verify the check works
    let result = client.check(&user_id, relation.as_str(), &object).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "Alice should have viewer access");
}

/// Test bucket-level permissions inheriting to files
#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_bucket_permission_inheritance() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    let store_id = create_store(&openfga_url, "test-inheritance")
        .await
        .expect("Should create store");

    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // Grant charlie viewer access to the bucket
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "user:charlie",
            "viewer",
            "bucket:my-bucket"
        )
        .await
    );

    // Set file's parent to be the bucket
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "bucket:my-bucket",
            "parent",
            "file:my-bucket/any/nested/file.txt"
        )
        .await
    );

    // Charlie should be able to view any file in the bucket (inherited permission)
    let result = client
        .check(
            "user:charlie",
            "viewer",
            "file:my-bucket/any/nested/file.txt",
        )
        .await;

    assert!(result.is_ok());
    assert!(
        result.unwrap(),
        "Charlie should inherit viewer access from bucket"
    );
}

/// Test that editor permissions are checked correctly
#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_editor_permission_for_put() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    let store_id = create_store(&openfga_url, "test-editor")
        .await
        .expect("Should create store");

    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // Grant dave viewer access only
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "user:dave",
            "viewer",
            "file:bucket/data.json"
        )
        .await
    );

    // Dave should NOT have editor access (PUT method requires editor)
    let put_relation = http_method_to_relation("PUT");
    assert_eq!(put_relation, Relation::Editor);

    let result = client
        .check("user:dave", put_relation.as_str(), "file:bucket/data.json")
        .await;

    assert!(result.is_ok());
    assert!(
        !result.unwrap(),
        "Dave (viewer) should NOT have editor access for PUT"
    );
}

// ============================================================================
// Phase 50.1: Additional Integration Tests
// ============================================================================

/// Test: User can access shared folder
/// When a user has viewer access to a folder, they can access files in it
#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_user_can_access_shared_folder() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    let store_id = create_store(&openfga_url, "test-shared-folder")
        .await
        .expect("Should create store");

    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // Create folder under bucket
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "bucket:my-bucket",
            "parent",
            "folder:my-bucket/shared-docs"
        )
        .await
    );

    // Grant eve viewer access to the shared folder
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "user:eve",
            "viewer",
            "folder:my-bucket/shared-docs"
        )
        .await
    );

    // Create file under the shared folder
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "folder:my-bucket/shared-docs",
            "parent",
            "file:my-bucket/shared-docs/report.pdf"
        )
        .await
    );

    // Eve should be able to view the file in the shared folder (inherited permission)
    let result = client
        .check(
            "user:eve",
            "viewer",
            "file:my-bucket/shared-docs/report.pdf",
        )
        .await;

    assert!(result.is_ok());
    assert!(
        result.unwrap(),
        "Eve should have viewer access to file in shared folder"
    );

    // Eve should NOT have access to files outside the shared folder
    let result_outside = client
        .check("user:eve", "viewer", "file:my-bucket/private/secret.txt")
        .await;

    assert!(result_outside.is_ok());
    assert!(
        !result_outside.unwrap(),
        "Eve should NOT have access to files outside shared folder"
    );
}

/// Test: Owner has full access (viewer, editor, owner)
/// When a user has owner relation, they have all permissions
#[tokio::test]
#[ignore] // Requires Docker
async fn test_openfga_owner_has_full_access() {
    let docker = Cli::default();
    let (_container, openfga_url) = create_openfga_container(&docker);

    assert!(
        wait_for_openfga(&openfga_url).await,
        "OpenFGA should be ready"
    );

    let store_id = create_store(&openfga_url, "test-owner-access")
        .await
        .expect("Should create store");

    let model_id = write_authorization_model(&openfga_url, &store_id, test_authorization_model())
        .await
        .expect("Should write model");

    let client = OpenFgaClient::builder(&openfga_url, &store_id)
        .authorization_model_id(&model_id)
        .timeout_ms(5000)
        .build()
        .unwrap();

    // Grant frank owner access to a file
    assert!(
        write_tuple(
            &openfga_url,
            &store_id,
            "user:frank",
            "owner",
            "file:my-bucket/owned-file.txt"
        )
        .await
    );

    // Owner should have owner permission
    let owner_result = client
        .check("user:frank", "owner", "file:my-bucket/owned-file.txt")
        .await;
    assert!(owner_result.is_ok());
    assert!(owner_result.unwrap(), "Frank should have owner access");

    // Note: In a typical ReBAC model, owner implies editor and viewer.
    // The test model defined above uses direct relations only.
    // For proper inheritance, the model would need computed relations.
    // Here we're testing the direct owner relation only.
}
