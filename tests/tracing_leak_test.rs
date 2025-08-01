//! Test that checks if tracing output leaks into HTTP response streams
//! 
//! This test specifically checks for the bug where debug logs get mixed
//! into the actual JSON-RPC response body, causing protocol violations.

use serde_json::json;

mod mcp_test_helpers;
use mcp_test_helpers::*;

#[tokio::test]
async fn test_tracing_output_does_not_leak_into_response() {
    // Set up environment variables that might cause the issue
    std::env::set_var("RUST_LOG", "trace,solidmcp=trace");
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("SOLIDMCP_LOG_LEVEL", "trace");
    std::env::set_var("SOLIDMCP_TRACE_PROTOCOL", "1");
    std::env::set_var("SOLIDMCP_TRACE_REQUESTS", "1");
    std::env::set_var("SOLIDMCP_TRACE_RESPONSES", "1");
    std::env::set_var("MCP_DEBUG", "1");
    std::env::set_var("MCP_TRACE_PROTOCOL", "1");

    // Start test server
    let test_server = McpTestServer::start().await.expect("Failed to start test server");
    let port = test_server.port;

    // Create a message that should trigger extensive logging
    let test_message = json!({
        "jsonrpc": "2.0",
        "id": "tracing-leak-test",
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",  // This will cause error logging
            "arguments": {"param": "test".repeat(1000)},  // Large param to trigger size warnings
            "_meta": {
                "progressToken": "leak-test-token-123"  // Triggers progress logging
            }
        }
    });

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    println!("=== TESTING WITH ALL DEBUG FLAGS ENABLED ===");
    println!("RUST_LOG: {}", std::env::var("RUST_LOG").unwrap_or_else(|_| "not set".to_string()));
    println!("SOLIDMCP_TRACE_PROTOCOL: {}", std::env::var("SOLIDMCP_TRACE_PROTOCOL").unwrap_or_else(|_| "not set".to_string()));
    
    // Send the request that should trigger lots of logging
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", "Cursor-TraceLeakTest/1.0")  // Trigger Cursor-specific logging
        .json(&test_message)
        .send()
        .await
        .expect("Failed to send request");

    let response_headers = response.headers().clone();
    let response_status = response.status();
    let response_body = response.text().await.expect("Failed to get response text");
    
    println!("=== RESPONSE ANALYSIS ===");
    println!("Status: {}", response_status);
    println!("Response size: {} bytes", response_body.len());
    println!("Content-Type header: {:?}", response_headers.get("content-type"));
    
    // Show first part of response for analysis
    let preview_len = std::cmp::min(200, response_body.len());
    println!("Response preview: {}", &response_body[..preview_len]);
    
    if response_body.len() > preview_len {
        println!("... (truncated, full size: {} bytes)", response_body.len());
    }

    // === CRITICAL TESTS ===
    
    // 1. Response should be pure JSON
    let json_parse_result = serde_json::from_str::<serde_json::Value>(&response_body);
    match &json_parse_result {
        Ok(json_value) => {
            println!("✅ Response is valid JSON");
            println!("JSON structure: {}", serde_json::to_string_pretty(json_value).unwrap_or_else(|_| "Failed to pretty print".to_string()));
        }
        Err(e) => {
            println!("❌ Response is NOT valid JSON: {}", e);
            println!("This indicates tracing output leaked into the response!");
            
            // Try to identify what leaked
            if response_body.contains("🚀") || response_body.contains("🔍") || response_body.contains("📊") {
                println!("   Found emoji debug markers in response body");
            }
            if response_body.contains("=== MCP REQUEST ANALYSIS") {
                println!("   Found debug section headers in response body");
            }
            if response_body.contains("Request ID:") {
                println!("   Found request ID debug info in response body");
            }
        }
    }
    
    // 2. Content-Type should be application/json
    let content_type = response_headers
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("missing");
    
    assert!(content_type.contains("application/json"), 
        "Content-Type should be application/json, got: {}", content_type);

    // 3. Response should not contain tracing artifacts
    let tracing_artifacts = [
        "🚀", "🔍", "📊", "⚡", "🎯", "📥", "📤", "🔧", "⚠️", "❌", "✅",
        "=== MCP REQUEST ANALYSIS ===",
        "=== MESSAGE STRUCTURE ===", 
        "=== REQUEST SIZE ANALYSIS ===",
        "Request ID:",
        "SESSION DEBUG:",
        "CURSOR CLIENT DETECTED",
        "PROGRESS TOKEN DETECTED",
        "trace:", "debug:", "info:", "warn:", "error:",
    ];
    
    let mut found_artifacts = Vec::new();
    for artifact in &tracing_artifacts {
        if response_body.contains(artifact) {
            found_artifacts.push(*artifact);
        }
    }
    
    if !found_artifacts.is_empty() {
        println!("❌ TRACING LEAK DETECTED!");
        println!("   Found these tracing artifacts in HTTP response body:");
        for artifact in &found_artifacts {
            println!("     - '{}'", artifact);
        }
        println!("   This proves tracing output is leaking into the JSON-RPC response!");
    }

    // 4. Main assertion: Response should be valid JSON without tracing artifacts
    assert!(json_parse_result.is_ok(), 
        "Response body should be valid JSON, not mixed with tracing output. Parse error: {:?}", 
        json_parse_result.err());
    
    assert!(found_artifacts.is_empty(),
        "Response body contains tracing artifacts: {:?}. This proves debug logs are leaking into the HTTP response stream!", 
        found_artifacts);
    
    // 5. Response size should be reasonable for a JSON-RPC error
    // A proper error response should be under 500 bytes
    assert!(response_body.len() < 1000,
        "Response is unexpectedly large ({} bytes), suggesting debug output contamination", 
        response_body.len());

    println!("✅ All tests passed - no tracing leak detected in this configuration");
}

#[tokio::test] 
async fn test_with_minimal_logging() {
    // Test with minimal logging to establish baseline
    std::env::remove_var("RUST_LOG");
    std::env::remove_var("SOLIDMCP_LOG_LEVEL");
    std::env::remove_var("SOLIDMCP_TRACE_PROTOCOL");
    std::env::remove_var("MCP_DEBUG");
    
    let test_server = McpTestServer::start().await.expect("Failed to start test server");
    let port = test_server.port;

    let test_message = json!({
        "jsonrpc": "2.0",
        "id": "minimal-test",
        "method": "ping",
        "params": {}
    });

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&test_message)
        .send()
        .await
        .expect("Failed to send request");

    let response_body = response.text().await.expect("Failed to get response text");
    
    println!("=== MINIMAL LOGGING BASELINE ===");
    println!("Response size: {} bytes", response_body.len());
    println!("Response: {}", response_body);
    
    // This should always be clean JSON
    let json_result = serde_json::from_str::<serde_json::Value>(&response_body);
    assert!(json_result.is_ok(), "Even with minimal logging, response should be valid JSON");
    
    // Should be small and contain no debug artifacts
    assert!(response_body.len() < 300, "Minimal response should be small");
    assert!(!response_body.contains("🚀"), "Should not contain debug emojis");
    assert!(!response_body.contains("Request ID:"), "Should not contain debug info");
}