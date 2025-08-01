//! MCP Server Tests
//!
//! Tests for Model Context Protocol server functionality.

pub mod capability_negotiation_tests;
pub mod dependency_integration_tests;
pub mod edge_case_tests;
pub mod error_handling_tests;
pub mod handler_trait_tests;
pub mod http;
pub mod jsonrpc_compliance_tests;
pub mod notifications_tests;
pub mod protocol_parsing_tests;
pub mod protocol_tests;
pub mod session_management_tests;
pub mod tools_tests;
pub mod transport_integration_tests;

#[cfg(test)]
mod integration_tests {
    use crate::logging::McpConnectionId;
    use crate::protocol::McpProtocol;
    use crate::protocol_impl::McpProtocolHandlerImpl;
    use serde_json::json;
    use std::io::Write;
    // Removed tempfile usage

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let _handler = McpProtocolHandlerImpl::new();
        // Basic handler creation test - just ensure it compiles
        // This test validates that handlers can be created successfully
    }

    // Note: Tool tests moved to examples/
    // The built-in tools are now example implementations

    #[tokio::test]
    async fn test_mcp_initialize() {
        let protocol = McpProtocol::new();
        let result = protocol.create_initialize_response();

        assert_eq!(result["protocolVersion"], "2025-06-18");
        assert_eq!(result["serverInfo"]["name"], "mcp-server");
    }


    #[tokio::test]
    async fn test_mcp_message_handling() {
        let handler = McpProtocolHandlerImpl::new();

        let message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let response = handler.handle_message(message).await.unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_object());
    }
}
