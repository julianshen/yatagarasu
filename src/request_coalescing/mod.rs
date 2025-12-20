// Request Coalescing Module - Phase 38
//
// Deduplicates concurrent S3 requests for the same object.
// When multiple clients request the same S3 object simultaneously:
// - First request (leader): Fetches from S3, caches result, signals completion
// - Subsequent requests (followers): Wait for leader to complete, then read from cache
// - All requests: Receive the same response (true deduplication)
//
// Expected benefit: 20-30% throughput improvement under high concurrency

use crate::cache::CacheKey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;

/// Request coalescing manager
/// Tracks in-flight S3 requests and deduplicates concurrent requests for the same object
#[derive(Debug, Clone)]
pub struct RequestCoalescer {
    /// Map of in-flight requests: key -> watch sender
    /// When a request completes, it sends on the channel to notify all waiters
    in_flight: Arc<tokio::sync::Mutex<HashMap<String, watch::Sender<bool>>>>,
}

impl RequestCoalescer {
    /// Create a new request coalescer
    pub fn new() -> Self {
        Self {
            in_flight: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Acquire a coalescing slot for a request
    ///
    /// Returns `CoalescingSlot::Leader` if this is the first request for the key.
    /// The leader should fetch from S3, cache the result, then drop the guard.
    ///
    /// Returns `CoalescingSlot::Follower` after waiting for the leader to complete.
    /// The follower should read from cache (the leader already populated it).
    pub async fn acquire(&self, key: &CacheKey) -> CoalescingSlot {
        let key_str = Self::cache_key_to_string(key);

        // Check if there's already an in-flight request
        let receiver = {
            let in_flight = self.in_flight.lock().await;
            in_flight.get(&key_str).map(|sender| sender.subscribe())
        };

        if let Some(mut rx) = receiver {
            // Another request is in-flight - wait for it to complete
            // The leader will send `true` when done
            let _ = rx.wait_for(|&completed| completed).await;
            CoalescingSlot::Follower
        } else {
            // We're the first request - become the leader
            let (tx, _rx) = watch::channel(false);

            {
                let mut in_flight = self.in_flight.lock().await;
                // Double-check: another request might have started while we weren't holding the lock
                if in_flight.contains_key(&key_str) {
                    // Race condition: someone else became leader, become a follower instead
                    drop(in_flight);
                    return Box::pin(self.acquire(key)).await;
                }
                in_flight.insert(key_str.clone(), tx.clone());
            }

            CoalescingSlot::Leader(LeaderGuard {
                key: key_str,
                coalescer: self.clone(),
                sender: tx,
            })
        }
    }

    /// Get current number of in-flight requests
    pub async fn in_flight_count(&self) -> usize {
        self.in_flight.lock().await.len()
    }

    /// Convert a CacheKey to a string for use as a map key
    fn cache_key_to_string(key: &CacheKey) -> String {
        format!(
            "{}:{}:{}",
            key.bucket,
            key.object_key,
            key.etag.as_deref().unwrap_or("")
        )
    }

    /// Remove a key from the in-flight map (called when leader completes)
    async fn remove_in_flight(&self, key: &str) {
        let mut in_flight = self.in_flight.lock().await;
        in_flight.remove(key);
    }
}

impl Default for RequestCoalescer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of acquiring a coalescing slot
#[derive(Debug)]
pub enum CoalescingSlot {
    /// This is the first request (leader) - fetch from S3 and cache the result.
    /// When the guard is dropped, all waiting followers will be notified.
    Leader(LeaderGuard),

    /// Another request was in-flight and has now completed.
    /// The result should be available in cache - read from there.
    Follower,
}

impl CoalescingSlot {
    /// Check if this is the leader (first request that should fetch from S3)
    pub fn is_leader(&self) -> bool {
        matches!(self, CoalescingSlot::Leader(_))
    }

    /// Check if this is a follower (waited for leader, should read from cache)
    pub fn is_follower(&self) -> bool {
        matches!(self, CoalescingSlot::Follower)
    }
}

/// Guard held by the leader request
/// When dropped, notifies all waiting followers that the request is complete
#[derive(Debug)]
pub struct LeaderGuard {
    key: String,
    coalescer: RequestCoalescer,
    sender: watch::Sender<bool>,
}

impl LeaderGuard {
    /// Explicitly mark the request as complete and notify all followers.
    /// This is called automatically when the guard is dropped, but can be
    /// called explicitly if needed.
    pub async fn complete(self) {
        // Send completion signal to all waiting followers
        let _ = self.sender.send(true);
        // Remove from in-flight map
        self.coalescer.remove_in_flight(&self.key).await;
        // Prevent Drop from running (we've already cleaned up)
        std::mem::forget(self);
    }
}

impl Drop for LeaderGuard {
    fn drop(&mut self) {
        // Send completion signal to all waiting followers
        let _ = self.sender.send(true);

        // Spawn a task to clean up the in-flight map
        // We must spawn because Drop is not async
        let coalescer = self.coalescer.clone();
        let key = self.key.clone();
        tokio::spawn(async move {
            coalescer.remove_in_flight(&key).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_first_request_becomes_leader() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: None,
        };

        let slot = coalescer.acquire(&key).await;
        assert!(slot.is_leader(), "First request should be leader");
        assert_eq!(coalescer.in_flight_count().await, 1);
    }

    #[tokio::test]
    async fn test_follower_waits_for_leader() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: None,
        };

        // Track execution order
        let order = Arc::new(AtomicUsize::new(0));

        // Leader acquires first
        let slot = coalescer.acquire(&key).await;
        assert!(slot.is_leader());

        // Spawn a follower that should wait
        let coalescer2 = coalescer.clone();
        let key2 = key.clone();
        let order2 = Arc::clone(&order);
        let follower_handle = tokio::spawn(async move {
            let slot = coalescer2.acquire(&key2).await;
            // Record when follower completes
            order2.fetch_add(1, Ordering::SeqCst);
            slot.is_follower()
        });

        // Give follower time to start waiting
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Follower should still be waiting (order should be 0)
        assert_eq!(
            order.load(Ordering::SeqCst),
            0,
            "Follower should be waiting"
        );

        // Leader completes - this should notify the follower
        drop(slot);

        // Wait for follower to complete
        let is_follower = follower_handle.await.unwrap();
        assert!(is_follower, "Second request should be follower");

        // Follower should have completed after leader
        assert_eq!(
            order.load(Ordering::SeqCst),
            1,
            "Follower should have completed"
        );
    }

    #[tokio::test]
    async fn test_multiple_followers_all_wait() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: None,
        };

        let fetch_count = Arc::new(AtomicUsize::new(0));

        // Leader acquires first
        let slot = coalescer.acquire(&key).await;
        assert!(slot.is_leader());

        // Spawn 5 followers
        let mut handles = vec![];
        for _ in 0..5 {
            let coalescer_clone = coalescer.clone();
            let key_clone = key.clone();
            let fetch_count_clone = Arc::clone(&fetch_count);

            let handle = tokio::spawn(async move {
                let slot = coalescer_clone.acquire(&key_clone).await;
                if slot.is_leader() {
                    // Only leader should "fetch"
                    fetch_count_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        // Give followers time to start waiting
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Simulate leader fetching from S3
        fetch_count.fetch_add(1, Ordering::SeqCst);

        // Leader completes
        drop(slot);

        // Wait for all followers
        for handle in handles {
            handle.await.unwrap();
        }

        // Only the leader should have "fetched"
        assert_eq!(
            fetch_count.load(Ordering::SeqCst),
            1,
            "Only leader should fetch"
        );
    }

    #[tokio::test]
    async fn test_different_keys_dont_block() {
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

        let slot1 = coalescer.acquire(&key1).await;
        let slot2 = coalescer.acquire(&key2).await;

        // Both should be leaders (different keys)
        assert!(slot1.is_leader(), "First key should have leader");
        assert!(slot2.is_leader(), "Second key should also have leader");
        assert_eq!(coalescer.in_flight_count().await, 2);

        drop(slot1);
        drop(slot2);
    }

    #[tokio::test]
    async fn test_cleanup_after_completion() {
        let coalescer = RequestCoalescer::new();
        let key = CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: "test-key".to_string(),
            etag: None,
        };

        {
            let slot = coalescer.acquire(&key).await;
            assert!(slot.is_leader());
            assert_eq!(coalescer.in_flight_count().await, 1);
        } // slot dropped here

        // Give cleanup task time to run
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(
            coalescer.in_flight_count().await,
            0,
            "In-flight count should be 0 after cleanup"
        );

        // New request should become leader
        let slot2 = coalescer.acquire(&key).await;
        assert!(
            slot2.is_leader(),
            "New request should be leader after cleanup"
        );
    }
}
