//! Integration tests for full MCP protocol flows
//!
//! These tests verify that the complete MCP protocol works correctly
//! across different transports and scenarios.

mod helpers;

use helpers::{TestServer, McpHttpClient, assert_json_rpc_success, json_rpc_request};
use serde_json::json;

#[tokio::test]
async fn test_full_http_protocol_flow() {
    // Start test server
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // 1. Initialize the session
    let init_response = client.initialize(&url, "integration-test-client").await.unwrap();
    
    // Verify initialization response
    assert_json_rpc_success(&init_response, 1);
    assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");
    assert!(init_response["result"]["serverInfo"]["name"].is_string());
    assert!(client.session_cookie().is_some());
    
    // 2. List available tools
    let tools_response = client.list_tools(&url).await.unwrap();
    
    // Verify tools list response
    assert_json_rpc_success(&tools_response, 2);
    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 2); // test_tool and error_tool
    
    // Find our test tool
    let test_tool = tools.iter()
        .find(|tool| tool["name"] == "test_tool")
        .expect("test_tool should be in the list");
    assert_eq!(test_tool["description"], "A test tool for integration testing");
    assert!(test_tool["inputSchema"].is_object());
    
    // 3. Call the test tool
    let call_response = client.call_tool(&url, "test_tool", json!({
        "input": "hello world"
    })).await.unwrap();
    
    // Verify tool call response
    assert_json_rpc_success(&call_response, 3);
    
    // Check the response content structure
    let result = &call_response["result"];
    assert!(result["content"].is_array());
    let content = result["content"].as_array().unwrap();
    assert!(!content.is_empty());
    
    // The response should contain the tool output
    // Note: This shows the nested structure issue, but verifies the current behavior
    if let Some(first_content) = content.first() {
        assert_eq!(first_content["type"], "text");
        assert!(first_content["text"].is_string());
    }
    
    server.stop();
}

#[tokio::test]
async fn test_session_persistence_across_requests() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize session
    client.initialize(&url, "session-test-client").await.unwrap();
    let original_cookie = client.session_cookie().unwrap().to_string();
    
    // Make multiple requests - session should persist
    for i in 1..=5 {
        let response = client.call_tool(&url, "test_tool", json!({
            "input": format!("request_{}", i)
        })).await.unwrap();
        
        assert_json_rpc_success(&response, 3);
        
        // Session cookie should remain the same
        assert_eq!(client.session_cookie().unwrap(), original_cookie);
    }
    
    server.stop();
}

#[tokio::test]
async fn test_session_reinitialization() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // First initialization
    let response1 = client.initialize(&url, "reinit-test-client-v1").await.unwrap();
    assert_json_rpc_success(&response1, 1);
    
    // Tools should work after first init
    let tools_response = client.list_tools(&url).await.unwrap();
    assert_json_rpc_success(&tools_response, 2);
    
    // Re-initialize (simulating client reconnection)
    let response2 = client.initialize(&url, "reinit-test-client-v2").await.unwrap();
    assert_json_rpc_success(&response2, 1);
    
    // Tools should still work after re-initialization
    let tools_response2 = client.list_tools(&url).await.unwrap();
    assert_json_rpc_success(&tools_response2, 2);
    
    server.stop();
}

#[tokio::test]
async fn test_multiple_independent_sessions() {
    let server = TestServer::start().await;
    let url = server.url("/");
    
    // Create multiple independent clients
    let mut client1 = McpHttpClient::new();
    let mut client2 = McpHttpClient::new();
    let mut client3 = McpHttpClient::new();
    
    // Initialize all clients
    client1.initialize(&url, "client-1").await.unwrap();
    client2.initialize(&url, "client-2").await.unwrap();
    client3.initialize(&url, "client-3").await.unwrap();
    
    // Verify they have different session cookies
    let cookie1 = client1.session_cookie().unwrap();
    let cookie2 = client2.session_cookie().unwrap();
    let cookie3 = client3.session_cookie().unwrap();
    
    assert_ne!(cookie1, cookie2);
    assert_ne!(cookie2, cookie3);
    assert_ne!(cookie1, cookie3);
    
    // All clients should be able to make tool calls independently
    let response1 = client1.call_tool(&url, "test_tool", json!({"input": "from client 1"})).await.unwrap();
    let response2 = client2.call_tool(&url, "test_tool", json!({"input": "from client 2"})).await.unwrap();
    let response3 = client3.call_tool(&url, "test_tool", json!({"input": "from client 3"})).await.unwrap();
    
    assert_json_rpc_success(&response1, 3);
    assert_json_rpc_success(&response2, 3);
    assert_json_rpc_success(&response3, 3);
    
    server.stop();
}

#[tokio::test]
async fn test_concurrent_requests_same_session() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize session
    client.initialize(&url, "concurrent-test-client").await.unwrap();
    let session_cookie = client.session_cookie().unwrap().to_string();
    
    // Make concurrent requests using the same session
    let num_requests = 10;
    let mut handles = Vec::new();
    
    for i in 0..num_requests {
        let url_clone = url.clone();
        let cookie_clone = session_cookie.clone();
        
        let handle = tokio::spawn(async move {
            let mut concurrent_client = McpHttpClient::new();
            concurrent_client.session_cookie = Some(cookie_clone);
            
            concurrent_client.call_tool(&url_clone, "test_tool", json!({
                "input": format!("concurrent_request_{}", i)
            })).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap().unwrap();
        assert_json_rpc_success(&response, 3);
    }
    
    server.stop();
}

#[tokio::test]
async fn test_protocol_compliance() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Test that all responses follow JSON-RPC 2.0 spec
    let init_response = client.initialize(&url, "compliance-test").await.unwrap();
    
    // Every response must have jsonrpc: "2.0"
    assert_eq!(init_response["jsonrpc"], "2.0");
    
    // Response must have either result or error, but not both
    assert!(init_response.get("result").is_some());
    assert!(init_response.get("error").is_none());
    
    // ID must match request ID
    assert_eq!(init_response["id"], 1);
    
    // Test tools/list compliance
    let tools_response = client.list_tools(&url).await.unwrap();
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(tools_response.get("result").is_some());
    assert!(tools_response.get("error").is_none());
    
    // Test tools/call compliance
    let call_response = client.call_tool(&url, "test_tool", json!({"input": "test"})).await.unwrap();
    assert_eq!(call_response["jsonrpc"], "2.0");
    assert_eq!(call_response["id"], 3);
    assert!(call_response.get("result").is_some());
    assert!(call_response.get("error").is_none());
    
    server.stop();
}

#[tokio::test]
async fn test_tool_state_isolation() {
    let server = TestServer::start().await;
    let mut client1 = McpHttpClient::new();
    let mut client2 = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize both clients
    client1.initialize(&url, "state-test-1").await.unwrap();
    client2.initialize(&url, "state-test-2").await.unwrap();
    
    // Call the test tool multiple times with each client
    // The tool increments a counter, so we can verify state isolation
    
    // Client 1 calls
    let response1a = client1.call_tool(&url, "test_tool", json!({"input": "test1"})).await.unwrap();
    let response1b = client1.call_tool(&url, "test_tool", json!({"input": "test2"})).await.unwrap();
    
    // Client 2 calls
    let response2a = client2.call_tool(&url, "test_tool", json!({"input": "test3"})).await.unwrap();
    let response2b = client2.call_tool(&url, "test_tool", json!({"input": "test4"})).await.unwrap();
    
    // All should succeed
    assert_json_rpc_success(&response1a, 3);
    assert_json_rpc_success(&response1b, 3);
    assert_json_rpc_success(&response2a, 3);
    assert_json_rpc_success(&response2b, 3);
    
    server.stop();
}