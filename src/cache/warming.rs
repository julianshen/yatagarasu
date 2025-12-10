use crate::cache::{Cache, CacheEntry, CacheKey};
use crate::config::S3Config;
use crate::metrics::Metrics;
use crate::s3::S3Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmOptions {
    #[serde(default = "default_recursive")]
    pub recursive: bool,
    pub max_depth: Option<u32>,
    pub max_files: Option<u64>,
}

impl Default for PrewarmOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            max_depth: None,
            max_files: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmConfig {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

impl Default for PrewarmConfig {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            rate_limit: default_rate_limit(),
        }
    }
}

fn default_concurrency() -> usize {
    10
}

fn default_rate_limit() -> u32 {
    100
}

fn default_recursive() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrewarmTask {
    pub id: String,
    pub bucket: String,
    pub path: String,
    pub status: TaskStatus,
    pub options: PrewarmOptions,

    // Progress stats
    pub files_scanned: u64,
    pub files_cached: u64,
    pub bytes_cached: u64,

    // Timing
    pub created_at: SystemTime,
    pub start_time: Option<SystemTime>,
    pub end_time: Option<SystemTime>,

    pub error_message: Option<String>,
}

impl PrewarmTask {
    pub fn new(bucket: String, path: String, options: PrewarmOptions) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            bucket,
            path,
            status: TaskStatus::Pending,
            options,
            files_scanned: 0,
            files_cached: 0,
            bytes_cached: 0,
            created_at: SystemTime::now(),
            start_time: None,
            end_time: None,
            error_message: None,
        }
    }

    pub fn duration_seconds(&self) -> Option<u64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => end.duration_since(start).ok().map(|d| d.as_secs()),
            (Some(start), None) => SystemTime::now()
                .duration_since(start)
                .ok()
                .map(|d| d.as_secs()),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct PrewarmManager {
    tasks: Arc<Mutex<HashMap<String, PrewarmTask>>>,
    cache: Arc<std::sync::RwLock<Option<Arc<dyn Cache>>>>,
}

impl PrewarmManager {
    pub fn new(cache: Option<Arc<dyn Cache>>) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            cache: Arc::new(std::sync::RwLock::new(cache)),
        }
    }

    pub fn set_cache(&self, cache: Arc<dyn Cache>) {
        let mut w = self.cache.write().unwrap();
        *w = Some(cache);
    }

    fn cleanup_old_tasks(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        let now = SystemTime::now();
        // Remove tasks older than 1 hour (3600 seconds)
        tasks.retain(|_, task| {
            if let Some(end_time) = task.end_time {
                if let Ok(duration) = now.duration_since(end_time) {
                    return duration.as_secs() < 3600;
                }
            }
            true
        });
    }

    pub fn create_task(
        &self,
        bucket: String,
        path: String,
        options: PrewarmOptions,
        s3_config: S3Config,
    ) -> String {
        // Cleanup old tasks before creating new one
        self.cleanup_old_tasks();

        let task = PrewarmTask::new(bucket.clone(), path.clone(), options.clone());
        let task_id = task.id.clone();

        {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.insert(task_id.clone(), task);
        }

        // Spawn worker
        let task_id_clone = task_id.clone();
        let cache_opt = {
            let r = self.cache.read().unwrap();
            r.clone()
        };
        let tasks_map = self.tasks.clone();

        // Update metrics
        Metrics::global().increment_prewarm_tasks();

        tokio::spawn(async move {
            worker(
                task_id_clone,
                bucket,
                path,
                options,
                s3_config,
                cache_opt,
                tasks_map,
            )
            .await;
        });

        task_id
    }

    pub fn get_task(&self, task_id: &str) -> Option<PrewarmTask> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(task_id).cloned()
    }

    pub fn list_tasks(&self) -> Vec<PrewarmTask> {
        let tasks = self.tasks.lock().unwrap();
        tasks.values().cloned().collect()
    }

    pub fn cancel_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(task_id) {
            match task.status {
                TaskStatus::Pending | TaskStatus::Running => {
                    task.status = TaskStatus::Cancelled;
                    task.end_time = Some(SystemTime::now());
                    true
                }
                _ => false, // Already completed/failed/cancelled
            }
        } else {
            false
        }
    }
}

// Removed Default impl because new() requires arguments now

async fn worker(
    task_id: String,
    bucket: String,
    path: String,
    options: PrewarmOptions,
    s3_config: S3Config,
    cache: Option<Arc<dyn Cache>>,
    tasks: Arc<Mutex<HashMap<String, PrewarmTask>>>,
) {
    let s3_client = S3Client { config: s3_config };
    let aws_client = s3_client.create_aws_client().await;

    // Check if cache is available
    if cache.is_none() {
        let mut t = tasks.lock().unwrap();
        if let Some(task) = t.get_mut(&task_id) {
            task.status = TaskStatus::Failed;
            task.error_message = Some("Cache is not enabled/configured".to_string());
            task.end_time = Some(SystemTime::now());
        }
        Metrics::global().increment_prewarm_errors();
        return;
    }
    let cache = cache.unwrap();

    // Update status to Running
    {
        let mut t = tasks.lock().unwrap();
        if let Some(task) = t.get_mut(&task_id) {
            if task.status == TaskStatus::Cancelled {
                return;
            }
            task.status = TaskStatus::Running;
            task.start_time = Some(SystemTime::now());
        } else {
            return; // Task removed?
        }
    }

    let mut continuation_token: Option<String> = None;

    loop {
        // Check cancellation
        {
            let t = tasks.lock().unwrap();
            if let Some(task) = t.get(&task_id) {
                if task.status == TaskStatus::Cancelled {
                    return;
                }
            } else {
                return;
            }
        }

        // List objects
        let list_res = s3_client
            .list_objects(
                if path.is_empty() { None } else { Some(&path) },
                continuation_token.as_deref(),
                Some(100), // Batch size
            )
            .await;

        match list_res {
            Ok(result) => {
                for obj in result.objects {
                    // Check cancellation
                    if is_cancelled(&tasks, &task_id) {
                        return;
                    }

                    // Update scanned count
                    {
                        let mut t = tasks.lock().unwrap();
                        if let Some(task) = t.get_mut(&task_id) {
                            task.files_scanned += 1;

                            // Check max files limit
                            if let Some(limit) = options.max_files {
                                if task.files_scanned >= limit {
                                    // Mark task as completed since we reached limit
                                    task.status = TaskStatus::Completed;
                                    task.end_time = Some(SystemTime::now());
                                    return;
                                }
                            }
                        } else {
                            return; // Task gone
                        }
                    }

                    // Download and Cache
                    // Use aws_client directly for GET
                    match aws_client
                        .get_object()
                        .bucket(&bucket)
                        .key(&obj.key)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            if let Ok(body) = resp.body.collect().await {
                                let data = body.into_bytes();
                                let len = data.len();

                                let entry = CacheEntry::new(
                                    data,
                                    resp.content_type
                                        .unwrap_or("application/octet-stream".to_string()),
                                    obj.etag.clone(),
                                    Some(obj.last_modified.clone()),
                                    None, // default TTL
                                );

                                let key = CacheKey {
                                    bucket: bucket.clone(),
                                    object_key: obj.key.clone(),
                                    etag: Some(obj.etag.clone()),
                                };

                                if cache.set(key, entry).await.is_ok() {
                                    // Update stats
                                    let mut t = tasks.lock().unwrap();
                                    if let Some(task) = t.get_mut(&task_id) {
                                        task.files_cached += 1;
                                        task.bytes_cached += len as u64;

                                        Metrics::global().increment_prewarm_files(1);
                                        Metrics::global().increment_prewarm_bytes(len as u64);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to download object {}: {}", obj.key, e);
                            Metrics::global().increment_prewarm_errors();
                            // Continue to next object
                        }
                    }
                }

                if !result.is_truncated {
                    break;
                }
                continuation_token = result.next_continuation_token;
            }
            Err(e) => {
                let mut t = tasks.lock().unwrap();
                if let Some(task) = t.get_mut(&task_id) {
                    task.status = TaskStatus::Failed;
                    task.error_message = Some(e);
                    task.end_time = Some(SystemTime::now());
                }
                Metrics::global().increment_prewarm_errors();
                return;
            }
        }
    }

    // Complete
    {
        let mut t = tasks.lock().unwrap();
        if let Some(task) = t.get_mut(&task_id) {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Completed;
                task.end_time = Some(SystemTime::now());

                if let Some(duration) = task.duration_seconds() {
                    Metrics::global().record_prewarm_duration(duration);
                }
            }
        }
    }
}

fn is_cancelled(tasks: &Arc<Mutex<HashMap<String, PrewarmTask>>>, task_id: &str) -> bool {
    let t = tasks.lock().unwrap();
    if let Some(task) = t.get(task_id) {
        task.status == TaskStatus::Cancelled
    } else {
        true // If task is gone, treat as cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prewarm_config_defaults() {
        let config = PrewarmConfig::default();
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.rate_limit, 100);
    }

    #[test]
    fn test_prewarm_task_creation() {
        let options = PrewarmOptions::default();
        let task = PrewarmTask::new("bucket".to_string(), "path".to_string(), options);

        assert_eq!(task.bucket, "bucket");
        assert_eq!(task.path, "path");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.id.is_empty());
    }

    #[tokio::test]
    async fn test_prewarm_manager() {
        let manager = PrewarmManager::new(None);
        let options = PrewarmOptions::default();
        let s3_config = S3Config {
            bucket: "bucket".to_string(),
            region: "region".to_string(),
            endpoint: Some("endpoint".to_string()),
            access_key: "key".to_string(),
            secret_key: "secret".to_string(),
            ..Default::default()
        };

        let task_id =
            manager.create_task("bucket".to_string(), "path".to_string(), options, s3_config);

        let task = manager.get_task(&task_id);
        assert!(task.is_some());
        assert_eq!(task.unwrap().status, TaskStatus::Pending); // Worker might not have updated it yet

        // List tasks
        let tasks = manager.list_tasks();
        assert_eq!(tasks.len(), 1);

        // Cancel task
        let cancelled = manager.cancel_task(&task_id);
        assert!(cancelled);

        let task = manager.get_task(&task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }
}
