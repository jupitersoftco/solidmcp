//! MCP (Model Context Protocol) Server Library
//!
//! A standalone implementation of the Model Context Protocol server
//! supporting both WebSocket and HTTP transports.

// Re-export the main modules
pub mod core;
pub mod handler;
pub mod handlers;
pub mod http;
pub mod logging;
pub mod protocol;
pub mod protocol_impl;
// Legacy trait removed - internal use only
// pub mod protocol_testable;
pub mod server;
pub mod shared;
pub mod tools;
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

// Re-export high-level server API (for convenience)
pub use server::{
    ExtendedToolDefinition, HighLevelMcpServer, McpPromptProvider, McpResourceProvider,
    McpServerBuilder, McpTool, ServerCapabilities, ToolContext,
};

// Re-export WebSocket handler for convenience
pub use websocket::handle_mcp_ws_main as handle_mcp_ws;
