//! Cache statistics types
//!
//! This module provides structures for tracking cache performance metrics:
//! - `CacheStats`: Aggregate statistics (hits, misses, evictions, sizes)
//! - `BucketCacheStats`: Per-bucket statistics tracking

use serde::Serialize;
use std::collections::HashMap;

/// Cache statistics for monitoring and metrics
#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions (due to size/TTL)
    pub evictions: u64,
    /// Current cache size in bytes
    pub current_size_bytes: u64,
    /// Current number of items in cache
    pub current_item_count: u64,
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
}

impl CacheStats {
    /// Calculate hit rate (hits / total requests)
    /// Returns 0.0 if there are no requests
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Per-bucket cache statistics tracking
#[derive(Debug, Clone, Default)]
pub struct BucketCacheStats {
    /// Map of bucket name to cache statistics
    stats: HashMap<String, CacheStats>,
}

impl BucketCacheStats {
    /// Create a new BucketCacheStats instance
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    /// Check if the stats map is empty
    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }

    /// Set statistics for a specific bucket
    pub fn set(&mut self, bucket_name: String, stats: CacheStats) {
        self.stats.insert(bucket_name, stats);
    }

    /// Get statistics for a specific bucket
    /// Returns None if the bucket is not found
    pub fn get(&self, bucket_name: &str) -> Option<&CacheStats> {
        self.stats.get(bucket_name)
    }

    /// Aggregate statistics across all buckets
    pub fn aggregate(&self) -> CacheStats {
        let mut aggregated = CacheStats::default();

        for stats in self.stats.values() {
            aggregated.hits += stats.hits;
            aggregated.misses += stats.misses;
            aggregated.evictions += stats.evictions;
            aggregated.current_size_bytes += stats.current_size_bytes;
            aggregated.current_item_count += stats.current_item_count;
            // max_size_bytes is not summed, as it represents the total limit
            if stats.max_size_bytes > aggregated.max_size_bytes {
                aggregated.max_size_bytes = stats.max_size_bytes;
            }
        }

        aggregated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_cache_stats_struct() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            evictions: 10,
            current_size_bytes: 1024,
            current_item_count: 5,
            max_size_bytes: 10240,
        };
        assert_eq!(stats.hits, 100);
    }

    #[test]
    fn test_cache_stats_contains_hits() {
        let stats = CacheStats::default();
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_cache_stats_contains_misses() {
        let stats = CacheStats::default();
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_stats_contains_evictions() {
        let stats = CacheStats::default();
        assert_eq!(stats.evictions, 0);
    }

    #[test]
    fn test_cache_stats_contains_current_size_bytes() {
        let stats = CacheStats::default();
        assert_eq!(stats.current_size_bytes, 0);
    }

    #[test]
    fn test_cache_stats_contains_current_item_count() {
        let stats = CacheStats::default();
        assert_eq!(stats.current_item_count, 0);
    }

    #[test]
    fn test_cache_stats_contains_max_size_bytes() {
        let stats = CacheStats::default();
        assert_eq!(stats.max_size_bytes, 0);
    }

    #[test]
    fn test_cache_stats_implements_clone_trait() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            evictions: 10,
            current_size_bytes: 1024,
            current_item_count: 5,
            max_size_bytes: 10240,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.hits, stats.hits);
        assert_eq!(cloned.misses, stats.misses);
    }

    #[test]
    fn test_cache_stats_can_calculate_hit_rate() {
        let stats = CacheStats {
            hits: 75,
            misses: 25,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        };
        let hit_rate = stats.hit_rate();
        assert!(hit_rate > 0.0 && hit_rate <= 1.0);
    }

    #[test]
    fn test_hit_rate_formula() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        };
        let expected = 80.0 / (80.0 + 20.0);
        assert_eq!(stats.hit_rate(), expected);
    }

    #[test]
    fn test_hit_rate_zero_when_no_requests() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_hit_rate_one_when_all_hits() {
        let stats = CacheStats {
            hits: 100,
            misses: 0,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        };
        assert_eq!(stats.hit_rate(), 1.0);
    }

    #[test]
    fn test_hit_rate_zero_when_all_misses() {
        let stats = CacheStats {
            hits: 0,
            misses: 100,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        };
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_hit_rate_half_when_fifty_percent() {
        let stats = CacheStats {
            hits: 50,
            misses: 50,
            evictions: 0,
            current_size_bytes: 0,
            current_item_count: 0,
            max_size_bytes: 0,
        };
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[test]
    fn test_cache_stats_implements_serialize_trait() {
        let stats = CacheStats::default();
        let _serialized = serde_json::to_string(&stats);
    }

    #[test]
    fn test_cache_stats_serializes_to_json() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            evictions: 10,
            current_size_bytes: 1024,
            current_item_count: 5,
            max_size_bytes: 10240,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("hits"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_json_includes_all_fields() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            evictions: 10,
            current_size_bytes: 1024,
            current_item_count: 5,
            max_size_bytes: 10240,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("hits"));
        assert!(json.contains("misses"));
        assert!(json.contains("evictions"));
        assert!(json.contains("current_size_bytes"));
        assert!(json.contains("current_item_count"));
        assert!(json.contains("max_size_bytes"));
    }

    #[test]
    fn test_can_create_bucket_cache_stats_struct() {
        let bucket_stats = BucketCacheStats::new();
        assert!(bucket_stats.is_empty());
    }

    #[test]
    fn test_bucket_cache_stats_maps_bucket_to_stats() {
        let mut bucket_stats = BucketCacheStats::new();
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            evictions: 10,
            current_size_bytes: 1024,
            current_item_count: 5,
            max_size_bytes: 10240,
        };
        bucket_stats.set("products".to_string(), stats.clone());

        let retrieved = bucket_stats.get("products");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().hits, 100);
    }

    #[test]
    fn test_can_aggregate_stats_across_all_buckets() {
        let mut bucket_stats = BucketCacheStats::new();

        bucket_stats.set(
            "products".to_string(),
            CacheStats {
                hits: 100,
                misses: 50,
                evictions: 10,
                current_size_bytes: 1024,
                current_item_count: 5,
                max_size_bytes: 10240,
            },
        );

        bucket_stats.set(
            "images".to_string(),
            CacheStats {
                hits: 200,
                misses: 100,
                evictions: 20,
                current_size_bytes: 2048,
                current_item_count: 10,
                max_size_bytes: 20480,
            },
        );

        let aggregated = bucket_stats.aggregate();
        assert_eq!(aggregated.hits, 300);
        assert_eq!(aggregated.misses, 150);
        assert_eq!(aggregated.evictions, 30);
    }

    #[test]
    fn test_can_retrieve_stats_for_specific_bucket() {
        let mut bucket_stats = BucketCacheStats::new();
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            evictions: 10,
            current_size_bytes: 1024,
            current_item_count: 5,
            max_size_bytes: 10240,
        };
        bucket_stats.set("products".to_string(), stats.clone());

        let retrieved = bucket_stats.get("products");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().hits, 100);
    }

    #[test]
    fn test_returns_empty_stats_for_unknown_bucket() {
        let bucket_stats = BucketCacheStats::new();
        let retrieved = bucket_stats.get("unknown");
        assert!(retrieved.is_none());
    }
}
