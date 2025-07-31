//! Test to verify that MCP responses preserve structured data for programmatic consumption
//! This test demonstrates the fix for the "no results found" issue in Claude Code

use serde_json::{json, Value};
use solidmcp::McpTools;

#[tokio::test]
async fn test_structured_data_preservation() {
    // Test echo tool - simulates a search-like response
    let search_like_result = json!({
        "query": "bacon",
        "results": [
            {"id": "doc1", "title": "Bacon Recipes", "score": 0.95},
            {"id": "doc2", "title": "Bacon History", "score": 0.87},
            {"id": "doc3", "title": "Bacon Nutrition", "score": 0.73}
        ],
        "total_found": 3
    });

    let tool_params = json!({
        "message": serde_json::to_string(&search_like_result).unwrap()
    });

    let result = McpTools::execute_tool("echo", tool_params).await.unwrap();

    // Verify human-readable summary exists
    let content = result["content"][0]["text"].as_str().unwrap();
    println!("Human-readable summary: {}", content);
    assert!(content.contains("Echo:"));

    // Verify structured data is preserved and directly accessible
    let data = &result["data"];
    assert!(data.is_object());
    
    // Parse the echoed JSON to get our search results
    let echoed_data: Value = serde_json::from_str(data["echo"].as_str().unwrap()).unwrap();
    
    // Verify structured access to results - this is what Claude Code needs
    assert_eq!(echoed_data["query"], "bacon");
    assert_eq!(echoed_data["total_found"], 3);
    
    let results = echoed_data["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["id"], "doc1");
    assert_eq!(results[0]["title"], "Bacon Recipes");
    assert_eq!(results[0]["score"], 0.95);

    println!("✅ Structured data preserved: Claude Code can now count {} results for query '{}'", 
             results.len(), echoed_data["query"]);
}

#[tokio::test]
async fn test_search_result_format_example() {
    // Simulate what a hypothetical search tool would return
    // This demonstrates the proper format for search results that Claude Code can parse
    
    let mock_search_result = json!({
        "query": "rust programming",
        "results": [
            {
                "id": "rust-001",
                "title": "The Rust Programming Language",
                "content": "Rust is a systems programming language...", 
                "relevance_score": 0.98,
                "source": "https://doc.rust-lang.org"
            },
            {
                "id": "rust-002", 
                "title": "Rust by Example",
                "content": "Learn Rust with examples...",
                "relevance_score": 0.92,
                "source": "https://doc.rust-lang.org/rust-by-example"
            }
        ],
        "total_found": 2,
        "search_time_ms": 45
    });

    // Use echo tool to simulate returning this structured data
    let tool_params = json!({
        "message": serde_json::to_string(&mock_search_result).unwrap()
    });

    let response = McpTools::execute_tool("echo", tool_params).await.unwrap();

    // Claude Code can now access the structured data directly
    let data = &response["data"];
    let search_data: Value = serde_json::from_str(data["echo"].as_str().unwrap()).unwrap();
    
    // Programmatic access works perfectly
    let results = search_data["results"].as_array().unwrap();
    let result_count = results.len();
    let query = search_data["query"].as_str().unwrap();
    
    assert_eq!(result_count, 2);
    assert_eq!(query, "rust programming");
    
    // Human-readable summary is also available
    let summary = response["content"][0]["text"].as_str().unwrap();
    assert!(summary.contains("Echo:"));
    
    println!("✅ Search simulation successful:");
    println!("   📊 Found {} results for '{}'", result_count, query);
    println!("   📝 Human summary: {}", summary);
    println!("   🔍 First result: {}", results[0]["title"]);
    
    // This is exactly what was broken before: Claude Code can now 
    // count results programmatically instead of parsing text summaries
    assert!(result_count > 0, "Claude Code can now detect {} results instead of reporting 'No results found'", result_count);
}