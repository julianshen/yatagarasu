// Server module - Pingora HTTP server setup and configuration

use crate::config::Config;

/// Configuration for the HTTP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "0.0.0.0:8080")
    pub address: String,
    /// Number of worker threads
    pub threads: usize,
}

impl ServerConfig {
    /// Create a new ServerConfig with default values
    pub fn new(address: String) -> Self {
        Self {
            address,
            threads: 4, // Default to 4 threads
        }
    }

    /// Create ServerConfig from application Config
    pub fn from_config(config: &Config) -> Self {
        // Combine address and port into a single socket address
        let address = format!("{}:{}", config.server.address, config.server.port);

        Self {
            address,
            threads: 4, // TODO: Add threads configuration to Config
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_new() {
        let config = ServerConfig::new("127.0.0.1:8080".to_string());
        assert_eq!(config.address, "127.0.0.1:8080");
        assert_eq!(config.threads, 4);
    }

    #[test]
    fn test_server_config_default_threads() {
        let config = ServerConfig {
            address: "0.0.0.0:8080".to_string(),
            threads: 8,
        };
        assert_eq!(config.threads, 8);
    }
}
