//! Proxy initialization logic.
//!
//! This module contains the initialization code for [`YatagarasuProxy`],
//! including component setup and configuration loading.
//!
//! The initialization is split into two stages:
//! 1. [`initialize_from_config`] - Creates all components from configuration
//! 2. [`build_from_components`] - Assembles the proxy from initialized components
//!
//! This separation allows code reuse between `new()` and `with_reload()` constructors.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::audit::AsyncAuditFileWriter;
use crate::cache::tiered::TieredCache;
use crate::cache::warming::PrewarmManager;
use crate::cache::Cache;
use crate::circuit_breaker::CircuitBreaker;
use crate::config::Config;
use crate::metrics::Metrics;
use crate::opa::{OpaCache, OpaClient, OpaClientConfig, SharedOpaClient};
use crate::openfga::OpenFgaClient;
use crate::rate_limit::RateLimitManager;
use crate::request_coalescing::Coalescer;
use crate::resources::ResourceMonitor;
use crate::retry::RetryPolicy;
use crate::router::Router;
use crate::security::SecurityLimits;

/// Components initialized from configuration.
///
/// This struct holds all the initialized components needed to build a
/// [`YatagarasuProxy`](super::YatagarasuProxy). It is used internally to
/// avoid code duplication between `new()` and `with_reload()`.
pub(super) struct ProxyComponents {
    pub config: Config,
    pub router: Router,
    pub metrics: Arc<Metrics>,
    pub resource_monitor: Arc<ResourceMonitor>,
    pub request_semaphore: Arc<Semaphore>,
    pub coalescer: Option<Coalescer>,
    pub circuit_breakers: HashMap<String, Arc<CircuitBreaker>>,
    pub rate_limit_manager: Option<Arc<RateLimitManager>>,
    pub retry_policies: HashMap<String, RetryPolicy>,
    pub security_limits: SecurityLimits,
    pub replica_sets: HashMap<String, crate::replica_set::ReplicaSet>,
    pub cache: Option<Arc<TieredCache>>,
    pub opa_clients: HashMap<String, SharedOpaClient>,
    pub opa_cache: Option<Arc<OpaCache>>,
    pub openfga_clients: HashMap<String, Arc<OpenFgaClient>>,
    pub audit_writer: Option<Arc<AsyncAuditFileWriter>>,
    pub prewarm_manager: Arc<PrewarmManager>,
}

/// Initialize audit writer from configuration.
///
/// Returns `Some(writer)` if audit logging is enabled and file configuration
/// is present, otherwise returns `None`.
pub(super) fn initialize_audit_writer(config: &Config) -> Option<Arc<AsyncAuditFileWriter>> {
    let audit_config = config.audit_log.as_ref()?;
    if !audit_config.enabled {
        return None;
    }
    let file_config = audit_config.file.as_ref()?;
    match AsyncAuditFileWriter::new(
        &file_config.path,
        file_config.max_file_size_mb,
        file_config.max_backup_files,
        file_config.rotation_policy.clone(),
        file_config.buffer_size,
    ) {
        Ok(writer) => Some(Arc::new(writer)),
        Err(e) => {
            tracing::error!("Failed to initialize audit file writer: {}", e);
            None
        }
    }
}

/// Initialize all proxy components from configuration.
///
/// This is the common initialization logic shared by `new()` and `with_reload()`.
/// It normalizes the configuration, creates all required components, and returns
/// them in a [`ProxyComponents`] struct.
///
/// # Components initialized
///
/// - Router for path-based bucket routing
/// - Metrics collector
/// - Resource monitor for system load tracking
/// - Request semaphore for concurrency limiting
/// - Circuit breakers per bucket (if configured)
/// - Rate limit manager (if enabled)
/// - Retry policies per bucket
/// - Replica sets for HA failover
/// - OPA clients and cache for authorization
/// - OpenFGA clients for authorization
/// - Audit writer for request logging
/// - Cache prewarm manager
pub(super) fn initialize_from_config(config: Config) -> ProxyComponents {
    // Normalize config to ensure all buckets have replicas populated (Phase 23: HA support)
    let config = config.normalize();
    let router = Router::new(config.buckets.clone());
    let metrics = Arc::new(Metrics::new());
    // Initialize resource monitor with auto-detected system limits
    let resource_monitor = Arc::new(ResourceMonitor::new_auto_detect());
    // Initialize request semaphore with max concurrent requests limit
    let request_semaphore = Arc::new(Semaphore::new(config.server.max_concurrent_requests));

    // Initialize circuit breakers for buckets that have circuit_breaker config
    let circuit_breakers = initialize_circuit_breakers(&config);

    // Initialize rate limit manager if enabled
    let rate_limit_manager = initialize_rate_limit_manager(&config);

    // Initialize retry policies for buckets that have retry config
    let retry_policies = initialize_retry_policies(&config);

    // Initialize replica sets for each bucket (Phase 23: HA bucket replication)
    let replica_sets = initialize_replica_sets(&config);

    let security_limits = config.server.security_limits.to_security_limits();

    // Cache is initialized to None here and then populated asynchronously
    // via YatagarasuProxy::init_cache() which is called from main.rs
    // This two-phase initialization is required because TieredCache::from_config()
    // is async (connects to Redis, validates disk paths, etc.)
    let cache = None;

    // Phase 32: Initialize OPA clients and cache for buckets with authorization config
    let (opa_clients, opa_cache) = initialize_opa_clients(&config);

    // Phase 49: Initialize OpenFGA clients for buckets with authorization config
    let openfga_clients = initialize_openfga_clients(&config);

    // Initialize audit writer if enabled
    let audit_writer = initialize_audit_writer(&config);

    // Initialize prewarm manager
    let prewarm_manager = Arc::new(PrewarmManager::new(
        cache.clone().map(|c| c as Arc<dyn Cache>),
    ));

    // Initialize coalescer based on config strategy (Phase 38/40)
    let coalescer = initialize_coalescer(&config);

    ProxyComponents {
        config,
        router,
        metrics,
        resource_monitor,
        request_semaphore,
        coalescer,
        circuit_breakers,
        rate_limit_manager,
        retry_policies,
        security_limits,
        replica_sets,
        cache,
        opa_clients,
        opa_cache,
        openfga_clients,
        audit_writer,
        prewarm_manager,
    }
}

/// Initialize circuit breakers for buckets with circuit_breaker config.
fn initialize_circuit_breakers(config: &Config) -> HashMap<String, Arc<CircuitBreaker>> {
    let mut circuit_breakers = HashMap::new();
    for bucket in &config.buckets {
        if let Some(ref cb_config) = bucket.s3.circuit_breaker {
            let breaker = CircuitBreaker::new(cb_config.to_circuit_breaker_config());
            circuit_breakers.insert(bucket.name.clone(), Arc::new(breaker));
        }
    }
    circuit_breakers
}

/// Initialize rate limit manager if enabled in config.
fn initialize_rate_limit_manager(config: &Config) -> Option<Arc<RateLimitManager>> {
    let rate_limit_config = config.server.rate_limit.as_ref()?;
    if !rate_limit_config.enabled {
        return None;
    }

    let global_rps = rate_limit_config
        .global
        .as_ref()
        .map(|g| g.requests_per_second);
    let per_ip_rps = rate_limit_config
        .per_ip
        .as_ref()
        .map(|p| p.requests_per_second);
    let manager = RateLimitManager::new(global_rps, per_ip_rps);

    // Add per-bucket rate limiters
    for bucket in &config.buckets {
        if let Some(ref bucket_rate_limit) = bucket.s3.rate_limit {
            manager.add_bucket_limiter(bucket.name.clone(), bucket_rate_limit.requests_per_second);
        }
    }

    // Start background cleanup task to evict idle rate limiters (Phase 36)
    // This prevents unbounded memory growth from per-IP/per-user tracking
    manager.start_cleanup_task(None); // Uses default interval (60s)

    Some(Arc::new(manager))
}

/// Initialize retry policies for all buckets.
fn initialize_retry_policies(config: &Config) -> HashMap<String, RetryPolicy> {
    let mut retry_policies = HashMap::new();
    for bucket in &config.buckets {
        if let Some(ref retry_config) = bucket.s3.retry {
            let policy = retry_config.to_retry_policy();
            retry_policies.insert(bucket.name.clone(), policy);
        } else {
            // Use default retry policy if not configured
            retry_policies.insert(bucket.name.clone(), RetryPolicy::default());
        }
    }
    retry_policies
}

/// Initialize replica sets for HA bucket replication.
fn initialize_replica_sets(config: &Config) -> HashMap<String, crate::replica_set::ReplicaSet> {
    let mut replica_sets = HashMap::new();
    for bucket in &config.buckets {
        // After normalization, all buckets have replicas populated
        // (either from replicas array or converted from legacy fields)
        if let Some(ref replicas) = bucket.s3.replicas {
            match crate::replica_set::ReplicaSet::new(replicas) {
                Ok(replica_set) => {
                    replica_sets.insert(bucket.name.clone(), replica_set);
                }
                Err(e) => {
                    tracing::error!(
                        bucket = %bucket.name,
                        error = %e,
                        "Failed to create ReplicaSet for bucket, skipping"
                    );
                    // Skip this bucket - it won't have failover support
                }
            }
        } else {
            tracing::warn!(
                bucket = %bucket.name,
                "Bucket has no replicas configured after normalization, skipping"
            );
        }
    }
    replica_sets
}

/// Initialize coalescer based on configuration.
///
/// Returns `None` if coalescing is disabled, otherwise creates the appropriate
/// coalescer based on the configured strategy (WaitForComplete or Streaming).
fn initialize_coalescer(config: &Config) -> Option<Coalescer> {
    let coalescing_config = &config.server.coalescing;

    if !coalescing_config.enabled {
        tracing::info!("Request coalescing is disabled");
        return None;
    }

    let strategy = coalescing_config.strategy;
    let coalescer = Coalescer::new(strategy);

    tracing::info!(
        strategy = ?strategy,
        "Request coalescing initialized"
    );

    Some(coalescer)
}

/// Initialize OPA clients and shared cache.
fn initialize_opa_clients(
    config: &Config,
) -> (HashMap<String, SharedOpaClient>, Option<Arc<OpaCache>>) {
    let mut opa_clients = HashMap::new();
    let mut max_cache_ttl = 0u64;

    for bucket in &config.buckets {
        if let Some(ref auth_config) = bucket.authorization {
            if auth_config.auth_type == "opa" {
                if let (Some(opa_url), Some(policy_path)) =
                    (&auth_config.opa_url, &auth_config.opa_policy_path)
                {
                    let client_config = OpaClientConfig {
                        url: opa_url.clone(),
                        policy_path: policy_path.clone(),
                        timeout_ms: auth_config.opa_timeout_ms,
                        cache_ttl_seconds: auth_config.opa_cache_ttl_seconds,
                    };
                    max_cache_ttl = max_cache_ttl.max(client_config.cache_ttl_seconds);
                    match OpaClient::new(client_config) {
                        Ok(client) => {
                            opa_clients.insert(bucket.name.clone(), Arc::new(client));
                            tracing::info!(
                                bucket = %bucket.name,
                                opa_url = %opa_url,
                                policy_path = %policy_path,
                                "OPA authorization enabled for bucket"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                bucket = %bucket.name,
                                error = %e,
                                "Failed to create OPA client for bucket, skipping OPA authorization"
                            );
                        }
                    }
                }
            }
        }
    }

    // Create shared OPA cache if any bucket uses OPA
    let opa_cache = if !opa_clients.is_empty() {
        Some(Arc::new(OpaCache::new(max_cache_ttl.max(60))))
    } else {
        None
    };

    (opa_clients, opa_cache)
}

/// Initialize OpenFGA clients for buckets with OpenFGA authorization.
fn initialize_openfga_clients(config: &Config) -> HashMap<String, Arc<OpenFgaClient>> {
    let mut openfga_clients = HashMap::new();

    for bucket in &config.buckets {
        if let Some(ref auth_config) = bucket.authorization {
            if auth_config.auth_type == "openfga" {
                if let (Some(endpoint), Some(store_id)) =
                    (&auth_config.openfga_endpoint, &auth_config.openfga_store_id)
                {
                    let mut builder = OpenFgaClient::builder(endpoint, store_id);

                    // Set optional API token
                    if let Some(ref api_token) = auth_config.openfga_api_token {
                        builder = builder.api_token(api_token);
                    }

                    // Set optional authorization model ID
                    if let Some(ref model_id) = auth_config.openfga_authorization_model_id {
                        builder = builder.authorization_model_id(model_id);
                    }

                    // Set timeout (default: 100ms)
                    builder = builder.timeout_ms(auth_config.openfga_timeout_ms);

                    match builder.build() {
                        Ok(client) => {
                            openfga_clients.insert(bucket.name.clone(), Arc::new(client));
                            tracing::info!(
                                bucket = %bucket.name,
                                endpoint = %endpoint,
                                store_id = %store_id,
                                "OpenFGA authorization enabled for bucket"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                bucket = %bucket.name,
                                error = %e,
                                "Failed to create OpenFGA client for bucket"
                            );
                        }
                    }
                }
            }
        }
    }

    openfga_clients
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal test config.
    fn minimal_config() -> Config {
        Config::from_yaml_with_env(
            r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#,
        )
        .expect("minimal config should parse")
    }

    #[test]
    fn test_initialize_circuit_breakers_empty_config() {
        let config = minimal_config();
        let breakers = initialize_circuit_breakers(&config);
        assert!(breakers.is_empty());
    }

    #[test]
    fn test_initialize_rate_limit_manager_disabled() {
        let config = minimal_config();
        let manager = initialize_rate_limit_manager(&config);
        assert!(manager.is_none());
    }

    #[test]
    fn test_initialize_retry_policies_defaults() {
        let config = minimal_config();
        let policies = initialize_retry_policies(&config);
        // Empty buckets means empty policies
        assert!(policies.is_empty());
    }

    #[test]
    fn test_initialize_opa_clients_no_opa_buckets() {
        let config = minimal_config();
        let (clients, cache) = initialize_opa_clients(&config);
        assert!(clients.is_empty());
        assert!(cache.is_none());
    }

    #[test]
    fn test_initialize_openfga_clients_no_openfga_buckets() {
        let config = minimal_config();
        let clients = initialize_openfga_clients(&config);
        assert!(clients.is_empty());
    }

    #[test]
    fn test_initialize_audit_writer_disabled() {
        let config = minimal_config();
        let writer = initialize_audit_writer(&config);
        assert!(writer.is_none());
    }
}
