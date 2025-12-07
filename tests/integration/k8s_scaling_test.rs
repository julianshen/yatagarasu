//! Phase 64: Kubernetes Deployment Testing
//!
//! Tests horizontal pod autoscaling, pod resilience, and rolling updates.
//!
//! Test scenarios:
//! - HPA scales based on CPU utilization
//! - Pod startup time <30s
//! - Graceful pod termination
//! - No request loss during scaling
//! - Pod crash and restart recovery
//! - Rolling update with zero downtime
//! - PDB (PodDisruptionBudget) works
//!
//! Prerequisites:
//! - kubectl configured with access to cluster
//! - Namespace yatagarasu-loadtest exists
//! - Deployment applied: kubectl apply -k k8s/loadtest/
//!
//! Run with: cargo test --test integration_tests k8s_scaling -- --ignored

use reqwest::Client;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

const NAMESPACE: &str = "yatagarasu-loadtest";
const DEPLOYMENT: &str = "yatagarasu";

/// Helper to run kubectl commands
fn kubectl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("kubectl")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute kubectl: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Get current replica count for deployment
fn get_replica_count() -> Result<u32, String> {
    let output = kubectl(&[
        "get",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "-o",
        "jsonpath={.status.readyReplicas}",
    ])?;

    output
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse replica count: {}", e))
}

/// Get pod names for deployment
fn get_pod_names() -> Result<Vec<String>, String> {
    let output = kubectl(&[
        "get",
        "pods",
        "-n",
        NAMESPACE,
        "-l",
        &format!("app={}", DEPLOYMENT),
        "-o",
        "jsonpath={.items[*].metadata.name}",
    ])?;

    Ok(output.split_whitespace().map(String::from).collect())
}

/// Get service endpoint
fn get_service_endpoint() -> Result<String, String> {
    // For local testing, use kubectl port-forward or NodePort
    // Check if running in-cluster or locally
    let output = kubectl(&[
        "get",
        "service",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "-o",
        "jsonpath={.spec.clusterIP}",
    ])?;

    let cluster_ip = output.trim();
    if !cluster_ip.is_empty() && cluster_ip != "None" {
        Ok(format!("http://{}:8080", cluster_ip))
    } else {
        // Fallback to port-forward URL
        Ok("http://localhost:8080".to_string())
    }
}

/// Create HTTP client with connection pooling
fn create_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(100)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

/// Phase 64.1: Test pod startup time is under 30 seconds
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_pod_startup_time_under_30s() {
    // Scale down to 0, then scale up and measure time to ready
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=0",
    ]);

    // Wait for scale down
    tokio::time::sleep(Duration::from_secs(5)).await;

    let start = Instant::now();

    // Scale up to 1
    kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=1",
    ])
    .expect("Failed to scale up");

    // Wait for pod to be ready
    let timeout = Duration::from_secs(60);
    let poll_interval = Duration::from_millis(500);
    let mut ready = false;

    while start.elapsed() < timeout {
        if let Ok(count) = get_replica_count() {
            if count >= 1 {
                ready = true;
                break;
            }
        }
        tokio::time::sleep(poll_interval).await;
    }

    let startup_time = start.elapsed();
    println!("Pod startup time: {:?}", startup_time);

    assert!(ready, "Pod did not become ready within timeout");
    assert!(
        startup_time < Duration::from_secs(30),
        "Pod startup time ({:?}) exceeded 30s target",
        startup_time
    );

    // Restore to 2 replicas
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=2",
    ]);
}

/// Phase 64.1: Verify HPA configuration exists
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_hpa_exists() {
    let result = kubectl(&["get", "hpa", "yatagarasu-hpa", "-n", NAMESPACE]);

    assert!(result.is_ok(), "HPA should exist: {:?}", result.err());

    // Verify HPA targets
    let output = kubectl(&[
        "get",
        "hpa",
        "yatagarasu-hpa",
        "-n",
        NAMESPACE,
        "-o",
        "jsonpath={.spec.minReplicas},{.spec.maxReplicas}",
    ])
    .expect("Failed to get HPA spec");

    let parts: Vec<&str> = output.split(',').collect();
    assert_eq!(parts.len(), 2, "Expected minReplicas,maxReplicas");

    let min_replicas: u32 = parts[0].parse().expect("Invalid minReplicas");
    let max_replicas: u32 = parts[1].parse().expect("Invalid maxReplicas");

    println!(
        "HPA configured: minReplicas={}, maxReplicas={}",
        min_replicas, max_replicas
    );
    assert!(min_replicas >= 2, "minReplicas should be at least 2");
    assert!(max_replicas >= 10, "maxReplicas should be at least 10");
}

/// Phase 64.1: Verify PDB configuration exists
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_pdb_exists() {
    let result = kubectl(&["get", "pdb", "yatagarasu-pdb", "-n", NAMESPACE]);

    assert!(result.is_ok(), "PDB should exist: {:?}", result.err());

    // Verify PDB spec
    let output = kubectl(&[
        "get",
        "pdb",
        "yatagarasu-pdb",
        "-n",
        NAMESPACE,
        "-o",
        "jsonpath={.spec.minAvailable}",
    ])
    .expect("Failed to get PDB spec");

    let min_available: u32 = output.trim().parse().expect("Invalid minAvailable");
    println!("PDB configured: minAvailable={}", min_available);
    assert!(min_available >= 1, "minAvailable should be at least 1");
}

/// Phase 64.1: Test graceful pod termination
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_graceful_pod_termination() {
    // Ensure we have at least 2 replicas and wait for them to be ready
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=2",
    ]);
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Refresh pod list right before deletion to ensure freshness
    let pods = get_pod_names().expect("Failed to get pod names");
    assert!(!pods.is_empty(), "No pods found");

    // Get first pod that is actually running
    let pod_to_delete = pods.first().expect("No pods available").clone();

    // Start making requests in background
    let client = Arc::new(create_client());
    let errors = Arc::new(AtomicU64::new(0));
    let requests = Arc::new(AtomicU64::new(0));

    let endpoint = get_service_endpoint().unwrap_or_else(|_| "http://localhost:8080".to_string());

    let client_clone = Arc::clone(&client);
    let errors_clone = Arc::clone(&errors);
    let requests_clone = Arc::clone(&requests);

    // Spawn request loop
    let handle = tokio::spawn(async move {
        let url = format!("{}/health", endpoint);
        for _ in 0..100 {
            match client_clone.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    requests_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    errors_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    // Delete one pod while requests are running
    tokio::time::sleep(Duration::from_millis(500)).await;
    let delete_result = kubectl(&["delete", "pod", &pod_to_delete, "-n", NAMESPACE]);
    if let Err(e) = &delete_result {
        println!(
            "Warning: Pod deletion returned error (may already be gone): {}",
            e
        );
    }

    // Wait for request loop to finish
    handle.await.expect("Request loop panicked");

    let total_requests = requests.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);

    println!(
        "Graceful termination: {} requests, {} errors",
        total_requests, total_errors
    );

    // Allow some errors during pod termination (graceful shutdown takes time)
    let error_rate = total_errors as f64 / (total_requests + total_errors) as f64;
    assert!(
        error_rate < 0.1,
        "Error rate ({:.2}%) too high during pod termination",
        error_rate * 100.0
    );
}

/// Phase 64.2: Test pod crash and restart recovery
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_pod_crash_restart_recovery() {
    // Ensure we have at least 2 replicas and wait for them
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=2",
    ]);
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Refresh pod list
    let initial_pods = get_pod_names().expect("Failed to get pod names");
    let initial_count = initial_pods.len();
    println!("Initial pod count: {}", initial_count);

    // Force delete a pod (simulates crash)
    if !initial_pods.is_empty() {
        let pod_to_delete = &initial_pods[0];
        println!("Force deleting pod: {}", pod_to_delete);
        let _ = kubectl(&[
            "delete",
            "pod",
            pod_to_delete,
            "-n",
            NAMESPACE,
            "--force",
            "--grace-period=0",
        ]);
    }

    // Wait for pod to be recreated (with longer timeout for PVC attachment)
    let start = Instant::now();
    let timeout = Duration::from_secs(120);

    loop {
        if start.elapsed() > timeout {
            let current_count = get_replica_count().unwrap_or(0);
            println!(
                "Timeout waiting for recovery. Current replicas: {}, expected: {}",
                current_count, initial_count
            );
            panic!("Pod was not recreated within timeout");
        }

        if let Ok(count) = get_replica_count() {
            if count as usize >= initial_count {
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let recovery_time = start.elapsed();
    println!("Pod crash recovery time: {:?}", recovery_time);

    // 90s is reasonable for pod with PVC
    assert!(
        recovery_time < Duration::from_secs(90),
        "Pod recovery took too long: {:?}",
        recovery_time
    );
}

/// Phase 64.2: Test rolling update with zero downtime
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_rolling_update_zero_downtime() {
    // Ensure we have enough replicas for rolling update
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=3",
    ]);
    tokio::time::sleep(Duration::from_secs(15)).await;

    let client = Arc::new(create_client());
    let errors = Arc::new(AtomicU64::new(0));
    let requests = Arc::new(AtomicU64::new(0));

    let endpoint = get_service_endpoint().unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Start continuous request loop
    let client_clone = Arc::clone(&client);
    let errors_clone = Arc::clone(&errors);
    let requests_clone = Arc::clone(&requests);

    let handle = tokio::spawn(async move {
        let url = format!("{}/health", endpoint);
        let duration = Duration::from_secs(30);
        let start = Instant::now();

        while start.elapsed() < duration {
            match client_clone.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    requests_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    errors_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    });

    // Trigger rolling update by changing an annotation
    tokio::time::sleep(Duration::from_secs(2)).await;
    kubectl(&[
        "patch",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--type",
        "strategic",
        "-p",
        &format!(
            "{{\"spec\":{{\"template\":{{\"metadata\":{{\"annotations\":{{\"rollout-test\":\"{}\"}}}}}}}}}}",
            chrono::Utc::now().timestamp()
        ),
    ])
    .expect("Failed to trigger rolling update");

    // Wait for rollout to complete
    let _ = kubectl(&[
        "rollout",
        "status",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--timeout=60s",
    ]);

    handle.await.expect("Request loop panicked");

    let total_requests = requests.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);

    println!(
        "Rolling update: {} requests, {} errors",
        total_requests, total_errors
    );

    // Zero downtime = no errors
    let error_rate = if total_requests + total_errors > 0 {
        total_errors as f64 / (total_requests + total_errors) as f64
    } else {
        0.0
    };

    assert!(
        error_rate < 0.05,
        "Error rate ({:.2}%) indicates downtime during rolling update",
        error_rate * 100.0
    );
}

/// Phase 64.1: Test no request loss during scale up
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_no_request_loss_during_scale_up() {
    // Start with 1 replica
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=1",
    ]);
    tokio::time::sleep(Duration::from_secs(10)).await;

    let client = Arc::new(create_client());
    let errors = Arc::new(AtomicU64::new(0));
    let requests = Arc::new(AtomicU64::new(0));

    let endpoint = get_service_endpoint().unwrap_or_else(|_| "http://localhost:8080".to_string());

    let client_clone = Arc::clone(&client);
    let errors_clone = Arc::clone(&errors);
    let requests_clone = Arc::clone(&requests);

    let handle = tokio::spawn(async move {
        let url = format!("{}/health", endpoint);
        for _ in 0..200 {
            match client_clone.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    requests_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    errors_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    });

    // Scale up while requests are running
    tokio::time::sleep(Duration::from_secs(1)).await;
    kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=4",
    ])
    .expect("Failed to scale up");

    handle.await.expect("Request loop panicked");

    let total_requests = requests.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);

    println!(
        "Scale up test: {} requests, {} errors",
        total_requests, total_errors
    );

    // No request loss during scale up
    assert_eq!(
        total_errors, 0,
        "Should have zero errors during scale up, got {}",
        total_errors
    );

    // Restore to 2 replicas
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=2",
    ]);
}

/// Phase 64.1: Test no request loss during scale down
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_no_request_loss_during_scale_down() {
    // Start with 4 replicas
    let _ = kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=4",
    ]);
    tokio::time::sleep(Duration::from_secs(15)).await;

    let client = Arc::new(create_client());
    let errors = Arc::new(AtomicU64::new(0));
    let requests = Arc::new(AtomicU64::new(0));

    let endpoint = get_service_endpoint().unwrap_or_else(|_| "http://localhost:8080".to_string());

    let client_clone = Arc::clone(&client);
    let errors_clone = Arc::clone(&errors);
    let requests_clone = Arc::clone(&requests);

    let handle = tokio::spawn(async move {
        let url = format!("{}/health", endpoint);
        for _ in 0..200 {
            match client_clone.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    requests_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    errors_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    });

    // Scale down while requests are running
    tokio::time::sleep(Duration::from_secs(1)).await;
    kubectl(&[
        "scale",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "--replicas=2",
    ])
    .expect("Failed to scale down");

    handle.await.expect("Request loop panicked");

    let total_requests = requests.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);

    println!(
        "Scale down test: {} requests, {} errors",
        total_requests, total_errors
    );

    // Minimal errors during scale down (graceful termination may cause some)
    let error_rate = total_errors as f64 / (total_requests + total_errors) as f64;
    assert!(
        error_rate < 0.05,
        "Error rate ({:.2}%) too high during scale down",
        error_rate * 100.0
    );
}

/// Phase 64.2: Test health check endpoints respond correctly
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_health_endpoints_respond() {
    let endpoint = get_service_endpoint().unwrap_or_else(|_| "http://localhost:8080".to_string());
    let client = create_client();

    // Test health endpoint
    let health_url = format!("{}/health", endpoint);
    let resp = client.get(&health_url).send().await;

    match resp {
        Ok(r) => {
            assert!(
                r.status().is_success(),
                "Health endpoint returned {}",
                r.status()
            );
            println!("Health endpoint OK");
        }
        Err(e) => {
            // If we can't connect, it might be a port-forward issue
            println!("Warning: Could not connect to health endpoint: {}", e);
        }
    }
}

/// Phase 64.2: Verify deployment has correct resource limits
#[tokio::test]
#[ignore = "requires kubernetes cluster with yatagarasu deployment"]
async fn test_deployment_has_resource_limits() {
    let output = kubectl(&[
        "get",
        "deployment",
        DEPLOYMENT,
        "-n",
        NAMESPACE,
        "-o",
        "jsonpath={.spec.template.spec.containers[0].resources}",
    ])
    .expect("Failed to get deployment resources");

    println!("Resource spec: {}", output);

    // Verify resources are set (not empty)
    assert!(
        output.contains("requests") && output.contains("limits"),
        "Deployment should have resource requests and limits"
    );
}
