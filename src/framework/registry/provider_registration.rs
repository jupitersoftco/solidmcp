//! Provider registration methods for ToolRegistry.
//!
//! This module contains methods for registering resource and prompt providers.

use crate::framework::{
    providers::{PromptProvider, ResourceProvider},
    registry::ToolRegistry,
};

impl<C: Send + Sync + 'static> ToolRegistry<C> {
    /// Register a resource provider for dynamic resource management.
    ///
    /// Resource providers allow your server to expose data and files through
    /// the MCP resource protocol. Resources are identified by URIs and can
    /// be listed and read by clients.
    ///
    /// # Parameters
    /// - `provider`: Boxed resource provider implementing the `ResourceProvider` trait
    ///
    /// # Examples
    /// ```rust
    /// struct FileSystemProvider;
    ///
    /// #[async_trait]
    /// impl ResourceProvider<AppContext> for FileSystemProvider {
    ///     async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
    ///         // Return list of available files
    ///     }
    ///     
    ///     async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
    ///         // Read and return file content
    ///     }
    /// }
    ///
    /// registry.register_resource_provider(Box::new(FileSystemProvider));
    /// ```
    pub fn register_resource_provider(&mut self, provider: Box<dyn ResourceProvider<C>>) {
        self.resources.push(provider);
    }

    /// Register a prompt provider for dynamic prompt template management.
    ///
    /// Prompt providers allow your server to expose reusable prompt templates
    /// that clients can use with AI models. Prompts can have parameters and
    /// generate contextual messages.
    ///
    /// # Parameters
    /// - `provider`: Boxed prompt provider implementing the `PromptProvider` trait
    ///
    /// # Examples
    /// ```rust
    /// struct CodeReviewProvider;
    ///
    /// #[async_trait]
    /// impl PromptProvider<AppContext> for CodeReviewProvider {
    ///     async fn list_prompts(&self, context: Arc<AppContext>) -> Result<Vec<PromptInfo>> {
    ///         // Return available prompt templates
    ///     }
    ///     
    ///     async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<AppContext>) -> Result<PromptContent> {
    ///         // Generate prompt content based on template and arguments
    ///     }
    /// }
    ///
    /// registry.register_prompt_provider(Box::new(CodeReviewProvider));
    /// ```  
    pub fn register_prompt_provider(&mut self, provider: Box<dyn PromptProvider<C>>) {
        self.prompts.push(provider);
    }
}