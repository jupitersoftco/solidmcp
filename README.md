# SolidMCP

A high-level Rust toolkit for building [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) servers with minimal boilerplate and maximum type safety.

## Features

- **🚀 Minimal Boilerplate**: Build MCP servers with just a few lines of code
- **🛡️ Type Safety**: Compile-time guarantees with automatic JSON schema generation
- **🔌 Dual Transport**: Built-in WebSocket and HTTP support
- **📚 Full MCP Support**: Tools, Resources, and Prompts with a unified API
- **🏗️ Flexible Architecture**: Generic context system for any application state
- **🔔 Notifications**: Simple API for sending log messages and updates
- **✅ Battle-tested**: Comprehensive test suite covering all MCP features

## Quick Start

Add SolidMCP to your `Cargo.toml`:

```toml
[dependencies]
solidmcp = { git = "https://github.com/yourusername/solidmcp.git" }
anyhow = "1.0"
schemars = "0.8"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```

Create a simple MCP server:

```rust
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use solidmcp::framework::McpServerBuilder;
use std::sync::Arc;

// Define your application context
#[derive(Debug)]
struct MyContext {
    name: String,
}

// Define input/output schemas with automatic JSON schema generation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GreetInput {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GreetOutput {
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let context = MyContext { name: "MyApp".to_string() };

    let mut server = McpServerBuilder::new(context, "my-mcp-server", "1.0.0")
        .with_tool(
            "greet",
            "Greet someone by name",
            |input: GreetInput, ctx: Arc<MyContext>, notify| async move {
                notify.info(&format!("Greeting {}", input.name))?;
                
                Ok(GreetOutput {
                    message: format!("Hello, {}! Welcome to {}", input.name, ctx.name),
                })
            },
        )
        .build()
        .await?;

    server.start(3000).await?;
    Ok(())
}
```

That's it! Your MCP server is now running with:
- Automatic JSON schema generation and validation
- WebSocket and HTTP endpoints
- Built-in error handling and logging
- Type-safe tool execution

## Complete Example

For a comprehensive example demonstrating all MCP features (tools, resources, and prompts), see the [toy notes server example](examples/toy/). It shows how to build a complete note-taking MCP server with:

- **Tools**: Create, read, list, and delete notes
- **Resources**: Expose notes as MCP resources with `note://` URIs  
- **Prompts**: Provide templates for meeting notes, daily journals, and tasks
- **Persistence**: File-based storage with automatic loading
- **Notifications**: Real-time updates when notes are modified

Run the example:

```bash
cd examples/toy
cargo run
```

Then connect with Claude Desktop or any MCP client at `ws://localhost:3002/mcp` or `http://localhost:3002/mcp`.

## MCP Protocol Support

SolidMCP implements the complete MCP specification:

- **Protocol Versions**: `2025-03-26` and `2025-06-18`
- **Transport**: WebSocket and HTTP with session management
- **Tools**: Execute functions with validated inputs and outputs  
- **Resources**: Expose data with URI-based access
- **Prompts**: Provide templates with argument substitution
- **Notifications**: Send log messages and capability change notifications

## Advanced Usage

### Custom Resource Providers

Expose your application data as MCP resources:

```rust
use async_trait::async_trait;
use solidmcp::framework::ResourceProvider;
use solidmcp::handler::{ResourceContent, ResourceInfo};

struct MyResourceProvider;

#[async_trait]
impl ResourceProvider<MyContext> for MyResourceProvider {
    async fn list_resources(&self, context: Arc<MyContext>) -> Result<Vec<ResourceInfo>> {
        Ok(vec![ResourceInfo {
            uri: "data://example".to_string(),
            name: "example".to_string(),
            description: Some("Example resource".to_string()),
            mime_type: Some("text/plain".to_string()),
        }])
    }

    async fn read_resource(&self, uri: &str, context: Arc<MyContext>) -> Result<ResourceContent> {
        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            content: "Hello from resource!".to_string(),
        })
    }
}

// Add to your server
let server = McpServerBuilder::new(context, "my-server", "1.0.0")
    .with_resource_provider(Box::new(MyResourceProvider))
    .build()
    .await?;
```

### Custom Prompt Providers

Provide dynamic templates for AI interactions:

```rust
use solidmcp::framework::PromptProvider;
use solidmcp::handler::{PromptContent, PromptInfo, PromptMessage};

struct MyPromptProvider;

#[async_trait]
impl PromptProvider<MyContext> for MyPromptProvider {
    async fn list_prompts(&self, context: Arc<MyContext>) -> Result<Vec<PromptInfo>> {
        // Return available prompts
    }

    async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<MyContext>) -> Result<PromptContent> {
        // Generate prompt content based on arguments
    }
}
```

### Configuration

The framework supports various configuration options:

```rust
// Custom port
server.start(8080).await?;

// Environment variables
// PORT=8080 - Server port
// RUST_LOG=debug - Logging level
```

## Architecture

SolidMCP is built with a modular architecture:

- **Framework Layer** (`framework.rs`) - High-level builder API for easy server creation
- **Handler Traits** (`handler.rs`) - Core traits for tools, resources, and prompts
- **Protocol Engine** (`shared.rs`) - MCP protocol implementation and message routing
- **Transport Layer** (`websocket.rs`, `http.rs`) - WebSocket and HTTP server implementations
- **Core Server** (`core.rs`) - Server lifecycle and connection management

The library abstracts away the complexity of the MCP protocol while providing full access to all its features.

## Testing

SolidMCP includes a comprehensive test suite:

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test "*integration*"

# Run with logging
RUST_LOG=debug cargo test
```

## Examples

- **[Basic Server](examples/basic_server.rs)** - Minimal MCP server setup
- **[Custom Tools](examples/custom_tools.rs)** - Implementing custom tools
- **[Toy Notes Server](examples/toy/)** - Complete application with all MCP features
- **[HTTP Server](examples/http_server.rs)** - HTTP-only server configuration
- **[WebSocket Server](examples/websocket_server.rs)** - WebSocket-only server

## Contributing

Contributions are welcome! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

## License

MIT