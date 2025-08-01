//! Test type-safe MCP response format to prevent "no results found" issues
//!
//! This test verifies that the new type-safe MCP response system correctly enforces
//! the MCP protocol format, preventing issues where Claude Code shows "no results found"
//! when tools return raw JSON data instead of properly formatted MCP responses.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use solidmcp::{
    McpServerBuilder, NotificationCtx, ToolResponse, ToolContent,
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
                let text = format!("Found {} results for '{}'", results.len(), input.query);
                
                // The key fix: Always return a proper ToolResponse with text content
                // This ensures Claude Code can see the results instead of "no results found"
                Ok(ToolResponse::success(text))
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
    // Test that ToolResponse creates the correct JSON structure
    let response = ToolResponse::success("Found 2 results for 'bacon'");

    // Convert to JSON to verify structure
    let json = json!({
        "content": response.content,
        "is_error": response.is_error
    });

    // Verify MCP protocol compliance
    assert!(json["content"].is_array(), "Response must have content array");
    assert_eq!(json["content"].as_array().unwrap().len(), 1);
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][0]["text"], "Found 2 results for 'bacon'");

    // Verify error flag
    assert_eq!(json["is_error"], false);

    println!("✅ MCP response structure is correct");
}

#[test]
fn test_mcp_content_types() {
    // Test different content types using the public API
    let text_content = ToolContent::Text { text: "Hello, world!".to_string() };
    let text_json = serde_json::to_value(&text_content).unwrap();
    assert_eq!(text_json["type"], "text");
    assert_eq!(text_json["text"], "Hello, world!");

    let image_content = ToolContent::Image { 
        data: "base64data".to_string(),
        mime_type: "image/png".to_string(),
    };
    let image_json = serde_json::to_value(&image_content).unwrap();
    assert_eq!(image_json["type"], "image");
    assert_eq!(image_json["data"], "base64data");
    assert_eq!(image_json["mime_type"], "image/png");

    let resource_content = ToolContent::Resource {
        uri: "file://results.json".to_string(),
    };
    let resource_json = serde_json::to_value(&resource_content).unwrap();
    assert_eq!(resource_json["type"], "resource");
    assert_eq!(resource_json["uri"], "file://results.json");

    println!("✅ MCP content types serialize correctly");
}

#[test]
fn test_mcp_response_prevents_claude_code_issue() {
    // This test demonstrates how the new types prevent the "no results found" issue
    
    // After: Type-safe MCP response that Claude Code can parse correctly
    let response = ToolResponse::success("Found 1 result for 'bacon'");
    
    // Convert to JSON to verify structure
    let json = json!({
        "content": response.content,
        "is_error": response.is_error
    });
    
    // Verify this has the structure Claude Code expects
    assert!(json["content"].is_array(), "Claude Code expects content array");
    assert!(!json["content"].as_array().unwrap().is_empty(), "Content array must not be empty");
    assert_eq!(json["content"][0]["type"], "text");
    
    // This is the key fix: Claude Code can now see the results in the content array
    // instead of getting "no results found"
    
    println!("✅ Type-safe MCP response prevents 'no results found' issue");
    println!("   - Content array: {}", json["content"]);
}

#[test]
fn test_error_responses() {
    let error_response = ToolResponse {
        content: vec![ToolContent::Text { text: "Search failed: database connection lost".to_string() }],
        is_error: true,
    };
    
    let json = json!({
        "content": error_response.content,
        "is_error": error_response.is_error
    });
    
    assert_eq!(json["is_error"], true);
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][0]["text"], "Search failed: database connection lost");
    
    println!("✅ Error responses are properly formatted");
}

#[test]
fn test_multiple_content_items() {
    let response = ToolResponse::with_content(vec![
        ToolContent::Text { text: "Search completed successfully".to_string() },
        ToolContent::Resource { uri: "file://results.json".to_string() },
        ToolContent::Text { text: "Results saved to file".to_string() }
    ]);
    
    let json = json!({
        "content": response.content,
        "is_error": response.is_error
    });
    
    assert_eq!(json["content"].as_array().unwrap().len(), 3);
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["content"][1]["type"], "resource");
    assert_eq!(json["content"][2]["type"], "text");
    
    println!("✅ Multiple content items are supported");
}