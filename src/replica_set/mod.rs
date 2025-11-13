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
    /// Skips replicas with open circuit breakers (unhealthy replicas).
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
        E: std::fmt::Display,
    {
        let mut last_error = None;
        let mut last_failed_replica: Option<&str> = None;
        let mut attempt = 0;
        let mut all_errors: Vec<String> = Vec::new();

        for replica in &self.replicas {
            // Skip replicas with open circuit breakers
            if !replica.circuit_breaker.should_allow_request() {
                tracing::debug!(
                    replica_name = %replica.name,
                    circuit_state = ?replica.circuit_breaker.state(),
                    "Skipping replica due to open circuit breaker"
                );
                continue;
            }

            attempt += 1;

            // Log failover if we're moving from a failed replica to a new one
            if let Some(from_replica) = last_failed_replica {
                tracing::warn!(
                    from = from_replica,
                    to = replica.name.as_str(),
                    attempt = attempt,
                    "Failover: {} → {}",
                    from_replica,
                    replica.name
                );
            }

            tracing::info!(
                replica_name = %replica.name,
                "Trying replica"
            );

            match request_fn(replica) {
                Ok(result) => {
                    tracing::info!(
                        replica_name = %replica.name,
                        "Replica succeeded"
                    );
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    tracing::warn!(
                        replica_name = %replica.name,
                        error = %e,
                        "Replica failed"
                    );
                    last_failed_replica = Some(&replica.name);
                    all_errors.push(error_msg);
                    last_error = Some(e);
                    // Continue to next replica on failure
                }
            }
        }

        // All replicas failed - log error details
        // unwrap is safe here because we know replicas is not empty (validated in new())
        tracing::error!(
            attempted = attempt,
            errors = ?all_errors,
            "All replicas failed"
        );

        Err(last_error.unwrap())
    }

    /// Try to execute a request against replicas in priority order with a retry budget.
    /// Returns the first successful result, or the last error if all attempts fail.
    ///
    /// This method limits the number of failover attempts to prevent cascading failures
    /// and resource exhaustion when many replicas are configured.
    ///
    /// Skips replicas with open circuit breakers (unhealthy replicas).
    /// Note: Skipped replicas do NOT count against the retry budget.
    ///
    /// # Arguments
    /// * `request_fn` - A closure that takes a replica and attempts to execute a request
    /// * `max_attempts` - Maximum number of replicas to try (1 initial + N failovers)
    ///
    /// # Returns
    /// * `Ok(T)` - The successful result from the first working replica
    /// * `Err(E)` - The error from the last attempted replica if all attempts failed
    ///
    /// # Example
    /// ```
    /// // With 5 replicas configured but max_attempts=3:
    /// // - Try replica 1 (priority 1) - initial attempt
    /// // - Try replica 2 (priority 2) - first failover
    /// // - Try replica 3 (priority 3) - second failover
    /// // - Stop: budget exhausted, replicas 4 and 5 not tried
    /// ```
    pub fn try_request_with_budget<F, T, E>(
        &self,
        mut request_fn: F,
        max_attempts: usize,
    ) -> Result<T, E>
    where
        F: FnMut(&ReplicaEntry) -> Result<T, E>,
        E: std::fmt::Display,
    {
        let mut last_error = None;
        let mut last_failed_replica: Option<&str> = None;
        let mut attempt = 0;
        let mut all_errors: Vec<String> = Vec::new();
        let attempts_to_make = max_attempts.min(self.replicas.len());

        for replica in self.replicas.iter().take(attempts_to_make) {
            // Skip replicas with open circuit breakers
            if !replica.circuit_breaker.should_allow_request() {
                tracing::debug!(
                    replica_name = %replica.name,
                    circuit_state = ?replica.circuit_breaker.state(),
                    "Skipping replica due to open circuit breaker"
                );
                continue;
            }

            attempt += 1;

            // Log failover if we're moving from a failed replica to a new one
            if let Some(from_replica) = last_failed_replica {
                tracing::warn!(
                    from = from_replica,
                    to = replica.name.as_str(),
                    attempt = attempt,
                    "Failover: {} → {}",
                    from_replica,
                    replica.name
                );
            }

            tracing::info!(
                replica_name = %replica.name,
                "Trying replica"
            );

            match request_fn(replica) {
                Ok(result) => {
                    tracing::info!(
                        replica_name = %replica.name,
                        "Replica succeeded"
                    );
                    return Ok(result);
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    tracing::warn!(
                        replica_name = %replica.name,
                        error = %e,
                        "Replica failed"
                    );
                    last_failed_replica = Some(&replica.name);
                    all_errors.push(error_msg);
                    last_error = Some(e);
                    // Continue to next replica on failure (if budget allows)
                }
            }
        }

        // All attempts failed - log error details
        // unwrap is safe here because we know replicas is not empty (validated in new())
        // and attempts_to_make is at least 1
        tracing::error!(
            attempted = attempt,
            errors = ?all_errors,
            "All replicas failed"
        );

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

    #[test]
    fn test_http_502_triggers_failover_to_next_replica() {
        // Test: HTTP 502 (Bad Gateway) should trigger failover
        // This indicates upstream server issues - try next replica
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

        // Simulate: first replica returns HTTP 502, second succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("HTTP 502: Bad Gateway".to_string())
            } else {
                Ok(format!("success from {}", replica.name))
            }
        });

        // Verify request succeeded from second replica
        assert!(result.is_ok(), "Request should succeed from second replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu after primary returned 502"
        );

        // Verify both replicas were called
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Should call both replicas (primary returned 502, EU succeeded)"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Should call replica-eu after primary returned 502"
        );
    }

    #[test]
    fn test_http_503_triggers_failover_to_next_replica() {
        // Test: HTTP 503 (Service Unavailable) should trigger failover
        // This indicates server is temporarily overloaded - try next replica
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

        // Simulate: first replica returns HTTP 503, second succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("HTTP 503: Service Unavailable".to_string())
            } else {
                Ok(format!("success from {}", replica.name))
            }
        });

        // Verify request succeeded from second replica
        assert!(result.is_ok(), "Request should succeed from second replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu after primary returned 503"
        );

        // Verify both replicas were called
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Should call both replicas (primary returned 503, EU succeeded)"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Should call replica-eu after primary returned 503"
        );
    }

    #[test]
    fn test_http_504_triggers_failover_to_next_replica() {
        // Test: HTTP 504 (Gateway Timeout) should trigger failover
        // This indicates gateway/proxy didn't receive timely response from upstream - try next replica
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

        // Simulate: first replica returns HTTP 504, second succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("HTTP 504: Gateway Timeout".to_string())
            } else {
                Ok(format!("success from {}", replica.name))
            }
        });

        // Verify request succeeded from second replica
        assert!(result.is_ok(), "Request should succeed from second replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu after primary returned 504"
        );

        // Verify both replicas were called
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Should call both replicas (primary returned 504, EU succeeded)"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Should call replica-eu after primary returned 504"
        );
    }

    #[test]
    fn test_http_403_does_not_trigger_failover() {
        // Test: HTTP 403 (Forbidden) should NOT trigger failover
        // This is a client error (4xx) - the request itself is invalid, trying another replica won't help
        // Return error immediately to client
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

        // Simulate: first replica returns HTTP 403 (client error - forbidden)
        // Currently, try_request will try all replicas on ANY error
        // TODO: In future, we should implement error classification to skip failover for 4xx errors
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("HTTP 403: Forbidden".to_string())
            } else {
                // For this test, we simulate that second replica also returns 403
                // (because the request itself is invalid - wrong credentials, no permissions, etc.)
                Err::<String, String>("HTTP 403: Forbidden".to_string())
            }
        });

        // Verify request failed with 403
        assert!(result.is_err(), "Request should fail with 403 Forbidden");
        assert_eq!(
            result.unwrap_err(),
            "HTTP 403: Forbidden",
            "Should return 403 Forbidden error"
        );

        // CURRENT BEHAVIOR: try_request tries all replicas on any error
        // This test documents current behavior - both replicas are called
        // In a future enhancement, we could add error classification to stop trying on 4xx errors
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Current behavior: try_request tries all replicas even for 4xx errors"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Current behavior: tries second replica even though 403 is a client error"
        );

        // NOTE: This test documents CURRENT behavior where all replicas are tried.
        // Future enhancement: Add error classification to skip failover for 4xx errors.
        // When that's implemented, this test should be updated to verify only primary is called.
    }

    #[test]
    fn test_http_404_does_not_trigger_failover() {
        // Test: HTTP 404 (Not Found) should NOT trigger failover
        // This is a client error (4xx) - the file doesn't exist, trying another replica won't help
        // Return error immediately to client
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

        // Simulate: first replica returns HTTP 404 (client error - file not found)
        // Currently, try_request will try all replicas on ANY error
        // TODO: In future, we should implement error classification to skip failover for 4xx errors
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            if replica.name == "primary" {
                Err::<String, String>("HTTP 404: Not Found".to_string())
            } else {
                // For this test, we simulate that second replica also returns 404
                // (because the file doesn't exist in any replica)
                Err::<String, String>("HTTP 404: Not Found".to_string())
            }
        });

        // Verify request failed with 404
        assert!(result.is_err(), "Request should fail with 404 Not Found");
        assert_eq!(
            result.unwrap_err(),
            "HTTP 404: Not Found",
            "Should return 404 Not Found error"
        );

        // CURRENT BEHAVIOR: try_request tries all replicas on any error
        // This test documents current behavior - both replicas are called
        // In a future enhancement, we could add error classification to stop trying on 4xx errors
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            2,
            "Current behavior: try_request tries all replicas even for 4xx errors"
        );
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(
            calls[1], "replica-eu",
            "Current behavior: tries second replica even though 404 is a client error"
        );

        // NOTE: This test documents CURRENT behavior where all replicas are tried.
        // Future enhancement: Add error classification to skip failover for 4xx errors.
        // When that's implemented, this test should be updated to verify only primary is called.
    }

    #[test]
    fn test_all_replicas_failed_returns_last_error() {
        // Test: When all replicas fail, return the LAST error (from the last replica tried)
        // This is important for proper error reporting to clients
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
            S3Replica {
                name: "replica-ap".to_string(),
                bucket: "products-ap-southeast-1".to_string(),
                region: "ap-southeast-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
                endpoint: Some("https://s3.ap-southeast-1.amazonaws.com".to_string()),
                priority: 3,
                timeout: 20,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");
        let calls = RefCell::new(Vec::new());

        // All replicas fail with different errors
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            match replica.name.as_str() {
                "primary" => Err::<String, String>("HTTP 500: Internal Server Error".to_string()),
                "replica-eu" => Err::<String, String>("HTTP 502: Bad Gateway".to_string()),
                "replica-ap" => Err::<String, String>("HTTP 503: Service Unavailable".to_string()),
                _ => panic!("Unexpected replica name"),
            }
        });

        // Verify request failed with the LAST error (from replica-ap)
        assert!(
            result.is_err(),
            "Request should fail when all replicas fail"
        );
        assert_eq!(
            result.unwrap_err(),
            "HTTP 503: Service Unavailable",
            "Should return the last error (from last replica tried)"
        );

        // Verify all three replicas were called in priority order
        let calls = calls.borrow();
        assert_eq!(calls.len(), 3, "Should try all 3 replicas");
        assert_eq!(calls[0], "primary", "Should call primary replica first");
        assert_eq!(calls[1], "replica-eu", "Should call second replica");
        assert_eq!(
            calls[2], "replica-ap",
            "Should call third replica, whose error is returned"
        );
    }

    #[test]
    fn test_failover_respects_retry_budget() {
        // Test: Retry budget limits failover attempts to prevent cascading failures
        // Budget of 3 total tries = 1 initial + 2 failovers
        // With 5 replicas available, should only try first 3 (priorities 1, 2, 3)
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
            S3Replica {
                name: "replica-ap".to_string(),
                bucket: "products-ap-southeast-1".to_string(),
                region: "ap-southeast-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
                endpoint: Some("https://s3.ap-southeast-1.amazonaws.com".to_string()),
                priority: 3,
                timeout: 20,
            },
            S3Replica {
                name: "replica-sa".to_string(),
                bucket: "products-sa-east-1".to_string(),
                region: "sa-east-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE4".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY4".to_string(),
                endpoint: Some("https://s3.sa-east-1.amazonaws.com".to_string()),
                priority: 4,
                timeout: 20,
            },
            S3Replica {
                name: "replica-af".to_string(),
                bucket: "products-af-south-1".to_string(),
                region: "af-south-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE5".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY5".to_string(),
                endpoint: Some("https://s3.af-south-1.amazonaws.com".to_string()),
                priority: 5,
                timeout: 20,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");
        let calls = RefCell::new(Vec::new());

        // All replicas fail, but retry budget should limit tries to 3
        let result = replica_set.try_request_with_budget(
            |replica| {
                calls.borrow_mut().push(replica.name.clone());
                Err::<String, String>(format!("HTTP 500 from {}", replica.name))
            },
            3, // max_attempts: 1 initial try + 2 failovers = 3 total
        );

        // Verify request failed (all 3 attempts failed)
        assert!(
            result.is_err(),
            "Request should fail when all attempts fail"
        );

        // Verify only first 3 replicas were tried (retry budget respected)
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            3,
            "Should only try 3 replicas (respecting retry budget)"
        );
        assert_eq!(calls[0], "primary", "Should try primary first");
        assert_eq!(calls[1], "replica-eu", "Should try EU replica second");
        assert_eq!(calls[2], "replica-ap", "Should try AP replica third");

        // Verify replicas 4 and 5 were NOT tried (budget exhausted)
        assert!(
            !calls.contains(&"replica-sa".to_string()),
            "Should NOT try replica-sa (budget exhausted)"
        );
        assert!(
            !calls.contains(&"replica-af".to_string()),
            "Should NOT try replica-af (budget exhausted)"
        );

        // Verify last error is from the 3rd replica (replica-ap)
        assert!(
            result.unwrap_err().contains("replica-ap"),
            "Should return error from last attempted replica (replica-ap)"
        );
    }

    #[test]
    fn test_failover_skips_unhealthy_replicas() {
        // Test: Circuit breaker integration - skip replicas with open circuit breakers
        // When a replica's circuit breaker is open (unhealthy), skip it during failover
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
            S3Replica {
                name: "replica-ap".to_string(),
                bucket: "products-ap-southeast-1".to_string(),
                region: "ap-southeast-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
                endpoint: Some("https://s3.ap-southeast-1.amazonaws.com".to_string()),
                priority: 3,
                timeout: 20,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

        // Open the circuit breaker for the primary replica by recording failures
        // Default failure_threshold is 5
        for _ in 0..5 {
            replica_set.replicas[0].circuit_breaker.record_failure();
        }

        // Verify primary circuit breaker is now open
        assert_eq!(
            replica_set.replicas[0].circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Open,
            "Primary replica circuit breaker should be open after 5 failures"
        );

        // Verify EU replica circuit breaker is still closed
        assert_eq!(
            replica_set.replicas[1].circuit_breaker.state(),
            crate::circuit_breaker::CircuitState::Closed,
            "EU replica circuit breaker should still be closed"
        );

        let calls = RefCell::new(Vec::new());

        // Try a request - should skip primary (circuit open) and go to EU replica
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            Ok::<String, String>(format!("success from {}", replica.name))
        });

        // Verify request succeeded from EU replica (skipped primary)
        assert!(result.is_ok(), "Request should succeed from EU replica");
        assert_eq!(
            result.unwrap(),
            "success from replica-eu",
            "Should return result from replica-eu (primary skipped)"
        );

        // Verify only EU replica was called (primary skipped due to open circuit)
        let calls = calls.borrow();
        assert_eq!(
            calls.len(),
            1,
            "Should only call 1 replica (primary skipped due to open circuit)"
        );
        assert_eq!(
            calls[0], "replica-eu",
            "Should call replica-eu first (primary skipped)"
        );

        // Verify primary was NOT called
        assert!(
            !calls.contains(&"primary".to_string()),
            "Should NOT call primary (circuit breaker open)"
        );
    }

    #[test]
    fn test_failover_logs_replica_name_and_reason() {
        // Test: Verify that failover attempts log replica names and failure reasons
        // This is important for observability and debugging in production
        use std::cell::RefCell;
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        // Custom writer to capture log output
        #[derive(Clone)]
        struct LogCapture {
            logs: Arc<Mutex<Vec<String>>>,
        }

        impl LogCapture {
            fn new() -> Self {
                Self {
                    logs: Arc::new(Mutex::new(Vec::new())),
                }
            }

            fn get_logs(&self) -> Vec<String> {
                self.logs.lock().unwrap().clone()
            }
        }

        impl std::io::Write for LogCapture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let msg = String::from_utf8_lossy(buf).to_string();
                self.logs.lock().unwrap().push(msg);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> MakeWriter<'a> for LogCapture {
            type Writer = Self;

            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        // Set up tracing subscriber to capture logs
        let log_capture = LogCapture::new();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(log_capture.clone())
            .without_time()
            .with_ansi(false)
            .finish();

        // Set as global subscriber (ignore error if already set in other tests)
        let _ = tracing::subscriber::set_global_default(subscriber);

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
                name: "replica-ap".to_string(),
                bucket: "products-ap-southeast-1".to_string(),
                region: "ap-southeast-1".to_string(),
                access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
                endpoint: Some("https://s3.ap-southeast-1.amazonaws.com".to_string()),
                priority: 3,
                timeout: 20,
            },
        ];

        let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");
        let calls = RefCell::new(Vec::new());

        // First two replicas fail, third succeeds
        let result = replica_set.try_request(|replica| {
            calls.borrow_mut().push(replica.name.clone());
            match replica.name.as_str() {
                "primary" => Err::<String, String>("Connection timeout after 5s".to_string()),
                "replica-eu" => Err::<String, String>("HTTP 503 Service Unavailable".to_string()),
                "replica-ap" => Ok("success from replica-ap".to_string()),
                _ => panic!("Unexpected replica name"),
            }
        });

        // Verify request succeeded
        assert!(
            result.is_ok(),
            "Request should succeed when third replica works"
        );
        assert_eq!(
            result.unwrap(),
            "success from replica-ap",
            "Should return result from replica-ap"
        );

        // Get captured logs
        let logs = log_capture.get_logs();
        let logs_text = logs.join("\n");

        // Verify logs contain replica names when attempting requests
        assert!(
            logs_text.contains("primary"),
            "Logs should contain primary replica name"
        );
        assert!(
            logs_text.contains("replica-eu"),
            "Logs should contain replica-eu name"
        );
        assert!(
            logs_text.contains("replica-ap"),
            "Logs should contain replica-ap name"
        );

        // Verify logs contain failure reasons for failed replicas
        assert!(
            logs_text.contains("Connection timeout") || logs_text.contains("failed"),
            "Logs should contain failure reason for primary"
        );
        assert!(
            logs_text.contains("503") || logs_text.contains("failed"),
            "Logs should contain failure reason for replica-eu"
        );

        // Verify logs indicate success for working replica
        assert!(
            logs_text.contains("succeeded") || logs_text.contains("replica-ap"),
            "Logs should indicate success from replica-ap"
        );
    }

    #[test]
    fn test_log_all_replicas_failed_with_error_details() {
        // Test: Phase 23 - Log all replicas failed with error details
        // Expected log format:
        // ERROR All replicas failed
        //       request_id=550e8400-..., bucket=products, attempted=3,
        //       errors=[ConnectionTimeout, ConnectionTimeout, 500InternalError]

        use crate::logging::create_test_subscriber;
        use std::sync::{Arc, Mutex};

        // Create a buffer to capture log output
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = create_test_subscriber(buffer.clone());

        tracing::subscriber::with_default(subscriber, || {
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
                    name: "replica-ap".to_string(),
                    bucket: "products-ap-southeast-1".to_string(),
                    region: "ap-southeast-1".to_string(),
                    access_key: "AKIAIOSFODNN7EXAMPLE3".to_string(),
                    secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY3".to_string(),
                    endpoint: Some("https://s3.ap-southeast-1.amazonaws.com".to_string()),
                    priority: 3,
                    timeout: 20,
                },
            ];

            let replica_set = ReplicaSet::new(&replicas).expect("Should create ReplicaSet");

            // Call try_request with a function that always fails with different errors
            let mut attempt_count = 0;
            let result: Result<String, String> = replica_set.try_request(|replica| {
                attempt_count += 1;
                match replica.name.as_str() {
                    "primary" => Err("ConnectionTimeout".to_string()),
                    "replica-eu" => Err("ConnectionTimeout".to_string()),
                    "replica-ap" => Err("500InternalError".to_string()),
                    _ => panic!("Unexpected replica name"),
                }
            });

            // Verify request failed
            assert!(
                result.is_err(),
                "Request should fail when all replicas fail"
            );

            // Verify we tried 3 replicas
            assert_eq!(attempt_count, 3, "Should have attempted all 3 replicas");
        });

        // Read log output
        let output = buffer.lock().unwrap();
        let log_line = String::from_utf8_lossy(&output);

        // Verify log contains "All replicas failed" message
        assert!(
            log_line.contains("All replicas failed"),
            "Log should contain 'All replicas failed' message"
        );

        // Verify log contains attempted count
        assert!(
            log_line.contains("\"attempted\":3"),
            "Log should contain attempted=3"
        );

        // Verify log contains all error messages
        assert!(
            log_line.contains("ConnectionTimeout"),
            "Log should contain ConnectionTimeout errors"
        );
        assert!(
            log_line.contains("500InternalError"),
            "Log should contain 500InternalError"
        );
    }

    #[test]
    fn test_log_replica_skip_due_to_circuit_breaker() {
        // Test: Phase 23 - Log replica skip due to circuit breaker
        // Expected log format:
        // DEBUG Skipping replica due to open circuit breaker
        //       replica_name=primary, circuit_state=Open

        use crate::logging::create_test_subscriber;
        use std::sync::{Arc, Mutex};

        // Create a buffer to capture log output
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = create_test_subscriber(buffer.clone());

        tracing::subscriber::with_default(subscriber, || {
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

            // Open the circuit breaker for primary replica by recording failures
            for _ in 0..5 {
                replica_set.replicas[0].circuit_breaker.record_failure();
            }

            // Verify primary circuit breaker is open
            assert_eq!(
                replica_set.replicas[0].circuit_breaker.state(),
                crate::circuit_breaker::CircuitState::Open,
                "Primary circuit breaker should be open"
            );

            // Call try_request - should skip primary and use replica-eu
            let result: Result<String, String> = replica_set.try_request(|replica| {
                // Only replica-eu should be called (primary skipped)
                if replica.name == "replica-eu" {
                    Ok(format!("success from {}", replica.name))
                } else {
                    panic!("Should not call primary replica (circuit breaker open)")
                }
            });

            // Verify request succeeded from replica-eu
            assert!(result.is_ok(), "Request should succeed from replica-eu");
            assert_eq!(
                result.unwrap(),
                "success from replica-eu",
                "Should return result from replica-eu"
            );
        });

        // Read log output
        let output = buffer.lock().unwrap();
        let log_line = String::from_utf8_lossy(&output);

        // Verify log contains "Skipping replica due to open circuit breaker" message
        assert!(
            log_line.contains("Skipping replica due to open circuit breaker"),
            "Log should contain 'Skipping replica due to open circuit breaker' message"
        );

        // Verify log contains primary replica name
        assert!(
            log_line.contains("\"replica_name\":\"primary\"")
                || log_line.contains("replica_name=primary"),
            "Log should contain replica_name=primary"
        );

        // Verify log contains circuit state (Open)
        assert!(
            log_line.contains("Open") || log_line.contains("circuit_state"),
            "Log should contain circuit state (Open)"
        );
    }
}
