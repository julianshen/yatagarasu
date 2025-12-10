// Proxy module - Pingora ProxyHttp implementation
// Implements the HTTP proxy logic for Yatagarasu S3 proxy

use async_trait::async_trait;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::Result;
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use crate::audit::AsyncAuditFileWriter;
use crate::auth::{authenticate_request, AuthError};
use crate::cache::tiered::TieredCache;
use crate::cache::warming::PrewarmManager;
use crate::cache::{Cache, CacheKey};
use crate::circuit_breaker::CircuitBreaker;
use crate::config::Config;
use crate::metrics::Metrics;
use crate::opa::{
    AuthorizationDecision as OpaAuthorizationDecision, FailMode as OpaFailMode, OpaCache,
    OpaClient, OpaClientConfig, OpaInput, SharedOpaClient,
};
use crate::openfga::{
    build_openfga_object, extract_user_id, http_method_to_relation,
    AuthorizationDecision as OpenFgaAuthorizationDecision, FailMode as OpenFgaFailMode,
    OpenFgaClient,
};
use crate::pipeline::RequestContext;
use crate::rate_limit::RateLimitManager;
use crate::reload::ReloadManager;
use crate::resources::ResourceMonitor;
use crate::retry::RetryPolicy;
use crate::router::Router;
use crate::s3::{build_get_object_request, build_head_object_request};
use crate::security::{self, SecurityLimits};
use arc_swap::ArcSwap;
use std::path::PathBuf;
use std::str::FromStr;

/// Components initialized from configuration
/// Used internally to avoid code duplication between new() and with_reload()
struct ProxyComponents {
    config: Config,
    router: Router,
    metrics: Arc<Metrics>,
    resource_monitor: Arc<ResourceMonitor>,
    request_semaphore: Arc<Semaphore>,
    circuit_breakers: HashMap<String, Arc<CircuitBreaker>>,
    rate_limit_manager: Option<Arc<RateLimitManager>>,
    retry_policies: HashMap<String, RetryPolicy>,
    security_limits: SecurityLimits,
    replica_sets: HashMap<String, crate::replica_set::ReplicaSet>,
    cache: Option<Arc<TieredCache>>,
    opa_clients: HashMap<String, SharedOpaClient>,
    opa_cache: Option<Arc<OpaCache>>,
    openfga_clients: HashMap<String, Arc<OpenFgaClient>>,
    audit_writer: Option<Arc<AsyncAuditFileWriter>>,
    prewarm_manager: Arc<PrewarmManager>,
}

/// YatagarasuProxy implements the Pingora ProxyHttp trait
/// Handles routing, authentication, and S3 proxying
pub struct YatagarasuProxy {
    config: ArcSwap<Config>,
    router: ArcSwap<Router>,
    metrics: Arc<Metrics>,
    reload_manager: Option<Arc<ReloadManager>>,
    resource_monitor: Arc<ResourceMonitor>,
    request_semaphore: Arc<Semaphore>,
    circuit_breakers: Arc<HashMap<String, Arc<CircuitBreaker>>>,
    rate_limit_manager: Option<Arc<RateLimitManager>>,
    /// Retry policies per bucket for automatic retry on transient S3 failures
    retry_policies: Arc<HashMap<String, RetryPolicy>>,
    /// Security validation limits (request size, headers, URI, path traversal)
    security_limits: SecurityLimits,
    /// Proxy start time (for uptime calculation in /health endpoint)
    start_time: Instant,
    /// Replica sets per bucket (Phase 23: High Availability bucket replication with automatic failover)
    replica_sets: Arc<HashMap<String, crate::replica_set::ReplicaSet>>,
    /// Tiered cache (memory → disk → redis) for caching S3 responses (Phase 30)
    /// Optional: cache is only enabled if configured
    cache: Option<Arc<TieredCache>>,
    /// OPA clients per bucket (Phase 32: OPA Integration)
    /// Maps bucket name to OPA client for authorization decisions
    opa_clients: Arc<HashMap<String, SharedOpaClient>>,
    /// OPA authorization decision cache (Phase 32: OPA Integration)
    /// Shared cache for all OPA clients to avoid redundant evaluations
    opa_cache: Option<Arc<OpaCache>>,
    /// OpenFGA clients per bucket (Phase 49: OpenFGA Integration)
    /// Maps bucket name to OpenFGA client for authorization decisions
    openfga_clients: Arc<HashMap<String, Arc<OpenFgaClient>>>,
    /// Audit writer for logging requests
    audit_writer: Option<Arc<AsyncAuditFileWriter>>,
    /// Cache warming task manager (Phase 1.3)
    prewarm_manager: Arc<PrewarmManager>,
}

impl YatagarasuProxy {
    /// Initialize audit writer from configuration
    fn initialize_audit_writer(config: &Config) -> Option<Arc<AsyncAuditFileWriter>> {
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

    /// Initialize all proxy components from configuration
    /// This is the common initialization logic shared by new() and with_reload()
    fn initialize_from_config(config: Config) -> ProxyComponents {
        // Normalize config to ensure all buckets have replicas populated (Phase 23: HA support)
        let config = config.normalize();
        let router = Router::new(config.buckets.clone());
        let metrics = Arc::new(Metrics::new());
        // Initialize resource monitor with auto-detected system limits
        let resource_monitor = Arc::new(ResourceMonitor::new_auto_detect());
        // Initialize request semaphore with max concurrent requests limit
        let request_semaphore = Arc::new(Semaphore::new(config.server.max_concurrent_requests));

        // Initialize circuit breakers for buckets that have circuit_breaker config
        let mut circuit_breakers = HashMap::new();
        for bucket in &config.buckets {
            if let Some(ref cb_config) = bucket.s3.circuit_breaker {
                let breaker = CircuitBreaker::new(cb_config.to_circuit_breaker_config());
                circuit_breakers.insert(bucket.name.clone(), Arc::new(breaker));
            }
        }

        // Initialize rate limit manager if enabled
        let rate_limit_manager = if let Some(ref rate_limit_config) = config.server.rate_limit {
            if rate_limit_config.enabled {
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
                        manager.add_bucket_limiter(
                            bucket.name.clone(),
                            bucket_rate_limit.requests_per_second,
                        );
                    }
                }

                Some(Arc::new(manager))
            } else {
                None
            }
        } else {
            None
        };

        // Initialize retry policies for buckets that have retry config
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

        // Initialize replica sets for each bucket (Phase 23: HA bucket replication)
        let mut replica_sets = HashMap::new();
        for bucket in &config.buckets {
            // After normalization, all buckets have replicas populated (either from replicas array or converted from legacy fields)
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

        let security_limits = config.server.security_limits.to_security_limits();

        // TODO Phase 30: Initialize cache from config if enabled
        let cache = None; // Temporarily None until cache initialization is implemented

        // Phase 32: Initialize OPA clients and cache for buckets with authorization config
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
                        let client = Arc::new(OpaClient::new(client_config));
                        opa_clients.insert(bucket.name.clone(), client);
                        tracing::info!(
                            bucket = %bucket.name,
                            opa_url = %opa_url,
                            policy_path = %policy_path,
                            "OPA authorization enabled for bucket"
                        );
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

        // Phase 49: Initialize OpenFGA clients for buckets with authorization config
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

        // Initialize audit writer if enabled
        let audit_writer = Self::initialize_audit_writer(&config);

        // Initialize prewarm manager
        let prewarm_manager = Arc::new(PrewarmManager::new(
            cache.clone().map(|c| c as Arc<dyn Cache>),
        ));

        ProxyComponents {
            config,
            router,
            metrics,
            resource_monitor,
            request_semaphore,
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

    /// Build YatagarasuProxy from initialized components
    fn build_from_components(
        components: ProxyComponents,
        reload_manager: Option<Arc<ReloadManager>>,
    ) -> Self {
        Self {
            config: ArcSwap::from_pointee(components.config),
            router: ArcSwap::from_pointee(components.router),
            metrics: components.metrics,
            reload_manager,
            resource_monitor: components.resource_monitor,
            request_semaphore: components.request_semaphore,
            circuit_breakers: Arc::new(components.circuit_breakers),
            rate_limit_manager: components.rate_limit_manager,
            retry_policies: Arc::new(components.retry_policies),
            security_limits: components.security_limits,
            start_time: Instant::now(),
            replica_sets: Arc::new(components.replica_sets),
            cache: components.cache,
            opa_clients: Arc::new(components.opa_clients),
            opa_cache: components.opa_cache,
            openfga_clients: Arc::new(components.openfga_clients),
            audit_writer: components.audit_writer,
            prewarm_manager: components.prewarm_manager,
        }
    }

    /// Create a new YatagarasuProxy instance from configuration
    pub fn new(config: Config) -> Self {
        let components = Self::initialize_from_config(config);
        Self::build_from_components(components, None)
    }

    /// Create a new YatagarasuProxy with reload support
    pub fn with_reload(config: Config, config_path: PathBuf) -> Self {
        let components = Self::initialize_from_config(config);

        let reload_manager = Arc::new(ReloadManager::new(config_path));

        #[cfg(unix)]
        {
            if let Err(e) = reload_manager.register_signal_handler() {
                tracing::warn!("Failed to register SIGHUP handler: {}", e);
            } else {
                tracing::info!("Registered SIGHUP handler for config reload");
            }
        }

        Self::build_from_components(components, Some(reload_manager))
    }

    /// Set the cache instance (used for testing and optional cache initialization)
    /// Phase 30: Cache integration
    pub fn with_cache(mut self, cache: Arc<TieredCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Check for reload request and reload if needed
    fn check_reload(&self) {
        if let Some(reload_manager) = &self.reload_manager {
            if reload_manager.check_and_clear_reload_request() {
                tracing::info!("SIGHUP received, triggering configuration reload");
                let _ = self.reload_configuration();
            }
        }
    }

    /// Reload configuration from disk
    fn reload_configuration(&self) -> Result<u64, String> {
        if let Some(reload_manager) = &self.reload_manager {
            // Load current config to get generation
            let current_config = self.config.load();
            let current_generation = current_config.generation;

            match reload_manager.reload_config_with_generation(current_generation) {
                Ok(new_config) => {
                    let new_generation = new_config.generation;
                    tracing::info!(
                        old_generation = current_generation,
                        new_generation = new_generation,
                        "Configuration loaded successfully, applying changes"
                    );

                    // Create new router
                    let new_router = Router::new(new_config.buckets.clone());

                    // Update shared state atomically (using ArcSwap)
                    self.config.store(Arc::new(new_config));
                    self.router.store(Arc::new(new_router));

                    // Record reload metrics
                    self.metrics.increment_reload_success();
                    self.metrics.set_config_generation(new_generation);

                    Ok(new_generation)
                }
                Err(e) => {
                    tracing::error!(error = %e, "Configuration reload failed");
                    self.metrics.increment_reload_failure();
                    Err(e)
                }
            }
        } else {
            Err("Hot reload not enabled".to_string())
        }
    }

    /// Initialize the cache from configuration asynchronously
    /// Phase 30: Cache integration
    ///
    /// This method should be called after creating the proxy to initialize
    /// the tiered cache if caching is enabled in the configuration.
    ///
    /// # Returns
    /// Returns the proxy with cache initialized, or the proxy unchanged if cache is disabled
    pub async fn init_cache(mut self) -> Self {
        // Check if cache is enabled in config
        let config = self.config.load();
        if let Some(ref cache_config) = config.cache {
            if cache_config.enabled && !cache_config.cache_layers.is_empty() {
                match TieredCache::from_config(cache_config).await {
                    Ok(tiered_cache) => {
                        tracing::info!(
                            layers = ?cache_config.cache_layers,
                            "Cache initialized successfully"
                        );
                        self.cache = Some(Arc::new(tiered_cache));

                        // Update prewarm manager with new cache instance
                        if let Some(ref cache) = self.cache {
                            self.prewarm_manager.set_cache(cache.clone());
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Failed to initialize cache, continuing without cache"
                        );
                        // Continue without cache - graceful degradation
                    }
                }
            } else {
                tracing::info!("Cache is disabled or no cache layers configured");
            }
        } else {
            tracing::info!("No cache configuration found");
        }

        self
    }

    /// Get a reference to the metrics instance
    pub fn metrics(&self) -> Arc<Metrics> {
        Arc::clone(&self.metrics)
    }

    /// Extract headers from Pingora RequestHeader into HashMap
    fn extract_headers(req: &RequestHeader) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        for (name, value) in req.headers.iter() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }
        headers
    }

    /// Extract query parameters from URI
    fn extract_query_params(req: &RequestHeader) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if let Some(query) = req.uri.query() {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    params.insert(
                        key.to_string(),
                        urlencoding::decode(value).unwrap_or_default().to_string(),
                    );
                }
            }
        }
        params
    }

    /// Extract client IP address from session (X-Forwarded-For aware)
    ///
    /// Checks X-Forwarded-For header first (for proxies/load balancers),
    /// then falls back to direct connection IP from session.
    fn get_client_ip(&self, session: &Session) -> String {
        // Check X-Forwarded-For header first (common in reverse proxy setups)
        if let Some(forwarded_for) = session
            .req_header()
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            // X-Forwarded-For can contain multiple IPs: "client, proxy1, proxy2"
            // The first IP is the original client
            if let Some(client_ip) = forwarded_for.split(',').next() {
                return client_ip.trim().to_string();
            }
        }

        // Fall back to direct connection IP
        session
            .client_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Export circuit breaker metrics for Prometheus
    fn export_circuit_breaker_metrics(&self) -> String {
        let mut output = String::new();

        // Circuit breaker state metric (gauge: 0=closed, 1=open, 2=half-open)
        output.push_str("\n# HELP circuit_breaker_state Circuit breaker state per bucket (0=closed, 1=open, 2=half-open)\n");
        output.push_str("# TYPE circuit_breaker_state gauge\n");

        for (bucket_name, circuit_breaker) in self.circuit_breakers.iter() {
            let state_value = circuit_breaker.state() as u8;
            output.push_str(&format!(
                "circuit_breaker_state{{bucket=\"{}\"}} {}\n",
                bucket_name, state_value
            ));
        }

        // Circuit breaker failure count metric (gauge)
        output.push_str("\n# HELP circuit_breaker_failures Current consecutive failure count\n");
        output.push_str("# TYPE circuit_breaker_failures gauge\n");

        for (bucket_name, circuit_breaker) in self.circuit_breakers.iter() {
            output.push_str(&format!(
                "circuit_breaker_failures{{bucket=\"{}\"}} {}\n",
                bucket_name,
                circuit_breaker.failure_count()
            ));
        }

        // Circuit breaker success count in half-open state (gauge)
        output.push_str("\n# HELP circuit_breaker_successes Success count in half-open state\n");
        output.push_str("# TYPE circuit_breaker_successes gauge\n");

        for (bucket_name, circuit_breaker) in self.circuit_breakers.iter() {
            output.push_str(&format!(
                "circuit_breaker_successes{{bucket=\"{}\"}} {}\n",
                bucket_name,
                circuit_breaker.success_count()
            ));
        }

        output
    }
}

#[async_trait]
impl ProxyHttp for YatagarasuProxy {
    type CTX = RequestContext;

    /// Create a new request context for each incoming request
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new("GET".to_string(), "/".to_string())
    }

    /// Determine the upstream S3 peer for this request
    /// Phase 23: Selects healthy replica from ReplicaSet if available
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // Get bucket config from context (set in request_filter)
        let bucket_config = ctx.bucket_config().ok_or_else(|| {
            pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                "No bucket config in context",
            )
        })?;

        // Phase 23: Check if ReplicaSet exists for this bucket
        let bucket_name = bucket_config.name.clone(); // Clone for logging to avoid borrow issues
        if let Some(replica_set) = self.replica_sets.get(&bucket_name) {
            // Select first healthy replica (circuit breaker not open)
            for replica in &replica_set.replicas {
                if replica.circuit_breaker.should_allow_request() {
                    // Store selected replica name in context for logging
                    ctx.set_replica_name(replica.name.clone());

                    // Build endpoint from replica config
                    let (endpoint, port, use_tls) =
                        if let Some(custom_endpoint) = &replica.client.config.endpoint {
                            let endpoint_str = custom_endpoint
                                .trim_start_matches("http://")
                                .trim_start_matches("https://");
                            let use_tls = custom_endpoint.starts_with("https://");

                            let (host, port) = if let Some((h, p)) = endpoint_str.split_once(':') {
                                (
                                    h.to_string(),
                                    p.parse::<u16>().unwrap_or(if use_tls { 443 } else { 80 }),
                                )
                            } else {
                                (endpoint_str.to_string(), if use_tls { 443 } else { 80 })
                            };

                            (host, port, use_tls)
                        } else {
                            // AWS S3 endpoint
                            let endpoint = format!(
                                "{}.s3.{}.amazonaws.com",
                                replica.client.config.bucket, replica.client.config.region
                            );
                            (endpoint, 443, true)
                        };

                    let mut peer = Box::new(HttpPeer::new(
                        (endpoint.clone(), port),
                        use_tls,
                        endpoint.clone(),
                    ));

                    // Configure timeouts from replica config
                    let timeout_duration = Duration::from_secs(replica.client.config.timeout);
                    peer.options.connection_timeout = Some(timeout_duration);
                    peer.options.read_timeout = Some(timeout_duration);
                    peer.options.write_timeout = Some(timeout_duration);

                    tracing::info!(
                        bucket = %bucket_name,
                        replica = %replica.name,
                        endpoint = %endpoint,
                        "Selected healthy replica for request"
                    );

                    // Phase 23: Track active replica in metrics
                    self.metrics.set_active_replica(&bucket_name, &replica.name);

                    return Ok(peer);
                }
            }

            // All replicas unhealthy - return error
            tracing::error!(
                bucket = %bucket_name,
                "All replicas unhealthy (circuit breakers open)"
            );
            return Err(pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                "All replicas unavailable",
            ));
        }

        // No ReplicaSet - use legacy single-bucket configuration
        // Build S3 endpoint - use custom endpoint if provided, otherwise use AWS
        let (endpoint, port, use_tls) = if let Some(custom_endpoint) = &bucket_config.s3.endpoint {
            // Parse custom endpoint (e.g., "http://localhost:9000")
            let endpoint_str = custom_endpoint
                .trim_start_matches("http://")
                .trim_start_matches("https://");
            let use_tls = custom_endpoint.starts_with("https://");

            // Extract host and port
            let (host, port) = if let Some((h, p)) = endpoint_str.split_once(':') {
                (
                    h.to_string(),
                    p.parse::<u16>().unwrap_or(if use_tls { 443 } else { 80 }),
                )
            } else {
                (endpoint_str.to_string(), if use_tls { 443 } else { 80 })
            };

            (host, port, use_tls)
        } else {
            // Default to AWS S3
            let endpoint = format!(
                "{}.s3.{}.amazonaws.com",
                bucket_config.s3.bucket, bucket_config.s3.region
            );
            (endpoint, 443, true)
        };

        // Store endpoint for logging before moving it
        let endpoint_for_logging = endpoint.clone();

        // Create HttpPeer for S3 endpoint - need to clone endpoint for SNI
        let mut peer = Box::new(HttpPeer::new((endpoint.clone(), port), use_tls, endpoint));

        // Configure timeouts from S3Config
        let timeout_duration = Duration::from_secs(bucket_config.s3.timeout);

        // Set connection timeout (how long to wait to establish connection)
        peer.options.connection_timeout = Some(timeout_duration);

        // Set read timeout (how long to wait for data from upstream)
        peer.options.read_timeout = Some(timeout_duration);

        // Set write timeout (how long to wait to send data to upstream)
        peer.options.write_timeout = Some(timeout_duration);

        tracing::debug!(
            bucket = %bucket_config.name,
            timeout_seconds = bucket_config.s3.timeout,
            endpoint = %endpoint_for_logging,
            "Configured S3 peer with timeout (legacy single-bucket mode)"
        );

        Ok(peer)
    }

    /// Filter and process incoming requests (routing and authentication)
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // Check for config reload first
        self.check_reload();

        // Load config and router atomically for this request
        let config = self.config.load_full();
        let router = self.router.load_full();

        // -- Audit Logging: Start Request --
        if self.audit_writer.is_some() {
            let req = session.req_header();
            let audit_ctx = ctx.audit();
            audit_ctx.http_method = Some(req.method.to_string());
            audit_ctx.request_path = Some(req.uri.path().to_string());
            if let Some(ua) = req.headers.get("user-agent").and_then(|v| v.to_str().ok()) {
                audit_ctx.user_agent = Some(ua.to_string());
            }
            if let Some(r) = req.headers.get("referer").and_then(|v| v.to_str().ok()) {
                audit_ctx.referer = Some(r.to_string());
            }
        }
        // -- End Audit Logging --

        // Check concurrency limit FIRST - reject if at max concurrent requests
        let _permit = match self.request_semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    "Rejecting request due to max concurrent requests reached"
                );

                // Increment metrics counter for concurrency limit rejections
                self.metrics.increment_concurrency_limit_rejection();

                let mut header = ResponseHeader::build(503, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Retry-After", "5")?; // Suggest retry after 5 seconds

                let error_body = serde_json::json!({
                    "error": "Service Temporarily Unavailable",
                    "message": "Server has reached maximum concurrent request limit. Please retry after 5 seconds.",
                    "status": 503
                })
                .to_string();

                header.insert_header("Content-Length", error_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(error_body.into()), true)
                    .await?;

                return Ok(true); // Request handled
            }
        };
        // Permit will be automatically released when _permit is dropped at end of function

        // Track active connections (in-flight requests gauge)
        self.metrics.increment_active_connections();

        // Check resource exhaustion SECOND - reject requests if resources exhausted
        if !self.resource_monitor.should_accept_request() {
            tracing::warn!(
                request_id = %ctx.request_id(),
                "Rejecting request due to resource exhaustion"
            );

            let mut header = ResponseHeader::build(503, None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Retry-After", "10")?; // Suggest retry after 10 seconds

            let error_body = serde_json::json!({
                "error": "Service Temporarily Unavailable",
                "message": "Server is under heavy load. Please retry after 10 seconds.",
                "status": 503
            })
            .to_string();

            header.insert_header("Content-Length", error_body.len().to_string())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(error_body.into()), true)
                .await?;

            return Ok(true); // Short-circuit (503 response sent)
        }

        // Extract request information
        let req = session.req_header();
        let path = req.uri.path().to_string();
        let method = req.method.to_string();

        // Extract client IP for logging (X-Forwarded-For aware)
        let client_ip = self.get_client_ip(session);
        if self.audit_writer.is_some() {
            ctx.audit().client_ip = Some(client_ip.clone());
        }

        // SECURITY VALIDATIONS (check early before routing)

        // 0. HTTP Method Validation (Read-Only Proxy - Phase 25)
        // This proxy only supports GET and HEAD for S3 operations
        // Special endpoints (/health, /ready, /metrics, /admin/reload, /admin/cache/*) are handled separately
        if !(path.starts_with("/health")
            || path.starts_with("/ready")
            || path.starts_with("/metrics")
            || (path == "/admin/reload" && method == "POST")
            || (path.starts_with("/admin/cache/") && (method == "POST" || method == "GET")))
        {
            // Only GET, HEAD, and OPTIONS are allowed for S3 operations
            match method.as_str() {
                "GET" | "HEAD" | "OPTIONS" => {} // Allowed
                _ => {
                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        client_ip = %client_ip,
                        method = %method,
                        path = %path,
                        "Unsupported HTTP method for read-only proxy"
                    );

                    let mut header = ResponseHeader::build(405, None)?;
                    header.insert_header("Content-Type", "application/json")?;
                    header.insert_header("Allow", "GET, HEAD, OPTIONS")?;

                    let error_body = serde_json::json!({
                        "error": "Method Not Allowed",
                        "message": format!(
                            "Method {} is not allowed. This is a read-only S3 proxy. Allowed methods: GET, HEAD, OPTIONS",
                            method
                        ),
                        "status": 405
                    })
                    .to_string();

                    header.insert_header("Content-Length", error_body.len().to_string())?;
                    session
                        .write_response_header(Box::new(header), false)
                        .await?;
                    session
                        .write_response_body(Some(error_body.into()), true)
                        .await?;

                    self.metrics.increment_status_count(405);
                    return Ok(true); // Short-circuit
                }
            }
        }

        // Handle OPTIONS requests (CORS pre-flight)
        // OPTIONS is allowed through method validation above, now handle it specially
        if method == "OPTIONS" {
            tracing::debug!(
                request_id = %ctx.request_id(),
                path = %path,
                "Handling OPTIONS request for CORS pre-flight"
            );

            let mut header = ResponseHeader::build(200, None)?;
            header.insert_header("Allow", "GET, HEAD, OPTIONS")?;
            header.insert_header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")?;
            header.insert_header(
                "Access-Control-Allow-Headers",
                "Authorization, Content-Type, Range",
            )?;
            header.insert_header("Access-Control-Max-Age", "86400")?; // 24 hours
            header.insert_header("Content-Length", "0")?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session.write_response_body(None, true).await?;

            self.metrics.increment_status_count(200);
            return Ok(true); // Short-circuit
        }

        // 1. Validate URI length
        let uri_str = req.uri.to_string();
        if let Err(security_error) =
            security::validate_uri_length(&uri_str, self.security_limits.max_uri_length)
        {
            tracing::warn!(
                request_id = %ctx.request_id(),
                client_ip = %client_ip,
                uri_length = uri_str.len(),
                limit = self.security_limits.max_uri_length,
                error = %security_error,
                "URI too long"
            );

            let mut header = ResponseHeader::build(414, None)?;
            header.insert_header("Content-Type", "application/json")?;

            let error_body = serde_json::json!({
                "error": "URI Too Long",
                "message": security_error.to_string(),
                "status": 414
            })
            .to_string();

            header.insert_header("Content-Length", error_body.len().to_string())?;
            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(error_body.into()), true)
                .await?;

            self.metrics.increment_status_count(414);
            self.metrics.increment_security_uri_too_long();
            return Ok(true); // Short-circuit
        }

        // 2. Validate total header size
        let total_header_size: usize = req
            .headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.len())
            .sum();

        if let Err(security_error) =
            security::validate_header_size(total_header_size, self.security_limits.max_header_size)
        {
            tracing::warn!(
                request_id = %ctx.request_id(),
                client_ip = %client_ip,
                header_size = total_header_size,
                limit = self.security_limits.max_header_size,
                error = %security_error,
                "Headers too large"
            );

            let mut header = ResponseHeader::build(431, None)?;
            header.insert_header("Content-Type", "application/json")?;

            let error_body = serde_json::json!({
                "error": "Request Header Fields Too Large",
                "message": security_error.to_string(),
                "status": 431
            })
            .to_string();

            header.insert_header("Content-Length", error_body.len().to_string())?;
            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(error_body.into()), true)
                .await?;

            self.metrics.increment_status_count(431);
            self.metrics.increment_security_headers_too_large();
            return Ok(true); // Short-circuit
        }

        // 3. Validate request body size (from Content-Length header)
        let content_length = req
            .headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());

        if let Err(security_error) =
            security::validate_body_size(content_length, self.security_limits.max_body_size)
        {
            tracing::warn!(
                request_id = %ctx.request_id(),
                client_ip = %client_ip,
                content_length = ?content_length,
                limit = self.security_limits.max_body_size,
                error = %security_error,
                "Request payload too large"
            );

            let mut header = ResponseHeader::build(413, None)?;
            header.insert_header("Content-Type", "application/json")?;

            let error_body = serde_json::json!({
                "error": "Payload Too Large",
                "message": security_error.to_string(),
                "status": 413
            })
            .to_string();

            header.insert_header("Content-Length", error_body.len().to_string())?;
            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(error_body.into()), true)
                .await?;

            self.metrics.increment_status_count(413);
            self.metrics.increment_security_payload_too_large();
            return Ok(true); // Short-circuit
        }

        // 4. Check for path traversal attempts (check RAW URI before normalization)
        // CRITICAL: Must check raw URI because path libraries normalize paths
        // /test/../etc/passwd gets normalized to /etc/passwd by uri.path()
        // We need to detect the attack BEFORE normalization
        if let Err(security_error) = security::check_path_traversal(&uri_str) {
            tracing::warn!(
                request_id = %ctx.request_id(),
                client_ip = %client_ip,
                uri = %uri_str,
                error = %security_error,
                "Path traversal attempt detected in raw URI"
            );

            let mut header = ResponseHeader::build(400, None)?;
            header.insert_header("Content-Type", "application/json")?;

            let error_body = serde_json::json!({
                "error": "Bad Request",
                "message": security_error.to_string(),
                "status": 400
            })
            .to_string();

            header.insert_header("Content-Length", error_body.len().to_string())?;
            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(error_body.into()), true)
                .await?;

            self.metrics.increment_status_count(400);
            self.metrics.increment_security_path_traversal_blocked();
            return Ok(true); // Short-circuit
        }

        // 5. Check for SQL injection attempts (check RAW URI before processing)
        if let Err(security_error) = security::check_sql_injection(&uri_str) {
            tracing::warn!(
                request_id = %ctx.request_id(),
                client_ip = %client_ip,
                uri = %uri_str,
                error = %security_error,
                "SQL injection attempt detected in raw URI"
            );

            let mut header = ResponseHeader::build(400, None)?;
            header.insert_header("Content-Type", "application/json")?;

            let error_body = serde_json::json!({
                "error": "Bad Request",
                "message": security_error.to_string(),
                "status": 400
            })
            .to_string();

            header.insert_header("Content-Length", error_body.len().to_string())?;
            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(error_body.into()), true)
                .await?;

            self.metrics.increment_status_count(400);
            self.metrics.increment_security_sql_injection_blocked();
            return Ok(true); // Short-circuit
        }

        // Record request metrics (conditionally based on resource pressure)
        if self.resource_monitor.metrics_enabled() {
            self.metrics.increment_request_count();
            self.metrics.increment_method_count(&method);

            // Phase 23: Track per-replica request count if replica was selected
            if let Some(bucket_config) = ctx.bucket_config() {
                if let Some(replica_name) = ctx.replica_name() {
                    self.metrics
                        .increment_replica_request_count(&bucket_config.name, replica_name);
                }
            }
        }

        // Special handling for /health endpoint (bypass auth, return health status)
        if path == "/health" {
            let uptime_seconds = self.start_time.elapsed().as_secs();
            let version = env!("CARGO_PKG_VERSION");

            let health_response = serde_json::json!({
                "status": "healthy",
                "uptime_seconds": uptime_seconds,
                "version": version
            })
            .to_string();

            let mut header = ResponseHeader::build(200, None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Content-Length", health_response.len().to_string())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(health_response.into()), true)
                .await?;

            // Record metrics for /health endpoint itself
            self.metrics.increment_status_count(200);

            return Ok(true); // Short-circuit (response already sent)
        }

        // Special handling for /ready endpoint (bypass auth, check S3 backend health with per-replica details)
        if path == "/ready" {
            // Check health of all S3 backends with per-replica granularity (Phase 23)
            let mut backends_health = serde_json::Map::new();
            let mut all_healthy = true;

            for bucket_config in &config.buckets {
                // Get the ReplicaSet for this bucket
                if let Some(replica_set) = self.replica_sets.get(&bucket_config.name) {
                    // Check health of each replica via circuit breaker state
                    let mut replicas_health = serde_json::Map::new();
                    let mut bucket_has_healthy_replica = false;

                    for replica in &replica_set.replicas {
                        // Check circuit breaker state to determine health
                        let is_healthy = replica.circuit_breaker.state()
                            == crate::circuit_breaker::CircuitState::Closed;

                        if is_healthy {
                            bucket_has_healthy_replica = true;
                        }

                        replicas_health.insert(
                            replica.name.clone(),
                            serde_json::Value::String(if is_healthy {
                                "healthy".to_string()
                            } else {
                                "unhealthy".to_string()
                            }),
                        );
                    }

                    // Determine overall bucket status
                    let bucket_status = if bucket_has_healthy_replica {
                        if replicas_health.values().all(|v| v == "healthy") {
                            "ready"
                        } else {
                            "degraded" // Some replicas unhealthy but at least one healthy
                        }
                    } else {
                        all_healthy = false;
                        "unavailable" // All replicas unhealthy
                    };

                    // Record backend health in metrics (for Prometheus export)
                    self.metrics
                        .set_backend_health(&bucket_config.name, bucket_has_healthy_replica);

                    // Build bucket health object with status and per-replica details
                    let mut bucket_health = serde_json::Map::new();
                    bucket_health.insert(
                        "status".to_string(),
                        serde_json::Value::String(bucket_status.to_string()),
                    );
                    bucket_health.insert(
                        "replicas".to_string(),
                        serde_json::Value::Object(replicas_health),
                    );

                    backends_health.insert(
                        bucket_config.name.clone(),
                        serde_json::Value::Object(bucket_health),
                    );
                } else {
                    // Fallback: No ReplicaSet found (shouldn't happen with proper config)
                    tracing::warn!(
                        bucket = %bucket_config.name,
                        "No ReplicaSet found for bucket, reporting as unavailable"
                    );
                    all_healthy = false;

                    let mut bucket_health = serde_json::Map::new();
                    bucket_health.insert(
                        "status".to_string(),
                        serde_json::Value::String("unavailable".to_string()),
                    );
                    backends_health.insert(
                        bucket_config.name.clone(),
                        serde_json::Value::Object(bucket_health),
                    );
                }
            }

            let status_code: u16 = if all_healthy { 200 } else { 503 };
            let ready_response = serde_json::json!({
                "status": if all_healthy { "ready" } else { "unavailable" },
                "backends": backends_health
            })
            .to_string();

            let mut header = ResponseHeader::build(status_code, None)?;
            header.insert_header("Content-Type", "application/json")?;
            header.insert_header("Content-Length", ready_response.len().to_string())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(ready_response.into()), true)
                .await?;

            // Record metrics for /ready endpoint itself
            self.metrics.increment_status_count(status_code);

            return Ok(true); // Short-circuit (response already sent)
        }

        // Special handling for /metrics endpoint (bypass auth, return Prometheus metrics)
        if path == "/metrics" {
            let mut metrics_output = self.metrics.export_prometheus();

            // Append circuit breaker metrics for each bucket
            metrics_output.push_str(&self.export_circuit_breaker_metrics());

            let mut header = ResponseHeader::build(200, None)?;
            header.insert_header("Content-Type", "text/plain; version=0.0.4")?;
            header.insert_header("Content-Length", metrics_output.len().to_string())?;

            session
                .write_response_header(Box::new(header), false)
                .await?;
            session
                .write_response_body(Some(metrics_output.into()), true)
                .await?;

            // Record metrics for /metrics endpoint itself
            self.metrics.increment_status_count(200);

            return Ok(true); // Short-circuit (response already sent)
        }

        // Admin API Router (Phase 1)
        // Delegates to admin module for centralized handling and authentication
        // Note: Returns true if handled, false if not handled (allowing legacy fallbacks)
        if crate::admin::is_handled_path(&path) {
            let headers_map = Self::extract_headers(req);
            let query_map = Self::extract_query_params(req);

            crate::admin::handle_request(
                session,
                &path,
                &method,
                &headers_map,
                &query_map,
                &config,
                &self.metrics,
                &self.prewarm_manager,
            )
            .await;

            return Ok(true);
        }

        // Special handling for /admin/reload endpoint (config hot reload)
        if path == "/admin/reload" && method == "POST" {
            if let Some(_reload_manager) = &self.reload_manager {
                // Check authentication if JWT is enabled
                if let Some(jwt_config) = &config.jwt {
                    if jwt_config.enabled {
                        // Extract headers and query params
                        let headers = Self::extract_headers(req);
                        let query_params = Self::extract_query_params(req);

                        // Authenticate request
                        match authenticate_request(&headers, &query_params, jwt_config) {
                            Ok(_claims) => {
                                tracing::debug!(
                                    request_id = %ctx.request_id(),
                                    "Admin reload request authenticated successfully"
                                );
                            }
                            Err(auth_error) => {
                                tracing::warn!(
                                    request_id = %ctx.request_id(),
                                    error = %auth_error,
                                    "Admin reload authentication failed"
                                );

                                // Build 401 Unauthorized response
                                let response_json = serde_json::json!({
                                    "status": "error",
                                    "message": format!("Authentication required: {}", auth_error),
                                });

                                let response_body = response_json.to_string();

                                let mut header = ResponseHeader::build(401, None)?;
                                header.insert_header("Content-Type", "application/json")?;
                                header.insert_header(
                                    "Content-Length",
                                    response_body.len().to_string(),
                                )?;

                                session
                                    .write_response_header(Box::new(header), false)
                                    .await?;
                                session
                                    .write_response_body(Some(response_body.into()), true)
                                    .await?;

                                // Record metrics
                                self.metrics.increment_status_count(401);

                                return Ok(true); // Short-circuit
                            }
                        }
                    }
                }

                // Attempt to reload configuration
                match self.reload_configuration() {
                    Ok(new_generation) => {
                        tracing::info!(
                            request_id = %ctx.request_id(),
                            new_generation = new_generation,
                            "Configuration reloaded successfully via API"
                        );

                        // Build success response JSON
                        let response_json = serde_json::json!({
                            "status": "success",
                            "message": "Configuration reloaded successfully",
                            "config_generation": new_generation,
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(200, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;

                        // Record metrics
                        self.metrics.increment_status_count(200);

                        return Ok(true); // Short-circuit
                    }
                    Err(error_msg) => {
                        tracing::error!(
                            request_id = %ctx.request_id(),
                            error = %error_msg,
                            "Configuration reload failed"
                        );

                        // Record reload failure metrics
                        self.metrics.increment_reload_failure();

                        // Build error response JSON
                        let response_json = serde_json::json!({
                            "status": "error",
                            "message": "Configuration reload failed",
                            "error": error_msg,
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(400, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;

                        // Record metrics
                        self.metrics.increment_status_count(400);

                        return Ok(true); // Short-circuit
                    }
                }
            } else {
                // Reload manager not configured
                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Hot reload not enabled",
                });

                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(503, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;

                // Record metrics
                self.metrics.increment_status_count(503);

                return Ok(true); // Short-circuit
            }
        }

        // Special handling for /admin/cache/purge endpoint (Phase 30: Cache Management API)
        if path == "/admin/cache/purge" && method == "POST" {
            // Check if cache is enabled
            if let Some(ref cache) = self.cache {
                // Check authentication if JWT is enabled
                if let Some(jwt_config) = &config.jwt {
                    if jwt_config.enabled {
                        // Extract headers and query params
                        let headers = Self::extract_headers(req);
                        let query_params = Self::extract_query_params(req);

                        // Authenticate request
                        match authenticate_request(&headers, &query_params, jwt_config) {
                            Ok(claims) => {
                                // Phase 65.1: Verify admin claims
                                if !crate::auth::verify_admin_claims(
                                    &claims,
                                    &jwt_config.admin_claims,
                                ) {
                                    tracing::warn!(
                                        request_id = %ctx.request_id(),
                                        "Cache purge admin claims verification failed"
                                    );

                                    let response_json = serde_json::json!({
                                        "status": "error",
                                        "message": "Admin access denied: insufficient privileges",
                                    });

                                    let response_body = response_json.to_string();

                                    let mut header = ResponseHeader::build(403, None)?;
                                    header.insert_header("Content-Type", "application/json")?;
                                    header.insert_header(
                                        "Content-Length",
                                        response_body.len().to_string(),
                                    )?;

                                    session
                                        .write_response_header(Box::new(header), false)
                                        .await?;
                                    session
                                        .write_response_body(Some(response_body.into()), true)
                                        .await?;

                                    self.metrics.increment_status_count(403);
                                    return Ok(true);
                                }

                                tracing::debug!(
                                    request_id = %ctx.request_id(),
                                    "Cache purge request authenticated successfully"
                                );
                            }
                            Err(auth_error) => {
                                tracing::warn!(
                                    request_id = %ctx.request_id(),
                                    error = %auth_error,
                                    "Cache purge authentication failed"
                                );

                                // Build 401 Unauthorized response
                                let response_json = serde_json::json!({
                                    "status": "error",
                                    "message": format!("Authentication required: {}", auth_error),
                                });

                                let response_body = response_json.to_string();

                                let mut header = ResponseHeader::build(401, None)?;
                                header.insert_header("Content-Type", "application/json")?;
                                header.insert_header(
                                    "Content-Length",
                                    response_body.len().to_string(),
                                )?;

                                session
                                    .write_response_header(Box::new(header), false)
                                    .await?;
                                session
                                    .write_response_body(Some(response_body.into()), true)
                                    .await?;

                                // Record metrics
                                self.metrics.increment_status_count(401);

                                return Ok(true); // Short-circuit
                            }
                        }
                    }
                }

                // Purge cache (clear all layers)
                match cache.clear().await {
                    Ok(()) => {
                        tracing::info!(
                            request_id = %ctx.request_id(),
                            "Cache purged successfully (all layers cleared)"
                        );

                        // Build success response JSON
                        let response_json = serde_json::json!({
                            "status": "success",
                            "message": "Cache purged successfully",
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(200, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;

                        // Record metrics
                        self.metrics.increment_status_count(200);
                        self.metrics.increment_cache_purge();

                        return Ok(true); // Short-circuit
                    }
                    Err(e) => {
                        tracing::error!(
                            request_id = %ctx.request_id(),
                            error = %e,
                            "Failed to purge cache"
                        );

                        // Build error response JSON
                        let response_json = serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to purge cache: {}", e),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(500, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;

                        // Record metrics
                        self.metrics.increment_status_count(500);

                        return Ok(true); // Short-circuit
                    }
                }
            } else {
                // Cache not enabled
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    "Cache purge requested but cache is not enabled"
                );

                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Cache is not enabled",
                });

                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;

                // Record metrics
                self.metrics.increment_status_count(404);

                return Ok(true); // Short-circuit
            }
        }

        // Special handling for /admin/cache/purge/:bucket/*path endpoint (Phase 30.6: Bucket-level purge)
        // Path format: /admin/cache/purge/{bucket}/{object_path}
        if path.starts_with("/admin/cache/purge/")
            && method == "POST"
            && path != "/admin/cache/purge"
        {
            // Parse bucket and optional object path from URL
            let parts: Vec<&str> = path
                .strip_prefix("/admin/cache/purge/")
                .unwrap()
                .splitn(2, '/')
                .collect();
            let bucket_name = parts[0];
            let object_path = parts.get(1).map(|s| format!("/{}", s));

            if let Some(ref cache) = self.cache {
                // Check authentication if JWT is enabled
                if let Some(jwt_config) = &config.jwt {
                    if jwt_config.enabled {
                        let headers = Self::extract_headers(req);
                        let query_params = Self::extract_query_params(req);

                        match authenticate_request(&headers, &query_params, jwt_config) {
                            Ok(claims) => {
                                // Phase 65.1: Verify admin claims
                                if !crate::auth::verify_admin_claims(
                                    &claims,
                                    &jwt_config.admin_claims,
                                ) {
                                    tracing::warn!(
                                        request_id = %ctx.request_id(),
                                        bucket = %bucket_name,
                                        "Cache purge admin claims verification failed"
                                    );

                                    let response_json = serde_json::json!({
                                        "status": "error",
                                        "message": "Admin access denied: insufficient privileges",
                                    });
                                    let response_body = response_json.to_string();

                                    let mut header = ResponseHeader::build(403, None)?;
                                    header.insert_header("Content-Type", "application/json")?;
                                    header.insert_header(
                                        "Content-Length",
                                        response_body.len().to_string(),
                                    )?;

                                    session
                                        .write_response_header(Box::new(header), false)
                                        .await?;
                                    session
                                        .write_response_body(Some(response_body.into()), true)
                                        .await?;
                                    self.metrics.increment_status_count(403);
                                    return Ok(true);
                                }

                                tracing::debug!(
                                    request_id = %ctx.request_id(),
                                    bucket = %bucket_name,
                                    "Cache purge request authenticated successfully"
                                );
                            }
                            Err(auth_error) => {
                                tracing::warn!(
                                    request_id = %ctx.request_id(),
                                    error = %auth_error,
                                    "Cache purge authentication failed"
                                );

                                let response_json = serde_json::json!({
                                    "status": "error",
                                    "message": format!("Authentication required: {}", auth_error),
                                });
                                let response_body = response_json.to_string();

                                let mut header = ResponseHeader::build(401, None)?;
                                header.insert_header("Content-Type", "application/json")?;
                                header.insert_header(
                                    "Content-Length",
                                    response_body.len().to_string(),
                                )?;

                                session
                                    .write_response_header(Box::new(header), false)
                                    .await?;
                                session
                                    .write_response_body(Some(response_body.into()), true)
                                    .await?;
                                self.metrics.increment_status_count(401);
                                return Ok(true);
                            }
                        }
                    }
                }

                // Handle purge based on whether object_path is provided
                if let Some(obj_path) = object_path {
                    // Purge specific object: /admin/cache/purge/:bucket/*path
                    let cache_key = CacheKey {
                        bucket: bucket_name.to_string(),
                        object_key: obj_path.clone(),
                        etag: None,
                    };

                    match cache.delete(&cache_key).await {
                        Ok(deleted) => {
                            tracing::info!(
                                request_id = %ctx.request_id(),
                                bucket = %bucket_name,
                                object_path = %obj_path,
                                deleted = deleted,
                                "Cache entry purged"
                            );

                            let response_json = serde_json::json!({
                                "status": "success",
                                "message": if deleted { "Cache entry purged" } else { "Cache entry not found" },
                                "bucket": bucket_name,
                                "path": obj_path,
                                "deleted": deleted,
                            });
                            let response_body = response_json.to_string();

                            let mut header = ResponseHeader::build(200, None)?;
                            header.insert_header("Content-Type", "application/json")?;
                            header
                                .insert_header("Content-Length", response_body.len().to_string())?;

                            session
                                .write_response_header(Box::new(header), false)
                                .await?;
                            session
                                .write_response_body(Some(response_body.into()), true)
                                .await?;
                            self.metrics.increment_status_count(200);
                            if deleted {
                                self.metrics.increment_cache_purge();
                            }
                            return Ok(true);
                        }
                        Err(e) => {
                            tracing::error!(
                                request_id = %ctx.request_id(),
                                error = %e,
                                "Failed to purge cache entry"
                            );

                            let response_json = serde_json::json!({
                                "status": "error",
                                "message": format!("Failed to purge cache entry: {}", e),
                            });
                            let response_body = response_json.to_string();

                            let mut header = ResponseHeader::build(500, None)?;
                            header.insert_header("Content-Type", "application/json")?;
                            header
                                .insert_header("Content-Length", response_body.len().to_string())?;

                            session
                                .write_response_header(Box::new(header), false)
                                .await?;
                            session
                                .write_response_body(Some(response_body.into()), true)
                                .await?;
                            self.metrics.increment_status_count(500);
                            return Ok(true);
                        }
                    }
                } else {
                    // Purge entire bucket: /admin/cache/purge/:bucket
                    tracing::info!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_name,
                        "Purging all cache entries for bucket"
                    );

                    match cache.clear_bucket(bucket_name).await {
                        Ok(deleted_count) => {
                            let response_json = serde_json::json!({
                                "status": "success",
                                "message": format!("Purged {} cache entries for bucket", deleted_count),
                                "bucket": bucket_name,
                                "entries_deleted": deleted_count,
                            });
                            let response_body = response_json.to_string();

                            let mut header = ResponseHeader::build(200, None)?;
                            header.insert_header("Content-Type", "application/json")?;
                            header
                                .insert_header("Content-Length", response_body.len().to_string())?;

                            session
                                .write_response_header(Box::new(header), false)
                                .await?;
                            session
                                .write_response_body(Some(response_body.into()), true)
                                .await?;
                            self.metrics.increment_status_count(200);
                            return Ok(true);
                        }
                        Err(e) => {
                            let response_json = serde_json::json!({
                                "status": "error",
                                "message": format!("Failed to purge cache: {}", e),
                            });
                            let response_body = response_json.to_string();

                            let mut header = ResponseHeader::build(500, None)?;
                            header.insert_header("Content-Type", "application/json")?;
                            header
                                .insert_header("Content-Length", response_body.len().to_string())?;

                            session
                                .write_response_header(Box::new(header), false)
                                .await?;
                            session
                                .write_response_body(Some(response_body.into()), true)
                                .await?;
                            self.metrics.increment_status_count(500);
                            return Ok(true);
                        }
                    }
                }
            } else {
                // Cache not enabled
                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Cache is not enabled",
                });
                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;
                self.metrics.increment_status_count(404);
                return Ok(true);
            }
        }

        // Special handling for /admin/cache/stats endpoint (Phase 30: Cache Management API)
        if path == "/admin/cache/stats" && method == "GET" {
            // Check if cache is enabled
            if let Some(ref cache) = self.cache {
                // Check authentication if JWT is enabled
                if let Some(jwt_config) = &config.jwt {
                    if jwt_config.enabled {
                        // Extract headers and query params
                        let headers = Self::extract_headers(req);
                        let query_params = Self::extract_query_params(req);

                        // Authenticate request
                        match authenticate_request(&headers, &query_params, jwt_config) {
                            Ok(_claims) => {
                                tracing::debug!(
                                    request_id = %ctx.request_id(),
                                    "Cache stats request authenticated successfully"
                                );
                            }
                            Err(auth_error) => {
                                tracing::warn!(
                                    request_id = %ctx.request_id(),
                                    error = %auth_error,
                                    "Cache stats authentication failed"
                                );

                                // Build 401 Unauthorized response
                                let response_json = serde_json::json!({
                                    "status": "error",
                                    "message": format!("Authentication required: {}", auth_error),
                                });

                                let response_body = response_json.to_string();

                                let mut header = ResponseHeader::build(401, None)?;
                                header.insert_header("Content-Type", "application/json")?;
                                header.insert_header(
                                    "Content-Length",
                                    response_body.len().to_string(),
                                )?;

                                session
                                    .write_response_header(Box::new(header), false)
                                    .await?;
                                session
                                    .write_response_body(Some(response_body.into()), true)
                                    .await?;

                                // Record metrics
                                self.metrics.increment_status_count(401);

                                return Ok(true); // Short-circuit
                            }
                        }
                    }
                }

                // Get cache statistics
                match cache.stats().await {
                    Ok(stats) => {
                        tracing::debug!(
                            request_id = %ctx.request_id(),
                            hits = stats.hits,
                            misses = stats.misses,
                            evictions = stats.evictions,
                            current_size_bytes = stats.current_size_bytes,
                            current_item_count = stats.current_item_count,
                            max_size_bytes = stats.max_size_bytes,
                            "Cache stats retrieved successfully"
                        );

                        // Calculate hit rate
                        let total_requests = stats.hits + stats.misses;
                        let hit_rate = if total_requests > 0 {
                            (stats.hits as f64) / (total_requests as f64)
                        } else {
                            0.0
                        };

                        // Build success response JSON
                        let response_json = serde_json::json!({
                            "status": "success",
                            "data": {
                                "hits": stats.hits,
                                "misses": stats.misses,
                                "hit_rate": format!("{:.4}", hit_rate),
                                "evictions": stats.evictions,
                                "current_size_bytes": stats.current_size_bytes,
                                "current_item_count": stats.current_item_count,
                                "max_size_bytes": stats.max_size_bytes,
                            },
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(200, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;

                        // Record metrics
                        self.metrics.increment_status_count(200);

                        return Ok(true); // Short-circuit
                    }
                    Err(e) => {
                        tracing::error!(
                            request_id = %ctx.request_id(),
                            error = %e,
                            "Failed to retrieve cache stats"
                        );

                        // Build error response JSON
                        let response_json = serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to retrieve cache stats: {}", e),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(500, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;

                        // Record metrics
                        self.metrics.increment_status_count(500);

                        return Ok(true); // Short-circuit
                    }
                }
            } else {
                // Cache not enabled
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    "Cache stats requested but cache is not enabled"
                );

                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Cache is not enabled",
                });

                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;

                // Record metrics
                self.metrics.increment_status_count(404);

                return Ok(true); // Short-circuit
            }
        }

        // Special handling for /admin/cache/stats/:bucket endpoint (Phase 30.5: Per-Bucket Stats)
        if path.starts_with("/admin/cache/stats/")
            && method == "GET"
            && path != "/admin/cache/stats"
        {
            // Extract bucket name from path
            let bucket_name = path.strip_prefix("/admin/cache/stats/").unwrap_or("");

            if bucket_name.is_empty() {
                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Bucket name is required",
                });
                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(400, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;
                self.metrics.increment_status_count(400);
                return Ok(true);
            }

            if let Some(ref cache) = self.cache {
                // Get bucket-specific statistics
                match cache.stats_bucket(bucket_name).await {
                    Ok(stats) => {
                        tracing::debug!(
                            request_id = %ctx.request_id(),
                            bucket = %bucket_name,
                            current_size_bytes = stats.current_size_bytes,
                            current_item_count = stats.current_item_count,
                            "Bucket cache stats retrieved successfully"
                        );

                        let response_json = serde_json::json!({
                            "status": "success",
                            "bucket": bucket_name,
                            "data": {
                                "current_size_bytes": stats.current_size_bytes,
                                "current_item_count": stats.current_item_count,
                            },
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(200, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;
                        self.metrics.increment_status_count(200);
                        return Ok(true);
                    }
                    Err(e) => {
                        tracing::error!(
                            request_id = %ctx.request_id(),
                            bucket = %bucket_name,
                            error = %e,
                            "Failed to retrieve bucket cache stats"
                        );

                        let response_json = serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to retrieve bucket cache stats: {}", e),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(500, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;
                        self.metrics.increment_status_count(500);
                        return Ok(true);
                    }
                }
            } else {
                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Cache is not enabled",
                });

                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;
                self.metrics.increment_status_count(404);
                return Ok(true);
            }
        }

        // Special handling for /admin/cache/info endpoint (Phase 36: Cache Entry Info API)
        if path == "/admin/cache/info" && method == "GET" {
            // Extract key from query params
            let query_params = Self::extract_query_params(req);

            if let Some(ref cache) = self.cache {
                // Check authentication if JWT is enabled
                if let Some(jwt_config) = &config.jwt {
                    if jwt_config.enabled {
                        // Extract headers for auth
                        let headers = Self::extract_headers(req);

                        // Authenticate request
                        match authenticate_request(&headers, &query_params, jwt_config) {
                            Ok(_claims) => {
                                tracing::debug!(
                                    request_id = %ctx.request_id(),
                                    "Cache info request authenticated successfully"
                                );
                            }
                            Err(auth_error) => {
                                tracing::warn!(
                                    request_id = %ctx.request_id(),
                                    error = %auth_error,
                                    "Cache info authentication failed"
                                );

                                // Build 401 Unauthorized response
                                let response_json = serde_json::json!({
                                    "status": "error",
                                    "message": format!("Authentication required: {}", auth_error),
                                });

                                let response_body = response_json.to_string();

                                let mut header = ResponseHeader::build(401, None)?;
                                header.insert_header("Content-Type", "application/json")?;
                                header.insert_header(
                                    "Content-Length",
                                    response_body.len().to_string(),
                                )?;

                                session
                                    .write_response_header(Box::new(header), false)
                                    .await?;
                                session
                                    .write_response_body(Some(response_body.into()), true)
                                    .await?;

                                // Record metrics
                                self.metrics.increment_status_count(401);

                                return Ok(true); // Short-circuit
                            }
                        }
                    }
                }

                // Get the key from query params (format: bucket:object_key)
                let key_param = query_params.get("key");

                if key_param.is_none() || key_param.unwrap().is_empty() {
                    let response_json = serde_json::json!({
                        "status": "error",
                        "message": "Missing required parameter 'key'. Format: bucket:object_key",
                    });
                    let response_body = response_json.to_string();

                    let mut header = ResponseHeader::build(400, None)?;
                    header.insert_header("Content-Type", "application/json")?;
                    header.insert_header("Content-Length", response_body.len().to_string())?;

                    session
                        .write_response_header(Box::new(header), false)
                        .await?;
                    session
                        .write_response_body(Some(response_body.into()), true)
                        .await?;
                    self.metrics.increment_status_count(400);
                    return Ok(true);
                }

                let key_str = key_param.unwrap();

                // Parse key format: bucket:object_key
                let parts: Vec<&str> = key_str.splitn(2, ':').collect();
                if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                    let response_json = serde_json::json!({
                        "status": "error",
                        "message": "Invalid key format. Expected: bucket:object_key",
                    });
                    let response_body = response_json.to_string();

                    let mut header = ResponseHeader::build(400, None)?;
                    header.insert_header("Content-Type", "application/json")?;
                    header.insert_header("Content-Length", response_body.len().to_string())?;

                    session
                        .write_response_header(Box::new(header), false)
                        .await?;
                    session
                        .write_response_body(Some(response_body.into()), true)
                        .await?;
                    self.metrics.increment_status_count(400);
                    return Ok(true);
                }

                let bucket = parts[0];
                let object_key = parts[1];

                // Build cache key
                let cache_key = CacheKey {
                    bucket: bucket.to_string(),
                    object_key: object_key.to_string(),
                    etag: None,
                };

                // Try to get the entry from cache
                match cache.get(&cache_key).await {
                    Ok(Some(entry)) => {
                        tracing::debug!(
                            request_id = %ctx.request_id(),
                            bucket = %bucket,
                            object_key = %object_key,
                            "Cache entry info retrieved successfully"
                        );

                        // Convert timestamps to Unix seconds
                        let created_at_secs = entry
                            .created_at
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let expires_at_secs = entry
                            .expires_at
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let last_accessed_at_secs = entry
                            .last_accessed_at
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        let response_json = serde_json::json!({
                            "status": "success",
                            "key": key_str,
                            "data": {
                                "bucket": bucket,
                                "object_key": object_key,
                                "content_type": entry.content_type,
                                "content_length": entry.content_length,
                                "etag": entry.etag,
                                "last_modified": entry.last_modified,
                                "created_at": created_at_secs,
                                "expires_at": expires_at_secs,
                                "last_accessed_at": last_accessed_at_secs,
                            },
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(200, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;
                        self.metrics.increment_status_count(200);
                        return Ok(true);
                    }
                    Ok(None) => {
                        tracing::debug!(
                            request_id = %ctx.request_id(),
                            bucket = %bucket,
                            object_key = %object_key,
                            "Cache entry not found"
                        );

                        let response_json = serde_json::json!({
                            "status": "error",
                            "message": "Cache entry not found",
                            "key": key_str,
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(404, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;
                        self.metrics.increment_status_count(404);
                        return Ok(true);
                    }
                    Err(e) => {
                        tracing::error!(
                            request_id = %ctx.request_id(),
                            bucket = %bucket,
                            object_key = %object_key,
                            error = %e,
                            "Failed to retrieve cache entry info"
                        );

                        let response_json = serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to retrieve cache entry: {}", e),
                        });

                        let response_body = response_json.to_string();

                        let mut header = ResponseHeader::build(500, None)?;
                        header.insert_header("Content-Type", "application/json")?;
                        header.insert_header("Content-Length", response_body.len().to_string())?;

                        session
                            .write_response_header(Box::new(header), false)
                            .await?;
                        session
                            .write_response_body(Some(response_body.into()), true)
                            .await?;
                        self.metrics.increment_status_count(500);
                        return Ok(true);
                    }
                }
            } else {
                let response_json = serde_json::json!({
                    "status": "error",
                    "message": "Cache is not enabled",
                });

                let response_body = response_json.to_string();

                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Content-Length", response_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(response_body.into()), true)
                    .await?;
                self.metrics.increment_status_count(404);
                return Ok(true);
            }
        }

        // Update context with request details
        ctx.set_method(method);
        ctx.set_path(path.clone());
        ctx.set_headers(Self::extract_headers(req));
        ctx.set_query_params(Self::extract_query_params(req));

        // Route request to bucket
        let bucket_config = match router.route(&path) {
            Some(config) => config,
            None => {
                // No matching bucket found - return 404
                let mut header = ResponseHeader::build(404, None)?;
                header.insert_header("Content-Type", "text/plain")?;
                session
                    .write_response_header(Box::new(header), true)
                    .await?;

                // Record 404 metrics
                self.metrics.increment_status_count(404);

                return Ok(true); // Short-circuit
            }
        };

        // Store bucket config in context
        ctx.set_bucket_config(bucket_config.clone());

        // -- Audit Logging: Populate bucket and key --
        if self.audit_writer.is_some() {
            let audit_ctx = ctx.audit();
            audit_ctx.bucket = Some(bucket_config.name.clone());
            let s3_key = router.extract_s3_key(&path).unwrap_or_default();
            audit_ctx.object_key = Some(s3_key);
        }
        // -- End Audit Logging --

        // Record bucket metrics
        self.metrics.increment_bucket_count(&bucket_config.name);

        // THIRD: Check rate limits (if enabled)
        if let Some(ref rate_limit_manager) = self.rate_limit_manager {
            // Get client IP from session (X-Forwarded-For aware for logging)
            let client_ip_str = self.get_client_ip(session);

            // Get IP for rate limiting (uses direct connection IP for security)
            // Note: For rate limiting, we use direct connection IP to prevent spoofing
            let client_ip = session
                .client_addr()
                .and_then(|addr| addr.as_inet().map(|inet| inet.ip()));

            // Check all rate limits (global, per-IP, per-bucket)
            if let Err(rate_limit_error) =
                rate_limit_manager.check_all(&bucket_config.name, client_ip)
            {
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    client_ip = %client_ip_str,
                    direct_ip = ?client_ip,
                    error = %rate_limit_error,
                    "Rate limit exceeded"
                );

                // Increment rate limit exceeded metrics
                self.metrics
                    .increment_rate_limit_exceeded(&bucket_config.name);

                let mut header = ResponseHeader::build(429, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Retry-After", "1")?; // Suggest retry after 1 second

                let error_body = serde_json::json!({
                    "error": "Too Many Requests",
                    "message": rate_limit_error.to_string(),
                    "status": 429
                })
                .to_string();

                header.insert_header("Content-Length", error_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(error_body.into()), true)
                    .await?;

                // Record 429 status
                self.metrics.increment_status_count(429);

                return Ok(true); // Request handled
            }
        }

        // FOURTH: Check circuit breaker for this bucket (if configured)
        if let Some(circuit_breaker) = self.circuit_breakers.get(&bucket_config.name) {
            // Check if circuit breaker allows request
            if !circuit_breaker.should_allow_request() {
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    state = ?circuit_breaker.state(),
                    "Circuit breaker rejecting request (circuit open)"
                );

                let mut header = ResponseHeader::build(503, None)?;
                header.insert_header("Content-Type", "application/json")?;
                header.insert_header("Retry-After", "60")?; // Suggest retry after circuit timeout

                let error_body = serde_json::json!({
                    "error": "Service Temporarily Unavailable",
                    "message": "S3 backend is experiencing issues. Circuit breaker is open.",
                    "bucket": bucket_config.name,
                    "status": 503
                })
                .to_string();

                header.insert_header("Content-Length", error_body.len().to_string())?;

                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session
                    .write_response_body(Some(error_body.into()), true)
                    .await?;

                self.metrics.increment_status_count(503);

                return Ok(true); // Request handled (circuit breaker rejected)
            }

            // If we're in half-open state, increment request counter
            circuit_breaker.start_half_open_request();
        }

        // Check if authentication is required
        if let Some(auth_config) = &bucket_config.auth {
            if auth_config.enabled {
                if let Some(jwt_config) = &config.jwt {
                    // Authenticate request
                    let headers = ctx.headers();
                    let query_params = ctx.query_params();

                    match authenticate_request(headers, query_params, jwt_config) {
                        Ok(claims) => {
                            if self.audit_writer.is_some() {
                                ctx.audit().user = claims.sub.clone();
                            }
                            ctx.set_claims(claims);
                            // Record successful authentication
                            self.metrics.increment_auth_success();
                        }
                        Err(AuthError::MissingToken) => {
                            // Return 401 Unauthorized
                            let mut header = ResponseHeader::build(401, None)?;
                            header.insert_header("Content-Type", "text/plain")?;
                            header.insert_header("WWW-Authenticate", "Bearer")?;
                            session
                                .write_response_header(Box::new(header), true)
                                .await?;

                            // Record authentication failure
                            self.metrics.increment_auth_failure();
                            self.metrics.increment_auth_error("missing");
                            self.metrics.increment_status_count(401);

                            return Ok(true); // Short-circuit
                        }
                        Err(_) => {
                            // Return 403 Forbidden (invalid token or claims)
                            let mut header = ResponseHeader::build(403, None)?;
                            header.insert_header("Content-Type", "text/plain")?;
                            session
                                .write_response_header(Box::new(header), true)
                                .await?;

                            // Record authentication failure
                            self.metrics.increment_auth_failure();
                            self.metrics.increment_auth_error("invalid");
                            self.metrics.increment_status_count(403);

                            return Ok(true); // Short-circuit
                        }
                    }
                }
            }
        } else {
            // Authentication bypassed (public bucket - no auth config)
            self.metrics.increment_auth_bypassed();
        }

        // Phase 32: OPA Authorization check (after JWT authentication)
        if let Some(opa_client) = self.opa_clients.get(&bucket_config.name) {
            // Get authorization config for fail mode
            let fail_mode = bucket_config
                .authorization
                .as_ref()
                .and_then(|a| a.opa_fail_mode.as_ref())
                .map(|s| OpaFailMode::from_str(s).unwrap_or_default())
                .unwrap_or_default();

            // Build OPA input from request context
            let jwt_claims = ctx
                .claims()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .unwrap_or(serde_json::json!({}));
            let opa_input = OpaInput::new(
                jwt_claims,
                bucket_config.name.clone(),
                ctx.path().to_string(),
                ctx.method().to_string(),
                ctx.headers().get("x-forwarded-for").cloned(),
            );

            // Check cache first
            let cache_key = opa_input.cache_key();
            let cached_decision = if let Some(ref opa_cache) = self.opa_cache {
                opa_cache.get(&cache_key).await
            } else {
                None
            };

            let decision = if let Some(allowed) = cached_decision {
                // Cache hit
                tracing::debug!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    allowed = %allowed,
                    "OPA authorization decision from cache"
                );
                OpaAuthorizationDecision::from_opa_result(Ok(allowed), fail_mode)
            } else {
                // Cache miss - call OPA
                let eval_result = opa_client.evaluate(&opa_input).await;
                let decision =
                    OpaAuthorizationDecision::from_opa_result(eval_result.clone(), fail_mode);

                // Cache the result on success
                if let (Ok(allowed), Some(ref opa_cache)) = (eval_result, &self.opa_cache) {
                    opa_cache.put(cache_key, allowed).await;
                }

                tracing::debug!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    allowed = %decision.is_allowed(),
                    fail_open = %decision.is_fail_open_allow(),
                    "OPA authorization decision"
                );

                decision
            };

            // Log warning for fail-open decisions
            if decision.is_fail_open_allow() {
                if let Some(error) = decision.error() {
                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_config.name,
                        error = %error,
                        "OPA authorization failed but allowing due to fail-open mode"
                    );
                }
            }

            // Deny if not allowed
            if !decision.is_allowed() {
                let mut header = ResponseHeader::build(403, None)?;
                header.insert_header("Content-Type", "text/plain")?;
                session
                    .write_response_header(Box::new(header), true)
                    .await?;

                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    "OPA authorization denied"
                );

                self.metrics.increment_status_count(403);
                return Ok(true); // Short-circuit
            }
        }

        // Phase 49: OpenFGA Authorization check (after JWT authentication and OPA)
        if let Some(openfga_client) = self.openfga_clients.get(&bucket_config.name) {
            // Get authorization config for fail mode
            let fail_mode = bucket_config
                .authorization
                .as_ref()
                .and_then(|a| a.openfga_fail_mode.as_ref())
                .map(|s| OpenFgaFailMode::from_str(s).unwrap_or_default())
                .unwrap_or_default();

            // Extract user ID from JWT claims
            let jwt_claims = ctx
                .claims()
                .map(|c| serde_json::to_value(c).unwrap_or_default())
                .unwrap_or(serde_json::json!({}));

            // Get the claim name to extract user ID from (default: "sub")
            let user_claim = bucket_config
                .authorization
                .as_ref()
                .and_then(|a| a.openfga_user_claim.as_deref());

            let user_id = extract_user_id(&jwt_claims, user_claim);

            if let Some(user) = user_id {
                // Build OpenFGA object from bucket and path
                let object_path = router.extract_s3_key(ctx.path()).unwrap_or_default();
                let object = build_openfga_object(&bucket_config.name, &object_path);

                // Map HTTP method to relation (GET/HEAD→viewer, PUT/POST→editor, DELETE→owner)
                let relation = http_method_to_relation(ctx.method());

                // Perform authorization check
                let check_result = openfga_client
                    .check(&user, relation.as_str(), &object)
                    .await;

                let decision =
                    OpenFgaAuthorizationDecision::from_check_result(check_result, fail_mode);

                tracing::debug!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    user = %user,
                    object = %object,
                    relation = %relation.as_str(),
                    allowed = %decision.is_allowed(),
                    fail_open = %decision.is_fail_open_allow(),
                    "OpenFGA authorization decision"
                );

                // Log warning for fail-open decisions
                if decision.is_fail_open_allow() {
                    if let Some(error) = decision.error() {
                        tracing::warn!(
                            request_id = %ctx.request_id(),
                            bucket = %bucket_config.name,
                            user = %user,
                            error = %error,
                            "OpenFGA authorization failed but allowing due to fail-open mode"
                        );
                    }
                }

                // Deny if not allowed
                if !decision.is_allowed() {
                    let mut header = ResponseHeader::build(403, None)?;
                    header.insert_header("Content-Type", "text/plain")?;
                    session
                        .write_response_header(Box::new(header), true)
                        .await?;

                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_config.name,
                        user = %user,
                        object = %object,
                        relation = %relation.as_str(),
                        "OpenFGA authorization denied"
                    );

                    self.metrics.increment_status_count(403);
                    return Ok(true); // Short-circuit
                }
            } else {
                // No user ID found in claims - deny or fail-open based on config
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_config.name,
                    "OpenFGA authorization failed: no user ID in JWT claims"
                );

                // Default to deny if no user ID (security-first approach)
                if fail_mode == OpenFgaFailMode::Closed {
                    let mut header = ResponseHeader::build(403, None)?;
                    header.insert_header("Content-Type", "text/plain")?;
                    session
                        .write_response_header(Box::new(header), true)
                        .await?;

                    self.metrics.increment_status_count(403);
                    return Ok(true); // Short-circuit
                }
                // If fail-open, continue to allow the request
            }
        }

        // FOURTH: Check cache (Phase 30.7: Cache Integration)
        if let Some(ref cache) = self.cache {
            // Check cache for GET and HEAD requests
            // GET: Return full response (headers + body)
            // HEAD: Return headers only (no body) - useful for metadata checks
            // Check method and extract HEAD flag before any mutable ctx borrows
            let is_get_or_head = ctx.method() == "GET" || ctx.method() == "HEAD";
            let is_head_request = ctx.method() == "HEAD";
            if is_get_or_head {
                // Cache Bypass Logic: Range requests always bypass cache
                // Range requests are for partial content (video seeking, parallel downloads)
                // and we don't cache partial responses
                let is_range_request =
                    ctx.headers().contains_key("range") || ctx.headers().contains_key("Range");

                if is_range_request {
                    tracing::debug!(
                        request_id = %ctx.request_id(),
                        "Range request detected - bypassing cache"
                    );
                    if self.audit_writer.is_some() {
                        ctx.audit()
                            .set_cache_status(crate::audit::CacheStatus::Bypass);
                    }
                    // Skip cache lookup - fall through to Ok(false) at the end
                } else {
                    let bucket_config = ctx.bucket_config().cloned().ok_or_else(|| {
                        pingora_core::Error::explain(
                            pingora_core::ErrorType::InternalError,
                            "Missing bucket config in context",
                        )
                    })?;

                    // Construct cache key from bucket and object path
                    // Use router.extract_s3_key for consistent key generation (same as cache set)
                    let object_key = router.extract_s3_key(ctx.path()).unwrap_or_default();

                    let cache_key = CacheKey {
                        bucket: bucket_config.name.clone(),
                        object_key: object_key.clone(),
                        etag: None, // We don't know the etag yet
                    };

                    // Extract conditional headers before mutable borrow of ctx for audit
                    let if_none_match = ctx
                        .headers()
                        .get("If-None-Match")
                        .or_else(|| ctx.headers().get("if-none-match"))
                        .cloned();

                    let if_modified_since = ctx
                        .headers()
                        .get("If-Modified-Since")
                        .or_else(|| ctx.headers().get("if-modified-since"))
                        .cloned();

                    // Try to get from cache (with duration tracking)
                    let cache_start = std::time::Instant::now();
                    let cache_result = cache.get(&cache_key).await;
                    let cache_duration = cache_start.elapsed().as_secs_f64() * 1000.0; // Convert to ms
                    self.metrics.record_cache_get_duration(cache_duration);

                    match cache_result {
                        Ok(Some(cached_entry)) => {
                            if self.audit_writer.is_some() {
                                ctx.audit().set_cache_status(crate::audit::CacheStatus::Hit);
                            }
                            // Cache hit!
                            // Phase 30.7: ETag validation
                            // Check if client sent If-None-Match header for conditional requests
                            if let Some(ref client_etag) = if_none_match {
                                // If ETags match, return 304 Not Modified
                                if client_etag == cached_entry.etag.as_str() {
                                    tracing::debug!(
                                        request_id = %ctx.request_id(),
                                        bucket = %bucket_config.name,
                                        object_key = %object_key,
                                        etag = %cached_entry.etag,
                                        "ETag matches - returning 304 Not Modified"
                                    );

                                    let mut header = ResponseHeader::build(304, None)?;
                                    header.insert_header("ETag", cached_entry.etag.as_str())?;
                                    header.insert_header("X-Cache", "HIT")?;

                                    session
                                        .write_response_header(Box::new(header), true)
                                        .await?;
                                    self.metrics.increment_status_count(304);
                                    self.metrics.increment_cache_hit();
                                    return Ok(true);
                                }
                            }

                            // Phase 36: Check If-Modified-Since header for conditional requests
                            if let Some(ref client_modified_since) = if_modified_since {
                                // If Last-Modified matches, return 304 Not Modified
                                if let Some(ref last_modified) = cached_entry.last_modified {
                                    if client_modified_since == last_modified.as_str() {
                                        tracing::debug!(
                                            request_id = %ctx.request_id(),
                                            bucket = %bucket_config.name,
                                            object_key = %object_key,
                                            last_modified = %last_modified,
                                            "Last-Modified matches If-Modified-Since - returning 304 Not Modified"
                                        );

                                        let mut header = ResponseHeader::build(304, None)?;
                                        header.insert_header(
                                            "Last-Modified",
                                            last_modified.as_str(),
                                        )?;
                                        if !cached_entry.etag.is_empty() {
                                            header.insert_header(
                                                "ETag",
                                                cached_entry.etag.as_str(),
                                            )?;
                                        }
                                        header.insert_header("X-Cache", "HIT")?;

                                        session
                                            .write_response_header(Box::new(header), true)
                                            .await?;
                                        self.metrics.increment_status_count(304);
                                        self.metrics.increment_cache_hit();
                                        return Ok(true);
                                    }
                                }
                            }

                            tracing::debug!(
                                request_id = %ctx.request_id(),
                                bucket = %bucket_config.name,
                                object_key = %object_key,
                                "Cache hit - returning cached response"
                            );

                            // Build response from cached entry
                            let mut header = ResponseHeader::build(200, None)?;
                            header.insert_header(
                                "Content-Type",
                                cached_entry.content_type.as_str(),
                            )?;
                            header.insert_header("ETag", cached_entry.etag.as_str())?;
                            // Add Last-Modified header if available
                            if let Some(ref last_modified) = cached_entry.last_modified {
                                header.insert_header("Last-Modified", last_modified.as_str())?;
                            }
                            header.insert_header(
                                "Content-Length",
                                cached_entry.data.len().to_string(),
                            )?;
                            header.insert_header("X-Cache", "HIT")?; // Indicate cache hit

                            // For HEAD requests: send only headers (no body)
                            // For GET requests: send headers + body

                            // Write response header (end_stream=true for HEAD, false for GET)
                            session
                                .write_response_header(Box::new(header), is_head_request)
                                .await?;

                            // Only send body for GET requests (not HEAD)
                            if !is_head_request {
                                session
                                    .write_response_body(Some(cached_entry.data), true)
                                    .await?;
                            }

                            // Record metrics
                            self.metrics.increment_status_count(200);
                            self.metrics.increment_cache_hit();

                            return Ok(true); // Short-circuit - don't go to upstream
                        }
                        Ok(None) => {
                            if self.audit_writer.is_some() {
                                ctx.audit()
                                    .set_cache_status(crate::audit::CacheStatus::Miss);
                            }
                            // Cache miss - continue to upstream
                            tracing::debug!(
                                request_id = %ctx.request_id(),
                                bucket = %bucket_config.name,
                                object_key = %object_key,
                                "Cache miss - proceeding to S3"
                            );
                            self.metrics.increment_cache_miss();
                            // Fall through to Ok(false) below
                        }
                        Err(e) => {
                            if self.audit_writer.is_some() {
                                ctx.audit()
                                    .set_cache_status(crate::audit::CacheStatus::Miss);
                            }
                            // Cache error - log but continue to upstream (don't fail request)
                            tracing::warn!(
                                request_id = %ctx.request_id(),
                                bucket = %bucket_config.name,
                                object_key = %object_key,
                                error = %e,
                                "Cache lookup error - proceeding to S3"
                            );
                            // Fall through to Ok(false) below
                        }
                    }
                }
            }
        }

        Ok(false) // Continue to upstream
    }

    /// Modify upstream request headers (add AWS Signature v4)
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let router = self.router.load_full();

        let bucket_config = ctx.bucket_config().ok_or_else(|| {
            pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                "No bucket config in context",
            )
        })?;

        // Extract S3 key from path
        let s3_key = router.extract_s3_key(ctx.path()).unwrap_or_default();

        // Phase 23: Use selected replica's config if available
        let (bucket, region, access_key, secret_key, endpoint): (
            String,
            String,
            String,
            String,
            Option<String>,
        ) = if let Some(replica_name) = ctx.replica_name() {
            // Look up replica from ReplicaSet
            if let Some(replica_set) = self.replica_sets.get(&bucket_config.name) {
                if let Some(replica) = replica_set.replicas.iter().find(|r| r.name == replica_name)
                {
                    (
                        replica.client.config.bucket.clone(),
                        replica.client.config.region.clone(),
                        replica.client.config.access_key.clone(),
                        replica.client.config.secret_key.clone(),
                        replica.client.config.endpoint.clone(),
                    )
                } else {
                    // Replica not found, fall back to legacy config
                    (
                        bucket_config.s3.bucket.clone(),
                        bucket_config.s3.region.clone(),
                        bucket_config.s3.access_key.clone(),
                        bucket_config.s3.secret_key.clone(),
                        bucket_config.s3.endpoint.clone(),
                    )
                }
            } else {
                // No ReplicaSet, fall back to legacy config
                (
                    bucket_config.s3.bucket.clone(),
                    bucket_config.s3.region.clone(),
                    bucket_config.s3.access_key.clone(),
                    bucket_config.s3.secret_key.clone(),
                    bucket_config.s3.endpoint.clone(),
                )
            }
        } else {
            // No replica selected, use legacy bucket config
            (
                bucket_config.s3.bucket.clone(),
                bucket_config.s3.region.clone(),
                bucket_config.s3.access_key.clone(),
                bucket_config.s3.secret_key.clone(),
                bucket_config.s3.endpoint.clone(),
            )
        };

        // Determine the correct host for this endpoint (without port for signature)
        let host_for_signing = if let Some(custom_endpoint) = &endpoint {
            // For custom endpoints (MinIO), use the endpoint hostname WITHOUT port
            // (AWS Signature v4 expects Host header without port)
            custom_endpoint
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split(':')
                .next()
                .unwrap_or("localhost")
                .to_string()
        } else {
            // For AWS S3, use the standard format
            format!("{}.s3.{}.amazonaws.com", bucket, region)
        };

        // Build S3 request with correct HTTP method
        let s3_request = match ctx.method() {
            "HEAD" => build_head_object_request(&bucket, &s3_key, &region),
            _ => build_get_object_request(&bucket, &s3_key, &region),
        };

        // Get signed headers with correct host for signature calculation
        let signed_headers = if endpoint.is_some() {
            // For custom endpoints, use the custom host in the signature
            s3_request.get_signed_headers_with_host(&access_key, &secret_key, &host_for_signing)
        } else {
            // For AWS, use the standard signing (AWS-style host)
            s3_request.get_signed_headers(&access_key, &secret_key)
        };

        // Add signed headers to upstream request
        // Use append_header instead of insert_header to avoid lifetime issues
        for (name, value) in signed_headers {
            let header_name =
                http::header::HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
                    pingora_core::Error::explain(
                        pingora_core::ErrorType::InternalError,
                        format!("Invalid header name: {}", e),
                    )
                })?;
            let header_value = http::header::HeaderValue::from_str(&value).map_err(|e| {
                pingora_core::Error::explain(
                    pingora_core::ErrorType::InternalError,
                    format!("Invalid header value: {}", e),
                )
            })?;
            upstream_request
                .append_header(header_name, header_value)
                .map_err(|e| {
                    pingora_core::Error::explain(
                        pingora_core::ErrorType::InternalError,
                        format!("Failed to append header: {}", e),
                    )
                })?;
        }

        // Update Host header to S3 endpoint
        let host = if let Some(custom_endpoint) = &endpoint {
            // For custom endpoints (MinIO), use the endpoint hostname
            custom_endpoint
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split(':')
                .next()
                .unwrap_or("localhost")
                .to_string()
        } else {
            // For AWS S3, use the standard format
            format!("{}.s3.{}.amazonaws.com", bucket, region)
        };

        upstream_request.remove_header(&http::header::HOST);
        upstream_request
            .append_header(
                http::header::HOST,
                http::header::HeaderValue::from_str(&host).map_err(|e| {
                    pingora_core::Error::explain(
                        pingora_core::ErrorType::InternalError,
                        format!("Invalid host header: {}", e),
                    )
                })?,
            )
            .map_err(|e| {
                pingora_core::Error::explain(
                    pingora_core::ErrorType::InternalError,
                    format!("Failed to set Host header: {}", e),
                )
            })?;

        // Update URI to S3 path - for MinIO use /bucket/key format, for AWS use /key
        let uri = if endpoint.is_some() {
            // MinIO path-style: /bucket/key
            format!("/{}/{}", bucket, s3_key)
        } else {
            // AWS virtual-hosted style: /key (bucket is in Host header)
            format!("/{}", s3_key)
        };
        let parsed_uri = uri.parse().map_err(|e: http::uri::InvalidUri| {
            pingora_core::Error::explain(
                pingora_core::ErrorType::InternalError,
                format!("Invalid URI: {}", e),
            )
        })?;
        upstream_request.set_uri(parsed_uri);

        // Record S3 operation metrics
        let method = ctx.method().to_uppercase();
        self.metrics.increment_s3_operation(&method);

        Ok(())
    }

    /// Log request completion for metrics and debugging
    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora_core::Error>,
        ctx: &mut Self::CTX,
    ) {
        // Get status code from response header
        let status_code = if let Some(resp) = session.response_written() {
            resp.status.as_u16()
        } else {
            500 // Default to 500 if no response written
        };

        // Calculate request duration
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64;
        let start = ctx.timestamp() as f64 * 1000.0; // Convert seconds to milliseconds
        let duration_ms = now - start;

        // Record metrics
        self.metrics.increment_status_count(status_code);
        self.metrics.increment_method_count(ctx.method());
        self.metrics.record_duration(duration_ms);

        // Record bucket-specific metrics if bucket was identified
        if let Some(bucket_config) = ctx.bucket_config() {
            self.metrics.increment_bucket_count(&bucket_config.name);
            self.metrics
                .record_bucket_latency(&bucket_config.name, duration_ms);

            // Record circuit breaker success/failure if circuit breaker is configured
            if let Some(circuit_breaker) = self.circuit_breakers.get(&bucket_config.name) {
                // 2xx: Success - record success
                // 5xx: Server error (S3 backend failure) - record failure
                // 4xx/3xx: Client error/redirect - don't affect circuit breaker
                if (200..300).contains(&status_code) {
                    circuit_breaker.record_success();
                    tracing::debug!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_config.name,
                        status_code = status_code,
                        "Circuit breaker recorded success"
                    );
                } else if status_code >= 500 {
                    circuit_breaker.record_failure();
                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_config.name,
                        status_code = status_code,
                        failure_count = circuit_breaker.failure_count(),
                        "Circuit breaker recorded failure"
                    );
                }
            }
        }

        // Decrement active connections (request completed)
        self.metrics.decrement_active_connections();

        // Extract client IP for logging
        let client_ip = self.get_client_ip(session);

        // Extract S3 error information from upstream response headers (if error status)
        let (s3_error_code, s3_error_message) = if status_code >= 400 {
            if let Some(resp) = session.response_written() {
                let error_code = resp
                    .headers
                    .get("x-amz-error-code")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let error_message = resp
                    .headers
                    .get("x-amz-error-message")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                (error_code, error_message)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Log S3 errors with error code and message (if available)
        if status_code >= 400 {
            if let (Some(code), Some(message)) = (&s3_error_code, &s3_error_message) {
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    client_ip = %client_ip,
                    method = %ctx.method(),
                    path = %ctx.path(),
                    status_code = status_code,
                    s3_error_code = %code,
                    s3_error_message = %message,
                    bucket = ctx.bucket_config().map(|b| b.name.as_str()).unwrap_or("unknown"),
                    duration_ms = duration_ms,
                    "S3 error response with error details"
                );
            } else {
                // Error response but no S3 error headers (might be proxy error, not S3)
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    client_ip = %client_ip,
                    method = %ctx.method(),
                    path = %ctx.path(),
                    status_code = status_code,
                    bucket = ctx.bucket_config().map(|b| b.name.as_str()).unwrap_or("unknown"),
                    duration_ms = duration_ms,
                    "Error response without S3 error headers"
                );
            }
        }

        // Log request completion with request ID for tracing
        tracing::info!(
            request_id = %ctx.request_id(),
            client_ip = %client_ip,
            method = %ctx.method(),
            path = %ctx.path(),
            status_code = status_code,
            duration_ms = duration_ms,
            "Request completed"
        );

        // -- Audit Logging: Finalize and write log --
        if let Some(writer) = &self.audit_writer {
            let audit_ctx = ctx.audit();
            audit_ctx.set_response_status(status_code);
            if let Some(resp) = session.response_written() {
                let content_length = resp
                    .headers
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                audit_ctx.set_response_size(content_length);
            }

            let entry = audit_ctx.to_audit_entry();
            if let Err(e) = writer.write_entry(entry) {
                tracing::error!("Failed to write audit entry: {}", e);
            }
        }
        // -- End Audit Logging --
    }

    /// Filter upstream responses to add custom headers (request correlation)
    /// Phase 30: Also captures response headers for cache population
    fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Add X-Request-ID header for request correlation
        upstream_response
            .insert_header("X-Request-ID", ctx.request_id())
            .map_err(|e| {
                tracing::warn!(
                    request_id = %ctx.request_id(),
                    error = ?e,
                    "Failed to add X-Request-ID header"
                );
                e
            })?;

        // Add X-Cache: MISS header for cache miss responses (response from S3)
        // This complements X-Cache: HIT added in request_filter for cache hits
        if self.cache.is_some() {
            upstream_response.insert_header("X-Cache", "MISS").ok();
        }

        // Log successful requests with replica information (Phase 23: HA bucket replication)
        let status = upstream_response.status.as_u16();
        if (200..300).contains(&status) {
            // Only log if we have replica information
            if let (Some(replica_name), Some(bucket_config)) =
                (ctx.replica_name(), ctx.bucket_config())
            {
                // Calculate request duration from start timestamp
                let duration_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .saturating_sub(ctx.timestamp()) as f64
                    * 1000.0; // Convert seconds to milliseconds

                tracing::info!(
                    request_id = %ctx.request_id(),
                    bucket = bucket_config.name.as_str(),
                    replica = replica_name,
                    status = status,
                    duration_ms = duration_ms,
                    "Request served from replica '{}'", replica_name
                );
            }
        }

        // Phase 30: Enable response buffering for cache population
        // Only buffer successful (200) responses that are not range requests
        if status == 200 && self.cache.is_some() {
            // Capture response headers for cache entry
            if let Some(content_type) = upstream_response
                .headers
                .get("content-type")
                .or_else(|| upstream_response.headers.get("Content-Type"))
            {
                if let Ok(ct_str) = content_type.to_str() {
                    ctx.set_response_content_type(ct_str.to_string());
                }
            }

            if let Some(etag) = upstream_response
                .headers
                .get("etag")
                .or_else(|| upstream_response.headers.get("ETag"))
            {
                if let Ok(etag_str) = etag.to_str() {
                    // Remove quotes from ETag if present
                    let etag_clean = etag_str.trim_matches('"').to_string();
                    ctx.set_response_etag(etag_clean);
                }
            }

            // Capture Last-Modified header for If-Modified-Since support
            if let Some(last_modified) = upstream_response
                .headers
                .get("last-modified")
                .or_else(|| upstream_response.headers.get("Last-Modified"))
            {
                if let Ok(lm_str) = last_modified.to_str() {
                    ctx.set_response_last_modified(lm_str.to_string());
                }
            }

            // Enable response buffering (will be used in response_body_filter)
            ctx.enable_response_buffering();

            tracing::debug!(
                request_id = %ctx.request_id(),
                "Enabled response buffering for cache population"
            );
        }

        Ok(())
    }

    /// Filter response body chunks for cache population (Phase 30)
    /// Buffers response data while streaming to client
    fn response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<bytes::Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<Option<std::time::Duration>>
    where
        Self::CTX: Send + Sync,
    {
        // If buffering is enabled, accumulate chunks
        if ctx.is_response_buffering_enabled() {
            // Buffer the current chunk (if any)
            if let Some(chunk) = body.as_ref() {
                // Check if we'd exceed max cacheable size (10MB)
                const MAX_CACHE_SIZE: usize = 10 * 1024 * 1024; // 10MB
                if ctx.total_response_size() + chunk.len() <= MAX_CACHE_SIZE {
                    ctx.append_response_chunk(chunk);
                } else {
                    // Response too large, disable buffering
                    tracing::debug!(
                        request_id = %ctx.request_id(),
                        total_size = ctx.total_response_size() + chunk.len(),
                        "Response too large for cache, disabling buffering"
                    );
                    ctx.disable_response_buffering();
                }
            }

            // On end of stream, write buffered data to cache
            if end_of_stream && ctx.should_cache_response() {
                if let Some(buffered_data) = ctx.take_response_buffer() {
                    // We need to populate the cache asynchronously
                    // Get necessary data from context before spawning task
                    if let (Some(bucket_config), Some(cache)) = (ctx.bucket_config(), &self.cache) {
                        use crate::cache::{CacheEntry, CacheKey};

                        let router = self.router.load_full();
                        let object_key = router.extract_s3_key(ctx.path()).unwrap_or_default();
                        let cache_key = CacheKey {
                            bucket: bucket_config.name.clone(),
                            object_key: object_key.to_string(),
                            // Use None to match lookup key - ETag is stored in CacheEntry, not key
                            etag: None,
                        };

                        let cache_entry = CacheEntry::new(
                            bytes::Bytes::from(buffered_data),
                            ctx.response_content_type()
                                .unwrap_or("application/octet-stream")
                                .to_string(),
                            ctx.response_etag().unwrap_or("").to_string(),
                            ctx.response_last_modified().map(|s| s.to_string()),
                            Some(std::time::Duration::from_secs(3600)), // 1 hour TTL
                        );

                        // Clone cache for async task
                        let cache_clone = Arc::clone(cache);
                        let request_id = ctx.request_id().to_string();

                        // Spawn async task to populate cache (don't block response)
                        tokio::spawn(async move {
                            match cache_clone.set(cache_key.clone(), cache_entry).await {
                                Ok(_) => {
                                    tracing::info!(
                                        request_id = %request_id,
                                        bucket = %cache_key.bucket,
                                        object_key = %cache_key.object_key,
                                        "Successfully populated cache from S3 response"
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        request_id = %request_id,
                                        bucket = %cache_key.bucket,
                                        object_key = %cache_key.object_key,
                                        error = %e,
                                        "Failed to populate cache from S3 response"
                                    );
                                }
                            }
                        });
                    }
                }
            }
        }

        // Don't modify the body - let it pass through to client unchanged
        Ok(None)
    }

    /// Handle connection failures with automatic retry for transient errors
    ///
    /// Called when connection to upstream fails. Checks if the bucket has retry
    /// enabled and if we haven't exceeded max attempts, then marks the error as
    /// retriable so Pingora will automatically retry the connection.
    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut e: Box<pingora_core::Error>,
    ) -> Box<pingora_core::Error> {
        // Get bucket name from context to look up retry policy
        // Clone to owned String to avoid borrow conflicts
        let bucket_name = ctx
            .bucket_config()
            .map(|bc| bc.name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Check if bucket has retry policy configured
        if let Some(retry_policy) = self.retry_policies.get(&bucket_name) {
            let current_attempt = ctx.retry_attempt();

            // Check if we should retry (haven't exceeded max attempts)
            // Use 502 as status code for connection failures
            if retry_policy.should_retry(current_attempt, 502) {
                // Mark this error as retriable so Pingora will retry
                e.set_retry(true);

                // Increment attempt counter for next potential retry
                let next_attempt = ctx.increment_retry_attempt();

                // Track retry metric
                self.metrics.increment_s3_retry_attempt(&bucket_name);

                tracing::warn!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_name,
                    attempt = current_attempt,
                    next_attempt = next_attempt,
                    max_attempts = retry_policy.max_attempts,
                    error = %e,
                    "Connection failed, scheduling retry"
                );
            } else {
                // Exhausted retries
                self.metrics.increment_s3_retry_exhausted(&bucket_name);

                tracing::error!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_name,
                    attempt = current_attempt,
                    max_attempts = retry_policy.max_attempts,
                    error = %e,
                    "Connection failed, retry attempts exhausted"
                );
            }
        }

        e
    }

    /// Handle errors during proxying with automatic retry for transient errors
    ///
    /// Called when an error occurs after the connection is established (e.g., during
    /// data transfer). Checks if the error is retriable and if we haven't exceeded
    /// max attempts.
    fn error_while_proxy(
        &self,
        peer: &HttpPeer,
        session: &mut Session,
        mut e: Box<pingora_core::Error>,
        ctx: &mut Self::CTX,
        client_reused: bool,
    ) -> Box<pingora_core::Error> {
        // Add peer context to error
        e = e.more_context(format!("Peer: {}", peer));

        // Get bucket name from context to look up retry policy
        // Clone to owned String to avoid borrow conflicts
        let bucket_name = ctx
            .bucket_config()
            .map(|bc| bc.name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Check if bucket has retry policy configured
        if let Some(retry_policy) = self.retry_policies.get(&bucket_name) {
            let current_attempt = ctx.retry_attempt();

            // Check if we should retry (haven't exceeded max attempts)
            // Use 502 as status code for proxy errors
            if retry_policy.should_retry(current_attempt, 502) {
                // Only retry if client connection can be reused and buffer isn't truncated
                let can_retry = client_reused && !session.as_ref().retry_buffer_truncated();

                if can_retry {
                    // Mark this error as retriable
                    e.retry.decide_reuse(true);

                    // Increment attempt counter for next potential retry
                    let next_attempt = ctx.increment_retry_attempt();

                    // Track retry metric
                    self.metrics.increment_s3_retry_attempt(&bucket_name);

                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_name,
                        attempt = current_attempt,
                        next_attempt = next_attempt,
                        max_attempts = retry_policy.max_attempts,
                        error = %e,
                        "Proxy error occurred, scheduling retry"
                    );
                } else {
                    tracing::warn!(
                        request_id = %ctx.request_id(),
                        bucket = %bucket_name,
                        attempt = current_attempt,
                        client_reused = client_reused,
                        buffer_truncated = session.as_ref().retry_buffer_truncated(),
                        error = %e,
                        "Proxy error occurred, cannot retry (client connection not reusable)"
                    );
                }
            } else {
                // Exhausted retries
                self.metrics.increment_s3_retry_exhausted(&bucket_name);

                tracing::error!(
                    request_id = %ctx.request_id(),
                    bucket = %bucket_name,
                    attempt = current_attempt,
                    max_attempts = retry_policy.max_attempts,
                    error = %e,
                    "Proxy error occurred, retry attempts exhausted"
                );
            }
        } else {
            // No retry policy, use default Pingora behavior
            e.retry
                .decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());
        }

        e
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::create_test_subscriber;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_log_successful_request_with_replica_name() {
        // Test: Phase 23 - Log successful request with replica name
        // Expected log format:
        // INFO  Request served from replica 'primary'
        //       request_id=550e8400-..., bucket=products, replica=primary, duration_ms=45

        // Create a buffer to capture log output
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = create_test_subscriber(buffer.clone());

        tracing::subscriber::with_default(subscriber, || {
            // Create a request context with replica name
            let mut ctx = RequestContext::new("GET".to_string(), "/products/test.jpg".to_string());
            ctx.set_replica_name("primary".to_string());

            // Simulate logging a successful request (2xx status)
            // This will be done in upstream_response_filter, but we test it here
            let bucket_name = "products";
            let replica_name = ctx.replica_name().unwrap();
            let request_id = ctx.request_id();
            let duration_ms = 45;

            tracing::info!(
                request_id = %request_id,
                bucket = bucket_name,
                replica = replica_name,
                duration_ms = duration_ms,
                "Request served from replica '{}'", replica_name
            );
        });

        // Read log output
        let output = buffer.lock().unwrap();
        let log_line = String::from_utf8_lossy(&output);

        // Verify log contains required fields
        assert!(
            log_line.contains("Request served from replica 'primary'"),
            "Log should contain message with replica name: {}",
            log_line
        );
        assert!(
            log_line.contains("\"bucket\":\"products\""),
            "Log should contain bucket field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"replica\":\"primary\""),
            "Log should contain replica field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"duration_ms\":45"),
            "Log should contain duration_ms field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"request_id\""),
            "Log should contain request_id field: {}",
            log_line
        );
    }

    #[test]
    fn test_log_failover_event_with_from_to_replicas() {
        // Test: Phase 23 - Log failover event with from/to replica names
        // Expected log format:
        // WARN  Failover: primary → replica-eu
        //       request_id=550e8400-..., bucket=products, reason=ConnectionTimeout, attempt=2

        // Create a buffer to capture log output
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = create_test_subscriber(buffer.clone());

        tracing::subscriber::with_default(subscriber, || {
            // Create a request context
            let ctx = RequestContext::new("GET".to_string(), "/products/test.jpg".to_string());
            let request_id = ctx.request_id();
            let bucket_name = "products";

            // Simulate a failover event: primary → replica-eu
            let from_replica = "primary";
            let to_replica = "replica-eu";
            let reason = "ConnectionTimeout";
            let attempt = 2;

            tracing::warn!(
                request_id = %request_id,
                bucket = bucket_name,
                from = from_replica,
                to = to_replica,
                reason = reason,
                attempt = attempt,
                "Failover: {} → {}", from_replica, to_replica
            );
        });

        // Read log output
        let output = buffer.lock().unwrap();
        let log_line = String::from_utf8_lossy(&output);

        // Verify log contains required fields
        assert!(
            log_line.contains("Failover: primary → replica-eu"),
            "Log should contain failover message: {}",
            log_line
        );
        assert!(
            log_line.contains("\"bucket\":\"products\""),
            "Log should contain bucket field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"from\":\"primary\""),
            "Log should contain from field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"to\":\"replica-eu\""),
            "Log should contain to field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"reason\":\"ConnectionTimeout\""),
            "Log should contain reason field: {}",
            log_line
        );
        assert!(
            log_line.contains("\"attempt\":2"),
            "Log should contain attempt number: {}",
            log_line
        );
        assert!(
            log_line.contains("\"request_id\""),
            "Log should contain request_id field: {}",
            log_line
        );
    }
}
