//! Response types for MCP operations
//!
//! This module provides type-safe response wrappers for tool execution and other MCP operations.

use serde::Serialize;
use serde_json::Value;
use crate::error::{McpError, McpResult};

/// Response from a tool execution
#[derive(Debug, Clone)]
pub struct ToolResponse {
    /// The content of the response
    pub content: Vec<ToolContent>,
    /// Whether this is an error response
    pub is_error: bool,
}

/// Content types for tool responses
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent {
    /// Text content
    Text { text: String },
    /// Image content
    Image { 
        data: String,
        mime_type: String,
    },
    /// Resource reference
    Resource {
        uri: String,
    },
}

impl ToolResponse {
    /// Create a successful text response
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: false,
        }
    }
    
    /// Create a successful response with custom content
    pub fn with_content(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: false,
        }
    }
    
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: message.into() }],
            is_error: true,
        }
    }
    
    /// Convert to JSON value
    pub fn to_value(&self) -> McpResult<Value> {
        serde_json::to_value(&self.content)
            .map_err(|e| McpError::Json(e))
    }
}

/// Type-safe response wrapper for structured data
#[derive(Debug, Clone)]
pub struct TypedResponse<T> {
    /// The wrapped data
    pub data: T,
}

impl<T: Serialize> TypedResponse<T> {
    /// Create a new typed response
    pub fn new(data: T) -> Self {
        Self { data }
    }
    
    /// Convert to JSON value
    pub fn to_value(&self) -> McpResult<Value> {
        serde_json::to_value(&self.data)
            .map_err(|e| McpError::Json(e))
    }
    
    /// Convert to tool response
    pub fn to_tool_response(&self) -> McpResult<ToolResponse> {
        let json = self.to_value()?;
        Ok(ToolResponse::success(json.to_string()))
    }
}

