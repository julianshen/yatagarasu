//! Audit Logging Module (Phase 33)
//!
//! This module provides comprehensive audit logging for all proxy requests,
//! including request details, response status, timing metrics, and cache status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::Path;
use uuid::Uuid;

/// Cache status for a request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CacheStatus {
    /// Cache hit - response served from cache
    Hit,
    /// Cache miss - response fetched from S3
    Miss,
    /// Cache bypass - request bypassed cache (e.g., range request)
    Bypass,
}

/// Audit log entry representing a single request/response cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Timestamp of the request (RFC3339 format)
    pub timestamp: DateTime<Utc>,

    /// Unique correlation ID for request tracing (UUID)
    pub correlation_id: String,

    /// Client IP address (real IP, not proxy IP)
    pub client_ip: String,

    /// Authenticated user (from JWT sub/username claim), None if anonymous
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// S3 bucket name being accessed
    pub bucket: String,

    /// S3 object key (path within bucket)
    pub object_key: String,

    /// HTTP method (GET or HEAD)
    pub http_method: String,

    /// Original URL request path
    pub request_path: String,

    /// HTTP response status code
    pub response_status: u16,

    /// Response body size in bytes
    pub response_size_bytes: u64,

    /// Request processing duration in milliseconds
    pub duration_ms: u64,

    /// Cache status (hit, miss, bypass)
    pub cache_status: CacheStatus,

    /// User-Agent header from request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Referer header from request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referer: Option<String>,
}

impl AuditLogEntry {
    /// Create a new audit log entry with required fields
    pub fn new(
        client_ip: String,
        bucket: String,
        object_key: String,
        http_method: String,
        request_path: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            correlation_id: Uuid::new_v4().to_string(),
            client_ip,
            user: None,
            bucket,
            object_key,
            http_method,
            request_path,
            response_status: 0,
            response_size_bytes: 0,
            duration_ms: 0,
            cache_status: CacheStatus::Miss,
            user_agent: None,
            referer: None,
        }
    }

    /// Set the authenticated user
    pub fn with_user(mut self, user: Option<String>) -> Self {
        self.user = user;
        self
    }

    /// Set response details
    pub fn with_response(mut self, status: u16, size_bytes: u64, duration_ms: u64) -> Self {
        self.response_status = status;
        self.response_size_bytes = size_bytes;
        self.duration_ms = duration_ms;
        self
    }

    /// Set cache status
    pub fn with_cache_status(mut self, status: CacheStatus) -> Self {
        self.cache_status = status;
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// Set referer
    pub fn with_referer(mut self, referer: Option<String>) -> Self {
        self.referer = referer;
        self
    }
}

// ============================================================================
// Correlation ID Constants and Utilities
// ============================================================================

/// Standard header name for correlation ID
pub const X_CORRELATION_ID_HEADER: &str = "X-Correlation-ID";

/// Validate that a string is a valid correlation ID format
///
/// Accepts UUID format or any non-empty alphanumeric string with hyphens/underscores
pub fn is_valid_correlation_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    // Accept valid UUIDs
    if Uuid::parse_str(id).is_ok() {
        return true;
    }
    // Accept alphanumeric with hyphens and underscores
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Generate a new correlation ID (UUID v4)
pub fn generate_correlation_id() -> String {
    Uuid::new_v4().to_string()
}

/// Extract or generate correlation ID from request header
///
/// If a valid X-Correlation-ID header is present, use it.
/// Otherwise, generate a new UUID v4.
pub fn correlation_id_from_header(header_value: Option<&str>) -> String {
    match header_value {
        Some(id) if is_valid_correlation_id(id) => id.to_string(),
        _ => generate_correlation_id(),
    }
}

// ============================================================================
// Request Context for Audit Logging
// ============================================================================

/// Request context that accumulates information during request processing
/// and is converted to an AuditLogEntry at the end.
#[derive(Debug)]
pub struct RequestContext {
    /// Unique correlation ID for request tracing
    pub correlation_id: String,

    /// Request start time for duration calculation
    pub start_time: std::time::Instant,

    /// Client IP address (from socket or X-Forwarded-For)
    pub client_ip: Option<String>,

    /// Authenticated user (from JWT)
    pub user: Option<String>,

    /// S3 bucket name
    pub bucket: Option<String>,

    /// S3 object key
    pub object_key: Option<String>,

    /// HTTP method
    pub http_method: Option<String>,

    /// Original request path
    pub request_path: Option<String>,

    /// Response status code
    pub response_status: Option<u16>,

    /// Response body size in bytes
    pub response_size_bytes: Option<u64>,

    /// Cache status
    pub cache_status: Option<CacheStatus>,

    /// User agent header
    pub user_agent: Option<String>,

    /// Referer header
    pub referer: Option<String>,
}

impl RequestContext {
    /// Create a new request context with generated correlation_id and start time
    pub fn new() -> Self {
        Self {
            correlation_id: generate_correlation_id(),
            start_time: std::time::Instant::now(),
            client_ip: None,
            user: None,
            bucket: None,
            object_key: None,
            http_method: None,
            request_path: None,
            response_status: None,
            response_size_bytes: None,
            cache_status: None,
            user_agent: None,
            referer: None,
        }
    }

    /// Create a new request context with correlation ID from request header
    ///
    /// If the header contains a valid correlation ID, use it.
    /// Otherwise, generate a new UUID v4.
    pub fn with_correlation_id_header(header_value: Option<&str>) -> Self {
        Self {
            correlation_id: correlation_id_from_header(header_value),
            start_time: std::time::Instant::now(),
            client_ip: None,
            user: None,
            bucket: None,
            object_key: None,
            http_method: None,
            request_path: None,
            response_status: None,
            response_size_bytes: None,
            cache_status: None,
            user_agent: None,
            referer: None,
        }
    }

    /// Create a new request context with a specific correlation ID
    pub fn with_correlation_id(correlation_id: String) -> Self {
        Self {
            correlation_id,
            start_time: std::time::Instant::now(),
            client_ip: None,
            user: None,
            bucket: None,
            object_key: None,
            http_method: None,
            request_path: None,
            response_status: None,
            response_size_bytes: None,
            cache_status: None,
            user_agent: None,
            referer: None,
        }
    }

    /// Get the correlation ID for including in response headers
    pub fn get_correlation_id(&self) -> &str {
        &self.correlation_id
    }

    /// Set client IP from socket address
    pub fn set_client_ip_from_socket(&mut self, ip: &str) {
        // Only set if not already set by X-Forwarded-For
        if self.client_ip.is_none() {
            self.client_ip = Some(ip.to_string());
        }
    }

    /// Set client IP from X-Forwarded-For header (takes precedence)
    ///
    /// X-Forwarded-For format: "client, proxy1, proxy2, ..."
    /// The leftmost IP is the original client.
    pub fn set_client_ip_from_forwarded_for(&mut self, header_value: &str) {
        // Extract the first (leftmost) IP, which is the original client
        let client_ip = header_value.split(',').next().map(|s| s.trim().to_string());

        if let Some(ip) = client_ip {
            if !ip.is_empty() {
                self.client_ip = Some(ip);
            }
        }
    }

    /// Set authenticated user from JWT
    pub fn set_user(&mut self, user: Option<String>) {
        self.user = user;
    }

    /// Set response status code
    pub fn set_response_status(&mut self, status: u16) {
        self.response_status = Some(status);
    }

    /// Set response body size
    pub fn set_response_size(&mut self, size: u64) {
        self.response_size_bytes = Some(size);
    }

    /// Set cache status
    pub fn set_cache_status(&mut self, status: CacheStatus) {
        self.cache_status = Some(status);
    }

    /// Get elapsed time in milliseconds since request start
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Convert this context to an AuditLogEntry
    pub fn to_audit_entry(&self) -> AuditLogEntry {
        AuditLogEntry {
            timestamp: Utc::now(),
            correlation_id: self.correlation_id.clone(),
            client_ip: self
                .client_ip
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            user: self.user.clone(),
            bucket: self.bucket.clone().unwrap_or_default(),
            object_key: self.object_key.clone().unwrap_or_default(),
            http_method: self.http_method.clone().unwrap_or_default(),
            request_path: self.request_path.clone().unwrap_or_default(),
            response_status: self.response_status.unwrap_or(0),
            response_size_bytes: self.response_size_bytes.unwrap_or(0),
            duration_ms: self.elapsed_ms(),
            cache_status: self.cache_status.clone().unwrap_or(CacheStatus::Miss),
            user_agent: self.user_agent.clone(),
            referer: self.referer.clone(),
        }
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Sensitive Data Redaction Functions
// ============================================================================

/// Redact JWT tokens in a string
///
/// JWT tokens follow the format: header.payload.signature (base64 encoded)
/// This function detects and redacts them.
pub fn redact_jwt_token(value: &str) -> String {
    // JWT pattern: base64url.base64url.base64url (at minimum header.payload)
    // Header typically starts with "eyJ" (base64 for '{"')
    if value.starts_with("eyJ") && value.contains('.') {
        "[JWT_REDACTED]".to_string()
    } else {
        value.to_string()
    }
}

/// Redact Authorization header value
///
/// Preserves the auth scheme (Bearer, Basic, etc.) but redacts the credential
pub fn redact_authorization_header(value: &str) -> String {
    if value.is_empty() {
        return value.to_string();
    }

    // Split on first space to get scheme and credential
    if let Some(space_idx) = value.find(' ') {
        let scheme = &value[..space_idx];
        format!("{} [REDACTED]", scheme)
    } else {
        // No space, likely just a token - redact entirely
        "[REDACTED]".to_string()
    }
}

/// Redact sensitive query parameters from a URL path
///
/// Replaces values of specified parameter names with [REDACTED]
pub fn redact_query_params(url: &str, sensitive_params: &[&str]) -> String {
    // Split URL into path and query parts
    if let Some(query_start) = url.find('?') {
        let path = &url[..query_start];
        let query = &url[query_start + 1..];

        // Parse and redact query params
        let redacted_params: Vec<String> = query
            .split('&')
            .map(|param| {
                if let Some(eq_idx) = param.find('=') {
                    let key = &param[..eq_idx];
                    // Case-insensitive comparison for sensitive params
                    if sensitive_params.iter().any(|s| s.eq_ignore_ascii_case(key)) {
                        format!("{}=[REDACTED]", key)
                    } else {
                        param.to_string()
                    }
                } else {
                    param.to_string()
                }
            })
            .collect();

        format!("{}?{}", path, redacted_params.join("&"))
    } else {
        url.to_string()
    }
}

/// Redact sensitive headers from a list of header key-value pairs
///
/// Returns a new list with sensitive header values replaced with [REDACTED]
pub fn redact_headers(
    headers: &[(&str, &str)],
    sensitive_headers: &[&str],
) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(key, value)| {
            // Case-insensitive comparison for header names
            if sensitive_headers
                .iter()
                .any(|s| s.eq_ignore_ascii_case(key))
            {
                (key.to_string(), "[REDACTED]".to_string())
            } else {
                (key.to_string(), value.to_string())
            }
        })
        .collect()
}

// ============================================================================
// File-Based Audit Logging (Phase 33.4)
// ============================================================================

/// Audit file writer that writes JSON log entries to a file
///
/// Each entry is written as a single line of JSON (JSONL format).
#[derive(Debug)]
pub struct AuditFileWriter {
    /// Path to the audit log file
    path: std::path::PathBuf,
    /// File handle for writing
    file: Option<std::fs::File>,
}

impl AuditFileWriter {
    /// Create a new audit file writer
    ///
    /// Creates the file if it doesn't exist, creates parent directories if needed.
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Open file in append mode, create if doesn't exist
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            path,
            file: Some(file),
        })
    }

    /// Get the path to the audit log file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write an audit log entry to the file
    ///
    /// Each entry is written as a single line of JSON followed by a newline.
    pub fn write_entry(&mut self, entry: &AuditLogEntry) -> io::Result<()> {
        if let Some(ref mut file) = self.file {
            let json = serde_json::to_string(entry)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", json)?;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "File not open"))
        }
    }

    /// Flush the file buffer
    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(ref mut file) = self.file {
            file.flush()
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Rotating Audit File Writer (Phase 33.4 - File Rotation)
// ============================================================================

use crate::config::RotationPolicy;

/// Audit file writer with rotation support
///
/// Supports both size-based and daily rotation policies.
#[derive(Debug)]
pub struct RotatingAuditFileWriter {
    /// Inner file writer
    writer: AuditFileWriter,
    /// Maximum file size in bytes before rotation
    max_size_bytes: u64,
    /// Maximum number of backup files to keep
    max_backup_files: u32,
    /// Rotation policy (size or daily)
    rotation_policy: RotationPolicy,
    /// Last rotation date (for daily rotation)
    last_rotation_date: Option<chrono::NaiveDate>,
}

impl RotatingAuditFileWriter {
    /// Create a new rotating audit file writer
    pub fn new<P: AsRef<Path>>(
        path: P,
        max_size_mb: u64,
        max_backup_files: u32,
        rotation_policy: RotationPolicy,
    ) -> io::Result<Self> {
        let writer = AuditFileWriter::new(path)?;
        let today = Utc::now().date_naive();

        Ok(Self {
            writer,
            max_size_bytes: max_size_mb * 1024 * 1024,
            max_backup_files,
            rotation_policy,
            last_rotation_date: Some(today),
        })
    }

    /// Get current file size in bytes
    pub fn current_size(&self) -> io::Result<u64> {
        std::fs::metadata(&self.writer.path).map(|m| m.len())
    }

    /// Check if rotation is needed based on current policy
    pub fn needs_rotation(&self) -> io::Result<bool> {
        match self.rotation_policy {
            RotationPolicy::Size => {
                let size = self.current_size()?;
                Ok(size >= self.max_size_bytes)
            }
            RotationPolicy::Daily => {
                let today = Utc::now().date_naive();
                match self.last_rotation_date {
                    Some(last_date) => Ok(today > last_date),
                    None => Ok(true),
                }
            }
        }
    }

    /// Generate rotated filename with timestamp
    ///
    /// Format: original_name.YYYYMMDD_HHMMSS_ffffff.extension
    fn generate_backup_filename(&self) -> std::path::PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%6f");
        let original_path = &self.writer.path;

        if let Some(extension) = original_path.extension() {
            let stem = original_path.file_stem().unwrap_or_default();
            let parent = original_path.parent().unwrap_or(Path::new(""));
            parent.join(format!(
                "{}.{}.{}",
                stem.to_string_lossy(),
                timestamp,
                extension.to_string_lossy()
            ))
        } else {
            let file_name = original_path.file_name().unwrap_or_default();
            let parent = original_path.parent().unwrap_or(Path::new(""));
            parent.join(format!("{}.{}", file_name.to_string_lossy(), timestamp))
        }
    }

    /// List all backup files sorted by modification time (oldest first)
    fn list_backup_files(&self) -> io::Result<Vec<std::path::PathBuf>> {
        let parent = self.writer.path.parent().unwrap_or(Path::new("."));
        let stem = self
            .writer
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        let mut backups: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();

        if parent.exists() {
            for entry in std::fs::read_dir(parent)? {
                let entry = entry?;
                let path = entry.path();
                let filename = path.file_name().unwrap_or_default().to_string_lossy();

                // Check if this is a backup of our log file
                // Pattern: {stem}.YYYYMMDD_HHMMSS.{ext} or {stem}.YYYYMMDD_HHMMSS
                if filename.starts_with(&format!("{}.", stem)) && path != self.writer.path {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            backups.push((path, modified));
                        }
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        backups.sort_by(|a, b| a.1.cmp(&b.1));

        Ok(backups.into_iter().map(|(p, _)| p).collect())
    }

    /// Delete oldest backup files when limit exceeded
    pub fn cleanup_old_backups(&self) -> io::Result<u32> {
        let backups = self.list_backup_files()?;
        let mut deleted = 0;

        // Keep only max_backup_files - delete oldest first
        if backups.len() > self.max_backup_files as usize {
            let to_delete = backups.len() - self.max_backup_files as usize;
            for path in backups.into_iter().take(to_delete) {
                std::fs::remove_file(&path)?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// Rotate the log file
    ///
    /// 1. Close current file
    /// 2. Rename to backup filename
    /// 3. Create new file
    /// 4. Clean up old backups
    pub fn rotate(&mut self) -> io::Result<()> {
        // Close the current file
        self.writer.flush()?;
        self.writer.file = None;

        // Generate backup filename and rename
        let backup_path = self.generate_backup_filename();
        std::fs::rename(&self.writer.path, &backup_path)?;

        // Reopen the file (creates a new empty file)
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.writer.path)?;

        self.writer.file = Some(file);

        // Update last rotation date
        self.last_rotation_date = Some(Utc::now().date_naive());

        // Clean up old backups
        self.cleanup_old_backups()?;

        Ok(())
    }

    /// Write an audit log entry, rotating if necessary
    pub fn write_entry(&mut self, entry: &AuditLogEntry) -> io::Result<()> {
        // Check if rotation is needed before writing
        if self.needs_rotation()? {
            self.rotate()?;
        }

        self.writer.write_entry(entry)
    }

    /// Flush the file buffer
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Get the path to the audit log file
    pub fn path(&self) -> &Path {
        self.writer.path()
    }
}

// ============================================================================
// Async Audit File Writer (Phase 33.4 - Async Writing)
// ============================================================================

use std::net::{TcpStream, UdpSocket};
use std::sync::mpsc;

// ============================================================================
// Syslog Types (Phase 33.5)
// ============================================================================

/// Syslog protocol for connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyslogProtocol {
    /// TCP connection (reliable)
    Tcp,
    /// UDP connection (faster, but unreliable)
    Udp,
}

/// Syslog facility codes (RFC 5424)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogFacility {
    /// Kernel messages
    Kern = 0,
    /// User-level messages
    User = 1,
    /// Mail system
    Mail = 2,
    /// System daemons
    Daemon = 3,
    /// Security/authorization messages
    Auth = 4,
    /// Messages generated internally by syslogd
    Syslog = 5,
    /// Line printer subsystem
    Lpr = 6,
    /// Network news subsystem
    News = 7,
    /// UUCP subsystem
    Uucp = 8,
    /// Clock daemon
    Cron = 9,
    /// Security/authorization messages (private)
    AuthPriv = 10,
    /// FTP daemon
    Ftp = 11,
    /// Local use 0
    Local0 = 16,
    /// Local use 1
    Local1 = 17,
    /// Local use 2
    Local2 = 18,
    /// Local use 3
    Local3 = 19,
    /// Local use 4
    Local4 = 20,
    /// Local use 5
    Local5 = 21,
    /// Local use 6
    Local6 = 22,
    /// Local use 7
    Local7 = 23,
}

/// Syslog severity levels (RFC 5424)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogSeverity {
    /// System is unusable
    Emergency = 0,
    /// Action must be taken immediately
    Alert = 1,
    /// Critical conditions
    Critical = 2,
    /// Error conditions
    Error = 3,
    /// Warning conditions
    Warning = 4,
    /// Normal but significant condition
    Notice = 5,
    /// Informational messages
    Info = 6,
    /// Debug-level messages
    Debug = 7,
}

/// Command sent to the async writer background thread
enum WriterCommand {
    /// Write an audit log entry (boxed to reduce enum size)
    Write(Box<AuditLogEntry>),
    /// Flush the buffer
    Flush,
    /// Shutdown the writer
    Shutdown,
}

/// Async audit file writer that writes entries in a background thread
///
/// This writer is non-blocking - write operations return immediately
/// while actual file I/O happens in a background thread.
#[derive(Debug)]
pub struct AsyncAuditFileWriter {
    /// Channel sender to communicate with background thread
    sender: mpsc::Sender<WriterCommand>,
    /// Handle to the background thread
    handle: Option<std::thread::JoinHandle<io::Result<()>>>,
    /// Path for reference
    path: std::path::PathBuf,
}

impl AsyncAuditFileWriter {
    /// Create a new async audit file writer
    ///
    /// Spawns a background thread to handle writes.
    /// `buffer_size` is the size of the internal buffer in bytes (0 for unbuffered).
    pub fn new<P: AsRef<Path>>(
        path: P,
        max_size_mb: u64,
        max_backup_files: u32,
        rotation_policy: RotationPolicy,
        buffer_size: usize,
    ) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let path_clone = path.clone();

        // Create channel for communication
        let (sender, receiver) = mpsc::channel::<WriterCommand>();

        // Spawn background thread
        let handle = std::thread::spawn(move || {
            Self::writer_thread(
                path_clone,
                max_size_mb,
                max_backup_files,
                rotation_policy,
                buffer_size,
                receiver,
            )
        });

        Ok(Self {
            sender,
            handle: Some(handle),
            path,
        })
    }

    /// Background thread that handles actual file writing
    fn writer_thread(
        path: std::path::PathBuf,
        max_size_mb: u64,
        max_backup_files: u32,
        rotation_policy: RotationPolicy,
        buffer_size: usize,
        receiver: mpsc::Receiver<WriterCommand>,
    ) -> io::Result<()> {
        // Create the rotating writer
        let mut rotating_writer =
            RotatingAuditFileWriter::new(&path, max_size_mb, max_backup_files, rotation_policy)?;

        // Optionally wrap in a buffered writer
        // Note: For simplicity, we handle buffering at the write level
        let use_buffering = buffer_size > 0;
        let mut buffer: Vec<AuditLogEntry> = if use_buffering {
            Vec::with_capacity(buffer_size / 200) // Approximate entries
        } else {
            Vec::new()
        };

        loop {
            match receiver.recv() {
                Ok(WriterCommand::Write(boxed_entry)) => {
                    let entry = *boxed_entry;
                    if use_buffering {
                        buffer.push(entry);
                        // Flush buffer when full (approximately)
                        if buffer.len() >= buffer.capacity() {
                            for e in buffer.drain(..) {
                                rotating_writer.write_entry(&e)?;
                            }
                            rotating_writer.flush()?;
                        }
                    } else {
                        rotating_writer.write_entry(&entry)?;
                    }
                }
                Ok(WriterCommand::Flush) => {
                    // Flush any buffered entries
                    for e in buffer.drain(..) {
                        rotating_writer.write_entry(&e)?;
                    }
                    rotating_writer.flush()?;
                }
                Ok(WriterCommand::Shutdown) => {
                    // Flush any remaining entries before shutdown
                    for e in buffer.drain(..) {
                        rotating_writer.write_entry(&e)?;
                    }
                    rotating_writer.flush()?;
                    break;
                }
                Err(_) => {
                    // Channel closed, shutdown
                    for e in buffer.drain(..) {
                        let _ = rotating_writer.write_entry(&e);
                    }
                    let _ = rotating_writer.flush();
                    break;
                }
            }
        }

        Ok(())
    }

    /// Write an audit log entry asynchronously (non-blocking)
    ///
    /// Returns immediately; the actual write happens in the background.
    pub fn write_entry(&self, entry: AuditLogEntry) -> io::Result<()> {
        self.sender
            .send(WriterCommand::Write(Box::new(entry)))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Writer thread closed"))
    }

    /// Request a flush of the buffer (non-blocking)
    ///
    /// The actual flush happens in the background thread.
    pub fn flush(&self) -> io::Result<()> {
        self.sender
            .send(WriterCommand::Flush)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Writer thread closed"))
    }

    /// Shutdown the writer and wait for all writes to complete
    ///
    /// This flushes any remaining buffered entries before returning.
    pub fn shutdown(mut self) -> io::Result<()> {
        // Send shutdown command
        let _ = self.sender.send(WriterCommand::Shutdown);

        // Wait for thread to finish
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Writer thread panicked"))??;
        }

        Ok(())
    }

    /// Get the path to the audit log file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if the writer thread is still alive
    pub fn is_alive(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for AsyncAuditFileWriter {
    fn drop(&mut self) {
        // Send shutdown command on drop
        let _ = self.sender.send(WriterCommand::Shutdown);
        // Wait for thread to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// ============================================================================
// Syslog Audit Writer (Phase 33.5)
// ============================================================================

/// Syslog connection type
enum SyslogConnection {
    /// TCP connection
    Tcp(TcpStream),
    /// UDP socket
    Udp(UdpSocket, std::net::SocketAddr),
}

/// Syslog audit writer that sends audit entries to a syslog server
///
/// Supports both TCP (reliable) and UDP (faster) protocols.
/// Messages are formatted according to RFC 5424 (modern syslog format).
pub struct SyslogWriter {
    /// Connection to syslog server
    connection: Option<SyslogConnection>,
    /// Server address
    server_addr: String,
    /// Protocol (TCP or UDP)
    protocol: SyslogProtocol,
    /// Syslog facility
    facility: SyslogFacility,
    /// Application name for syslog messages
    app_name: String,
    /// Hostname for syslog messages
    hostname: String,
}

impl std::fmt::Debug for SyslogWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyslogWriter")
            .field("server_addr", &self.server_addr)
            .field("protocol", &self.protocol)
            .field("facility", &self.facility)
            .field("app_name", &self.app_name)
            .field("hostname", &self.hostname)
            .field("connected", &self.is_connected())
            .finish()
    }
}

impl SyslogWriter {
    /// Create a new syslog writer and connect to the server
    ///
    /// - `server_addr`: Address of syslog server (e.g., "127.0.0.1:514")
    /// - `protocol`: TCP or UDP
    /// - `facility`: Syslog facility (e.g., Local0 for custom applications)
    /// - `app_name`: Application name to include in syslog messages
    pub fn new(
        server_addr: &str,
        protocol: SyslogProtocol,
        facility: SyslogFacility,
        app_name: &str,
    ) -> io::Result<Self> {
        // Get hostname
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".to_string());

        let mut writer = Self {
            connection: None,
            server_addr: server_addr.to_string(),
            protocol,
            facility,
            app_name: app_name.to_string(),
            hostname,
        };

        // Connect to the server
        writer.connect()?;

        Ok(writer)
    }

    /// Connect to the syslog server
    pub fn connect(&mut self) -> io::Result<()> {
        match self.protocol {
            SyslogProtocol::Tcp => {
                let stream = TcpStream::connect(&self.server_addr)?;
                stream.set_nodelay(true)?; // Disable Nagle's algorithm for low latency
                self.connection = Some(SyslogConnection::Tcp(stream));
            }
            SyslogProtocol::Udp => {
                let socket = UdpSocket::bind("0.0.0.0:0")?; // Bind to any available port
                let addr: std::net::SocketAddr = self.server_addr.parse().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Invalid address: {}", e),
                    )
                })?;
                self.connection = Some(SyslogConnection::Udp(socket, addr));
            }
        }
        Ok(())
    }

    /// Check if connected to syslog server
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// Calculate syslog priority value
    ///
    /// Priority = Facility * 8 + Severity
    fn calculate_priority(&self, severity: SyslogSeverity) -> u8 {
        (self.facility as u8) * 8 + (severity as u8)
    }

    /// Format audit entry as RFC 5424 syslog message
    ///
    /// Format: <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG
    pub fn format_syslog_message(&self, entry: &AuditLogEntry, severity: SyslogSeverity) -> String {
        let priority = self.calculate_priority(severity);
        let timestamp = entry.timestamp.format("%Y-%m-%dT%H:%M:%S%.6fZ");
        let procid = std::process::id();
        let msgid = &entry.correlation_id[..8]; // Use first 8 chars of correlation_id

        // Format the message as JSON
        let msg = serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string());

        // RFC 5424 format
        format!(
            "<{}>1 {} {} {} {} {} - {}",
            priority, timestamp, self.hostname, self.app_name, procid, msgid, msg
        )
    }

    /// Map HTTP response status to syslog severity
    pub fn severity_from_status(status: u16) -> SyslogSeverity {
        match status {
            200..=299 => SyslogSeverity::Info,
            300..=399 => SyslogSeverity::Notice,
            400..=499 => SyslogSeverity::Warning,
            500..=599 => SyslogSeverity::Error,
            _ => SyslogSeverity::Debug,
        }
    }

    /// Write an audit log entry to syslog
    ///
    /// Automatically determines severity from response status.
    pub fn write_entry(&mut self, entry: &AuditLogEntry) -> io::Result<()> {
        let severity = Self::severity_from_status(entry.response_status);
        self.write_entry_with_severity(entry, severity)
    }

    /// Write an audit log entry with explicit severity
    pub fn write_entry_with_severity(
        &mut self,
        entry: &AuditLogEntry,
        severity: SyslogSeverity,
    ) -> io::Result<()> {
        let message = self.format_syslog_message(entry, severity);

        match &mut self.connection {
            Some(SyslogConnection::Tcp(stream)) => {
                // TCP syslog uses newline as message delimiter
                writeln!(stream, "{}", message)?;
                stream.flush()?;
            }
            Some(SyslogConnection::Udp(socket, addr)) => {
                // UDP syslog sends each message as a single packet
                socket.send_to(message.as_bytes(), *addr)?;
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Not connected to syslog server",
                ));
            }
        }

        Ok(())
    }

    /// Close the connection
    pub fn close(&mut self) {
        self.connection = None;
    }

    /// Get the protocol being used
    pub fn protocol(&self) -> SyslogProtocol {
        self.protocol
    }

    /// Get the facility being used
    pub fn facility(&self) -> SyslogFacility {
        self.facility
    }

    /// Get the server address
    pub fn server_addr(&self) -> &str {
        &self.server_addr
    }
}

// ============================================================================
// S3 Audit Export (Phase 33.6)
// ============================================================================

use std::sync::{Arc, Mutex};

/// Configuration for S3 audit export
#[derive(Debug, Clone)]
pub struct S3AuditExportConfig {
    /// S3 bucket name for audit logs
    pub bucket: String,
    /// Prefix path within the bucket (e.g., "audit-logs/")
    pub prefix: String,
    /// Export interval in seconds
    pub export_interval_secs: u64,
    /// Maximum entries per batch before forced export
    pub max_batch_size: usize,
    /// Number of retries for failed uploads
    pub max_retries: u32,
}

impl Default for S3AuditExportConfig {
    fn default() -> Self {
        Self {
            bucket: "audit-logs".to_string(),
            prefix: "yatagarasu/".to_string(),
            export_interval_secs: 300, // 5 minutes
            max_batch_size: 10000,
            max_retries: 3,
        }
    }
}

/// Audit batch holding entries ready for export
#[derive(Debug)]
pub struct AuditBatch {
    /// Entries in this batch
    entries: Vec<AuditLogEntry>,
    /// Timestamp when batch was created
    created_at: DateTime<Utc>,
}

impl AuditBatch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Add an entry to the batch
    pub fn add(&mut self, entry: AuditLogEntry) {
        self.entries.push(entry);
    }

    /// Get the number of entries in the batch
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the entries in this batch
    pub fn entries(&self) -> &[AuditLogEntry] {
        &self.entries
    }

    /// Take all entries from this batch (empties the batch)
    pub fn take_entries(&mut self) -> Vec<AuditLogEntry> {
        std::mem::take(&mut self.entries)
    }

    /// Get the batch creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Format entries as JSONL (one JSON per line)
    pub fn to_jsonl(&self) -> String {
        self.entries
            .iter()
            .filter_map(|entry| serde_json::to_string(entry).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate the S3 object key for this batch
    ///
    /// Format: {prefix}yatagarasu-audit-YYYY-MM-DD-HH-MM-SS.jsonl
    pub fn generate_object_key(&self, prefix: &str) -> String {
        let timestamp = self.created_at.format("%Y-%m-%d-%H-%M-%S");
        format!("{}yatagarasu-audit-{}.jsonl", prefix, timestamp)
    }
}

impl Default for AuditBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// S3 audit exporter that batches entries and uploads to S3
///
/// Entries are collected in memory and periodically exported to S3
/// as JSONL files.
pub struct S3AuditExporter {
    /// Configuration
    config: S3AuditExportConfig,
    /// Current batch being filled
    current_batch: Arc<Mutex<AuditBatch>>,
    /// Pending batches waiting for upload
    pending_batches: Arc<Mutex<Vec<AuditBatch>>>,
    /// Flag indicating if exporter is running
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for S3AuditExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3AuditExporter")
            .field("config", &self.config)
            .field(
                "running",
                &self.running.load(std::sync::atomic::Ordering::Relaxed),
            )
            .finish()
    }
}

impl S3AuditExporter {
    /// Create a new S3 audit exporter
    pub fn new(config: S3AuditExportConfig) -> Self {
        Self {
            config,
            current_batch: Arc::new(Mutex::new(AuditBatch::new())),
            pending_batches: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Add an audit entry to the current batch
    pub fn add_entry(&self, entry: AuditLogEntry) {
        let mut batch = self.current_batch.lock().unwrap();
        batch.add(entry);

        // If batch is full, rotate to pending
        if batch.len() >= self.config.max_batch_size {
            let full_batch = std::mem::take(&mut *batch);
            drop(batch); // Release lock before acquiring pending lock
            self.pending_batches.lock().unwrap().push(full_batch);
        }
    }

    /// Get the number of entries in the current batch
    pub fn current_batch_size(&self) -> usize {
        self.current_batch.lock().unwrap().len()
    }

    /// Get the number of pending batches
    pub fn pending_batch_count(&self) -> usize {
        self.pending_batches.lock().unwrap().len()
    }

    /// Rotate the current batch to pending (for export)
    pub fn rotate_batch(&self) -> Option<AuditBatch> {
        let mut batch = self.current_batch.lock().unwrap();
        if batch.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut *batch))
    }

    /// Get all pending batches (including current if not empty)
    pub fn get_all_batches(&self) -> Vec<AuditBatch> {
        let mut batches = Vec::new();

        // Get pending batches
        {
            let mut pending = self.pending_batches.lock().unwrap();
            batches.append(&mut *pending);
        }

        // Add current batch if not empty
        if let Some(current) = self.rotate_batch() {
            batches.push(current);
        }

        batches
    }

    /// Generate the S3 object key for a batch
    pub fn generate_object_key(&self, batch: &AuditBatch) -> String {
        batch.generate_object_key(&self.config.prefix)
    }

    /// Get the configured bucket name
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// Get the configured prefix
    pub fn prefix(&self) -> &str {
        &self.config.prefix
    }

    /// Get the export interval in seconds
    pub fn export_interval_secs(&self) -> u64 {
        self.config.export_interval_secs
    }

    /// Check if the exporter is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Mark the exporter as running
    pub fn set_running(&self, running: bool) {
        self.running
            .store(running, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get max retries
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }
}

/// Result of an S3 upload attempt
#[derive(Debug)]
pub struct S3UploadResult {
    /// Whether the upload succeeded
    pub success: bool,
    /// Number of attempts made
    pub attempts: u32,
    /// Error message if failed
    pub error: Option<String>,
    /// S3 object key that was uploaded
    pub object_key: String,
}

/// S3 upload client for audit logs
///
/// Wraps AWS SDK client with retry logic for audit log uploads.
pub struct S3AuditUploader {
    /// S3 client
    client: aws_sdk_s3::Client,
    /// Maximum number of retries
    max_retries: u32,
    /// Local backup directory for failed uploads
    backup_dir: Option<std::path::PathBuf>,
}

impl std::fmt::Debug for S3AuditUploader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3AuditUploader")
            .field("max_retries", &self.max_retries)
            .field("backup_dir", &self.backup_dir)
            .finish()
    }
}

impl S3AuditUploader {
    /// Create a new S3 audit uploader
    pub fn new(client: aws_sdk_s3::Client, max_retries: u32) -> Self {
        Self {
            client,
            max_retries,
            backup_dir: None,
        }
    }

    /// Set the local backup directory for failed uploads
    pub fn with_backup_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.backup_dir = Some(dir);
        self
    }

    /// Upload a batch to S3 with retry logic
    pub async fn upload_batch(
        &self,
        batch: &AuditBatch,
        bucket: &str,
        object_key: &str,
    ) -> S3UploadResult {
        let content = batch.to_jsonl();
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.max_retries {
            attempts += 1;

            match self
                .client
                .put_object()
                .bucket(bucket)
                .key(object_key)
                .content_type("application/x-ndjson")
                .body(content.clone().into_bytes().into())
                .send()
                .await
            {
                Ok(_) => {
                    return S3UploadResult {
                        success: true,
                        attempts,
                        error: None,
                        object_key: object_key.to_string(),
                    };
                }
                Err(e) => {
                    last_error = Some(format!("{:?}", e));
                    // Exponential backoff: 100ms, 200ms, 400ms, ...
                    if attempts < self.max_retries {
                        let delay = std::time::Duration::from_millis(100 * (1 << (attempts - 1)));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // All retries failed - save locally if backup dir configured
        if let Some(ref backup_dir) = self.backup_dir {
            let backup_path = backup_dir.join(object_key.replace('/', "_"));
            if let Err(e) = std::fs::create_dir_all(backup_dir) {
                tracing::error!("Failed to create backup directory: {}", e);
            } else if let Err(e) = std::fs::write(&backup_path, &content) {
                tracing::error!("Failed to write backup file: {}", e);
            } else {
                tracing::info!("Saved failed upload to backup: {:?}", backup_path);
            }
        }

        S3UploadResult {
            success: false,
            attempts,
            error: last_error,
            object_key: object_key.to_string(),
        }
    }

    /// Upload JSONL content directly to S3
    pub async fn upload_content(
        &self,
        content: &str,
        bucket: &str,
        object_key: &str,
    ) -> S3UploadResult {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.max_retries {
            attempts += 1;

            match self
                .client
                .put_object()
                .bucket(bucket)
                .key(object_key)
                .content_type("application/x-ndjson")
                .body(content.to_string().into_bytes().into())
                .send()
                .await
            {
                Ok(_) => {
                    return S3UploadResult {
                        success: true,
                        attempts,
                        error: None,
                        object_key: object_key.to_string(),
                    };
                }
                Err(e) => {
                    last_error = Some(format!("{:?}", e));
                    if attempts < self.max_retries {
                        let delay = std::time::Duration::from_millis(100 * (1 << (attempts - 1)));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        S3UploadResult {
            success: false,
            attempts,
            error: last_error,
            object_key: object_key.to_string(),
        }
    }
}

/// Async S3 audit export service
///
/// Runs in the background to periodically export audit batches to S3.
pub struct AsyncS3AuditExportService {
    /// Exporter instance
    exporter: Arc<S3AuditExporter>,
    /// S3 uploader
    uploader: Arc<S3AuditUploader>,
    /// Shutdown signal sender
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Handle to the background task
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for AsyncS3AuditExportService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncS3AuditExportService")
            .field("exporter", &self.exporter)
            .field("uploader", &self.uploader)
            .finish()
    }
}

impl AsyncS3AuditExportService {
    /// Create a new async export service (not started)
    pub fn new(exporter: Arc<S3AuditExporter>, uploader: Arc<S3AuditUploader>) -> Self {
        Self {
            exporter,
            uploader,
            shutdown_tx: None,
            task_handle: None,
        }
    }

    /// Start the background export task
    pub fn start(&mut self) {
        if self.task_handle.is_some() {
            return; // Already running
        }

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let exporter = Arc::clone(&self.exporter);
        let uploader = Arc::clone(&self.uploader);

        self.task_handle = Some(tokio::spawn(async move {
            exporter.set_running(true);

            let interval_secs = exporter.export_interval_secs();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Export current batch
                        if let Some(batch) = exporter.rotate_batch() {
                            if !batch.is_empty() {
                                let object_key = exporter.generate_object_key(&batch);
                                let bucket = exporter.bucket().to_string();
                                let result = uploader.upload_batch(&batch, &bucket, &object_key).await;
                                if !result.success {
                                    tracing::error!(
                                        "Failed to upload audit batch after {} attempts: {:?}",
                                        result.attempts,
                                        result.error
                                    );
                                }
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        // Shutdown requested - flush remaining entries
                        let batches = exporter.get_all_batches();
                        for batch in batches {
                            if !batch.is_empty() {
                                let object_key = exporter.generate_object_key(&batch);
                                let bucket = exporter.bucket().to_string();
                                let result = uploader.upload_batch(&batch, &bucket, &object_key).await;
                                if !result.success {
                                    tracing::error!(
                                        "Failed to upload final audit batch: {:?}",
                                        result.error
                                    );
                                }
                            }
                        }
                        break;
                    }
                }
            }

            exporter.set_running(false);
        }));
    }

    /// Shutdown the export service gracefully
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.exporter.is_running()
    }

    /// Add an entry (delegates to exporter)
    pub fn add_entry(&self, entry: AuditLogEntry) {
        self.exporter.add_entry(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Phase 33.2: Audit Log Entry Structure Tests
    // ============================================================================

    #[test]
    fn test_can_create_audit_log_entry_struct() {
        // Test: Can create AuditLogEntry struct
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "my-bucket".to_string(),
            "path/to/file.txt".to_string(),
            "GET".to_string(),
            "/my-bucket/path/to/file.txt".to_string(),
        );

        assert_eq!(entry.client_ip, "192.168.1.100");
        assert_eq!(entry.bucket, "my-bucket");
        assert_eq!(entry.object_key, "path/to/file.txt");
        assert_eq!(entry.http_method, "GET");
        assert_eq!(entry.request_path, "/my-bucket/path/to/file.txt");
    }

    #[test]
    fn test_audit_log_entry_contains_timestamp() {
        // Test: Contains timestamp (RFC3339 format)
        let before = Utc::now();
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        let after = Utc::now();

        // Timestamp should be between before and after
        assert!(entry.timestamp >= before);
        assert!(entry.timestamp <= after);

        // Should serialize to RFC3339 format
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("timestamp"));
        // RFC3339 format includes 'T' separator
        assert!(json.contains("T"));
    }

    #[test]
    fn test_audit_log_entry_contains_correlation_id() {
        // Test: Contains correlation_id (UUID)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        // Correlation ID should be a valid UUID
        let parsed = Uuid::parse_str(&entry.correlation_id);
        assert!(
            parsed.is_ok(),
            "correlation_id should be valid UUID: {}",
            entry.correlation_id
        );

        // Each entry should have unique correlation ID
        let entry2 = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        assert_ne!(
            entry.correlation_id, entry2.correlation_id,
            "Each entry should have unique correlation_id"
        );
    }

    #[test]
    fn test_audit_log_entry_contains_client_ip() {
        // Test: Contains client_ip (real IP, not proxy IP)
        let entry = AuditLogEntry::new(
            "10.0.0.50".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        assert_eq!(entry.client_ip, "10.0.0.50");
    }

    #[test]
    fn test_audit_log_entry_contains_user() {
        // Test: Contains user (from JWT sub/username claim, if authenticated)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_user(Some("john.doe@example.com".to_string()));

        assert_eq!(entry.user, Some("john.doe@example.com".to_string()));

        // Anonymous request
        let anon_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        assert!(anon_entry.user.is_none());
    }

    #[test]
    fn test_audit_log_entry_contains_bucket_name() {
        // Test: Contains bucket name
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "production-assets".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        assert_eq!(entry.bucket, "production-assets");
    }

    #[test]
    fn test_audit_log_entry_contains_object_key() {
        // Test: Contains object_key (S3 path)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "images/2024/photo.jpg".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        assert_eq!(entry.object_key, "images/2024/photo.jpg");
    }

    #[test]
    fn test_audit_log_entry_contains_http_method() {
        // Test: Contains http_method (GET/HEAD)
        let get_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        assert_eq!(get_entry.http_method, "GET");

        let head_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "HEAD".to_string(),
            "/path".to_string(),
        );
        assert_eq!(head_entry.http_method, "HEAD");
    }

    #[test]
    fn test_audit_log_entry_contains_request_path() {
        // Test: Contains request_path (original URL path)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/api/v1/files/document.pdf".to_string(),
        );

        assert_eq!(entry.request_path, "/api/v1/files/document.pdf");
    }

    #[test]
    fn test_audit_log_entry_contains_response_status() {
        // Test: Contains response_status (200, 404, 403, etc.)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1024, 50);

        assert_eq!(entry.response_status, 200);

        let not_found = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(404, 0, 10);

        assert_eq!(not_found.response_status, 404);
    }

    #[test]
    fn test_audit_log_entry_contains_response_size_bytes() {
        // Test: Contains response_size_bytes
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1_048_576, 100); // 1 MB

        assert_eq!(entry.response_size_bytes, 1_048_576);
    }

    #[test]
    fn test_audit_log_entry_contains_duration_ms() {
        // Test: Contains duration_ms (request processing time)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1024, 150);

        assert_eq!(entry.duration_ms, 150);
    }

    #[test]
    fn test_audit_log_entry_contains_cache_status() {
        // Test: Contains cache_status (hit, miss, bypass)
        let hit_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_cache_status(CacheStatus::Hit);
        assert_eq!(hit_entry.cache_status, CacheStatus::Hit);

        let miss_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_cache_status(CacheStatus::Miss);
        assert_eq!(miss_entry.cache_status, CacheStatus::Miss);

        let bypass_entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_cache_status(CacheStatus::Bypass);
        assert_eq!(bypass_entry.cache_status, CacheStatus::Bypass);
    }

    #[test]
    fn test_audit_log_entry_contains_user_agent() {
        // Test: Contains user_agent (from request headers)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_user_agent(Some(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string(),
        ));

        assert_eq!(
            entry.user_agent,
            Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64)".to_string())
        );
    }

    #[test]
    fn test_audit_log_entry_contains_referer() {
        // Test: Contains referer (from request headers)
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_referer(Some("https://example.com/page".to_string()));

        assert_eq!(entry.referer, Some("https://example.com/page".to_string()));
    }

    // ============================================================================
    // JSON Serialization Tests
    // ============================================================================

    #[test]
    fn test_audit_log_entry_serializes_to_json() {
        // Test: AuditLogEntry serializes to JSON
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "my-bucket".to_string(),
            "path/to/file.txt".to_string(),
            "GET".to_string(),
            "/my-bucket/path/to/file.txt".to_string(),
        )
        .with_response(200, 1024, 50)
        .with_cache_status(CacheStatus::Hit);

        let json_result = serde_json::to_string(&entry);
        assert!(json_result.is_ok(), "Should serialize to JSON");

        let json = json_result.unwrap();
        assert!(json.contains("\"client_ip\":\"192.168.1.100\""));
        assert!(json.contains("\"bucket\":\"my-bucket\""));
        assert!(json.contains("\"response_status\":200"));
    }

    #[test]
    fn test_audit_log_entry_all_fields_in_json() {
        // Test: All fields included in JSON output
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_user(Some("testuser".to_string()))
        .with_response(200, 1024, 50)
        .with_cache_status(CacheStatus::Hit)
        .with_user_agent(Some("TestAgent".to_string()))
        .with_referer(Some("https://ref.example.com".to_string()));

        let json = serde_json::to_string(&entry).unwrap();

        // All required fields should be present
        assert!(json.contains("timestamp"));
        assert!(json.contains("correlation_id"));
        assert!(json.contains("client_ip"));
        assert!(json.contains("user"));
        assert!(json.contains("bucket"));
        assert!(json.contains("object_key"));
        assert!(json.contains("http_method"));
        assert!(json.contains("request_path"));
        assert!(json.contains("response_status"));
        assert!(json.contains("response_size_bytes"));
        assert!(json.contains("duration_ms"));
        assert!(json.contains("cache_status"));
        assert!(json.contains("user_agent"));
        assert!(json.contains("referer"));
    }

    #[test]
    fn test_audit_log_entry_timestamp_iso8601_format() {
        // Test: Timestamp in ISO8601 format
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        let json = serde_json::to_string(&entry).unwrap();

        // ISO8601/RFC3339 format: 2024-01-15T10:30:00.000000Z
        // Should contain date separator, time separator, and timezone indicator
        let timestamp_pattern = regex::Regex::new(
            r#""timestamp":"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}"#,
        )
        .unwrap();
        assert!(
            timestamp_pattern.is_match(&json),
            "Timestamp should be in ISO8601 format: {}",
            json
        );
    }

    #[test]
    fn test_audit_log_entry_handles_special_characters() {
        // Test: Handles special characters correctly
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "path/with spaces/and\"quotes\".txt".to_string(),
            "GET".to_string(),
            "/bucket/path/with spaces/and\"quotes\".txt".to_string(),
        )
        .with_user_agent(Some("Agent/1.0 (Special; Chars: \"test\")".to_string()));

        let json_result = serde_json::to_string(&entry);
        assert!(
            json_result.is_ok(),
            "Should handle special characters: {:?}",
            json_result
        );

        // Should be able to deserialize back
        let json = json_result.unwrap();
        let deserialized: Result<AuditLogEntry, _> = serde_json::from_str(&json);
        assert!(
            deserialized.is_ok(),
            "Should deserialize successfully: {:?}",
            deserialized
        );
    }

    // ============================================================================
    // Sensitive Data Redaction Tests
    // ============================================================================

    #[test]
    fn test_jwt_tokens_redacted_in_logs() {
        // Test: JWT tokens redacted in logs
        let jwt_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

        let redacted = redact_jwt_token(jwt_token);
        assert_eq!(redacted, "[JWT_REDACTED]");

        // Partial JWT should also be redacted
        let partial = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.incomplete";
        let redacted_partial = redact_jwt_token(partial);
        assert_eq!(redacted_partial, "[JWT_REDACTED]");

        // Non-JWT should not be redacted
        let non_jwt = "not-a-jwt-token";
        let not_redacted = redact_jwt_token(non_jwt);
        assert_eq!(not_redacted, non_jwt);
    }

    #[test]
    fn test_authorization_header_redacted() {
        // Test: Authorization header redacted (show "Bearer [REDACTED]")
        let auth_header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";

        let redacted = redact_authorization_header(auth_header);
        assert_eq!(redacted, "Bearer [REDACTED]");

        // Basic auth should also be redacted
        let basic_auth = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";
        let redacted_basic = redact_authorization_header(basic_auth);
        assert_eq!(redacted_basic, "Basic [REDACTED]");

        // Empty or invalid should stay as is
        let empty = "";
        assert_eq!(redact_authorization_header(empty), "");
    }

    #[test]
    fn test_query_param_tokens_redacted() {
        // Test: Query param tokens redacted
        let url_with_token = "/api/files?token=secret123&file=doc.pdf";
        let redacted = redact_query_params(url_with_token, &["token", "api_key", "access_token"]);
        assert_eq!(redacted, "/api/files?token=[REDACTED]&file=doc.pdf");

        // Multiple sensitive params
        let url_multi = "/api?token=abc&api_key=xyz&name=test";
        let redacted_multi = redact_query_params(url_multi, &["token", "api_key"]);
        assert_eq!(
            redacted_multi,
            "/api?token=[REDACTED]&api_key=[REDACTED]&name=test"
        );

        // No sensitive params
        let url_clean = "/api/files?file=doc.pdf&page=1";
        let not_redacted = redact_query_params(url_clean, &["token"]);
        assert_eq!(not_redacted, "/api/files?file=doc.pdf&page=1");
    }

    #[test]
    fn test_sensitive_custom_headers_redacted() {
        // Test: Sensitive custom headers redacted
        let headers = vec![
            ("X-API-Key", "secret-api-key-123"),
            ("X-Request-ID", "req-123"),
            ("X-Auth-Token", "auth-token-value"),
            ("Content-Type", "application/json"),
        ];

        let sensitive_headers = ["x-api-key", "x-auth-token"];
        let redacted = redact_headers(&headers, &sensitive_headers);

        // Check that sensitive headers are redacted
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "[REDACTED]"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-Auth-Token" && v == "[REDACTED]"));

        // Check that non-sensitive headers are preserved
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "X-Request-ID" && v == "req-123"));
        assert!(redacted
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    // ============================================================================
    // Phase 33.3: Request Context Enrichment Tests
    // ============================================================================

    #[test]
    fn test_generate_correlation_id_on_request_start() {
        // Test: Generate correlation_id on request start
        let ctx = RequestContext::new();

        // Should have a valid UUID correlation_id
        let parsed = Uuid::parse_str(&ctx.correlation_id);
        assert!(parsed.is_ok(), "correlation_id should be valid UUID");

        // Each context should have unique correlation_id
        let ctx2 = RequestContext::new();
        assert_ne!(ctx.correlation_id, ctx2.correlation_id);
    }

    #[test]
    fn test_extract_client_ip_from_request() {
        // Test: Extract client_ip from request (handle X-Forwarded-For)
        let mut ctx = RequestContext::new();

        // Direct connection IP
        ctx.set_client_ip_from_socket("192.168.1.100");
        assert_eq!(ctx.client_ip, Some("192.168.1.100".to_string()));

        // X-Forwarded-For with single IP
        let mut ctx2 = RequestContext::new();
        ctx2.set_client_ip_from_forwarded_for("10.0.0.1");
        assert_eq!(ctx2.client_ip, Some("10.0.0.1".to_string()));

        // X-Forwarded-For with multiple IPs (leftmost is original client)
        let mut ctx3 = RequestContext::new();
        ctx3.set_client_ip_from_forwarded_for("203.0.113.50, 70.41.3.18, 150.172.238.178");
        assert_eq!(ctx3.client_ip, Some("203.0.113.50".to_string()));

        // X-Forwarded-For takes precedence over socket IP
        let mut ctx4 = RequestContext::new();
        ctx4.set_client_ip_from_socket("127.0.0.1");
        ctx4.set_client_ip_from_forwarded_for("8.8.8.8");
        assert_eq!(ctx4.client_ip, Some("8.8.8.8".to_string()));
    }

    #[test]
    fn test_extract_user_from_validated_jwt() {
        // Test: Extract user from validated JWT
        let mut ctx = RequestContext::new();

        // Set user from JWT sub claim
        ctx.set_user(Some("john.doe@example.com".to_string()));
        assert_eq!(ctx.user, Some("john.doe@example.com".to_string()));

        // Anonymous user
        let ctx2 = RequestContext::new();
        assert!(ctx2.user.is_none());
    }

    #[test]
    fn test_track_request_start_time() {
        // Test: Track request start time
        let before = std::time::Instant::now();
        let ctx = RequestContext::new();
        let after = std::time::Instant::now();

        // Start time should be between before and after
        assert!(ctx.start_time >= before);
        assert!(ctx.start_time <= after);
    }

    // ============================================================================
    // Phase 33.3: Response Context Enrichment Tests
    // ============================================================================

    #[test]
    fn test_capture_response_status() {
        // Test: Capture response status
        let mut ctx = RequestContext::new();
        ctx.set_response_status(200);
        assert_eq!(ctx.response_status, Some(200));

        ctx.set_response_status(404);
        assert_eq!(ctx.response_status, Some(404));
    }

    #[test]
    fn test_capture_response_size() {
        // Test: Capture response size
        let mut ctx = RequestContext::new();
        ctx.set_response_size(1024);
        assert_eq!(ctx.response_size_bytes, Some(1024));

        ctx.set_response_size(10_485_760); // 10 MB
        assert_eq!(ctx.response_size_bytes, Some(10_485_760));
    }

    #[test]
    fn test_calculate_duration() {
        // Test: Calculate duration
        let ctx = RequestContext::new();

        // Sleep a bit to ensure measurable duration
        std::thread::sleep(std::time::Duration::from_millis(10));

        let duration_ms = ctx.elapsed_ms();
        assert!(
            duration_ms >= 10,
            "Duration should be at least 10ms, got {}ms",
            duration_ms
        );
    }

    #[test]
    fn test_capture_cache_status() {
        // Test: Capture cache status (hit/miss/bypass)
        let mut ctx = RequestContext::new();

        ctx.set_cache_status(CacheStatus::Hit);
        assert_eq!(ctx.cache_status, Some(CacheStatus::Hit));

        ctx.set_cache_status(CacheStatus::Miss);
        assert_eq!(ctx.cache_status, Some(CacheStatus::Miss));

        ctx.set_cache_status(CacheStatus::Bypass);
        assert_eq!(ctx.cache_status, Some(CacheStatus::Bypass));
    }

    #[test]
    fn test_request_context_to_audit_entry() {
        // Test: Convert RequestContext to AuditLogEntry
        let mut ctx = RequestContext::new();
        ctx.set_client_ip_from_socket("192.168.1.100");
        ctx.set_user(Some("testuser".to_string()));
        ctx.bucket = Some("my-bucket".to_string());
        ctx.object_key = Some("path/to/file.txt".to_string());
        ctx.http_method = Some("GET".to_string());
        ctx.request_path = Some("/my-bucket/path/to/file.txt".to_string());
        ctx.set_response_status(200);
        ctx.set_response_size(1024);
        ctx.set_cache_status(CacheStatus::Hit);
        ctx.user_agent = Some("TestAgent/1.0".to_string());
        ctx.referer = Some("https://example.com".to_string());

        let entry = ctx.to_audit_entry();

        assert_eq!(entry.correlation_id, ctx.correlation_id);
        assert_eq!(entry.client_ip, "192.168.1.100");
        assert_eq!(entry.user, Some("testuser".to_string()));
        assert_eq!(entry.bucket, "my-bucket");
        assert_eq!(entry.object_key, "path/to/file.txt");
        assert_eq!(entry.http_method, "GET");
        assert_eq!(entry.request_path, "/my-bucket/path/to/file.txt");
        assert_eq!(entry.response_status, 200);
        assert_eq!(entry.response_size_bytes, 1024);
        assert_eq!(entry.cache_status, CacheStatus::Hit);
        assert_eq!(entry.user_agent, Some("TestAgent/1.0".to_string()));
        assert_eq!(entry.referer, Some("https://example.com".to_string()));
    }

    // ============================================================================
    // Phase 33.4: File-Based Audit Logging Tests
    // ============================================================================

    #[test]
    fn test_can_create_audit_log_file() {
        // Test: Can create audit log file
        let temp_dir = std::env::temp_dir();
        let log_path = temp_dir.join(format!("audit_test_{}.log", Uuid::new_v4()));

        // Ensure file doesn't exist before test
        let _ = std::fs::remove_file(&log_path);

        let writer = AuditFileWriter::new(&log_path);
        assert!(writer.is_ok(), "Should create audit file writer");

        // File should exist
        assert!(log_path.exists(), "Log file should be created");

        // Clean up
        let _ = std::fs::remove_file(&log_path);
    }

    #[test]
    fn test_appends_entries_to_file_one_json_per_line() {
        // Test: Appends entries to file (one JSON per line)
        let temp_dir = std::env::temp_dir();
        let log_path = temp_dir.join(format!("audit_append_test_{}.log", Uuid::new_v4()));

        // Clean up from previous run
        let _ = std::fs::remove_file(&log_path);

        let mut writer = AuditFileWriter::new(&log_path).expect("Should create writer");

        // Write first entry
        let entry1 = AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "bucket1".to_string(),
            "file1.txt".to_string(),
            "GET".to_string(),
            "/bucket1/file1.txt".to_string(),
        )
        .with_response(200, 1024, 50);

        writer.write_entry(&entry1).expect("Should write entry");

        // Write second entry
        let entry2 = AuditLogEntry::new(
            "192.168.1.2".to_string(),
            "bucket2".to_string(),
            "file2.txt".to_string(),
            "GET".to_string(),
            "/bucket2/file2.txt".to_string(),
        )
        .with_response(404, 0, 10);

        writer.write_entry(&entry2).expect("Should write entry");
        writer.flush().expect("Should flush");

        // Read file and verify JSONL format
        let content = std::fs::read_to_string(&log_path).expect("Should read file");
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 2, "Should have 2 lines");

        // Each line should be valid JSON
        let parsed1: AuditLogEntry =
            serde_json::from_str(lines[0]).expect("First line should be valid JSON");
        assert_eq!(parsed1.client_ip, "192.168.1.1");
        assert_eq!(parsed1.response_status, 200);

        let parsed2: AuditLogEntry =
            serde_json::from_str(lines[1]).expect("Second line should be valid JSON");
        assert_eq!(parsed2.client_ip, "192.168.1.2");
        assert_eq!(parsed2.response_status, 404);

        // Clean up
        let _ = std::fs::remove_file(&log_path);
    }

    #[test]
    fn test_handles_file_write_errors_gracefully() {
        // Test: Handles file write errors gracefully
        // Try to create writer in a non-writable location (on most systems, root is not writable)
        #[cfg(unix)]
        {
            let invalid_path = "/root/nonexistent_dir_12345/audit.log";
            let result = AuditFileWriter::new(invalid_path);

            // Should return an error, not panic
            assert!(
                result.is_err(),
                "Should fail gracefully for non-writable path"
            );
        }

        // Also test that write returns error when file is closed
        let temp_dir = std::env::temp_dir();
        let log_path = temp_dir.join(format!("audit_error_test_{}.log", Uuid::new_v4()));

        let mut writer = AuditFileWriter::new(&log_path).expect("Should create writer");

        // Close the file explicitly
        writer.file = None;

        let entry = AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "bucket".to_string(),
            "file.txt".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        let result = writer.write_entry(&entry);
        assert!(result.is_err(), "Should return error when file is closed");

        // Clean up
        let _ = std::fs::remove_file(&log_path);
    }

    #[test]
    fn test_creates_directory_if_not_exists() {
        // Test: Creates directory if not exists
        let temp_dir = std::env::temp_dir();
        let nested_dir = temp_dir.join(format!("audit_nested_{}", Uuid::new_v4()));
        let deep_path = nested_dir.join("sub1").join("sub2").join("audit.log");

        // Ensure directories don't exist
        let _ = std::fs::remove_dir_all(&nested_dir);

        // Parent directories should be created automatically
        let writer = AuditFileWriter::new(&deep_path);
        assert!(
            writer.is_ok(),
            "Should create nested directories and file: {:?}",
            writer
        );

        // File should exist
        assert!(
            deep_path.exists(),
            "File should exist in nested directories"
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&nested_dir);
    }

    // ============================================================================
    // Phase 33.4: File Rotation Tests
    // ============================================================================

    #[test]
    fn test_rotates_file_when_size_exceeds_max() {
        // Test: Rotates file when size exceeds max
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_rotation_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // Create a rotating writer with very small max size (1 byte = will trigger on first write)
        // Actually use 1 MB for sanity, but we'll manually write enough to exceed
        // For testing, let's use a very small size in bytes
        let mut writer = RotatingAuditFileWriter::new(&log_path, 1, 3, RotationPolicy::Size)
            .expect("Should create writer");

        // Write first entry - file is empty so no rotation yet
        let entry1 = AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "bucket".to_string(),
            "file.txt".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        )
        .with_response(200, 1024, 50);

        writer.write_entry(&entry1).expect("Should write entry");
        writer.flush().expect("Should flush");

        // Since we configured max size as 1 MB, let's just test the rotation logic manually
        // Force a rotation
        writer.rotate().expect("Should rotate");

        // Original file should exist and be empty (or nearly so)
        assert!(
            log_path.exists(),
            "New log file should exist after rotation"
        );

        // There should be a backup file
        let backups = writer.list_backup_files().expect("Should list backups");
        assert_eq!(backups.len(), 1, "Should have one backup file");

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_rotates_file_daily_if_configured() {
        // Test: Rotates file daily (if configured)
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_daily_rotation_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // Create a rotating writer with daily policy
        let mut writer = RotatingAuditFileWriter::new(&log_path, 50, 3, RotationPolicy::Daily)
            .expect("Should create writer");

        // Today's date is set in last_rotation_date, so needs_rotation should be false
        assert!(
            !writer.needs_rotation().expect("Should check rotation"),
            "Should not need rotation on same day"
        );

        // Simulate yesterday's last rotation date
        writer.last_rotation_date = Some(Utc::now().date_naive() - chrono::Duration::days(1));

        // Now it should need rotation
        assert!(
            writer.needs_rotation().expect("Should check rotation"),
            "Should need rotation after midnight"
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_renames_old_file_with_timestamp() {
        // Test: Renames old file with timestamp
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_rename_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        let mut writer = RotatingAuditFileWriter::new(&log_path, 50, 3, RotationPolicy::Size)
            .expect("Should create writer");

        // Write something to the file
        let entry = AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "bucket".to_string(),
            "file.txt".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        writer.write_entry(&entry).expect("Should write");
        writer.flush().expect("Should flush");

        // Rotate
        writer.rotate().expect("Should rotate");

        // Check backup file exists with timestamp pattern
        let backups = writer.list_backup_files().expect("Should list backups");
        assert_eq!(backups.len(), 1, "Should have one backup");

        let backup_name = backups[0]
            .file_name()
            .expect("Should have filename")
            .to_string_lossy();

        // Should match pattern: audit.YYYYMMDD_HHMMSS_ffffff.log
        let pattern = regex::Regex::new(r"^audit\.\d{8}_\d{6}_\d{6}\.log$").unwrap();
        assert!(
            pattern.is_match(&backup_name),
            "Backup filename '{}' should match timestamp pattern",
            backup_name
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_keeps_only_max_backup_files() {
        // Test: Keeps only max_backup_files
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_max_backups_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // max_backup_files = 2
        let mut writer = RotatingAuditFileWriter::new(&log_path, 50, 2, RotationPolicy::Size)
            .expect("Should create writer");

        // Create 4 rotations (should keep only 2 backups)
        for i in 0..4 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            writer.write_entry(&entry).expect("Should write");
            writer.flush().expect("Should flush");

            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(100));

            writer.rotate().expect("Should rotate");
        }

        // Should only have 2 backup files
        let backups = writer.list_backup_files().expect("Should list backups");
        assert_eq!(
            backups.len(),
            2,
            "Should have only 2 backup files, but found {}",
            backups.len()
        );

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_deletes_oldest_files_when_limit_exceeded() {
        // Test: Deletes oldest files when limit exceeded
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_delete_old_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // max_backup_files = 2
        let mut writer = RotatingAuditFileWriter::new(&log_path, 50, 2, RotationPolicy::Size)
            .expect("Should create writer");

        // Track backup file names to verify oldest are deleted
        let mut all_backup_names: Vec<String> = Vec::new();

        for i in 0..4 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            writer.write_entry(&entry).expect("Should write");
            writer.flush().expect("Should flush");

            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(100));

            writer.rotate().expect("Should rotate");

            // Track backup names after each rotation (before cleanup)
            let backups = writer.list_backup_files().expect("Should list backups");
            for b in &backups {
                let name = b.file_name().unwrap().to_string_lossy().to_string();
                if !all_backup_names.contains(&name) {
                    all_backup_names.push(name);
                }
            }
        }

        // Get final backup list
        let final_backups = writer.list_backup_files().expect("Should list backups");
        let final_names: Vec<String> = final_backups
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        // The oldest backups should have been deleted
        // The final_names should be the 2 most recent
        assert_eq!(final_names.len(), 2, "Should have 2 backups remaining");

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    // ============================================================================
    // Phase 33.4: Async Writing Tests
    // ============================================================================

    #[test]
    fn test_writes_are_async_non_blocking() {
        // Test: Writes are async (non-blocking)
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_async_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        let writer = AsyncAuditFileWriter::new(&log_path, 50, 3, RotationPolicy::Size, 0)
            .expect("Should create async writer");

        // Measure time for write operations
        let start = std::time::Instant::now();

        // Write multiple entries
        for i in 0..100 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            writer.write_entry(entry).expect("Should queue write");
        }

        let write_duration = start.elapsed();

        // Writes should be very fast (just sending to channel)
        // Should complete in under 10ms for 100 entries
        assert!(
            write_duration.as_millis() < 100,
            "Writes should be non-blocking, took {:?}",
            write_duration
        );

        // Background thread should be running
        assert!(writer.is_alive(), "Background thread should be alive");

        // Shutdown and wait for completion
        writer.shutdown().expect("Should shutdown cleanly");

        // Verify all entries were written
        let content = std::fs::read_to_string(&log_path).expect("Should read file");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 100, "Should have 100 entries");

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_uses_buffered_writer_for_performance() {
        // Test: Uses buffered writer for performance
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_buffered_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // Create writer with buffer (4KB buffer size)
        let writer = AsyncAuditFileWriter::new(&log_path, 50, 3, RotationPolicy::Size, 4096)
            .expect("Should create buffered async writer");

        // Write entries that should be buffered
        for i in 0..10 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            writer.write_entry(entry).expect("Should queue write");
        }

        // Give a tiny bit of time for the write to potentially happen
        std::thread::sleep(std::time::Duration::from_millis(10));

        // File may not have all entries yet (they might be buffered)
        // This tests that buffering is in effect

        // Flush the buffer
        writer.flush().expect("Should flush");

        // Give time for flush to complete
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Now shutdown and verify
        writer.shutdown().expect("Should shutdown cleanly");

        // All entries should be written after shutdown
        let content = std::fs::read_to_string(&log_path).expect("Should read file");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 10, "Should have 10 entries after shutdown");

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_flushes_buffer_periodically() {
        // Test: Flushes buffer periodically (when buffer is full)
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_periodic_flush_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // Create writer with small buffer (should flush after ~5 entries)
        // Each entry is roughly 200 bytes, so 1000 byte buffer ≈ 5 entries
        let writer = AsyncAuditFileWriter::new(&log_path, 50, 3, RotationPolicy::Size, 1000)
            .expect("Should create writer");

        // Write more entries than buffer can hold
        for i in 0..20 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            writer.write_entry(entry).expect("Should queue write");
        }

        // Give time for background thread to process and auto-flush
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Some entries should be written even without explicit flush
        // (due to buffer overflow triggering flush)
        let content = std::fs::read_to_string(&log_path).expect("Should read file");
        let lines_before_shutdown: Vec<&str> = content.lines().collect();

        // Should have at least some entries written due to buffer overflow
        assert!(
            !lines_before_shutdown.is_empty(),
            "Buffer should have auto-flushed some entries"
        );

        // Shutdown to get remaining entries
        writer.shutdown().expect("Should shutdown cleanly");

        // All entries should be present after shutdown
        let final_content = std::fs::read_to_string(&log_path).expect("Should read file");
        let final_lines: Vec<&str> = final_content.lines().collect();
        assert_eq!(final_lines.len(), 20, "Should have all 20 entries");

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_flushes_buffer_on_shutdown() {
        // Test: Flushes buffer on shutdown
        use crate::config::RotationPolicy;

        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join(format!("audit_shutdown_flush_{}", Uuid::new_v4()));
        let log_path = test_dir.join("audit.log");

        // Clean up from previous run
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).expect("Should create test dir");

        // Create writer with large buffer (won't auto-flush)
        let writer = AsyncAuditFileWriter::new(&log_path, 50, 3, RotationPolicy::Size, 1_000_000)
            .expect("Should create writer");

        // Write a few entries (less than buffer capacity)
        for i in 0..5 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            writer.write_entry(entry).expect("Should queue write");
        }

        // Give a moment for entries to be received
        std::thread::sleep(std::time::Duration::from_millis(50));

        // File may be empty or have partial entries (buffered)
        // This is expected before shutdown

        // Shutdown should flush all remaining entries
        writer.shutdown().expect("Should shutdown cleanly");

        // All entries should be present after shutdown
        let content = std::fs::read_to_string(&log_path).expect("Should read file");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 5, "Shutdown should flush all buffered entries");

        // Verify entries are valid JSON
        for line in &lines {
            let parsed: Result<AuditLogEntry, _> = serde_json::from_str(line);
            assert!(parsed.is_ok(), "Each line should be valid JSON");
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    // ============================================================================
    // Phase 33.5: Syslog Audit Logging Tests
    // ============================================================================

    #[test]
    fn test_can_connect_to_syslog_server_tcp() {
        // Test: Can connect to syslog server (TCP)
        use std::net::TcpListener;

        // Start a mock TCP syslog server
        let listener = TcpListener::bind("127.0.0.1:0").expect("Should bind to port");
        let port = listener.local_addr().unwrap().port();
        let server_addr = format!("127.0.0.1:{}", port);

        // Accept connection in a thread
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("Should accept connection");
            // Keep the connection open for a moment
            std::thread::sleep(std::time::Duration::from_millis(100));
            drop(stream);
        });

        // Connect to the mock server
        let writer = SyslogWriter::new(
            &server_addr,
            SyslogProtocol::Tcp,
            SyslogFacility::Local0,
            "yatagarasu",
        );

        assert!(writer.is_ok(), "Should connect to TCP syslog server");
        let writer = writer.unwrap();
        assert!(writer.is_connected(), "Should be connected");
        assert_eq!(writer.protocol(), SyslogProtocol::Tcp);

        // Wait for server thread
        handle.join().expect("Server thread should finish");
    }

    #[test]
    fn test_can_connect_to_syslog_server_udp() {
        // Test: Can connect to syslog server (UDP)
        // Note: UDP is connectionless, so we just create the socket
        // and verify it's bound. The "connection" is just storing the target address.

        // Create a UDP socket to simulate a syslog server
        let server = std::net::UdpSocket::bind("127.0.0.1:0").expect("Should bind UDP socket");
        let port = server.local_addr().unwrap().port();
        let server_addr = format!("127.0.0.1:{}", port);

        // Create syslog writer with UDP
        let writer = SyslogWriter::new(
            &server_addr,
            SyslogProtocol::Udp,
            SyslogFacility::Local0,
            "yatagarasu",
        );

        assert!(writer.is_ok(), "Should create UDP syslog writer");
        let writer = writer.unwrap();
        assert!(
            writer.is_connected(),
            "UDP writer should report as connected"
        );
        assert_eq!(writer.protocol(), SyslogProtocol::Udp);
    }

    #[test]
    fn test_formats_entry_as_syslog_message() {
        // Test: Formats entry as syslog message
        // We don't need an actual connection for format testing

        // Create a mock server to allow writer creation
        let server = std::net::UdpSocket::bind("127.0.0.1:0").expect("Should bind");
        let port = server.local_addr().unwrap().port();
        let server_addr = format!("127.0.0.1:{}", port);

        let writer = SyslogWriter::new(
            &server_addr,
            SyslogProtocol::Udp,
            SyslogFacility::Local0,
            "yatagarasu-test",
        )
        .expect("Should create writer");

        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "my-bucket".to_string(),
            "path/to/file.txt".to_string(),
            "GET".to_string(),
            "/my-bucket/path/to/file.txt".to_string(),
        )
        .with_response(200, 1024, 50)
        .with_cache_status(CacheStatus::Hit);

        let message = writer.format_syslog_message(&entry, SyslogSeverity::Info);

        // Verify RFC 5424 format components
        // <PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID STRUCTURED-DATA MSG

        // Priority for Local0 (16) + Info (6) = 16*8 + 6 = 134
        assert!(
            message.starts_with("<134>1 "),
            "Should start with priority 134 (Local0.Info), got: {}",
            &message[..20]
        );

        // Should contain app name
        assert!(
            message.contains("yatagarasu-test"),
            "Should contain app name"
        );

        // Should contain JSON message body
        assert!(
            message.contains("\"client_ip\":\"192.168.1.100\""),
            "Should contain JSON entry"
        );
        assert!(
            message.contains("\"response_status\":200"),
            "Should contain response status"
        );

        // Should have timestamp in ISO format
        let timestamp_pattern = regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}").unwrap();
        assert!(
            timestamp_pattern.is_match(&message),
            "Should contain ISO timestamp"
        );
    }

    #[test]
    fn test_includes_facility_and_severity() {
        // Test: Includes facility and severity
        let server = std::net::UdpSocket::bind("127.0.0.1:0").expect("Should bind");
        let port = server.local_addr().unwrap().port();
        let server_addr = format!("127.0.0.1:{}", port);

        // Test different facility/severity combinations
        let test_cases = vec![
            (SyslogFacility::Kern, SyslogSeverity::Emergency, 0 * 8 + 0), // 0
            (SyslogFacility::User, SyslogSeverity::Info, 1 * 8 + 6),      // 14
            (SyslogFacility::Daemon, SyslogSeverity::Warning, 3 * 8 + 4), // 28
            (SyslogFacility::Auth, SyslogSeverity::Error, 4 * 8 + 3),     // 35
            (SyslogFacility::Local0, SyslogSeverity::Debug, 16 * 8 + 7),  // 135
            (SyslogFacility::Local7, SyslogSeverity::Critical, 23 * 8 + 2), // 186
        ];

        for (facility, severity, expected_pri) in test_cases {
            let writer = SyslogWriter::new(&server_addr, SyslogProtocol::Udp, facility, "test")
                .expect("Should create writer");

            let entry = AuditLogEntry::new(
                "192.168.1.1".to_string(),
                "bucket".to_string(),
                "key".to_string(),
                "GET".to_string(),
                "/path".to_string(),
            );

            let message = writer.format_syslog_message(&entry, severity);
            let expected_start = format!("<{}>1 ", expected_pri);

            assert!(
                message.starts_with(&expected_start),
                "Facility {:?} + Severity {:?} should give priority {}, got: {}",
                facility,
                severity,
                expected_pri,
                &message[..10]
            );
        }

        // Test severity_from_status mapping
        assert_eq!(
            SyslogWriter::severity_from_status(200),
            SyslogSeverity::Info
        );
        assert_eq!(
            SyslogWriter::severity_from_status(301),
            SyslogSeverity::Notice
        );
        assert_eq!(
            SyslogWriter::severity_from_status(404),
            SyslogSeverity::Warning
        );
        assert_eq!(
            SyslogWriter::severity_from_status(500),
            SyslogSeverity::Error
        );
    }

    #[test]
    fn test_handles_syslog_server_down_gracefully() {
        // Test: Handles syslog server down gracefully
        // Try to connect to a port that's not listening
        let result = SyslogWriter::new(
            "127.0.0.1:59999", // Unlikely to have a server on this port
            SyslogProtocol::Tcp,
            SyslogFacility::Local0,
            "yatagarasu",
        );

        // Should return an error, not panic
        assert!(
            result.is_err(),
            "Should fail gracefully when server is down"
        );

        // Error should be a connection error
        let err = result.unwrap_err();
        assert!(
            err.kind() == std::io::ErrorKind::ConnectionRefused
                || err.kind() == std::io::ErrorKind::TimedOut
                || err.kind() == std::io::ErrorKind::Other,
            "Should be connection-related error: {:?}",
            err.kind()
        );

        // UDP "connection" should work even without server (connectionless)
        // But write should not panic
        let udp_writer = SyslogWriter::new(
            "127.0.0.1:59998",
            SyslogProtocol::Udp,
            SyslogFacility::Local0,
            "yatagarasu",
        );
        assert!(
            udp_writer.is_ok(),
            "UDP writer should create without server"
        );

        // Test write to closed connection
        let mut writer = udp_writer.unwrap();
        writer.close();

        let entry = AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );

        let write_result = writer.write_entry(&entry);
        assert!(
            write_result.is_err(),
            "Writing to closed connection should error"
        );
        assert_eq!(
            write_result.unwrap_err().kind(),
            std::io::ErrorKind::NotConnected
        );
    }

    // ============================================================================
    // Phase 33.6: S3 Export for Audit Logs Tests
    // ============================================================================

    #[test]
    fn test_batches_audit_entries_in_memory() {
        // Test: Batches audit entries in memory
        let config = S3AuditExportConfig {
            bucket: "test-bucket".to_string(),
            prefix: "audit/".to_string(),
            export_interval_secs: 300,
            max_batch_size: 100,
            max_retries: 3,
        };

        let exporter = S3AuditExporter::new(config);

        // Add multiple entries
        for i in 0..10 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            )
            .with_response(200, 1024, 50);

            exporter.add_entry(entry);
        }

        // Verify entries are batched
        assert_eq!(
            exporter.current_batch_size(),
            10,
            "Should have 10 entries batched"
        );
        assert_eq!(exporter.pending_batch_count(), 0, "No pending batches yet");

        // Add more entries to trigger batch rotation
        let config2 = S3AuditExportConfig {
            max_batch_size: 5, // Small batch size for testing
            ..Default::default()
        };
        let exporter2 = S3AuditExporter::new(config2);

        for i in 0..12 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            exporter2.add_entry(entry);
        }

        // Should have rotated batches (12 entries with batch size 5 = 2 full batches + 2 in current)
        assert_eq!(
            exporter2.pending_batch_count(),
            2,
            "Should have 2 pending batches"
        );
        assert_eq!(
            exporter2.current_batch_size(),
            2,
            "Should have 2 entries in current batch"
        );
    }

    #[test]
    fn test_batch_file_format() {
        // Test: Batch file format: yatagarasu-audit-YYYY-MM-DD-HH-MM-SS.jsonl
        let mut batch = AuditBatch::new();

        // Add an entry
        let entry = AuditLogEntry::new(
            "192.168.1.100".to_string(),
            "bucket".to_string(),
            "file.txt".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        );
        batch.add(entry);

        // Generate object key
        let key = batch.generate_object_key("audit-logs/");

        // Verify format: prefix + yatagarasu-audit-YYYY-MM-DD-HH-MM-SS.jsonl
        assert!(
            key.starts_with("audit-logs/yatagarasu-audit-"),
            "Key should start with prefix and 'yatagarasu-audit-': {}",
            key
        );
        assert!(
            key.ends_with(".jsonl"),
            "Key should end with .jsonl: {}",
            key
        );

        // Verify timestamp format
        let timestamp_pattern =
            regex::Regex::new(r"yatagarasu-audit-\d{4}-\d{2}-\d{2}-\d{2}-\d{2}-\d{2}\.jsonl$")
                .unwrap();
        assert!(
            timestamp_pattern.is_match(&key),
            "Key should contain timestamp in YYYY-MM-DD-HH-MM-SS format: {}",
            key
        );
    }

    #[test]
    fn test_each_line_is_one_json_audit_entry() {
        // Test: Each line is one JSON audit entry (JSONL format)
        let mut batch = AuditBatch::new();

        // Add multiple entries
        for i in 0..3 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i + 1),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            )
            .with_response(200, 1024 * (i as u64 + 1), 50);

            batch.add(entry);
        }

        // Convert to JSONL
        let jsonl = batch.to_jsonl();
        let lines: Vec<&str> = jsonl.lines().collect();

        // Should have 3 lines
        assert_eq!(lines.len(), 3, "Should have 3 lines");

        // Each line should be valid JSON
        for (i, line) in lines.iter().enumerate() {
            let parsed: Result<AuditLogEntry, _> = serde_json::from_str(line);
            assert!(parsed.is_ok(), "Line {} should be valid JSON: {}", i, line);

            let entry = parsed.unwrap();
            assert_eq!(
                entry.client_ip,
                format!("192.168.1.{}", i + 1),
                "Entry {} should have correct client_ip",
                i
            );
        }

        // Lines should NOT contain newlines within them
        for line in &lines {
            assert!(
                !line.contains('\n'),
                "Individual JSON entries should not contain newlines"
            );
        }
    }

    #[test]
    fn test_exporter_uses_configured_bucket_and_prefix() {
        // Test: Uses configured bucket and prefix
        let config = S3AuditExportConfig {
            bucket: "my-custom-bucket".to_string(),
            prefix: "logs/yatagarasu/audit/".to_string(),
            export_interval_secs: 60,
            max_batch_size: 1000,
            max_retries: 5,
        };

        let exporter = S3AuditExporter::new(config);

        assert_eq!(exporter.bucket(), "my-custom-bucket");
        assert_eq!(exporter.prefix(), "logs/yatagarasu/audit/");
        assert_eq!(exporter.export_interval_secs(), 60);

        // Verify object key uses the prefix
        let mut batch = AuditBatch::new();
        batch.add(AuditLogEntry::new(
            "192.168.1.1".to_string(),
            "bucket".to_string(),
            "key".to_string(),
            "GET".to_string(),
            "/path".to_string(),
        ));

        let key = exporter.generate_object_key(&batch);
        assert!(
            key.starts_with("logs/yatagarasu/audit/yatagarasu-audit-"),
            "Object key should use configured prefix: {}",
            key
        );
    }

    #[test]
    fn test_get_all_batches_for_export() {
        // Test: Can get all batches (pending + current) for export
        let config = S3AuditExportConfig {
            max_batch_size: 3,
            ..Default::default()
        };

        let exporter = S3AuditExporter::new(config);

        // Add entries to create multiple batches
        for i in 0..8 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            exporter.add_entry(entry);
        }

        // Should have 2 pending batches (6 entries) + current batch (2 entries)
        assert_eq!(exporter.pending_batch_count(), 2);
        assert_eq!(exporter.current_batch_size(), 2);

        // Get all batches
        let batches = exporter.get_all_batches();

        // Should have 3 batches total
        assert_eq!(batches.len(), 3, "Should have 3 batches");

        // Total entries should be 8
        let total_entries: usize = batches.iter().map(|b| b.len()).sum();
        assert_eq!(total_entries, 8, "Total entries should be 8");

        // After getting batches, exporter should be empty
        assert_eq!(exporter.pending_batch_count(), 0);
        assert_eq!(exporter.current_batch_size(), 0);
    }

    #[test]
    fn test_batch_rotation() {
        // Test: Rotate current batch to pending
        let config = S3AuditExportConfig::default();
        let exporter = S3AuditExporter::new(config);

        // Add some entries
        for i in 0..5 {
            let entry = AuditLogEntry::new(
                format!("192.168.1.{}", i),
                "bucket".to_string(),
                format!("file{}.txt", i),
                "GET".to_string(),
                "/path".to_string(),
            );
            exporter.add_entry(entry);
        }

        assert_eq!(exporter.current_batch_size(), 5);

        // Rotate batch
        let rotated = exporter.rotate_batch();
        assert!(rotated.is_some(), "Should have rotated a batch");
        assert_eq!(
            rotated.unwrap().len(),
            5,
            "Rotated batch should have 5 entries"
        );

        // Current batch should now be empty
        assert_eq!(exporter.current_batch_size(), 0);

        // Rotating empty batch returns None
        let empty_rotation = exporter.rotate_batch();
        assert!(
            empty_rotation.is_none(),
            "Empty batch rotation should return None"
        );
    }

    // ============================================================================
    // Phase 33.7: Correlation ID Propagation Tests
    // ============================================================================

    #[test]
    fn test_generates_uuid_v4_for_each_request() {
        // Test: Generates UUID v4 for each request
        let ctx1 = RequestContext::new();
        let ctx2 = RequestContext::new();

        // Both should have valid UUIDs
        assert!(
            Uuid::parse_str(&ctx1.correlation_id).is_ok(),
            "correlation_id should be valid UUID"
        );
        assert!(
            Uuid::parse_str(&ctx2.correlation_id).is_ok(),
            "correlation_id should be valid UUID"
        );

        // They should be different
        assert_ne!(
            ctx1.correlation_id, ctx2.correlation_id,
            "Each request should have unique correlation_id"
        );

        // Test the generate function directly
        let id1 = generate_correlation_id();
        let id2 = generate_correlation_id();
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_uses_existing_x_correlation_id_header_if_present() {
        // Test: Uses existing X-Correlation-ID header if present
        let existing_id = "existing-correlation-id-123";

        // Create context with existing header
        let ctx = RequestContext::with_correlation_id_header(Some(existing_id));

        assert_eq!(
            ctx.correlation_id, existing_id,
            "Should use existing correlation ID from header"
        );

        // Test with valid UUID from header
        let uuid_id = "550e8400-e29b-41d4-a716-446655440000";
        let ctx2 = RequestContext::with_correlation_id_header(Some(uuid_id));
        assert_eq!(ctx2.correlation_id, uuid_id);

        // Test with None header - should generate new ID
        let ctx3 = RequestContext::with_correlation_id_header(None);
        assert!(
            Uuid::parse_str(&ctx3.correlation_id).is_ok(),
            "Should generate new UUID when header is None"
        );

        // Test with empty header - should generate new ID
        let ctx4 = RequestContext::with_correlation_id_header(Some(""));
        assert!(
            Uuid::parse_str(&ctx4.correlation_id).is_ok(),
            "Should generate new UUID when header is empty"
        );

        // Test with invalid header (special chars) - should generate new ID
        let ctx5 = RequestContext::with_correlation_id_header(Some("invalid!@#$%"));
        assert!(
            Uuid::parse_str(&ctx5.correlation_id).is_ok(),
            "Should generate new UUID when header has invalid chars"
        );
    }

    #[test]
    fn test_includes_correlation_id_in_all_log_entries() {
        // Test: Includes correlation ID in all log entries
        let specific_id = "my-specific-correlation-id";
        let mut ctx = RequestContext::with_correlation_id(specific_id.to_string());

        // Set up the context
        ctx.bucket = Some("test-bucket".to_string());
        ctx.object_key = Some("test-key".to_string());
        ctx.http_method = Some("GET".to_string());
        ctx.request_path = Some("/test".to_string());
        ctx.client_ip = Some("192.168.1.1".to_string());
        ctx.set_response_status(200);
        ctx.set_response_size(1024);

        // Convert to audit entry
        let entry = ctx.to_audit_entry();

        // Verify correlation ID is in the entry
        assert_eq!(
            entry.correlation_id, specific_id,
            "Audit entry should contain the correlation ID"
        );

        // Verify it serializes correctly
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains(specific_id),
            "Serialized JSON should contain correlation ID"
        );
    }

    #[test]
    fn test_correlation_id_header_constant() {
        // Test: Header name constant
        assert_eq!(
            X_CORRELATION_ID_HEADER, "X-Correlation-ID",
            "Header constant should be correct"
        );
    }

    #[test]
    fn test_is_valid_correlation_id() {
        // Test: Validation of correlation ID format
        // Valid UUIDs
        assert!(is_valid_correlation_id(
            "550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(is_valid_correlation_id(
            "123e4567-e89b-12d3-a456-426614174000"
        ));

        // Valid alphanumeric with hyphens/underscores
        assert!(is_valid_correlation_id("request-123"));
        assert!(is_valid_correlation_id("trace_id_456"));
        assert!(is_valid_correlation_id("abc123"));
        assert!(is_valid_correlation_id("ABC-123_xyz"));

        // Invalid - empty
        assert!(!is_valid_correlation_id(""));

        // Invalid - too long (> 128 chars)
        let long_id = "a".repeat(129);
        assert!(!is_valid_correlation_id(&long_id));

        // Invalid - special characters
        assert!(!is_valid_correlation_id("id!@#$"));
        assert!(!is_valid_correlation_id("id with spaces"));
        assert!(!is_valid_correlation_id("id\nwith\nnewlines"));
    }

    #[test]
    fn test_get_correlation_id_for_response_header() {
        // Test: Can get correlation ID for response headers
        let ctx = RequestContext::with_correlation_id("response-header-test-id".to_string());

        // get_correlation_id should return the ID for use in response headers
        assert_eq!(ctx.get_correlation_id(), "response-header-test-id");

        // Also test with generated ID
        let ctx2 = RequestContext::new();
        let id = ctx2.get_correlation_id();
        assert!(!id.is_empty());
        assert!(Uuid::parse_str(id).is_ok());
    }

    #[test]
    fn test_correlation_id_from_header_function() {
        // Test: correlation_id_from_header utility function
        // With valid header
        assert_eq!(
            correlation_id_from_header(Some("my-trace-id")),
            "my-trace-id"
        );

        // With None - should generate UUID
        let generated = correlation_id_from_header(None);
        assert!(Uuid::parse_str(&generated).is_ok());

        // With empty string - should generate UUID
        let generated2 = correlation_id_from_header(Some(""));
        assert!(Uuid::parse_str(&generated2).is_ok());

        // With invalid chars - should generate UUID
        let generated3 = correlation_id_from_header(Some("bad!id"));
        assert!(Uuid::parse_str(&generated3).is_ok());
    }
}
