//! Tool registration methods for McpServerBuilder.
//!
//! This module contains all the tool-related builder methods including
//! with_tool, with_tool_schemas, and with_typed_tool.

use crate::{
    handler::ToolDefinition,
    tool_response::IntoToolResponse,
};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, sync::Arc};

use crate::framework::{
    builder::McpServerBuilder,
    notification::NotificationCtx,
    registry::ToolFunction,
};

impl<C: Send + Sync + 'static> McpServerBuilder<C> {
    /// Register a tool with automatic typed output conversion.
    ///
    /// This method ensures all tools return properly typed outputs that are automatically
    /// converted to MCP-compliant responses. Any type that implements `Serialize + JsonSchema`
    /// will be automatically converted to an `McpResponse` with appropriate text summary
    /// and structured data.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `O`: Output type (must implement `Serialize` and `JsonSchema`)
    /// - `F`: Handler function type (async closure or function)
    /// - `Fut`: Future type returned by the handler
    ///
    /// # Parameters
    /// - `name`: Unique tool name (used by clients to invoke the tool)
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that returns any type implementing `Serialize + JsonSchema`
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Error Handling
    /// Tool handlers should return `Result<O>` where O implements `Serialize + JsonSchema`.
    /// Any errors will be automatically converted to MCP protocol errors.
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::{McpServerBuilder, JsonSchema};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct SearchInput {
    ///     query: String,
    ///     limit: Option<u32>,
    /// }
    ///
    /// #[derive(JsonSchema, Serialize)]
    /// struct SearchResult {
    ///     results: Vec<Document>,
    ///     total_count: usize,
    ///     query: String,
    /// }
    ///
    /// let server = McpServerBuilder::new(AppContext::new(), "search-server", "1.0.0")
    ///     .with_tool("search", "Search the knowledge base", |input: SearchInput, ctx, notif| async move {
    ///         notif.info(&format!("Searching for: {}", input.query))?;
    ///         
    ///         let results = ctx.database.search(&input.query).await?;
    ///         
    ///         // Return typed output - automatically converted to McpResponse
    ///         Ok(SearchResult {
    ///             results,
    ///             total_count: results.len(),
    ///             query: input.query,
    ///         })
    ///     });
    /// ```
    pub fn with_tool<I, O, F, Fut>(mut self, name: &str, description: &str, handler: F) -> Self
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: Serialize + JsonSchema + Send + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = crate::error::McpResult<O>> + Send + 'static,
    {
        use crate::tool_response::IntoToolResponse;
        
        // Wrap the handler in an Arc to make it cloneable
        let handler = Arc::new(handler);
        
        // Wrap the handler to convert output to McpResponse
        let wrapped_handler = move |input: I, ctx: Arc<C>, notif: NotificationCtx| {
            let handler = Arc::clone(&handler);
            async move {
                let result = handler(input, ctx, notif).await?;
                Ok(result.into_tool_response())
            }
        };
        
        self.handler
            .registry_mut()
            .register_tool(name, description, wrapped_handler);
        self
    }

    /// Register a tool with both input and output schema types.
    ///
    /// This method provides full type safety for both input and output of tools,
    /// automatically generating JSON schemas for validation on both ends.
    ///
    /// # Type Parameters
    /// - `I`: Input type (must implement `JsonSchema` and `DeserializeOwned`)
    /// - `O`: Output type (must implement `JsonSchema` and `Serialize`)
    /// - `F`: Handler function type (async closure or function)
    /// - `Fut`: Future type returned by the handler
    ///
    /// # Parameters
    /// - `name`: Unique tool name (used by clients to invoke the tool)
    /// - `description`: Human-readable description of what the tool does
    /// - `handler`: Async function that takes input type I and returns output type O
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::{McpServerBuilder, JsonSchema};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(JsonSchema, Deserialize)]
    /// struct TranslateInput {
    ///     text: String,
    ///     from_language: String,
    ///     to_language: String,
    /// }
    ///
    /// #[derive(JsonSchema, Serialize)]
    /// struct TranslateOutput {
    ///     translated_text: String,
    ///     confidence: f32,
    ///     detected_language: Option<String>,
    /// }
    ///
    /// let server = McpServerBuilder::new(AppContext::new(), "translator", "1.0.0")
    ///     .with_tool_schemas("translate", "Translate text between languages", |input: TranslateInput, ctx, notif| async move {
    ///         notif.info(&format!("Translating from {} to {}", input.from_language, input.to_language))?;
    ///         
    ///         let translation = ctx.translator.translate(&input.text, &input.from_language, &input.to_language).await?;
    ///         
    ///         Ok(TranslateOutput {
    ///             translated_text: translation.text,
    ///             confidence: translation.confidence,
    ///             detected_language: translation.detected_language,
    ///         })
    ///     });
    /// ```
    pub fn with_tool_schemas<I, O, F, Fut>(mut self, name: &str, description: &str, handler: F) -> Self
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: JsonSchema + serde::Serialize + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = crate::error::McpResult<O>> + Send + 'static,
    {
        self.handler
            .registry_mut()
            .register_tool_with_schemas(name, description, handler);
        self
    }

    /// Register a tool with automatic type inference and conversion.
    ///
    /// This is the simplest and most idiomatic way to register tools. Just return
    /// any type that implements `Serialize + JsonSchema` and it will automatically
    /// be converted to the proper MCP response format.
    ///
    /// # Type Inference
    /// 
    /// Rust will infer the input and output types from your closure. You can
    /// explicitly specify the return type using block syntax for clarity.
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
    /// }
    ///
    /// #[derive(JsonSchema, Serialize)]
    /// struct CalculateOutput {
    ///     result: f64,
    ///     formula: String,
    /// }
    ///
    /// let server = McpServerBuilder::new(context, "calc", "1.0.0")
    ///     .with_typed_tool("add", "Add two numbers", |input: CalculateInput, ctx, notif| async move {
    ///         // Explicitly declare return type for clarity (optional)
    ///         let output: CalculateOutput = CalculateOutput {
    ///             result: input.a + input.b,
    ///             formula: format!("{} + {} = {}", input.a, input.b, input.a + input.b),
    ///         };
    ///         Ok(output)
    ///     })
    ///     .with_typed_tool("multiply", "Multiply two numbers", |input: CalculateInput, ctx, notif| async move {
    ///         // Or just return the type directly - Rust infers everything
    ///         Ok(CalculateOutput {
    ///             result: input.a * input.b,
    ///             formula: format!("{} × {} = {}", input.a, input.b, input.a * input.b),
    ///         })
    ///     })
    ///     .build()
    ///     .await?;
    /// ```
    pub fn with_typed_tool<I, O, F, Fut>(mut self, name: &str, description: &str, handler: F) -> Self
    where
        I: JsonSchema + DeserializeOwned + Send + 'static,
        O: IntoToolResponse + JsonSchema + serde::Serialize + 'static,
        F: Fn(I, Arc<C>, NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = crate::error::McpResult<O>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        
        // Generate tool definition with schemas
        let tool_def = ToolDefinition::from_schemas::<I, O>(name, description);
        
        // Create wrapper that converts the output type to McpResponse
        let wrapper: ToolFunction<C> = Box::new(move |args, context, notification_ctx| {
            let handler = Arc::clone(&handler);
            
            Box::pin(async move {
                // Parse input
                let input: I = serde_json::from_value(args)?;
                
                // Call handler
                let output = handler(input, context, notification_ctx).await?;
                
                // Convert to MCP response automatically
                let response = output.into_tool_response();
                
                // Return as JSON
                Ok(serde_json::to_value(response)?)
            })
        });
        
        self.handler.registry.tools.insert(name.to_string(), (tool_def, wrapper));
        self
    }
}