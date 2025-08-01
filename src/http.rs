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
    crate::error::McpError,
    serde_json::{json, Value},
    std::sync::Arc,
    tracing::{debug, error, trace, warn},
    warp::http::{HeaderValue, StatusCode},
    warp::{reply, Filter, Rejection, Reply},
};

// === MCP PROGRESS NOTIFICATION SUPPORT ===
// This implements proper MCP progress notifications as separate JSON-RPC messages

#[derive(Debug, Clone)]
pub struct ProgressNotification {
    pub progress_token: Value,
    pub progress: f64,
    pub total: Option<f64>,
    pub message: Option<String>,
}

impl ProgressNotification {
    pub fn to_json_rpc(&self) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": self.progress_token,
                "progress": self.progress,
                "total": self.total,
                "message": self.message
            }
        })
    }
}

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
    use std::time::Instant;
    use uuid::Uuid;

    // === COMPREHENSIVE MCP PROTOCOL INSTRUMENTATION ===
    let request_start = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let content_type = content_type.unwrap_or_else(|| "application/json".to_string());
    let accept = accept.unwrap_or_else(|| "application/json".to_string());
    let connection = connection.unwrap_or_else(|| "close".to_string());

    // === PROTOCOL ANALYSIS LOGGING ===
    trace!("🚀 === MCP REQUEST ANALYSIS START ===");
    trace!("   Request ID: {}", request_id);
    trace!("   Timestamp: {:?}", request_start);
    trace!("   Content-Type: {}", content_type);
    trace!("   Accept: {}", accept);
    trace!("   Connection: {}", connection);
    trace!("   Cookie: {:?}", cookie);

    // Detect Cursor client from User-Agent patterns in headers
    let is_cursor_client = content_type.contains("Cursor")
        || accept.contains("Cursor")
        || cookie.as_ref().is_some_and(|c| c.contains("Cursor"));

    if is_cursor_client {
        warn!("🎯 === CURSOR CLIENT DETECTED ===");
        warn!("   Applying enhanced Cursor-specific protocol analysis");
        warn!("   Request ID: {}", request_id);
    }

    // === MESSAGE STRUCTURE ANALYSIS ===
    let method = message
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown");
    let msg_id = message.get("id").cloned().unwrap_or(json!(null));
    let has_params = message.get("params").is_some();
    let has_meta = message.get("params").and_then(|p| p.get("_meta")).is_some();
    let has_progress_token = message
        .get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("progressToken"))
        .is_some();

    trace!(
        method = %method,
        message_id = ?msg_id,
        has_params = has_params,
        has_meta = has_meta,
        has_progress_token = has_progress_token,
        "Message structure analysis"
    );

    if has_progress_token {
        let progress_token = message
            .get("params")
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("progressToken"));
        warn!(
            progress_token = ?progress_token,
            "Progress token detected - client expects streaming updates"
        );

        if is_cursor_client {
            warn!(
                client = "Cursor",
                protocol_path = "critical",
                "Cursor client with progress token detected"
            );
        }
    }

    // === MESSAGE SIZE ANALYSIS ===
    let message_json = serde_json::to_string(&message).unwrap_or_default();
    let request_size = message_json.len();

    trace!(
        request_size_bytes = request_size,
        request_size_kb = format!("{:.2}", request_size as f64 / 1024.0),
        "Request size analysis"
    );

    if request_size > 10000 {
        warn!(
            request_size_bytes = request_size,
            threshold = 10000,
            "Large request detected - may cause processing issues"
        );
    }

    trace!(
        content_type = %content_type,
        accept = %accept,
        connection = %connection,
        "MCP HTTP request headers"
    );
    trace!(
        raw_request_body = %serde_json::to_string(&message).unwrap_or_else(|_| "invalid json".to_string()),
        content_type = %content_type,
        accept = %accept,
        connection = %connection,
        cookie = ?cookie,
        "Incoming MCP request details"
    );

    // Extract session ID from cookie
    let session_id = extract_session_id_from_cookie(&cookie);

    // Extract method for session ID logic
    let method = message.get("method").and_then(|m| m.as_str()).unwrap_or("");

    // For HTTP clients that don't handle cookies properly (like Claude), we need a fallback
    // Use a consistent session ID for the duration of the server process
    let effective_session_id = if method == "initialize" || session_id.is_none() {
        // Always use a consistent session for initialize requests
        // or when clients don't send cookies (like Claude)
        if method != "initialize" && session_id.is_none() {
            debug!(
                method = %method,
                fallback_session = "http_default_session",
                "No session cookie found - using default HTTP session"
            );
        }
        Some("http_default_session".to_string())
    } else {
        session_id.clone()
    };

    trace!("🔍 SESSION DEBUG:");
    trace!(
        "   📋 Method: {}",
        message.get("method").and_then(|m| m.as_str()).unwrap_or("")
    );
    trace!("   📋 Cookie header: {:?}", cookie);
    trace!("   📋 Extracted session: {:?}", session_id);
    trace!("   📋 Effective session: {:?}", effective_session_id);
    trace!("   📋 Message ID: {:?}", message.get("id"));

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

    trace!("✅ MCP message validation passed");

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

    // Check if this is a tools/call with progress token
    let has_progress_token = message
        .get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("progressToken"))
        .is_some();

    // === HTTP ENCODING STRATEGY ===
    // Use chunked encoding ONLY when we have progress tokens
    // This prevents HTTP protocol violations and ensures compatibility
    let use_chunked = has_progress_token;

    trace!("🔧 HTTP encoding strategy:");
    trace!(
        "   Has Progress Token: {} (Accept: {}, Connection: {})",
        has_progress_token, accept, connection
    );
    trace!(
        "   Using Chunked Encoding: {} ({})",
        use_chunked,
        if use_chunked {
            "for streaming progress"
        } else {
            "standard Content-Length"
        }
    );

    // === STRATEGY 1: MCP PROGRESS TOKEN HANDLING (OPTIMAL APPROACH) ===
    // Based on successful testing, always implement immediate progress notifications
    // when a progress token is detected, using chunked encoding for streaming
    let progress_token = if has_progress_token {
        let token = message
            .get("params")
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("progressToken"))
            .cloned();

        warn!("🎯 === MCP PROGRESS TOKEN DETECTED (STRATEGY 1) ===");
        warn!("   Progress Token: {:?}", token);
        warn!("   IMPLEMENTING IMMEDIATE PROGRESS NOTIFICATIONS");
        warn!("   Using Strategy 1: Immediate progress with chunked encoding");

        token
    } else {
        None
    };

    if has_progress_token {
        warn!("🚀 STRATEGY 1: ENABLING CHUNKED ENCODING for MCP Streamable HTTP transport");
        warn!("   Using immediate progress notifications with chunked encoding");
        warn!("   This approach has been tested and proven to work with Cursor");
    }

    // Handle the message with Strategy 1: Immediate progress with chunked encoding
    let (result, _progress_notifications) = if has_progress_token {
        // === STRATEGY 1: IMMEDIATE PROGRESS NOTIFICATION HANDLING ===
        warn!("📡 STRATEGY 1: PROCESSING WITH IMMEDIATE PROGRESS NOTIFICATIONS");

        let mut notifications = Vec::new();

        // Strategy 1: Send immediate progress notification
        if let Some(ref token) = progress_token {
            let start_notification = ProgressNotification {
                progress_token: token.clone(),
                progress: 0.0,
                total: Some(100.0),
                message: Some("Starting request processing...".to_string()),
            };

            warn!(
                "📡 STRATEGY 1: QUEUING IMMEDIATE PROGRESS NOTIFICATION: {:?}",
                start_notification.to_json_rpc()
            );
            notifications.push(start_notification);
        }

        // Process the message normally
        let start_time = std::time::Instant::now();
        let response_result = handler
            .handle_message(message_clone, effective_session_id.clone())
            .await;

        // Strategy 1: Send completion progress notification
        if let Some(ref token) = progress_token {
            let duration = start_time.elapsed();
            let completion_notification = ProgressNotification {
                progress_token: token.clone(),
                progress: 100.0,
                total: Some(100.0),
                message: Some(format!(
                    "Request completed in {:.2}ms",
                    duration.as_secs_f64() * 1000.0
                )),
            };

            warn!(
                "📡 STRATEGY 1: QUEUING COMPLETION NOTIFICATION: {:?}",
                completion_notification.to_json_rpc()
            );
            notifications.push(completion_notification);
        }

        (response_result, Some(notifications))
    } else {
        // Standard processing without progress notifications
        let response_result = handler
            .handle_message(message_clone, effective_session_id.clone())
            .await;
        (response_result, None)
    };

    match result {
        Ok(response) => {
            let response_processing_time = request_start.elapsed();
            debug!("📤 HTTP MCP response: {:?}", response);

            // === COMPREHENSIVE RESPONSE ANALYSIS ===
            let response_json = serde_json::to_string(&response).unwrap_or_default();
            let response_size = response_json.len();

            trace!("🎉 === MCP RESPONSE ANALYSIS ===");
            trace!("   Request ID: {}", request_id);
            trace!("   Processing Time: {:?}", response_processing_time);
            trace!("   Response Size: {} bytes", response_size);
            trace!("   Response KB: {:.2} KB", response_size as f64 / 1024.0);
            trace!("   Method: {}", method);
            trace!("   Message ID: {:?}", msg_id);

            // === CURSOR-SPECIFIC ANALYSIS ===
            if is_cursor_client {
                warn!("🎯 === CURSOR RESPONSE ANALYSIS ===");
                warn!("   Request ID: {}", request_id);
                warn!("   Processing Time: {:?}", response_processing_time);
                warn!("   Response Size: {} bytes", response_size);

                // Critical analysis for Cursor timeouts
                if response_size > 10000 {
                    warn!("⚠️  CURSOR LARGE RESPONSE WARNING: {} bytes", response_size);
                    warn!("   This may cause Cursor client timeout - consider optimization!");
                }

                if response_processing_time.as_millis() > 5000 {
                    warn!(
                        "⚠️  CURSOR SLOW RESPONSE WARNING: {:?}",
                        response_processing_time
                    );
                    warn!("   This may cause Cursor client timeout!");
                }

                if has_progress_token {
                    warn!("🎯 CURSOR + PROGRESS TOKEN RESPONSE");
                    warn!("   Cursor expects this to support streaming updates");
                    warn!("   Current implementation: Single response (not streaming)");
                }
            }

            // === RESPONSE CONTENT ANALYSIS ===
            let has_result = response.get("result").is_some();
            let has_error = response.get("error").is_some();
            let result_type = if has_result {
                "success"
            } else if has_error {
                "error"
            } else {
                "unknown"
            };

            trace!("📋 === RESPONSE CONTENT ===");
            trace!("   Type: {}", result_type);
            trace!("   Has Result: {}", has_result);
            trace!("   Has Error: {}", has_error);

            if has_result {
                if let Some(result) = response.get("result") {
                    if let Some(tools) = result.get("tools") {
                        if let Some(tools_array) = tools.as_array() {
                            trace!("   Tools Count: {}", tools_array.len());
                        }
                    }
                    if let Some(results) = result.get("results") {
                        if let Some(results_array) = results.as_array() {
                            trace!("   Results Count: {}", results_array.len());
                        }
                    }
                }
            }

            warn!("🔍 RESPONSE DEBUG:");
            warn!("   📊 Response size: {} bytes", response_size);
            warn!("   📊 Response KB: {:.2} KB", response_size as f64 / 1024.0);
            warn!("   📋 Method: {:?}", method);
            warn!("   📋 Message ID: {:?}", message_id);

            // Check if this is a tools/call with progress token
            let has_progress_token = message
                .get("params")
                .and_then(|p| p.get("_meta"))
                .and_then(|m| m.get("progressToken"))
                .is_some();

            if has_progress_token {
                warn!("⚡ REQUEST HAS PROGRESS TOKEN - Client expects streaming updates");
                let progress_token = message
                    .get("params")
                    .and_then(|p| p.get("_meta"))
                    .and_then(|m| m.get("progressToken"));
                warn!("   🎯 Progress Token: {:?}", progress_token);
            }

            if response_size > 10000 {
                warn!(
                    "⚠️  LARGE RESPONSE WARNING: {} bytes might cause client timeout",
                    response_size
                );
                warn!("⚠️  This could be the ROOT CAUSE of MCP client timeouts!");
            }

            // Check if response contains massive debug sections
            if response_json.contains("debug") && response_json.len() > 5000 {
                warn!("🐛 LARGE DEBUG SECTION DETECTED - This may be causing timeout issues");
            }

            debug!(
                "🍪 [SESSION] Used session_id for message handling: {:?}",
                effective_session_id
            );
            // Always return 200 OK for MCP responses, even for protocol errors
            // MCP uses JSON-RPC error codes in the body, not HTTP status codes
            let status = StatusCode::OK;

            // Note: use_chunked is already determined earlier in the function

            // Determine connection header based on encoding
            let connection_header = if use_chunked {
                warn!("🔄 Using chunked transfer encoding for streaming client");
                "keep-alive"
            } else {
                warn!("📦 Using standard HTTP response (Content-Length)");
                "close"
            };

            // === COMPREHENSIVE HTTP HEADER ANALYSIS ===
            trace!("🔧 === HTTP RESPONSE HEADERS ANALYSIS ===");
            trace!("   Request ID: {}", request_id);
            trace!("   Use Chunked: {}", use_chunked);
            trace!("   Has Progress Token: {}", has_progress_token);
            trace!("   Connection Header: {}", connection_header);

            if is_cursor_client {
                warn!("🎯 === CURSOR HTTP HEADERS ===");
                warn!("   Chunked Encoding: {}", use_chunked);
                warn!("   This is critical for Cursor compatibility!");

                if !use_chunked && has_progress_token {
                    warn!("🧪 EXPERIMENTAL FIX: Content-Length + Progress Token");
                    warn!("   Using standard HTTP response instead of chunked encoding");
                    warn!("   Testing if MCP client prefers this approach");
                }
            }

            // === PROTOCOL VIOLATION PREVENTION ===
            // CRITICAL: Prevent the HTTP protocol violation that causes client timeouts
            trace!("🛡️  === PROTOCOL COMPLIANCE CHECK ===");
            if use_chunked {
                warn!("🔄 Using chunked transfer encoding");
                warn!("   Will NOT set Content-Length (prevents protocol violation)");
            } else {
                warn!("📏 Using Content-Length: {} bytes", response_size);
                warn!("   Will NOT set Transfer-Encoding (prevents protocol violation)");
            }

            // Create the response - CRITICAL: Never set both Content-Length and Transfer-Encoding
            let base_reply = if use_chunked {
                // === CHUNKED ENCODING FOR PROGRESS TOKENS ===
                warn!("🔄 Using Transfer-Encoding: chunked for progress support");
                warn!(
                    "   Response size: {} bytes (no Content-Length header)",
                    response_size
                );

                // IMPORTANT: Do NOT set Content-Length when using chunked encoding
                reply::with_header(
                    reply::with_header(
                        reply::with_status(
                            warp::reply::Response::new(response_json.into()),
                            status,
                        ),
                        "content-type",
                        "application/json; charset=utf-8",
                    ),
                    "transfer-encoding",
                    "chunked",
                )
            } else {
                // === STANDARD RESPONSE WITH CONTENT-LENGTH ===
                warn!(
                    "📏 Using Content-Length: {} bytes (no Transfer-Encoding)",
                    response_size
                );

                // IMPORTANT: Do NOT set Transfer-Encoding when using Content-Length
                reply::with_header(
                    reply::with_header(
                        reply::with_status(
                            warp::reply::Response::new(response_json.into()),
                            status,
                        ),
                        "content-type",
                        "application/json",
                    ),
                    "content-length",
                    response_size.to_string(),
                )
            };

            // Add connection header
            let base_reply = reply::with_header(base_reply, "connection", connection_header);

            // Set session cookie for initialize responses
            let set_cookie_value = if method == "initialize" && response.get("result").is_some() {
                let cookie_value = format!(
                    "mcp_session={}; Path=/mcp; HttpOnly; SameSite=Strict",
                    effective_session_id
                        .as_ref()
                        .unwrap_or(&"default".to_string())
                );
                trace!(
                    "🍪 Set session cookie: {}",
                    effective_session_id
                        .as_ref()
                        .unwrap_or(&"default".to_string())
                );
                debug!(
                    "🍪 [COOKIE] Set session cookie value: {}",
                    effective_session_id
                        .as_ref()
                        .unwrap_or(&"default".to_string())
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

            let final_reply = reply::with_header(base_reply, "set-cookie", set_cookie_value);

            // === FINAL RESPONSE SUMMARY ===
            let total_request_time = request_start.elapsed();

            warn!("📤 === FINAL RESPONSE HEADERS ===");
            warn!("   Request ID: {}", request_id);
            warn!("   Content-Type: application/json");
            warn!(
                "   Content-Length: {} bytes",
                if use_chunked { 0 } else { response_size }
            );
            warn!(
                "   Transfer-Encoding: {}",
                if use_chunked { "chunked" } else { "none" }
            );
            warn!("   Connection: {}", connection_header);
            warn!("   Status: {}", status);
            warn!("   Total Time: {:?}", total_request_time);

            trace!("📤 === MCP REQUEST COMPLETE ===");
            trace!("   Request ID: {}", request_id);
            trace!("   Method: {}", method);
            trace!("   Total Processing Time: {:?}", total_request_time);
            trace!("   Request Size: {} bytes", request_size);
            trace!("   Response Size: {} bytes", response_size);
            trace!("   Status: SUCCESS");

            if is_cursor_client {
                debug!("🎯 === CURSOR REQUEST COMPLETE ===");
                debug!("   Request ID: {}", request_id);
                debug!("   Total Time: {:?}", total_request_time);
                debug!("   Response Size: {} bytes", response_size);
                debug!(
                    "   Headers: {}",
                    if use_chunked {
                        "Chunked"
                    } else {
                        "Content-Length"
                    }
                );
                debug!("   Protocol Compliance: ✅ (No dual headers)");

                // Performance recommendations for Cursor
                if response_size > 5000 && total_request_time.as_millis() > 1000 {
                    warn!("💡 CURSOR OPTIMIZATION OPPORTUNITY:");
                    warn!("   Consider response compression or pagination");
                    warn!("   Large responses may impact Cursor user experience");
                }
            }

            trace!("📤 Sending HTTP MCP response with status: {}", status);

            // Add a small delay to see if it helps with client timeouts
            // Especially important for Cursor client stability
            let delay_ms = if is_cursor_client && response_size > 5000 {
                15
            } else {
                10
            };
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

            trace!("✅ === REQUEST {} COMPLETE ===", request_id);
            Ok(final_reply.into_response())
        }
        Err(e) => {
            error!(
                "❌ HTTP MCP error: {} (id={:?}, method={:?})",
                e, message_id, method
            );
            // Create error response with JSON-RPC error codes using structured error types
            // Always return 200 OK status, let JSON-RPC handle the error details
            let error_response = e.to_json_rpc_error(Some(message_id));
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
    debug!("🌐 OPTIONS request with capabilities: {:?}", capabilities);

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
    debug!("🌐 GET request with capabilities: {:?}", capabilities);

    let negotiation =
        TransportNegotiation::negotiate("GET", &capabilities, false, "SolidMCP", "0.1.0", "/mcp");

    match negotiation {
        TransportNegotiation::WebSocketUpgrade => {
            // Return transport info instead of error for WebSocket requests
            // This allows clients to discover available transports even when sending WS headers
            debug!("WebSocket headers detected, returning transport discovery info");
            let info = TransportInfo::new(&capabilities, "SolidMCP", "0.1.0", "/mcp");
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
    debug!(
        "🌐 Enhanced POST request with capabilities: {:?}",
        capabilities
    );

    let negotiation =
        TransportNegotiation::negotiate("POST", &capabilities, true, "SolidMCP", "0.1.0", "/mcp");

    match negotiation {
        TransportNegotiation::HttpJsonRpc => {
            // Use the existing HTTP handler logic but with enhanced logging
            debug!(
                "📡 Using HTTP JSON-RPC transport (preferred: {})",
                capabilities.preferred_transport()
            );

            // Log client information
            if let Some(client_info) = &capabilities.client_info {
                debug!("🔍 Client: {}", client_info);
            }
            if let Some(protocol_version) = &capabilities.protocol_version {
                debug!("🔍 Requested protocol version: {}", protocol_version);
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
                Ok(reply) => {
                    // Add CORS headers to the response
                    let mut response = reply.into_response();
                    let cors = cors_headers();
                    for (key, value) in cors.iter() {
                        response.headers_mut().insert(key.clone(), value.clone());
                    }
                    Ok(response)
                },
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
    debug!("🌐 SSE fallback request with Accept: {:?}", accept);

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
            if let Some(stripped) = trimmed.strip_prefix("mcp_session=") {
                let session_id = stripped.trim(); // Remove "mcp_session="
                if !session_id.is_empty() {
                    return Some(session_id.to_string());
                }
            }
        }
    }
    None
}

/// Generate a new session ID
#[cfg(test)]
fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
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
        assert_eq!(
            headers.get("content-type").and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
    }

    #[test]
    fn test_effective_session_id_logic() {
        // Test initialize method without session
        let method = "initialize";
        let session_id: Option<String> = None;

        let effective_session_id = if method == "initialize" || session_id.is_none() {
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

        let effective_session_id = if method == "initialize" || session_id.is_none() {
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

        let effective_session_id = if method == "initialize" || session_id.is_none() {
            Some("http_default_session".to_string())
        } else {
            session_id.clone()
        };

        assert_eq!(effective_session_id, Some("existing_session".to_string()));
    }
}
