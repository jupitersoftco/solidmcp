# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, and Development Commands

```bash
# Build the project
cargo build

# Run all tests (99+ tests)
cargo test

# Run specific test categories
cargo test --lib                          # Library unit tests
cargo test --test "*integration*"         # Integration tests
cargo test --test http_protocol_compliance_test  # Specific test file
cargo test transport::tests               # Transport detection tests
cargo test http::tests                    # HTTP functionality tests

# Run with logging enabled
RUST_LOG=debug cargo test

# Format code
cargo fmt

# Lint code
cargo clippy --all-targets -- -D warnings

# Run the example server
cd examples/toy && cargo run

# Run with custom port
PORT=8080 cargo run
```

## High-Level Architecture

SolidMCP is a Rust framework for building MCP (Model Context Protocol) servers with minimal boilerplate. The architecture consists of several key layers:

### Core Protocol Engine (`src/shared.rs`)
- **McpProtocolEngine**: Central message router that maintains per-session protocol handlers
- Uses `Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>` for thread-safe session management
- Handles protocol version negotiation (supports both `2025-03-26` and `2025-06-18`)
- Routes messages to custom handlers or falls back to built-in implementation

### Transport Layer
- **WebSocket (`src/websocket.rs`)**: Real-time bidirectional communication
- **HTTP (`src/http.rs`)**: Request/response with session management via cookies
- **Transport Detection (`src/transport.rs`)**: Automatically detects client capabilities from headers
- Smart capability negotiation based on `Upgrade: websocket` and `Accept` headers

### Framework Layer (`src/framework.rs`)
- **McpServerBuilder**: High-level API for building servers with type safety
- Generic context system `<C>` allows any application state
- Automatic JSON schema generation via `schemars`
- Tool registration with compile-time type checking

### Handler System (`src/handler.rs`)
- **McpHandler trait**: Core abstraction for implementing MCP functionality
- **Tool execution**: Type-safe function calls with validated inputs/outputs
- **Resource providers**: Expose data via URI-based access
- **Prompt providers**: Dynamic template generation

### Protocol Implementation (`src/protocol_impl.rs`)
- **McpProtocolHandlerImpl**: Built-in implementation of MCP protocol
- Maintains initialization state and client info per session
- Handles protocol handshake and capability negotiation
- Implements core MCP methods (initialize, tools/list, tools/call, etc.)

## Key Design Patterns

### Session Management
- HTTP uses session cookies (`mcp_session`) for state persistence
- WebSocket maintains state per connection
- Sessions can be re-initialized (important for reconnecting clients like Cursor)

### Error Handling
- All `unwrap()` calls have been replaced with proper error handling
- JSON-RPC errors returned with appropriate codes (-32600, -32601, etc.)
- Graceful handling of malformed messages and large payloads

### Type Safety
- Tools use generic parameters with `JsonSchema` for automatic validation
- Input/output types are compile-time checked
- Schema generation ensures protocol compliance

### Concurrent Access
- Thread-safe session storage with Mutex protection
- Each session has isolated state
- No shared mutable state between sessions

## Recent Critical Fixes

The codebase recently underwent fixes for several protocol implementation issues:

1. **HTTP Protocol Compliance**: Chunked encoding is now conditional based on progress tokens
2. **Session Re-initialization**: Clients can now reconnect and re-initialize
3. **Panic Prevention**: All unwrap() calls replaced with error handling
4. **Large Message Support**: Verified to handle messages up to 2MB

See `docs/fixes/` for detailed documentation of each fix.

## Testing Philosophy

- Test files follow the pattern `tests/*_test.rs`
- Each major fix has a corresponding test file proving the issue is resolved
- Tests use the `mcp_test_helpers` module for consistent server setup
- Integration tests run actual WebSocket and HTTP servers on random ports

## Important Implementation Details

- The server supports both WebSocket and HTTP transports on the same port
- HTTP sessions use cookies for state management
- Progress tokens trigger chunked encoding for streaming responses
- The built-in handler provides basic MCP functionality even without custom handlers
- Transport detection happens automatically based on request headers