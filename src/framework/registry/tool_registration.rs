//! Tool registration methods for ToolRegistry.
//!
//! This module contains all tool registration methods including register_tool,
//! register_tool_with_schemas, and register_typed_tool.

use crate::{
    content_types::McpResponse,
    handler::ToolDefinition,
    typed_response::McpToolOutput,
};
use anyhow::Result;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::{future::Future, sync::Arc};

use crate::framework::{
    notification::NotificationCtx,
    registry::{ToolFunction, ToolRegistry},
};

impl<C: Send + Sync + 'static> ToolRegistry<C> {
    /// Register a tool function with type-safe MCP response format.
    ///
    /// This method ensures all tool responses follow the MCP protocol format by requiring
    /// tools to return `McpResponse`. This prevents issues where raw data is returned
    /// instead of properly formatted MCP responses that clients like Claude Code expect.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `F`: Handler function type
    /// - `Fut`: Future returned by the handler function
    ///
    /// # Parameters
    /// - `name`: Unique name for the tool (used by clients to invoke it)
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that returns an `McpResponse`
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::{McpResponse, McpContent, JsonSchema};
    /// use serde::Deserialize;
    /// use serde_json::json;
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     limit: Option<u32>,
    /// }
    ///
    /// registry.register_tool("search", "Search the knowledge base", |input: SearchInput, ctx, notif| async move {
    ///     notif.info(&format!("Searching for: {}", input.query))?;
    ///     
    ///     let results = ctx.database.search(&input.query).await?;
    ///     
    ///     // Type-safe MCP response - prevents "no results found" issues
    ///     Ok(McpResponse::with_text_and_data(
    ///         format!("Found {} results for '{}'", results.len(), input.query),
    ///         json!({
    ///             "results": results,
    ///             "query": input.query,
    ///             "total": results.len()
    ///         })
    ///     ))
    /// });
    /// ```
    pub fn register_tool<I, F, Fut>(&mut self, name: &str, description: &str, handler: F)
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<McpResponse>> + Send + 'static,
    {
        let tool_def = ToolDefinition::from_schema::<I>(name, description);
        let handler = Arc::new(handler);

        let wrapper: ToolFunction<C> = Box::new(move |args, context, notification_ctx| {
            let handler = Arc::clone(&handler);

            Box::pin(async move {
                // Parse and validate input
                let input: I = serde_json::from_value(args)?;

                // Call the handler with clean API
                let mcp_response = handler(input, context, notification_ctx).await?;

                // Convert McpResponse to JSON - this ensures MCP protocol compliance
                Ok(serde_json::to_value(mcp_response)?)
            })
        });

        self.tools.insert(name.to_string(), (tool_def, wrapper));
    }

    /// Register a tool with both input and output schema types.
    ///
    /// This method provides full type safety for both input and output of tools,
    /// automatically generating JSON schemas for validation on both ends.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `O`: Output type (must implement `JsonSchema` and `Serialize`)
    /// - `F`: Handler function type
    /// - `Fut`: Future returned by the handler function
    ///
    /// # Parameters
    /// - `name`: Unique name for the tool
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that takes input type I and returns output type O
    ///
    /// # Examples
    /// ```rust
    /// use schemars::JsonSchema;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct CalculateInput {
    ///     a: f64,
    ///     b: f64,
    ///     operation: String,
    /// }
    ///
    /// #[derive(JsonSchema, Serialize)]
    /// struct CalculateOutput {
    ///     result: f64,
    ///     formula: String,
    /// }
    ///
    /// registry.register_tool_with_schemas("calculate", "Perform calculations", |input: CalculateInput, ctx, notif| async move {
    ///     let result = match input.operation.as_str() {
    ///         "add" => input.a + input.b,
    ///         "multiply" => input.a * input.b,
    ///         _ => return Err(anyhow::anyhow!("Unknown operation")),
    ///     };
    ///     
    ///     Ok(CalculateOutput {
    ///         result,
    ///         formula: format!("{} {} {} = {}", input.a, input.operation, input.b, result),
    ///     })
    /// });
    /// ```
    pub fn register_tool_with_schemas<I, O, F, Fut>(&mut self, name: &str, description: &str, handler: F)
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: JsonSchema + serde::Serialize + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O>> + Send + 'static,
    {
        let tool_def = ToolDefinition::from_schemas::<I, O>(name, description);
        let handler = Arc::new(handler);

        let wrapper: ToolFunction<C> = Box::new(move |args, context, notification_ctx| {
            let handler = Arc::clone(&handler);

            Box::pin(async move {
                // Parse and validate input
                let input: I = serde_json::from_value(args)?;

                // Call the handler
                let output = handler(input, context, notification_ctx).await?;

                // Serialize output - this is already validated against the schema
                Ok(serde_json::to_value(output)?)
            })
        });

        self.tools.insert(name.to_string(), (tool_def, wrapper));
    }

    /// Register a tool with enforced typed output.
    ///
    /// This method ensures maximum type safety by requiring:
    /// 1. Input type with JSON schema (for validation)
    /// 2. Output type that implements `McpToolOutput` (for rich context)
    /// 3. Automatic schema registration for both input and output
    ///
    /// This is the recommended way to register tools as it provides the richest
    /// context to LLMs and prevents common errors like returning raw JSON.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `O`: Output type (must implement `McpToolOutput`)
    /// - `F`: Handler function type
    /// - `Fut`: Future returned by the handler function
    ///
    /// # Parameters
    /// - `name`: Unique name for the tool
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that takes typed input and returns typed output
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::typed_response::McpToolOutput;
    /// use schemars::JsonSchema;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     limit: Option<u32>,
    /// }
    ///
    /// #[derive(Debug, Serialize, JsonSchema)]
    /// struct SearchOutput {
    ///     query: String,
    ///     results: Vec<SearchResult>,
    ///     total_count: usize,
    /// }
    ///
    /// impl McpToolOutput for SearchOutput {
    ///     fn to_mcp_response(&self) -> McpResponse {
    ///         let summary = format!("Found {} results for '{}'", self.total_count, self.query);
    ///         McpResponse::with_text_and_data(summary, serde_json::to_value(self).unwrap())
    ///     }
    ///     
    ///     fn output_schema() -> OutputSchema {
    ///         OutputSchema {
    ///             name: "SearchOutput",
    ///             description: "Search results with metadata",
    ///             schema: Self::json_schema(),
    ///         }
    ///     }
    /// }
    ///
    /// registry.register_typed_tool("search", "Search the knowledge base", |input: SearchInput, ctx, notif| async move {
    ///     let results = ctx.search_engine.search(&input.query).await?;
    ///     
    ///     Ok(SearchOutput {
    ///         query: input.query,
    ///         total_count: results.len(),
    ///         results,
    ///     })
    /// });
    /// ```
    pub fn register_typed_tool<I, O, F, Fut>(&mut self, name: &str, description: &str, handler: F)
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: McpToolOutput + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O>> + Send + 'static,
    {
        // Generate tool definition with input and output schemas
        let mut tool_def = ToolDefinition::from_schema::<I>(name, description);
        
        // Store the output schema for discovery
        let output_schema = O::output_schema();
        let output_schema_desc = output_schema.description.to_string();
        self.output_schemas.insert(name.to_string(), output_schema);
        
        // Add output schema info to tool definition if possible
        // This helps LLMs understand what the tool returns
        if let Some(obj) = tool_def.input_schema.as_object() {
            let mut modified_schema = obj.clone();
            modified_schema.insert("_output_schema".to_string(), serde_json::json!({
                "description": output_schema_desc,
                "type": "object"
            }));
            tool_def.input_schema = serde_json::Value::Object(modified_schema);
        }
        
        let handler = Arc::new(handler);

        let wrapper: ToolFunction<C> = Box::new(move |args, context, notification_ctx| {
            let handler = Arc::clone(&handler);

            Box::pin(async move {
                // Parse and validate input
                let input: I = serde_json::from_value(args)?;

                // Call the handler to get typed output
                let output = handler(input, context, notification_ctx).await?;

                // Convert to MCP response using the trait method
                let mcp_response = output.to_mcp_response();

                // Return as JSON value for protocol compatibility
                Ok(serde_json::to_value(mcp_response)?)
            })
        });

        self.tools.insert(name.to_string(), (tool_def, wrapper));
    }
}