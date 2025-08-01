//! Provider traits for resources and prompts.
//!
//! This module defines the trait interfaces for dynamic resource and prompt
//! providers that can be registered with the MCP framework.

use crate::handler::{PromptContent, PromptInfo, ResourceContent, ResourceInfo};
use crate::error::{McpError, McpResult};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Trait for providing resources dynamically to MCP clients.
///
/// Resource providers allow your server to expose data, files, or other content
/// through URI-based access. Clients can list available resources and read their
/// content on demand.
///
/// # Type Parameters
/// - `C`: The application context type
///
/// # Examples
/// ```rust
/// struct DatabaseProvider {
///     connection_pool: sqlx::Pool<sqlx::Postgres>,
/// }
///
/// #[async_trait]
/// impl ResourceProvider<AppContext> for DatabaseProvider {
///     async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
///         Ok(vec![
///             ResourceInfo {
///                 uri: "db://users".to_string(),
///                 name: "User Database".to_string(),
///                 description: Some("All registered users".to_string()),
///                 mime_type: Some("application/json".to_string()),
///             }
///         ])
///     }
///
///     async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
///         match uri {
///             "db://users" => {
///                 let users = sqlx::query!("SELECT * FROM users")
///                     .fetch_all(&self.connection_pool)
///                     .await?;
///                 Ok(ResourceContent {
///                     uri: uri.to_string(),
///                     mime_type: Some("application/json".to_string()),
///                     content: serde_json::to_string_pretty(&users)?,
///                 })
///             }
///             _ => Err(McpError::UnknownResource(uri.to_string()))
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait ResourceProvider<C>: Send + Sync {
    /// List all available resources that this provider can serve.
    ///
    /// # Parameters
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<Vec<ResourceInfo>>` - List of available resources or an error
    async fn list_resources(&self, context: Arc<C>) -> McpResult<Vec<ResourceInfo>>;

    /// Read the content of a specific resource identified by URI.
    ///
    /// # Parameters
    /// - `uri`: The unique identifier for the resource to read
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<ResourceContent>` - The resource content or an error if not found
    async fn read_resource(&self, uri: &str, context: Arc<C>) -> McpResult<ResourceContent>;
}

/// Trait for providing dynamic prompt templates to MCP clients.
///
/// Prompt providers allow your server to expose reusable prompt templates that
/// clients can use with AI models. Prompts can have parameters and generate
/// contextual conversation messages.
///
/// # Type Parameters
/// - `C`: The application context type
///
/// # Examples
/// ```rust
/// struct TemplateProvider;
///
/// #[async_trait]
/// impl PromptProvider<AppContext> for TemplateProvider {
///     async fn list_prompts(&self, context: Arc<AppContext>) -> McpResult<Vec<PromptInfo>> {
///         Ok(vec![
///             PromptInfo {
///                 name: "code_review".to_string(),
///                 description: Some("Generate a code review for the given code".to_string()),
///                 arguments: vec![
///                     PromptArgument {
///                         name: "code".to_string(),
///                         description: Some("The code to review".to_string()),
///                         required: true,
///                     },
///                     PromptArgument {
///                         name: "language".to_string(),
///                         description: Some("Programming language".to_string()),
///                         required: false,
///                     },
///                 ],
///             }
///         ])
///     }
///
///     async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<AppContext>) -> McpResult<PromptContent> {
///         match name {
///             "code_review" => {
///                 let args: serde_json::Map<String, Value> = arguments
///                     .and_then(|v| v.as_object().cloned())
///                     .unwrap_or_default();
///                 
///                 let code = args.get("code")
///                     .and_then(|v| v.as_str())
///                     .ok_or_else(|| McpError::InvalidParams("Missing required argument: code".to_string()))?;
///                 
///                 let language = args.get("language")
///                     .and_then(|v| v.as_str())
///                     .unwrap_or("unknown");
///
///                 Ok(PromptContent {
///                     messages: vec![
///                         PromptMessage {
///                             role: "user".to_string(),
///                             content: format!("Please review this {} code:\n\n{}", language, code),
///                         }
///                     ],
///                 })
///             }
///             _ => Err(McpError::UnknownPrompt(name.to_string()))
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait PromptProvider<C>: Send + Sync {
    /// List all available prompt templates that this provider can serve.
    ///
    /// # Parameters
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<Vec<PromptInfo>>` - List of available prompts or an error
    async fn list_prompts(&self, context: Arc<C>) -> McpResult<Vec<PromptInfo>>;

    /// Generate prompt content for a specific template with given arguments.
    ///
    /// # Parameters
    /// - `name`: The name of the prompt template to generate
    /// - `arguments`: Optional JSON object containing template parameters
    /// - `context`: Shared application context
    ///
    /// # Returns
    /// `Result<PromptContent>` - The generated prompt messages or an error if not found
    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        context: Arc<C>,
    ) -> McpResult<PromptContent>;
}