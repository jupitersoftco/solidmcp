//! Prompt Metadata Validation Tests
//!
//! Tests for validating prompt metadata including descriptions, arguments,
//! required/optional parameters, and schema compliance.

use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use solidmcp::{McpResult, McpError};

mod mcp_test_helpers;
mod prompt_test_helpers;

use mcp_test_helpers::*;
use prompt_test_helpers::*;

/// Test prompt metadata structure and completeness
#[tokio::test]
async fn test_prompt_metadata_structure() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    // Find an available port
    let port = find_available_port().await?;

    // Start test server with prompts
    let server = create_test_server_with_prompts().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and test
    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
    let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Send prompts/list request
    let list_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/list",
        "params": {}
    });

    write.send(Message::Text(serde_json::to_string(&list_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    let result = &response["result"];
    let prompts = result["prompts"].as_array().unwrap();

    // Test hello_world prompt metadata
    let hello_prompt = prompts.iter()
        .find(|p| p["name"] == "hello_world")
        .expect("hello_world prompt not found");

    assert_eq!(hello_prompt["name"], "hello_world");
    assert!(hello_prompt["description"].is_string());
    assert_eq!(hello_prompt["description"], "A simple hello world prompt");

    // Verify arguments structure
    let arguments = hello_prompt["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 1);

    let name_arg = &arguments[0];
    assert_eq!(name_arg["name"], "name");
    assert_eq!(name_arg["description"], "The name to greet");
    assert_eq!(name_arg["required"], true);

    server_handle.abort();
    Ok(())
}

/// Test prompt arguments validation with required/optional parameters
#[tokio::test]
async fn test_prompt_arguments_validation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    // Find an available port
    let port = find_available_port().await?;

    // Start test server with prompts
    let server = create_test_server_with_prompts().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and test
    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
    let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Test code_review prompt with optional parameter
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "code_review",
            "arguments": {
                "code": "fn main() { println!(\"Hello\"); }"
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify successful response with default language
    assert_eq!(response["jsonrpc"], "2.0");  
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);

    // Check that default language "unknown" was used
    let system_message = &messages[0];
    assert_eq!(system_message["role"], "system");
    assert!(system_message["content"]["text"].as_str().unwrap().contains("unknown"));

    server_handle.abort();
    Ok(())
}

/// Test prompt with both required and optional parameters
#[tokio::test]
async fn test_prompt_mixed_parameters() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    // Find an available port
    let port = find_available_port().await?;

    // Start test server with prompts
    let server = create_test_server_with_prompts().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and test
    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
    let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Test code_review prompt with both required and optional parameters
    let get_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/get",
        "params": {
            "name": "code_review",
            "arguments": {
                "code": "fn main() { println!(\"Hello\"); }",
                "language": "rust"
            }
        }
    });

    write.send(Message::Text(serde_json::to_string(&get_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify successful response with specified language
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("result").is_some());

    let result = &response["result"];
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);

    // Check that specified language "rust" was used
    let system_message = &messages[0];
    assert_eq!(system_message["role"], "system");
    assert!(system_message["content"]["text"].as_str().unwrap().contains("rust"));

    // Check user message contains the code
    let user_message = &messages[1];
    assert_eq!(user_message["role"], "user");
    assert!(user_message["content"]["text"].as_str().unwrap().contains("fn main()"));

    server_handle.abort();
    Ok(())
}

/// Test prompt schema compliance
#[tokio::test]
async fn test_prompt_schema_compliance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_test_tracing();

    // Find an available port
    let port = find_available_port().await?;

    // Start test server with prompts
    let server = create_test_server_with_prompts().await?;
    let server_handle = tokio::spawn(async move {
        let mut server = server;
        if let Err(e) = server.start(port).await {
            eprintln!("Test server error: {e}");
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and test
    let ws_url = format!("ws://127.0.0.1:{}/mcp", port);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Initialize connection
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    write.send(Message::Text(serde_json::to_string(&init_message)?.into())).await?;
    let _init_response = receive_ws_message(&mut read, Duration::from_secs(5)).await?;

    // Send prompts/list request
    let list_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "prompts/list",
        "params": {}
    });

    write.send(Message::Text(serde_json::to_string(&list_message)?.into())).await?;
    let response_text = receive_ws_message(&mut read, Duration::from_secs(5)).await?;
    let response: Value = serde_json::from_str(&response_text)?;

    // Verify all prompts have required schema fields
    let result = &response["result"];
    let prompts = result["prompts"].as_array().unwrap();

    for prompt in prompts {
        // Required fields
        assert!(prompt["name"].is_string());
        assert!(prompt["arguments"].is_array());
        
        // Optional fields that should be present in our test data
        assert!(prompt["description"].is_string());
        
        // Validate arguments structure
        let arguments = prompt["arguments"].as_array().unwrap();
        for arg in arguments {
            assert!(arg["name"].is_string());
            assert!(arg["required"].is_boolean());
            // Description is optional but present in our test data
            if arg["description"].is_string() {
                assert!(!arg["description"].as_str().unwrap().is_empty());
            }
        }
    }

    server_handle.abort();
    Ok(())
}