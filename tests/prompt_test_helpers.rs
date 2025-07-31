//! Test helpers for prompt system tests
//!
//! Shared utilities for testing prompt functionality across different transports.

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use solidmcp::framework::{McpServerBuilder, PromptProvider};
use solidmcp::handler::{PromptInfo, PromptContent, PromptMessage, PromptArgument};

/// Test context for prompt tests
#[derive(Clone)]
pub struct TestContext {
    pub server_name: String,
}

/// Test prompt provider that provides sample prompts
pub struct TestPromptProvider;

#[async_trait]
impl PromptProvider<TestContext> for TestPromptProvider {
    async fn list_prompts(&self, _context: Arc<TestContext>) -> Result<Vec<PromptInfo>> {
        Ok(vec![
            PromptInfo {
                name: "hello_world".to_string(),
                description: Some("A simple hello world prompt".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "name".to_string(),
                        description: Some("The name to greet".to_string()),
                        required: true,
                    },
                ],
            },
            PromptInfo {
                name: "code_review".to_string(),
                description: Some("Generate a code review prompt".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "code".to_string(),
                        description: Some("The code to review".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "language".to_string(),
                        description: Some("Programming language".to_string()),
                        required: false,
                    },
                ],
            },
        ])
    }

    async fn get_prompt(&self, name: &str, arguments: Option<Value>, _context: Arc<TestContext>) -> Result<PromptContent> {
        match name {
            "hello_world" => {
                let default_map = serde_json::Map::new();
                let args = arguments.as_ref().and_then(|v| v.as_object()).unwrap_or(&default_map);
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required argument: name"))?;

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Hello, {}!", name),
                        }
                    ],
                })
            }
            "code_review" => {
                let default_map = serde_json::Map::new();
                let args = arguments.as_ref().and_then(|v| v.as_object()).unwrap_or(&default_map);
                let code = args.get("code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required argument: code"))?;
                let language = args.get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: format!("You are a code reviewer specializing in {}.", language),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Please review this code:\n\n{}", code),
                        }
                    ],
                })
            }
            _ => Err(anyhow::anyhow!("Prompt not found: {}", name))
        }
    }
}

/// Helper to create a test server with prompt providers
pub async fn create_test_server_with_prompts() -> Result<solidmcp::McpServer, Box<dyn std::error::Error + Send + Sync>> {
    let context = TestContext {
        server_name: "test-prompt-server".to_string(),
    };

    let server = McpServerBuilder::new(context, "test-prompt-server", "1.0.0")
        .with_prompt_provider(Box::new(TestPromptProvider))
        .build()
        .await?;

    Ok(server)
}