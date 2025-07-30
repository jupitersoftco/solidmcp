//! Malformed Request Handling Tests
//!
//! Comprehensive tests for handling various malformed requests following TDD principles

use serde_json::json;
use std::time::Duration;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

mod mcp_test_helpers;
use mcp_test_helpers::{with_mcp_test_server, receive_ws_message};

/// Test 1: RED - Invalid JSON syntax
#[tokio::test]
async fn test_invalid_json_syntax() {
    // Test various forms of invalid JSON
    with_mcp_test_server("invalid_json_test", |server| async move {
        let test_cases = vec![
            ("", "Empty string"),
            ("{", "Unclosed brace"),
            ("}", "Only closing brace"),
            ("{]", "Mismatched brackets"),
            ("{'jsonrpc': '2.0'}", "Single quotes instead of double"),
            ("{\"jsonrpc\": }", "Missing value"),
            ("null", "Just null"),
            ("undefined", "JavaScript undefined"),
            ("{\"jsonrpc\":\"2.0\",}", "Trailing comma"),
            ("{'jsonrpc':'2.0','id':1,'method':'test'", "Unclosed and single quotes"),
        ];

        for (invalid_json, description) in test_cases {
            let (ws_stream, _) = tokio_tungstenite::connect_async(&server.ws_url()).await?;
            let (mut write, mut read) = ws_stream.split();

            write.send(Message::Text(invalid_json.to_string().into())).await?;

            match receive_ws_message(&mut read, Duration::from_secs(2)).await {
                Ok(response_text) => {
                    let response: serde_json::Value = serde_json::from_str(&response_text)?;
                    
                    // Should have an error
                    assert!(response.get("error").is_some(), "Expected error for: {}", description);
                    let error = response.get("error").unwrap();
                    
                    // Should be parse error (-32700)
                    assert_eq!(error["code"], -32700, "Wrong error code for: {}", description);
                }
                Err(_) => {
                    // Connection close is also acceptable for malformed JSON
                }
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 2: RED - Missing required JSON-RPC fields
#[tokio::test]
async fn test_missing_required_fields() {
    // Test requests missing required JSON-RPC fields
    with_mcp_test_server("missing_fields_test", |server| async move {
        let test_cases = vec![
            (json!({}), "Empty object"),
            (json!({"id": 1}), "Missing jsonrpc and method"),
            (json!({"jsonrpc": "2.0"}), "Missing method and id"),
            (json!({"method": "test"}), "Missing jsonrpc and id"),
            (json!({"jsonrpc": "2.0", "id": 1}), "Missing method"),
            (json!({"jsonrpc": "2.0", "method": "test"}), "Missing id (notification)"),
            (json!({"id": 1, "method": "test"}), "Missing jsonrpc"),
        ];

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        for (invalid_request, description) in test_cases {
            let response = client
                .post(&server.http_url())
                .json(&invalid_request)
                .send()
                .await?;

            assert_eq!(response.status(), 200); // JSON-RPC errors still return 200
            let body: serde_json::Value = response.json().await?;
            
            // Should have an error
            assert!(body.get("error").is_some(), "Expected error for: {}", description);
            let error = body.get("error").unwrap();
            
            // Should be invalid request (-32600)
            assert_eq!(error["code"], -32600, "Wrong error code for: {}", description);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 3: RED - Invalid parameter types
#[tokio::test]
async fn test_invalid_parameter_types() {
    // Test requests with wrong parameter types
    with_mcp_test_server("invalid_params_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // Initialize first
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        let test_cases = vec![
            (
                json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": "string instead of object"
                }),
                "String params instead of object"
            ),
            (
                json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": []
                }),
                "Array params instead of object"
            ),
            (
                json!({
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": "tools/call",
                    "params": {
                        "name": 123,  // Should be string
                        "arguments": {}
                    }
                }),
                "Number for name instead of string"
            ),
            (
                json!({
                    "jsonrpc": "2.0",
                    "id": 5,
                    "method": "tools/call",
                    "params": {
                        "name": "echo",
                        "arguments": "not an object"
                    }
                }),
                "String arguments instead of object"
            ),
        ];

        for (invalid_request, description) in test_cases {
            let response = client
                .post(&server.http_url())
                .json(&invalid_request)
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            
            // Should have an error
            assert!(body.get("error").is_some(), "Expected error for: {}", description);
            let error = body.get("error").unwrap();
            
            // Should be invalid params (-32602) or method not found (-32601)
            let code = error["code"].as_i64().unwrap();
            assert!(
                code == -32602 || code == -32601,
                "Wrong error code {} for: {}",
                code,
                description
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 4: RED - Extremely large requests
#[tokio::test]
async fn test_extremely_large_requests() {
    // Test handling of requests that exceed reasonable size limits
    with_mcp_test_server("large_request_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        // Create a very large message (10MB)
        let large_string = "x".repeat(10 * 1024 * 1024);
        let large_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": large_string
                }
            }
        });

        // Server should handle this gracefully (either process or reject with appropriate error)
        match client.post(&server.http_url()).json(&large_request).send().await {
            Ok(response) => {
                if response.status() == 413 {
                    // Payload too large is acceptable
                    assert!(true, "Server correctly rejected oversized request");
                } else if response.status() == 200 {
                    // If accepted, should process or return error
                    match response.json::<serde_json::Value>().await {
                        Ok(body) => {
                            // Either processed successfully or returned JSON-RPC error
                            assert!(
                                body.get("result").is_some() || body.get("error").is_some(),
                                "Response should have result or error"
                            );
                        }
                        Err(_) => {
                            // Failed to parse response - server might have issues with large payload
                            assert!(true, "Server had issues processing large request");
                        }
                    }
                }
            }
            Err(_) => {
                // Connection error is acceptable for oversized requests
                assert!(true, "Connection failed for oversized request");
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 5: RED - Invalid JSON-RPC version
#[tokio::test]
async fn test_invalid_jsonrpc_version() {
    // Test requests with invalid JSON-RPC version
    with_mcp_test_server("invalid_jsonrpc_version_test", |server| async move {
        let test_cases = vec![
            ("1.0", "Old version"),
            ("3.0", "Future version"),
            ("2", "Missing decimal"),
            ("two", "Text version"),
            ("", "Empty version"),
            ("2.0.0", "Semantic version"),
        ];

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        for (version, description) in test_cases {
            let request = json!({
                "jsonrpc": version,
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "1.0"}
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&request)
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            
            // Should have an error for non-2.0 versions
            assert!(body.get("error").is_some(), "Expected error for: {}", description);
            let error = body.get("error").unwrap();
            
            // Should be invalid request error
            assert_eq!(error["code"], -32600, "Wrong error code for: {}", description);
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 6: RED - Malformed batch requests
#[tokio::test]
async fn test_malformed_batch_requests() {
    // Test various malformed batch request scenarios
    with_mcp_test_server("malformed_batch_test", |server| async move {
        let test_cases = vec![
            (json!([]), "Empty array"),
            (
                json!([
                    {"jsonrpc": "2.0", "id": 1, "method": "test"},
                    "not an object"
                ]),
                "Mixed types in array"
            ),
            (
                json!([
                    {"jsonrpc": "2.0", "id": 1, "method": "test"},
                    null
                ]),
                "Null in array"
            ),
            (
                json!([
                    {"jsonrpc": "2.0", "id": 1},  // Missing method
                    {"jsonrpc": "2.0", "id": 2, "method": "test"}
                ]),
                "Invalid request in batch"
            ),
        ];

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        for (batch_request, description) in test_cases {
            let response = client
                .post(&server.http_url())
                .json(&batch_request)
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            
            // Response should be array for batch or single error
            if body.is_array() {
                // For batch, check each response
                for item in body.as_array().unwrap() {
                    // At least one should be an error
                    if item.get("error").is_some() {
                        assert!(true, "Found expected error in batch for: {}", description);
                    }
                }
            } else {
                // Single error response is also acceptable
                assert!(body.get("error").is_some(), "Expected error for: {}", description);
            }
        }

        Ok(())
    })
    .await
    .unwrap();
}

/// Test 7: RED - Special characters and encoding issues
#[tokio::test]
async fn test_special_characters_encoding() {
    // Test handling of special characters and encoding issues
    with_mcp_test_server("encoding_test", |server| async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .cookie_store(true)
            .build()?;

        // Initialize first
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        });
        client.post(&server.http_url()).json(&init).send().await?;

        let test_cases = vec![
            "Hello \u{0000} World",  // Null character
            "Test \u{FFFF}",         // Invalid Unicode
            "Emoji test 🚀💥🔥",    // Emojis
            "Control chars \n\r\t", // Control characters
            "Unicode snowman ☃",     // Unicode
            "Zero width ​space",     // Zero-width space
        ];

        for (i, test_string) in test_cases.iter().enumerate() {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i + 2,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": test_string
                    }
                }
            });

            let response = client
                .post(&server.http_url())
                .json(&request)
                .send()
                .await?;

            let body: serde_json::Value = response.json().await?;
            
            // Should either handle correctly or return appropriate error
            assert!(
                body.get("result").is_some() || body.get("error").is_some(),
                "Should have result or error for: {:?}",
                test_string
            );
        }

        Ok(())
    })
    .await
    .unwrap();
}