//! Concurrent Session Management Tests
//!
//! Comprehensive tests for concurrent session handling following TDD principles

use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
use futures_util::future::join_all;
use futures_util::{SinkExt, StreamExt};

mod mcp_test_helpers;
use mcp_test_helpers::with_mcp_test_server;

/// Test 1: RED - Concurrent session creation
#[tokio::test]
async fn test_concurrent_session_creation() {
    // Test multiple clients creating sessions simultaneously
    with_mcp_test_server("concurrent_creation_test", |server| async move {
        let num_clients = 10;
        let barrier = Arc::new(Barrier::new(num_clients));
        
        let tasks: Vec<_> = (0..num_clients)
            .map(|i| {
                let server_url = server.http_url();
                let barrier = barrier.clone();
                
                tokio::spawn(async move {
                    // Wait for all tasks to be ready
                    barrier.wait().await;
                    
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(5))
                        .build()?;

                    let init = json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": {
                            "protocolVersion": "2025-06-18",
                            "capabilities": {},
                            "clientInfo": {"name": format!("client-{}", i), "version": "1.0"}
                        }
                    });

                    let response = client.post(&server_url).json(&init).send().await?;
                    assert_eq!(response.status(), 200);
                    
                    let body: serde_json::Value = response.json().await?;
                    assert!(body.get("result").is_some(), "Client {} failed to initialize", i);
                    
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(i)
                })
            })
            .collect();

        let results = join_all(tasks).await;
        
        // All clients should succeed
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Task {} failed: {:?}", i, result);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 2: RED - Concurrent operations on same session
#[tokio::test]
async fn test_concurrent_same_session_operations() {
    // Test multiple concurrent operations on the same session
    with_mcp_test_server("concurrent_same_session_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // Initialize session
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "concurrent-test", "version": "1.0"}
            }
        });

        client.post(&server.http_url()).json(&init).send().await?;

        // Now send multiple concurrent requests on same session
        let num_requests = 20;
        let tasks: Vec<_> = (0..num_requests)
            .map(|i| {
                let client = client.clone();
                let server_url = server.http_url();
                
                tokio::spawn(async move {
                    let request = json!({
                        "jsonrpc": "2.0",
                        "id": i + 100,
                        "method": "tools/call",
                        "params": {
                            "name": "echo",
                            "arguments": {
                                "message": format!("concurrent message {}", i)
                            }
                        }
                    });

                    let response = client.post(&server_url).json(&request).send().await?;
                    let body: serde_json::Value = response.json().await?;
                    
                    // Verify we got the correct response
                    assert_eq!(body["id"], i + 100);
                    assert!(
                        body.get("result").is_some() || body.get("error").is_some(),
                        "Request {} got neither result nor error",
                        i
                    );
                    
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(i)
                })
            })
            .collect();

        let results = join_all(tasks).await;
        
        // All requests should complete
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Request {} failed: {:?}", i, result);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 3: RED - Session isolation under load
#[tokio::test]
async fn test_session_isolation_under_load() {
    // Test that sessions remain isolated even under concurrent load
    with_mcp_test_server("session_isolation_load_test", |server| async move {
        let num_sessions = 5;
        let requests_per_session = 10;
        
        let tasks: Vec<_> = (0..num_sessions)
            .map(|session_id| {
                let server_url = server.http_url();
                
                tokio::spawn(async move {
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(5))
                        .cookie_store(true)
                        .build()?;

                    // Initialize with unique version per session
                    let version = if session_id % 2 == 0 { "2025-06-18" } else { "2025-03-26" };
                    let init = json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": {
                            "protocolVersion": version,
                            "capabilities": {},
                            "clientInfo": {
                                "name": format!("session-{}", session_id),
                                "version": "1.0"
                            }
                        }
                    });

                    let response = client.post(&server_url).json(&init).send().await?;
                    let body: serde_json::Value = response.json().await?;
                    assert_eq!(body["result"]["protocolVersion"], version);

                    // Send multiple requests
                    for req_id in 0..requests_per_session {
                        let request = json!({
                            "jsonrpc": "2.0",
                            "id": req_id + 100,
                            "method": "tools/list"
                        });

                        let response = client.post(&server_url).json(&request).send().await?;
                        let body: serde_json::Value = response.json().await?;
                        assert!(body.get("result").is_some());
                    }

                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(session_id)
                })
            })
            .collect();

        let results = join_all(tasks).await;
        
        // All sessions should complete successfully
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Session {} failed: {:?}", i, result);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 4: RED - Race condition on session initialization
#[tokio::test]
async fn test_session_init_race_condition() {
    // Test race condition where multiple requests arrive before initialization completes
    with_mcp_test_server("init_race_condition_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        let barrier = Arc::new(Barrier::new(3));

        // Task 1: Initialize
        let init_task = {
            let client = client.clone();
            let server_url = server.http_url();
            let barrier = barrier.clone();
            
            tokio::spawn(async move {
                barrier.wait().await;
                
                let init = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": "race-test", "version": "1.0"}
                    }
                });

                client.post(&server_url).json(&init).send().await
            })
        };

        // Task 2: Try to use tools immediately
        let tools_task = {
            let client = client.clone();
            let server_url = server.http_url();
            let barrier = barrier.clone();
            
            tokio::spawn(async move {
                barrier.wait().await;
                // Small delay to let init start but not complete
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                let tools = json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/list"
                });

                client.post(&server_url).json(&tools).send().await
            })
        };

        // Task 3: Another concurrent request
        let echo_task = {
            let client = client.clone();
            let server_url = server.http_url();
            let barrier = barrier.clone();
            
            tokio::spawn(async move {
                barrier.wait().await;
                // Small delay
                tokio::time::sleep(Duration::from_millis(15)).await;
                
                let echo = json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {
                        "name": "echo",
                        "arguments": {"message": "race test"}
                    }
                });

                client.post(&server_url).json(&echo).send().await
            })
        };

        // Wait for all tasks
        let (init_result, tools_result, echo_result) = 
            tokio::join!(init_task, tools_task, echo_task);

        // All should complete without panic
        assert!(init_result.is_ok());
        assert!(tools_result.is_ok());
        assert!(echo_result.is_ok());

        // Check responses
        if let Ok(Ok(response)) = init_result {
            assert_eq!(response.status(), 200);
        }

        // Other requests might fail with "not initialized" or succeed if init completed
        if let Ok(Ok(response)) = tools_result {
            assert_eq!(response.status(), 200);
        }

        if let Ok(Ok(response)) = echo_result {
            assert_eq!(response.status(), 200);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 5: RED - Session cleanup and resource management
#[tokio::test]
async fn test_session_cleanup() {
    // Test that sessions are properly cleaned up
    with_mcp_test_server("session_cleanup_test", |server| async move {
        // Create many short-lived sessions
        for i in 0..50 {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .cookie_store(true)
                .build()?;

            let init = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": format!("temp-client-{}", i), "version": "1.0"}
                }
            });

            let response = client.post(&server.http_url()).json(&init).send().await?;
            assert_eq!(response.status(), 200);
            
            // Make one request
            let tools = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            });
            
            client.post(&server.http_url()).json(&tools).send().await?;
            
            // Drop client to simulate disconnect
            drop(client);
        }

        // Server should still be responsive after many sessions
        let final_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        let final_init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "final-client", "version": "1.0"}
            }
        });

        let response = final_client.post(&server.http_url()).json(&final_init).send().await?;
        assert_eq!(response.status(), 200);

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 6: RED - Concurrent WebSocket and HTTP sessions
#[tokio::test]
async fn test_concurrent_websocket_http_sessions() {
    // Test concurrent WebSocket and HTTP sessions
    with_mcp_test_server("concurrent_ws_http_test", |server| async move {
        let barrier = Arc::new(Barrier::new(2));

        // WebSocket session task
        let ws_task = {
            let ws_url = server.ws_url();
            let barrier = barrier.clone();
            
            tokio::spawn(async move {
                barrier.wait().await;
                
                let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
                let (mut write, mut read) = ws_stream.split();
                
                // Initialize WebSocket
                use futures_util::SinkExt;
                use tokio_tungstenite::tungstenite::Message;
                
                let init = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": "ws-client", "version": "1.0"}
                    }
                });
                
                write.send(Message::Text(serde_json::to_string(&init)?.into())).await?;
                
                // Read response
                use mcp_test_helpers::receive_ws_message;
                let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
                
                // Send multiple messages
                for i in 0..10 {
                    let msg = json!({
                        "jsonrpc": "2.0",
                        "id": i + 10,
                        "method": "tools/list"
                    });
                    
                    write.send(Message::Text(serde_json::to_string(&msg)?.into())).await?;
                    let _response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
                }
                
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })
        };

        // HTTP session task
        let http_task = {
            let server_url = server.http_url();
            let barrier = barrier.clone();
            
            tokio::spawn(async move {
                barrier.wait().await;
                
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .cookie_store(true)
                    .build()?;

                // Initialize HTTP
                let init = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-03-26",
                        "capabilities": {},
                        "clientInfo": {"name": "http-client", "version": "1.0"}
                    }
                });

                client.post(&server_url).json(&init).send().await?;

                // Send multiple requests
                for i in 0..10 {
                    let request = json!({
                        "jsonrpc": "2.0",
                        "id": i + 100,
                        "method": "tools/call",
                        "params": {
                            "name": "echo",
                            "arguments": {"message": format!("http message {}", i)}
                        }
                    });

                    client.post(&server_url).json(&request).send().await?;
                }
                
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })
        };

        // Both should complete successfully
        let (ws_result, http_result) = tokio::join!(ws_task, http_task);
        
        assert!(ws_result.is_ok() && ws_result.unwrap().is_ok(), "WebSocket task failed");
        assert!(http_result.is_ok() && http_result.unwrap().is_ok(), "HTTP task failed");

        Ok(())
    })
    .await
    .unwrap();
}