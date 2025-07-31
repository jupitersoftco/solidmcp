//! Prompt Parameter Handling Tests
//!
//! Tests prompt parameter validation, substitution, and edge cases with various data types.

use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use anyhow::Result;

mod mcp_test_helpers;
use mcp_test_helpers::*;

// Create a test server with custom prompt provider for parameter testing
async fn create_test_prompt_server() -> Result<u16> {
    use std::sync::Arc;
    use solidmcp::{
        framework::{McpServerBuilder, PromptProvider},
        handler::{PromptInfo, PromptContent, PromptMessage, PromptArgument},
    };
    use async_trait::async_trait;

    struct TestContext;

    struct TestPromptProvider;

    #[async_trait]
    impl PromptProvider<TestContext> for TestPromptProvider {
        async fn list_prompts(&self, _context: Arc<TestContext>) -> Result<Vec<PromptInfo>> {
            Ok(vec![
                PromptInfo {
                    name: "simple_template".to_string(),
                    description: Some("Simple template with one parameter".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "name".to_string(),
                            description: Some("The name to substitute".to_string()),
                            required: true,
                        }
                    ],
                },
                PromptInfo {
                    name: "complex_template".to_string(),
                    description: Some("Complex template with multiple parameters".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "title".to_string(),
                            description: Some("Document title".to_string()),
                            required: true,
                        },
                        PromptArgument {
                            name: "author".to_string(),
                            description: Some("Document author".to_string()),
                            required: false,
                        },
                        PromptArgument {
                            name: "tags".to_string(),
                            description: Some("Comma-separated tags".to_string()),
                            required: false,
                        },
                    ],
                },
                PromptInfo {
                    name: "numeric_template".to_string(),
                    description: Some("Template with numeric parameters".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "count".to_string(),
                            description: Some("Number of items".to_string()),
                            required: true,
                        },
                        PromptArgument {
                            name: "percentage".to_string(),
                            description: Some("Percentage value".to_string()),
                            required: false,
                        },
                    ],
                },
            ])
        }

        async fn get_prompt(
            &self,
            name: &str,
            arguments: Option<Value>,
            _context: Arc<TestContext>,
        ) -> Result<PromptContent> {
            let args = arguments.unwrap_or_default();

            match name {
                "simple_template" => {
                    let name = args.get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;
                    
                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "user".to_string(),
                                content: format!("Hello, {}! How are you today?", name),
                            }
                        ],
                    })
                }
                "complex_template" => {
                    let title = args.get("title")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Missing required parameter: title"))?;
                    
                    let author = args.get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    
                    let tags = args.get("tags")
                        .and_then(|v| v.as_str())
                        .unwrap_or("general");

                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "system".to_string(),
                                content: "You are a document reviewer.".to_string(),
                            },
                            PromptMessage {
                                role: "user".to_string(),
                                content: format!(
                                    "Please review this document:\n\nTitle: {}\nAuthor: {}\nTags: {}",
                                    title, author, tags
                                ),
                            }
                        ],
                    })
                }
                "numeric_template" => {
                    let count = args.get("count")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameter: count"))?;
                    
                    let percentage = args.get("percentage")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100.0);

                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "user".to_string(),
                                content: format!(
                                    "Process {} items with {}% completion rate.",
                                    count, percentage
                                ),
                            }
                        ],
                    })
                }
                _ => Err(anyhow::anyhow!("Prompt not found: {}", name))
            }
        }
    }

    let port = find_available_port().await?;
    let context = TestContext;

    let mut server = McpServerBuilder::new(context, "test-prompt-server", "1.0.0")
        .with_prompt_provider(Box::new(TestPromptProvider))
        .build()
        .await?;

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;
    Ok(port)
}

#[tokio::test]
async fn test_prompt_list_with_arguments() -> Result<()> {
    init_test_tracing();
    let port = create_test_prompt_server().await?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(init_request.to_string())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // List prompts
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/list",
        "params": {}
    });

    write.send(Message::Text(list_request.to_string())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    let prompts = response["result"]["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 3);

    // Verify simple_template
    let simple = prompts.iter().find(|p| p["name"] == "simple_template").unwrap();
    assert_eq!(simple["description"], "Simple template with one parameter");
    
    let args = simple["arguments"].as_array().unwrap();
    assert_eq!(args.len(), 1);
    assert_eq!(args[0]["name"], "name");
    assert_eq!(args[0]["required"], true);

    // Verify complex_template
    let complex = prompts.iter().find(|p| p["name"] == "complex_template").unwrap();
    let complex_args = complex["arguments"].as_array().unwrap();
    assert_eq!(complex_args.len(), 3);
    
    // Check required/optional parameters
    let title_arg = complex_args.iter().find(|a| a["name"] == "title").unwrap();
    assert_eq!(title_arg["required"], true);
    
    let author_arg = complex_args.iter().find(|a| a["name"] == "author").unwrap();
    assert_eq!(author_arg["required"], false);

    Ok(())
}

#[tokio::test]
async fn test_prompt_simple_parameter_substitution() -> Result<()> {
    init_test_tracing();
    let port = create_test_prompt_server().await?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(init_request.to_string())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get prompt with parameter
    let get_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "simple_template",
            "arguments": {
                "name": "Alice"
            }
        }
    });

    write.send(Message::Text(get_request.to_string())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["role"], "user");
    assert_eq!(message["content"], "Hello, Alice! How are you today?");

    Ok(())
}

#[tokio::test]
async fn test_prompt_missing_required_parameter() -> Result<()> {
    init_test_tracing();
    let port = create_test_prompt_server().await?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(init_request.to_string())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get prompt without required parameter
    let get_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "simple_template",
            "arguments": {}
        }
    });

    write.send(Message::Text(get_request.to_string())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Should return error
    assert!(response.get("error").is_some());
    let error = &response["error"];
    assert!(error["message"].as_str().unwrap().contains("Missing required parameter"));

    Ok(())
}

#[tokio::test]
async fn test_prompt_numeric_parameters() -> Result<()> {
    init_test_tracing();
    let port = create_test_prompt_server().await?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(&format!("ws://127.0.0.1:{}/mcp", port)).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(init_request.to_string())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Get prompt with numeric parameters
    let get_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "numeric_template",
            "arguments": {
                "count": 42,
                "percentage": 85.5
            }
        }
    });

    write.send(Message::Text(get_request.to_string())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["role"], "user");
    assert_eq!(message["content"], "Process 42 items with 85.5% completion rate.");

    Ok(())
}