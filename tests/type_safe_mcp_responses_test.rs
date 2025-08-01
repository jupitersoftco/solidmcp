//! Test type-safe MCP response format to prevent "no results found" issues
//!
//! This test verifies that the new type-safe MCP response system correctly enforces
//! the MCP protocol format, preventing issues where Claude Code shows "no results found"
//! when tools return raw JSON data instead of properly formatted MCP responses.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use solidmcp::{
    content_types::{McpContent, McpResponse},
    framework::{McpServerBuilder, NotificationCtx},
};
use std::sync::Arc;

/// Test context for our MCP server
#[derive(Clone)]
struct TestContext {
    data: Vec<SearchResult>,
}

#[derive(Clone, Debug)]
struct SearchResult {
    id: String,
    title: String,
    content: String,
    score: f64,
}

/// Input type for search tool
#[derive(JsonSchema, Deserialize)]
struct SearchInput {
    query: String,
    limit: Option<u32>,
}

#[tokio::test]
async fn test_type_safe_mcp_response_compilation() {
    // This test verifies that the type-safe API compiles correctly
    let context = TestContext {
        data: vec![
            SearchResult {
                id: "1".to_string(),
                title: "Bacon Recipes".to_string(),
                content: "How to cook bacon perfectly...".to_string(),
                score: 0.95,
            },
            SearchResult {
                id: "2".to_string(),
                title: "Bacon History".to_string(),
                content: "The history of bacon dates back...".to_string(),
                score: 0.87,
            },
        ],
    };

    // This should compile successfully with the new type-safe API
    let _server = McpServerBuilder::new(context, "test-server", "1.0.0")
        .with_tool(
            "search",
            "Search for information",
            |input: SearchInput, ctx: Arc<TestContext>, _notif: NotificationCtx| async move {
                let results: Vec<&SearchResult> = ctx
                    .data
                    .iter()
                    .filter(|result| {
                        result
                            .title
                            .to_lowercase()
                            .contains(&input.query.to_lowercase())
                    })
                    .take(input.limit.unwrap_or(10) as usize)
                    .collect();

                // Type-safe MCP response - this prevents the "no results found" issue
                let response = McpResponse::with_text_and_data(
                    format!("Found {} results for '{}'", results.len(), input.query),
                    json!({
                        "results": results.iter().map(|r| json!({
                            "id": r.id,
                            "title": r.title,
                            "content": r.content,
                            "score": r.score
                        })).collect::<Vec<_>>(),
                        "query": input.query,
                        "total": results.len()
                    }),
                );

                Ok(response)
            },
        )
        .build()
        .await
        .expect("Server should build successfully");

    // If we get here, the type-safe API compiled successfully
    println!("✅ Type-safe MCP response API compiles correctly");
}

#[test]
fn test_mcp_response_structure() {
    // Test that McpResponse creates the correct JSON structure
    let response = McpResponse::with_text_and_data(
        "Found 2 results for 'bacon'",
        json!({
            "results": [
                {"id": "1", "title": "Bacon Recipes", "score": 0.95},
                {"id": "2", "title": "Bacon History", "score": 0.87}
            ],
            "total": 2,
            "query": "bacon"
        }),
    );

    let json = serde_json::to_value(&response).unwrap();

    // Verify MCP protocol compliance
    assert!(json["content"].is_array(), "Response must have content array");
    assert_eq!(json["content"].as_array().unwrap().len(), 1);
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][0]["text"], "Found 2 results for 'bacon'");

    // Verify structured data is preserved
    assert!(json["data"].is_object(), "Response should have data object");
    assert_eq!(json["data"]["total"], 2);
    assert_eq!(json["data"]["query"], "bacon");
    assert_eq!(json["data"]["results"].as_array().unwrap().len(), 2);

    // Verify error flag
    assert_eq!(json["is_error"], false);

    println!("✅ MCP response structure is correct");
}

#[test]
fn test_mcp_content_types() {
    // Test different content types
    let text_content = McpContent::text("Hello, world!");
    let text_json = serde_json::to_value(&text_content).unwrap();
    assert_eq!(text_json["type"], "text");
    assert_eq!(text_json["text"], "Hello, world!");

    let image_content = McpContent::image("base64data", Some("image/png".to_string()));
    let image_json = serde_json::to_value(&image_content).unwrap();
    assert_eq!(image_json["type"], "image");
    assert_eq!(image_json["data"], "base64data");
    assert_eq!(image_json["mime_type"], "image/png");

    let resource_content = McpContent::resource(
        "file://results.json", 
        Some("application/json".to_string()), 
        Some("Search results".to_string())
    );
    let resource_json = serde_json::to_value(&resource_content).unwrap();
    assert_eq!(resource_json["type"], "resource");
    assert_eq!(resource_json["uri"], "file://results.json");
    assert_eq!(resource_json["mime_type"], "application/json");
    assert_eq!(resource_json["text"], "Search results");

    println!("✅ MCP content types serialize correctly");
}

#[test]
fn test_mcp_response_prevents_claude_code_issue() {
    // This test demonstrates how the new types prevent the "no results found" issue
    
    // Before: Raw JSON that caused Claude Code to show "no results found"
    let raw_response = json!({
        "results": [
            {"id": "1", "title": "Bacon Recipes", "score": 0.95}
        ],
        "total": 1
    });
    
    // After: Type-safe MCP response that Claude Code can parse correctly
    let mcp_response = McpResponse::with_text_and_data(
        "Found 1 result for 'bacon'",
        raw_response.clone()
    );
    
    let mcp_json = serde_json::to_value(&mcp_response).unwrap();
    
    // Verify this has the structure Claude Code expects
    assert!(mcp_json["content"].is_array(), "Claude Code expects content array");
    assert!(!mcp_json["content"].as_array().unwrap().is_empty(), "Content array must not be empty");
    assert_eq!(mcp_json["content"][0]["type"], "text");
    
    // Verify structured data is still accessible for programmatic use
    assert!(mcp_json["data"].is_object(), "Structured data should be preserved");
    assert_eq!(mcp_json["data"]["total"], 1);
    
    // This is the key difference: Claude Code can now see both:
    // 1. Human-readable content in the content array
    // 2. Structured data in the data field
    
    println!("✅ Type-safe MCP response prevents 'no results found' issue");
    println!("   - Content array: {}", mcp_json["content"]);
    println!("   - Structured data: {}", mcp_json["data"]);
}

#[test]
fn test_error_responses() {
    let error_response = McpResponse::error("Search failed: database connection lost");
    let json = serde_json::to_value(&error_response).unwrap();
    
    assert_eq!(json["is_error"], true);
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][0]["text"], "Search failed: database connection lost");
    assert!(json["data"].is_null());
    
    println!("✅ Error responses are properly formatted");
}

#[test]
fn test_multiple_content_items() {
    let response = McpResponse::new(vec![
        McpContent::text("Search completed successfully"),
        McpContent::resource("file://results.json", Some("application/json".to_string()), None),
        McpContent::text("Results saved to file")
    ]);
    
    let json = serde_json::to_value(&response).unwrap();
    
    assert_eq!(json["content"].as_array().unwrap().len(), 3);
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][1]["type"], "resource");
    assert_eq!(json["content"][2]["type"], "text");
    
    println!("✅ Multiple content items are supported");
}