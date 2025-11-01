use clap::Parser;
use pingora_core::server::configuration::Opt;
use pingora_core::server::Server;
use std::path::PathBuf;
use yatagarasu::config::Config;
use yatagarasu::proxy::YatagarasuProxy;

/// Yatagarasu S3 Proxy - High-performance S3 proxy built with Cloudflare's Pingora
#[derive(Parser, Debug)]
#[command(name = "yatagarasu")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Daemon mode
    #[arg(short = 'd', long)]
    daemon: bool,

    /// Test configuration and exit
    #[arg(long)]
    test: bool,

    /// Upgrade workers gracefully
    #[arg(long)]
    upgrade: bool,
}

fn main() {
    // Initialize logging subsystem
    yatagarasu::logging::init_subscriber().expect("Failed to initialize logging subsystem");

    // Parse command-line arguments
    let args = Args::parse();

    // Load Yatagarasu configuration from file
    let config = Config::from_file(&args.config).unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {}", e);
        std::process::exit(1);
    });

    tracing::info!(
        config_file = %args.config.display(),
        server_address = %config.server.address,
        server_port = config.server.port,
        buckets = config.buckets.len(),
        jwt_enabled = config.jwt.is_some(),
        "Configuration loaded successfully"
    );

    // Build Pingora server options
    let opt = Opt {
        daemon: args.daemon,
        test: args.test,
        upgrade: args.upgrade,
        ..Default::default()
    };

    // Create Pingora server
    let mut server = Server::new(Some(opt)).expect("Failed to create Pingora server");
    server.bootstrap();

    // Create YatagarasuProxy instance
    let proxy = YatagarasuProxy::new(config.clone());

    // Create HTTP proxy service
    let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

    // Add TCP listener for HTTP
    let listen_addr = format!("{}:{}", config.server.address, config.server.port);
    proxy_service.add_tcp(&listen_addr);

    tracing::info!(
        address = %listen_addr,
        "Starting Yatagarasu S3 Proxy"
    );

    // Register service with server
    server.add_service(proxy_service);

    // Run server forever (blocks until shutdown)
    server.run_forever();
}
