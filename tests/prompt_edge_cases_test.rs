//! Edge Case Tests for Prompt System
//!
//! Tests for handling edge cases including large prompts, special characters,
//! Unicode content, boundary conditions, and malformed templates.

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

/// Test context for edge case tests
#[derive(Clone)]
pub struct EdgeCaseTestContext {
    pub server_name: String,
}

/// Edge case prompt provider with various boundary conditions
pub struct EdgeCasePromptProvider;

#[async_trait]
impl PromptProvider<EdgeCaseTestContext> for EdgeCasePromptProvider {
    async fn list_prompts(&self, _context: Arc<EdgeCaseTestContext>) -> McpResult<Vec<PromptInfo>> {
        Ok(vec![
            PromptInfo {
                name: "large_prompt".to_string(),
                description: Some("A prompt that generates large content".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "size".to_string(),
                        description: Some("Size multiplier for content".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "unicode_prompt".to_string(),
                description: Some("A prompt with Unicode and special characters".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "text".to_string(),
                        description: Some("Text with special characters".to_string()),
                        required: true,
                    },
                ],
            },
            PromptInfo {
                name: "empty_prompt".to_string(),
                description: Some("A prompt that can generate empty content".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "generate_empty".to_string(),
                        description: Some("Whether to generate empty content".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "json_escape_prompt".to_string(),
                description: Some("A prompt that tests JSON character escaping".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "special_chars".to_string(),
                        description: Some("Special characters to include".to_string()),
                        required: true,
                    },
                ],
            },
        ])
    }

    async fn get_prompt(&self, name: &str, arguments: Option<Value>, _context: Arc<EdgeCaseTestContext>) -> McpResult<PromptContent> {
        let default_map = serde_json::Map::new();
        let args = arguments.as_ref().and_then(|v| v.as_object()).unwrap_or(&default_map);

        match name {
            "large_prompt" => {
                let size = args.get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;

                // Generate large content
                let base_text = "This is a large prompt content that will be repeated many times to test large message handling. ";
                let large_content = base_text.repeat(size * 100); // Multiply by 100 for significant size

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are processing a large prompt.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: large_content,
                        },
                    ],
                })
            }
            "unicode_prompt" => {
                let text = args.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: text".to_string()))?;

                // Add various Unicode characters
                let unicode_content = format!(
                    "Processing text with Unicode: {} 🚀 ✨ 🎯 \n\
                     Chinese: 你好世界 \n\
                     Arabic: مرحبا بالعالم \n\
                     Emoji: 😀😂🤣😊 \n\
                     Mathematical: ∑∏∫√π∞ \n\
                     Currency: €£¥₹₿ \n\
                     Original: {}",
                    text, text
                );

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are handling Unicode and international text.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: unicode_content,
                        },
                    ],
                })
            }
            "empty_prompt" => {
                let generate_empty = args.get("generate_empty")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if generate_empty {
                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "system".to_string(),
                                content: "".to_string(),
                            },
                        ],
                    })
                } else {
                    Ok(PromptContent {
                        messages: vec![
                            PromptMessage {
                                role: "system".to_string(),
                                content: "This is a minimal prompt.".to_string(),
                            },
                        ],
                    })
                }
            }
            "json_escape_prompt" => {
                let special_chars = args.get("special_chars")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: special_chars".to_string()))?;

                // Test various JSON escape characters
                let escape_content = format!(
                    "Testing JSON escaping:\n\
                     Quotes: \"Hello 'World'\"\n\
                     Backslashes: \\path\\to\\file\n\
                     Newlines: Line1\nLine2\n\
                     Tabs: Column1\tColumn2\n\
                     Unicode escapes: \\u0048\\u0065\\u006C\\u006C\\u006F\n\
                     Control chars: \x08\x0C\r\n\
                     Original input: {}",
                    special_chars
                );

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are handling special JSON characters.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: escape_content,
                        },
                    ],
                })
            }
            _ => Err(McpError::InvalidParams(format!("Prompt not found: {}", name)))
        }
    }
}

/// Helper to create edge case test server
async fn create_edge_case_test_server() -> Result<solidmcp::McpServer, Box<dyn std::error::Error + Send + Sync>> {
    let context = EdgeCaseTestContext {
        server_name: "edge-case-test-server".to_string(),
    };

    let server = McpServerBuilder::new(context, "edge-case-test-server", "1.0.0")
        .with_prompt_provider(Box::new(EdgeCasePromptProvider))
        .build()
        .await?;

    Ok(server)
}

/// Test large prompt content handling
#[tokio::test]
async fn test_large_prompt_content() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_edge_case_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

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

    // Test large prompt content
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "large_prompt",
            "arguments": {
                "size": 10
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(10)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);

    // Verify the large content was handled
    let user_message = &messages[1];
    let content_text = user_message["content"]["text"].as_str().unwrap();
    assert!(content_text.len() > 1000); // Should be quite large

    server_handle.abort();
    Ok(())
}

/// Test Unicode and special character handling
#[tokio::test]
async fn test_unicode_special_characters() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_edge_case_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

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

    // Test Unicode prompt
    let unicode_text = "测试文本 🌍 Ñoël café résumé naïve";
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "unicode_prompt",
            "arguments": {
                "text": unicode_text
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);

    // Verify Unicode content is preserved
    let user_message = &messages[1];
    let content_text = user_message["content"]["text"].as_str().unwrap();
    assert!(content_text.contains(unicode_text));
    assert!(content_text.contains("🚀"));
    assert!(content_text.contains("你好世界"));

    server_handle.abort();
    Ok(())
}

/// Test JSON character escaping
#[tokio::test]
async fn test_json_character_escaping() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_edge_case_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

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

    // Test special characters that need JSON escaping
    let special_chars = "\"quotes\" and \\backslashes\\ and \nnewlines\n and \ttabs\t";
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "json_escape_prompt",
            "arguments": {
                "special_chars": special_chars
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify response is valid JSON and contains escaped content
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);

    // Verify the special characters are properly handled in the response
    let user_message = &messages[1];
    let content_text = user_message["content"]["text"].as_str().unwrap();
    assert!(content_text.contains(special_chars));

    server_handle.abort();
    Ok(())
}

/// Test empty content handling
#[tokio::test]
async fn test_empty_content_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_edge_case_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

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

    // Test empty content generation
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "empty_prompt",
            "arguments": {
                "generate_empty": true
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify response with empty content
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    // Verify empty content is handled properly
    let system_message = &messages[0];
    assert_eq!(system_message["role"], "system");
    assert_eq!(system_message["content"]["text"], "");

    server_handle.abort();
    Ok(())
}