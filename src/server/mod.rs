// Server module - Pingora HTTP server setup and configuration

use crate::config::Config;
use crate::constants::*;
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
            threads: DEFAULT_THREADS,
        }
    }

    /// Create ServerConfig from application Config
    pub fn from_config(config: &Config) -> Self {
        // Combine address and port into a single socket address
        let address = format!("{}:{}", config.server.address, config.server.port);

        Self {
            address,
            threads: config.server.threads,
        }
    }
}

impl YatagarasuServer {
    /// Create a new YatagarasuServer instance
    pub fn new(config: ServerConfig) -> Result<Self, String> {
        // Validate that the address can be parsed
        config
            .address
            .parse::<std::net::SocketAddr>()
            .map_err(|e| format!("Invalid address '{}': {}", config.address, e))?;

        Ok(Self { config })
    }

    /// Get the server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Create Pingora server options
    fn create_server_opt(&self) -> ServerOpt {
        ServerOpt {
            upgrade: false,   // Disable graceful upgrade for now
            daemon: false,    // Don't daemonize
            nocapture: false, // Don't capture stdout/stderr
            test: false,      // Not in test mode
            conf: None,       // No config file for now
        }
    }

    /// Parse the configured address into a SocketAddr
    pub fn parse_address(&self) -> Result<std::net::SocketAddr, String> {
        self.config
            .address
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

impl Default for HttpService {
    fn default() -> Self {
        Self::new()
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
        // Validate request parameters (return 400 for malformed requests)
        if method.is_empty() {
            let mut response = HttpResponse::new(400);
            response.add_header("Content-Type", "application/json");
            let error_body =
                r#"{"error":"Bad Request","message":"Method cannot be empty","status":400}"#;
            response.set_body(error_body.as_bytes().to_vec());
            return Ok(response);
        }

        if path.is_empty() {
            let mut response = HttpResponse::new(400);
            response.add_header("Content-Type", "application/json");
            let error_body =
                r#"{"error":"Bad Request","message":"Path cannot be empty","status":400}"#;
            response.set_body(error_body.as_bytes().to_vec());
            return Ok(response);
        }

        if !path.starts_with('/') {
            let mut response = HttpResponse::new(400);
            response.add_header("Content-Type", "application/json");
            let error_body =
                r#"{"error":"Bad Request","message":"Path must start with /","status":400}"#;
            response.set_body(error_body.as_bytes().to_vec());
            return Ok(response);
        }

        if path.len() > 8192 {
            let mut response = HttpResponse::new(400);
            response.add_header("Content-Type", "application/json");
            let error_body = r#"{"error":"Bad Request","message":"Path too long (max 8192 bytes)","status":400}"#;
            response.set_body(error_body.as_bytes().to_vec());
            return Ok(response);
        }

        // Check if method is supported
        if !self.supports_method(method) {
            let mut response = HttpResponse::new(405);
            response.add_header("Content-Type", "application/json");
            let error_body = r#"{"error":"Method Not Allowed","message":"HTTP method not supported","status":405}"#;
            response.set_body(error_body.as_bytes().to_vec());
            return Ok(response);
        }

        // Route based on path
        match path {
            "/health" => {
                // Health check endpoint
                let mut response = HttpResponse::new(200);
                response.add_header("Content-Type", "application/json");

                // For HEAD requests, only return headers (no body)
                if method != "HEAD" {
                    // Create JSON response body with config status
                    let body = r#"{"status":"ok","config_loaded":true}"#;
                    response.set_body(body.as_bytes().to_vec());
                }

                Ok(response)
            }
            _ => {
                // Unknown path
                let mut response = HttpResponse::new(404);
                response.add_header("Content-Type", "application/json");
                let error_body = r#"{"error":"Not Found","message":"The requested resource was not found","status":404}"#;
                response.set_body(error_body.as_bytes().to_vec());
                Ok(response)
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

    #[test]
    fn test_server_config_from_config_uses_threads() {
        // Create a Config with custom threads value
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
  threads: 16
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // ServerConfig::from_config should use the threads value from config
        let server_config = ServerConfig::from_config(&config);

        assert_eq!(server_config.address, "127.0.0.1:8080");
        assert_eq!(server_config.threads, 16);
    }

    #[test]
    fn test_server_config_from_config_default_threads() {
        // Create a Config without explicit threads value
        let yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets: []
"#;
        let config = Config::from_yaml_with_env(yaml).unwrap();

        // ServerConfig::from_config should use default threads
        let server_config = ServerConfig::from_config(&config);

        assert_eq!(server_config.threads, DEFAULT_THREADS);
    }
}
