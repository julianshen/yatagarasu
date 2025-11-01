use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use yatagarasu::config::Config;

/// Yatagarasu S3 Proxy - High-performance S3 proxy built with Cloudflare's Pingora
#[derive(Parser, Debug)]
#[command(name = "yatagarasu")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,
}

fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logging subsystem
    yatagarasu::logging::init_subscriber()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    // Load configuration from file
    let config = Config::from_file(&args.config)
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    tracing::info!(
        config_file = %args.config.display(),
        server_address = %config.server.address,
        server_port = config.server.port,
        "Configuration loaded successfully"
    );

    // Create server instance
    let server_config = yatagarasu::server::ServerConfig::from_config(&config);
    let server = yatagarasu::server::YatagarasuServer::new(server_config)
        .map_err(|e| anyhow::anyhow!("Failed to create server: {}", e))?;

    // Log server startup
    tracing::info!(
        address = %server.config().address,
        threads = server.config().threads,
        "Starting Yatagarasu S3 Proxy"
    );

    // In a real implementation, this would start the Pingora server
    // For now, we verify the server can be created and configured correctly
    tracing::info!("Server initialized successfully");

    Ok(())
}
