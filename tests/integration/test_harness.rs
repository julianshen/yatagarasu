// Test harness for integration tests
// Provides utilities to start/stop the proxy server for testing

use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

/// Test proxy instance that automatically starts and stops
pub struct ProxyTestHarness {
    process: Option<Child>,
    pub port: u16,
    pub base_url: String,
}

impl ProxyTestHarness {
    /// Start a new proxy instance with the given config file
    pub fn start(config_path: &str, port: u16) -> Result<Self, String> {
        // Build path to the binary
        let binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("yatagarasu");

        if !binary_path.exists() {
            return Err(format!(
                "Binary not found at {:?}. Run 'cargo build --release' first.",
                binary_path
            ));
        }

        // Start the proxy process
        let mut child = Command::new(&binary_path)
            .arg("--config")
            .arg(config_path)
            .spawn()
            .map_err(|e| format!("Failed to start proxy: {}", e))?;

        // Wait for the proxy to start
        thread::sleep(Duration::from_secs(2));

        // Check if process is still running
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!("Proxy exited immediately with status: {}", status));
            }
            Ok(None) => {
                // Still running, good
            }
            Err(e) => {
                return Err(format!("Error checking proxy status: {}", e));
            }
        }

        // Try to connect to verify it's up
        let base_url = format!("http://127.0.0.1:{}", port);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Try a few times to connect
        for attempt in 1..=5 {
            if let Ok(response) = client.get(&format!("{}/health", base_url)).send() {
                if response.status().is_success()
                    || response.status() == reqwest::StatusCode::NOT_FOUND
                {
                    // Proxy is responding (404 is fine, means routing works)
                    log::info!("Proxy started successfully on port {}", port);
                    return Ok(ProxyTestHarness {
                        process: Some(child),
                        port,
                        base_url,
                    });
                }
            }

            if attempt < 5 {
                log::debug!("Attempt {} to connect to proxy failed, retrying...", attempt);
                thread::sleep(Duration::from_millis(500));
            }
        }

        // If we get here, proxy didn't start properly
        let _ = child.kill();
        Err(format!("Proxy did not respond after 5 attempts on port {}", port))
    }

    /// Get the base URL for making requests
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Stop the proxy (called automatically on drop)
    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            log::info!("Stopping proxy on port {}", self.port);
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ProxyTestHarness {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires binary to be built
    fn test_proxy_harness_starts_and_stops() {
        // This test verifies the harness itself works
        // Requires a valid config file at /tmp/yatagarasu-test/config.yaml

        let harness = ProxyTestHarness::start("/tmp/yatagarasu-test/config.yaml", 18080);
        assert!(harness.is_ok(), "Proxy should start successfully");

        let harness = harness.unwrap();
        assert_eq!(harness.port, 18080);
        assert_eq!(harness.base_url, "http://127.0.0.1:18080");

        // Harness will be dropped here, stopping the proxy
    }
}
