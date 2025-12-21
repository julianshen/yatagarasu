// Request Coalescing Integration Tests - Phase 38
//
// Tests for request deduplication when multiple clients request the same S3 object
// Verifies that followers actually wait for the leader to complete

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use yatagarasu::cache::CacheKey;
    use yatagarasu::request_coalescing::RequestCoalescer;

    #[tokio::test]
    async fn test_request_coalescer_deduplicates_concurrent_requests() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-object.txt".to_string(),
            etag: None,
            variant: None,
        };

        // Track how many times the "fetch" operation runs
        let fetch_count = Arc::new(AtomicUsize::new(0));

        // Leader acquires first
        let leader_slot = coalescer.acquire(&key).await;
        assert!(leader_slot.is_leader(), "First request should be leader");

        // Spawn 5 concurrent follower requests for the same object
        let mut handles = vec![];
        for _ in 0..5 {
            let coalescer_clone = coalescer.clone();
            let key_clone = key.clone();
            let fetch_count_clone = Arc::clone(&fetch_count);

            let handle = tokio::spawn(async move {
                // Acquire coalescing slot - followers will wait here
                let slot = coalescer_clone.acquire(&key_clone).await;

                // Only the leader should fetch from S3
                if slot.is_leader() {
                    fetch_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                // Followers don't fetch - they wait for leader and read from cache
            });

            handles.push(handle);
        }

        // Give followers time to start waiting
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Simulate leader fetching from S3
        fetch_count.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Leader completes - this releases all waiting followers
        drop(leader_slot);

        // Wait for all followers to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Only 1 request (the leader) should have "fetched" from S3
        assert_eq!(
            fetch_count.load(Ordering::SeqCst),
            1,
            "Only the leader should fetch from S3 - true deduplication"
        );
    }

    #[tokio::test]
    async fn test_followers_actually_wait_for_leader() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-object.txt".to_string(),
            etag: None,
            variant: None,
        };

        // Track when followers complete
        let follower_completed = Arc::new(AtomicUsize::new(0));

        // Leader acquires first
        let leader_slot = coalescer.acquire(&key).await;
        assert!(leader_slot.is_leader());

        // Spawn a follower
        let coalescer_clone = coalescer.clone();
        let key_clone = key.clone();
        let follower_completed_clone = Arc::clone(&follower_completed);
        let follower_handle = tokio::spawn(async move {
            // This should block until leader completes
            let slot = coalescer_clone.acquire(&key_clone).await;
            follower_completed_clone.fetch_add(1, Ordering::SeqCst);
            slot.is_follower()
        });

        // Give follower time to start waiting
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Follower should NOT have completed yet (still waiting)
        assert_eq!(
            follower_completed.load(Ordering::SeqCst),
            0,
            "Follower should be blocked waiting for leader"
        );

        // Leader completes
        drop(leader_slot);

        // Now follower should complete
        let is_follower = follower_handle.await.unwrap();
        assert!(is_follower, "Request should be marked as follower");
        assert_eq!(
            follower_completed.load(Ordering::SeqCst),
            1,
            "Follower should complete after leader"
        );
    }

    #[tokio::test]
    async fn test_request_coalescer_tracks_in_flight_requests() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-object.txt".to_string(),
            etag: None,
            variant: None,
        };

        assert_eq!(coalescer.in_flight_count().await, 0);

        let slot = coalescer.acquire(&key).await;
        assert!(slot.is_leader());
        assert_eq!(coalescer.in_flight_count().await, 1);

        drop(slot);
        // Note: cleanup happens asynchronously, so we need to wait
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(
            coalescer.in_flight_count().await,
            0,
            "in_flight_count should be 0 after leader completes and cleanup runs"
        );
    }

    #[tokio::test]
    async fn test_request_coalescer_different_keys_dont_block() {
        let coalescer = RequestCoalescer::new();
        let key1 = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "object1.txt".to_string(),
            etag: None,
            variant: None,
        };
        let key2 = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "object2.txt".to_string(),
            etag: None,
            variant: None,
        };

        let slot1 = coalescer.acquire(&key1).await;
        let slot2 = coalescer.acquire(&key2).await;

        // Both should be leaders (different keys don't block each other)
        assert!(slot1.is_leader(), "First key should have its own leader");
        assert!(slot2.is_leader(), "Second key should have its own leader");
        assert_eq!(coalescer.in_flight_count().await, 2);

        drop(slot1);
        drop(slot2);
    }

    #[tokio::test]
    async fn test_request_coalescer_same_bucket_different_keys() {
        let coalescer = RequestCoalescer::new();
        let fetch_count = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for i in 0..3 {
            let coalescer_clone = coalescer.clone();
            let fetch_count_clone = Arc::clone(&fetch_count);
            let handle = tokio::spawn(async move {
                let key = CacheKey {
                    bucket: "test-bucket".to_string(),
                    object_key: format!("object{}.txt", i),
                    etag: None,
                    variant: None,
                };
                let slot = coalescer_clone.acquire(&key).await;
                // Each key should have its own leader
                if slot.is_leader() {
                    fetch_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // All 3 should be leaders (different keys)
        assert_eq!(
            fetch_count.load(Ordering::SeqCst),
            3,
            "Each unique key should have its own leader"
        );
    }

    #[tokio::test]
    async fn test_new_request_after_completion_becomes_leader() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-object.txt".to_string(),
            etag: None,
            variant: None,
        };

        // First request
        {
            let slot = coalescer.acquire(&key).await;
            assert!(slot.is_leader());
        } // Dropped here

        // Wait for cleanup
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Second request should also be leader (first one completed)
        let slot2 = coalescer.acquire(&key).await;
        assert!(
            slot2.is_leader(),
            "New request after completion should become leader"
        );
    }
}
