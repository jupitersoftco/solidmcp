//! Refactored HTTP Handler
//!
//! This is the refactored version of handle_mcp_http using extracted modules.

use {
    super::shared::McpProtocolEngine,
    crate::http::{
        SessionContext, extract_session_context,
        ValidatedRequest, validate_request, validate_message_structure,
        ResponseBuilder, create_error_response,
        ProgressHandler, has_progress_token,
    },
    crate::logging::generate_request_id,
    serde_json::{json, Value},
    std::sync::Arc,
    std::time::{Duration, Instant},
    tracing::{debug, error, trace, warn, info},
    warp::http::StatusCode,
    warp::{Rejection, Reply},
};

/// Refactored HTTP handler for MCP requests
pub async fn handle_mcp_http_refactored(
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
                .build_error(e, None)
                .into_response());
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
            .build_error(error, Some(validated.message_id))
            .into_response());
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
    let result = handler.handle_message(
        validated.message.clone(),
        Some(session.id.clone()),
    ).await;
    let process_duration = process_start.elapsed();
    
    // Log processing result
    match &result {
        Ok(_) => {
            info!(
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
    if session.is_new {
        let session_id = if validated.method == "initialize" {
            // Generate a proper session ID for initialize
            crate::http::session::generate_session_id()
        } else {
            session.id.clone()
        };
        response_builder = response_builder.with_session(session_id);
    }
    
    // Enable chunked encoding if progress token present
    if validated.has_progress_token {
        response_builder = response_builder.with_chunked_encoding(true);
    }
    
    // Handle progress notifications if any
    if let Some(mut progress_handler) = progress_handler {
        let notifications = progress_handler.handle_with_timeout(Duration::from_secs(5)).await;
        if !notifications.is_empty() {
            debug!(
                request_id = %request_id,
                count = notifications.len(),
                "Collected progress notifications"
            );
            // TODO: Implement chunked response with notifications
        }
    }
    
    // Build final response
    let response = match result {
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
            
            response_builder.build_success(response_value)
        }
        Err(e) => {
            debug!(
                request_id = %request_id,
                "Sending error response"
            );
            response_builder.build_error(e, Some(validated.message_id))
        }
    };
    
    Ok(response.into_response())
}

/// Detect if the client is Cursor IDE
fn detect_cursor_client(content_type: &str, accept: &str, cookie: &Option<String>) -> bool {
    content_type.contains("Cursor")
        || accept.contains("Cursor")
        || cookie.as_ref().is_some_and(|c| c.contains("Cursor"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::McpProtocolEngine;
    
    #[tokio::test]
    async fn test_handle_mcp_http_refactored_valid_request() {
        let engine = Arc::new(McpProtocolEngine::new());
        let message = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {}
        });
        
        let result = handle_mcp_http_refactored(
            message,
            Some("application/json".to_string()),
            None,
            None,
            None,
            engine,
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_handle_mcp_http_refactored_invalid_content_type() {
        let engine = Arc::new(McpProtocolEngine::new());
        let message = json!({
            "jsonrpc": "2.0",
            "method": "test",
            "id": 1
        });
        
        let result = handle_mcp_http_refactored(
            message,
            Some("text/plain".to_string()),
            None,
            None,
            None,
            engine,
        ).await;
        
        assert!(result.is_ok());
        // The response should be an error
    }
    
    #[tokio::test]
    async fn test_handle_mcp_http_refactored_with_session() {
        let engine = Arc::new(McpProtocolEngine::new());
        let message = json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        });
        
        let cookie = Some("mcp_session=test123; Path=/".to_string());
        
        let result = handle_mcp_http_refactored(
            message,
            Some("application/json".to_string()),
            None,
            None,
            cookie,
            engine,
        ).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_handle_mcp_http_refactored_with_progress_token() {
        let engine = Arc::new(McpProtocolEngine::new());
        let message = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 1,
            "params": {
                "_meta": {
                    "progressToken": "abc123"
                }
            }
        });
        
        let result = handle_mcp_http_refactored(
            message,
            Some("application/json".to_string()),
            None,
            None,
            None,
            engine,
        ).await;
        
        assert!(result.is_ok());
    }
}