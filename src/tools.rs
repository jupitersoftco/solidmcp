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
        
        // Return structured data directly for programmatic consumption
        // instead of stringifying it into human-readable text
        Ok(json!({
            "content": [
                {
                    "type": "text", 
                    "text": Self::format_result_summary(&result, tool_name)
                }
            ],
            "data": result  // Structured data for programmatic access
        }))
    }

    /// Format a human-readable summary of the result while preserving structured data
    fn format_result_summary(result: &Value, tool_name: &str) -> String {
        match tool_name {
            "echo" => {
                if let Some(message) = result.get("echo").and_then(|v| v.as_str()) {
                    format!("Echo: {message}")
                } else {
                    "Echo completed".to_string()
                }
            },
            "read_file" => {
                if let Some(error) = result.get("error") {
                    format!("File read error: {error}")
                } else if let Some(path) = result.get("file_path").and_then(|v| v.as_str()) {
                    if let Some(content) = result.get("content").and_then(|v| v.as_str()) {
                        let len = content.len();
                        format!("Successfully read file '{path}' ({len} bytes)")
                    } else {
                        format!("Read file: {path}")
                    }
                } else {
                    "File read completed".to_string()
                }
            },
            _ => {
                // For search results or other tools, try to extract meaningful info
                if let Some(query) = result.get("query").and_then(|v| v.as_str()) {
                    if let Some(results) = result.get("results").and_then(|v| v.as_array()) {
                        format!("Found {} results for query '{query}'", results.len())
                    } else {
                        format!("Search completed for query '{query}'")
                    }
                } else {
                    format!("Tool '{tool_name}' completed successfully")
                }
            }
        }
    }

    /// Echo handler for MCP
    async fn handle_echo(params: Value) -> Result<Value> {
        let message = params["message"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter 'message'"))?
            .to_string();
        debug!("🔊 Echo request received: '{}'", message);

        let response = json!({ "echo": message });
        info!("🔊 Echo response sent: '{}'", message);

        Ok(response)
    }

    /// Read file handler for MCP with error logging
    async fn handle_read_file(params: Value) -> Result<Value> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter 'file_path'"))?;
        
        if file_path.is_empty() {
            return Err(anyhow::anyhow!("Parameter 'file_path' cannot be empty"));
        }
        
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
