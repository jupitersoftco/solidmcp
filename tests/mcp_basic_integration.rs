//! Basic MCP Integration Tests
//!
//! Tests the MCP server functionality without complex WebSocket testing.

mod mcp_test_helpers;
use mcp_test_helpers::init_test_tracing;

/// Test MCP server creation and basic functionality
#[tokio::test]
async fn test_mcp_server_basic_functionality() {
    init_test_tracing();

    // Test server creation
    let _server = solidmcp::McpServer::new().await.unwrap();
    println!("✅ MCP server created successfully");

    // Test tools list
    let tools_list = solidmcp::McpTools::get_tools_list();
    assert!(tools_list["tools"].is_array());

    let tools = tools_list["tools"].as_array().unwrap();
    assert!(tools.len() >= 2); // Should have echo and read_file

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"echo"));
    assert!(tool_names.contains(&"read_file"));
    println!("✅ Tools list validated: {tool_names:?}");

    // Test echo tool
    let echo_result = solidmcp::McpTools::execute_tool(
        "echo",
        serde_json::json!({
            "message": "Hello from basic integration test!"
        }),
    )
    .await;

    assert!(echo_result.is_ok());
    let echo_result = echo_result.unwrap();

    let content = echo_result["content"][0]["text"].as_str().unwrap();
    println!("DEBUG: Content from echo tool: '{content}'");
    // The tool returns human-readable text, not JSON. Check the structured data instead.
    let data = &echo_result["data"];
    assert_eq!(data["echo"], "Hello from basic integration test!");
    println!("✅ Echo tool validated");

    // Test read_file tool with existing file
    let read_result = solidmcp::McpTools::execute_tool(
        "read_file",
        serde_json::json!({
            "file_path": "Cargo.toml"
        }),
    )
    .await;

    assert!(read_result.is_ok());
    let read_result = read_result.unwrap();

    let content = read_result["content"][0]["text"].as_str().unwrap();
    println!("DEBUG: Content from read_file tool: '{content}'");
    // Check the structured data instead of parsing the human-readable text
    let data = &read_result["data"];
    assert_eq!(data["file_path"], "Cargo.toml");
    assert!(data["content"].as_str().is_some());
    println!("✅ Read file tool validated");

    // Test read_file tool with non-existent file
    let read_error_result = solidmcp::McpTools::execute_tool(
        "read_file",
        serde_json::json!({
            "file_path": "/nonexistent/file"
        }),
    )
    .await;

    assert!(read_error_result.is_ok());
    let read_error_result = read_error_result.unwrap();

    let content = read_error_result["content"][0]["text"].as_str().unwrap();
    println!("DEBUG: Content from read_file error: '{content}'");
    // Check the structured data for error
    let data = &read_error_result["data"];
    assert!(data["error"].as_str().is_some());
    println!("✅ Read file error handling validated");

    // Test unknown tool
    let unknown_result =
        solidmcp::McpTools::execute_tool("unknown_tool", serde_json::json!({})).await;
    assert!(unknown_result.is_err());
    println!("✅ Unknown tool error handling validated");

    println!("🎉 All basic MCP functionality tests passed!");
}

/// Test MCP protocol message handling
#[tokio::test]
async fn test_mcp_protocol_messages() {
    init_test_tracing();

    let _server = solidmcp::McpServer::new().await.unwrap();
    let protocol = solidmcp::McpProtocol::new();

    // Test initialize response
    let init_response = protocol.create_initialize_response();
    assert!(init_response["protocolVersion"].as_str().is_some());
    assert!(init_response["serverInfo"]["name"].as_str().unwrap() == "mcp-server");
    println!("✅ Initialize response validated");

    // Test success response
    let test_result = serde_json::json!({"test": "data"});
    let success_response =
        protocol.create_success_response(serde_json::json!(1), test_result.clone());
    assert_eq!(success_response["jsonrpc"], "2.0");
    assert_eq!(success_response["id"], 1);
    assert_eq!(success_response["result"], test_result);
    println!("✅ Success response validated");

    // Test error response
    let error_response =
        protocol.create_error_response(serde_json::json!(2), -32603, "Internal error");
    assert_eq!(error_response["jsonrpc"], "2.0");
    assert_eq!(error_response["id"], 2);
    assert_eq!(error_response["error"]["code"], -32603);
    assert_eq!(error_response["error"]["message"], "Internal error");
    println!("✅ Error response validated");

    println!("🎉 All MCP protocol message tests passed!");
}

/// Test MCP handler functionality
#[tokio::test]
async fn test_mcp_handlers() {
    init_test_tracing();

    let _server = solidmcp::McpServer::new().await.unwrap();
    let connection_id = solidmcp::logging::McpConnectionId::new();
    let logger = solidmcp::logging::McpDebugLogger::new(connection_id);
    let handler = solidmcp::handlers::McpHandlers::new(logger);

    // Test initialize handler
    let init_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = handler.handle_mcp_message(init_message).await.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    println!("✅ Initialize handler validated");

    // Test tools/list handler
    let tools_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = handler.handle_mcp_message(tools_message).await.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());
    assert!(response["result"]["tools"].is_array());
    println!("✅ Tools list handler validated");

    // Test tools/call handler
    let tool_call_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "message": "Hello from handler test!"
            }
        }
    });

    let response = handler.handle_mcp_message(tool_call_message).await.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object());
    println!("✅ Tool call handler validated");

    // Test unknown method handler
    let unknown_message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "unknown_method",
        "params": {}
    });

    let response = handler.handle_mcp_message(unknown_message).await;
    assert!(response.is_err());
    println!("✅ Unknown method error handling validated");

    println!("🎉 All MCP handler tests passed!");
}
