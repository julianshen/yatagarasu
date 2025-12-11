//! Audit logging configuration types.
//!
//! This module defines configuration structures for audit logging, including:
//! - File output with rotation policies
//! - Syslog output with configurable protocol and facility
//! - S3 export for long-term storage
//!
//! Default values for file sizes, backup counts, and intervals are sourced from
//! `crate::constants` to maintain centralized configuration defaults.

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_AUDIT_BUFFER_SIZE, DEFAULT_EXPORT_INTERVAL_SECS, DEFAULT_MAX_BACKUP_FILES,
    DEFAULT_MAX_FILE_SIZE_MB,
};

/// Audit output destination types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutput {
    File,
    Syslog,
    S3,
}

/// Audit log level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuditLogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// Log rotation policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RotationPolicy {
    /// Rotate when file reaches max size
    #[default]
    Size,
    /// Rotate daily
    Daily,
}

/// Default max file size for audit logs
fn default_max_file_size_mb() -> u64 {
    DEFAULT_MAX_FILE_SIZE_MB
}

/// Default max backup files to keep
fn default_max_backup_files() -> u32 {
    DEFAULT_MAX_BACKUP_FILES
}

/// Default audit buffer size
fn default_audit_buffer_size() -> usize {
    DEFAULT_AUDIT_BUFFER_SIZE
}

/// File output configuration for audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFileConfig {
    /// Path to the audit log file
    pub path: String,

    /// Maximum file size in MB before rotation (default: 50)
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,

    /// Maximum number of backup files to keep (default: 5)
    #[serde(default = "default_max_backup_files")]
    pub max_backup_files: u32,

    /// Rotation policy (default: size)
    #[serde(default)]
    pub rotation_policy: RotationPolicy,

    /// Buffer size in bytes (default: 1MB)
    /// Set to 0 to disable buffering
    #[serde(default = "default_audit_buffer_size")]
    pub buffer_size: usize,
}

/// Syslog transport protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SyslogProtocol {
    /// UDP transport (default)
    #[default]
    Udp,
    /// TCP transport
    Tcp,
}

/// Syslog facility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SyslogFacility {
    /// Local use 0 (default)
    #[default]
    Local0,
    /// Local use 1
    Local1,
    /// Local use 2
    Local2,
    /// Local use 3
    Local3,
    /// Local use 4
    Local4,
    /// Local use 5
    Local5,
    /// Local use 6
    Local6,
    /// Local use 7
    Local7,
}

/// Syslog output configuration for audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSyslogConfig {
    /// Syslog server address (host:port)
    pub address: String,

    /// Transport protocol (default: udp)
    #[serde(default)]
    pub protocol: SyslogProtocol,

    /// Syslog facility (default: local0)
    #[serde(default)]
    pub facility: SyslogFacility,
}

/// Default export interval in seconds (60s = 1 minute)
fn default_export_interval_seconds() -> u64 {
    DEFAULT_EXPORT_INTERVAL_SECS
}

/// S3 export configuration for audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditS3ExportConfig {
    /// S3 bucket name for audit log export
    pub bucket: String,

    /// AWS region for the bucket
    pub region: String,

    /// Path prefix for audit log files in S3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_prefix: Option<String>,

    /// Export interval in seconds (default: 60)
    #[serde(default = "default_export_interval_seconds")]
    pub export_interval_seconds: u64,
}

/// Audit log configuration for access and security event logging.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogConfig {
    /// Enable/disable audit logging (default: false)
    #[serde(default)]
    pub enabled: bool,

    /// Output destinations (file, syslog, s3)
    #[serde(default)]
    pub outputs: Vec<AuditOutput>,

    /// Log level (default: info)
    #[serde(default)]
    pub log_level: AuditLogLevel,

    /// File output configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<AuditFileConfig>,

    /// Syslog output configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syslog: Option<AuditSyslogConfig>,

    /// S3 export configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_export: Option<AuditS3ExportConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create minimal config YAML with audit section
    #[allow(dead_code)]
    fn create_yaml_with_audit(audit_section: &str) -> String {
        format!(
            r#"
server:
  address: "127.0.0.1"
  port: 8080
buckets:
  - name: test
    path_prefix: /test
    s3:
      bucket: test-bucket
      region: us-west-2
      access_key: test
      secret_key: test
{}
"#,
            audit_section
        )
    }

    #[test]
    fn test_audit_output_deserialize() {
        let file: AuditOutput = serde_yaml::from_str("file").unwrap();
        assert_eq!(file, AuditOutput::File);

        let syslog: AuditOutput = serde_yaml::from_str("syslog").unwrap();
        assert_eq!(syslog, AuditOutput::Syslog);

        let s3: AuditOutput = serde_yaml::from_str("s3").unwrap();
        assert_eq!(s3, AuditOutput::S3);
    }

    #[test]
    fn test_audit_log_level_default() {
        let level = AuditLogLevel::default();
        assert_eq!(level, AuditLogLevel::Info);
    }

    #[test]
    fn test_audit_log_level_deserialize() {
        let debug: AuditLogLevel = serde_yaml::from_str("debug").unwrap();
        assert_eq!(debug, AuditLogLevel::Debug);

        let info: AuditLogLevel = serde_yaml::from_str("info").unwrap();
        assert_eq!(info, AuditLogLevel::Info);

        let warn: AuditLogLevel = serde_yaml::from_str("warn").unwrap();
        assert_eq!(warn, AuditLogLevel::Warn);

        let error: AuditLogLevel = serde_yaml::from_str("error").unwrap();
        assert_eq!(error, AuditLogLevel::Error);
    }

    #[test]
    fn test_rotation_policy_default() {
        let policy = RotationPolicy::default();
        assert_eq!(policy, RotationPolicy::Size);
    }

    #[test]
    fn test_rotation_policy_deserialize() {
        let size: RotationPolicy = serde_yaml::from_str("size").unwrap();
        assert_eq!(size, RotationPolicy::Size);

        let daily: RotationPolicy = serde_yaml::from_str("daily").unwrap();
        assert_eq!(daily, RotationPolicy::Daily);
    }

    #[test]
    fn test_audit_file_config_defaults() {
        let yaml = r#"
path: /var/log/audit.log
"#;
        let config: AuditFileConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.path, "/var/log/audit.log");
        assert_eq!(config.max_file_size_mb, DEFAULT_MAX_FILE_SIZE_MB);
        assert_eq!(config.max_backup_files, DEFAULT_MAX_BACKUP_FILES);
        assert_eq!(config.rotation_policy, RotationPolicy::Size);
        assert_eq!(config.buffer_size, DEFAULT_AUDIT_BUFFER_SIZE);
    }

    #[test]
    fn test_audit_file_config_custom_values() {
        let yaml = r#"
path: /var/log/audit.log
max_file_size_mb: 100
max_backup_files: 10
rotation_policy: daily
buffer_size: 2097152
"#;
        let config: AuditFileConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.max_file_size_mb, 100);
        assert_eq!(config.max_backup_files, 10);
        assert_eq!(config.rotation_policy, RotationPolicy::Daily);
        assert_eq!(config.buffer_size, 2097152);
    }

    #[test]
    fn test_syslog_protocol_default() {
        let protocol = SyslogProtocol::default();
        assert_eq!(protocol, SyslogProtocol::Udp);
    }

    #[test]
    fn test_syslog_protocol_deserialize() {
        let udp: SyslogProtocol = serde_yaml::from_str("udp").unwrap();
        assert_eq!(udp, SyslogProtocol::Udp);

        let tcp: SyslogProtocol = serde_yaml::from_str("tcp").unwrap();
        assert_eq!(tcp, SyslogProtocol::Tcp);
    }

    #[test]
    fn test_syslog_facility_default() {
        let facility = SyslogFacility::default();
        assert_eq!(facility, SyslogFacility::Local0);
    }

    #[test]
    fn test_syslog_facility_deserialize() {
        let local0: SyslogFacility = serde_yaml::from_str("local0").unwrap();
        assert_eq!(local0, SyslogFacility::Local0);

        let local7: SyslogFacility = serde_yaml::from_str("local7").unwrap();
        assert_eq!(local7, SyslogFacility::Local7);
    }

    #[test]
    fn test_audit_syslog_config_defaults() {
        let yaml = r#"
address: "localhost:514"
"#;
        let config: AuditSyslogConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.address, "localhost:514");
        assert_eq!(config.protocol, SyslogProtocol::Udp);
        assert_eq!(config.facility, SyslogFacility::Local0);
    }

    #[test]
    fn test_audit_syslog_config_custom_values() {
        let yaml = r#"
address: "syslog.example.com:1514"
protocol: tcp
facility: local3
"#;
        let config: AuditSyslogConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.address, "syslog.example.com:1514");
        assert_eq!(config.protocol, SyslogProtocol::Tcp);
        assert_eq!(config.facility, SyslogFacility::Local3);
    }

    #[test]
    fn test_audit_s3_export_config_defaults() {
        let yaml = r#"
bucket: audit-logs
region: us-east-1
"#;
        let config: AuditS3ExportConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.bucket, "audit-logs");
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.path_prefix, None);
        assert_eq!(config.export_interval_seconds, DEFAULT_EXPORT_INTERVAL_SECS);
    }

    #[test]
    fn test_audit_s3_export_config_custom_values() {
        let yaml = r#"
bucket: audit-logs
region: us-east-1
path_prefix: /logs/yatagarasu/
export_interval_seconds: 300
"#;
        let config: AuditS3ExportConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.path_prefix, Some("/logs/yatagarasu/".to_string()));
        assert_eq!(config.export_interval_seconds, 300);
    }

    #[test]
    fn test_audit_log_config_default() {
        let config = AuditLogConfig::default();

        assert!(!config.enabled);
        assert!(config.outputs.is_empty());
        assert_eq!(config.log_level, AuditLogLevel::Info);
        assert!(config.file.is_none());
        assert!(config.syslog.is_none());
        assert!(config.s3_export.is_none());
    }

    #[test]
    fn test_audit_log_config_enabled_only() {
        let yaml = r#"
enabled: true
"#;
        let config: AuditLogConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert!(config.outputs.is_empty());
        assert_eq!(config.log_level, AuditLogLevel::Info);
    }

    #[test]
    fn test_audit_log_config_with_outputs() {
        let yaml = r#"
enabled: true
outputs:
  - file
  - syslog
log_level: debug
"#;
        let config: AuditLogConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert_eq!(config.outputs.len(), 2);
        assert!(config.outputs.contains(&AuditOutput::File));
        assert!(config.outputs.contains(&AuditOutput::Syslog));
        assert_eq!(config.log_level, AuditLogLevel::Debug);
    }

    #[test]
    fn test_audit_log_config_full() {
        let yaml = r#"
enabled: true
outputs:
  - file
  - syslog
  - s3
log_level: warn
file:
  path: /var/log/audit.log
  max_file_size_mb: 100
syslog:
  address: "localhost:514"
  protocol: tcp
s3_export:
  bucket: audit-logs
  region: us-east-1
  path_prefix: /logs/
  export_interval_seconds: 120
"#;
        let config: AuditLogConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert_eq!(config.outputs.len(), 3);
        assert_eq!(config.log_level, AuditLogLevel::Warn);

        let file = config.file.unwrap();
        assert_eq!(file.path, "/var/log/audit.log");
        assert_eq!(file.max_file_size_mb, 100);

        let syslog = config.syslog.unwrap();
        assert_eq!(syslog.address, "localhost:514");
        assert_eq!(syslog.protocol, SyslogProtocol::Tcp);

        let s3 = config.s3_export.unwrap();
        assert_eq!(s3.bucket, "audit-logs");
        assert_eq!(s3.export_interval_seconds, 120);
    }
}
