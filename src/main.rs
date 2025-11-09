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

    // Log startup banner
    let version = env!("CARGO_PKG_VERSION");
    tracing::info!(
        version = version,
        "Starting Yatagarasu S3 Proxy"
    );

    // Load and validate configuration from file
    let config_path_display = args.config.display().to_string();

    // Check if config file exists
    if !args.config.exists() {
        eprintln!("Error: Configuration file not found: {}", config_path_display);
        eprintln!("Please ensure the config file exists or specify a different path with --config");
        std::process::exit(1);
    }

    tracing::info!(
        config_file = %config_path_display,
        "Loading configuration"
    );

    let config = Config::from_file(&args.config).unwrap_or_else(|e| {
        eprintln!("Error: Failed to load configuration from {}", config_path_display);
        eprintln!("Reason: {}", e);
        eprintln!("\nPlease check:");
        eprintln!("  - YAML syntax is correct");
        eprintln!("  - All required fields are present");
        eprintln!("  - Environment variables are set (if using ${{VAR}} syntax)");
        std::process::exit(1);
    });

    tracing::info!(
        config_file = %config_path_display,
        server_address = %config.server.address,
        server_port = config.server.port,
        buckets = config.buckets.len(),
        jwt_enabled = config.jwt.is_some(),
        "Configuration loaded and validated successfully"
    );

    // Build Pingora server options
    let opt = Opt {
        daemon: args.daemon,
        test: args.test,
        upgrade: args.upgrade,
        ..Default::default()
    };

    // Test mode: validate config and exit
    if args.test {
        tracing::info!("Configuration test mode: validation successful");
        println!("Configuration is valid:");
        println!("  Version: {}", version);
        println!("  Config file: {}", config_path_display);
        println!("  Listen address: {}:{}", config.server.address, config.server.port);
        println!("  Buckets configured: {}", config.buckets.len());
        println!("  JWT enabled: {}", config.jwt.is_some());
        std::process::exit(0);
    }

    // Create Pingora server
    let mut server = Server::new(Some(opt)).unwrap_or_else(|e| {
        eprintln!("Error: Failed to create Pingora server");
        eprintln!("Reason: {}", e);
        std::process::exit(1);
    });
    server.bootstrap();

    // Create YatagarasuProxy instance with reload support
    let proxy = YatagarasuProxy::with_reload(config.clone(), args.config.clone());

    // Create HTTP proxy service
    let mut proxy_service = pingora_proxy::http_proxy_service(&server.configuration, proxy);

    // Add TCP listener for HTTP
    let listen_addr = format!("{}:{}", config.server.address, config.server.port);

    tracing::info!(
        version = version,
        address = %listen_addr,
        config_file = %config_path_display,
        buckets = config.buckets.len(),
        "Yatagarasu S3 Proxy starting"
    );

    proxy_service.add_tcp(&listen_addr);

    // Register service with server
    server.add_service(proxy_service);

    tracing::info!(
        address = %listen_addr,
        "Listening for connections"
    );

    // Run server forever (blocks until shutdown)
    server.run_forever();
}
