//! Integration tests for UTF-8 validation in optimized protocol handling

mod helpers;

use helpers::{TestServer, McpHttpClient};
use serde_json::json;

#[tokio::test]
async fn test_utf8_validation_valid_international_characters() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize session
    client.initialize(&url, "utf8-test-client").await.unwrap();
    
    // Test with international characters in tool call
    let response = client.call_tool(&url, "test_tool", json!({
        "input": "测试 Hello 世界! 🌍 𝕌𝕟𝕚𝕔𝕠𝕕𝕖"
    })).await.unwrap();
    
    // Should succeed with international characters
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("result").is_some());
    assert!(response.get("error").is_none());
    
    server.stop();
}

#[tokio::test]
async fn test_utf8_validation_with_emoji_and_symbols() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize session
    client.initialize(&url, "emoji-test-client").await.unwrap();
    
    // Test with emojis and mathematical symbols
    let response = client.call_tool(&url, "test_tool", json!({
        "input": "🚀 Testing with emojis! 🎉 ∑∆π ⚡️ 🔥"
    })).await.unwrap();
    
    // Should succeed with emoji characters
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("result").is_some());
    
    server.stop();
}

#[tokio::test]
async fn test_http_server_handles_utf8_gracefully() {
    // This test verifies that our HTTP server can handle UTF-8 input correctly
    // If UTF-8 validation is working, the server should process these correctly
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize with international client name
    let init_response = client.initialize(&url, "UTF-8客户端🌍").await.unwrap();
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert!(init_response.get("result").is_some());
    
    // Make multiple requests with different UTF-8 content
    let test_cases = vec![
        "Simple ASCII",
        "Français avec accents éèç",
        "日本語テスト",
        "Русский текст", 
        "العربية",
        "🚀🌍🎉 Mixed emojis with text",
        "Mathematical: ∑∆π∞ ≠ ≤ ≥",
        "Currency: $€¥£₹",
    ];
    
    for (i, test_input) in test_cases.iter().enumerate() {
        let response = client.call_tool(&url, "test_tool", json!({
            "input": test_input
        })).await.unwrap();
        
        assert_eq!(response["jsonrpc"], "2.0", "Failed for test case {}: {}", i, test_input);
        assert!(response.get("result").is_some(), "No result for test case {}: {}", i, test_input);
        assert!(response.get("error").is_none(), "Got error for test case {}: {}", i, test_input);
    }
    
    server.stop();
}

#[tokio::test]
async fn test_large_utf8_message() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize session
    client.initialize(&url, "large-utf8-test").await.unwrap();
    
    // Create a large message with international characters
    let mut large_text = String::new();
    for i in 0..1000 {
        large_text.push_str(&format!("行 {} 测试国际化字符 🌍 ", i));
    }
    
    let response = client.call_tool(&url, "test_tool", json!({
        "input": large_text
    })).await.unwrap();
    
    // Should handle large UTF-8 messages without issues
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("result").is_some());
    assert!(response.get("error").is_none());
    
    server.stop();
}

#[tokio::test]
async fn test_boundary_conditions_utf8() {
    let server = TestServer::start().await;
    let mut client = McpHttpClient::new();
    let url = server.url("/");
    
    // Initialize session
    client.initialize(&url, "boundary-test").await.unwrap();
    
    // Test various boundary conditions that might cause UTF-8 issues
    let boundary_cases = vec![
        "",  // Empty string 
        " ",  // Single space
        "\n\t\r",  // Whitespace characters
        "a",  // Single ASCII character
        "🌍",  // Single emoji (4-byte UTF-8)
        "é",  // Single accented character (2-byte UTF-8)
        "测",  // Single CJK character (3-byte UTF-8)
        "🌍a测é",  // Mix of different UTF-8 byte lengths
    ];
    
    for (i, test_case) in boundary_cases.iter().enumerate() {
        let response = client.call_tool(&url, "test_tool", json!({
            "input": test_case
        })).await.unwrap();
        
        assert_eq!(response["jsonrpc"], "2.0", "Failed for boundary case {}: '{}'", i, test_case);
        assert!(response.get("result").is_some(), "No result for boundary case {}", i);
    }
    
    server.stop();
}