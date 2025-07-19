//! MCP Debug Logging Module
//!
//! Provides comprehensive debug logging for the MCP server with structured
//! logging, connection tracking, and performance metrics.

use {
    std::time::{Duration, Instant},
    uuid::Uuid,
};

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

#[derive(Debug)]
pub struct McpDebugLogger {
    connection_id: McpConnectionId,
    start_time: Instant,
}

impl McpDebugLogger {
    pub fn new(connection_id: McpConnectionId) -> Self {
        let start_time = Instant::now();
        Self {
            connection_id,
            start_time,
        }
    }

    pub fn fmt_connection_upgrade(&self) -> String {
        format!(
            "[MCP:{}] 🔌 WebSocket connection upgraded",
            self.connection_id.0
        )
    }

    pub fn fmt_message_received(&self, message_type: &str, message_size: usize) -> String {
        format!(
            "[MCP:{}] 📥 Received {} message ({} bytes)",
            self.connection_id.0, message_type, message_size
        )
    }

    pub fn fmt_message_parsed(&self, method: &str, id: &str) -> String {
        format!(
            "[MCP:{}] 🔍 Parsed message - Method: {}, ID: {}",
            self.connection_id.0, method, id
        )
    }

    pub fn fmt_message_handling_start(&self, method: &str) -> String {
        format!(
            "[MCP:{}] ⚙️  Starting to handle method: {}",
            self.connection_id.0, method
        )
    }

    pub fn fmt_message_handling_success(&self, method: &str, duration: Duration) -> String {
        format!(
            "[MCP:{}] ✅ Successfully handled {} in {:?}",
            self.connection_id.0, method, duration
        )
    }

    pub fn fmt_message_handling_error(
        &self,
        method: &str,
        error: &str,
        duration: Duration,
    ) -> String {
        format!(
            "[MCP:{}] ❌ Failed to handle {} after {:?}: {}",
            self.connection_id.0, method, duration, error
        )
    }

    pub fn fmt_response_sent(&self, response_size: usize) -> String {
        format!(
            "[MCP:{}] 📤 Sent response ({} bytes)",
            self.connection_id.0, response_size
        )
    }

    pub fn fmt_response_error(&self, error: &str) -> String {
        format!(
            "[MCP:{}] 💥 Failed to send response: {}",
            self.connection_id.0, error
        )
    }

    pub fn fmt_connection_closed(&self) -> String {
        let duration = self.start_time.elapsed();
        format!(
            "[MCP:{}] 🔌 Connection closed after {:?}",
            self.connection_id.0, duration
        )
    }

    pub fn fmt_parse_error(&self, error: &str, raw_message: &str) -> String {
        format!(
            "[MCP:{}] 🚫 Failed to parse message: {} | Raw: {}",
            self.connection_id.0, error, raw_message
        )
    }

    pub fn fmt_unknown_method(&self, method: &str) -> String {
        format!(
            "[MCP:{}] ❓ Unknown MCP method requested: {}",
            self.connection_id.0, method
        )
    }

    pub fn fmt_unknown_tool(&self, tool: &str) -> String {
        format!(
            "[MCP:{}] 🛠️  Unknown tool requested: {}",
            self.connection_id.0, tool
        )
    }

    pub fn fmt_tool_call(&self, tool: &str, args: &str) -> String {
        format!(
            "[MCP:{}] 🛠️  Tool call - {} with args: {}",
            self.connection_id.0, tool, args
        )
    }

    pub fn fmt_server_creation(&self) -> String {
        format!(
            "[MCP:{}] 🏗️  Creating MCP server instance",
            self.connection_id.0
        )
    }

    pub fn fmt_server_creation_error(&self, error: &str) -> String {
        format!(
            "[MCP:{}] 💥 Failed to create MCP server: {}",
            self.connection_id.0, error
        )
    }

    pub fn fmt_connection_start(&self) -> String {
        format!(
            "[MCP:{}] 🔄 Starting MCP message processing loop",
            self.connection_id.0
        )
    }

    pub fn connection_id(&self) -> &McpConnectionId {
        &self.connection_id
    }
}

pub fn fmt_mcp_server_startup(port: u16) -> String {
    format!("🚀 Starting MCP Server on port {port}")
}

pub fn fmt_mcp_server_ready(addr: &str) -> String {
    format!("✅ MCP Server ready and listening on {addr}")
}

pub fn fmt_mcp_server_shutdown() -> String {
    "🛑 MCP Server shutting down".to_string()
}
