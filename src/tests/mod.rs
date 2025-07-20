//! MCP Server Tests
//!
//! Tests for Model Context Protocol server functionality.

pub mod capability_negotiation_tests;
pub mod dependency_integration_tests;
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
mod tests {
    use crate::handlers::McpHandlers;
    use crate::logging::{McpConnectionId, McpDebugLogger};
    use crate::protocol::McpProtocol;
    use crate::tools::McpTools;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let connection_id = McpConnectionId::new();
        let logger = McpDebugLogger::new(connection_id);
        let _handlers = McpHandlers::new(logger);
        // Basic handler creation test - just ensure it compiles
        assert!(true);
    }

    #[tokio::test]
    async fn test_mcp_read_file_handler() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "hello-mcp").unwrap();
        let path = tmp.path().to_str().unwrap();

        let tool_params = json!({
            "file_path": path
        });

        let result = McpTools::execute_tool("read_file", tool_params)
            .await
            .unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
        assert_eq!(parsed["content"], "hello-mcp");
    }

    #[tokio::test]
    async fn test_mcp_initialize() {
        let protocol = McpProtocol::new();
        let result = protocol.create_initialize_response();

        assert_eq!(result["protocolVersion"], "2025-06-18");
        assert_eq!(result["serverInfo"]["name"], "mcp-server");
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let result = McpTools::get_tools_list();

        let tools = result["tools"].as_array().unwrap();
        assert!(tools.len() >= 2); // echo and read_file

        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(tool_names.contains(&"echo"));
        assert!(tool_names.contains(&"read_file"));
    }

    #[tokio::test]
    async fn test_mcp_echo_tool() {
        let tool_params = json!({
            "message": "Hello, MCP!"
        });

        let result = McpTools::execute_tool("echo", tool_params).await.unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
        assert_eq!(parsed["echo"], "Hello, MCP!");
    }

    #[tokio::test]
    async fn test_mcp_read_file_tool() {
        let tool_params = json!({
            "file_path": "Cargo.toml"
        });

        let result = McpTools::execute_tool("read_file", tool_params)
            .await
            .unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content).unwrap();

        assert_eq!(parsed["file_path"], "Cargo.toml");
        assert!(parsed["content"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_mcp_message_handling() {
        let connection_id = McpConnectionId::new();
        let logger = McpDebugLogger::new(connection_id);
        let handlers = McpHandlers::new(logger);

        let message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let response = handlers.handle_mcp_message(message).await.unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_object());
    }
}
