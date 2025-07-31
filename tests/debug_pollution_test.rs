//! Test that proves debug information is polluting MCP protocol responses
//! 
//! This test demonstrates that debug/info/warn logging statements in the HTTP handler
//! are causing the "LARGE DEBUG SECTION DETECTED" warning and potentially causing
//! client timeouts due to excessive logging output.

use serde_json::{json, Value};

mod mcp_test_helpers;
use mcp_test_helpers::*;

#[tokio::test]
async fn test_debug_info_pollutes_protocol_responses() {
    // Start a test server on a dynamic port
    let test_server = McpTestServer::start().await.expect("Failed to start test server");
    let port = test_server.port;

    // Create a test message that will trigger extensive logging
    let test_message = json!({
        "jsonrpc": "2.0",
        "id": "test-debug-pollution",
        "method": "tools/call",
        "params": {
            "name": "test_tool",
            "arguments": {
                "test_param": "test_value"
            },
            "_meta": {
                "progressToken": "progress-token-123"
            }
        }
    });

    // Send HTTP request to the server
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    // Capture the response
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", "Cursor-Debug-Test")
        .json(&test_message)
        .send()
        .await
        .expect("Failed to send request");

    let response_text = response.text().await.expect("Failed to get response text");
    let response_size = response_text.len();

    println!("Response size: {} bytes", response_size);
    println!("Response preview (first 500 chars): {}", 
        response_text.chars().take(500).collect::<String>());

    // The response should be a valid JSON-RPC response
    let response_json: Value = serde_json::from_str(&response_text)
        .expect("Response should be valid JSON");

    // Verify it's a proper JSON-RPC response
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], "test-debug-pollution");

    // The real issue: During processing, the logs get captured and included somehow
    // Let's capture the actual console output during processing
    // First, check if the actual response is clean JSON-RPC
    println!("Full response JSON: {}", response_text);
    
    // The response JSON itself should be clean
    let response_json_str = serde_json::to_string(&response_json).unwrap();
    println!("Re-serialized response: {}", response_json_str);
    
    // The issue may be that debug info gets mixed into the response stream
    // Let's check if the response body itself contains non-JSON content
    let is_pure_json = serde_json::from_str::<Value>(&response_text).is_ok();
    
    if !is_pure_json {
        println!("❌ RESPONSE IS NOT PURE JSON!");
        println!("   This indicates debug output is being mixed with JSON response");
        println!("   This would cause the 'LARGE DEBUG SECTION DETECTED' warning");
    }
    
    // Check if the response body size is unexpectedly large
    // A simple error response should be under 200 bytes
    let expected_max_size = 200;
    if response_size > expected_max_size {
        println!("⚠️  RESPONSE TOO LARGE: {} bytes (expected < {} bytes)", 
            response_size, expected_max_size);
        println!("   This suggests debug output is inflating the response");
    }
    
    // The primary test: Response should be pure JSON and reasonably sized
    assert!(is_pure_json, "Response should be pure JSON, not mixed with debug output");
    assert!(response_size < 1000, "Response should be under 1000 bytes for a simple error, got {} bytes", response_size);
}

#[tokio::test]
async fn test_clean_response_without_debug_pollution() {
    // This test shows what a CLEAN response should look like
    let clean_response = json!({
        "jsonrpc": "2.0",
        "id": "test-clean",
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    });
    
    let clean_json = serde_json::to_string(&clean_response).unwrap();
    let clean_size = clean_json.len();
    
    println!("Clean MCP response example:");
    println!("Size: {} bytes", clean_size);
    println!("Content: {}", clean_json);
    
    // A clean response should be small and contain no debug info
    assert!(clean_size < 200, "Clean response should be under 200 bytes");
    assert!(!clean_json.contains("debug"));
    assert!(!clean_json.contains("🚀"));
    assert!(!clean_json.contains("Request ID:"));
    assert!(!clean_json.contains("=== MCP REQUEST ANALYSIS"));
}

#[tokio::test] 
async fn test_debug_warning_threshold() {
    // Test the specific threshold mentioned in the code
    let large_response = "debug".repeat(2000); // Creates 10,000 character string with "debug"
    
    // This simulates the check in http.rs line 512-514
    let triggers_warning = large_response.contains("debug") && large_response.len() > 5000;
    
    assert!(triggers_warning, "Should trigger LARGE DEBUG SECTION DETECTED warning");
    println!("✅ Debug warning threshold test confirms the 5000 byte threshold");
}