# 📖 SolidMCP Usage Guide

Complete guide for building MCP servers with SolidMCP.

## 🚀 Quick Start

### Basic Server

```rust
use solidmcp::{McpServerBuilder, McpResult};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::sync::Arc;

// Define your application context
#[derive(Clone)]
struct MyContext {
    database: Arc<Database>,
}

// Define tool input/output with JsonSchema for validation
#[derive(JsonSchema, Deserialize)]
struct SearchInput {
    query: String,
    limit: Option<u32>,
}

#[derive(JsonSchema, Serialize)]
struct SearchOutput {
    results: Vec<String>,
    total: u32,
}

#[tokio::main]
async fn main() -> McpResult<()> {
    let context = MyContext {
        database: Arc::new(Database::connect().await?),
    };

    let server = McpServerBuilder::new(context, "my-server", "1.0.0")
        .with_tool("search", "Search the database", |input: SearchInput, ctx: Arc<MyContext>, _notif| async move {
            let results = ctx.database.search(&input.query, input.limit.unwrap_or(10)).await?;
            
            Ok(SearchOutput {
                results: results.clone(),
                total: results.len() as u32,
            })
        })
        .build()
        .await?;

    println!("🚀 Server starting on http://localhost:3000");
    server.start(3000).await
}
```

### With Resource Limits & Health Checks

```rust
use solidmcp::{McpServerBuilder, ResourceLimits};

let server = McpServerBuilder::new(context, "production-server", "1.0.0")
    .with_limits(ResourceLimits {
        max_sessions: Some(1000),
        max_message_size: 1024 * 1024, // 1MB
        max_tools: Some(100),
        ..Default::default()
    })
    .with_tool("status", "Server status", status_handler)
    .build()
    .await?;

// Health check available at /health
server.start(3000).await?;
```

## 🛠️ Building Tools

### Simple Tool

```rust
use solidmcp::framework::NotificationSender;

async fn echo_handler(
    input: EchoInput,
    _ctx: Arc<MyContext>,
    notif: Option<NotificationSender>,
) -> McpResult<EchoOutput> {
    // Send progress notification (optional)
    if let Some(notif) = notif {
        notif.info("Processing echo request")?;
    }
    
    Ok(EchoOutput {
        message: format!("Echo: {}", input.text),
    })
}
```

### Tool with Error Handling

```rust
use solidmcp::error::McpError;

async fn divide_handler(
    input: DivideInput,
    _ctx: Arc<MyContext>,
    _notif: Option<NotificationSender>,
) -> McpResult<DivideOutput> {
    if input.divisor == 0.0 {
        return Err(McpError::InvalidParams("Cannot divide by zero".to_string()));
    }
    
    Ok(DivideOutput {
        result: input.dividend / input.divisor,
    })
}
```

### Async Tool with External API

```rust
async fn weather_handler(
    input: WeatherInput,
    ctx: Arc<MyContext>,
    notif: Option<NotificationSender>,
) -> McpResult<WeatherOutput> {
    if let Some(notif) = notif {
        notif.info(&format!("Fetching weather for {}", input.city))?;
    }
    
    let weather = ctx.weather_client
        .get_weather(&input.city)
        .await
        .map_err(|e| McpError::ToolError {
            tool: "weather".to_string(),
            message: format!("Weather API error: {}", e),
        })?;
    
    Ok(WeatherOutput {
        temperature: weather.temperature,
        condition: weather.condition,
        humidity: weather.humidity,
    })
}
```

## 📁 Resources

### File Resource Provider

```rust
use solidmcp::handler::{ResourceProvider, ResourceDefinition, ResourceContent};
use async_trait::async_trait;

struct FileResourceProvider {
    base_path: PathBuf,
}

#[async_trait]
impl ResourceProvider for FileResourceProvider {
    async fn list_resources(&self) -> McpResult<Vec<ResourceDefinition>> {
        let mut resources = Vec::new();
        
        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                resources.push(ResourceDefinition {
                    uri: format!("file://{}", path.display()),
                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                    description: Some(format!("File: {}", path.display())),
                    mime_type: Some("text/plain".to_string()),
                });
            }
        }
        
        Ok(resources)
    }
    
    async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        if !uri.starts_with("file://") {
            return Err(McpError::InvalidParams(format!("Invalid URI: {}", uri)));
        }
        
        let path = &uri[7..]; // Remove "file://"
        let full_path = self.base_path.join(path);
        
        // Security: ensure path is within base_path
        let canonical = full_path.canonicalize()
            .map_err(|_| McpError::ResourceError {
                uri: uri.to_string(),
                message: "File not found".to_string(),
            })?;
        
        if !canonical.starts_with(&self.base_path) {
            return Err(McpError::InvalidParams("Path traversal not allowed".to_string()));
        }
        
        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| McpError::ResourceError {
                uri: uri.to_string(),
                message: format!("Failed to read file: {}", e),
            })?;
        
        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: "text/plain".to_string(),
            content,
        })
    }
}

// Register with server
let server = McpServerBuilder::new(context, "file-server", "1.0.0")
    .with_resource_provider(Box::new(FileResourceProvider {
        base_path: PathBuf::from("./data"),
    }))
    .build()
    .await?;
```

## 📝 Prompts

### Dynamic Prompt Provider

```rust
use solidmcp::handler::{PromptProvider, PromptDefinition, PromptContent, PromptMessage};

struct CodeReviewPromptProvider;

#[async_trait]
impl PromptProvider for CodeReviewPromptProvider {
    async fn list_prompts(&self) -> McpResult<Vec<PromptDefinition>> {
        Ok(vec![
            PromptDefinition {
                name: "code_review".to_string(),
                description: Some("Generate code review comments".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "language".to_string(),
                        description: Some("Programming language".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "code".to_string(), 
                        description: Some("Code to review".to_string()),
                        required: true,
                    },
                ],
            }
        ])
    }
    
    async fn get_prompt(&self, name: &str, args: Option<Value>) -> McpResult<PromptContent> {
        match name {
            "code_review" => {
                let args = args.ok_or_else(|| McpError::InvalidParams("Arguments required".to_string()))?;
                let language = args["language"].as_str().ok_or_else(|| McpError::InvalidParams("Missing language".to_string()))?;
                let code = args["code"].as_str().ok_or_else(|| McpError::InvalidParams("Missing code".to_string()))?;
                
                Ok(PromptContent {
                    messages: vec![
                        PromptMessage {
                            role: "system".to_string(),
                            content: format!(
                                "You are a senior {} developer. Review the following code and provide constructive feedback.",
                                language
                            ),
                        },
                        PromptMessage {
                            role: "user".to_string(),
                            content: format!("Please review this {} code:\n\n```{}\n{}\n```", language, language, code),
                        },
                    ],
                })
            }
            _ => Err(McpError::UnknownPrompt(name.to_string())),
        }
    }
}
```

## 🔧 Configuration

### Production Configuration

```rust
use solidmcp::{ResourceLimits, logging};
use tracing_subscriber;

#[tokio::main]
async fn main() -> McpResult<()> {
    // Configure structured logging
    tracing_subscriber::fmt()
        .with_env_filter("info,solidmcp=debug")
        .json()
        .init();
    
    let context = ProductionContext::new().await?;
    
    let server = McpServerBuilder::new(context, "production-mcp", "1.2.0")
        .with_limits(ResourceLimits {
            max_sessions: Some(10000),
            max_message_size: 2 * 1024 * 1024, // 2MB
            max_tools: Some(500),
            max_resources: Some(10000),
            max_prompts: Some(100),
        })
        .with_tool("search", "Search service", search_handler)
        .with_tool("analyze", "Analyze data", analyze_handler)
        .with_resource_provider(Box::new(DatabaseResourceProvider::new()))
        .with_prompt_provider(Box::new(AIPromptProvider::new()))
        .build()
        .await?;
    
    // Health checks available at /health
    tracing::info!("🚀 Production server starting on port 8080");
    server.start(8080).await
}
```

### Development Configuration

```rust
#[tokio::main]
async fn main() -> McpResult<()> {
    // Simple console logging for development
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .pretty()
        .init();
    
    let context = DevContext::new();
    
    let server = McpServerBuilder::new(context, "dev-server", "0.1.0")
        .with_limits(ResourceLimits {
            max_sessions: Some(10),
            max_message_size: 64 * 1024, // 64KB
            ..Default::default()
        })
        .with_tool("echo", "Echo tool", echo_handler)
        .build()
        .await?;
    
    println!("🔧 Development server: http://localhost:3000");
    println!("🔍 Health check: http://localhost:3000/health");
    
    server.start(3000).await
}
```

## 🧪 Testing

### Testing Tools

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use solidmcp::testing::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_search_tool() -> McpResult<()> {
        let context = TestContext::new();
        let server = create_test_server(context).await?;
        
        let client = McpHttpClient::new();
        
        // Initialize client
        let init_response = client.initialize(&server.http_url()).await?;
        assert_eq!(init_response["result"]["capabilities"]["tools"]["listChanged"], true);
        
        // List tools
        let tools_response = client.list_tools(&server.http_url()).await?;
        let tools = tools_response["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "search"));
        
        // Call tool
        let result = client.call_tool(
            &server.http_url(),
            "search",
            json!({
                "query": "test",
                "limit": 5
            })
        ).await?;
        
        assert!(result["result"]["content"].is_array());
        Ok(())
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_workflow() -> McpResult<()> {
    let server = TestServer::start().await?;
    let client = McpHttpClient::new();
    
    // Test initialize -> list -> call workflow
    client.initialize(&server.http_url()).await?;
    
    let tools = client.list_tools(&server.http_url()).await?;
    assert!(!tools["result"]["tools"].as_array().unwrap().is_empty());
    
    let result = client.call_tool(
        &server.http_url(),
        "echo",
        json!({"text": "Hello, World!"})
    ).await?;
    
    assert_eq!(result["result"]["content"][0]["text"], "Echo: Hello, World!");
    
    Ok(())
}
```

## 📊 Monitoring & Observability

### Health Check Response

```json
{
  "status": "healthy",
  "timestamp": 1735776033,
  "version": "1.0.0",
  "session_count": 42,
  "uptime_seconds": 3600,
  "metadata": {
    "server_name": "my-mcp-server",
    "protocol_version": "2025-06-18"
  }
}
```

### Structured Logging

```rust
use tracing::{info, debug, error, instrument};

#[instrument(fields(tool_name = name))]
async fn call_tool_handler(name: &str, args: Value) -> McpResult<Value> {
    debug!(args = ?args, "Tool called");
    
    let start = std::time::Instant::now();
    let result = execute_tool(name, args).await;
    let duration = start.elapsed();
    
    match &result {
        Ok(_) => info!(duration_ms = duration.as_millis(), "Tool completed successfully"),
        Err(e) => error!(error = %e, duration_ms = duration.as_millis(), "Tool failed"),
    }
    
    result
}
```

### Metrics Integration

```rust
// Using metrics crate
use metrics::{counter, histogram, gauge};

async fn handle_request(method: &str) -> McpResult<Value> {
    counter!("mcp_requests_total", "method" => method).increment(1);
    
    let start = std::time::Instant::now();
    let result = process_request(method).await;
    let duration = start.elapsed();
    
    histogram!("mcp_request_duration_seconds", "method" => method)
        .record(duration.as_secs_f64());
    
    match &result {
        Ok(_) => counter!("mcp_requests_success", "method" => method).increment(1),
        Err(_) => counter!("mcp_requests_error", "method" => method).increment(1),
    }
    
    gauge!("mcp_active_sessions").set(session_count() as f64);
    
    result
}
```

## 🚨 Error Handling

### Custom Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyAppError {
    #[error("Database connection failed: {0}")]
    DatabaseError(String),
    
    #[error("Invalid configuration: {field}")]
    ConfigError { field: String },
    
    #[error("External API error: {service} - {message}")]
    ExternalApiError { service: String, message: String },
}

// Convert to McpError
impl From<MyAppError> for McpError {
    fn from(err: MyAppError) -> Self {
        match err {
            MyAppError::DatabaseError(msg) => McpError::ToolError {
                tool: "database".to_string(),
                message: msg,
            },
            MyAppError::ConfigError { field } => McpError::InvalidParams(
                format!("Configuration error in field: {}", field)
            ),
            MyAppError::ExternalApiError { service, message } => McpError::ToolError {
                tool: service,
                message,
            },
        }
    }
}
```

### Error Recovery

```rust
async fn resilient_handler(
    input: MyInput,
    ctx: Arc<MyContext>,
    notif: Option<NotificationSender>,
) -> McpResult<MyOutput> {
    // Retry with exponential backoff
    let mut retry_count = 0;
    let max_retries = 3;
    
    loop {
        match attempt_operation(&input, &ctx).await {
            Ok(result) => return Ok(result),
            Err(e) if retry_count < max_retries && is_retryable(&e) => {
                retry_count += 1;
                let delay = Duration::from_millis(100 * 2_u64.pow(retry_count));
                
                if let Some(ref notif) = notif {
                    notif.warning(&format!("Attempt {} failed, retrying in {}ms", retry_count, delay.as_millis()))?;
                }
                
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
}
```

## 🔐 Security Best Practices

### Path Validation

```rust
use std::path::{Path, PathBuf};

fn validate_path(requested_path: &str, allowed_dir: &Path) -> McpResult<PathBuf> {
    let requested = Path::new(requested_path);
    
    // Convert to absolute path within allowed directory
    let full_path = if requested.is_relative() {
        allowed_dir.join(requested)
    } else {
        return Err(McpError::InvalidParams("Absolute paths not allowed".to_string()));
    };
    
    // Resolve symlinks and .. segments
    let canonical = full_path.canonicalize()
        .map_err(|e| McpError::InvalidParams(format!("Invalid path: {}", e)))?;
    
    let allowed_canonical = allowed_dir.canonicalize()
        .map_err(|_| McpError::InvalidParams("Invalid base directory".to_string()))?;
    
    // Ensure resolved path is within allowed directory
    if !canonical.starts_with(&allowed_canonical) {
        return Err(McpError::InvalidParams("Path traversal attempt detected".to_string()));
    }
    
    Ok(canonical)
}
```

### Input Validation

```rust
#[derive(JsonSchema, Deserialize)]
struct ValidatedInput {
    #[schemars(regex = "^[a-zA-Z0-9_-]+$")]
    username: String,
    
    #[schemars(range(min = 1, max = 100))]
    count: u32,
    
    #[schemars(email)]
    email: String,
}

async fn secure_handler(
    input: ValidatedInput,
    _ctx: Arc<MyContext>,
    _notif: Option<NotificationSender>,
) -> McpResult<MyOutput> {
    // Input is already validated by JsonSchema
    // Additional business logic validation here
    
    if input.username.len() < 3 {
        return Err(McpError::InvalidParams("Username too short".to_string()));
    }
    
    // Process validated input...
    Ok(MyOutput { /* ... */ })
}
```

---

## 📚 More Examples

See the `examples/` directory for complete working examples:

- **`examples/toy/`**: Basic server with echo and file tools
- **`examples/database/`**: Database integration example
- **`examples/api_client/`**: External API integration
- **`examples/production/`**: Production-ready configuration

---

## 🆘 Troubleshooting

### Common Issues

**1. Tool not found**
```
Error: Unknown tool: my_tool
```
→ Check tool name matches exactly, ensure tool is registered

**2. Schema validation failed**
```
Error: Invalid params: missing field `required_field`
```
→ Check input struct derives JsonSchema and matches client parameters

**3. Session initialization failed**
```
Error: Client not initialized
```
→ Client must call `initialize` before other methods

**4. Resource limits exceeded**
```
Error: Message too large: 2097152 bytes (limit: 1048576)
```
→ Increase `max_message_size` in ResourceLimits or reduce message size

### Debug Mode

```bash
RUST_LOG=solidmcp=debug,my_app=debug cargo run
```

### Health Check

```bash
curl http://localhost:3000/health
```

---

**💡 Pro Tip**: Start with the `examples/toy` server and gradually add complexity as you understand the patterns!