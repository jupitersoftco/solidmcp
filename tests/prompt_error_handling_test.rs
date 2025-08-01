//! Prompt Error Handling Tests
//!
//! Tests various error conditions and edge cases in the prompt system.

use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use solidmcp::{McpResult, McpError};

mod mcp_test_helpers;
use mcp_test_helpers::*;

// Create a test server with error-prone prompt provider
async fn create_error_test_server() -> McpResult<u16> {
    use std::sync::Arc;
    use solidmcp::{
        McpServerBuilder, PromptProvider,
        PromptInfo, PromptContent, PromptMessage, PromptArgument,
    };
    use async_trait::async_trait;

    struct TestContext;

    struct ErrorPromptProvider;

    #[async_trait]
    impl PromptProvider<TestContext> for ErrorPromptProvider {
        async fn list_prompts(&self, _context: Arc<TestContext>) -> McpResult<Vec<PromptInfo>> {
            Ok(vec![
                PromptInfo {
                    name: "error_prompt".to_string(),
                    description: Some("Prompt that always fails".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "input".to_string(),
                            description: Some("Input parameter".to_string()),
                            required: true,
                        }
                    ],
                },
                PromptInfo {
                    name: "large_prompt".to_string(),
                    description: Some("Prompt with very large output".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "size".to_string(),
                            description: Some("Size multiplier".to_string()),
                            required: false,
                        }
                    ],
                },
                PromptInfo {
                    name: "special_chars_prompt".to_string(),
                    description: Some("Prompt with special characters".to_string()),
                    arguments: vec![
                        PromptArgument {
                            name: "text".to_string(),
                            description: Some("Text with special characters".to_string()),
                            required: true,
                        }
                    ],
                },
            ])
        }

        async fn get_prompt(
            &self,
            name: &str,
            arguments: Option<Value>,
            _context: Arc<TestContext>,
        ) -> McpResult<PromptContent> {
            let args = arguments.unwrap_or_default();

            match name {
                "error_prompt" => {
                    Err(McpError::InvalidParams("Intentional error for testing"))
                }
                "large_prompt" => {
                    let size = args.get("size")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1);
                    
                    // Generate a large content string
                    let content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat((size * 1000) as usize);
                    
                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "user".to_string(),
                                content,
                            }
                        ],
                    })
                }
                "special_chars_prompt" => {
                    let text = args.get("text")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::InvalidParams("Missing required parameter: text"))?;
                    
                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "user".to_string(),
                                content: format!("Special text: {}", text),
                            }
                        ],
                    })
                }
                _ => Err(McpError::InvalidParams(format!("Prompt not found: {}", name)))
            }
        }
    }

    let port = find_available_port().await
        .map_err(|e| McpError::InvalidParams(format!("Failed to find port: {}", e)))?;
    let context = TestContext;

    let mut server = McpServerBuilder::new(context, "error-test-server", "1.0.0")
        .with_prompt_provider(Box::new(ErrorPromptProvider))
        .build()
        .await?;

    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(300)).await;
    Ok(port)
}

#[tokio::test]
async fn test_prompt_provider_error() -> McpResult<()> {
    init_test_tracing();
    let port = create_error_test_server().await?;

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

    write.send(Message::Text(init_request.to_string().into())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await
        .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

    // Request prompt that throws error
    let get_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "error_prompt",
            "arguments": {
                "input": "test"
            }
        }
    });

    write.send(Message::Text(get_request.to_string().into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await
        .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Should return error
    assert!(response.get("error").is_some());
    let error = &response["error"];
    assert!(error["message"].as_str().unwrap().contains("Prompt not found"));

    Ok(())
}

#[tokio::test]
async fn test_prompt_large_content() -> McpResult<()> {
    init_test_tracing();
    let port = create_error_test_server().await?;

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

    write.send(Message::Text(init_request.to_string().into())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await
        .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

    // Request large prompt
    let get_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "large_prompt",
            "arguments": {
                "size": 2
            }
        }
    });

    write.send(Message::Text(get_request.to_string().into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(10)).await
        .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Should succeed despite large size
    assert!(response.get("result").is_some());
    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    
    let content = &messages[0]["content"];
    let text_content = if let Some(text_obj) = content.get("text") {
        text_obj.as_str().unwrap()
    } else {
        content.as_str().unwrap()
    };
    assert!(text_content.len() > 100000); // Should be large

    Ok(())
}

#[tokio::test]
async fn test_prompt_special_characters() -> McpResult<()> {
    init_test_tracing();
    let port = create_error_test_server().await?;

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

    write.send(Message::Text(init_request.to_string().into())).await?;
    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await
        .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

    // Test with special characters including Unicode
    let special_text = "Hello 🌍! Test with \"quotes\", 'apostrophes', & ampersands, <tags>, and newlines\nand tabs\t";
    
    let get_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "special_chars_prompt",
            "arguments": {
                "text": special_text
            }
        }
    });

    write.send(Message::Text(get_request.to_string().into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await
        .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Should handle special characters correctly
    assert!(response.get("result").is_some());
    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let content = &messages[0]["content"];
    let text_content = if let Some(text_obj) = content.get("text") {
        text_obj.as_str().unwrap()
    } else {
        content.as_str().unwrap()
    };
    assert!(text_content.contains("🌍"));
    assert!(text_content.contains("\"quotes\""));
    assert!(text_content.contains("<tags>"));

    Ok(())
}

#[tokio::test]
async fn test_prompt_concurrent_requests() -> McpResult<()> {
    init_test_tracing();
    let port = create_error_test_server().await?;

    // Create multiple connections
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let port = port;
        let handle = tokio::spawn(async move {
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
                    "clientInfo": {"name": format!("test-client-{}", i), "version": "1.0.0"}
                }
            });

            write.send(Message::Text(init_request.to_string().into())).await?;
            let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await
                .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;

            // Request prompt
            let get_request = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "prompts/get",
                "params": {
                    "name": "special_chars_prompt",
                    "arguments": {
                        "text": format!("Client {} test", i)
                    }
                }
            });

            write.send(Message::Text(get_request.to_string().into())).await?;
            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await
                .map_err(|e| McpError::InvalidParams(format!("WebSocket error: {}", e)))?;
            let response: Value = serde_json::from_str(&response_text)?;

            // Should succeed
            assert!(response.get("result").is_some());
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await??;
    }

    Ok(())
}