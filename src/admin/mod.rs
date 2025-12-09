use crate::auth::{authenticate_request, verify_admin_claims};
use crate::cache::warming::PrewarmManager;
use crate::config::Config;
use crate::metrics::Metrics;
use pingora_http::ResponseHeader;
use pingora_proxy::Session;
use std::collections::HashMap;
use std::sync::Arc;

pub mod prewarm;

/// Check if the path is handled by the admin module
pub fn is_handled_path(path: &str) -> bool {
    path.starts_with("/admin/cache/prewarm")
}

/// Handle requests to the /admin API tree
/// Returns true if the request was handled, false otherwise
#[allow(clippy::too_many_arguments)]
pub async fn handle_request(
    session: &mut Session,
    path: &str,
    method: &str,
    headers: &HashMap<String, String>,
    query_params: &HashMap<String, String>,
    config: &Config,
    metrics: &Arc<Metrics>,
    prewarm_manager: &Arc<PrewarmManager>,
) -> bool {
    // 1. Authentication & Authorization
    // All admin endpoints require authentication and admin claims
    if let Some(jwt_config) = &config.jwt {
        if jwt_config.enabled {
            match authenticate_request(headers, query_params, jwt_config) {
                Ok(claims) => {
                    // Check admin claims
                    if !verify_admin_claims(&claims, &jwt_config.admin_claims) {
                        tracing::warn!(
                            path = %path,
                            "Admin access denied: insufficient privileges"
                        );
                        let _ = send_json_response(
                            session,
                            403,
                            serde_json::json!({
                                "status": "error",
                                "message": "Admin access denied: insufficient privileges"
                            }),
                        )
                        .await;
                        metrics.increment_status_count(403);
                        return true;
                    }
                    // Auth success, proceed to routing
                    tracing::debug!("Admin request authenticated successfully");
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path,
                        error = %e,
                        "Admin authentication failed"
                    );
                    let _ = send_json_response(
                        session,
                        401,
                        serde_json::json!({
                            "status": "error",
                            "message": format!("Authentication required: {}", e)
                        }),
                    )
                    .await;
                    metrics.increment_status_count(401);
                    return true;
                }
            }
        }
    }

    // 2. Routing
    if path.starts_with("/admin/cache/prewarm") {
        return prewarm::handle_request(session, path, method, prewarm_manager, config).await;
    }

    // Return false for unhandled admin paths (to allow legacy handlers in proxy/mod.rs to work)
    // Note: Legacy handlers (reload, cache/purge) perform their own auth checking.
    // Ideally we should move them here in future refactoring.
    false
}

/// Helper to send JSON response
async fn send_json_response(
    session: &mut Session,
    status: u16,
    body: serde_json::Value,
) -> pingora_core::Result<()> {
    let body_str = body.to_string();
    let mut header = ResponseHeader::build(status, None)?;
    header.insert_header("Content-Type", "application/json")?;
    header.insert_header("Content-Length", body_str.len().to_string())?;

    session
        .write_response_header(Box::new(header), false)
        .await?;
    session
        .write_response_body(Some(body_str.into()), true)
        .await?;
    Ok(())
}
