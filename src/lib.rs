//! MCP (Model Context Protocol) Server Library
//!
//! A standalone implementation of the Model Context Protocol server
//! supporting both WebSocket and HTTP transports.

// Re-export the main modules
pub mod core;
pub mod handlers;
pub mod http;
pub mod logging;
pub mod protocol;
pub mod protocol_impl;
pub mod protocol_testable;
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
pub use protocol_testable::McpProtocolHandler;
pub use tools::McpTools;

// Re-export WebSocket handler for convenience
pub use websocket::handle_mcp_ws_main as handle_mcp_ws;
