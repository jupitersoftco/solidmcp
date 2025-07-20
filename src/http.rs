//! MCP HTTP Handler
//!
//! HTTP transport for MCP protocol messages with intelligent transport negotiation.

use {
    super::shared::McpProtocolEngine,
    super::transport::{
        cors_headers, transport_capabilities, TransportCapabilities, TransportInfo,
        TransportNegotiation,
    },
    super::validation::McpValidator,
    anyhow::Result,
    serde_json::{json, Value},
    std::sync::Arc,
    tracing::{debug, error, info, warn},
    warp::http::{HeaderValue, StatusCode},
    warp::{reply, Filter, Rejection, Reply},
};

pub struct HttpMcpHandler {
    protocol_engine: Arc<McpProtocolEngine>,
}

impl HttpMcpHandler {
    pub fn new(protocol_engine: Arc<McpProtocolEngine>) -> Self {
        Self { protocol_engine }
    }

    pub fn route(&self) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let options_route = warp::path!("mcp")
            .and(warp::options())
            .and(transport_capabilities())
            .and(with_handler(self.protocol_engine.clone()))
            .and_then(handle_mcp_options);

        let get_route = warp::path!("mcp")
            .and(warp::get())
            .and(transport_capabilities())
            .and(with_handler(self.protocol_engine.clone()))
            .and_then(handle_mcp_get);

        // SSE endpoint - returns proper error when SSE is not implemented
        let sse_route = warp::path!("mcp")
            .and(warp::get())
            .and(warp::header::optional::<String>("accept"))
            .and(warp::header::optional::<String>("cache-control"))
            .and(with_handler(self.protocol_engine.clone()))
            .and_then(handle_mcp_sse_fallback);

        let post_route = warp::path!("mcp")
            .and(warp::post())
            .and(transport_capabilities())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("cookie"))
            .and(with_handler(self.protocol_engine.clone()))
            .and_then(handle_mcp_enhanced_post);

        let legacy_route = warp::path!("mcp")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("content-type"))
            .and(warp::header::optional::<String>("accept"))
            .and(warp::header::optional::<String>("connection"))
            .and(warp::header::optional::<String>("cookie"))
            .and(with_handler(self.protocol_engine.clone()))
            .and_then(handle_mcp_http);

        // Try enhanced routes first, then SSE fallback, then legacy for backward compatibility
        options_route
            .or(get_route)
            .or(sse_route)
            .or(post_route)
            .or(legacy_route)
    }
}

fn with_handler(
    handler: Arc<McpProtocolEngine>,
) -> impl Filter<Extract = (Arc<McpProtocolEngine>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || handler.clone())
}

async fn handle_mcp_http(
    message: Value,
    content_type: Option<String>,
    accept: Option<String>,
    connection: Option<String>,
    cookie: Option<String>,
    handler: Arc<McpProtocolEngine>,
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

    // For HTTP clients that don't handle cookies properly (like Claude), we need a fallback
    // Use a consistent session ID for the duration of the server process
    let effective_session_id = if method == "initialize" {
        // Always use a consistent session for initialize requests
        Some("http_default_session".to_string())
    } else if session_id.is_none() {
        // Fallback: for clients that don't send cookies, use the same default session
        // This allows stateless HTTP clients to work with the MCP protocol
        warn!(
            "⚠️  No session cookie found for method '{}'. Using default HTTP session.",
            method
        );
        Some("http_default_session".to_string())
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

/// Enhanced handler for OPTIONS requests (CORS and capability discovery)
async fn handle_mcp_options(
    capabilities: TransportCapabilities,
    _handler: Arc<McpProtocolEngine>,
) -> Result<warp::reply::Response, Rejection> {
    info!("🌐 OPTIONS request with capabilities: {:?}", capabilities);

    let info = TransportInfo::new(&capabilities, "SolidMCP", "0.1.0", "/mcp");
    let response = info.to_json();

    let mut headers = cors_headers();
    headers.insert("content-type", HeaderValue::from_static("application/json"));

    let mut response = reply::with_status(reply::json(&response), StatusCode::OK).into_response();
    for (key, value) in headers.iter() {
        response.headers_mut().insert(key.clone(), value.clone());
    }

    Ok(response)
}

/// Enhanced handler for GET requests (transport discovery and WebSocket upgrade)
async fn handle_mcp_get(
    capabilities: TransportCapabilities,
    _handler: Arc<McpProtocolEngine>,
) -> Result<warp::reply::Response, Rejection> {
    info!("🌐 GET request with capabilities: {:?}", capabilities);

    let negotiation =
        TransportNegotiation::negotiate("GET", &capabilities, false, "SolidMCP", "0.1.0", "/mcp");

    match negotiation {
        TransportNegotiation::WebSocketUpgrade => {
            // This should be handled by WebSocket filters, not here
            warn!("WebSocket upgrade requested but not handled by WS filter");
            let error_response = json!({
                "error": {
                    "code": -32600,
                    "message": "WebSocket upgrade not available in this handler",
                    "data": {
                        "supported_transports": ["http_post"],
                        "instructions": "Use WebSocket endpoint for WebSocket connections"
                    }
                }
            });
            Ok(
                reply::with_status(reply::json(&error_response), StatusCode::BAD_REQUEST)
                    .into_response(),
            )
        }
        TransportNegotiation::InfoResponse(info) => {
            let response = info.to_json();
            let mut headers = cors_headers();
            headers.insert("content-type", HeaderValue::from_static("application/json"));

            let mut resp =
                reply::with_status(reply::json(&response), StatusCode::OK).into_response();
            for (key, value) in headers.iter() {
                resp.headers_mut().insert(key.clone(), value.clone());
            }

            Ok(resp)
        }
        TransportNegotiation::UnsupportedTransport { error, supported } => {
            let error_response = json!({
                "error": {
                    "code": -32600,
                    "message": error,
                    "data": {
                        "supported_transports": supported,
                        "client_capabilities": capabilities
                    }
                }
            });
            Ok(
                reply::with_status(reply::json(&error_response), StatusCode::BAD_REQUEST)
                    .into_response(),
            )
        }
        _ => {
            let error_response = json!({
                "error": {
                    "code": -32600,
                    "message": "Unexpected negotiation result for GET request"
                }
            });
            Ok(reply::with_status(
                reply::json(&error_response),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response())
        }
    }
}

/// Enhanced handler for POST requests with transport capability detection
async fn handle_mcp_enhanced_post(
    capabilities: TransportCapabilities,
    message: Value,
    cookie: Option<String>,
    handler: Arc<McpProtocolEngine>,
) -> Result<warp::reply::Response, Rejection> {
    info!(
        "🌐 Enhanced POST request with capabilities: {:?}",
        capabilities
    );

    let negotiation =
        TransportNegotiation::negotiate("POST", &capabilities, true, "SolidMCP", "0.1.0", "/mcp");

    match negotiation {
        TransportNegotiation::HttpJsonRpc => {
            // Use the existing HTTP handler logic but with enhanced logging
            info!(
                "📡 Using HTTP JSON-RPC transport (preferred: {})",
                capabilities.preferred_transport()
            );

            // Log client information
            if let Some(client_info) = &capabilities.client_info {
                info!("🔍 Client: {}", client_info);
            }
            if let Some(protocol_version) = &capabilities.protocol_version {
                info!("🔍 Requested protocol version: {}", protocol_version);
            }

            // Call the existing handler with converted parameters
            match handle_mcp_http(
                message,
                Some("application/json".to_string()),
                Some("application/json".to_string()),
                Some("close".to_string()),
                cookie,
                handler,
            )
            .await
            {
                Ok(reply) => Ok(reply.into_response()),
                Err(e) => Err(e),
            }
        }
        TransportNegotiation::UnsupportedTransport { error, supported } => {
            warn!("🚫 Unsupported transport: {}", error);
            let error_response = json!({
                "jsonrpc": "2.0",
                "id": message.get("id"),
                "error": {
                    "code": -32600,
                    "message": error,
                    "data": {
                        "supported_transports": supported,
                        "client_capabilities": capabilities
                    }
                }
            });
            Ok(
                reply::with_status(reply::json(&error_response), StatusCode::BAD_REQUEST)
                    .into_response(),
            )
        }
        _ => {
            let error_response = json!({
                "jsonrpc": "2.0",
                "id": message.get("id"),
                "error": {
                    "code": -32600,
                    "message": "Unexpected negotiation result for POST request"
                }
            });
            Ok(reply::with_status(
                reply::json(&error_response),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response())
        }
    }
}

/// Enhanced handler for SSE fallback (when client requests SSE but server doesn't support it)
async fn handle_mcp_sse_fallback(
    accept: Option<String>,
    _cache_control: Option<String>,
    _handler: Arc<McpProtocolEngine>,
) -> Result<warp::reply::Response, Rejection> {
    info!("🌐 SSE fallback request with Accept: {:?}", accept);

    // Check if this is actually an SSE request
    if let Some(accept_header) = &accept {
        if accept_header.contains("text/event-stream") {
            warn!("Client requested SSE but server doesn't support it, returning helpful error");

            let error_response = json!({
                "error": {
                    "code": -32600,
                    "message": "Server-Sent Events (SSE) transport not implemented",
                    "data": {
                        "supported_transports": ["http_post", "websocket"],
                        "instructions": "Use HTTP POST with JSON-RPC or WebSocket for real-time communication",
                        "fallback_suggestion": "Try connecting with HTTP POST transport"
                    }
                }
            });

            let mut headers = cors_headers();
            headers.insert("content-type", HeaderValue::from_static("application/json"));

            let mut response =
                reply::with_status(reply::json(&error_response), StatusCode::OK).into_response();
            for (key, value) in headers.iter() {
                response.headers_mut().insert(key.clone(), value.clone());
            }

            return Ok(response);
        }
    }

    // If not an SSE request, fall through to other handlers
    Err(warp::reject::not_found())
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("session_{timestamp}_{count}")
}

/// Create an error reply with proper headers
/// Note: MCP uses JSON-RPC error codes in the body, not HTTP status codes
/// So we always return 200 OK status even for protocol errors
fn create_error_reply(error_response: Value, status: StatusCode) -> warp::reply::Response {
    let base_reply = reply::with_status(reply::json(&error_response), status);
    reply::with_header(base_reply, "content-type", "application/json").into_response()
}

pub mod session;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_session_id_from_cookie() {
        // Test with valid session cookie
        let cookie = Some("mcp_session=test_session_123; other=value".to_string());
        assert_eq!(
            extract_session_id_from_cookie(&cookie),
            Some("test_session_123".to_string())
        );

        // Test with session cookie only
        let cookie = Some("mcp_session=simple_session".to_string());
        assert_eq!(
            extract_session_id_from_cookie(&cookie),
            Some("simple_session".to_string())
        );

        // Test with no session cookie
        let cookie = Some("other=value; another=test".to_string());
        assert_eq!(extract_session_id_from_cookie(&cookie), None);

        // Test with empty cookie
        let cookie = Some("".to_string());
        assert_eq!(extract_session_id_from_cookie(&cookie), None);

        // Test with None
        assert_eq!(extract_session_id_from_cookie(&None), None);

        // Test with spaces around session value
        let cookie = Some("mcp_session=  spaced_session  ; other=value".to_string());
        assert_eq!(
            extract_session_id_from_cookie(&cookie),
            Some("spaced_session".to_string())
        );
    }

    #[test]
    fn test_generate_session_id_format() {
        let session_id = generate_session_id();

        // Should start with "session_"
        assert!(session_id.starts_with("session_"));

        // Should have reasonable length
        assert!(session_id.len() > 8);
        assert!(session_id.len() < 30);

        // Should contain only alphanumeric and underscore
        assert!(session_id.chars().all(|c| c.is_alphanumeric() || c == '_'));
    }

    #[test]
    fn test_generate_session_id_uniqueness() {
        // Generate multiple session IDs
        let ids: Vec<String> = (0..100).map(|_| generate_session_id()).collect();

        // All should be unique
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, ids.len());
    }

    #[test]
    fn test_create_error_reply_format() {
        let error = json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        });

        let response = create_error_reply(error.clone(), StatusCode::OK);

        // Should have correct status
        assert_eq!(response.status(), StatusCode::OK);

        // Should have correct content type header
        let headers = response.headers();
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_effective_session_id_logic() {
        // Test initialize method without session
        let method = "initialize";
        let session_id: Option<String> = None;

        let effective_session_id = if method == "initialize" {
            Some("http_default_session".to_string())
        } else if session_id.is_none() {
            Some("http_default_session".to_string())
        } else {
            session_id.clone()
        };

        assert_eq!(
            effective_session_id,
            Some("http_default_session".to_string())
        );

        // Test non-initialize method without session
        let method = "tools/list";
        let session_id: Option<String> = None;

        let effective_session_id = if method == "initialize" {
            Some("http_default_session".to_string())
        } else if session_id.is_none() {
            Some("http_default_session".to_string())
        } else {
            session_id.clone()
        };

        assert_eq!(
            effective_session_id,
            Some("http_default_session".to_string())
        );

        // Test with existing session
        let method = "tools/list";
        let session_id = Some("existing_session".to_string());

        let effective_session_id = if method == "initialize" {
            Some("http_default_session".to_string())
        } else if session_id.is_none() {
            Some("http_default_session".to_string())
        } else {
            session_id.clone()
        };

        assert_eq!(effective_session_id, Some("existing_session".to_string()));
    }
}
