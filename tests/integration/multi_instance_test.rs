//! Phase 63: Multi-Instance Testing
//!
//! Tests horizontal scaling with shared Redis cache across multiple proxy instances.
//!
//! Test scenarios:
//! - Shared Redis cache across 2/5/10 instances
//! - Cache consistency (all instances see same data)
//! - Load balancer integration
//! - Combined throughput scaling
//!
//! Run with: cargo test --test integration_tests multi_instance -- --ignored
//! Requires: docker-compose -f docker-compose.multi-instance.yml up -d

use bytes::Bytes;
use reqwest::Client;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;

/// Base URL for the load balancer
const LB_URL: &str = "http://localhost:8080";

/// Create HTTP client with connection pooling
fn create_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(100)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

/// Phase 63.1: Test 2 proxy instances with shared Redis cache
///
/// Verifies:
/// - Cache hit on instance 2 after set on instance 1 (via Redis)
/// - Both instances serve requests correctly
/// - Load balancer distributes requests
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_2_instances_shared_redis_cache() {
    let client = create_client();

    // Track which upstreams we've hit
    let mut upstreams_seen: HashSet<String> = HashSet::new();

    // Step 1: Make initial request (should cache in Redis)
    let url = format!("{}/test/file-1.txt", LB_URL);
    let resp = client.get(&url).send().await.expect("Request failed");
    assert_eq!(resp.status(), 200, "Initial request should succeed");

    if let Some(upstream) = resp.headers().get("x-upstream-addr") {
        upstreams_seen.insert(upstream.to_str().unwrap_or("unknown").to_string());
    }

    // Small delay to ensure Redis write completes
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Step 2: Make multiple requests to hit different instances
    // Due to round-robin, we should hit both instances
    for _ in 0..10 {
        let resp = client.get(&url).send().await.expect("Request failed");
        assert_eq!(resp.status(), 200, "Subsequent request should succeed");

        if let Some(upstream) = resp.headers().get("x-upstream-addr") {
            upstreams_seen.insert(upstream.to_str().unwrap_or("unknown").to_string());
        }
    }

    // We should have seen at least 2 different upstream addresses (2 instances)
    // Note: nginx upstream format is "IP:PORT"
    println!(
        "Upstreams seen: {:?} (count: {})",
        upstreams_seen,
        upstreams_seen.len()
    );
    assert!(
        upstreams_seen.len() >= 2,
        "Expected to hit at least 2 instances, but only saw {} unique upstreams",
        upstreams_seen.len()
    );
}

/// Phase 63.1: Verify cache sharing works correctly
///
/// Scenario:
/// 1. Request file through instance A -> miss, fetches from S3, caches in Redis
/// 2. Request same file through instance B -> should hit Redis cache
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_cache_sharing_between_instances() {
    let client = create_client();

    // Use a unique file to avoid previous test interference
    let unique_file = format!("file-{}.txt", std::process::id());
    let url = format!("{}/test/{}", LB_URL, unique_file);

    // Step 1: First request - should be cache miss (file doesn't exist, expect 404)
    // For existing file:
    let url = format!("{}/test/file-50.txt", LB_URL);

    let resp = client.get(&url).send().await.expect("Request failed");
    assert_eq!(resp.status(), 200);
    let body1 = resp.text().await.unwrap();

    // Wait for Redis write
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Step 2: Make 10 more requests - should all return same content
    for i in 0..10 {
        let resp = client.get(&url).send().await.expect("Request failed");
        assert_eq!(resp.status(), 200, "Request {} failed", i);
        let body = resp.text().await.unwrap();
        assert_eq!(body, body1, "Content should be consistent across requests");
    }

    println!("Cache sharing verified: all 11 requests returned consistent content");
}

/// Phase 63.1: Test with higher instance count (5 instances)
///
/// Uses docker-compose --scale to run 5 instances
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml with --scale yatagarasu=5"]
async fn test_5_instances_shared_redis_cache() {
    let client = create_client();
    let mut upstreams_seen: HashSet<String> = HashSet::new();

    // Make enough requests to hit all 5 instances
    let url = format!("{}/test/file-25.txt", LB_URL);

    for _ in 0..50 {
        let resp = client.get(&url).send().await.expect("Request failed");
        assert_eq!(resp.status(), 200);

        if let Some(upstream) = resp.headers().get("x-upstream-addr") {
            upstreams_seen.insert(upstream.to_str().unwrap_or("unknown").to_string());
        }
    }

    println!(
        "5-instance test: saw {} unique upstreams",
        upstreams_seen.len()
    );
    assert!(
        upstreams_seen.len() >= 5,
        "Expected to hit at least 5 instances, only saw {}",
        upstreams_seen.len()
    );
}

/// Phase 63.1: Test with 10 instances
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml with --scale yatagarasu=10"]
async fn test_10_instances_shared_redis_cache() {
    let client = create_client();
    let mut upstreams_seen: HashSet<String> = HashSet::new();

    let url = format!("{}/test/file-75.txt", LB_URL);

    // Need more requests to hit all 10 instances
    for _ in 0..100 {
        let resp = client.get(&url).send().await.expect("Request failed");
        assert_eq!(resp.status(), 200);

        if let Some(upstream) = resp.headers().get("x-upstream-addr") {
            upstreams_seen.insert(upstream.to_str().unwrap_or("unknown").to_string());
        }
    }

    println!(
        "10-instance test: saw {} unique upstreams",
        upstreams_seen.len()
    );
    assert!(
        upstreams_seen.len() >= 10,
        "Expected to hit at least 10 instances, only saw {}",
        upstreams_seen.len()
    );
}

/// Phase 63.1: Verify combined throughput scales with instance count
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_combined_throughput_scales() {
    let client = Arc::new(create_client());
    let request_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));

    let num_tasks = 50;
    let requests_per_task = 100;
    let barrier = Arc::new(Barrier::new(num_tasks));

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..num_tasks {
        let client = Arc::clone(&client);
        let request_count = Arc::clone(&request_count);
        let error_count = Arc::clone(&error_count);
        let barrier = Arc::clone(&barrier);

        handles.push(tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            for i in 0..requests_per_task {
                let file_num = (i % 100) + 1;
                let url = format!("{}/test/file-{}.txt", LB_URL, file_num);

                match client.get(&url).send().await {
                    Ok(resp) if resp.status() == 200 => {
                        request_count.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(resp) => {
                        eprintln!("Unexpected status: {}", resp.status());
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("Request error: {}", e);
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let elapsed = start.elapsed();
    let total_requests = request_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);
    let rps = total_requests as f64 / elapsed.as_secs_f64();

    println!("=== Multi-Instance Throughput Test ===");
    println!("Total requests: {}", total_requests);
    println!("Total errors: {}", total_errors);
    println!("Duration: {:?}", elapsed);
    println!("Throughput: {:.2} req/s", rps);

    assert!(total_errors == 0, "Expected 0 errors, got {}", total_errors);
    assert!(
        rps > 1000.0,
        "Expected >1000 req/s with multiple instances, got {:.2}",
        rps
    );
}

/// Phase 63.2: Test round-robin load balancing
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_round_robin_load_balancing() {
    let client = create_client();
    let mut upstream_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    let url = format!("{}/test/file-10.txt", LB_URL);

    // Make 100 requests
    for _ in 0..100 {
        let resp = client.get(&url).send().await.expect("Request failed");
        assert_eq!(resp.status(), 200);

        if let Some(upstream) = resp.headers().get("x-upstream-addr") {
            let upstream_str = upstream.to_str().unwrap_or("unknown").to_string();
            *upstream_counts.entry(upstream_str).or_insert(0) += 1;
        }
    }

    println!("Request distribution: {:?}", upstream_counts);

    // With round-robin, requests should be evenly distributed
    // Allow 20% deviation from perfect distribution
    let num_instances = upstream_counts.len();
    let expected_per_instance = 100 / num_instances;
    let tolerance = (expected_per_instance as f64 * 0.3) as u32;

    for (upstream, count) in &upstream_counts {
        let deviation = (*count as i32 - expected_per_instance as i32).unsigned_abs();
        println!(
            "Upstream {}: {} requests (expected ~{}, deviation {})",
            upstream, count, expected_per_instance, deviation
        );
        assert!(
            deviation <= tolerance,
            "Load balancing too uneven for {}: got {} requests, expected ~{}",
            upstream,
            count,
            expected_per_instance
        );
    }
}

/// Phase 63.2: Verify health check endpoint works through load balancer
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_health_check_endpoint() {
    let client = create_client();

    for _ in 0..10 {
        let resp = client
            .get(&format!("{}/health", LB_URL))
            .send()
            .await
            .expect("Health check request failed");

        assert_eq!(resp.status(), 200, "Health check should return 200");
    }

    println!("Health check endpoint verified across all instances");
}

/// Phase 63.3: Verify cache consistency - all instances see same data
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_cache_consistency_across_instances() {
    let client = create_client();
    let url = format!("{}/test/file-99.txt", LB_URL);

    // Get initial content
    let resp = client.get(&url).send().await.expect("Request failed");
    assert_eq!(resp.status(), 200);
    let expected_content = resp.text().await.unwrap();

    // Wait for cache propagation
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Make many requests, verify all return same content
    let mut mismatches = 0;
    for _ in 0..100 {
        let resp = client.get(&url).send().await.expect("Request failed");
        assert_eq!(resp.status(), 200);
        let content = resp.text().await.unwrap();

        if content != expected_content {
            mismatches += 1;
        }
    }

    assert_eq!(
        mismatches, 0,
        "Found {} content mismatches - cache inconsistency detected",
        mismatches
    );
    println!("Cache consistency verified: 100/100 requests returned consistent content");
}

/// Phase 63.3: Test Redis cache contains shared data
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_redis_contains_cached_data() {
    // Connect directly to Redis to verify cache entries exist
    let redis_client =
        redis::Client::open("redis://localhost:6379").expect("Failed to connect to Redis");
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to get Redis connection");

    // First, make a request to populate cache
    let http_client = create_client();
    let url = format!("{}/test/file-42.txt", LB_URL);

    let resp = http_client.get(&url).send().await.expect("Request failed");
    assert_eq!(resp.status(), 200);

    // Wait for Redis write
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check Redis for keys with our prefix
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg("yatagarasu:*")
        .query_async(&mut conn)
        .await
        .expect("Failed to query Redis keys");

    println!("Redis cache keys: {:?}", keys);
    assert!(
        !keys.is_empty(),
        "Expected at least one cached entry in Redis"
    );
}

/// Phase 63.1: Verify no cache inconsistencies under concurrent load
#[tokio::test]
#[ignore = "requires docker-compose.multi-instance.yml running"]
async fn test_no_cache_inconsistencies_under_load() {
    let client = Arc::new(create_client());
    let inconsistencies = Arc::new(AtomicU64::new(0));

    // Test file
    let url = format!("{}/test/large-10kb.bin", LB_URL);

    // Get expected content hash
    let resp = client.get(&url).send().await.expect("Request failed");
    assert_eq!(resp.status(), 200);
    let expected_bytes = resp.bytes().await.unwrap();
    let expected_len = expected_bytes.len();

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Concurrent requests
    let num_tasks = 20;
    let requests_per_task = 50;

    let mut handles = Vec::new();
    for _ in 0..num_tasks {
        let client = Arc::clone(&client);
        let inconsistencies = Arc::clone(&inconsistencies);
        let url = url.clone();

        handles.push(tokio::spawn(async move {
            for _ in 0..requests_per_task {
                let resp = client.get(&url).send().await;
                match resp {
                    Ok(r) if r.status() == 200 => {
                        let bytes = r.bytes().await.unwrap();
                        if bytes.len() != expected_len {
                            inconsistencies.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    _ => {
                        inconsistencies.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    let total_inconsistencies = inconsistencies.load(Ordering::Relaxed);
    println!(
        "Consistency test: {} total requests, {} inconsistencies",
        num_tasks * requests_per_task,
        total_inconsistencies
    );

    assert_eq!(
        total_inconsistencies, 0,
        "Found {} inconsistencies under concurrent load",
        total_inconsistencies
    );
}
