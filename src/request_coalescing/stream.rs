//! Streaming Request Coalescer
//!
//! Real-time streaming coalescing for large file downloads.
//! Unlike the basic RequestCoalescer that waits for complete download,
//! this streams chunks to followers as the leader downloads.
//!
//! Benefits:
//! - No head-of-line blocking for large files
//! - Followers receive first byte quickly
//! - Zero-copy sharing using bytes::Bytes
//!
//! Error handling:
//! - If a follower is too slow (Lagged), request is rejected with 503

use bytes::Bytes;
use pingora_http::ResponseHeader;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::cache::CacheKey;

/// Buffer size for the broadcast channel
/// Larger = more tolerance for slow consumers, but more memory
const BROADCAST_BUFFER_SIZE: usize = 64;

/// Streaming request coalescer
/// Tracks in-flight S3 requests and streams data to all concurrent requesters
#[derive(Debug, Clone)]
pub struct StreamingCoalescer {
    /// Map of in-flight streams: key -> broadcast sender
    streams: Arc<Mutex<HashMap<String, broadcast::Sender<StreamMessage>>>>,
}

impl StreamingCoalescer {
    /// Create a new streaming coalescer
    pub fn new() -> Self {
        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Acquire a streaming slot for a request
    ///
    /// Returns `StreamingSlot::Leader` if this is the first request for the key.
    /// The leader should fetch from S3 and broadcast chunks via the StreamLeader.
    ///
    /// Returns `StreamingSlot::Follower` with a receiver to stream chunks in real-time.
    pub fn acquire(&self, key: &CacheKey) -> StreamingSlot {
        let key_str = Self::cache_key_to_string(key);

        let mut streams = self.streams.lock().unwrap();

        // Check if there's already an in-flight stream
        if let Some(sender) = streams.get(&key_str) {
            // Subscribe to the existing stream
            let receiver = sender.subscribe();
            return StreamingSlot::Follower(receiver);
        }

        // We're the first request - become the leader
        let (tx, _rx) = broadcast::channel(BROADCAST_BUFFER_SIZE);
        streams.insert(key_str.clone(), tx.clone());

        StreamingSlot::Leader(StreamLeader {
            tx,
            key: key_str,
            streams: Arc::clone(&self.streams),
        })
    }

    /// Get current number of in-flight streams
    pub fn in_flight_count(&self) -> usize {
        self.streams.lock().unwrap().len()
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
}

impl Default for StreamingCoalescer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of acquiring a streaming slot
#[derive(Debug)]
pub enum StreamingSlot {
    /// This is the first request (leader) - fetch from S3 and broadcast chunks.
    Leader(StreamLeader),
    /// Another request is in-flight - receive chunks in real-time.
    Follower(broadcast::Receiver<StreamMessage>),
}

impl StreamingSlot {
    /// Check if this is the leader
    pub fn is_leader(&self) -> bool {
        matches!(self, StreamingSlot::Leader(_))
    }

    /// Check if this is a follower
    pub fn is_follower(&self) -> bool {
        matches!(self, StreamingSlot::Follower(_))
    }
}

/// Messages broadcast from leader to followers
#[derive(Debug, Clone)]
pub enum StreamMessage {
    /// Response headers from S3
    Headers(Arc<ResponseHeader>),
    /// A data chunk
    Chunk(Bytes),
    /// Stream completed successfully
    Done,
    /// Stream failed with error
    Error(String),
}

/// Leader handle for broadcasting stream data
///
/// When dropped, automatically cleans up the stream from the coalescer map.
#[derive(Debug)]
pub struct StreamLeader {
    tx: broadcast::Sender<StreamMessage>,
    key: String,
    streams: Arc<Mutex<HashMap<String, broadcast::Sender<StreamMessage>>>>,
}

impl StreamLeader {
    /// Broadcast response headers to all followers
    pub fn send_headers(
        &self,
        headers: ResponseHeader,
    ) -> Result<usize, broadcast::error::SendError<StreamMessage>> {
        self.tx.send(StreamMessage::Headers(Arc::new(headers)))
    }

    /// Broadcast a data chunk to all followers
    pub fn send_chunk(
        &self,
        data: Bytes,
    ) -> Result<usize, broadcast::error::SendError<StreamMessage>> {
        self.tx.send(StreamMessage::Chunk(data))
    }

    /// Signal successful completion
    pub fn finish(self) -> Result<usize, broadcast::error::SendError<StreamMessage>> {
        // Cleanup happens in Drop
        self.tx.send(StreamMessage::Done)
    }

    /// Signal an error occurred
    pub fn send_error(
        self,
        err: String,
    ) -> Result<usize, broadcast::error::SendError<StreamMessage>> {
        // Cleanup happens in Drop
        self.tx.send(StreamMessage::Error(err))
    }

    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Drop for StreamLeader {
    fn drop(&mut self) {
        // Clean up the stream from the map
        if let Ok(mut streams) = self.streams.lock() {
            streams.remove(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn test_cache_key(object_key: &str) -> CacheKey {
        CacheKey {
            bucket: "test-bucket".to_string(),
            object_key: object_key.to_string(),
            etag: None,
            variant: None,
        }
    }

    #[test]
    fn test_first_request_becomes_leader() {
        let coalescer = StreamingCoalescer::new();
        let key = test_cache_key("test-object.txt");

        let slot = coalescer.acquire(&key);
        assert!(slot.is_leader());
        assert_eq!(coalescer.in_flight_count(), 1);
    }

    #[test]
    fn test_second_request_becomes_follower() {
        let coalescer = StreamingCoalescer::new();
        let key = test_cache_key("test-object.txt");

        let slot1 = coalescer.acquire(&key);
        let slot2 = coalescer.acquire(&key);

        assert!(slot1.is_leader());
        assert!(slot2.is_follower());
        assert_eq!(coalescer.in_flight_count(), 1);
    }

    #[test]
    fn test_different_keys_both_leaders() {
        let coalescer = StreamingCoalescer::new();
        let key1 = test_cache_key("object1.txt");
        let key2 = test_cache_key("object2.txt");

        let slot1 = coalescer.acquire(&key1);
        let slot2 = coalescer.acquire(&key2);

        assert!(slot1.is_leader());
        assert!(slot2.is_leader());
        assert_eq!(coalescer.in_flight_count(), 2);
    }

    #[tokio::test]
    async fn test_follower_receives_chunks() {
        let coalescer = StreamingCoalescer::new();
        let key = test_cache_key("test-object.txt");

        let slot1 = coalescer.acquire(&key);
        let slot2 = coalescer.acquire(&key);

        let leader = match slot1 {
            StreamingSlot::Leader(l) => l,
            _ => panic!("Expected leader"),
        };

        let mut follower = match slot2 {
            StreamingSlot::Follower(r) => r,
            _ => panic!("Expected follower"),
        };

        // Leader sends chunks
        leader.send_chunk(Bytes::from("chunk1")).unwrap();
        leader.send_chunk(Bytes::from("chunk2")).unwrap();
        leader.finish().unwrap();

        // Follower receives them in order
        let msg1 = follower.recv().await.unwrap();
        assert!(matches!(msg1, StreamMessage::Chunk(b) if b == Bytes::from("chunk1")));

        let msg2 = follower.recv().await.unwrap();
        assert!(matches!(msg2, StreamMessage::Chunk(b) if b == Bytes::from("chunk2")));

        let msg3 = follower.recv().await.unwrap();
        assert!(matches!(msg3, StreamMessage::Done));
    }

    #[tokio::test]
    async fn test_multiple_followers_receive_all_chunks() {
        let coalescer = StreamingCoalescer::new();
        let key = test_cache_key("test-object.txt");

        let slot = coalescer.acquire(&key);
        let leader = match slot {
            StreamingSlot::Leader(l) => l,
            _ => panic!("Expected leader"),
        };

        // Create 3 followers
        let mut followers: Vec<_> = (0..3)
            .map(|_| match coalescer.acquire(&key) {
                StreamingSlot::Follower(r) => r,
                _ => panic!("Expected follower"),
            })
            .collect();

        let received = Arc::new(AtomicUsize::new(0));

        // Spawn receivers
        let mut handles = vec![];
        for mut follower in followers.drain(..) {
            let received = Arc::clone(&received);
            handles.push(tokio::spawn(async move {
                let mut chunk_count = 0;
                loop {
                    match follower.recv().await {
                        Ok(StreamMessage::Chunk(_)) => chunk_count += 1,
                        Ok(StreamMessage::Done) => break,
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
                received.fetch_add(chunk_count, Ordering::SeqCst);
            }));
        }

        // Give followers time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Leader sends 5 chunks
        for i in 0..5 {
            leader
                .send_chunk(Bytes::from(format!("chunk{}", i)))
                .unwrap();
        }
        leader.finish().unwrap();

        // Wait for all receivers
        for handle in handles {
            handle.await.unwrap();
        }

        // All 3 followers should have received all 5 chunks = 15 total
        assert_eq!(received.load(Ordering::SeqCst), 15);
    }

    #[test]
    fn test_cleanup_on_leader_drop() {
        let coalescer = StreamingCoalescer::new();
        let key = test_cache_key("test-object.txt");

        {
            let slot = coalescer.acquire(&key);
            assert!(slot.is_leader());
            assert_eq!(coalescer.in_flight_count(), 1);
        } // Leader dropped here

        assert_eq!(coalescer.in_flight_count(), 0);

        // New request should become leader
        let slot = coalescer.acquire(&key);
        assert!(slot.is_leader());
    }

    #[tokio::test]
    async fn test_error_broadcast() {
        let coalescer = StreamingCoalescer::new();
        let key = test_cache_key("test-object.txt");

        let slot1 = coalescer.acquire(&key);
        let slot2 = coalescer.acquire(&key);

        let leader = match slot1 {
            StreamingSlot::Leader(l) => l,
            _ => panic!("Expected leader"),
        };

        let mut follower = match slot2 {
            StreamingSlot::Follower(r) => r,
            _ => panic!("Expected follower"),
        };

        // Leader encounters error
        leader
            .send_error("S3 connection failed".to_string())
            .unwrap();

        // Follower receives error
        let msg = follower.recv().await.unwrap();
        assert!(matches!(msg, StreamMessage::Error(e) if e == "S3 connection failed"));
    }
}
