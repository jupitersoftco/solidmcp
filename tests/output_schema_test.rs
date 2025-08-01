mod mcp_test_helpers;

use {
    solidmcp::{McpResult, McpError},
    schemars::JsonSchema,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    solidmcp::{McpServerBuilder, NotificationCtx},
    std::sync::Arc,
};

const MCP_VERSION_LATEST: &str = "2025-06-18";

#[derive(Debug, Clone)]
struct TestContext {
    _name: String,
}

#[derive(JsonSchema, Deserialize)]
struct CalculateInput {
    a: f64,
    b: f64,
    operation: String,
}

#[derive(JsonSchema, Serialize)]
struct CalculateOutput {
    result: f64,
    formula: String,
    computed_at: String,
}

#[tokio::test]
async fn test_tool_with_output_schema() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a server with a tool that has both input and output schemas
    let context = TestContext {
        _name: "test".to_string(),
    };

    let mut server = McpServerBuilder::new(context, "output-schema-test", "1.0.0")
        .with_tool_schemas(
            "calculate",
            "Perform mathematical calculations",
            |input: CalculateInput, _ctx: Arc<TestContext>, notif: NotificationCtx| async move {
                notif.info(&format!("Computing {} {} {}", input.a, input.operation, input.b))?;

                let result = match input.operation.as_str() {
                    "add" => input.a + input.b,
                    "subtract" => input.a - input.b,
                    "multiply" => input.a * input.b,
                    "divide" => {
                        if input.b == 0.0 {
                            return Err(McpError::InvalidParams("Division by zero"));
                        }
                        input.a / input.b
                    }
                    _ => return Err(McpError::InvalidParams(format!("Unknown operation: {}", input.operation))),
                };

                Ok(CalculateOutput {
                    result,
                    formula: format!("{} {} {} = {}", input.a, input.operation, input.b, result),
                    computed_at: format!("2024-01-01T00:00:00Z"),
                })
            },
        )
        .build()
        .await?;

    // Start the server on a random port
    let port = mcp_test_helpers::find_available_port().await.unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });
    
    // Wait for server to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    {
        let client = reqwest::Client::new();

        // Initialize the connection
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_VERSION_LATEST,
                "capabilities": {},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }
        });

        let response = client.post(&url).json(&init).send().await?;
        
        // Check response status
        let status = response.status();
        let response_text = response.text().await?;
        
        if !status.is_success() {
            panic!("Init request failed with status {}: {}", status, response_text);
        }
        
        let init_response: Value = serde_json::from_str(&response_text)
            .map_err(|e| McpError::InvalidParams(format!("Failed to parse init response: {} - Body: {}", e, response_text)))?;
        assert_eq!(init_response["result"]["protocolVersion"], MCP_VERSION_LATEST);

        // List tools to verify output schema is included
        let list_tools = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let response = client.post(&url).json(&list_tools).send().await?;
        let tools_response: Value = response.json().await?;

        // Verify the tool has both input and output schemas
        let tools = tools_response["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);

        let tool = &tools[0];
        assert_eq!(tool["name"], "calculate");
        assert_eq!(tool["description"], "Perform mathematical calculations");

        // Debug print the tool structure
        println!("Tool structure: {}", serde_json::to_string_pretty(&tool).unwrap());
        
        // Verify input schema (camelCase in protocol)
        let input_schema = &tool["inputSchema"];
        assert!(input_schema.is_object(), "inputSchema is not an object: {:?}", input_schema);
        assert!(input_schema["properties"].is_object());
        assert!(input_schema["properties"]["a"].is_object());
        assert!(input_schema["properties"]["b"].is_object());
        assert!(input_schema["properties"]["operation"].is_object());

        // Verify output schema exists and is properly formed (camelCase in protocol)
        let output_schema = &tool["outputSchema"];
        assert!(output_schema.is_object(), "outputSchema is not an object: {:?}", output_schema);
        assert!(output_schema["properties"].is_object());
        assert!(output_schema["properties"]["result"].is_object());
        assert!(output_schema["properties"]["formula"].is_object());
        assert!(output_schema["properties"]["computed_at"].is_object());

        // Test calling the tool
        let call_tool = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "calculate",
                "arguments": {
                    "a": 10.0,
                    "b": 5.0,
                    "operation": "multiply"
                }
            }
        });

        let response = client.post(&url).json(&call_tool).send().await?;
        let call_response: Value = response.json().await?;

        // Verify the output matches the schema
        let result = &call_response["result"];
        assert_eq!(result["result"], 50.0);
        assert_eq!(result["formula"], "10 multiply 5 = 50");
        assert!(result["computed_at"].is_string());
    }

    Ok(())
}

#[derive(JsonSchema, Deserialize)]
struct SearchInput {
    query: String,
    limit: Option<u32>,
    filters: Option<SearchFilters>,
}

#[derive(JsonSchema, Deserialize)]
struct SearchFilters {
    category: Option<String>,
    min_score: Option<f32>,
}

#[derive(JsonSchema, Serialize)]
struct SearchOutput {
    results: Vec<SearchResult>,
    total_count: u32,
    query_time_ms: u64,
}

#[derive(JsonSchema, Serialize)]
struct SearchResult {
    id: String,
    title: String,
    score: f32,
    snippet: String,
}

#[tokio::test]
async fn test_complex_nested_schemas() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Test with more complex nested types
    let context = TestContext {
        _name: "search-test".to_string(),
    };

    let mut server = McpServerBuilder::new(context, "search-server", "1.0.0")
        .with_tool_schemas(
            "search",
            "Search through documents",
            |input: SearchInput, _ctx: Arc<TestContext>, _notif: NotificationCtx| async move {
                let limit = input.limit.unwrap_or(10);
                let mut results = Vec::new();

                for i in 0..limit.min(3) {
                    results.push(SearchResult {
                        id: format!("doc-{}", i),
                        title: format!("Document {} matching '{}'", i, input.query),
                        score: 0.95 - (i as f32 * 0.1),
                        snippet: format!("...{} found in document...", input.query),
                    });
                }

                Ok(SearchOutput {
                    results,
                    total_count: 42,
                    query_time_ms: 123,
                })
            },
        )
        .build()
        .await?;

    // Start the server on a random port
    let port = mcp_test_helpers::find_available_port().await.unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", port);
    
    tokio::spawn(async move {
        server.start(port).await.unwrap();
    });
    
    // Wait for server to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    {
        let client = reqwest::Client::new();

        // Initialize
        let init = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_VERSION_LATEST,
                "capabilities": {},
                "clientInfo": {
                    "name": "test",
                    "version": "1.0"
                }
            }
        });

        client.post(&url).json(&init).send().await?;

        // List tools
        let list_tools = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let response = client.post(&url).json(&list_tools).send().await?;
        let tools_response: Value = response.json().await?;

        let tool = &tools_response["result"]["tools"][0];

        // Verify nested input schema (camelCase in protocol)
        let input_schema = &tool["inputSchema"];
        assert!(input_schema["properties"]["filters"].is_object());
        // The schema uses $ref for nested types, so we need to check the $defs
        assert!(input_schema["$defs"]["SearchFilters"].is_object());
        assert!(input_schema["$defs"]["SearchFilters"]["properties"]["category"].is_object());
        assert!(input_schema["$defs"]["SearchFilters"]["properties"]["min_score"].is_object());

        // Verify nested output schema (camelCase in protocol)
        let output_schema = &tool["outputSchema"];
        assert!(output_schema["properties"]["results"].is_object());
        assert!(output_schema["properties"]["results"]["items"].is_object());
        // The items use $ref to SearchResult
        assert!(output_schema["$defs"]["SearchResult"].is_object());
        assert!(output_schema["$defs"]["SearchResult"]["properties"]["id"].is_object());
        assert!(output_schema["$defs"]["SearchResult"]["properties"]["title"].is_object());
        assert!(output_schema["$defs"]["SearchResult"]["properties"]["score"].is_object());
        assert!(output_schema["$defs"]["SearchResult"]["properties"]["snippet"].is_object());
    }

    Ok(())
}