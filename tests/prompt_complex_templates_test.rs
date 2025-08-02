//! Complex Template Scenarios Tests
//!
//! Tests for advanced prompt template functionality including conditional logic,
//! nested parameter substitution, multi-message prompts, and complex argument handling.

use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use solidmcp::{McpResult, McpError};
use std::sync::Arc;
use async_trait::async_trait;
use solidmcp::{McpServerBuilder, PromptProvider, PromptInfo, PromptContent, PromptMessage, PromptArgument};

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Test context for complex template tests
#[derive(Clone)]
pub struct ComplexTestContext {
    pub server_name: String,
}

/// Complex prompt provider with advanced template scenarios
pub struct ComplexPromptProvider;

#[async_trait]
impl PromptProvider<ComplexTestContext> for ComplexPromptProvider {
    async fn list_prompts(&self, _context: Arc<ComplexTestContext>) -> McpResult<Vec<PromptInfo>> {
        Ok(vec![
            PromptInfo {
                name: "multi_role_prompt".to_string(),
                description: Some("A prompt with multiple roles and messages".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "topic".to_string(),
                        description: Some("The topic to discuss".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "expertise_level".to_string(),
                        description: Some("Level of expertise (beginner, intermediate, advanced)".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "conditional_prompt".to_string(),
                description: Some("A prompt with conditional content based on parameters".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "task_type".to_string(),
                        description: Some("Type of task (analysis, creation, review)".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "context".to_string(),
                        description: Some("Additional context for the task".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "nested_params_prompt".to_string(),
                description: Some("A prompt with nested parameter substitution".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "user_name".to_string(),
                        description: Some("User's name".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "project_name".to_string(),
                        description: Some("Project name".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "deadline".to_string(),
                        description: Some("Project deadline".to_string()),
                        required: false,
                    },
                ],
            },
        ])
    }

    async fn get_prompt(&self, name: &str, arguments: Option<Value>, _context: Arc<ComplexTestContext>) -> McpResult<PromptContent> {
        let default_map = serde_json::Map::new();
        let args = arguments.as_ref().and_then(|v| v.as_object()).unwrap_or(&default_map);

        match name {
            "multi_role_prompt" => {
                let topic = args.get("topic")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: topic".to_string()))?;
                let expertise_level = args.get("expertise_level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("intermediate");

                let system_instruction = match expertise_level {
                    "beginner" => "You are a patient teacher explaining concepts to beginners. Use simple language and provide step-by-step explanations.",
                    "advanced" => "You are an expert consultant providing detailed technical analysis. Assume deep domain knowledge.",
                    _ => "You are a knowledgeable assistant providing balanced explanations suitable for intermediate users.",
                };

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: system_instruction.to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Please explain {} in detail.", topic),
                        },
                        PromptMessage {
                            role: "assistant".to_string(),
                            content: format!("I'll explain {} at the {} level. Let me break this down for you.", topic, expertise_level),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: "Please continue with the explanation.".to_string(),
                        },
                    ],
                })
            }
            "conditional_prompt" => {
                let task_type = args.get("task_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: task_type".to_string()))?;
                let context = args.get("context")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let (system_role, user_prompt) = match task_type {
                    "analysis" => (
                        "You are a data analyst specializing in thorough analysis and insights.",
                        format!("Please analyze the following{}: {}", 
                            if context.is_empty() { "" } else { " in the context of" }, 
                            if context.is_empty() { "the provided data" } else { context })
                    ),
                    "creation" => (
                        "You are a creative assistant helping with content creation.",
                        format!("Please create content{}: {}", 
                            if context.is_empty() { "" } else { " based on" }, 
                            if context.is_empty() { "for the specified requirements" } else { context })
                    ),
                    "review" => (
                        "You are a thorough reviewer providing constructive feedback.",
                        format!("Please review the following{}: {}", 
                            if context.is_empty() { "" } else { " considering" }, 
                            if context.is_empty() { "content for quality and accuracy" } else { context })
                    ),
                    _ => return Err(McpError::InvalidParams(format!("Unknown task_type: {}", task_type))),
                };

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: system_role.to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: user_prompt,
                        },
                    ],
                })
            }
            "nested_params_prompt" => {
                let user_name = args.get("user_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: user_name".to_string()))?;
                let project_name = args.get("project_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: project_name".to_string()))?;
                let deadline = args.get("deadline")
                    .and_then(|v| v.as_str());

                let deadline_text = if let Some(deadline) = deadline {
                    format!(" The deadline for {} is {}.", project_name, deadline)
                } else {
                    format!(" No specific deadline has been set for {}.", project_name)
                };

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: format!("You are assisting {} with project management for {}.", user_name, project_name),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Hello, I'm {} and I need help with my project '{}'.{}", user_name, project_name, deadline_text),
                        },
                    ],
                })
            }
            _ => Err(McpError::InvalidParams(format!("Prompt not found: {}", name)))
        }
    }
}

/// Helper to create test server with complex prompts
async fn create_complex_test_server() -> McpResult<solidmcp::McpServer> {
    let context = ComplexTestContext {
        server_name: "complex-prompt-server".to_string(),
    };

    let server = McpServerBuilder::new(context, "complex-prompt-server", "1.0.0")
        .with_prompt_provider(Box::new(ComplexPromptProvider))
        .build()
        .await
        .map_err(|e| McpError::InvalidParams(format!("Failed to build server: {}", e)))?;

    Ok(server)
}

/// Test multi-role prompt with different expertise levels
#[tokio::test]
async fn test_multi_role_prompt() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let server = create_complex_test_server().await?;
    let (server_handle, port) = server.start_dynamic().await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
    let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Test with advanced expertise level
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "multi_role_prompt",
            "arguments": {
                "topic": "machine learning",
                "expertise_level": "advanced"
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4); // system, user, assistant, user

    // Verify system message contains advanced instruction
    let system_message = &messages[0];
    assert_eq!(system_message["role"], "system");
    assert!(system_message["content"]["text"].as_str().unwrap().contains("expert consultant"));

    // Verify topic is mentioned in user message
    let user_message = &messages[1];
    assert_eq!(user_message["role"], "user");
    assert!(user_message["content"]["text"].as_str().unwrap().contains("machine learning"));

    server_handle.abort();
    Ok(())
}

/// Test conditional prompt with different task types
#[tokio::test]
async fn test_conditional_prompt() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let server = create_complex_test_server().await?;
    let (server_handle, port) = server.start_dynamic().await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
    let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Test analysis task type
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "conditional_prompt",
            "arguments": {
                "task_type": "analysis",
                "context": "quarterly sales data"
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify analysis-specific content
    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);

    let system_message = &messages[0];
    assert!(system_message["content"]["text"].as_str().unwrap().contains("data analyst"));

    let user_message = &messages[1];
    assert!(user_message["content"]["text"].as_str().unwrap().contains("quarterly sales data"));

    server_handle.abort();
    Ok(())
}