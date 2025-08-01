//! Comprehensive End-to-End Progress Notification Tests
//!
//! Tests comprehensive progress notification functionality across both HTTP and WebSocket transports
//! including streaming responses, chunked encoding, JSON-RPC compliance, and edge cases.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

mod mcp_test_helpers;
use mcp_test_helpers::{with_mcp_test_server, with_mcp_connection};

// ============================================================================
// HTTP Transport Progress Notification Tests
// ============================================================================

#[tokio::test]
async fn test_http_progress_notifications_with_chunked_encoding() {
    // Test basic progress notification flow with HTTP chunked encoding
    with_mcp_test_server("http_progress_chunked", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        // Tool call with progress token
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "test with progress"},
                "_meta": {
                    "progressToken": "progress-test-token-001"
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        // Verify chunked encoding is used
        assert_eq!(response.status(), 200);
        let headers = response.headers();
        assert_eq!(
            headers.get("transfer-encoding").map(|v| v.to_str().unwrap()),
            Some("chunked"),
            "Should use chunked encoding for progress tokens"
        );
        assert!(
            !headers.contains_key("content-length"),
            "Should not have content-length with chunked encoding"
        );

        // Verify response is valid
        let body: Value = response.json().await?;
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 2);
        assert!(body.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_http_progress_notifications_with_different_token_types() {
    // Test various progress token formats (string, object, number)
    with_mcp_test_server("http_progress_token_types", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        // Test different token types
        let test_cases = vec![
            ("string", json!("string-token-123")),
            ("number", json!(42)),
            ("object", json!({"id": "obj-token", "session": "test"})),
            ("array", json!(["token", "array", 123])),
        ];

        for (test_name, token) in test_cases {
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": format!("test-{}", test_name),
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {"message": format!("test-{}", test_name)},
                    "_meta": {
                        "progressToken": token
                    }
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&tool_call)
                .send()
                .await?;

            assert_eq!(response.status(), 200, "Failed for token type: {}", test_name);
            
            // Should use chunked encoding regardless of token type
            let headers = response.headers();
            assert_eq!(
                headers.get("transfer-encoding").map(|v| v.to_str().unwrap()),
                Some("chunked"),
                "Should use chunked encoding for {} token", test_name
            );

            let body: Value = response.json().await?;
            assert!(body.get("result").is_some() || body.get("error").is_some(), 
                   "Should have result or error for {}", test_name);
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_http_no_chunked_encoding_without_progress_token() {
    // Verify that requests without progress tokens use Content-Length
    with_mcp_test_server("http_no_progress_content_length", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        let response = client.post(&server.http_url()).json(&init).send().await?;

        // Should use Content-Length, not chunked encoding
        let headers = response.headers();
        assert!(
            headers.contains_key("content-length"),
            "Should have content-length without progress token"
        );
        assert!(
            !headers.contains_key("transfer-encoding"),
            "Should not have transfer-encoding without progress token"
        );

        // Regular tool call without progress token
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "test without progress"}
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        let headers = response.headers();
        assert!(
            headers.contains_key("content-length"),
            "Regular requests should use content-length"
        );
        assert!(
            !headers.contains_key("transfer-encoding"),
            "Regular requests should not use transfer-encoding"
        );

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_http_progress_with_session_management() {
    // Test progress notifications work correctly with HTTP session management
    with_mcp_test_server("http_progress_sessions", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .cookie_store(true)  // Enable cookie jar
            .build()?;

        // Initialize to establish session
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "session-test", "version": "1.0"}
            }
        });
        
        let init_response = client.post(&server.http_url()).json(&init).send().await?;
        assert_eq!(init_response.status(), 200);

        // Verify session cookie was set
        let cookies = init_response.headers().get_all("set-cookie");
        let has_session_cookie = cookies.iter().any(|cookie| {
            cookie.to_str().unwrap_or("").contains("mcp_session=")
        });
        assert!(has_session_cookie, "Should set session cookie on initialize");

        // Multiple tool calls with progress in same session
        for i in 1..=3 {
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {"message": format!("session test {}", i)},
                    "_meta": {
                        "progressToken": format!("session-progress-{}", i)
                    }
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&tool_call)
                .send()
                .await?;

            assert_eq!(response.status(), 200);
            // Should still use chunked encoding
            assert_eq!(
                response.headers().get("transfer-encoding").map(|v| v.to_str().unwrap()),
                Some("chunked")
            );

            let body: Value = response.json().await?;
            assert!(body.get("result").is_some());
        }

        Ok(())
    })
    .await
    .unwrap();
}

// ============================================================================
// WebSocket Transport Progress Notification Tests
// ============================================================================

#[tokio::test]
async fn test_websocket_progress_notifications_basic_flow() {
    // Test basic progress notification flow over WebSocket
    with_mcp_connection("ws_progress_basic", |_server, mut write, mut read| async move {
        // Tool call with progress token
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "websocket progress test"},
                "_meta": {
                    "progressToken": "ws-progress-token-001"
                }
            }
        });

        write.send(Message::Text(tool_call.to_string().into())).await?;

        // Read response - should get the final result
        let response_text = timeout(Duration::from_secs(5), async {
            while let Some(Ok(Message::Text(text))) = read.next().await {
                let msg: Value = serde_json::from_str(&text.to_string())?;
                if msg.get("id") == Some(&json!(2)) {
                    return Ok::<String, anyhow::Error>(text.to_string());
                }
            }
            Err(anyhow::anyhow!("No response received"))
        }).await??;

        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 2);
        assert!(response.get("result").is_some(), "Should have result");

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_websocket_concurrent_progress_notifications() {
    // Test multiple concurrent progress notifications over WebSocket
    with_mcp_connection("ws_concurrent_progress", |_server, mut write, mut read| async move {
        // Send multiple tool calls with different progress tokens concurrently
        let progress_tokens = vec![
            "concurrent-token-1",
            "concurrent-token-2", 
            "concurrent-token-3",
            "concurrent-token-4",
            "concurrent-token-5"
        ];

        // Send all requests
        for (i, token) in progress_tokens.iter().enumerate() {
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {"message": format!("concurrent test {}", i)},
                    "_meta": {
                        "progressToken": token
                    }
                }
            });

            write.send(Message::Text(tool_call.to_string().into())).await?;
        }

        // Collect all responses
        let mut received_ids = std::collections::HashSet::new();
        let timeout_duration = Duration::from_secs(10);
        
        let responses = timeout(timeout_duration, async {
            let mut responses = Vec::new();
            while received_ids.len() < progress_tokens.len() {
                if let Some(Ok(Message::Text(text))) = read.next().await {
                    let msg: Value = serde_json::from_str(&text.to_string())?;
                    if let Some(id) = msg.get("id").and_then(|id| id.as_u64()) {
                        if id >= 2 && id < 2 + progress_tokens.len() as u64 {
                            received_ids.insert(id);
                            responses.push(msg);
                        }
                    }
                }
            }
            Ok::<Vec<Value>, anyhow::Error>(responses)
        }).await??;

        // Verify all responses received
        assert_eq!(responses.len(), progress_tokens.len());
        for response in responses {
            assert_eq!(response["jsonrpc"], "2.0");
            assert!(response.get("result").is_some());
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_websocket_progress_with_large_payloads() {
    // Test progress notifications with large message payloads
    with_mcp_connection("ws_progress_large", |_server, mut write, mut read| async move {
        // Create large payload
        let large_data = (0..10000).map(|i| format!("data-item-{}", i)).collect::<Vec<_>>();
        
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "large payload test",
                    "large_data": large_data
                },
                "_meta": {
                    "progressToken": "large-payload-token"
                }
            }
        });

        write.send(Message::Text(tool_call.to_string().into())).await?;

        // Should handle large payloads without issues
        let response_text = timeout(Duration::from_secs(10), async {
            while let Some(Ok(Message::Text(text))) = read.next().await {
                let msg: Value = serde_json::from_str(&text.to_string())?;
                if msg.get("id") == Some(&json!(2)) {
                    return Ok::<String, anyhow::Error>(text.to_string());
                }
            }
            Err(anyhow::anyhow!("No response received"))
        }).await??;

        let response: Value = serde_json::from_str(&response_text)?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

// ============================================================================
// Edge Cases and Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_malformed_progress_tokens() {
    // Test handling of malformed or edge-case progress tokens
    with_mcp_test_server("malformed_progress_tokens", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        // Test edge cases
        let test_cases = vec![
            ("null_token", json!(null)),
            ("empty_string", json!("")),
            ("very_long_string", json!("x".repeat(10000))),
            ("special_chars", json!("token-with-!@#$%^&*()_+-={}[]|\\:;\"'<>?,./~`")),
            ("unicode", json!("🚀💻📡🎯✅❌⚡🔄📊")),
            ("nested_object", json!({"level1": {"level2": {"token": "deep"}}})),
        ];

        for (test_name, token) in test_cases {
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": format!("test-{}", test_name),
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {"message": format!("test-{}", test_name)},
                    "_meta": {
                        "progressToken": token
                    }
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&tool_call)
                .send()
                .await?;

            // Should handle gracefully (not crash)
            assert_eq!(response.status(), 200, "Failed for test case: {}", test_name);
            
            let body: Value = response.json().await?;
            assert!(
                body.get("result").is_some() || body.get("error").is_some(),
                "Should have result or error for {}", test_name
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_missing_progress_token_fields() {
    // Test requests with malformed _meta fields
    with_mcp_test_server("missing_progress_fields", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        // Test various malformed _meta scenarios
        let test_cases = vec![
            ("empty_meta", json!({})),
            ("null_meta", json!(null)),
            ("missing_progress_token", json!({"other": "field"})),
            ("progress_token_typo", json!({"progresstoken": "typo"})),
        ];

        for (test_name, meta) in test_cases {
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": format!("test-{}", test_name),
                "method": "tools/call",
                "params": {
                    "name": "echo",  
                    "arguments": {"message": format!("test-{}", test_name)},
                    "_meta": meta
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&tool_call)
                .send()
                .await?;

            // Should not use chunked encoding without valid progress token
            let headers = response.headers();
            assert!(
                headers.contains_key("content-length"),
                "Should use content-length for {}", test_name
            );
            assert!(
                !headers.contains_key("transfer-encoding"),
                "Should not use chunked encoding for {}", test_name
            );

            let body: Value = response.json().await?;
            assert!(body.get("result").is_some(), "Should have result for {}", test_name);
        }

        Ok(())
    })
    .await
    .unwrap();
}

// ============================================================================
// JSON-RPC Compliance Tests
// ============================================================================

#[tokio::test]
async fn test_progress_notification_jsonrpc_compliance() {
    // Test that all progress-related responses follow JSON-RPC 2.0 spec
    with_mcp_test_server("jsonrpc_compliance", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        let init_response = client.post(&server.http_url()).json(&init).send().await?;
        let init_body: Value = init_response.json().await?;
        
        // Verify initialize response compliance
        assert_eq!(init_body["jsonrpc"], "2.0");
        assert_eq!(init_body["id"], 1);
        assert!(init_body.get("result").is_some());

        // Tool call with progress token
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {"message": "compliance test"},
                "_meta": {
                    "progressToken": "compliance-token"
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        let body: Value = response.json().await?;
        
        // Verify JSON-RPC 2.0 compliance
        assert_eq!(body["jsonrpc"], "2.0", "Must have jsonrpc field with value '2.0'");
        assert_eq!(body["id"], 42, "Must echo the request ID");
        
        // Must have either result or error, but not both
        let has_result = body.get("result").is_some();
        let has_error = body.get("error").is_some();
        assert!(
            has_result || has_error,
            "Response must have either 'result' or 'error' field"
        );
        assert!(
            !(has_result && has_error),
            "Response must not have both 'result' and 'error' fields"
        );

        if has_error {
            let error = &body["error"];
            assert!(error.get("code").is_some(), "Error must have 'code' field");
            assert!(error.get("message").is_some(), "Error must have 'message' field");
            
            // Verify error code is valid JSON-RPC error code
            if let Some(code) = error["code"].as_i64() {
                assert!(
                    code >= -32768 && code <= -32000 || code >= -1 && code <= 1000,
                    "Error code should be in valid JSON-RPC range"
                );
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

// ============================================================================
// Performance and Load Tests
// ============================================================================

#[tokio::test]
async fn test_progress_notifications_performance() {
    // Test performance with rapid progress notifications
    with_mcp_test_server("progress_performance", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        let start_time = std::time::Instant::now();
        let num_requests = 50;

        // Send multiple requests with progress tokens rapidly
        let mut handles = Vec::new();
        for i in 0..num_requests {
            let client = client.clone();
            let url = server.http_url();
            
            let handle = tokio::spawn(async move {
                let tool_call = json!({
                    "jsonrpc": "2.0",
                    "id": i + 2,
                    "method": "tools/call",
                    "params": {
                        "name": "echo",
                        "arguments": {"message": format!("perf test {}", i)},
                        "_meta": {
                            "progressToken": format!("perf-token-{}", i)
                        }
                    }
                });

                let response = client.post(&url).json(&tool_call).send().await?;
                let body: Value = response.json().await?;
                
                Ok::<Value, anyhow::Error>(body)
            });
            
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut success_count = 0;
        for handle in handles {
            match handle.await? {
                Ok(body) => {
                    if body.get("result").is_some() {
                        success_count += 1;
                    }
                }
                Err(e) => println!("Request failed: {}", e),
            }
        }

        let elapsed = start_time.elapsed();
        println!(
            "Performance test: {}/{} requests succeeded in {:?} ({:.2} req/sec)",
            success_count, num_requests, elapsed, 
            num_requests as f64 / elapsed.as_secs_f64()
        );

        // Should handle at least 80% of requests successfully
        assert!(
            success_count >= (num_requests * 4 / 5),
            "Should handle most requests successfully under load"
        );

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_websocket_progress_notifications_stress() {
    // Stress test WebSocket progress notifications
    with_mcp_connection("ws_progress_stress", |_server, mut write, mut read| async move {
        let num_requests = 20;
        let start_time = std::time::Instant::now();

        // Send rapid progress requests
        for i in 0..num_requests {
            let tool_call = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {"message": format!("stress test {}", i)},
                    "_meta": {
                        "progressToken": format!("stress-token-{}", i)
                    }
                }
            });

            write.send(Message::Text(tool_call.to_string().into())).await?;
        }

        // Collect responses
        let mut received_responses = 0;
        let timeout_duration = Duration::from_secs(20);
        
        timeout(timeout_duration, async {
            while received_responses < num_requests {
                if let Some(Ok(Message::Text(text))) = read.next().await {
                    let msg: Value = serde_json::from_str(&text.to_string())?;
                    if let Some(id) = msg.get("id").and_then(|id| id.as_u64()) {
                        if id >= 2 && id < 2 + num_requests as u64 {
                            received_responses += 1;
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        }).await??;

        let elapsed = start_time.elapsed();
        println!(
            "WebSocket stress test: {}/{} responses in {:?} ({:.2} req/sec)",
            received_responses, num_requests, elapsed,
            num_requests as f64 / elapsed.as_secs_f64()
        );

        assert_eq!(received_responses, num_requests, "Should receive all responses");

        Ok(())
    })
    .await
    .unwrap();
}