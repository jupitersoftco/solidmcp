//! Server builder for creating MCP servers with minimal boilerplate.
//!
//! This module provides the `McpServerBuilder` struct which offers a fluent API
//! for registering tools, resources, and prompts with compile-time type safety
//! and automatic schema generation.

mod provider_methods;
mod tool_methods;

#[cfg(test)]
mod tests;

use crate::server::McpServer;
use anyhow::Result;
use std::sync::Arc;

use super::handler::FrameworkHandler;


/// Convenience builder for creating MCP servers with minimal boilerplate.
///
/// This is the main entry point for building MCP servers using the framework.
/// It provides a fluent API for registering tools, resources, and prompts with
/// compile-time type safety and automatic schema generation.
///
/// # Type Parameters
/// - `C`: The application context type (can be any type you want to share across tools)
///
/// # Examples
/// ```rust
/// use solidmcp::framework::McpServerBuilder;
/// use serde::{Deserialize, Serialize};
/// use schemars::JsonSchema;
/// use std::sync::Arc;
/// use anyhow::Result;
///
/// // Define your application context
/// struct AppContext {
///     config: Config,
///     database: Database,
/// }
///
/// // Define tool input/output types with JsonSchema
/// #[derive(JsonSchema, Deserialize)]
/// struct SearchInput {
///     query: String,
///     limit: Option<u32>,
/// }
///
/// #[derive(JsonSchema, Serialize)]
/// struct SearchOutput {
///     results: Vec<String>,
///     total: u32,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let context = AppContext::new().await?;
///
///     let server = McpServerBuilder::new(context, "search-server", "1.0.0")
///         .with_tool("search", "Search the database", |input: SearchInput, ctx: Arc<AppContext>, notif| async move {
///             notif.info(&format!("Searching for: {}", input.query))?;
///             
///             let results = ctx.database.search(&input.query, input.limit.unwrap_or(10)).await?;
///             
///             Ok(SearchOutput {
///                 results: results.clone(),
///                 total: results.len() as u32,
///             })
///         })
///         .with_resource_provider(Box::new(MyResourceProvider::new()))
///         .build()
///         .await?;
///
///     server.start(3000).await
/// }
/// ```
pub struct McpServerBuilder<C> {
    pub(super) handler: FrameworkHandler<C>,
}

impl<C: Send + Sync + 'static> McpServerBuilder<C> {
    /// Create a new server builder with the specified context and server information.
    ///
    /// This is the starting point for building an MCP server. The context you provide
    /// here will be shared across all tools and can contain any application-specific
    /// state, configuration, or resources.
    ///
    /// # Parameters
    /// - `context`: Your application context (can be any type)
    /// - `server_name`: Name of your MCP server (shown to clients)
    /// - `server_version`: Version string for your server
    ///
    /// # Returns
    /// A new `McpServerBuilder` ready for tool registration
    ///
    /// # Examples
    /// ```rust
    /// // Simple context with just configuration
    /// struct Config {
    ///     api_endpoint: String,
    ///     timeout: Duration,
    /// }
    ///
    /// let builder = McpServerBuilder::new(
    ///     Config {
    ///         api_endpoint: "https://api.example.com".to_string(),
    ///         timeout: Duration::from_secs(30),
    ///     },
    ///     "example-server",
    ///     "1.0.0"
    /// );
    /// ```
    pub fn new(context: C, server_name: &str, server_version: &str) -> Self {
        Self {
            handler: FrameworkHandler::new(context, server_name, server_version),
        }
    }

    /// Configure resource limits for the server.
    ///
    /// This allows you to set limits on various resources to prevent DoS attacks
    /// and resource exhaustion. By default, reasonable limits are applied.
    ///
    /// # Parameters
    /// - `limits`: Resource limits configuration
    ///
    /// # Returns
    /// Self for method chaining
    ///
    /// # Examples
    /// ```rust
    /// use solidmcp::{McpServerBuilder, ResourceLimits};
    /// 
    /// let server = McpServerBuilder::new(context, "server", "1.0.0")
    ///     .with_limits(ResourceLimits {
    ///         max_sessions: Some(1000),
    ///         max_message_size: 1024 * 1024, // 1MB
    ///         ..Default::default()
    ///     })
    ///     .build()
    ///     .await?;
    /// ```
    pub fn with_limits(mut self, limits: crate::limits::ResourceLimits) -> Self {
        self.handler.limits = limits;
        self
    }

    /// Build the MCP server and prepare it for startup.
    ///
    /// This method finalizes the server configuration and creates an `McpServer`
    /// instance that can be started on a specific port. All registered tools,
    /// resources, and prompts are validated and prepared for serving.
    ///
    /// # Returns
    /// `Result<McpServer>` - A configured server ready to start, or an error if configuration is invalid
    ///
    /// # Errors
    /// - Configuration validation errors
    /// - Schema generation errors for registered tools
    /// - Internal setup errors
    ///
    /// # Examples
    /// ```rust
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let server = McpServerBuilder::new(MyContext::new(), "my-server", "1.0.0")
    ///         .with_tool("hello", "Say hello", |input: HelloInput, ctx, notif| async move {
    ///             Ok(HelloOutput { message: format!("Hello, {}!", input.name) })
    ///         })
    ///         .build()
    ///         .await?;
    ///
    ///     // Start the server on port 3000
    ///     server.start(3000).await?
    ///     Ok(())
    /// }
    /// ```
    pub async fn build(self) -> Result<McpServer> {
        let server_name = self.handler.server_name.clone();
        let server_version = self.handler.server_version.clone();
        let limits = self.handler.limits();
        let mut server = McpServer::with_handler(Arc::new(self.handler)).await?;
        
        // Set server info for health checks
        server.set_server_info(server_name, server_version);
        
        // TODO: Once McpServer supports setting limits, apply them here
        // server.set_limits(limits);
        
        Ok(server)
    }
}

/// Convenience macro for registering tools with reduced boilerplate.
///
/// This macro provides a slightly more concise syntax for tool registration,
/// though using the builder methods directly is generally preferred for clarity.
///
/// # Examples
/// ```rust
/// let tool_info = mcp_tool!("calculate", "Perform arithmetic", calculate_handler);
/// ```
#[macro_export]
macro_rules! mcp_tool {
    ($name:expr, $desc:expr, $handler:expr) => {
        ($name, $desc, $handler)
    };
}