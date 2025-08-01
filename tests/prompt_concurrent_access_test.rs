//! Concurrent Access Tests for Prompt Providers
//!
//! Tests for prompt system behavior under concurrent access scenarios,
//! thread safety, and parallel request handling.

use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use solidmcp::{McpResult, McpError};
use std::sync::Arc;
use async_trait::async_trait;
use solidmcp::McpServerBuilder, PromptProvider;
use solidmcp::PromptInfo, PromptContent, PromptMessage, PromptArgument;

mod mcp_test_helpers;
use mcp_test_helpers::*;

/// Thread-safe test context for concurrent tests
#[derive(Clone)]
pub struct ConcurrentTestContext {
    pub server_name: String,
    pub request_count: Arc<std::sync::atomic::AtomicUsize>,
}

/// Concurrent prompt provider that tracks access patterns
pub struct ConcurrentPromptProvider;

#[async_trait]
impl PromptProvider<ConcurrentTestContext> for ConcurrentPromptProvider {
    async fn list_prompts(&self, context: Arc<ConcurrentTestContext>) -> McpResult<Vec<PromptInfo>> {
        // Increment request counter to track concurrent access
        context.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        // Add small delay to simulate realistic processing time
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        Ok(vec![
            PromptInfo {
                name: "concurrent_prompt".to_string(),
                description: Some("A prompt for testing concurrent access".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "data".to_string(),
                        description: Some("Data to process".to_string()),
                        required: true,
                    },
                ],
            },
            PromptInfo {
                name: "slow_prompt".to_string(),
                description: Some("A slow prompt for testing concurrency".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "delay_ms".to_string(),
                        description: Some("Delay in milliseconds".to_string()),
                        required: false,
                    },
                ],
            },
        ])
    }

    async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<ConcurrentTestContext>) -> McpResult<PromptContent> {
        // Increment request counter
        context.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let default_map = serde_json::Map::new();
        let args = arguments.as_ref().and_then(|v| v.as_object()).unwrap_or(&default_map);

        match name {
            "concurrent_prompt" => {
                let data = args.get("data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing required argument: data"))?;

                // Simulate some processing time
                tokio::time::sleep(Duration::from_millis(5)).await;

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: "You are processing concurrent requests efficiently.".to_string(),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Process this data: {}", data),
                        },
                    ],
                })
            }
            "slow_prompt" => {
                let delay_ms = args.get("delay_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100);

                // Simulate variable processing time
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: format!("Processed after {}ms delay", delay_ms),
                        },
                    ],
                })
            }
            _ => Err(McpError::InvalidParams(format!("Prompt not found: {}", name)))
        }
    }
}

/// Helper to create concurrent test server
async fn create_concurrent_test_server() -> Result<solidmcp::McpServer, Box<dyn std::error::Error + Send + Sync>> {
    let context = ConcurrentTestContext {
        server_name: "concurrent-test-server".to_string(),
        request_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };

    let server = McpServerBuilder::new(context, "concurrent-test-server", "1.0.0")
        .with_prompt_provider(Box::new(ConcurrentPromptProvider))
        .build()
        .await?;

    Ok(server)
}

/// Test concurrent prompt list requests
#[tokio::test]
async fn test_concurrent_prompt_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_concurrent_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create multiple concurrent connections
    let mut handles = Vec::new();
    for i in 0..5 {
        let port = port;
        let handle = tokio::spawn(async move {
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
                    "clientInfo": {"name": format!("test-client-{}", i), "version": "1.0.0"}
                }
            });

            write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
            let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

            // Send prompts/list request
            let list_message = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "prompts/list",
                "params": {}
            });

            write.send(Message::Text(serde_json::to_string(&list_message)?.into())).await?;
            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text)?;

            // Verify response
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 2);
            assert!(response.get("result").is_some());

            let result = &response["result"];
            let prompts = result["prompts"].as_array().unwrap();
            assert_eq!(prompts.len(), 2);

            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // Wait for all concurrent requests to complete
    for handle in handles {
        handle.await??;
    }

    server_handle.abort();
    Ok(())
}

/// Test concurrent prompt get requests with different parameters
#[tokio::test]
async fn test_concurrent_prompt_get() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_concurrent_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create multiple concurrent connections with different data
    let mut handles = Vec::new();
    for i in 0..3 {
        let port = port;
        let handle = tokio::spawn(async move {
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
                    "clientInfo": {"name": format!("test-client-{}", i), "version": "1.0.0"}
                }
            });

            write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
            let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

            // Send concurrent_prompt request with unique data
            let get_message = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "prompts/get",
                "params": {
                    "name": "concurrent_prompt",
                    "arguments": {
                        "data": format!("test-data-{}", i)
                    }
                }
            });

            write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text)?;

            // Verify response contains the correct data
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 2);
            assert!(response.get("result").is_some());

            let result = &response["result"];
            let messages = result["messages"].as_array().unwrap();
            assert_eq!(messages.len(), 2);

            let user_message = &messages[1];
            let expected_data = format!("test-data-{}", i);
            assert!(user_message["content"]["text"].as_str().unwrap().contains(&expected_data));

            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // Wait for all concurrent requests to complete
    for handle in handles {
        handle.await??;
    }

    server_handle.abort();
    Ok(())
}

/// Test mixed concurrent operations (list and get)
#[tokio::test]
async fn test_mixed_concurrent_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    let port = find_available_port().await?;
    let server = create_concurrent_test_server().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create concurrent operations: both list and get requests
    let mut handles = Vec::new();
    
    // List operations
    for i in 0..2 {
        let port = port;
        let handle = tokio::spawn(async move {
            let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
            let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
            let (mut write, mut read) = ws_stream.split();

            // Initialize and send list request
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

            let list_message = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "prompts/list",
                "params": {}
            });

            write.send(Message::Text(serde_json::to_string(&list_message)?.into())).await?;
            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text)?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert!(response.get("result").is_some());

            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // Get operations
    for i in 0..3 {
        let port = port;
        let handle = tokio::spawn(async move {
            let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
            let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
            let (mut write, mut read) = ws_stream.split();

            // Initialize and send get request
            let init_message = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": format!("get-client-{}", i), "version": "1.0.0"}
                }
            });

            write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
            let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

            let get_message = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "prompts/get",
                "params": {
                    "name": "slow_prompt",
                    "arguments": {
                        "delay_ms": 50
                    }
                }
            });

            write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
            let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
            let response: Value = serde_json::from_str(&response_text)?;

            assert_eq!(response["jsonrpc"], "2.0");
            assert!(response.get("result").is_some());

            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await??;
    }

    server_handle.abort();
    Ok(())
}