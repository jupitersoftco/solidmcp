//! MCP (Model Context Protocol) Server Library
//!
//! A standalone implementation of the Model Context Protocol server
//! supporting both WebSocket and HTTP transports.

// Re-export the main modules
pub mod content_types;
pub mod core;
pub mod framework;
pub mod handler;
pub mod handlers;
pub mod http;
pub mod logging;
pub mod protocol;
pub mod protocol_impl;
// Legacy trait removed - internal use only
// pub mod protocol_testable;
// Legacy server module removed - use framework module instead
pub mod shared;
pub mod tools;
pub mod transport;
pub mod validation;
pub mod websocket;

// Test modules
#[cfg(test)]
pub mod tests;

// Re-export key types
pub use core::McpServer;
pub use protocol::McpProtocol;
pub use protocol_impl::McpProtocolHandlerImpl;
// Legacy trait removed - McpProtocolHandler is now internal
pub use tools::McpTools;

// Re-export core handler trait and types
pub use handler::{
    LogLevel, McpContext, McpHandler, McpNotification, PromptArgument, PromptContent, PromptInfo,
    PromptMessage, ResourceContent, ResourceInfo, ToolDefinition, TypedToolDefinition,
};

// Re-export schemars for convenience
pub use schemars::JsonSchema;

// Re-export new framework API
pub use framework::{FrameworkHandler, McpServerBuilder, PromptProvider, ResourceProvider};

// Re-export content types for type-safe MCP responses
pub use content_types::{McpContent, McpResponse, ToMcpResponse};

// Re-export WebSocket handler for convenience
pub use websocket::handle_mcp_ws_main as handle_mcp_ws;
