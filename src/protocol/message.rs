//! Optimized JSON-RPC message processing
//!
//! This module provides zero-copy JSON parsing and unified message types
//! to optimize the MCP protocol processing pipeline.

use {
    crate::error::{McpError, McpResult},
    serde::{Deserialize, Serialize},
    serde_json::{value::RawValue, Value},
};

/// Raw JSON-RPC message with lazy parsing
/// 
/// This type uses borrowed data to avoid unnecessary allocations
/// and provides zero-copy parsing for better performance.
#[derive(Debug, Deserialize)]
pub struct RawMessage<'a> {
    /// JSON-RPC version (should be "2.0")
    pub jsonrpc: &'a str,
    /// Request ID (can be string, number, or null)
    #[serde(borrow)]
    pub id: Option<&'a RawValue>,
    /// Method name
    pub method: &'a str,
    /// Parameters as raw JSON (lazy parsing)
    #[serde(borrow)]
    pub params: Option<&'a RawValue>,
}

/// Parsed and validated MCP message
/// 
/// After the raw message is validated, it's converted to this enum
/// which provides type-safe access to the parsed parameters.
#[derive(Debug)]
pub enum ParsedMessage {
    /// MCP initialize method
    Initialize(InitializeParams),
    /// MCP tools/list method (no params)
    ToolsList,
    /// MCP tools/call method
    ToolsCall(ToolCallParams),
    /// MCP resources/list method (no params)
    ResourcesList,
    /// MCP resources/read method
    ResourcesRead(ResourceReadParams),
    /// MCP prompts/list method (no params)
    PromptsList,
    /// MCP prompts/get method
    PromptsGet(PromptGetParams),
    /// Notification methods
    Notification(NotificationMessage),
}

/// Initialize method parameters
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: Value,
    pub client_info: Option<Value>,
}

/// Tool call parameters
#[derive(Debug, Deserialize, Serialize)]  
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}

/// Resource read parameters
#[derive(Debug, Deserialize, Serialize)]
pub struct ResourceReadParams {
    pub uri: String,
}

/// Prompt get parameters
#[derive(Debug, Deserialize, Serialize)]
pub struct PromptGetParams {
    pub name: String,
    pub arguments: Option<Value>,
}

/// Notification message
#[derive(Debug, Deserialize, Serialize)]
pub struct NotificationMessage {
    pub method: String,
    pub params: Option<Value>,
}

impl<'a> RawMessage<'a> {
    /// Parse from bytes without UTF-8 validation
    /// 
    /// This is more efficient than parsing from a string since it
    /// avoids the UTF-8 validation step that `from_str` performs.
    pub fn from_slice(bytes: &'a [u8]) -> McpResult<Self> {
        serde_json::from_slice(bytes).map_err(McpError::Json)
    }
    
    /// Validate the raw message structure
    /// 
    /// Performs early validation before parsing parameters
    pub fn validate(&self) -> McpResult<()> {
        // Validate JSON-RPC version
        if self.jsonrpc != "2.0" {
            return Err(McpError::InvalidParams(
                format!("Invalid jsonrpc version: {}", self.jsonrpc)
            ));
        }
        
        // Method name validation is implicit (non-empty string)
        if self.method.is_empty() {
            return Err(McpError::InvalidParams("Empty method name".into()));
        }
        
        Ok(())
    }
    
    /// Parse parameters based on method name
    /// 
    /// This performs type-safe parsing of the params field based on
    /// the method, avoiding multiple parsing passes.
    pub fn parse_params(self) -> McpResult<ParsedMessage> {
        // First validate the message structure
        self.validate()?;
        
        match self.method {
            "initialize" => {
                let params = self.params
                    .ok_or_else(|| McpError::InvalidParams("Missing params for initialize".into()))?;
                let parsed: InitializeParams = serde_json::from_str(params.get())
                    .map_err(|e| McpError::InvalidParams(format!("Invalid initialize params: {}", e)))?;
                Ok(ParsedMessage::Initialize(parsed))
            }
            "tools/list" => {
                // tools/list doesn't require params
                Ok(ParsedMessage::ToolsList)
            }
            "tools/call" => {
                let params = self.params
                    .ok_or_else(|| McpError::InvalidParams("Missing params for tools/call".into()))?;
                let parsed: ToolCallParams = serde_json::from_str(params.get())
                    .map_err(|e| McpError::InvalidParams(format!("Invalid tools/call params: {}", e)))?;
                Ok(ParsedMessage::ToolsCall(parsed))
            }
            "resources/list" => {
                Ok(ParsedMessage::ResourcesList)
            }
            "resources/read" => {
                let params = self.params
                    .ok_or_else(|| McpError::InvalidParams("Missing params for resources/read".into()))?;
                let parsed: ResourceReadParams = serde_json::from_str(params.get())
                    .map_err(|e| McpError::InvalidParams(format!("Invalid resources/read params: {}", e)))?;
                Ok(ParsedMessage::ResourcesRead(parsed))
            }
            "prompts/list" => {
                Ok(ParsedMessage::PromptsList)
            }
            "prompts/get" => {
                let params = self.params
                    .ok_or_else(|| McpError::InvalidParams("Missing params for prompts/get".into()))?;
                let parsed: PromptGetParams = serde_json::from_str(params.get())
                    .map_err(|e| McpError::InvalidParams(format!("Invalid prompts/get params: {}", e)))?;
                Ok(ParsedMessage::PromptsGet(parsed))
            }
            method if method.starts_with("notifications/") => {
                let notification = NotificationMessage {
                    method: method.to_string(),
                    params: self.params.map(|p| serde_json::from_str(p.get()).unwrap_or(Value::Null)),
                };
                Ok(ParsedMessage::Notification(notification))
            }
            _ => Err(McpError::UnknownMethod(self.method.to_string()))
        }
    }
    
    /// Get the request ID as a string
    /// 
    /// This handles the conversion from RawValue to a consistent string format
    pub fn get_id_string(&self) -> Option<String> {
        self.id.map(|raw_id| {
            // Try to parse as different types and convert to string
            if let Ok(s) = serde_json::from_str::<String>(raw_id.get()) {
                s
            } else if let Ok(n) = serde_json::from_str::<i64>(raw_id.get()) {
                n.to_string()
            } else {
                // Fallback: use the raw JSON
                raw_id.get().to_string()
            }
        })
    }
    
    /// Get the raw ID value for response building
    pub fn get_id_value(&self) -> Option<Value> {
        self.id.and_then(|raw_id| serde_json::from_str(raw_id.get()).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_message_parsing() {
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"foo":"bar"}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        
        assert_eq!(raw.jsonrpc, "2.0");
        assert_eq!(raw.method, "test");
        assert!(raw.params.is_some());
        assert_eq!(raw.get_id_string(), Some("1".to_string()));
    }

    #[test]
    fn test_initialize_parsing() {
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test"}}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let parsed = raw.parse_params().unwrap();
        
        match parsed {
            ParsedMessage::Initialize(params) => {
                assert_eq!(params.protocol_version, "2025-06-18");
                assert!(params.client_info.is_some());
            }
            _ => panic!("Expected Initialize message"),
        }
    }

    #[test]
    fn test_tools_call_parsing() {
        let msg = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"test_tool","arguments":{"input":"hello"}}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let parsed = raw.parse_params().unwrap();
        
        match parsed {
            ParsedMessage::ToolsCall(params) => {
                assert_eq!(params.name, "test_tool");
                assert_eq!(params.arguments["input"], "hello");
            }
            _ => panic!("Expected ToolsCall message"),
        }
    }

    #[test]
    fn test_validation_error() {
        let msg = r#"{"jsonrpc":"1.0","id":1,"method":"test","params":{}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let result = raw.parse_params();
        
        assert!(result.is_err());
        if let Err(McpError::InvalidParams(msg)) = result {
            assert!(msg.contains("Invalid jsonrpc version"));
        } else {
            panic!("Expected InvalidParams error");
        }
    }

    #[test]
    fn test_tools_list_no_params() {
        let msg = r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let parsed = raw.parse_params().unwrap();
        
        matches!(parsed, ParsedMessage::ToolsList);
    }

    #[test]
    fn test_unknown_method() {
        let msg = r#"{"jsonrpc":"2.0","id":4,"method":"unknown/method","params":{}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let result = raw.parse_params();
        
        assert!(result.is_err());
        matches!(result.unwrap_err(), McpError::UnknownMethod(_));
    }
}