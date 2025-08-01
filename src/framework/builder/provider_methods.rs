//! Provider registration methods for McpServerBuilder.
//!
//! This module contains methods for registering resource and prompt providers.

use crate::framework::{
    builder::McpServerBuilder,
    providers::{PromptProvider, ResourceProvider},
};

impl<C: Send + Sync + 'static> McpServerBuilder<C> {
    /// Add a resource provider to expose data through the MCP resource protocol.
    ///
    /// Resource providers allow clients to discover and read data from your server
    /// through URI-based access. This is useful for exposing files, database content,
    /// API responses, or any other data that clients might need.
    ///
    /// # Parameters
    /// - `provider`: A boxed resource provider implementing the `ResourceProvider` trait
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Examples
    /// ```rust
    /// struct FileProvider {
    ///     base_path: PathBuf,
    /// }
    ///
    /// #[async_trait]
    /// impl ResourceProvider<AppContext> for FileProvider {
    ///     async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
    ///         let mut resources = Vec::new();
    ///         let entries = std::fs::read_dir(&self.base_path)?;
    ///         
    ///         for entry in entries {
    ///             let entry = entry?;
    ///             if entry.file_type()?.is_file() {
    ///                 resources.push(ResourceInfo {
    ///                     uri: format!("file://{}", entry.path().display()),
    ///                     name: entry.file_name().to_string_lossy().to_string(),
    ///                     description: Some("Local file".to_string()),
    ///                     mime_type: mime_guess::from_path(&entry.path())
    ///                         .first()
    ///                         .map(|m| m.to_string()),
    ///                 });
    ///             }
    ///         }
    ///         
    ///         Ok(resources)
    ///     }
    ///
    ///     async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
    ///         if let Some(path) = uri.strip_prefix("file://") {
    ///             let full_path = self.base_path.join(path);
    ///             let content = tokio::fs::read_to_string(&full_path).await?;
    ///             
    ///             Ok(ResourceContent {
    ///                 uri: uri.to_string(),
    ///                 mime_type: mime_guess::from_path(&full_path)
    ///                     .first()
    ///                     .map(|m| m.to_string()),
    ///                 content,
    ///             })
    ///         } else {
    ///             Err(anyhow::anyhow!("Invalid file URI: {}", uri))
    ///         }
    ///     }
    /// }
    ///
    /// let server = McpServerBuilder::new(context, "file-server", "1.0.0")
    ///     .with_resource_provider(Box::new(FileProvider {
    ///         base_path: PathBuf::from("./data"),
    ///     }));
    /// ```
    pub fn with_resource_provider(mut self, provider: Box<dyn ResourceProvider<C>>) -> Self {
        self.handler
            .registry_mut()
            .register_resource_provider(provider);
        self
    }

    /// Add a prompt provider to expose reusable prompt templates.
    ///
    /// Prompt providers allow clients to discover and use parameterized prompt
    /// templates that you define. This is useful for providing consistent prompts
    /// for AI interactions or generating contextual conversation starters.
    ///
    /// # Parameters
    /// - `provider`: A boxed prompt provider implementing the `PromptProvider` trait
    ///
    /// # Returns
    /// The builder (for method chaining)
    ///
    /// # Examples
    /// ```rust
    /// struct DocumentationProvider;
    ///
    /// #[async_trait]
    /// impl PromptProvider<AppContext> for DocumentationProvider {
    ///     async fn list_prompts(&self, context: Arc<AppContext>) -> Result<Vec<PromptInfo>> {
    ///         Ok(vec![
    ///             PromptInfo {
    ///                 name: "document_function".to_string(),
    ///                 description: Some("Generate documentation for a function".to_string()),
    ///                 arguments: vec![
    ///                     PromptArgument {
    ///                         name: "function_code".to_string(),
    ///                         description: Some("The function code to document".to_string()),
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
    ///     async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<AppContext>) -> Result<PromptContent> {
    ///         match name {
    ///             "document_function" => {
    ///                 let args = arguments.unwrap_or_default();
    ///                 let code = args.get("function_code")
    ///                     .and_then(|v| v.as_str())
    ///                     .ok_or_else(|| anyhow::anyhow!("Missing function_code argument"))?;
    ///                 
    ///                 let language = args.get("language")
    ///                     .and_then(|v| v.as_str())
    ///                     .unwrap_or("unknown");
    ///
    ///                 Ok(PromptContent {
    ///                     messages: vec![
    ///                         PromptMessage {
    ///                             role: "system".to_string(),
    ///                             content: format!("You are a documentation expert for {} code.", language),
    ///                         },
    ///                         PromptMessage {
    ///                             role: "user".to_string(),
    ///                             content: format!("Please write comprehensive documentation for this function:\n\n```{}\n{}\n```", language, code),
    ///                         },
    ///                     ],
    ///                 })
    ///             }
    ///             _ => Err(anyhow::anyhow!("Unknown prompt: {}", name))
    ///         }
    ///     }
    /// }
    ///
    /// let server = McpServerBuilder::new(context, "doc-server", "1.0.0")
    ///     .with_prompt_provider(Box::new(DocumentationProvider));
    /// ```
    pub fn with_prompt_provider(mut self, provider: Box<dyn PromptProvider<C>>) -> Self {
        self.handler
            .registry_mut()
            .register_prompt_provider(provider);
        self
    }
}