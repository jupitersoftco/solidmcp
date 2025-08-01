//! Caching and Invalidation Behavior Tests
//!
//! Tests for prompt system consistency, state management, and behavior
//! that would be relevant for caching implementations.

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

/// Test context for caching behavior tests
#[derive(Clone)]
pub struct CachingTestContext {
    pub server_name: String,
    pub call_count: Arc<std::sync::atomic::AtomicUsize>,
    pub dynamic_content: Arc<std::sync::Mutex<String>>,
}

/// Caching-aware prompt provider that tracks calls and state
pub struct CachingPromptProvider;

#[async_trait]
impl PromptProvider<CachingTestContext> for CachingPromptProvider {
    async fn list_prompts(&self, context: Arc<CachingTestContext>) -> McpResult<Vec<PromptInfo>> {
        // Increment call counter
        context.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        Ok(vec![
            PromptInfo {
                name: "static_prompt".to_string(),
                description: Some("A static prompt that should be cacheable".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "input".to_string(),
                        description: Some("Input parameter".to_string()),
                        required: true,
                    },
                ],
            },
            PromptInfo {
                name: "dynamic_prompt".to_string(),
                description: Some("A dynamic prompt with changing content".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "refresh".to_string(),
                        description: Some("Whether to refresh content".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "timestamp_prompt".to_string(),
                description: Some("A prompt that includes timestamps".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "include_time".to_string(),
                        description: Some("Whether to include current time".to_string()),
                        required: false,
                    },
                ],
            },
        ])
    }

    async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<CachingTestContext>) -> McpResult<PromptContent> {
        // Increment call counter
        context.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let default_map = serde_json::Map::new();
        let args = arguments.as_ref().and_then(|v| v.as_object()).unwrap_or(&default_map);

        match name {
            "static_prompt" => {
                let input = args.get("input")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: input".to_string()))?;

                // This should be cacheable - same input always produces same output
                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are processing static content that should be cacheable.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Process this static input: {}", input),
                        },
                    ],
                })
            }
            "dynamic_prompt" => {
                let refresh = args.get("refresh")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let content = if refresh {
                    // Update dynamic content
                    let new_content = format!("Updated content at call #{}", 
                        context.call_count.load(std::sync::atomic::Ordering::SeqCst));
                    if let Ok(mut dynamic) = context.dynamic_content.lock() {
                        *dynamic = new_content.clone();
                    }
                    new_content
                } else {
                    // Use existing dynamic content
                    context.dynamic_content.lock()
                        .map(|content| content.clone())
                        .unwrap_or_else(|_| "Default dynamic content".to_string())
                };

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are processing dynamic content that changes over time.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Dynamic content: {}", content),
                        },
                    ],
                })
            }
            "timestamp_prompt" => {
                let include_time = args.get("include_time")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let content = if include_time {
                    // Use nanoseconds for more precision and add call count for uniqueness
                    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                    let call_num = context.call_count.load(std::sync::atomic::Ordering::SeqCst);
                    format!("Current timestamp: {} (call {})", nanos, call_num)
                } else {
                    "Timeless content".to_string()
                };

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are processing content that may include timestamps.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content,
                        },
                    ],
                })
            }
            _ => Err(McpError::InvalidParams(format!("Prompt not found: {}", name)))
        }
    }
}

/// Helper to create caching test server
async fn create_caching_test_server() -> Result<(solidmcp::McpServer, Arc<CachingTestContext>), Box<dyn std::error::Error + Send + Sync>> {
    let context = CachingTestContext {
        server_name: "caching-test-server".to_string(),
        call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        dynamic_content: Arc::new(std::sync::Mutex::new("Initial dynamic content".to_string())),
    };

    let context_arc = Arc::new(context.clone());

    let server = McpServerBuilder::new(context, "caching-test-server", "1.0.0")
        .with_prompt_provider(Box::new(CachingPromptProvider))
        .build()
        .await?;

    Ok((server, context_arc))
}

/// Test consistent behavior for static prompts (cacheable scenarios)
#[tokio::test]
async fn test_static_prompt_consistency() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let (server, context) = create_caching_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Make multiple requests with same parameters
    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let mut responses = Vec::new();

    for i in 0..3 {
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
                "clientInfo": {"name": format!("test-client-{}", i), "version": "1.0.0"}
            }
        });

        write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
        let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

        // Request same static prompt
        let get_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/get",
            "params": {
                "name": "static_prompt",
                "arguments": {
                    "input": "test_data"
                }
            }
        });

        write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        responses.push(response);
    }

    // Verify all responses are identical (cacheable behavior)
    for i in 1..responses.len() {
        assert_eq!(responses[0]["result"]["messages"], responses[i]["result"]["messages"]);
    }

    // Verify call count increased (showing provider was called)
    let final_count = context.call_count.load(std::sync::atomic::Ordering::SeqCst);
    assert!(final_count >= 3); // At least one list_prompts + 3 get_prompt calls

    server_handle.abort();
    Ok(())
}

/// Test dynamic content behavior (cache invalidation scenarios)
#[tokio::test]
async fn test_dynamic_content_changes() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let (server, _context) = create_caching_test_server().await?;
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

    // First request without refresh
    let get_message1 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "dynamic_prompt",
            "arguments": {
                "refresh": false
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message1)?.into())).await?;
    let response1_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response1: Value = serde_json::from_str(&response1_text)?;

    // Second request with refresh (should update content)
    let get_message2 = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "prompts/get",
        "params": {
            "name": "dynamic_prompt",
            "arguments": {
                "refresh": true
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message2)?.into())).await?;
    let response2_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response2: Value = serde_json::from_str(&response2_text)?;

    // Third request without refresh (should use updated content)
    let get_message3 = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "prompts/get",
        "params": {
            "name": "dynamic_prompt",
            "arguments": {
                "refresh": false
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message3)?.into())).await?;
    let response3_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response3: Value = serde_json::from_str(&response3_text)?;

    // Verify content changed after refresh
    let content1 = response1["result"]["messages"][1]["content"]["text"].as_str().unwrap();
    let content2 = response2["result"]["messages"][1]["content"]["text"].as_str().unwrap();
    let content3 = response3["result"]["messages"][1]["content"]["text"].as_str().unwrap();

    assert_ne!(content1, content2); // Content should change after refresh
    assert_eq!(content2, content3); // Content should remain same after update

    server_handle.abort();
    Ok(())
}

/// Test timestamp-based invalidation scenarios
#[tokio::test]
async fn test_timestamp_based_content() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let (server, _context) = create_caching_test_server().await?;
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

    // Request without timestamp
    let get_message1 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "timestamp_prompt",
            "arguments": {
                "include_time": false
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message1)?.into())).await?;
    let response1_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response1: Value = serde_json::from_str(&response1_text)?;

    // Wait a moment
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Request with timestamp (first time)
    let get_message2 = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "prompts/get",
        "params": {
            "name": "timestamp_prompt",
            "arguments": {
                "include_time": true
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message2)?.into())).await?;
    let response2_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response2: Value = serde_json::from_str(&response2_text)?;

    // Wait a moment
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Request with timestamp (second time)
    let get_message3 = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "prompts/get",
        "params": {
            "name": "timestamp_prompt",
            "arguments": {
                "include_time": true
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message3)?.into())).await?;
    let response3_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response3: Value = serde_json::from_str(&response3_text)?;

    // Verify behavior
    let content1 = response1["result"]["messages"][1]["content"]["text"].as_str().unwrap();
    let content2 = response2["result"]["messages"][1]["content"]["text"].as_str().unwrap();
    let content3 = response3["result"]["messages"][1]["content"]["text"].as_str().unwrap();

    assert_eq!(content1, "Timeless content"); // Static content without time
    assert!(content2.contains("Current timestamp:")); // Should contain timestamp
    assert!(content3.contains("Current timestamp:")); // Should contain timestamp
    assert_ne!(content2, content3); // Timestamps should be different

    server_handle.abort();
    Ok(())
}

/// Test prompt list consistency behavior
#[tokio::test]
async fn test_prompt_list_consistency() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let (server, context) = create_caching_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);

    // Make multiple list requests from different connections
    let mut list_responses = Vec::new();
    for i in 0..3 {
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
                "clientInfo": {"name": format!("list-client-{}", i), "version": "1.0.0"}
            }
        });

        write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
        let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

        // Request prompt list
        let list_message = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": {}
        });

        write.send(Message::Text(serde_json::to_string(&list_message)?.into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        list_responses.push(response);
    }

    // Verify all list responses are identical (consistent behavior)
    for i in 1..list_responses.len() {
        assert_eq!(list_responses[0]["result"]["prompts"], list_responses[i]["result"]["prompts"]);
    }

    // Verify the list contains expected prompts
    let prompts = list_responses[0]["result"]["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 3);

    let prompt_names: Vec<&str> = prompts.iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert!(prompt_names.contains(&"static_prompt"));
    assert!(prompt_names.contains(&"dynamic_prompt"));
    assert!(prompt_names.contains(&"timestamp_prompt"));

    // Verify call count shows list_prompts was called multiple times
    let final_count = context.call_count.load(std::sync::atomic::Ordering::SeqCst);
    assert!(final_count >= 3); // Should have been called at least 3 times

    server_handle.abort();
    Ok(())
}