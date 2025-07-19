//! MCP Tools Implementation
//!
//! Defines available MCP tools and their implementations.

use {
    anyhow::Result,
    serde_json::{json, Value},
    tokio::fs,
    tracing::{debug, error, info},
};

pub struct McpTools;

impl McpTools {
    /// Get list of available tools with their schemas
    pub fn get_tools_list() -> Value {
        Self::get_tools_list_for_version(None)
    }

    /// Get list of available tools with protocol version-specific schemas
    pub fn get_tools_list_for_version(protocol_version: Option<&str>) -> Value {
        info!(
            "📋 MCP tools list requested for protocol version: {:?}",
            protocol_version
        );

        // Both protocol versions use the same schema format for tools
        // The main difference is in the initialization response, not the tools list
        let tools = vec![
            json!({
                "name": "echo",
                "description": "Echo back the input message",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Message to echo"
                        }
                    },
                    "required": ["message"]
                },
                "output_schema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": {
                                        "type": "string",
                                        "enum": ["text"]
                                    },
                                    "text": {
                                        "type": "string"
                                    }
                                },
                                "required": ["type", "text"]
                            }
                        }
                    },
                    "required": ["content"]
                }
            }),
            json!({
                "name": "read_file",
                "description": "Read contents of a file",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["file_path"]
                },
                "output_schema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": {
                                        "type": "string",
                                        "enum": ["text"]
                                    },
                                    "text": {
                                        "type": "string"
                                    }
                                },
                                "required": ["type", "text"]
                            }
                        }
                    },
                    "required": ["content"]
                }
            }),
        ];

        info!(
            "📋 Returning {} available tools for protocol version {:?}",
            tools.len(),
            protocol_version
        );
        json!({ "tools": tools })
    }

    /// Execute a tool by name
    pub async fn execute_tool(tool_name: &str, arguments: Value) -> Result<Value> {
        let result = match tool_name {
            "echo" => Self::handle_echo(arguments).await?,
            "read_file" => Self::handle_read_file(arguments).await?,
            _ => {
                error!("Unknown tool: {}", tool_name);
                return Err(anyhow::anyhow!("Unknown tool: {}", tool_name));
            }
        };

        debug!(
            "\u{1F6E0}\u{FE0F}  Tool '{}' executed successfully",
            tool_name
        );
        let text = serde_json::to_string(&result)?;
        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        }))
    }

    /// Echo handler for MCP
    async fn handle_echo(params: Value) -> Result<Value> {
        let message = params["message"].as_str().unwrap_or("").to_string();
        debug!("🔊 Echo request received: '{}'", message);

        let response = json!({ "echo": message });
        info!("🔊 Echo response sent: '{}'", message);

        Ok(response)
    }

    /// Read file handler for MCP with error logging
    async fn handle_read_file(params: Value) -> Result<Value> {
        let file_path = params["file_path"].as_str().unwrap_or("");
        debug!("📖 Reading file: {}", file_path);

        match fs::read_to_string(file_path).await {
            Ok(content) => {
                let content_length = content.len();
                info!(
                    "📖 Successfully read file '{}' ({} bytes)",
                    file_path, content_length
                );
                Ok(json!({ "file_path": file_path, "content": content }))
            }
            Err(e) => {
                error!("📖 Failed to read file '{}': {}", file_path, e);
                Ok(json!({
                    "file_path": file_path,
                    "content": format!("[ERROR] {}", e),
                    "error": e.to_string(),
                    "error_kind": format!("{:?}", e.kind())
                }))
            }
        }
    }
}