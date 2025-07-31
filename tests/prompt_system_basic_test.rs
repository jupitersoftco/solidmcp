//! Basic Prompt System Tests
//!
//! Tests core prompt functionality including listing prompts, retrieving prompt content,
//! and basic parameter substitution across HTTP and WebSocket transports.

use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use futures_util::SinkExt;
use anyhow::Result;

mod mcp_test_helpers;
use mcp_test_helpers::*;

#[tokio::test]
async fn test_prompt_list_websocket() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_mcp_connection("prompt_list_ws", |_server, mut write, mut read| async move {
        // Request prompts list
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/list",
            "params": {}
        });

        write.send(Message::Text(list_request.to_string().into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        // Validate response structure
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("result").is_some());
        
        let result = &response["result"];
        assert!(result.get("prompts").is_some());
        
        let prompts = result["prompts"].as_array().unwrap();
        // Basic server should have no prompts by default
        assert_eq!(prompts.len(), 0);

        Ok(())
    }).await
}

#[tokio::test]
async fn test_prompt_list_http() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_mcp_test_server("prompt_list_http", |server| async move {
        let client = reqwest::Client::new();
        
        // Initialize session first
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

        let init_response = client
            .post(&server.http_url())
            .json(&init_request)
            .send()
            .await?;

        let cookies = init_response.headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect::<Vec<_>>();

        // Extract session cookie
        let session_cookie = cookies.iter()
            .find(|c| c.starts_with("mcp_session="))
            .expect("Should have session cookie");

        // Request prompts list
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": {}
        });

        let response = client
            .post(&server.http_url())
            .header("Cookie", *session_cookie)
            .json(&list_request)
            .send()
            .await?;

        let response_json: Value = response.json().await?;

        // Validate response
        assert_eq!(response_json["jsonrpc"], "2.0");
        assert_eq!(response_json["id"], 2);
        assert!(response_json.get("result").is_some());

        let result = &response_json["result"];
        assert!(result.get("prompts").is_some());

        Ok(())
    }).await
}

#[tokio::test]
async fn test_prompt_get_not_found() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_mcp_connection("prompt_get_not_found", |_server, mut write, mut read| async move {
        // Request non-existent prompt
        let get_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/get",
            "params": {
                "name": "nonexistent_prompt",
                "arguments": {}
            }
        });

        write.send(Message::Text(get_request.to_string().into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        // Should return an error
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("error").is_some());
        
        let error = &response["error"];
        assert!(error["message"].as_str().unwrap().contains("Prompt not found"));

        Ok(())
    }).await
}

#[tokio::test]
async fn test_prompt_invalid_method() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_mcp_connection("prompt_invalid_method", |_server, mut write, mut read| async move {
        // Request with invalid method
        let invalid_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/invalid",
            "params": {}
        });

        write.send(Message::Text(invalid_request.to_string().into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        // Should return method not found error
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("error").is_some());
        
        let error = &response["error"];
        assert_eq!(error["code"], -32601); // Method not found

        Ok(())
    }).await
}

#[tokio::test]
async fn test_prompt_malformed_request() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_mcp_connection("prompt_malformed", |_server, mut write, mut read| async move {
        // Send malformed JSON
        let malformed_json = r#"{"jsonrpc": "2.0", "id": 1, "method": "prompts/list", "params": {INVALID}"#;

        write.send(Message::Text(malformed_json.to_string().into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        // Should return parse error
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response.get("error").is_some());
        
        let error = &response["error"];
        assert_eq!(error["code"], -32700); // Parse error

        Ok(())
    }).await
}

#[tokio::test]
async fn test_prompt_missing_params() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    with_mcp_connection("prompt_missing_params", |_server, mut write, mut read| async move {
        // Request prompts/get without required name parameter
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "prompts/get",
            "params": {
                "arguments": {}
            }
        });

        write.send(Message::Text(request.to_string().into())).await?;
        let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
        let response: Value = serde_json::from_str(&response_text)?;

        // Should return invalid params error
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response.get("error").is_some());
        
        let error = &response["error"];
        assert_eq!(error["code"], -32602); // Invalid params

        Ok(())
    }).await
}