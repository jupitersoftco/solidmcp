# SolidMCP API Reference

This document provides a comprehensive reference for the SolidMCP Rust framework's public API.

## Table of Contents

1. [Core Server API](#core-server-api)
2. [Framework API](#framework-api)
3. [Handler Trait API](#handler-trait-api)
4. [Protocol Engine API](#protocol-engine-api)
5. [Type Definitions](#type-definitions)
6. [Error Handling](#error-handling)

## Core Server API

### `McpServer`

The main server struct that handles both WebSocket and HTTP transports.

```rust
pub struct McpServer {
    protocol: McpProtocol,
    protocol_engine: Arc<McpProtocolEngine>,
}
```

#### Methods

##### `new() -> Result<Self>`

Create a new MCP server instance with the default built-in handler.

```rust
let mut server = McpServer::new().await?;
```

##### `with_handler(handler: Arc<dyn McpHandler>) -> Result<Self>`

Create a new MCP server instance with a custom handler.

```rust
let handler = Arc::new(MyHandler);
let mut server = McpServer::with_handler(handler).await?;
```

##### `start(&mut self, port: u16) -> Result<()>`

Start the server on the specified port. Supports both WebSocket and HTTP on the same port.

```rust
server.start(3000).await?;
```

## Framework API

### `McpServerBuilder<C>`

Fluent builder for creating MCP servers with minimal boilerplate.

```rust
pub struct McpServerBuilder<C> {
    handler: FrameworkHandler<C>,
}
```

#### Type Parameters

- `C`: Application context type (shared across all tools)

#### Methods

##### `new(context: C, server_name: &str, server_version: &str) -> Self`

Create a new server builder with custom context.

```rust
let builder = McpServerBuilder::new(
    AppContext { db: Database::new() },
    "my-server",
    "1.0.0"
);
```

##### `with_tool<I, O, F, Fut>(...) -> Self`

Register a tool with automatic schema generation.

**Type Parameters:**
- `I`: Input type (must implement `JsonSchema` + `DeserializeOwned`)
- `O`: Output type (must implement `JsonSchema` + `Serialize`)
- `F`: Handler function type
- `Fut`: Future returned by handler

```rust
#[derive(JsonSchema, Deserialize)]
struct AddInput { a: i32, b: i32 }

#[derive(JsonSchema, Serialize)]
struct AddOutput { result: i32 }

builder.with_tool("add", "Add two numbers", |input: AddInput, ctx, notif| async move {
    Ok(AddOutput { result: input.a + input.b })
})
```

##### `with_resource_provider(provider: Box<dyn ResourceProvider<C>>) -> Self`

Add a resource provider for exposing data.

```rust
builder.with_resource_provider(Box::new(FileSystemProvider::new()))
```

##### `with_prompt_provider(provider: Box<dyn PromptProvider<C>>) -> Self`

Add a prompt provider for exposing templates.

```rust
builder.with_prompt_provider(Box::new(CodeReviewProvider))
```

##### `build() -> Result<McpServer>`

Build and return the configured server.

```rust
let server = builder.build().await?;
```

### `NotificationCtx`

Ergonomic context for sending notifications.

#### Methods

##### `info(&self, message: impl Into<String>) -> Result<()>`

Send an informational notification.

```rust
ctx.info("Processing started")?;
```

##### `debug(&self, message: impl Into<String>) -> Result<()>`

Send a debug notification.

```rust
ctx.debug("Internal state: processing")?;
```

##### `warn(&self, message: impl Into<String>) -> Result<()>`

Send a warning notification.

```rust
ctx.warn("This might take a while")?;
```

##### `error(&self, message: impl Into<String>) -> Result<()>`

Send an error notification.

```rust
ctx.error("Failed to process file")?;
```

##### `log<T>(&self, level: LogLevel, message: impl Into<String>, data: Option<T>) -> Result<()>`

Send a log notification with custom level and data.

```rust
ctx.log(LogLevel::Info, "Progress", Some(json!({
    "progress": 50,
    "total": 100
})))?;
```

##### `resources_changed(&self) -> Result<()>`

Notify that resources have changed.

```rust
ctx.resources_changed()?;
```

### Traits

#### `ResourceProvider<C>`

Trait for providing resources dynamically.

```rust
#[async_trait]
pub trait ResourceProvider<C>: Send + Sync {
    async fn list_resources(&self, context: Arc<C>) -> Result<Vec<ResourceInfo>>;
    async fn read_resource(&self, uri: &str, context: Arc<C>) -> Result<ResourceContent>;
}
```

**Example Implementation:**

```rust
struct FileProvider { base_path: PathBuf }

#[async_trait]
impl ResourceProvider<AppContext> for FileProvider {
    async fn list_resources(&self, context: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
        // List files in base_path
    }
    
    async fn read_resource(&self, uri: &str, context: Arc<AppContext>) -> Result<ResourceContent> {
        // Read file content
    }
}
```

#### `PromptProvider<C>`

Trait for providing prompt templates.

```rust
#[async_trait]
pub trait PromptProvider<C>: Send + Sync {
    async fn list_prompts(&self, context: Arc<C>) -> Result<Vec<PromptInfo>>;
    async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<C>) -> Result<PromptContent>;
}
```

## Handler Trait API

### `McpHandler`

Core trait for implementing MCP functionality.

```rust
#[async_trait]
pub trait McpHandler: Send + Sync {
    // Required methods
    async fn list_tools(&self, context: &McpContext) -> Result<Vec<ToolDefinition>>;
    async fn call_tool(&self, name: &str, arguments: Value, context: &McpContext) -> Result<Value>;
    
    // Optional methods with defaults
    async fn initialize(&self, params: Value, context: &McpContext) -> Result<Value>;
    async fn list_resources(&self, context: &McpContext) -> Result<Vec<ResourceInfo>>;
    async fn read_resource(&self, uri: &str, context: &McpContext) -> Result<ResourceContent>;
    async fn list_prompts(&self, context: &McpContext) -> Result<Vec<PromptInfo>>;
    async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: &McpContext) -> Result<PromptContent>;
    async fn cancel_notification(&self, params: Value, context: &McpContext) -> Result<Value>;
    async fn handle_initialized(&self, context: &McpContext) -> Result<()>;
}
```

### `McpContext`

Context provided to handler methods.

```rust
pub struct McpContext {
    pub session_id: Option<String>,
    pub notification_sender: Option<mpsc::UnboundedSender<McpNotification>>,
    pub protocol_version: Option<String>,
    pub client_info: Option<Value>,
}
```

## Protocol Engine API

### `McpProtocolEngine`

Core protocol routing engine.

```rust
pub struct McpProtocolEngine {
    session_handlers: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>,
    handler: Option<Arc<dyn McpHandler>>,
}
```

#### Methods

##### `new() -> Self`

Create engine with no custom handler.

```rust
let engine = McpProtocolEngine::new();
```

##### `with_handler(handler: Arc<dyn McpHandler>) -> Self`

Create engine with custom handler.

```rust
let engine = McpProtocolEngine::with_handler(handler);
```

##### `handle_message(&self, message: Value, session_id: Option<String>) -> Result<Value>`

Process a JSON-RPC message and return response.

```rust
let response = engine.handle_message(message, Some("session-123".to_string())).await?;
```

## Type Definitions

### Tool Types

#### `ToolDefinition`

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

### Resource Types

#### `ResourceInfo`

```rust
pub struct ResourceInfo {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}
```

#### `ResourceContent`

```rust
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub content: String,
}
```

### Prompt Types

#### `PromptInfo`

```rust
pub struct PromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<PromptArgument>,
}
```

#### `PromptArgument`

```rust
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
```

#### `PromptContent`

```rust
pub struct PromptContent {
    pub messages: Vec<PromptMessage>,
}
```

#### `PromptMessage`

```rust
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}
```

### Notification Types

#### `McpNotification`

```rust
pub enum McpNotification {
    ToolsListChanged,
    ResourcesListChanged,
    PromptsListChanged,
    Progress {
        progress_token: String,
        progress: f64,
        total: Option<f64>,
    },
    LogMessage {
        level: LogLevel,
        logger: Option<String>,
        message: String,
        data: Option<Value>,
    },
    Custom {
        method: String,
        params: Option<Value>,
    },
}
```

#### `LogLevel`

```rust
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}
```

## Error Handling

SolidMCP uses `anyhow::Result<T>` for error handling throughout the API. All errors are automatically converted to appropriate JSON-RPC error responses:

- `-32700`: Parse error (malformed JSON)
- `-32600`: Invalid Request (missing required fields)
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error (handler errors)

### Best Practices

1. **Use descriptive error messages**: Include context about what failed
2. **Chain errors with context**: Use `.context("description")` for better debugging
3. **Return early on errors**: Use the `?` operator for clean error propagation

```rust
async fn my_tool(input: Input) -> Result<Output> {
    let data = fetch_data(&input.id)
        .await
        .context("Failed to fetch data")?;
    
    let processed = process_data(data)
        .context("Failed to process data")?;
    
    Ok(Output { result: processed })
}
```

## Complete Example

Here's a complete example using the framework API:

```rust
use solidmcp::framework::{McpServerBuilder, NotificationCtx};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use anyhow::Result;
use std::sync::Arc;

// Application context
struct AppContext {
    database: Database,
    config: Config,
}

// Tool definitions
#[derive(JsonSchema, Deserialize)]
struct SearchInput {
    query: String,
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 { 10 }

#[derive(JsonSchema, Serialize)]
struct SearchResult {
    items: Vec<String>,
    total: u32,
}

// Resource provider
struct DatabaseResourceProvider;

#[async_trait]
impl ResourceProvider<AppContext> for DatabaseResourceProvider {
    async fn list_resources(&self, ctx: Arc<AppContext>) -> Result<Vec<ResourceInfo>> {
        Ok(vec![
            ResourceInfo {
                uri: "db://users".to_string(),
                name: "Users Table".to_string(),
                description: Some("All user records".to_string()),
                mime_type: Some("application/json".to_string()),
            }
        ])
    }
    
    async fn read_resource(&self, uri: &str, ctx: Arc<AppContext>) -> Result<ResourceContent> {
        match uri {
            "db://users" => {
                let users = ctx.database.get_all_users().await?;
                Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    content: serde_json::to_string_pretty(&users)?,
                })
            }
            _ => Err(anyhow::anyhow!("Resource not found"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let context = AppContext {
        database: Database::connect("postgresql://localhost/mydb").await?,
        config: Config::from_env()?,
    };
    
    let server = McpServerBuilder::new(context, "search-server", "1.0.0")
        .with_tool("search", "Search the database", |input: SearchInput, ctx: Arc<AppContext>, notif: NotificationCtx| async move {
            notif.info(&format!("Searching for: {}", input.query))?;
            
            let results = ctx.database.search(&input.query, input.limit).await?;
            
            notif.info(&format!("Found {} results", results.len()))?;
            
            Ok(SearchResult {
                items: results,
                total: results.len() as u32,
            })
        })
        .with_resource_provider(Box::new(DatabaseResourceProvider))
        .build()
        .await?;
    
    println!("Server starting on http://localhost:3000");
    server.start(3000).await?;
    
    Ok(())
}
```