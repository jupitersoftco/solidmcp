//! MCP Debug Logging Module
//!
//! Provides comprehensive structured logging for the MCP server using the tracing crate.
//! Includes connection tracking, request IDs, and performance metrics.

use {
    std::sync::atomic::{AtomicU64, Ordering},
    std::time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    tracing::{debug, error, info, span, trace, warn, Level, Span},
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter},
    uuid::Uuid,
};

/// Initialize the tracing subscriber with appropriate configuration
pub fn init_tracing() {
    // Try to get log level from environment, default to info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("solidmcp=info,warp=info"));

    // Create the formatting layer with useful defaults
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .with_ansi(true);

    // Check if JSON format is requested
    let json_format = std::env::var("LOG_FORMAT")
        .map(|v| v.to_lowercase() == "json")
        .unwrap_or(false);

    if json_format {
        // JSON format for production/structured logging
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    } else {
        // Human-readable format for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    info!("Tracing initialized");
}

/// Generate a unique request ID for tracking
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn generate_request_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    
    format!("{:x}-{:04x}", timestamp, counter % 0x10000)
}

#[derive(Debug, Clone)]
pub struct McpConnectionId(pub String);

impl McpConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for McpConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for McpConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Create a span for tracking a connection lifecycle
pub fn connection_span(connection_id: &McpConnectionId) -> Span {
    span!(
        Level::INFO,
        "mcp_connection",
        connection_id = %connection_id,
        start_time = ?Instant::now()
    )
}

/// Create a span for tracking a request
pub fn request_span(method: &str, request_id: &str, session_id: Option<&str>) -> Span {
    span!(
        Level::INFO,
        "mcp_request",
        method = %method,
        request_id = %request_id,
        session_id = session_id,
    )
}

/// Log connection events
pub fn log_connection_upgrade(connection_id: &McpConnectionId) {
    info!(
        connection_id = %connection_id,
        event = "websocket_upgrade",
        "WebSocket connection upgraded"
    );
}

pub fn log_connection_closed(connection_id: &McpConnectionId, duration: Duration) {
    info!(
        connection_id = %connection_id,
        event = "connection_closed",
        duration_ms = duration.as_millis(),
        "Connection closed"
    );
}

/// Log message events
pub fn log_message_received(message_type: &str, message_size: usize) {
    debug!(
        message_type = %message_type,
        message_size = message_size,
        event = "message_received",
        "Received message"
    );
}

pub fn log_message_parsed(method: &str, id: Option<&serde_json::Value>) {
    trace!(
        method = %method,
        message_id = ?id,
        event = "message_parsed",
        "Parsed message"
    );
}

/// Log handler events
pub fn log_handler_start(method: &str) {
    debug!(
        method = %method,
        event = "handler_start",
        "Starting to handle method"
    );
}

pub fn log_handler_success(method: &str, duration: Duration) {
    info!(
        method = %method,
        duration_ms = duration.as_millis(),
        event = "handler_success",
        "Successfully handled method"
    );
}

pub fn log_handler_error(method: &str, error: &str, duration: Duration) {
    error!(
        method = %method,
        error = %error,
        duration_ms = duration.as_millis(),
        event = "handler_error",
        "Failed to handle method"
    );
}

/// Log response events
pub fn log_response_sent(response_size: usize) {
    debug!(
        response_size = response_size,
        event = "response_sent",
        "Sent response"
    );
}

pub fn log_response_error(error: &str) {
    error!(
        error = %error,
        event = "response_error",
        "Failed to send response"
    );
}

/// Log tool events
pub fn log_tool_call(tool: &str, args: &serde_json::Value) {
    info!(
        tool = %tool,
        args = ?args,
        event = "tool_call",
        "Tool call requested"
    );
}

pub fn log_unknown_tool(tool: &str) {
    warn!(
        tool = %tool,
        event = "unknown_tool",
        "Unknown tool requested"
    );
}

/// Log error events with context
pub fn log_parse_error(error: &str, raw_message: &str) {
    error!(
        error = %error,
        raw_message = %raw_message,
        event = "parse_error",
        "Failed to parse message"
    );
}

pub fn log_unknown_method(method: &str) {
    warn!(
        method = %method,
        event = "unknown_method",
        "Unknown MCP method requested"
    );
}

/// Server lifecycle logging
pub fn log_server_startup(port: u16) {
    info!(
        port = port,
        event = "server_startup",
        "Starting MCP Server"
    );
}

pub fn log_server_ready(addr: &str) {
    info!(
        address = %addr,
        event = "server_ready",
        "MCP Server ready and listening"
    );
}

pub fn log_server_shutdown() {
    info!(
        event = "server_shutdown",
        "MCP Server shutting down"
    );
}

/// Utility macros for common patterns
#[macro_export]
macro_rules! log_error_with_context {
    ($err:expr, $msg:expr) => {
        tracing::error!(
            error = %$err,
            error_type = std::any::type_name_of_val(&$err),
            $msg
        )
    };
    ($err:expr, $msg:expr, $($field:tt)*) => {
        tracing::error!(
            error = %$err,
            error_type = std::any::type_name_of_val(&$err),
            $($field)*,
            $msg
        )
    };
}

#[macro_export]
macro_rules! log_tool_execution {
    ($tool_name:expr, $duration:expr, $result:expr) => {
        match $result {
            Ok(_) => tracing::info!(
                tool = $tool_name,
                duration_ms = $duration.as_millis(),
                event = "tool_execution_success",
                "Tool executed successfully"
            ),
            Err(ref e) => tracing::error!(
                tool = $tool_name,
                duration_ms = $duration.as_millis(),
                error = %e,
                event = "tool_execution_error",
                "Tool execution failed"
            ),
        }
    };
}

// Tracing macros are already imported above - no need to re-export

