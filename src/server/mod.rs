// Server module - Pingora HTTP server setup and configuration

use crate::config::Config;
use pingora::server::configuration::Opt as ServerOpt;
use pingora::server::Server;

/// Configuration for the HTTP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g., "0.0.0.0:8080")
    pub address: String,
    /// Number of worker threads
    pub threads: usize,
}

/// HTTP service that handles requests
#[derive(Debug)]
pub struct HttpService {
    supported_methods: Vec<String>,
}

/// HTTP response
#[derive(Debug)]
pub struct HttpResponse {
    status_code: u16,
    headers: std::collections::HashMap<String, String>,
    body: Vec<u8>,
}

/// Yatagarasu HTTP Server wrapper around Pingora
#[derive(Debug)]
pub struct YatagarasuServer {
    config: ServerConfig,
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

impl YatagarasuServer {
    /// Create a new YatagarasuServer instance
    pub fn new(config: ServerConfig) -> Result<Self, String> {
        // Validate that the address can be parsed
        config.address.parse::<std::net::SocketAddr>()
            .map_err(|e| format!("Invalid address '{}': {}", config.address, e))?;

        Ok(Self {
            config,
        })
    }

    /// Get the server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Create Pingora server options
    fn create_server_opt(&self) -> ServerOpt {
        let mut server_opt = ServerOpt::default();
        server_opt.upgrade = false; // Disable graceful upgrade for now
        server_opt.daemon = false; // Don't daemonize
        server_opt.nocapture = false; // Don't capture stdout/stderr
        server_opt.test = false; // Not in test mode
        server_opt.conf = None; // No config file for now
        server_opt
    }

    /// Parse the configured address into a SocketAddr
    pub fn parse_address(&self) -> Result<std::net::SocketAddr, String> {
        self.config.address
            .parse()
            .map_err(|e| format!("Failed to parse address '{}': {}", self.config.address, e))
    }

    /// Build a Pingora Server instance
    pub fn build_pingora_server(&self) -> Result<Server, String> {
        // Create a new Pingora server with the configured options
        let server_opt = self.create_server_opt();
        let mut server = Server::new(Some(server_opt))
            .map_err(|e| format!("Failed to create Pingora server: {}", e))?;

        // Bootstrap the server with default configuration
        server.bootstrap();

        Ok(server)
    }

    /// Create an HTTP service that handles requests
    pub fn create_http_service(&self) -> Result<HttpService, String> {
        Ok(HttpService::new())
    }
}

impl HttpService {
    /// Create a new HttpService with default supported methods
    pub fn new() -> Self {
        Self {
            supported_methods: vec!["GET".to_string(), "HEAD".to_string(), "POST".to_string()],
        }
    }

    /// Check if a method is supported
    pub fn supports_method(&self, method: &str) -> bool {
        self.supported_methods.iter().any(|m| m == method)
    }

    /// Create an HTTP response with the given status code
    pub fn create_response(&self, status_code: u16) -> Result<HttpResponse, String> {
        Ok(HttpResponse::new(status_code))
    }

    /// Handle an HTTP request and return a response
    pub fn handle_request(&self, method: &str, path: &str) -> Result<HttpResponse, String> {
        // Check if method is supported
        if !self.supports_method(method) {
            return Ok(HttpResponse::new(405)); // Method Not Allowed
        }

        // Route based on path
        match path {
            "/health" => {
                // Health check endpoint
                let mut response = HttpResponse::new(200);
                response.add_header("Content-Type", "application/json");

                // Create JSON response body with config status
                let body = r#"{"status":"ok","config_loaded":true}"#;
                response.set_body(body.as_bytes().to_vec());

                Ok(response)
            }
            _ => {
                // Unknown path
                Ok(HttpResponse::new(404))
            }
        }
    }
}

impl HttpResponse {
    /// Create a new HTTP response with the given status code
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: std::collections::HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Get the status code of the response
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Add a header to the response
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }

    /// Get a header value by name
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Get all headers
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }

    /// Set the response body
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Get the response body
    pub fn body(&self) -> &[u8] {
        &self.body
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
