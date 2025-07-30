//! Progress Notification Error Handling Test
//!
//! Tests that progress notifications handle errors gracefully

use serde_json::json;
use std::time::Duration;

mod mcp_test_helpers;
use mcp_test_helpers::with_mcp_test_server;

#[tokio::test]
async fn test_progress_notification_serialization_error() {
    // Test that serialization errors in progress notifications don't crash
    with_mcp_test_server("progress_serialization_test", |server| async move {
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

        // Create a tool call with progress token containing problematic data
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "test"
                },
                "_meta": {
                    "progressToken": {"complex": "object", "that": "might", "fail": true}
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Should handle complex progress token without crashing
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("result").is_some() || body.get("error").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_progress_notification_missing_fields() {
    // Test that missing fields in progress meta don't crash
    with_mcp_test_server("progress_missing_fields_test", |server| async move {
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

        // Create a tool call with empty _meta
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "test"
                },
                "_meta": {}
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Should handle empty meta without crashing
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_progress_notification_null_token() {
    // Test that null progress token is handled properly
    with_mcp_test_server("progress_null_token_test", |server| async move {
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

        // Create a tool call with null progress token
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "test"
                },
                "_meta": {
                    "progressToken": null
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Should handle null token without crashing
        let body: serde_json::Value = response.json().await?;
        assert!(body.get("result").is_some());

        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_chunked_response_headers() {
    // Test that chunked responses have proper headers
    with_mcp_test_server("chunked_headers_test", |server| async move {
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

        // Create a tool call with valid progress token
        let tool_call = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": {
                    "message": "test with progress"
                },
                "_meta": {
                    "progressToken": "valid-token-123"
                }
            }
        });

        let response = client
            .post(&server.http_url())
            .json(&tool_call)
            .send()
            .await?;

        assert_eq!(response.status(), 200);

        // Should have chunked encoding
        let headers = response.headers();
        assert_eq!(
            headers
                .get("transfer-encoding")
                .map(|v| v.to_str().unwrap()),
            Some("chunked")
        );

        // Should NOT have content-length
        assert!(headers.get("content-length").is_none());

        Ok(())
    })
    .await
    .unwrap();
}
