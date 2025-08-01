//! Core types for the MCP protocol
//!
//! This module contains all the shared type definitions used throughout the library.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Definition of a tool that can be called
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// JSON Schema defining the tool's input parameters
    pub input_schema: Value,
}

/// Definition of a resource that can be accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDefinition {
    /// The URI of the resource
    pub uri: String,
    /// A human-readable name for the resource
    pub name: String,
    /// A description of the resource
    pub description: String,
    /// The MIME type of the resource content
    pub mime_type: String,
}

/// Definition of a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDefinition {
    /// The name of the prompt
    pub name: String,
    /// A description of what the prompt does
    pub description: String,
    /// The arguments this prompt accepts
    pub arguments: Vec<PromptArgument>,
}

/// An argument for a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// The name of the argument
    pub name: String,
    /// A description of the argument
    pub description: String,
    /// Whether this argument is required
    pub required: bool,
}

/// Content types for prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptContent {
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

/// A message in a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// The role of the message sender
    pub role: String,
    /// The content of the message
    pub content: PromptContent,
}

/// Information about a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptInfo {
    /// The name of the prompt
    pub name: String,
    /// A description of the prompt
    pub description: Option<String>,
    /// Arguments for the prompt
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Content of a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    /// The URI of the resource
    pub uri: String,
    /// The MIME type of the content
    pub mime_type: Option<String>,
    /// The actual content
    pub text: Option<String>,
    /// Base64-encoded binary content
    pub blob: Option<String>,
}

/// Information about a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    /// The URI of the resource
    pub uri: String,
    /// The name of the resource
    pub name: String,
    /// A description of the resource
    pub description: Option<String>,
    /// The MIME type of the resource
    pub mime_type: Option<String>,
}

// Re-export from handler module for backward compatibility
pub use crate::handler::{LogLevel, McpContext, McpNotification, TypedToolDefinition};