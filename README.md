# 🚀 SolidMCP

**Production-ready Model Context Protocol (MCP) server framework for Rust**

[![Crates.io](https://img.shields.io/crates/v/solidmcp.svg)](https://crates.io/crates/solidmcp)
[![Documentation](https://docs.rs/solidmcp/badge.svg)](https://docs.rs/solidmcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/your-org/solidmcp/workflows/CI/badge.svg)](https://github.com/your-org/solidmcp/actions)

SolidMCP is a high-performance, type-safe Rust framework for building [Model Context Protocol](https://modelcontextprotocol.io) servers that AI assistants (like Claude) can interact with.

## ✨ Features

- 🛡️ **Production Ready**: Health checks, resource limits, structured logging  
- ⚡ **High Performance**: Zero-copy JSON parsing, lock-free concurrency
- 🔒 **Type Safe**: Compile-time guarantees with automatic JSON schema generation
- 🌐 **Multi-Transport**: HTTP and WebSocket support on the same port
- 🔧 **Batteries Included**: Tools, resources, prompts, and notifications
- 📊 **Observable**: Built-in metrics, tracing, and health monitoring
- 🧪 **Well Tested**: 164+ tests with comprehensive coverage

## Quick Start

Add SolidMCP to your `Cargo.toml`:

```toml
[dependencies]
solidmcp = { git = "https://github.com/jupitersoftco/solidmcp.git" }
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
- WebSocket and HTTP endpoints with smart transport negotiation
- Built-in error handling and logging
- Type-safe tool execution
- CORS support for web clients

## Transport Layer

SolidMCP features an intelligent transport system that automatically detects and negotiates capabilities:

### 🔄 **Automatic Transport Detection**

- **WebSocket Upgrade**: Detects `Upgrade: websocket` headers and establishes WebSocket connections
- **HTTP JSON-RPC**: Falls back to HTTP with session management for compatibility
- **Transport Discovery**: GET requests return capability information for client negotiation
- **CORS Support**: Full CORS headers for web-based MCP clients

### 🌟 **Smart Capability Negotiation**

```rust
// Server automatically detects client capabilities from headers:
// - Connection: upgrade + Upgrade: websocket → WebSocket transport
// - Accept: application/json → HTTP JSON-RPC transport
// - GET request → Transport capability discovery response
```

### 🔮 **Planned Transport Features**

- **Server-Sent Events (SSE)**: Real-time streaming support (currently disabled, marked for future implementation)
- **Enhanced WebSocket**: Advanced WebSocket features and sub-protocols
- **Custom Transports**: Plugin system for custom transport implementations

### 📡 **Current Transport Support Matrix**

| Transport              | Status              | Description                              |
| ---------------------- | ------------------- | ---------------------------------------- |
| **WebSocket**          | ✅ **Full Support** | Real-time bidirectional communication    |
| **HTTP JSON-RPC**      | ✅ **Full Support** | Request/response with session management |
| **Server-Sent Events** | 🔮 **Future Work**  | Streaming responses (architecture ready) |
| **Custom Transports**  | 🔮 **Planned**      | Plugin system for extensions             |

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

SolidMCP implements core MCP functionality with comprehensive transport support:

### ✅ **Fully Supported**

- **Protocol Versions**: `2025-03-26` and `2025-06-18`
- **Transport**: WebSocket and HTTP with intelligent capability detection
- **Session Management**: HTTP session cookies and state persistence
- **Transport Discovery**: GET endpoint for capability negotiation
- **CORS**: Full cross-origin support for web clients
- **Tools**: Execute functions with validated inputs and outputs (`tools/list`, `tools/call`)
- **Resources**: Expose data with URI-based access (`resources/list`, `resources/read`)
- **Prompts**: Provide templates with argument substitution (`prompts/list`, `prompts/get`)
- **Notifications**: Log messages and resource change notifications
- **Initialization**: Full handshake and capability negotiation

### 🚧 **Planned/Partial Support**

- **Server-Sent Events**: Architecture complete, implementation planned
- **Client Features**: Sampling, roots, and completion (server-side features not yet implemented)
- **Progress Tracking**: Basic framework exists, needs expansion
- **Cancellation**: Basic support for `notifications/cancel`
- **Configuration**: Environment variable support, needs structured config API

### ❌ **Not Yet Implemented**

- **Sampling**: LLM sampling requests from servers to clients
- **Roots**: Server-initiated boundary inquiries
- **Completion**: Advanced completion capabilities
- **Advanced Security**: Comprehensive consent flows and access controls

SolidMCP focuses on the **server-side** of MCP, providing everything needed to build robust MCP servers that work with existing MCP clients like Claude Desktop, with intelligent transport negotiation for optimal compatibility.

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

## Latest Dependencies

SolidMCP uses the latest stable versions of core dependencies for optimal performance and security:

- **Tokio 1.43** - Latest async runtime with performance improvements
- **Warp 0.3.8** - Lightweight web framework for HTTP transport
- **tokio-tungstenite 0.27** - Latest WebSocket implementation
- **Serde 1.0.217** - JSON serialization with latest optimizations
- **thiserror 2.0** - Enhanced error handling with latest improvements
- **rand 0.9** - Cryptographically secure random generation
- **schemars 0.8.21** - JSON schema generation for type safety

All dependencies are regularly updated and tested for compatibility.

## Architecture

SolidMCP is built with a modular architecture featuring intelligent transport handling:

- **Framework Layer** (`framework.rs`) - High-level builder API for easy server creation
- **Handler Traits** (`handler.rs`) - Core traits for tools, resources, and prompts
- **Protocol Engine** (`shared.rs`) - MCP protocol implementation and message routing
- **Transport Layer** (`websocket.rs`, `http.rs`) - WebSocket and HTTP server implementations
- **Transport Capability Detection** (`transport.rs`) - Smart transport negotiation and discovery
- **Core Server** (`core.rs`) - Server lifecycle and connection management

The library abstracts away the complexity of the MCP protocol while providing full access to all its features, with automatic transport detection ensuring compatibility across different MCP clients.

## Current Limitations

SolidMCP is a **server-focused** implementation. Some limitations to be aware of:

1. **Server-Sent Events**: Architecture is ready but implementation is marked as future work
2. **Client Features**: Does not implement client-side capabilities like sampling, roots, or completion
3. **Progress API**: Basic progress tracking exists but needs expansion for complex operations
4. **Security Model**: Basic session management, but enterprise-grade security features are planned
5. **Config Management**: Relies on environment variables; structured configuration API coming
6. **Protocol Extensions**: Focuses on core MCP; custom protocol extensions not yet supported

These limitations reflect our focus on providing an excellent **server development experience**. Client features and advanced capabilities are on the roadmap.

## Testing

SolidMCP includes a comprehensive test suite with 99+ tests covering all functionality:

```bash
# Run all tests (99+ tests)
cargo test

# Run library tests specifically
cargo test --lib

# Run integration tests
cargo test --test "*integration*"

# Run with logging
RUST_LOG=debug cargo test

# Test transport capability detection
cargo test transport::tests

# Test HTTP functionality
cargo test http::tests
```

**Test Coverage:**

- ✅ **Transport Detection**: WebSocket, HTTP, and capability negotiation
- ✅ **Protocol Compliance**: JSON-RPC 2.0 compliance and error handling
- ✅ **Session Management**: HTTP sessions and state persistence
- ✅ **Tool Execution**: Type-safe tool calls and validation
- ✅ **Resource Access**: URI-based resource listing and reading
- ✅ **Prompt System**: Template generation and argument substitution
- ✅ **Error Handling**: Comprehensive error scenarios and recovery
- ✅ **Concurrent Access**: Multi-client and high-load scenarios

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
