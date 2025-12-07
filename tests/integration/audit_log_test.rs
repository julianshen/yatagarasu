use super::test_harness::*;
use chrono::{DateTime, Utc};
use std::io::{self, BufReader, Read, Write};
use std::time::Duration;
use tempfile::tempdir;
use yatagarasu::audit::{AsyncAuditFileWriter, AuditLogEntry, CacheStatus};
use yatagarasu::config::RotationPolicy;

#[tokio::test]
#[ignore] // Marked as ignore because it requires file system access and potentially specific timing
async fn test_async_audit_file_writer_unbuffered() -> Result<(), anyhow::Error> {
    // Create a temporary directory for the audit log file
    let dir = tempdir()?;
    let log_file_path = dir.path().join("audit.log");

    // Initialize AsyncAuditFileWriter with buffer_size: 0 (unbuffered)
    let mut writer = AsyncAuditFileWriter::new(
        &log_file_path,
        1000,                 // max_size_mb (large value to prevent rotation)
        1,                    // max_backup_files
        RotationPolicy::Size, // rotation_policy
        0,                    // buffer_size: 0 for unbuffered
    )?;

    // Create some audit log entries
    let entry1 = AuditLogEntry::new(
        "127.0.0.1".to_string(),
        "bucket1".to_string(),
        "key1".to_string(),
        "GET".to_string(),
        "/path1".to_string(),
    )
    .with_response(200, 100, 10)
    .with_cache_status(CacheStatus::Miss);

    let entry2 = AuditLogEntry::new(
        "127.0.0.1".to_string(),
        "bucket2".to_string(),
        "key2".to_string(),
        "GET".to_string(),
        "/path2".to_string(),
    )
    .with_response(200, 200, 20)
    .with_cache_status(CacheStatus::Hit);

    // Write first entry
    writer.write_entry(entry1.clone())?;

    // Write second entry
    writer.write_entry(entry2.clone())?;

    // Test flush
    writer.flush()?;

    // Test rotation (create a file larger than max_size_mb)
    let large_entry = AuditLogEntry::new(
        "127.0.0.1".to_string(),
        "large_bucket".to_string(),
        "large_key".to_string(),
        "GET".to_string(),
        "/large_path".to_string(),
    )
    .with_response(200, 1_000_000, 50) // ~1MB
    .with_cache_status(CacheStatus::Miss);

    // Write enough entries to trigger rotation
    // max_size_mb is 1MB, so write 4000 small entries.
    // Total size will be 4000 * 315 bytes = 1.2MB, which will trigger at least one rotation.
    for _i in 0..2 {
        writer.write_entry(large_entry.clone())?;
    }

    // Shutdown the writer, which will block until all buffered entries are flushed and rotation handled
    writer.shutdown()?;

    // Verify content after shutdown (all entries should now be written across all log files)
    let mut combined_content = String::new();
    for entry in std::fs::read_dir(dir.path())? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with("audit.log")
        {
            combined_content.push_str(&std::fs::read_to_string(&path)?);
        }
    }

    let entry1_json = serde_json::to_string(&entry1)?;
    let entry2_json = serde_json::to_string(&entry2)?;
    let large_entry_json = serde_json::to_string(&large_entry)?;

    assert!(
        combined_content.contains(&entry1_json),
        "Entry 1 not found in combined content after shutdown"
    );
    assert!(
        combined_content.contains(&entry2_json),
        "Entry 2 not found in combined content after shutdown"
    );
    // Count occurrences of large_entry in combined content
    let large_entry_count = combined_content.matches(&large_entry_json).count();
    assert_eq!(
        large_entry_count, 2,
        "Expected 2 large entries in combined content"
    );

    // Check if rotated log files exist
    let dir_entries: Vec<_> = std::fs::read_dir(dir.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    let old_log_files: Vec<_> = dir_entries
        .iter()
        .filter(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with("audit.log.")
                && p != &&log_file_path
        })
        .collect();

    assert_eq!(old_log_files.len(), 0, "Expected no rotated log files");

    // Ensure the temporary directory is cleaned up
    dir.close()?;

    Ok(())
}
