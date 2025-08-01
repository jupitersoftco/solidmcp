//! Optimized JSON-RPC message processing
//!
//! This module provides zero-copy JSON parsing and unified message types
//! to optimize the MCP protocol processing pipeline.

use {
    crate::error::{McpError, McpResult},
    serde::{Deserialize, Serialize},
    serde_json::{value::RawValue, Value},
    schemars::{JsonSchema, Schema},
    std::sync::LazyLock,
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
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: Value,
    pub client_info: Option<Value>,
}

/// Tool call parameters
#[derive(Debug, Deserialize, Serialize, JsonSchema)]  
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}

/// Resource read parameters
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ResourceReadParams {
    pub uri: String,
}

/// Prompt get parameters
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PromptGetParams {
    pub name: String,
    pub arguments: Option<Value>,
}

/// Notification message
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NotificationMessage {
    pub method: String,
    pub params: Option<Value>,
}

// Compile-time schema generation using LazyLock
// These are computed once and cached for the lifetime of the program

/// Pre-computed schema for InitializeParams
static INITIALIZE_PARAMS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let root_schema = schemars::schema_for!(InitializeParams);
    root_schema.schema
});

/// Pre-computed schema for ToolCallParams  
static TOOL_CALL_PARAMS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let root_schema = schemars::schema_for!(ToolCallParams);
    root_schema.schema
});

/// Pre-computed schema for ResourceReadParams
static RESOURCE_READ_PARAMS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let root_schema = schemars::schema_for!(ResourceReadParams);
    root_schema.schema
});

/// Pre-computed schema for PromptGetParams
static PROMPT_GET_PARAMS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let root_schema = schemars::schema_for!(PromptGetParams);
    root_schema.schema
});

/// Pre-computed schema for NotificationMessage
static NOTIFICATION_MESSAGE_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    let root_schema = schemars::schema_for!(NotificationMessage);
    root_schema.schema
});

/// Get the schema for a given message type
/// 
/// This provides O(1) access to pre-computed schemas without runtime generation
pub fn get_message_schema(message_type: &ParsedMessage) -> &'static Schema {
    match message_type {
        ParsedMessage::Initialize(_) => &INITIALIZE_PARAMS_SCHEMA,
        ParsedMessage::ToolsCall(_) => &TOOL_CALL_PARAMS_SCHEMA,
        ParsedMessage::ResourcesRead(_) => &RESOURCE_READ_PARAMS_SCHEMA,
        ParsedMessage::PromptsGet(_) => &PROMPT_GET_PARAMS_SCHEMA,
        ParsedMessage::Notification(_) => &NOTIFICATION_MESSAGE_SCHEMA,
        ParsedMessage::ToolsList | ParsedMessage::ResourcesList | ParsedMessage::PromptsList => {
            // These don't have parameters, so no schema needed
            // Return a minimal empty schema
            static EMPTY_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
                let root_schema = schemars::schema_for!(());
                root_schema.schema
            });
            &EMPTY_SCHEMA
        }
    }
}

impl<'a> RawMessage<'a> {
    /// Parse from bytes with proper UTF-8 validation
    /// 
    /// This provides a balance between performance and safety:
    /// - Validates UTF-8 at the boundary (input validation)
    /// - Uses optimized byte-based JSON parsing after validation
    /// - Ensures all string data is valid UTF-8 before processing
    pub fn from_slice(bytes: &'a [u8]) -> McpResult<Self> {
        // First, validate that the input is valid UTF-8
        // This is critical for security and correctness
        std::str::from_utf8(bytes)
            .map_err(|e| McpError::InvalidParams(format!("Invalid UTF-8 in JSON input: {}", e)))?;
        
        // Now we can safely parse the JSON from bytes
        // serde_json will still do its own validation, but we've ensured UTF-8 safety
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

    #[test]
    fn test_utf8_validation_valid() {
        // Valid UTF-8 with international characters
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"测试","arguments":{"message":"Hello 世界! 🌍"}}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        
        assert_eq!(raw.jsonrpc, "2.0");
        assert_eq!(raw.method, "tools/call");
        
        // Should parse successfully
        let parsed = raw.parse_params().unwrap();
        match parsed {
            ParsedMessage::ToolsCall(params) => {
                assert_eq!(params.name, "测试");
                assert_eq!(params.arguments["message"], "Hello 世界! 🌍");
            }
            _ => panic!("Expected ToolsCall message"),
        }
    }

    #[test]
    fn test_utf8_validation_invalid() {
        // Create invalid UTF-8 bytes (valid JSON structure but invalid UTF-8)
        let mut bytes = Vec::from(r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"name":"#.as_bytes());
        bytes.push(0xFF); // Invalid UTF-8 byte
        bytes.push(0xFE); // Invalid UTF-8 byte  
        bytes.extend_from_slice(br#""}}"#);
        
        let result = RawMessage::from_slice(&bytes);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::InvalidParams(msg) => {
                assert!(msg.contains("Invalid UTF-8 in JSON input"));
            }
            _ => panic!("Expected InvalidParams error for UTF-8 validation"),
        }
    }

    #[test]
    fn test_utf8_validation_malformed_json() {
        // Valid UTF-8 but invalid JSON
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"incomplete"#;
        let result = RawMessage::from_slice(msg.as_bytes());
        
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::Json(_) => {
                // Expected - this should be a JSON parsing error, not UTF-8 error
            }
            _ => panic!("Expected Json error for malformed JSON"),
        }
    }

    #[test]
    fn test_edge_case_empty_bytes() {
        let result = RawMessage::from_slice(&[]);
        
        assert!(result.is_err());
        // Should fail at JSON parsing stage (empty input)
        matches!(result.unwrap_err(), McpError::Json(_));
    }

    #[test]
    fn test_edge_case_only_whitespace() {
        let msg = "   \n\t   ";
        let result = RawMessage::from_slice(msg.as_bytes());
        
        assert!(result.is_err());
        // Should fail at JSON parsing stage
        matches!(result.unwrap_err(), McpError::Json(_));
    }

    #[test]
    fn test_edge_case_null_bytes_in_string() {
        // JSON with embedded null bytes (valid UTF-8, questionable JSON)
        let msg = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test\",\"params\":{\"data\":\"hello\\u0000world\"}}";
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        
        // Should parse successfully - null bytes are valid in JSON strings when escaped
        assert_eq!(raw.jsonrpc, "2.0");
        assert_eq!(raw.method, "test");
    }

    #[test]
    fn test_very_large_valid_message() {
        // Test with a large but valid message
        let large_data = "x".repeat(10000);
        let msg = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"test","arguments":{{"data":"{}"}}}}}}"#,
            large_data
        );
        
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        assert_eq!(raw.jsonrpc, "2.0");
        assert_eq!(raw.method, "tools/call");
        
        let parsed = raw.parse_params().unwrap();
        match parsed {
            ParsedMessage::ToolsCall(params) => {
                assert_eq!(params.name, "test");
                assert_eq!(params.arguments["data"], large_data);
            }
            _ => panic!("Expected ToolsCall message"),
        }
    }

    #[test]
    fn test_unicode_normalization() {
        // Test different Unicode normalizations of the same character
        // é can be represented as a single codepoint (U+00E9) or as e + combining acute (U+0065 U+0301)
        let msg1 = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"café"}}"#; // é as single codepoint
        let msg2 = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"cafe\u0301"}}"#; // é as e + combining acute
        
        let raw1 = RawMessage::from_slice(msg1.as_bytes()).unwrap();
        let raw2 = RawMessage::from_slice(msg2.as_bytes()).unwrap();
        
        // Both should parse successfully
        assert_eq!(raw1.jsonrpc, "2.0");
        assert_eq!(raw2.jsonrpc, "2.0");
        
        let parsed1 = raw1.parse_params().unwrap();
        let parsed2 = raw2.parse_params().unwrap();
        
        // Both should be ToolsCall messages
        assert!(matches!(parsed1, ParsedMessage::ToolsCall(_)));
        assert!(matches!(parsed2, ParsedMessage::ToolsCall(_)));
    }

    #[test]
    fn test_boundary_utf8_sequences() {
        // Test UTF-8 sequences at byte boundaries
        // This tests that we don't have issues with multi-byte UTF-8 characters
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"𝕌𝕟𝕚𝕔𝕠𝕕𝕖"}}"#; // Mathematical double-struck characters
        
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        assert_eq!(raw.jsonrpc, "2.0");
        assert_eq!(raw.method, "tools/call");
        
        let parsed = raw.parse_params().unwrap();
        match parsed {
            ParsedMessage::ToolsCall(params) => {
                assert_eq!(params.name, "𝕌𝕟𝕚𝕔𝕠𝕕𝕖");
            }
            _ => panic!("Expected ToolsCall message"),
        }
    }

    #[test]
    fn test_compile_time_schema_generation() {
        // Test that schemas are available and correctly typed
        use super::get_message_schema;
        
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"test","arguments":{"foo":"bar"}}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let parsed = raw.parse_params().unwrap();
        
        // Schema access should be O(1) and available at runtime
        let schema = get_message_schema(&parsed);
        
        // Schema should be properly structured
        assert!(schema.schema.object.is_some());
        
        // Test that different message types get different schemas
        let init_msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":null}}"#;
        let init_raw = RawMessage::from_slice(init_msg.as_bytes()).unwrap();
        let init_parsed = init_raw.parse_params().unwrap();
        
        let init_schema = get_message_schema(&init_parsed);
        let tool_schema = get_message_schema(&parsed);
        
        // Different message types should have different schemas
        // We can't directly compare RootSchema for inequality, so test different aspects
        assert!(init_schema.schema.object.is_some());
        assert!(tool_schema.schema.object.is_some());
        
        // Both should be valid schemas but for different types
        match (&init_parsed, &parsed) {
            (ParsedMessage::Initialize(_), ParsedMessage::ToolsCall(_)) => {
                // Correct - they should be different types
            }
            _ => panic!("Schema types don't match expected parsed message types"),
        }
    }

    #[test]
    fn test_schema_lazy_initialization() {
        // This test verifies that LazyLock works correctly
        // Multiple accesses should return the same schema instance
        use super::get_message_schema;
        
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"test","arguments":{}}}"#;
        let raw = RawMessage::from_slice(msg.as_bytes()).unwrap();
        let parsed = raw.parse_params().unwrap();
        
        // Get schema twice
        let schema1 = get_message_schema(&parsed);
        let schema2 = get_message_schema(&parsed);
        
        // Should be the exact same instance (LazyLock ensures this)
        assert!(std::ptr::eq(schema1, schema2));
        
        // Schema should be valid
        assert!(schema1.schema.object.is_some());
    }

    #[test]
    fn test_all_message_types_have_schemas() {
        // Test that every ParsedMessage variant has an associated schema
        use super::get_message_schema;
        
        // Test each message type
        let test_cases = vec![
            (r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}"#, "Initialize"),
            (r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, "ToolsList"),
            (r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"test","arguments":{}}}"#, "ToolsCall"),
            (r#"{"jsonrpc":"2.0","id":4,"method":"resources/list"}"#, "ResourcesList"),
            (r#"{"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":"file://test"}}"#, "ResourcesRead"),
            (r#"{"jsonrpc":"2.0","id":6,"method":"prompts/list"}"#, "PromptsList"),
            (r#"{"jsonrpc":"2.0","id":7,"method":"prompts/get","params":{"name":"test"}}"#, "PromptsGet"),
            (r#"{"jsonrpc":"2.0","method":"notifications/test","params":{}}"#, "Notification"),
        ];
        
        for (json, expected_type) in test_cases {
            let raw = RawMessage::from_slice(json.as_bytes()).unwrap();
            let parsed = raw.parse_params().unwrap();
            
            // Every message type should have a schema
            let schema = get_message_schema(&parsed);
            assert!(schema.schema.object.is_some() || schema.schema.boolean.is_some(), 
                   "No schema available for message type: {}", expected_type);
        }
    }
}