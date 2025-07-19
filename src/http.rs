//! MCP HTTP Handler
//!
//! HTTP transport for MCP protocol messages.

use {
    super::shared::SharedMcpHandler,
    super::validation::McpValidator,
    anyhow::Result,
    serde_json::{json, Value},
    std::sync::Arc,
    tracing::{debug, error, info},
    warp::http::StatusCode,
    warp::{reply, Filter, Rejection, Reply},
};

pub struct HttpMcpHandler {
    shared_handler: Arc<SharedMcpHandler>,
}

impl HttpMcpHandler {
    pub fn new(shared_handler: Arc<SharedMcpHandler>) -> Self {
        Self { shared_handler }
    }

    pub fn route(&self) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path!("mcp")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("content-type"))
            .and(warp::header::optional::<String>("accept"))
            .and(warp::header::optional::<String>("connection"))
            .and(warp::header::optional::<String>("cookie"))
            .and(with_handler(self.shared_handler.clone()))
            .and_then(handle_mcp_http)
    }
}

fn with_handler(
    handler: Arc<SharedMcpHandler>,
) -> impl Filter<Extract = (Arc<SharedMcpHandler>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || handler.clone())
}

async fn handle_mcp_http(
    message: Value,
    content_type: Option<String>,
    accept: Option<String>,
    connection: Option<String>,
    cookie: Option<String>,
    handler: Arc<SharedMcpHandler>,
) -> Result<impl Reply, Rejection> {
    let content_type = content_type.unwrap_or_else(|| "application/json".to_string());
    let accept = accept.unwrap_or_else(|| "application/json".to_string());
    let connection = connection.unwrap_or_else(|| "close".to_string());

    info!(
        "📥 MCP HTTP request - Content-Type: {}, Accept: {}, Connection: {}",
        content_type, accept, connection
    );
    info!("📥 INCOMING MCP REQUEST:");
    info!(
        "   📋 Raw request body: {}",
        serde_json::to_string(&message).unwrap_or_else(|_| "invalid json".to_string())
    );
    info!(
        "   📋 Raw request body (hex): {:?}",
        serde_json::to_string(&message)
            .unwrap_or_else(|_| "invalid json".to_string())
            .as_bytes()
    );
    info!("   📋 Content-Type: {}", content_type);
    info!("   📋 Accept: {}", accept);
    info!("   📋 Connection: {}", connection);
    info!("   📋 Cookie: {:?}", cookie);

    // Extract session ID from cookie
    let session_id = extract_session_id_from_cookie(&cookie);

    // Extract method for session ID logic
    let method = message.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // For initialize requests without a session cookie, generate a new session ID
    let effective_session_id = if method == "initialize" && session_id.is_none() {
        Some(generate_session_id())
    } else {
        session_id.clone()
    };

    info!("🔍 SESSION DEBUG:");
    info!(
        "   📋 Method: {}",
        message.get("method").and_then(|m| m.as_str()).unwrap_or("")
    );
    info!("   📋 Cookie header: {:?}", cookie);
    info!("   📋 Extracted session: {:?}", session_id);
    info!("   📋 Effective session: {:?}", effective_session_id);
    info!("   📋 Message ID: {:?}", message.get("id"));

    // Validate the message
    if let Err(e) = McpValidator::validate_message(&message) {
        error!("❌ MCP message validation failed: {:?}", e);
        let error_response = json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32600,
                "message": format!("Invalid request: {:?}", e)
            }
        });
        return Ok(create_error_reply(error_response, StatusCode::BAD_REQUEST));
    }

    info!("✅ MCP message validation passed");

    // Check content type
    if !content_type.contains("application/json") {
        error!("❌ Invalid Content-Type: {}", content_type);
        let error_response = json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32600,
                "message": format!("Invalid Content-Type: {}", content_type)
            }
        });
        debug!(
            "📤 Sending error response for invalid content-type: {:?}",
            error_response
        );
        return Ok(create_error_reply(error_response, StatusCode::BAD_REQUEST));
    }

    // Extract message ID before moving the message
    let message_id = message.get("id").unwrap_or(&json!(null)).clone();
    debug!(
        "📥 Parsed MCP message: id={:?}, method={:?}",
        message_id, method
    );
    let message_clone = message.clone();

    // Extract session ID from cookie
    let session_id = extract_session_id_from_cookie(&cookie);
    debug!("🍪 Session ID from cookie: {:?}", session_id);
    debug!("🍪 Effective session ID: {:?}", effective_session_id);

    // Check if client supports streaming (Cursor typically sends this)
    let supports_streaming = accept.contains("text/event-stream")
        || connection.to_lowercase().contains("keep-alive")
        || connection.to_lowercase().contains("upgrade");

    info!(
        "🔍 Client streaming support: {} (Accept: {}, Connection: {})",
        supports_streaming, accept, connection
    );

    // Handle the message using shared logic with session management
    match handler
        .handle_message(message_clone, effective_session_id.clone())
        .await
    {
        Ok(response) => {
            debug!("📤 HTTP MCP response: {:?}", response);
            debug!(
                "🍪 [SESSION] Used session_id for message handling: {:?}",
                effective_session_id
            );
            // Always return 200 OK for MCP responses, even for protocol errors
            // MCP uses JSON-RPC error codes in the body, not HTTP status codes
            let status = StatusCode::OK;

            // Create the response with headers
            let base_reply = reply::with_status(reply::json(&response), status);
            let (transfer_encoding, connection) = if supports_streaming {
                ("chunked", "keep-alive")
            } else {
                ("", "")
            };

            // Create the response with headers
            let base_reply = reply::with_header(
                reply::with_header(
                    reply::with_header(base_reply, "content-type", "application/json"),
                    "transfer-encoding",
                    transfer_encoding,
                ),
                "connection",
                connection,
            );

            // Set session cookie for initialize responses
            let set_cookie_value = if method == "initialize" && response.get("result").is_some() {
                let cookie_value = format!(
                    "mcp_session={}; Path=/mcp; HttpOnly; SameSite=Strict",
                    effective_session_id.as_ref().unwrap()
                );
                info!(
                    "🍪 Set session cookie: {}",
                    effective_session_id.as_ref().unwrap()
                );
                debug!(
                    "🍪 [COOKIE] Set session cookie value: {}",
                    effective_session_id.as_ref().unwrap()
                );
                debug!(
                    "🍪 [COOKIE] Incoming session_id from cookie: {:?}",
                    session_id
                );
                debug!(
                    "🍪 [COOKIE] Effective session_id: {:?}",
                    effective_session_id
                );
                cookie_value
            } else {
                "".to_string()
            };

            let reply = reply::with_header(base_reply, "set-cookie", set_cookie_value);

            info!("📤 Sending HTTP MCP response with status: {}", status);
            Ok(reply.into_response())
        }
        Err(e) => {
            error!(
                "❌ HTTP MCP error: {} (id={:?}, method={:?})",
                e, message_id, method
            );
            // Create error response with JSON-RPC error codes
            // Always return 200 OK status, let JSON-RPC handle the error details
            let error_code = match e.to_string() {
                s if s.contains("Not initialized") => -32002,
                s if s.contains("Unknown method") || s.contains("Method not found") => -32601,
                s if s.contains("Invalid params") => -32602,
                _ => -32603,
            };
            let error_response = json!({
                "jsonrpc": "2.0",
                "id": message_id,
                "error": {
                    "code": error_code,
                    "message": format!("{}", e)
                }
            });
            debug!("📤 Sending error response: {:?}", error_response);
            Ok(create_error_reply(error_response, StatusCode::OK))
        }
    }
}

/// Extract session ID from cookie header
fn extract_session_id_from_cookie(cookie: &Option<String>) -> Option<String> {
    if let Some(cookie_str) = cookie {
        for cookie_pair in cookie_str.split(';') {
            let trimmed = cookie_pair.trim();
            if trimmed.starts_with("mcp_session=") {
                let session_id = trimmed[12..].trim(); // Remove "mcp_session="
                if !session_id.is_empty() {
                    return Some(session_id.to_string());
                }
            }
        }
    }
    None
}

/// Generate a new session ID
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("session_{timestamp}")
}

/// Create an error reply with proper headers
/// Note: MCP uses JSON-RPC error codes in the body, not HTTP status codes
/// So we always return 200 OK status even for protocol errors
fn create_error_reply(error_response: Value, status: StatusCode) -> warp::reply::Response {
    let base_reply = reply::with_status(reply::json(&error_response), status);
    reply::with_header(base_reply, "content-type", "application/json").into_response()
}

pub mod session;
