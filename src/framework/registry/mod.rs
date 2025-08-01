//! Tool registry for managing tools, resources, and prompts.
//!
//! This module provides the `ToolRegistry` struct which maintains collections
//! of registered functionality that can be exposed to MCP clients. It provides
//! type-safe registration methods with automatic schema generation.

mod provider_registration;
mod tool_registration;

use crate::{
    handler::ToolDefinition,
    typed_response::OutputSchema,
};
use serde_json::Value;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use super::{notification::NotificationCtx, providers::{ResourceProvider, PromptProvider}};


/// Type alias for tool handler functions.
///
/// This represents a boxed, async function that can be called by the MCP client.
/// Tool functions receive validated input parameters, application context, and
/// a notification context for sending updates back to the client.
///
/// # Type Parameters
/// - `C`: The application context type (shared across all tools)
///
/// # Function Signature
/// - `Value`: JSON input parameters (already validated against the tool's schema)
/// - `Arc<C>`: Shared reference to application context
/// - `NotificationCtx`: Context for sending notifications
/// - Returns: `Pin<Box<dyn Future<Output = Result<Value>>>>` - Async result with JSON output
pub type ToolFunction<C> = Box<
    dyn Fn(Value, Arc<C>, NotificationCtx) -> Pin<Box<dyn Future<Output = anyhow::Result<Value>> + Send>>
        + Send
        + Sync,
>;

/// Registry for managing tools, resources, and prompts within a server instance.
///
/// This struct maintains collections of registered functionality that can be
/// exposed to MCP clients. It provides type-safe registration methods with
/// automatic schema generation.
///
/// # Type Parameters
/// - `C`: The application context type shared across all registered handlers
pub struct ToolRegistry<C> {
    pub(crate) tools: HashMap<String, (ToolDefinition, ToolFunction<C>)>,
    pub(crate) output_schemas: HashMap<String, OutputSchema>,
    pub(crate) resources: Vec<Box<dyn ResourceProvider<C>>>,
    pub(crate) prompts: Vec<Box<dyn PromptProvider<C>>>,
}

impl<C> Default for ToolRegistry<C> {
    fn default() -> Self {
        Self {
            tools: HashMap::new(),
            output_schemas: HashMap::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        }
    }
}

impl<C: Send + Sync + 'static> ToolRegistry<C> {
    /// Create a new, empty tool registry.
    ///
    /// # Returns
    /// A new `ToolRegistry` with no registered tools, resources, or prompts
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the output schema for a registered tool.
    ///
    /// This is useful for tool discovery and documentation generation.
    pub fn get_tool_output_schema(&self, name: &str) -> Option<&OutputSchema> {
        self.output_schemas.get(name)
    }
}