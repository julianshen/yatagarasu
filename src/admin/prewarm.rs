use crate::cache::warming::{PrewarmManager, PrewarmOptions};
use crate::config::Config;
use pingora_http::ResponseHeader;
use pingora_proxy::Session;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct CreateTaskRequest {
    bucket: String,
    path: String,
    #[serde(flatten)]
    options: PrewarmOptions,
}

/// Handle requests to /admin/cache/prewarm/*
pub async fn handle_request(
    session: &mut Session,
    path: &str,
    method: &str,
    manager: &Arc<PrewarmManager>,
    config: &Config,
) -> bool {
    // POST /admin/cache/prewarm - Create task
    if path == "/admin/cache/prewarm" && method == "POST" {
        // Read body
        let body_bytes = match session.read_request_body().await {
            Ok(Some(b)) => b,
            Ok(None) => {
                return send_json_response(
                    session,
                    400,
                    serde_json::json!({"error": "Missing request body"}),
                )
                .await
            }
            Err(e) => {
                return send_json_response(
                    session,
                    500,
                    serde_json::json!({"error": e.to_string()}),
                )
                .await
            }
        };

        let req: CreateTaskRequest = match serde_json::from_slice(&body_bytes) {
            Ok(r) => r,
            Err(e) => {
                return send_json_response(
                    session,
                    400,
                    serde_json::json!({"error": "Invalid JSON", "details": e.to_string()}),
                )
                .await
            }
        };

        // Validate inputs
        if req.bucket.is_empty() {
            return send_json_response(
                session,
                400,
                serde_json::json!({"error": "bucket is required"}),
            )
            .await;
        }

        let task_id =
            if let Some(bucket_config) = config.buckets.iter().find(|b| b.name == req.bucket) {
                manager.create_task(req.bucket, req.path, req.options, bucket_config.s3.clone())
            } else {
                return send_json_response(
                    session,
                    404,
                    serde_json::json!({"error": format!("Bucket '{}' not found", req.bucket)}),
                )
                .await;
            };

        return send_json_response(
            session,
            201,
            serde_json::json!({
                "status": "success",
                "task_id": task_id,
                "message": "Prewarm task created"
            }),
        )
        .await;
    }

    // GET /admin/cache/prewarm/tasks - List tasks
    if path == "/admin/cache/prewarm/tasks" && method == "GET" {
        let tasks = manager.list_tasks();
        return send_json_response(session, 200, serde_json::json!({"tasks": tasks})).await;
    }

    // GET /admin/cache/prewarm/status/{id} - Get task status
    if path.starts_with("/admin/cache/prewarm/status/") && method == "GET" {
        let task_id = path.strip_prefix("/admin/cache/prewarm/status/").unwrap();
        if let Some(task) = manager.get_task(task_id) {
            return send_json_response(session, 200, serde_json::json!(task)).await;
        } else {
            return send_json_response(
                session,
                404,
                serde_json::json!({"error": "Task not found"}),
            )
            .await;
        }
    }

    // DELETE /admin/cache/prewarm/{id} - Cancel task
    // Need to handle just /{id} so we check if it starts with base path
    if path.starts_with("/admin/cache/prewarm/") && method == "DELETE" {
        let task_id = path.strip_prefix("/admin/cache/prewarm/").unwrap();
        if manager.cancel_task(task_id) {
            return send_json_response(session, 200, serde_json::json!({"status": "cancelled"}))
                .await;
        } else {
            if manager.get_task(task_id).is_none() {
                return send_json_response(
                    session,
                    404,
                    serde_json::json!({"error": "Task not found"}),
                )
                .await;
            }
            return send_json_response(session, 409, serde_json::json!({"error": "Task cannot be cancelled (already completed or failed)"})).await;
        }
    }

    // Unhandled path
    let _ = send_json_response(
        session,
        404,
        serde_json::json!({"error": "Endpoint not found"}),
    )
    .await;
    true
}

async fn send_json_response(session: &mut Session, status: u16, body: serde_json::Value) -> bool {
    let body_str = body.to_string();
    if let Ok(mut header) = ResponseHeader::build(status, None) {
        let _ = header.insert_header("Content-Type", "application/json");
        let _ = header.insert_header("Content-Length", body_str.len().to_string());

        let _ = session.write_response_header(Box::new(header), false).await;
        let _ = session
            .write_response_body(Some(body_str.into()), true)
            .await;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task_request_deserialization() {
        let json = r#"{"bucket": "test-bucket", "path": "test/path", "recursive": true}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        
        assert_eq!(req.bucket, "test-bucket");
        assert_eq!(req.path, "test/path");
        assert_eq!(req.options.recursive, true);
    }
}
