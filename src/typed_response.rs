//! Type-safe response system for MCP tools
//!
//! This module provides traits and types that enforce proper output typing
//! for MCP tools, ensuring rich context injection into LLMs.

use crate::content_types::McpResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Trait that all MCP tool outputs must implement.
///
/// This trait ensures that:
/// 1. Output types are properly typed (not raw JSON)
/// 2. Schemas are automatically derived for LLM context
/// 3. Responses include both human-readable and structured data
///
/// # Example
///
/// ```rust
/// use solidmcp::typed_response::{McpToolOutput, OutputSchema};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Debug, Serialize, JsonSchema)]
/// struct SearchResult {
///     query: String,
///     results: Vec<Document>,
///     total_count: usize,
///     search_time_ms: u64,
/// }
///
/// #[derive(Debug, Serialize, JsonSchema)]
/// struct Document {
///     id: String,
///     title: String,
///     snippet: String,
///     relevance_score: f32,
/// }
///
/// impl McpToolOutput for SearchResult {
///     fn to_mcp_response(&self) -> McpResponse {
///         let summary = format!(
///             "Found {} results for '{}' in {}ms",
///             self.total_count, self.query, self.search_time_ms
///         );
///         
///         McpResponse::with_text_and_data(summary, serde_json::to_value(self).unwrap())
///     }
///     
///     fn output_schema() -> OutputSchema {
///         OutputSchema {
///             name: "SearchResult",
///             description: "Results from searching the knowledge base",
///             schema: Self::json_schema(),
///         }
///     }
/// }
/// ```
pub trait McpToolOutput: Serialize + JsonSchema + Send + Sync {
    /// Convert this output to an MCP response.
    ///
    /// This method should:
    /// 1. Create a human-readable summary
    /// 2. Include the full structured data
    /// 3. Optionally add metadata or additional context
    fn to_mcp_response(&self) -> McpResponse;
    
    /// Get the output schema information.
    ///
    /// This is used for:
    /// 1. Tool discovery and documentation
    /// 2. LLM context about expected outputs
    /// 3. Client-side validation
    fn output_schema() -> OutputSchema
    where
        Self: Sized;
}

/// Schema information for a tool output type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSchema {
    /// Name of the output type
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// JSON Schema for the output
    pub schema: schemars::Schema,
}

/// A type-safe tool handler function.
///
/// This type alias enforces that tool handlers must:
/// 1. Accept typed input (not raw JSON)
/// 2. Return typed output (not raw JSON)
/// 3. Be async and return Result
pub type TypedToolHandler<I, O, C> = for<'a> fn(
    input: I,
    context: &'a C,
    notif: &'a crate::framework::NotificationCtx,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<O>> + Send + 'a>>;

/// Extension trait for registering typed tools
pub trait TypedToolRegistry<C: Clone + Send + Sync + 'static> {
    /// Register a tool with enforced input and output types.
    ///
    /// This method ensures:
    /// 1. Input type has a schema (via JsonSchema)
    /// 2. Output type implements McpToolOutput
    /// 3. Both schemas are registered for discovery
    ///
    /// # Example
    ///
    /// ```rust
    /// registry.register_typed_tool(
    ///     "search",
    ///     "Search the knowledge base",
    ///     |input: SearchInput, ctx, notif| async move {
    ///         let results = ctx.search_engine.search(&input.query).await?;
    ///         
    ///         Ok(SearchResult {
    ///             query: input.query,
    ///             results,
    ///             total_count: results.len(),
    ///             search_time_ms: timer.elapsed().as_millis() as u64,
    ///         })
    ///     }
    /// );
    /// ```
    fn register_typed_tool<I, O, F, Fut>(
        &mut self,
        name: &str,
        description: &str,
        handler: F,
    ) where
        I: JsonSchema + for<'de> Deserialize<'de> + Send + 'static,
        O: McpToolOutput + 'static,
        F: Fn(I, C, crate::framework::NotificationCtx) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = anyhow::Result<O>> + Send + 'static;
    
    /// Get the output schema for a registered tool
    fn get_tool_output_schema(&self, name: &str) -> Option<OutputSchema>;
}

/// Helper macro to implement McpToolOutput with minimal boilerplate
///
/// # Example
///
/// ```rust
/// #[derive(Debug, Serialize, JsonSchema)]
/// struct MyOutput {
///     status: String,
///     data: Vec<Item>,
/// }
///
/// impl_mcp_output!(MyOutput, "MyOutput", "Output from my tool", |output| {
///     format!("Processed {} items with status: {}", output.data.len(), output.status)
/// });
/// ```
#[macro_export]
macro_rules! impl_mcp_output {
    ($type:ty, $name:expr, $description:expr, $summary_fn:expr) => {
        impl $crate::typed_response::McpToolOutput for $type {
            fn to_mcp_response(&self) -> $crate::content_types::McpResponse {
                let summary = $summary_fn(self);
                $crate::content_types::McpResponse::with_text_and_data(
                    summary,
                    serde_json::to_value(self).expect("Failed to serialize output")
                )
            }
            
            fn output_schema() -> $crate::typed_response::OutputSchema {
                $crate::typed_response::OutputSchema {
                    name: $name,
                    description: $description,
                    schema: schemars::schema_for!(Self),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Serialize;
    
    #[derive(Debug, Serialize, JsonSchema)]
    struct TestOutput {
        message: String,
        count: usize,
    }
    
    impl McpToolOutput for TestOutput {
        fn to_mcp_response(&self) -> McpResponse {
            McpResponse::with_text_and_data(
                format!("Test: {} ({})", self.message, self.count),
                serde_json::to_value(self).unwrap()
            )
        }
        
        fn output_schema() -> OutputSchema {
            OutputSchema {
                name: "TestOutput",
                description: "Test output type",
                schema: schemars::schema_for!(Self),
            }
        }
    }
    
    #[test]
    fn test_typed_output() {
        let output = TestOutput {
            message: "Hello".to_string(),
            count: 42,
        };
        
        let response = output.to_mcp_response();
        assert!(response.content.len() > 0);
        assert!(response.data.is_some());
        
        let schema = TestOutput::output_schema();
        assert_eq!(schema.name, "TestOutput");
    }
}