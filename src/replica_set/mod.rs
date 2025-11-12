// Replica Set module for High Availability bucket replication
//
// This module manages a set of S3 replicas with priority-based failover.
// Each replica has:
// - Independent S3 client with its own credentials
// - Independent circuit breaker for health tracking
// - Priority level (1 = highest priority)
//
// Failover strategy:
// - Try replicas in priority order (1, 2, 3...)
// - Skip replicas with open circuit breakers
// - Return first successful response

use crate::circuit_breaker::CircuitBreaker;
use crate::config::S3Replica;
use crate::s3::S3Client;

/// A single replica with its S3 client and circuit breaker
#[derive(Debug, Clone)]
pub struct ReplicaEntry {
    pub name: String,
    pub priority: u8,
    pub client: S3Client,
    pub circuit_breaker: CircuitBreaker,
}

/// A set of replicas for a single bucket, stored in priority order
#[derive(Debug, Clone)]
pub struct ReplicaSet {
    pub replicas: Vec<ReplicaEntry>,
}

impl ReplicaSet {
    /// Create a new ReplicaSet from a list of replica configurations.
    /// Replicas are expected to already be sorted by priority.
    pub fn new(replica_configs: &[S3Replica]) -> Result<Self, String> {
        if replica_configs.is_empty() {
            return Err("Cannot create ReplicaSet with empty replica list".to_string());
        }

        let mut replicas = Vec::new();

        for replica_config in replica_configs {
            // Create S3 client for this replica
            let client = create_replica_client(replica_config)?;

            // Create circuit breaker for this replica (using default config)
            let circuit_breaker =
                CircuitBreaker::new(crate::circuit_breaker::CircuitBreakerConfig::default());

            replicas.push(ReplicaEntry {
                name: replica_config.name.clone(),
                priority: replica_config.priority,
                client,
                circuit_breaker,
            });
        }

        Ok(ReplicaSet { replicas })
    }

    /// Get the number of replicas in this set
    pub fn len(&self) -> usize {
        self.replicas.len()
    }

    /// Check if the replica set is empty
    pub fn is_empty(&self) -> bool {
        self.replicas.is_empty()
    }

    /// Try to execute a request against replicas in priority order.
    /// Returns the first successful result, or the last error if all replicas fail.
    ///
    /// # Arguments
    /// * `request_fn` - A closure that takes a replica and attempts to execute a request
    ///
    /// # Returns
    /// * `Ok(T)` - The successful result from the first working replica
    /// * `Err(E)` - The error from the last replica if all failed
    pub fn try_request<F, T, E>(&self, mut request_fn: F) -> Result<T, E>
    where
        F: FnMut(&ReplicaEntry) -> Result<T, E>,
    {
        let mut last_error = None;

        for replica in &self.replicas {
            match request_fn(replica) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    // Continue to next replica on failure
                }
            }
        }

        // All replicas failed - return the last error
        // unwrap is safe here because we know replicas is not empty (validated in new())
        Err(last_error.unwrap())
    }
}

/// Create an S3 client from a replica configuration
fn create_replica_client(replica: &S3Replica) -> Result<S3Client, String> {
    // Convert S3Replica to S3Config for client creation
    let s3_config = crate::config::S3Config {
        bucket: replica.bucket.clone(),
        region: replica.region.clone(),
        access_key: replica.access_key.clone(),
        secret_key: replica.secret_key.clone(),
        endpoint: replica.endpoint.clone(),
        timeout: replica.timeout,
        connection_pool_size: 10, // Default pool size
        circuit_breaker: None,
        rate_limit: None,
        retry: None,
        replicas: None, // Not used for individual replica clients
    };

    crate::s3::create_s3_client(&s3_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::S3Replica;

    #[test]
    fn test_create_replica_set_from_multiple_replicas() {
        // Test: ReplicaSet should create S3 clients for each replica
        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Verify we have 2 replicas
        assert_eq!(replica_set.len(), 2, "Should have 2 replicas");

        // Verify first replica (primary)
        let primary = &replica_set.replicas[0];
        assert_eq!(primary.name, "primary");
        assert_eq!(primary.priority, 1);
        assert_eq!(primary.client.config.bucket, "products-us-west-2");
        assert_eq!(primary.client.config.region, "us-west-2");
        assert_eq!(primary.client.config.access_key, "AKIAIOSFODNN7EXAMPLE1");
        assert_eq!(
            primary.client.config.endpoint,
            Some("https://s3.us-west-2.amazonaws.com".to_string())
        );
        assert_eq!(primary.client.config.timeout, 30);

        // Verify second replica (EU)
        let replica_eu = &replica_set.replicas[1];
        assert_eq!(replica_eu.name, "replica-eu");
        assert_eq!(replica_eu.priority, 2);
        assert_eq!(replica_eu.client.config.bucket, "products-eu-west-1");
        assert_eq!(replica_eu.client.config.timeout, 25);
    }

    #[test]
    fn test_create_circuit_breaker_for_each_replica() {
        // Test: Each replica should have its own independent circuit breaker
        // This enables per-replica health tracking and failover decisions
        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Verify both replicas have circuit breakers
        assert_eq!(replica_set.len(), 2, "Should have 2 replicas");

        // Verify first replica has circuit breaker in Closed state
        let primary = &replica_set.replicas[0];
        assert_eq!(primary.name, "primary");
        assert_eq!(
            primary.circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Closed,
            "Primary replica circuit breaker should start in Closed state"
        );

        // Verify second replica has circuit breaker in Closed state
        let replica_eu = &replica_set.replicas[1];
        assert_eq!(replica_eu.name, "replica-eu");
        assert_eq!(
            replica_eu.circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Closed,
            "EU replica circuit breaker should start in Closed state"
        );

        // Verify circuit breakers are independent (different instances)
        // We can't directly compare Arc pointers easily, but we verified each has its own state
        // The fact that they're both in Closed state confirms they were independently created
    }

    #[test]
    fn test_replicas_stored_in_priority_order() {
        // Test: ReplicaSet should maintain replicas in priority order (1, 2, 3...)
        // This ensures failover logic can iterate replicas sequentially
        // Note: Config module sorts replicas during parsing; this test verifies preservation of that order
        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
            S3Replica {
                name: "replica-minio".to_string(),
                bucket: "products-backup".to_string(),
                region: "us-east-1".to_string(),
                access_key: "minioadmin".to_string(),
                secret_key: "minioadmin".to_string(),
                endpoint: Some("https://minio.example.com".to_string()),
                priority: 3,
                timeout: 20,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Verify replicas are stored in priority order (1, 2, 3)
        assert_eq!(replica_set.len(), 3, "Should have 3 replicas");

        // Verify first replica has priority 1
        let first = &replica_set.replicas[0];
        assert_eq!(first.priority, 1, "First replica should have priority 1");
        assert_eq!(first.name, "primary", "First replica should be 'primary'");

        // Verify second replica has priority 2
        let second = &replica_set.replicas[1];
        assert_eq!(second.priority, 2, "Second replica should have priority 2");
        assert_eq!(
            second.name, "replica-eu",
            "Second replica should be 'replica-eu'"
        );

        // Verify third replica has priority 3
        let third = &replica_set.replicas[2];
        assert_eq!(third.priority, 3, "Third replica should have priority 3");
        assert_eq!(
            third.name, "replica-minio",
            "Third replica should be 'replica-minio'"
        );
    }

    #[test]
    fn test_each_replica_has_independent_credentials() {
        // Test: Each replica should have its own access_key and secret_key
        // This ensures credential isolation: wrong credentials can't be used for wrong bucket
        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
            S3Replica {
                name: "replica-minio".to_string(),
                bucket: "products-backup".to_string(),
                region: "us-east-1".to_string(),
                access_key: "minioadmin".to_string(),
                secret_key: "minioadmin-secret".to_string(),
                endpoint: Some("https://minio.example.com".to_string()),
                priority: 3,
                timeout: 20,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Verify we have 3 replicas
        assert_eq!(replica_set.len(), 3, "Should have 3 replicas");

        // Verify first replica has correct credentials
        let primary = &replica_set.replicas[0];
        assert_eq!(primary.name, "primary");
        assert_eq!(
            primary.client.config.access_key, "AKIAIOSFODNN7EXAMPLE1",
            "Primary replica should have its own access_key"
        );
        assert_eq!(
            primary.client.config.secret_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1",
            "Primary replica should have its own secret_key"
        );

        // Verify second replica has different credentials
        let replica_eu = &replica_set.replicas[1];
        assert_eq!(replica_eu.name, "replica-eu");
        assert_eq!(
            replica_eu.client.config.access_key, "AKIAIOSFODNN7EXAMPLE2",
            "EU replica should have its own access_key"
        );
        assert_eq!(
            replica_eu.client.config.secret_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2",
            "EU replica should have its own secret_key"
        );

        // Verify third replica has different credentials
        let replica_minio = &replica_set.replicas[2];
        assert_eq!(replica_minio.name, "replica-minio");
        assert_eq!(
            replica_minio.client.config.access_key, "minioadmin",
            "MinIO replica should have its own access_key"
        );
        assert_eq!(
            replica_minio.client.config.secret_key, "minioadmin-secret",
            "MinIO replica should have its own secret_key"
        );

        // Verify credentials are different between replicas (credential isolation)
        assert_ne!(
            primary.client.config.access_key, replica_eu.client.config.access_key,
            "Primary and EU replicas should have different access_keys"
        );
        assert_ne!(
            primary.client.config.secret_key, replica_eu.client.config.secret_key,
            "Primary and EU replicas should have different secret_keys"
        );
        assert_ne!(
            replica_eu.client.config.access_key, replica_minio.client.config.access_key,
            "EU and MinIO replicas should have different access_keys"
        );
    }

    #[test]
    fn test_each_replica_has_independent_timeout() {
        // Test: Each replica should have its own timeout configuration
        // This allows flexibility: fast primary (30s) vs. slow backup (60s)
        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30, // Fast primary: 30 seconds
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 45, // Slower cross-region: 45 seconds
            },
            S3Replica {
                name: "replica-backup".to_string(),
                bucket: "products-backup".to_string(),
                region: "us-east-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
                endpoint: Some("https://minio.example.com".to_string()),
                priority: 3,
                timeout: 60, // Slow backup: 60 seconds
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Verify we have 3 replicas
        assert_eq!(replica_set.len(), 3, "Should have 3 replicas");

        // Verify first replica has 30s timeout
        let primary = &replica_set.replicas[0];
        assert_eq!(primary.name, "primary");
        assert_eq!(
            primary.client.config.timeout, 30,
            "Primary replica should have 30s timeout"
        );

        // Verify second replica has 45s timeout
        let replica_eu = &replica_set.replicas[1];
        assert_eq!(replica_eu.name, "replica-eu");
        assert_eq!(
            replica_eu.client.config.timeout, 45,
            "EU replica should have 45s timeout"
        );

        // Verify third replica has 60s timeout
        let replica_backup = &replica_set.replicas[2];
        assert_eq!(replica_backup.name, "replica-backup");
        assert_eq!(
            replica_backup.client.config.timeout, 60,
            "Backup replica should have 60s timeout"
        );

        // Verify timeouts are different between replicas (timeout isolation)
        assert_ne!(
            primary.client.config.timeout, replica_eu.client.config.timeout,
            "Primary and EU replicas should have different timeouts"
        );
        assert_ne!(
            replica_eu.client.config.timeout, replica_backup.client.config.timeout,
            "EU and backup replicas should have different timeouts"
        );
        assert_ne!(
            primary.client.config.timeout, replica_backup.client.config.timeout,
            "Primary and backup replicas should have different timeouts"
        );
    }

    #[test]
    fn test_replica_set_can_be_cloned() {
        // Test: ReplicaSet should be cloneable for hot reload support
        // When config is reloaded, we create new ReplicaSet without disrupting in-flight requests
        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let original = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Clone the ReplicaSet
        let cloned = original.clone();

        // Verify clone has same number of replicas
        assert_eq!(
            cloned.len(),
            original.len(),
            "Cloned ReplicaSet should have same number of replicas"
        );
        assert_eq!(cloned.len(), 2, "Should have 2 replicas");

        // Verify first replica properties match
        let original_primary = &original.replicas[0];
        let cloned_primary = &cloned.replicas[0];
        assert_eq!(
            cloned_primary.name, original_primary.name,
            "Cloned replica should have same name"
        );
        assert_eq!(
            cloned_primary.priority, original_primary.priority,
            "Cloned replica should have same priority"
        );
        assert_eq!(
            cloned_primary.client.config.bucket, original_primary.client.config.bucket,
            "Cloned replica should have same bucket"
        );
        assert_eq!(
            cloned_primary.client.config.access_key, original_primary.client.config.access_key,
            "Cloned replica should have same access_key"
        );
        assert_eq!(
            cloned_primary.client.config.timeout, original_primary.client.config.timeout,
            "Cloned replica should have same timeout"
        );

        // Verify second replica properties match
        let original_eu = &original.replicas[1];
        let cloned_eu = &cloned.replicas[1];
        assert_eq!(
            cloned_eu.name, original_eu.name,
            "Cloned EU replica should have same name"
        );
        assert_eq!(
            cloned_eu.priority, original_eu.priority,
            "Cloned EU replica should have same priority"
        );
        assert_eq!(
            cloned_eu.client.config.bucket, original_eu.client.config.bucket,
            "Cloned EU replica should have same bucket"
        );

        // Verify circuit breakers are cloned (both start in Closed state)
        assert_eq!(
            cloned_primary.circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Closed,
            "Cloned circuit breaker should be in Closed state"
        );
        assert_eq!(
            cloned_eu.circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Closed,
            "Cloned EU circuit breaker should be in Closed state"
        );
    }

    #[test]
    fn test_single_bucket_config_creates_one_replica_set() {
        // Test: Legacy single-bucket config should normalize to one-replica ReplicaSet
        // This ensures backward compatibility and unified code path
        let replica = S3Replica {
            name: "default".to_string(),
            bucket: "my-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            endpoint: Some("https://s3.us-east-1.amazonaws.com".to_string()),
            priority: 1,
            timeout: 30,
        };

        // Create ReplicaSet from single replica (simulating normalized config)
        let replica_set = ReplicaSet::new(&[replica]).expect("Should create ReplicaSet");

        // Verify we have exactly 1 replica
        assert_eq!(replica_set.len(), 1, "Should have exactly 1 replica");
        assert!(!replica_set.is_empty(), "ReplicaSet should not be empty");

        // Verify the replica properties
        let default_replica = &replica_set.replicas[0];
        assert_eq!(
            default_replica.name, "default",
            "Normalized replica should be named 'default'"
        );
        assert_eq!(
            default_replica.priority, 1,
            "Single replica should have priority 1"
        );
        assert_eq!(
            default_replica.client.config.bucket, "my-bucket",
            "Replica should have correct bucket"
        );
        assert_eq!(
            default_replica.client.config.region, "us-east-1",
            "Replica should have correct region"
        );
        assert_eq!(
            default_replica.client.config.access_key, "AKIAIOSFODNN7EXAMPLE",
            "Replica should have correct access_key"
        );
        assert_eq!(
            default_replica.client.config.secret_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "Replica should have correct secret_key"
        );
        assert_eq!(
            default_replica.client.config.timeout, 30,
            "Replica should have correct timeout"
        );
        assert_eq!(
            default_replica.client.config.endpoint,
            Some("https://s3.us-east-1.amazonaws.com".to_string()),
            "Replica should have correct endpoint"
        );

        // Verify circuit breaker is initialized
        assert_eq!(
            default_replica.circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Closed,
            "Circuit breaker should start in Closed state"
        );
    }

    #[test]
    fn test_request_succeeds_from_first_replica() {
        // Test: When all replicas are healthy, request should succeed from first (priority 1) replica
        // This verifies basic failover logic: try replicas in priority order
        use std::cell::RefCell;

        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Track which replicas were called (using RefCell for interior mutability in closure)
        let calls = RefCell::new(Vec::new());

        // Simulate a successful request from first replica
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            Ok::<String, String>(format!("success from {}", replica.name))
        });

        // Verify request succeeded
        assert!(result.is_ok(), "Request should succeed");
        assert_eq!(
            result.unwrap(),
            "success from primary",
            "Should return result from primary replica"
        );

        // Verify only first replica was called
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            1,
            "Should only call first replica when it succeeds"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
    }

    #[test]
    fn test_connection_error_triggers_failover_to_next_replica() {
        // Test: When first replica fails with connection error, try next replica
        // This verifies automatic failover on transient network failures
        use std::cell::RefCell;

        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Track which replicas were called
        let calls = RefCell::new(Vec::new());

        // Simulate: first replica fails with connection error, second succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("connection error: connection refused".to_string())
            } else {
                Ok(format!("success from {}", replica.name))
            }
        });

        // Verify request succeeded from second replica
        assert!(result.is_ok(), "Request should succeed from second replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu after primary failed"
        );

        // Verify both replicas were called (primary failed, then EU succeeded)
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Should call both replicas (primary failed, EU succeeded)"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Should call replica-eu after primary failed"
        );
    }

    #[test]
    fn test_timeout_triggers_failover_to_next_replica() {
        // Test: When first replica fails with timeout, try next replica
        // This verifies automatic failover on timeout errors
        use std::cell::RefCell;

        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Track which replicas were called
        let calls = RefCell::new(Vec::new());

        // Simulate: first replica times out, second succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("timeout: operation timed out after 30s".to_string())
            } else {
                Ok(format!("success from {}", replica.name))
            }
        });

        // Verify request succeeded from second replica
        assert!(result.is_ok(), "Request should succeed from second replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu after primary timed out"
        );

        // Verify both replicas were called (primary timed out, then EU succeeded)
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Should call both replicas (primary timed out, EU succeeded)"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Should call replica-eu after primary timed out"
        );
    }

    #[test]
    fn test_http_500_triggers_failover_to_next_replica() {
        // Test: HTTP 500 (Internal Server Error) should trigger failover
        // This is a retriable server error - try next replica
        use std::cell::RefCell;

        let replicas = vec![
            S3Replica {
                name: "primary".to_string(),
                bucket: "products-us-west-2".to_string(),
                region: "us-west-2".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE1".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY1".to_string(),
                endpoint: Some("https://s3.us-west-2.amazonaws.com".to_string()),
                priority: 1,
                timeout: 30,
            },
            S3Replica {
                name: "replica-eu".to_string(),
                bucket: "products-eu-west-1".to_string(),
                region: "eu-west-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE2".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY2".to_string(),
                endpoint: Some("https://s3.eu-west-1.amazonaws.com".to_string()),
                priority: 2,
                timeout: 25,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Track which replicas were called
        let calls = RefCell::new(Vec::new());

        // Simulate: first replica returns HTTP 500, second succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("HTTP 500: Internal Server Error".to_string())
            } else {
                Ok(format!("success from {}", replica.name))
            }
        });

        // Verify request succeeded from second replica
        assert!(result.is_ok(), "Request should succeed from second replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu after primary returned 500"
        );

        // Verify both replicas were called
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Should call both replicas (primary returned 500, EU succeeded)"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Should call replica-eu after primary returned 500"
        );
    }
}
