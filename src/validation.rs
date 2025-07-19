//! MCP Message Validation
//!
//! Lightweight validation for MCP protocol messages using serde_valid.

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
    tracing::{debug, error, info},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: Option<String>,

    pub method: Option<String>,

    pub id: Option<Value>,

    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: Option<String>,

    #[serde(default)]
    pub capabilities: Option<Value>,

    #[serde(default)]
    #[serde(rename = "clientInfo")]
    pub client_info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallParams {
    pub name: Option<String>,

    #[serde(default)]
    pub arguments: Option<Value>,
}

pub struct McpValidator;

impl McpValidator {
    /// Validate a raw MCP message with detailed error reporting
    pub fn validate_message(message: &Value) -> Result<(), Vec<String>> {
        debug!("🔍 Validating MCP message structure: {:?}", message);

        // First, try to deserialize as a basic MCP message
        let mcp_message: McpMessage = match serde_json::from_value(message.clone()) {
            Ok(msg) => msg,
            Err(e) => {
                error!("❌ Failed to deserialize MCP message: {}", e);
                return Err(vec![format!("Invalid JSON structure: {}", e)]);
            }
        };

        // Validate required fields manually
        let mut errors = Vec::new();

        if mcp_message.jsonrpc.is_none() {
            errors.push("Missing required field: 'jsonrpc'".to_string());
        }

        if mcp_message.method.is_none() {
            errors.push("Missing required field: 'method'".to_string());
        }

        // Note: id is optional for notifications (messages without id are valid JSON-RPC 2.0 notifications)
        // We don't validate id presence here as it's optional

        if !errors.is_empty() {
            error!("❌ MCP message validation failed: {:?}", errors);
            return Err(errors);
        }

        // Validate jsonrpc version
        if let Some(jsonrpc) = &mcp_message.jsonrpc {
            if jsonrpc != "2.0" {
                error!("❌ Invalid jsonrpc version: {}", jsonrpc);
                return Err(vec![format!(
                    "Invalid jsonrpc version: {}. Expected: 2.0",
                    jsonrpc
                )]);
            }
        }

        // Validate method-specific parameters
        if let Some(method) = &mcp_message.method {
            match method.as_str() {
                "initialize" => {
                    // params must be present for initialize
                    if mcp_message.params.is_none() {
                        return Err(vec![
                            "Missing required field: 'params' for 'initialize' method".to_string(),
                        ]);
                    }
                    if let Some(params) = &mcp_message.params {
                        Self::validate_initialize_params(params)?
                    }
                }
                "tools/call" => {
                    // params must be present for tools/call
                    if mcp_message.params.is_none() {
                        return Err(vec![
                            "Missing required field: 'params' for 'tools/call' method".to_string(),
                        ]);
                    }
                    if let Some(params) = &mcp_message.params {
                        Self::validate_tool_call_params(params)?
                    }
                }
                "tools/list" | "notifications/cancel" | "notifications/initialized" => {
                    // These methods don't require specific parameter validation
                    debug!("✅ Method '{}' parameters validated", method);
                }
                _ => {
                    // Unknown methods should be handled by the protocol layer, not rejected here
                    // The protocol handler will return a JSON-RPC error with code -32601
                    debug!(
                        "⚠️ Unknown MCP method: {} - will be handled by protocol layer",
                        method
                    );
                }
            }
        }

        info!("✅ MCP message validation passed");
        Ok(())
    }

    /// Validate initialize method parameters
    fn validate_initialize_params(params: &Value) -> Result<(), Vec<String>> {
        let init_params: McpInitializeParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                error!("❌ Failed to deserialize initialize params: {}", e);
                return Err(vec![format!("Invalid initialize parameters: {}", e)]);
            }
        };

        // Validate required fields manually
        let mut errors = Vec::new();

        if init_params.protocol_version.is_none() {
            errors.push("Missing required field: 'protocolVersion'".to_string());
        }

        if !errors.is_empty() {
            error!("❌ Initialize params validation failed: {:?}", errors);
            return Err(errors);
        }

        // Validate protocol version
        if let Some(version) = &init_params.protocol_version {
            let supported_versions = ["2025-03-26", "2025-06-18"];
            if !supported_versions.contains(&version.as_str()) {
                error!("❌ Unsupported protocol version: {}", version);
                return Err(vec![format!(
                    "Unsupported protocol version: {}. Supported versions: {:?}",
                    version, supported_versions
                )]);
            }
        }

        Ok(())
    }

    /// Validate tool call parameters
    fn validate_tool_call_params(params: &Value) -> Result<(), Vec<String>> {
        let tool_params: McpToolCallParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                error!("❌ Failed to deserialize tool call params: {}", e);
                return Err(vec![format!("Invalid tool call parameters: {}", e)]);
            }
        };

        // Validate required fields manually
        let mut errors = Vec::new();

        if tool_params.name.is_none() {
            errors.push("Missing required field: 'name'".to_string());
        }

        if !errors.is_empty() {
            error!("❌ Tool call params validation failed: {:?}", errors);
            return Err(errors);
        }

        // Validate tool name
        if let Some(name) = &tool_params.name {
            let valid_tools = ["echo", "read_file"];
            if !valid_tools.contains(&name.as_str()) {
                error!("❌ Unknown tool: {}", name);
                return Err(vec![format!(
                    "Unknown tool: {}. Available tools: {:?}",
                    name, valid_tools
                )]);
            }
        }

        Ok(())
    }

    /// Get detailed validation report for debugging
    pub fn get_validation_report(message: &Value) -> ValidationReport {
        let mut report = ValidationReport {
            is_valid: false,
            errors: Vec::new(),
            warnings: Vec::new(),
            message_structure: None,
            method_info: None,
        };

        // Try to extract basic info even if validation fails
        if let Some(jsonrpc) = message.get("jsonrpc").and_then(|v| v.as_str()) {
            report.message_structure = Some(format!("jsonrpc: {jsonrpc}"));
        }

        if let Some(method) = message.get("method").and_then(|v| v.as_str()) {
            report.method_info = Some(method.to_string());
        }

        // Perform validation
        match Self::validate_message(message) {
            Ok(()) => {
                report.is_valid = true;
                report.message_structure = Some("Valid MCP message structure".to_string());
            }
            Err(errors) => {
                report.errors = errors;
            }
        }

        report
    }
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub message_structure: Option<String>,
    pub method_info: Option<String>,
}

impl ValidationReport {
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(structure) = &self.message_structure {
            parts.push(format!("Structure: {structure}"));
        }

        if let Some(method) = &self.method_info {
            parts.push(format!("Method: {method}"));
        }

        if !self.errors.is_empty() {
            parts.push(format!("Errors: {}", self.errors.join("; ")));
        }

        if !self.warnings.is_empty() {
            parts.push(format!("Warnings: {}", self.warnings.join("; ")));
        }

        parts.join(" | ")
    }
}
