//! MCP HTTP Handler
//!
//! HTTP transport for MCP protocol messages with intelligent transport negotiation.

use {
    super::shared::McpProtocolEngine,
    super::transport::{
        cors_headers, transport_capabilities, TransportCapabilities, TransportInfo,
        TransportNegotiation,
    },
    crate::http::{
        extract_session_context,
        validate_request, validate_message_structure,
        ResponseBuilder,
        ProgressHandler,
    },
    crate::logging::generate_request_id,
    serde_json::{json, Value},
    std::sync::Arc,
    std::time::{Duration, Instant},
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
    // Use the refactored handler
    handle_mcp_http_impl(message, content_type, accept, connection, cookie, handler).await
}

/// Implementation of the HTTP handler using extracted modules
async fn handle_mcp_http_impl(
    message: Value,
    content_type: Option<String>,
    accept: Option<String>,
    connection: Option<String>,
    cookie: Option<String>,
    handler: Arc<McpProtocolEngine>,
) -> Result<impl Reply, Rejection> {
    let request_start = Instant::now();
    let request_id = generate_request_id();
    
    // Normalize headers
    let content_type = content_type.unwrap_or_else(|| "application/json".to_string());
    let accept = accept.unwrap_or_else(|| "application/json".to_string());
    let connection = connection.unwrap_or_else(|| "close".to_string());
    
    // Log request start
    trace!(
        request_id = %request_id,
        content_type = %content_type,
        accept = %accept,
        connection = %connection,
        cookie = ?cookie,
        "MCP HTTP request received"
    );
    
    // Detect special clients
    let is_cursor_client = detect_cursor_client(&content_type, &accept, &cookie);
    if is_cursor_client {
        warn!(
            request_id = %request_id,
            client = "Cursor",
            "Cursor client detected - applying special handling"
        );
    }
    
    // Validate request
    let validated = match validate_request(message, &content_type) {
        Ok(v) => v,
        Err(e) => {
            error!(
                request_id = %request_id,
                error = %e,
                "Request validation failed"
            );
            return Ok(ResponseBuilder::new()
                .build_error(e, None));
        }
    };
    
    // Extract session context
    let session = extract_session_context(&validated.method, &cookie);
    debug!(
        request_id = %request_id,
        session_id = %session.id,
        method = %validated.method,
        is_new_session = session.is_new,
        "Processing request with session context"
    );
    
    // Additional message structure validation
    if let Err(e) = validate_message_structure(&validated.message) {
        error!(
            request_id = %request_id,
            error = ?e,
            "Message structure validation failed"
        );
        let error = crate::error::McpError::InvalidParams(format!("Invalid request: {:?}", e));
        return Ok(ResponseBuilder::new()
            .build_error(error, Some(validated.message_id.clone())));
    }
    
    // Log request details
    if validated.has_progress_token {
        warn!(
            request_id = %request_id,
            method = %validated.method,
            "Progress token detected - client expects streaming updates"
        );
    }
    
    if validated.request_size > 10000 {
        warn!(
            request_id = %request_id,
            size_bytes = validated.request_size,
            size_kb = format!("{:.2}", validated.request_size as f64 / 1024.0),
            "Large request detected"
        );
    }
    
    // Set up progress handling if needed
    let progress_handler = if validated.has_progress_token {
        Some(ProgressHandler::new())
    } else {
        None
    };
    
    // Process the message
    let process_start = Instant::now();
    let progress_sender = progress_handler.as_ref().map(|h| h.sender());
    
    // For now, we don't pass the progress sender to maintain compatibility
    let result = handler.handle_message(
        validated.message.clone(),
        Some(session.id.clone()),
    ).await;
    let process_duration = process_start.elapsed();
    
    // Log processing result
    match &result {
        Ok(_) => {
            debug!(
                request_id = %request_id,
                method = %validated.method,
                duration_ms = process_duration.as_millis(),
                "Request processed successfully"
            );
        }
        Err(e) => {
            error!(
                request_id = %request_id,
                method = %validated.method,
                error = %e,
                duration_ms = process_duration.as_millis(),
                "Request processing failed"
            );
        }
    }
    
    // Build response
    let mut response_builder = ResponseBuilder::new();
    
    // Add session cookie for new sessions
    if session.is_new && validated.method == "initialize" {
        let session_id = crate::http::session::generate_session_id();
        response_builder = response_builder.with_session(session_id);
    }
    
    // Enable chunked encoding if progress token present
    if validated.has_progress_token {
        response_builder = response_builder.with_chunked_encoding(true);
    }
    
    // Add a small delay for large responses to Cursor clients
    let result_value = match &result {
        Ok(response_value) => {
            let response_size = serde_json::to_string(&response_value)
                .map(|s| s.len())
                .unwrap_or(0);
            
            debug!(
                request_id = %request_id,
                response_size_bytes = response_size,
                response_size_kb = format!("{:.2}", response_size as f64 / 1024.0),
                "Sending success response"
            );
            
            // Log performance metrics
            let total_duration = request_start.elapsed();
            if is_cursor_client && total_duration.as_millis() > 500 {
                warn!(
                    request_id = %request_id,
                    duration_ms = total_duration.as_millis(),
                    "Slow response for Cursor client"
                );
            }
            
            if is_cursor_client && response_size > 5000 {
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
            
            Some(response_value.clone())
        }
        Err(e) => {
            debug!(
                request_id = %request_id,
                error = %e,
                "Sending error response"
            );
            None
        }
    };
    
    // Build final response
    let response = if let Some(response_value) = result_value {
        response_builder.build_success(response_value)
    } else if let Err(e) = result {
        response_builder.build_error(e, Some(validated.message_id))
    } else {
        unreachable!()
    };
    
    trace!(
        request_id = %request_id,
        total_duration_ms = request_start.elapsed().as_millis(),
        "Request complete"
    );
    
    Ok(response)
}

/// Detect if the client is Cursor IDE
fn detect_cursor_client(content_type: &str, accept: &str, cookie: &Option<String>) -> bool {
    content_type.contains("Cursor")
        || accept.contains("Cursor")
        || cookie.as_ref().is_some_and(|c| c.contains("Cursor"))
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


/// Create an error reply with proper headers
/// Note: MCP uses JSON-RPC error codes in the body, not HTTP status codes
/// So we always return 200 OK status even for protocol errors
fn create_error_reply(error_response: Value, status: StatusCode) -> warp::reply::Response {
    let base_reply = reply::with_status(reply::json(&error_response), status);
    reply::with_header(base_reply, "content-type", "application/json").into_response()
}
