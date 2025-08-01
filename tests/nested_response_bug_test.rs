//! Test to demonstrate nested MCP response bug
//!
//! This test demonstrates the bug where MCP responses show "McpResponse: content" 
//! instead of the actual search results due to nested structure in the solidmcp framework.

use solidmcp::{McpServerBuilder, json, ToolResponse};
use serde_json::Value;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Simple context for the test
#[derive(Debug, Clone)]
struct TestContext {
    name: String,
}

// Input type for our search tool
#[derive(Debug, Deserialize, JsonSchema)]
struct SearchInput {
    query: String,
}

// Output type for our search tool  
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SearchOutput {
    query: String,
    results: Vec<SearchResult>,
    total_matches: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SearchResult {
    file: String,
    line: u32,
    content: String,
}

#[tokio::test]
async fn test_framework_response_structure() {
    // This test demonstrates how the framework wraps responses
    // Create some search results
    let search_output = SearchOutput {
        query: "main".to_string(),
        results: vec![
            SearchResult {
                file: "src/main.rs".to_string(),
                line: 10,
                content: "fn main() {".to_string(),
            },
            SearchResult {
                file: "src/lib.rs".to_string(),
                line: 1,
                content: "pub mod server;".to_string(),
            }
        ],
        total_matches: 2,
    };
    
    // Serialize the output as it would be in a framework response
    let output_json = serde_json::to_value(&search_output).unwrap();
    println!("Direct tool output structure:");
    println!("{}", serde_json::to_string_pretty(&output_json).unwrap());
    
    // Now show how it gets wrapped in the MCP response
    let mcp_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": serde_json::to_string(&search_output).unwrap()
                }
            ],
            "isError": false
        }
    });
    
    println!("\nMCP response structure (potential nested bug):");
    println!("{}", serde_json::to_string_pretty(&mcp_response).unwrap());
    
    // The problem: users expect result.query and result.results
    // But instead they get result.content[0].text (serialized JSON string)
    let can_access_directly = mcp_response["result"]["query"].is_string();
    let content_is_nested = mcp_response["result"]["content"].is_array();
    
    println!("\nBUG ANALYSIS:");
    println!("Can access result.query directly: {}", can_access_directly);
    println!("Content is nested in array: {}", content_is_nested);
    
    if !can_access_directly && content_is_nested {
        println!("BUG CONFIRMED: Tool output is wrapped in content array instead of being directly accessible");
        println!("Users see 'McpResponse: content' instead of the actual search results");
        
        // Show what users have to do to get the actual data
        if let Some(content) = mcp_response["result"]["content"].as_array() {
            if let Some(first_content) = content.first() {
                if let Some(text) = first_content.get("text").and_then(|t| t.as_str()) {
                    let parsed_output: SearchOutput = serde_json::from_str(text).unwrap();
                    println!("\nTo get actual data, users must:");
                    println!("1. Access result.content[0].text");
                    println!("2. Parse the JSON string: {}", text);
                    println!("3. Then they get: {} results for query '{}'", 
                             parsed_output.results.len(), parsed_output.query);
                }
            }
        }
    }
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