//! MCP Debug Logging Module
//!
//! Provides comprehensive structured logging for the MCP server using the tracing crate.
//! Includes connection tracking, request IDs, and performance metrics.

use {
    std::sync::atomic::{AtomicU64, Ordering},
    std::time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    tracing::{debug, error, info, instrument, span, trace, warn, Level, Span},
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
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_layer)
            .init();
    } else {
        // Human-readable format for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    info!("Tracing initialized with level: {}", env_filter);
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

// Re-export commonly used tracing macros
pub use tracing::{debug, error, info, trace, warn};

// Backwards compatibility - these will be removed after migration
#[deprecated(note = "Use structured logging functions instead")]
pub struct McpDebugLogger {
    connection_id: McpConnectionId,
    start_time: Instant,
}

#[allow(deprecated)]
impl McpDebugLogger {
    pub fn new(connection_id: McpConnectionId) -> Self {
        Self {
            connection_id,
            start_time: Instant::now(),
        }
    }

    pub fn fmt_connection_upgrade(&self) -> String {
        log_connection_upgrade(&self.connection_id);
        format!("[MCP:{}] 🔌 WebSocket connection upgraded", self.connection_id.0)
    }

    pub fn fmt_message_received(&self, message_type: &str, message_size: usize) -> String {
        log_message_received(message_type, message_size);
        format!("[MCP:{}] 📥 Received {} message ({} bytes)", self.connection_id.0, message_type, message_size)
    }

    pub fn fmt_message_parsed(&self, method: &str, id: &str) -> String {
        log_message_parsed(method, None);
        format!("[MCP:{}] 🔍 Parsed message - Method: {}, ID: {}", self.connection_id.0, method, id)
    }

    pub fn fmt_message_handling_start(&self, method: &str) -> String {
        log_handler_start(method);
        format!("[MCP:{}] ⚙️  Starting to handle method: {}", self.connection_id.0, method)
    }

    pub fn fmt_message_handling_success(&self, method: &str, duration: Duration) -> String {
        log_handler_success(method, duration);
        format!("[MCP:{}] ✅ Successfully handled {} in {:?}", self.connection_id.0, method, duration)
    }

    pub fn fmt_message_handling_error(&self, method: &str, error: &str, duration: Duration) -> String {
        log_handler_error(method, error, duration);
        format!("[MCP:{}] ❌ Failed to handle {} after {:?}: {}", self.connection_id.0, method, duration, error)
    }

    pub fn fmt_response_sent(&self, response_size: usize) -> String {
        log_response_sent(response_size);
        format!("[MCP:{}] 📤 Sent response ({} bytes)", self.connection_id.0, response_size)
    }

    pub fn fmt_response_error(&self, error: &str) -> String {
        log_response_error(error);
        format!("[MCP:{}] 💥 Failed to send response: {}", self.connection_id.0, error)
    }

    pub fn fmt_connection_closed(&self) -> String {
        let duration = self.start_time.elapsed();
        log_connection_closed(&self.connection_id, duration);
        format!("[MCP:{}] 🔌 Connection closed after {:?}", self.connection_id.0, duration)
    }

    pub fn fmt_parse_error(&self, error: &str, raw_message: &str) -> String {
        log_parse_error(error, raw_message);
        format!("[MCP:{}] 🚫 Failed to parse message: {} | Raw: {}", self.connection_id.0, error, raw_message)
    }

    pub fn fmt_unknown_method(&self, method: &str) -> String {
        log_unknown_method(method);
        format!("[MCP:{}] ❓ Unknown MCP method requested: {}", self.connection_id.0, method)
    }

    pub fn fmt_unknown_tool(&self, tool: &str) -> String {
        log_unknown_tool(tool);
        format!("[MCP:{}] 🛠️  Unknown tool requested: {}", self.connection_id.0, tool)
    }

    pub fn fmt_tool_call(&self, tool: &str, args: &str) -> String {
        format!("[MCP:{}] 🛠️  Tool call - {} with args: {}", self.connection_id.0, tool, args)
    }

    pub fn fmt_server_creation(&self) -> String {
        format!("[MCP:{}] 🏗️  Creating MCP server instance", self.connection_id.0)
    }

    pub fn fmt_server_creation_error(&self, error: &str) -> String {
        format!("[MCP:{}] 💥 Failed to create MCP server: {}", self.connection_id.0, error)
    }

    pub fn fmt_connection_start(&self) -> String {
        format!("[MCP:{}] 🔄 Starting MCP message processing loop", self.connection_id.0)
    }

    pub fn connection_id(&self) -> &McpConnectionId {
        &self.connection_id
    }
}

// Backwards compatibility functions
#[deprecated(note = "Use log_server_startup instead")]
pub fn fmt_mcp_server_startup(port: u16) -> String {
    log_server_startup(port);
    format!("🚀 Starting MCP Server on port {port}")
}

#[deprecated(note = "Use log_server_ready instead")]
pub fn fmt_mcp_server_ready(addr: &str) -> String {
    log_server_ready(addr);
    format!("✅ MCP Server ready and listening on {addr}")
}

#[deprecated(note = "Use log_server_shutdown instead")]
pub fn fmt_mcp_server_shutdown() -> String {
    log_server_shutdown();
    "🛑 MCP Server shutting down".to_string()
}