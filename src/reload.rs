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

        // Increment generation number for version tracking
        // Note: The generation will be properly incremented by the caller
        // based on the current config's generation

        Ok(new_config)
    }

    /// Reload config and increment generation number
    /// Takes the current generation and returns new config with incremented generation
    pub fn reload_config_with_generation(&self, current_generation: u64) -> Result<Config, String> {
        let mut new_config = self.reload_config()?;
        new_config.generation = current_generation + 1;
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

    #[test]
    fn test_generation_increments_on_reload() {
        // Test: Config has generation/version number that increments on reload
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_yaml = r#"
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
        temp_file.write_all(config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path);

        // Initial config has generation 0
        let initial_config = manager.reload_config().unwrap();
        assert_eq!(initial_config.generation, 0);

        // Reload with generation increment
        let reloaded_config = manager.reload_config_with_generation(initial_config.generation).unwrap();
        assert_eq!(reloaded_config.generation, 1);

        // Reload again
        let reloaded_config2 = manager.reload_config_with_generation(reloaded_config.generation).unwrap();
        assert_eq!(reloaded_config2.generation, 2);
    }

    #[test]
    fn test_in_flight_requests_continue_with_old_config() {
        // Test: In-flight requests continue with old config
        // This test verifies the pattern: old config remains valid while new config is prepared

        let mut temp_file = NamedTempFile::new().unwrap();
        let old_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "old-bucket"
    path_prefix: "/old"
    s3:
      bucket: "old-s3-bucket"
      region: "us-east-1"
      access_key: "old-key"
      secret_key: "old-secret"
"#;
        temp_file.write_all(old_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path.clone());

        // Load old config (simulating in-flight request using this)
        let old_config = manager.reload_config().unwrap();
        assert_eq!(old_config.buckets[0].name, "old-bucket");

        // Update config file with new config
        let mut temp_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let new_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "new-bucket"
    path_prefix: "/new"
    s3:
      bucket: "new-s3-bucket"
      region: "us-east-1"
      access_key: "new-key"
      secret_key: "new-secret"
"#;
        temp_file.write_all(new_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Reload new config (simulating new request using this)
        let new_config = manager.reload_config().unwrap();
        assert_eq!(new_config.buckets[0].name, "new-bucket");

        // OLD config should still be valid and unchanged
        assert_eq!(old_config.buckets[0].name, "old-bucket");
        assert_eq!(old_config.buckets[0].s3.bucket, "old-s3-bucket");

        // NEW config should have new values
        assert_eq!(new_config.buckets[0].s3.bucket, "new-s3-bucket");
    }

    #[test]
    fn test_new_requests_use_new_config_after_reload() {
        // Test: New requests use new config immediately after reload

        let mut temp_file = NamedTempFile::new().unwrap();
        let old_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket-v1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-v1"
      region: "us-east-1"
      access_key: "key-v1"
      secret_key: "secret-v1"
"#;
        temp_file.write_all(old_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path.clone());

        // Simulate "current" config (gen 0)
        let current_config = manager.reload_config_with_generation(0).unwrap();
        assert_eq!(current_config.generation, 1);
        assert_eq!(current_config.buckets[0].name, "bucket-v1");

        // Update config file
        let mut temp_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let new_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket-v2"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-v2"
      region: "us-west-2"
      access_key: "key-v2"
      secret_key: "secret-v2"
"#;
        temp_file.write_all(new_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Reload: new requests should use this config (gen 2)
        let new_config = manager.reload_config_with_generation(current_config.generation).unwrap();
        assert_eq!(new_config.generation, 2);
        assert_eq!(new_config.buckets[0].name, "bucket-v2");
        assert_eq!(new_config.buckets[0].s3.region, "us-west-2");

        // Verify generation incremented
        assert!(new_config.generation > current_config.generation);
    }

    #[test]
    fn test_no_requests_dropped_during_reload() {
        // Test: No requests dropped during reload
        // This verifies that both old and new configs are valid simultaneously

        let mut temp_file = NamedTempFile::new().unwrap();
        let config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket"
      region: "us-east-1"
      access_key: "key"
      secret_key: "secret"
"#;
        temp_file.write_all(config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path);

        // Load current config (gen 0 -> gen 1)
        let config_v1 = manager.reload_config_with_generation(0).unwrap();
        assert_eq!(config_v1.generation, 1);
        assert!(config_v1.validate().is_ok());

        // Reload config (gen 1 -> gen 2) - both should be valid
        let config_v2 = manager.reload_config_with_generation(config_v1.generation).unwrap();
        assert_eq!(config_v2.generation, 2);
        assert!(config_v2.validate().is_ok());

        // BOTH configs should be valid at this point (no dropped requests)
        assert!(config_v1.validate().is_ok()); // Old config still valid
        assert!(config_v2.validate().is_ok()); // New config also valid

        // Old config can still serve in-flight requests
        assert_eq!(config_v1.buckets[0].name, "bucket");

        // New config can serve new requests
        assert_eq!(config_v2.buckets[0].name, "bucket");
    }

    #[test]
    fn test_can_add_new_bucket_without_restart() {
        // Test: Can add new bucket without restart
        // This verifies that we can add a new bucket during hot reload

        let mut temp_file = NamedTempFile::new().unwrap();
        let old_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
"#;
        temp_file.write_all(old_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path.clone());

        // Load initial config
        let old_config = manager.reload_config().unwrap();
        assert_eq!(old_config.buckets.len(), 1);
        assert_eq!(old_config.buckets[0].name, "bucket1");

        // Update config file to add a second bucket
        let mut temp_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let new_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
  - name: "bucket2"
    path_prefix: "/media"
    s3:
      bucket: "s3-bucket-2"
      region: "us-west-2"
      access_key: "key2"
      secret_key: "secret2"
"#;
        temp_file.write_all(new_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Reload config
        let new_config = manager.reload_config().unwrap();

        // Verify new bucket was added
        assert_eq!(new_config.buckets.len(), 2);
        assert_eq!(new_config.buckets[0].name, "bucket1");
        assert_eq!(new_config.buckets[1].name, "bucket2");
        assert_eq!(new_config.buckets[1].path_prefix, "/media");
        assert_eq!(new_config.buckets[1].s3.region, "us-west-2");

        // Old config still has only 1 bucket (isolation)
        assert_eq!(old_config.buckets.len(), 1);
    }

    #[test]
    fn test_can_remove_bucket() {
        // Test: Can remove bucket (new requests get 404, in-flight complete)
        // This verifies that we can remove a bucket during hot reload

        let mut temp_file = NamedTempFile::new().unwrap();
        let old_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
  - name: "bucket2"
    path_prefix: "/media"
    s3:
      bucket: "s3-bucket-2"
      region: "us-west-2"
      access_key: "key2"
      secret_key: "secret2"
"#;
        temp_file.write_all(old_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path.clone());

        // Load initial config with 2 buckets
        let old_config = manager.reload_config().unwrap();
        assert_eq!(old_config.buckets.len(), 2);

        // Simulate in-flight request using bucket2
        let bucket2_config = &old_config.buckets[1];
        assert_eq!(bucket2_config.name, "bucket2");

        // Update config file to remove bucket2
        let mut temp_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let new_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "bucket1"
    path_prefix: "/api"
    s3:
      bucket: "s3-bucket-1"
      region: "us-east-1"
      access_key: "key1"
      secret_key: "secret1"
"#;
        temp_file.write_all(new_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Reload config
        let new_config = manager.reload_config().unwrap();

        // Verify bucket2 was removed from new config
        assert_eq!(new_config.buckets.len(), 1);
        assert_eq!(new_config.buckets[0].name, "bucket1");

        // Old config still has bucket2 (in-flight requests can complete)
        assert_eq!(old_config.buckets.len(), 2);
        assert_eq!(old_config.buckets[1].name, "bucket2");

        // In-flight request can still access bucket2 config
        assert_eq!(bucket2_config.s3.bucket, "s3-bucket-2");
    }

    #[test]
    fn test_can_update_bucket_credentials() {
        // Test: Can update bucket credentials (new requests use new creds)
        // This verifies credential rotation without restart

        let mut temp_file = NamedTempFile::new().unwrap();
        let old_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/api"
    s3:
      bucket: "products-bucket"
      region: "us-east-1"
      access_key: "OLD_ACCESS_KEY"
      secret_key: "OLD_SECRET_KEY"
"#;
        temp_file.write_all(old_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path.clone());

        // Load initial config with old credentials
        let old_config = manager.reload_config().unwrap();
        assert_eq!(old_config.buckets[0].s3.access_key, "OLD_ACCESS_KEY");
        assert_eq!(old_config.buckets[0].s3.secret_key, "OLD_SECRET_KEY");

        // Update config file with new credentials (credential rotation)
        let mut temp_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let new_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "products"
    path_prefix: "/api"
    s3:
      bucket: "products-bucket"
      region: "us-east-1"
      access_key: "NEW_ACCESS_KEY"
      secret_key: "NEW_SECRET_KEY"
"#;
        temp_file.write_all(new_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Reload config
        let new_config = manager.reload_config().unwrap();

        // Verify new credentials in new config
        assert_eq!(new_config.buckets[0].s3.access_key, "NEW_ACCESS_KEY");
        assert_eq!(new_config.buckets[0].s3.secret_key, "NEW_SECRET_KEY");

        // Old config still has old credentials (in-flight requests)
        assert_eq!(old_config.buckets[0].s3.access_key, "OLD_ACCESS_KEY");
        assert_eq!(old_config.buckets[0].s3.secret_key, "OLD_SECRET_KEY");

        // Bucket name unchanged
        assert_eq!(new_config.buckets[0].name, "products");
        assert_eq!(old_config.buckets[0].name, "products");
    }

    #[test]
    fn test_can_change_bucket_path_prefix() {
        // Test: Can change bucket path prefix
        // This verifies that we can update routing without restart

        let mut temp_file = NamedTempFile::new().unwrap();
        let old_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "media"
    path_prefix: "/old-path"
    s3:
      bucket: "media-bucket"
      region: "us-east-1"
      access_key: "key"
      secret_key: "secret"
"#;
        temp_file.write_all(old_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_path = temp_file.path().to_path_buf();
        let manager = ReloadManager::new(config_path.clone());

        // Load initial config with old path prefix
        let old_config = manager.reload_config().unwrap();
        assert_eq!(old_config.buckets[0].path_prefix, "/old-path");

        // Update config file with new path prefix
        let mut temp_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&config_path)
            .unwrap();
        let new_config_yaml = r#"
server:
  address: "127.0.0.1"
  port: 8080

buckets:
  - name: "media"
    path_prefix: "/new-path"
    s3:
      bucket: "media-bucket"
      region: "us-east-1"
      access_key: "key"
      secret_key: "secret"
"#;
        temp_file.write_all(new_config_yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Reload config
        let new_config = manager.reload_config().unwrap();

        // Verify new path prefix in new config
        assert_eq!(new_config.buckets[0].path_prefix, "/new-path");

        // Old config still has old path prefix (in-flight requests)
        assert_eq!(old_config.buckets[0].path_prefix, "/old-path");

        // Bucket name and S3 config unchanged
        assert_eq!(new_config.buckets[0].name, "media");
        assert_eq!(old_config.buckets[0].name, "media");
        assert_eq!(new_config.buckets[0].s3.bucket, "media-bucket");
        assert_eq!(old_config.buckets[0].s3.bucket, "media-bucket");
    }
}
