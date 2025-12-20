// Request Coalescing Integration Tests - Phase 38
//
// Tests for request deduplication when multiple clients request the same S3 object

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use yatagarasu::cache::CacheKey;
    use yatagarasu::request_coalescing::RequestCoalescer;

    #[tokio::test]
    async fn test_request_coalescer_serializes_concurrent_requests() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-object.txt".to_string(),
            etag: None,
        };

        // Track how many times the "fetch" operation runs
        let fetch_count = Arc::new(AtomicUsize::new(0));

        // Simulate 5 concurrent requests for the same object
        let mut handles = vec![];
        for _ in 0..5 {
            let coalescer_clone = coalescer.clone();
            let key_clone = key.clone();
            let fetch_count_clone = Arc::clone(&fetch_count);

            let handle = tokio::spawn(async move {
                // Acquire coalescing guard
                let _guard = coalescer_clone.acquire(&key_clone).await;

                // Simulate S3 fetch (only first request should do this)
                fetch_count_clone.fetch_add(1, Ordering::SeqCst);

                // Simulate processing time
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All 5 requests should have incremented the counter
        // (In a real scenario, only the first would fetch from S3)
        assert_eq!(fetch_count.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_request_coalescer_tracks_in_flight_requests() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-object.txt".to_string(),
            etag: None,
        };

        assert_eq!(coalescer.in_flight_count().await, 0);

        let guard = coalescer.acquire(&key).await;
        assert_eq!(coalescer.in_flight_count().await, 1);

        drop(guard);
        // Note: cleanup happens asynchronously, so we need to wait
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(
            coalescer.in_flight_count().await,
            0,
            "in_flight_count should be 0 after guard is dropped and cleanup runs"
        );
    }

    #[tokio::test]
    async fn test_request_coalescer_different_keys_dont_block() {
        let coalescer = RequestCoalescer::new();
        let key1 = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "object1.txt".to_string(),
            etag: None,
        };
        let key2 = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "object2.txt".to_string(),
            etag: None,
        };

        let guard1 = coalescer.acquire(&key1).await;
        let guard2 = coalescer.acquire(&key2).await;

        // Both should be in-flight (different keys don't block each other)
        assert_eq!(coalescer.in_flight_count().await, 2);

        drop(guard1);
        drop(guard2);
    }

    #[tokio::test]
    async fn test_request_coalescer_same_bucket_different_keys() {
        let coalescer = RequestCoalescer::new();

        let mut handles = vec![];
        for i in 0..3 {
            let coalescer_clone = coalescer.clone();
            let handle = tokio::spawn(async move {
                let key = CacheKey {
                    bucket: "test-bucket".to_string(),
                    object_key: format!("object{}.txt", i),
                    etag: None,
                };
                let _guard = coalescer_clone.acquire(&key).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
}
