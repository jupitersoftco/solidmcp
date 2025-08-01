//! Integration tests for MCP protocol error handling
//!
//! These tests verify that error conditions are handled correctly
//! and follow the JSON-RPC 2.0 error specification.

mod helpers;

use helpers::{TestServer, McpHttpClient, assert_json_rpc_error, json_rpc_request};
use serde_json::json;

#[tokio::test]
async fn test_not_initialized_error() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Try to list tools without initializing first
    let response = client.list_tools(&url).await.unwrap();
    
    // Should get a "not initialized" error
    assert_json_rpc_error(&response, 2, -32002); // Custom error code for not initialized
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("not initialized"));
    
    server.stop();
}

#[tokio::test]
async fn test_unknown_method_error() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize first
    client.initialize(&url, "error-test-client").await.unwrap();
    
    // Try to call an unknown method
    let response = client.call(&url, json_rpc_request(100, "unknown/method", None)).await.unwrap();
    
    // Should get "Method not found" error
    assert_json_rpc_error(&response, 100, -32601);
    
    server.stop();
}

#[tokio::test]
async fn test_invalid_params_error() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize first
    client.initialize(&url, "invalid-params-test").await.unwrap();
    
    // Try to call a tool with invalid parameters
    let response = client.call(&url, json_rpc_request(200, "tools/call", Some(json!({
        "name": "test_tool",
        "arguments": "invalid - should be object"
    })))).await.unwrap();
    
    // Should get "Invalid params" error
    assert_json_rpc_error(&response, 200, -32602);
    
    server.stop();
}

#[tokio::test] 
async fn test_unknown_tool_error() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize first
    client.initialize(&url, "unknown-tool-test").await.unwrap();
    
    // Try to call a tool that doesn't exist
    let response = client.call_tool(&url, "nonexistent_tool", json!({"input": "test"})).await.unwrap();
    
    // Should get an error (exact code may vary)
    assert!(response.get("error").is_some());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("unknown") || 
        response["error"]["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("not found"));
    
    server.stop();
}

#[tokio::test]
async fn test_tool_execution_error() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize first
    client.initialize(&url, "tool-error-test").await.unwrap();
    
    // Call the error_tool which always throws an error
    let response = client.call_tool(&url, "error_tool", json!({"input": "test"})).await.unwrap();
    
    // Should get an error response
    assert!(response.get("error").is_some());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Test error"));
    
    server.stop();
}

#[tokio::test]
async fn test_malformed_json_error() {
    let server = TestServer::start().await;
    let url = server.url("/");
    
    // Send malformed JSON
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body("{invalid json syntax")
        .send()
        .await
        .unwrap();
    
    let error_response: serde_json::Value = response.json().await.unwrap();
    
    // Should get "Parse error"
    assert_eq!(error_response["error"]["code"], -32700);
    assert!(error_response["error"]["message"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("parse"));
    
    server.stop();
}

#[tokio::test]
async fn test_missing_required_fields() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Send request without required jsonrpc field
    let response = client.call(&url, json!({
        "id": 1,
        "method": "initialize"
        // Missing "jsonrpc": "2.0"
    })).await.unwrap();
    
    // Should get "Invalid Request" error
    assert_json_rpc_error(&response, 1, -32600);
    
    server.stop();
}

#[tokio::test]
async fn test_wrong_jsonrpc_version() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Send request with wrong JSON-RPC version
    let response = client.call(&url, json!({
        "jsonrpc": "1.0", // Wrong version
        "id": 1,
        "method": "initialize",
        "params": {}
    })).await.unwrap();
    
    // Should get "Invalid Request" error
    assert_json_rpc_error(&response, 1, -32600);
    
    server.stop();
}

#[tokio::test]
async fn test_missing_method_field() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Send request without method field
    let response = client.call(&url, json!({
        "jsonrpc": "2.0",
        "id": 1
        // Missing "method"
    })).await.unwrap();
    
    // Should get "Invalid Request" error
    assert_json_rpc_error(&response, 1, -32600);
    
    server.stop();
}

#[tokio::test]
async fn test_invalid_initialize_params() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Try to initialize without required params
    let response = client.call(&url, json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize"
        // Missing params field entirely
    })).await.unwrap();
    
    // Should get an error
    assert!(response.get("error").is_some());
    
    server.stop();
}

#[tokio::test]
async fn test_error_response_format() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Trigger an error
    let response = client.call(&url, json_rpc_request(999, "unknown/method", None)).await.unwrap();
    
    // Verify error response format follows JSON-RPC 2.0 spec
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 999);
    assert!(response.get("result").is_none());
    
    let error = &response["error"];
    assert!(error["code"].is_number());
    assert!(error["message"].is_string());
    
    // Data field is optional but if present should be structured
    if let Some(data) = error.get("data") {
        assert!(data.is_object() || data.is_array() || data.is_string());
    }
    
    server.stop();
}

#[tokio::test]
async fn test_large_request_handling() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize first
    client.initialize(&url, "large-request-test").await.unwrap();
    
    // Create a very large input string
    let large_input = "x".repeat(100_000); // 100KB of data
    
    let response = client.call_tool(&url, "test_tool", json!({
        "input": large_input
    })).await.unwrap();
    
    // Should handle large requests successfully
    assert!(response.get("result").is_some());
    assert!(response.get("error").is_none());
    
    server.stop();
}

#[tokio::test]
async fn test_concurrent_error_handling() {
    let server = TestServer::start().await;
    let url = server.url("/");
    
    // Spawn multiple clients that will all cause errors
    let num_clients = 10;
    let mut handles = Vec::new();
    
    for i in 0..num_clients {
        let url_clone = url.clone();
        let handle = tokio::spawn(async move {
            let mut client = McpHttpClient::new();
            
            // Each client tries to use tools without initializing
            client.call(&url_clone, json_rpc_request(i as u32, "tools/list", None)).await
        });
        
        handles.push(handle);
    }
    
    // All should get "not initialized" errors
    for handle in handles {
        let response = handle.await.unwrap().unwrap();
        assert!(response.get("error").is_some());
    }
    
    server.stop();
}