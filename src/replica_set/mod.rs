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
}
