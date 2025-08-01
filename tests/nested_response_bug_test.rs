//! Test to demonstrate nested MCP response bug
//!
//! This test demonstrates the bug where MCP responses show "McpResponse: content" 
//! instead of the actual search results due to nested structure in the solidmcp framework.

use solidmcp::{McpServerBuilder, ToolResponse, json};
use serde_json::Value;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use warp::Filter;

#[tokio::test]
async fn test_nested_response_bug() {
    // Create a server with a search tool that returns meaningful results
    let server = McpServerBuilder::new()
        .with_tool("search_files", "Search for files in the codebase", |params| async move {
            let query = params["query"].as_str().unwrap_or("default");
            
            // Return structured search results like a real search would
            let results = vec![
                json!({"file": "src/main.rs", "line": 10, "content": "fn main() {"}),
                json!({"file": "src/lib.rs", "line": 1, "content": "pub mod server;"}) 
            ];
            
            Ok(ToolResponse::success(json!({
                "query": query,
                "results": results,
                "total_matches": 2
            })))
        })
        .build();

    // Find available port for test server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    // Start the server in the background
    let server_handle = tokio::spawn(async move {
        server.start(addr.port()).await.unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make a tools/call request directly to the HTTP API
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}", addr.port()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "search_files",
                "arguments": {
                    "query": "main"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let response_json: Value = response.json().await.unwrap();
    
    // Print the actual response structure to show the bug
    println!("Raw MCP response structure:");
    println!("{}", serde_json::to_string_pretty(&response_json).unwrap());
    
    // The bug: instead of getting clean search results, we get nested structure
    // This assertion will show the problem - the response is wrapped in layers
    if let Some(result) = response_json.get("result") {
        if let Some(content) = result.get("content") {
            // This should fail, demonstrating the bug where we get "McpResponse: content"
            // instead of direct access to the search results
            assert!(
                content.get("results").is_some(),
                "BUG DEMONSTRATED: Search results are buried in nested structure. \
                 Response structure: {}",
                serde_json::to_string_pretty(&response_json).unwrap()
            );
        }
    }

    // Cleanup
    server_handle.abort();
}

#[tokio::test] 
async fn test_expected_clean_response_structure() {
    // This test shows what the response SHOULD look like
    // without the nested structure bug
    
    let expected_clean_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "query": "main",
            "results": [
                {"file": "src/main.rs", "line": 10, "content": "fn main() {"},
                {"file": "src/lib.rs", "line": 1, "content": "pub mod server;"}
            ],
            "total_matches": 2
        }
    });
    
    println!("Expected clean response structure:");
    println!("{}", serde_json::to_string_pretty(&expected_clean_response).unwrap());
    
    // The search results should be directly accessible at result level
    assert!(expected_clean_response["result"]["results"].is_array());
    assert_eq!(expected_clean_response["result"]["total_matches"], 2);
}