# SolidMCP

A standalone implementation of the Model Context Protocol (MCP) server in Rust, supporting both WebSocket and HTTP transports.

## Features

- **Dual Transport Support**: Both WebSocket and HTTP endpoints
- **Protocol Compatibility**: Supports MCP protocol versions 2025-03-26 and 2025-06-18
- **Session Management**: Maintains client state across requests
- **Built-in Tools**: Includes example tools (echo, read_file)
- **Comprehensive Logging**: Detailed debug logging for troubleshooting
- **Validation**: Message validation with detailed error reporting

## Usage

### Running the Server

```bash
cargo run
```

This starts the MCP server on port 3000 by default.

### Endpoints

- WebSocket: `ws://localhost:3000/mcp`
- HTTP: `http://localhost:3000/mcp`

### Example Client Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {},
    "clientInfo": {
      "name": "example-client",
      "version": "1.0.0"
    }
  }
}
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
solidmcp = { path = "../solidmcp" }
```

Create and start a server:

```rust
use solidmcp::McpServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = McpServer::new().await?;
    server.start(3000).await?;
    Ok(())
}
```

## Architecture

- `core.rs` - Core server implementation
- `protocol.rs` - Protocol definitions and utilities
- `protocol_impl.rs` - Concrete protocol handler implementation
- `handlers.rs` - Request handlers
- `shared.rs` - Shared logic between transports
- `tools.rs` - Tool implementations
- `validation.rs` - Message validation
- `websocket.rs` - WebSocket transport
- `http.rs` - HTTP transport
- `logging.rs` - Logging utilities

## License

MIT