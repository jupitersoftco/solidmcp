//! Type-safe MCP content types for tool responses
//!
//! This module provides compile-time safe wrappers for MCP tool responses to ensure
//! they follow the MCP protocol format. This prevents issues where raw data is returned
//! instead of properly formatted MCP responses.
//!
//! # Problem Solved
//!
//! Previously, tools could return raw JSON objects which would cause MCP clients like
//! Claude Code to report "no results found" even when data was successfully retrieved.
//! These types ensure all responses follow the MCP protocol's expected format.
//!
//! # Usage
//!
//! ```rust
//! use solidmcp::content_types::{McpResponse, McpContent};
//! use serde_json::json;
//!
//! // Instead of returning raw data:
//! // Ok(json!({"results": [...], "total": 10}))
//!
//! // Return type-safe MCP response:
//! Ok(McpResponse::with_text_and_data(
//!     "Found 10 results for your search query",
//!     json!({"results": [...], "total": 10})
//! ))
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Type-safe representation of MCP content
///
/// This enum ensures that tool responses contain the proper content structure
/// expected by MCP clients. Each variant represents a different content type
/// that can be included in an MCP response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum McpContent {
    /// Plain text content
    #[serde(rename = "text")]
    Text { text: String },
    
    /// Image content (base64 encoded)
    #[serde(rename = "image")]
    Image { 
        /// Base64 encoded image data
        data: String,
        /// MIME type (e.g., "image/png", "image/jpeg")
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    
    /// Resource reference
    #[serde(rename = "resource")]
    Resource { 
        /// Resource URI
        uri: String,
        /// Optional MIME type
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        /// Optional display text
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
}

impl McpContent {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
    
    /// Create image content
    pub fn image(data: impl Into<String>, mime_type: Option<String>) -> Self {
        Self::Image { 
            data: data.into(), 
            mime_type 
        }
    }
    
    /// Create resource reference
    pub fn resource(uri: impl Into<String>, mime_type: Option<String>, text: Option<String>) -> Self {
        Self::Resource { 
            uri: uri.into(), 
            mime_type, 
            text 
        }
    }
}

/// Complete MCP-compliant tool response
///
/// This struct ensures that tool responses follow the MCP protocol format with
/// a `content` array for client display and optional `data` field for structured
/// programmatic access.
///
/// # MCP Protocol Compliance
///
/// - `content`: Array of content items for display (required by MCP spec)
/// - `data`: Optional structured data for programmatic access
/// - `is_error`: Whether this represents an error response
///
/// # Examples
///
/// ```rust
/// use solidmcp::content_types::{McpResponse, McpContent};
/// use serde_json::json;
///
/// // Simple text response
/// let response = McpResponse::text("File processed successfully");
///
/// // Text with structured data
/// let response = McpResponse::with_text_and_data(
///     "Found 3 results",
///     json!({
///         "results": [
///             {"id": "1", "title": "First result"},
///             {"id": "2", "title": "Second result"},
///             {"id": "3", "title": "Third result"}
///         ],
///         "total": 3
///     })
/// );
///
/// // Multiple content items
/// let response = McpResponse::new(vec![
///     McpContent::text("Search completed"),
///     McpContent::resource("file://results.json", Some("application/json".to_string()), None)
/// ]);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpResponse {
    /// Content array for client display (MCP protocol requirement)
    pub content: Vec<McpContent>,
    
    /// Optional structured data for programmatic access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    
    /// Whether this is an error response
    #[serde(default)]
    pub is_error: bool,
}

impl McpResponse {
    /// Create a new MCP response with the given content
    pub fn new(content: Vec<McpContent>) -> Self {
        Self {
            content,
            data: None,
            is_error: false,
        }
    }
    
    /// Create a simple text response
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(vec![McpContent::text(text)])
    }
    
    /// Create a text response with structured data
    /// 
    /// This is the most common pattern for search results and similar tools
    /// where you want both human-readable summary and programmatic access.
    pub fn with_text_and_data(text: impl Into<String>, data: Value) -> Self {
        Self {
            content: vec![McpContent::text(text)],
            data: Some(data),
            is_error: false,
        }
    }
    
    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::text(message)],
            data: None,
            is_error: true,
        }
    }
    
    /// Add structured data to this response
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
    
    /// Add additional content to this response
    pub fn with_content(mut self, content: McpContent) -> Self {
        self.content.push(content);
        self
    }
    
    /// Mark this response as an error
    pub fn as_error(mut self) -> Self {
        self.is_error = true;
        self
    }
}

/// Helper trait for converting types to MCP responses
///
/// This trait provides a convenient way to convert common types into
/// MCP-compliant responses without manual wrapping.
pub trait ToMcpResponse {
    /// Convert this type into an MCP response
    fn to_mcp_response(self) -> McpResponse;
}

impl ToMcpResponse for String {
    fn to_mcp_response(self) -> McpResponse {
        McpResponse::text(self)
    }
}

impl ToMcpResponse for &str {
    fn to_mcp_response(self) -> McpResponse {
        McpResponse::text(self)
    }
}

impl ToMcpResponse for Value {
    fn to_mcp_response(self) -> McpResponse {
        // Try to extract a reasonable text summary from the JSON
        let text = if let Some(message) = self.get("message").and_then(|v| v.as_str()) {
            message.to_string()
        } else if let Some(status) = self.get("status").and_then(|v| v.as_str()) {
            status.to_string()
        } else if let Some(results) = self.get("results").and_then(|v| v.as_array()) {
            format!("Operation completed with {} results", results.len())
        } else {
            "Operation completed".to_string()
        };
        
        McpResponse::with_text_and_data(text, self)
    }
}

impl ToMcpResponse for McpResponse {
    fn to_mcp_response(self) -> McpResponse {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_content_text() {
        let content = McpContent::text("Hello, world!");
        let json = serde_json::to_value(&content).unwrap();
        
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello, world!");
    }

    #[test]
    fn test_mcp_content_image() {
        let content = McpContent::image("base64data", Some("image/png".to_string()));
        let json = serde_json::to_value(&content).unwrap();
        
        assert_eq!(json["type"], "image");
        assert_eq!(json["data"], "base64data");
        assert_eq!(json["mime_type"], "image/png");
    }

    #[test]
    fn test_mcp_response_text() {
        let response = McpResponse::text("Success");
        let json = serde_json::to_value(&response).unwrap();
        
        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][0]["text"], "Success");
        assert_eq!(json["is_error"], false);
        assert!(json["data"].is_null());
    }

    #[test]
    fn test_mcp_response_with_data() {
        let data = json!({"results": [1, 2, 3], "total": 3});
        let response = McpResponse::with_text_and_data("Found 3 results", data.clone());
        let json = serde_json::to_value(&response).unwrap();
        
        assert_eq!(json["content"][0]["text"], "Found 3 results");
        assert_eq!(json["data"], data);
    }

    #[test]
    fn test_to_mcp_response_string() {
        let response = "Hello".to_mcp_response();
        assert_eq!(response.content.len(), 1);
        assert!(matches!(response.content[0], McpContent::Text { ref text } if text == "Hello"));
    }

    #[test]
    fn test_to_mcp_response_json_with_results() {
        let data = json!({"results": [1, 2], "status": "ok"});
        let response = data.to_mcp_response();
        
        // Should extract meaningful text from JSON
        let content_json = serde_json::to_value(&response.content[0]).unwrap();
        let text = content_json["text"].as_str().unwrap();
        assert!(text.contains("2 results") || text.contains("ok"));
        assert!(response.data.is_some());
    }
}