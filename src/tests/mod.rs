//! MCP Server Tests
//!
//! Tests for Model Context Protocol server functionality.

pub mod protocol_tests;
pub mod notifications_tests;
pub mod tools_tests;
pub mod http;

#[tokio::test]
async fn test_mcp_server_creation() {
    use crate::handlers::{SolidMcpHandler, Handler};
    let handler = SolidMcpHandler::new();
    // Basic handler creation test
    assert_eq!(handler.server_info().name, "solidmcp");
}

#[tokio::test]
async fn test_mcp_read_file_handler() {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::handlers::{SolidMcpHandler, Handler};
    use crate::tools::ToolCall;
    use serde_json::json;

    let handler = SolidMcpHandler::new();
    
    // Create a temp file
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "hello-mcp").unwrap();
    let path = tmp.path().to_str().unwrap();
    
    let tool_call = ToolCall {
        name: "read_file".to_string(),
        arguments: json!({
            "filePath": path
        }),
    };
    
    let result = handler.handle_tool_call(tool_call).await.unwrap();
    let content = result.content[0].text.as_ref().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
    assert_eq!(parsed["content"], "hello-mcp");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::{SolidMcpHandler, Handler};
    use crate::protocol::{InitializeRequest, InitializeParams, ClientInfo};
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_initialize() {
        let handler = SolidMcpHandler::new();
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: Default::default(),
            client_info: Some(ClientInfo {
                name: "test-client".to_string(),
                version: Some("1.0.0".to_string()),
            }),
        };
        
        let request = InitializeRequest {
            params,
        };
        
        let result = handler.handle_initialize(request).await.unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert_eq!(result.server_info.name, "solidmcp");
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let handler = SolidMcpHandler::new();
        let result = handler.handle_tools_list().await.unwrap();
        
        let tools = &result.tools;
        assert!(tools.len() >= 2); // echo and read_file
        
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"echo"));
        assert!(tool_names.contains(&"read_file"));
    }

    #[tokio::test]
    async fn test_mcp_echo_tool() {
        use crate::tools::ToolCall;
        let handler = SolidMcpHandler::new();
        
        let tool_call = ToolCall {
            name: "echo".to_string(),
            arguments: json!({
                "message": "Hello, MCP!"
            }),
        };
        
        let result = handler.handle_tool_call(tool_call).await.unwrap();
        let content = result.content[0].text.as_ref().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
        assert_eq!(parsed["echo"], "Hello, MCP!");
    }

    #[tokio::test]
    async fn test_mcp_read_file_tool() {
        use crate::tools::ToolCall;
        let handler = SolidMcpHandler::new();
        
        let tool_call = ToolCall {
            name: "read_file".to_string(),
            arguments: json!({
                "filePath": "Cargo.toml"
            }),
        };
        
        let result = handler.handle_tool_call(tool_call).await.unwrap();
        let content = result.content[0].text.as_ref().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
        
        assert_eq!(parsed["filePath"], "Cargo.toml");
        assert!(parsed["content"].as_str().is_some());
    }
}