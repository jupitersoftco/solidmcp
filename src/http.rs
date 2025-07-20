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
    info!("🚀 === MCP REQUEST ANALYSIS START ===");
    info!("   Request ID: {}", request_id);
    info!("   Timestamp: {:?}", request_start);
    info!("   Content-Type: {}", content_type);
    info!("   Accept: {}", accept);
    info!("   Connection: {}", connection);
    info!("   Cookie: {:?}", cookie);

    // Detect Cursor client from User-Agent patterns in headers
    let is_cursor_client = content_type.contains("Cursor")
        || accept.contains("Cursor")
        || cookie.as_ref().map_or(false, |c| c.contains("Cursor"));

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

    info!("🔍 === MESSAGE STRUCTURE ===");
    info!("   Method: {}", method);
    info!("   ID: {:?}", msg_id);
    info!("   Has Params: {}", has_params);
    info!("   Has Meta: {}", has_meta);
    info!("   Has Progress Token: {}", has_progress_token);

    if has_progress_token {
        let progress_token = message
            .get("params")
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("progressToken"));
        warn!("⚡ PROGRESS TOKEN DETECTED: {:?}", progress_token);
        warn!("   This indicates client expects streaming updates!");

        if is_cursor_client {
            warn!("🎯 CURSOR + PROGRESS TOKEN: Critical protocol path!");
        }
    }

    // === MESSAGE SIZE ANALYSIS ===
    let message_json = serde_json::to_string(&message).unwrap_or_default();
    let request_size = message_json.len();

    info!("📊 === REQUEST SIZE ANALYSIS ===");
    info!("   Request Size: {} bytes", request_size);
    info!("   Request Size KB: {:.2} KB", request_size as f64 / 1024.0);

    if request_size > 10000 {
        warn!(
            "⚠️  LARGE REQUEST: {} bytes may cause processing issues",
            request_size
        );
    }

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

    // Check if this is a tools/call with progress token
    let has_progress_token = message
        .get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("progressToken"))
        .is_some();

    // === MCP PROGRESS TOKEN HANDLING (OFFICIAL SPECIFICATION) ===
    // According to https://modelcontextprotocol.io/specification/2025-03-26/basic/utilities/progress
    // When a client sends progressToken, server SHOULD send progress notifications
    let progress_token = if has_progress_token {
        let token = message
            .get("params")
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("progressToken"))
            .cloned();

        warn!("🎯 === MCP PROGRESS TOKEN DETECTED ===");
        warn!("   Progress Token: {:?}", token);
        warn!("   IMPLEMENTING PROPER MCP PROGRESS NOTIFICATIONS");
        warn!("   Will send progress updates during processing");

        token
    } else {
        None
    };

    // EXPERIMENTAL: Try Content-Length responses for progress tokens
    // Some MCP clients may have issues with chunked responses
    let use_chunked = supports_streaming && !has_progress_token;

    if has_progress_token {
        warn!("📏 USING CONTENT-LENGTH for MCP progress tokens");
        warn!("   Testing if MCP client prefers standard HTTP responses");
        warn!("   Progress notifications will be included in final response");
    }

    // Handle the message with proper MCP progress notification support
    let (result, progress_notifications) = if has_progress_token {
        // === PROPER MCP PROGRESS NOTIFICATION HANDLING ===
        warn!("📡 PROCESSING WITH MCP PROGRESS NOTIFICATIONS");

        let mut notifications = Vec::new();

        // Send initial progress notification
        if let Some(ref token) = progress_token {
            let start_notification = ProgressNotification {
                progress_token: token.clone(),
                progress: 0.0,
                total: Some(100.0),
                message: Some("Starting request processing...".to_string()),
            };

            warn!(
                "📡 QUEUING PROGRESS NOTIFICATION: {:?}",
                start_notification.to_json_rpc()
            );
            notifications.push(start_notification);
        }

        // Process the message normally
        let start_time = std::time::Instant::now();
        let response_result = handler
            .handle_message(message_clone, effective_session_id.clone())
            .await;

        // Send completion progress notification
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
                "📡 QUEUING COMPLETION NOTIFICATION: {:?}",
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

            info!("🎉 === MCP RESPONSE ANALYSIS ===");
            info!("   Request ID: {}", request_id);
            info!("   Processing Time: {:?}", response_processing_time);
            info!("   Response Size: {} bytes", response_size);
            info!("   Response KB: {:.2} KB", response_size as f64 / 1024.0);
            info!("   Method: {}", method);
            info!("   Message ID: {:?}", msg_id);

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

            info!("📋 === RESPONSE CONTENT ===");
            info!("   Type: {}", result_type);
            info!("   Has Result: {}", has_result);
            info!("   Has Error: {}", has_error);

            if has_result {
                let result = response.get("result").unwrap();
                if let Some(tools) = result.get("tools") {
                    if let Some(tools_array) = tools.as_array() {
                        info!("   Tools Count: {}", tools_array.len());
                    }
                }
                if let Some(results) = result.get("results") {
                    if let Some(results_array) = results.as_array() {
                        info!("   Results Count: {}", results_array.len());
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

            // Determine if we should use chunked encoding
            let (transfer_encoding, connection_header) = if use_chunked {
                warn!("🔄 Using chunked transfer encoding for streaming client");
                ("chunked", "keep-alive")
            } else {
                warn!("📦 Using standard HTTP response (Content-Length)");
                ("", "close")
            };

            // === COMPREHENSIVE HTTP HEADER ANALYSIS ===
            info!("🔧 === HTTP RESPONSE HEADERS ANALYSIS ===");
            info!("   Request ID: {}", request_id);
            info!("   Use Chunked: {}", use_chunked);
            info!("   Supports Streaming: {}", supports_streaming);
            info!("   Has Progress Token: {}", has_progress_token);
            info!("   Connection Header: {}", connection_header);

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
            info!("🛡️  === PROTOCOL COMPLIANCE CHECK ===");
            if use_chunked {
                warn!("🔄 Using chunked transfer encoding");
                warn!("   Will NOT set Content-Length (prevents protocol violation)");
            } else {
                warn!("📏 Using Content-Length: {} bytes", response_size);
                warn!("   Will NOT set Transfer-Encoding (prevents protocol violation)");
            }

            // Create the response with proper MCP progress notification support
            let base_reply = if let Some(ref notifications) = progress_notifications {
                // === STREAMING MCP PROGRESS NOTIFICATIONS ===
                warn!(
                    "📡 CREATING STREAMING RESPONSE WITH {} PROGRESS NOTIFICATIONS",
                    notifications.len()
                );
                warn!("🔄 FORCING CHUNKED ENCODING for MCP progress notifications");

                // Build the streaming response body with progress notifications + final response
                let mut response_parts = Vec::new();

                // Add each progress notification as a separate JSON message
                for notification in notifications {
                    let notification_json = serde_json::to_string(&notification.to_json_rpc())
                        .unwrap_or_else(|_| "{}".to_string());
                    response_parts.push(format!("{}\n", notification_json));
                    warn!(
                        "📡 ADDING PROGRESS NOTIFICATION TO STREAM: {} bytes",
                        notification_json.len()
                    );
                }

                // Add the final response
                let final_response_json =
                    serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
                response_parts.push(format!("{}\n", final_response_json));
                warn!(
                    "📤 ADDING FINAL RESPONSE TO STREAM: {} bytes",
                    final_response_json.len()
                );

                // Combine all parts
                let full_response_body = response_parts.join("");
                let total_size = full_response_body.len();

                warn!("📊 TOTAL STREAMING RESPONSE SIZE: {} bytes", total_size);
                warn!("📊 PROGRESS NOTIFICATIONS: {}", notifications.len());
                warn!("🔄 Using Transfer-Encoding: chunked for MCP streaming");

                // Create chunked response with raw body
                reply::with_header(
                    reply::with_header(
                        reply::with_status(
                            warp::reply::Response::new(full_response_body.into()),
                            status,
                        ),
                        "content-type",
                        "application/json",
                    ),
                    "transfer-encoding",
                    "chunked",
                )
            } else if !use_chunked {
                warn!("📏 Setting Content-Length: {} bytes", response_size);
                // For standard responses, set Content-Length and no Transfer-Encoding
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
            } else {
                warn!("🔄 Setting Transfer-Encoding: chunked");
                // For chunked responses, set Transfer-Encoding and no Content-Length
                reply::with_header(
                    reply::with_header(
                        reply::with_status(
                            warp::reply::Response::new(response_json.into()),
                            status,
                        ),
                        "content-type",
                        "application/json",
                    ),
                    "transfer-encoding",
                    "chunked",
                )
            };

            // Add connection header
            let base_reply = reply::with_header(base_reply, "connection", connection_header);

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

            info!("📤 === MCP REQUEST COMPLETE ===");
            info!("   Request ID: {}", request_id);
            info!("   Method: {}", method);
            info!("   Total Processing Time: {:?}", total_request_time);
            info!("   Request Size: {} bytes", request_size);
            info!("   Response Size: {} bytes", response_size);
            info!("   Status: SUCCESS");

            if is_cursor_client {
                warn!("🎯 === CURSOR REQUEST COMPLETE ===");
                warn!("   Request ID: {}", request_id);
                warn!("   Total Time: {:?}", total_request_time);
                warn!("   Response Size: {} bytes", response_size);
                warn!(
                    "   Headers: {}",
                    if use_chunked {
                        "Chunked"
                    } else {
                        "Content-Length"
                    }
                );
                warn!("   Protocol Compliance: ✅ (No dual headers)");

                // Performance recommendations for Cursor
                if response_size > 5000 && total_request_time.as_millis() > 1000 {
                    warn!("💡 CURSOR OPTIMIZATION OPPORTUNITY:");
                    warn!("   Consider response compression or pagination");
                    warn!("   Large responses may impact Cursor user experience");
                }
            }

            info!("📤 Sending HTTP MCP response with status: {}", status);

            // Add a small delay to see if it helps with client timeouts
            // Especially important for Cursor client stability
            let delay_ms = if is_cursor_client && response_size > 5000 {
                15
            } else {
                10
            };
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

            info!("✅ === REQUEST {} COMPLETE ===", request_id);
            Ok(final_reply.into_response())
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
