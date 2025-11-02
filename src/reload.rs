// Configuration hot reload module
// Handles SIGHUP signal to reload configuration without downtime

use crate::config::Config;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// ReloadManager handles configuration reload via SIGHUP signal
pub struct ReloadManager {
    config_path: PathBuf,
    reload_requested: Arc<AtomicBool>,
}

impl ReloadManager {
    /// Create a new ReloadManager with the config file path
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            reload_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register SIGHUP signal handler
    /// Returns a handle that can be used to check if reload was requested
    #[cfg(unix)]
    pub fn register_signal_handler(&self) -> Result<(), String> {
        use signal_hook::consts::SIGHUP;
        use signal_hook::flag;

        // Register SIGHUP handler that sets the reload_requested flag
        flag::register(SIGHUP, Arc::clone(&self.reload_requested))
            .map_err(|e| format!("Failed to register SIGHUP handler: {}", e))?;

        Ok(())
    }

    /// Check if reload was requested via SIGHUP
    pub fn is_reload_requested(&self) -> bool {
        self.reload_requested.load(Ordering::Relaxed)
    }

    /// Clear the reload request flag
    pub fn clear_reload_request(&self) {
        self.reload_requested.store(false, Ordering::Relaxed);
    }

    /// Attempt to reload configuration from file
    /// Returns Ok(new_config) if reload successful, Err if validation fails
    pub fn reload_config(&self) -> Result<Config, String> {
        // Load new config from file
        let new_config = Config::from_file(&self.config_path)?;

        // Validate before applying
        new_config.validate()?;

        Ok(new_config)
    }

    /// Get the config file path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_reload_manager_can_be_created() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let manager = ReloadManager::new(config_path.clone());
        assert_eq!(manager.config_path(), &config_path);
    }

    #[test]
    #[cfg(unix)]
    fn test_can_register_sighup_handler() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let manager = ReloadManager::new(config_path);
        let result = manager.register_signal_handler();

        assert!(result.is_ok(), "Should be able to register SIGHUP handler");
    }

    #[test]
    fn test_reload_requested_flag_starts_false() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let manager = ReloadManager::new(config_path);
        assert!(!manager.is_reload_requested());
    }

    #[test]
    fn test_can_clear_reload_request() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let manager = ReloadManager::new(config_path);

        // Manually set the flag (simulating SIGHUP)
        manager.reload_requested.store(true, Ordering::Relaxed);
        assert!(manager.is_reload_requested());

        // Clear the flag
        manager.clear_reload_request();
        assert!(!manager.is_reload_requested());
    }

    #[test]
    fn test_reload_config_validates_before_applying() {
        // Create temp file with valid config
        let mut temp_file = NamedTempFile::new().unwrap();
        let valid_config = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "test-bucket"
    path_prefix: "/test"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "test-key"
      secret_key: "test-secret"
"#;
        temp_file.write_all(valid_config.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path);

        // Reload should succeed
        let result = manager.reload_config();
        assert!(result.is_ok());

        let new_config = result.unwrap();
        assert_eq!(new_config.buckets.len(), 1);
        assert_eq!(new_config.buckets[0].name, "test-bucket");
    }

    #[test]
    fn test_reload_config_rejects_invalid_config() {
        // Create temp file with invalid config (duplicate path_prefix)
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_config = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "my-bucket-1"
      region: "us-east-1"
      access_key: "test-key-1"
      secret_key: "test-secret-1"
  - name: "bucket2"
    path_prefix: "/api"
    s3:
      bucket: "my-bucket-2"
      region: "us-east-1"
      access_key: "test-key-2"
      secret_key: "test-secret-2"
"#;
        temp_file.write_all(invalid_config.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path);

        // Reload should fail validation
        let result = manager.reload_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate path_prefix"));
    }
}
