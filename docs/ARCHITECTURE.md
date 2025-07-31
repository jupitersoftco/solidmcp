# SolidMCP Architecture

This document provides a comprehensive overview of the SolidMCP framework architecture, consolidating information from various sources to provide a single authoritative reference.

## Table of Contents

1. [Overview](#overview)
2. [Core Components](#core-components)
3. [Architectural Layers](#architectural-layers)
4. [Transport Layer](#transport-layer)
5. [Protocol Engine](#protocol-engine)
6. [Framework Layer](#framework-layer)
7. [Handler System](#handler-system)
8. [Session Management](#session-management)
9. [Error Handling](#error-handling)
10. [Performance Considerations](#performance-considerations)
11. [Future Architecture](#future-architecture)

## Overview

SolidMCP is a Rust framework for building MCP (Model Context Protocol) servers with minimal boilerplate. The architecture is designed around several key principles:

- **Transport Agnostic**: Supports multiple transport protocols on a single port
- **Type Safety**: Compile-time type checking with automatic schema generation
- **Session Isolation**: Per-session state management for proper client isolation
- **Extensibility**: Plugin-based architecture for tools, resources, and prompts
- **Performance**: Async/await with efficient connection pooling

## Core Components

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        MCP Client                            │
└─────────────────────┬───────────────┬───────────────────────┘
                      │               │
                 WebSocket          HTTP/SSE
                      │               │
┌─────────────────────┴───────────────┴───────────────────────┐
│                    Transport Layer                           │
│  ┌─────────────┐  ┌─────────────┐  ┌───────────────────┐   │
│  │  WebSocket  │  │    HTTP     │  │  SSE (planned)    │   │
│  │  Handler    │  │   Handler   │  │    Handler        │   │
│  └─────────────┘  └─────────────┘  └───────────────────┘   │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────────────┐
│                  Protocol Engine                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           McpProtocolEngine                          │   │
│  │  - Message routing                                   │   │
│  │  - Session management                                │   │
│  │  - Protocol version negotiation                      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────────────┐
│                   Handler Layer                              │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐    │
│  │   Custom     │  │   Built-in   │  │   Framework    │    │
│  │   Handler    │  │   Handler    │  │    Handler     │    │
│  └──────────────┘  └──────────────┘  └────────────────┘    │
└──────────────────────────────────────────────────────────────┘
```

### File Structure

```
src/
├── core.rs              # McpServer - main entry point
├── shared.rs            # McpProtocolEngine - message routing
├── transport.rs         # Transport detection and negotiation
├── http.rs              # HTTP transport implementation
├── websocket.rs         # WebSocket transport implementation
├── handler.rs           # McpHandler trait and types
├── protocol_impl.rs     # Built-in protocol implementation
├── framework.rs         # High-level framework API
└── lib.rs               # Public API exports
```

## Architectural Layers

### 1. Transport Layer

The transport layer handles protocol-specific communication details:

**Transport Detection (`transport.rs`)**:
- Analyzes HTTP headers to determine client capabilities
- Negotiates optimal transport based on:
  - `Upgrade: websocket` header
  - `Accept: text/event-stream` header
  - `Content-Type: application/json` header

**WebSocket Transport (`websocket.rs`)**:
- Real-time bidirectional communication
- Maintains persistent connections
- Per-connection session state
- Automatic ping/pong handling

**HTTP Transport (`http.rs`)**:
- Request/response pattern
- Session management via cookies
- CORS support for web clients
- Conditional chunked encoding for progress

### 2. Protocol Engine

The `McpProtocolEngine` is the core message router:

```rust
pub struct McpProtocolEngine {
    session_handlers: Arc<Mutex<HashMap<String, McpProtocolHandlerImpl>>>,
    handler: Option<Arc<dyn McpHandler>>,
}
```

**Responsibilities**:
- Message validation and parsing
- Session lifecycle management
- Protocol version negotiation
- Handler delegation based on method

**Message Flow**:
1. Transport layer receives raw message
2. Engine validates JSON-RPC structure
3. Engine looks up session handler
4. Routes to appropriate handler method
5. Returns JSON-RPC response

### 3. Framework Layer

The framework provides a high-level API for building servers:

**McpServerBuilder**:
```rust
McpServerBuilder::new(context, "server-name", "1.0.0")
    .with_tool("tool_name", "description", handler)
    .with_resource_provider(provider)
    .with_prompt_provider(provider)
    .build()
```

**Features**:
- Automatic JSON schema generation
- Type-safe tool registration
- Ergonomic notification API
- Context sharing across handlers

**NotificationCtx**:
- Simplified notification sending
- Log level abstractions
- Resource change notifications

### 4. Handler System

The handler system defines how MCP functionality is implemented:

**McpHandler Trait**:
- Core abstraction for MCP functionality
- Required methods: `list_tools`, `call_tool`
- Optional methods with defaults for resources, prompts, etc.

**Handler Types**:
1. **Custom Handler**: User-implemented functionality
2. **Built-in Handler**: Default protocol compliance
3. **Framework Handler**: Automatic routing for registered tools

## Session Management

### WebSocket Sessions

- One session per connection
- State maintained in connection scope
- Automatic cleanup on disconnect

### HTTP Sessions

- Cookie-based session tracking (`mcp_session`)
- Session ID generation on first request
- State persistence across requests
- Re-initialization support for reconnecting clients

### Session State

Each session maintains:
- Initialization status
- Protocol version
- Client information
- Handler instance

## Error Handling

### JSON-RPC Error Codes

| Code | Constant | Description |
|------|----------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid JSON-RPC |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid parameters |
| -32603 | Internal error | Server error |

### Error Propagation

1. **Handler Errors**: Converted to -32603 Internal error
2. **Validation Errors**: Converted to -32602 Invalid params
3. **Protocol Errors**: Appropriate JSON-RPC error code
4. **Transport Errors**: Connection-specific handling

## Performance Considerations

### Connection Pooling

- Shared `Arc<McpProtocolEngine>` across connections
- Per-session mutex for state isolation
- Minimal lock contention

### Memory Management

- Lazy session creation
- Automatic session cleanup
- Bounded message sizes (configurable)

### Async Processing

- Tokio runtime for concurrent connections
- Non-blocking I/O throughout
- Parallel request handling

### Optimization Strategies

1. **Message Batching**: Process multiple requests per connection
2. **Progress Streaming**: SSE for long-running operations
3. **Resource Caching**: Client-side caching hints
4. **Connection Reuse**: HTTP keep-alive, WebSocket persistence

## Future Architecture

### Planned Enhancements

1. **Server-Sent Events (SSE)**:
   - Streaming responses for progress
   - Real-time notifications
   - Graceful degradation

2. **Middleware System**:
   - Authentication/authorization
   - Rate limiting
   - Request logging
   - Metrics collection

3. **Plugin Architecture**:
   - Dynamic handler loading
   - Hot reloading support
   - Version management

4. **Clustering Support**:
   - Multi-instance coordination
   - Shared session storage
   - Load balancing

### Architecture Evolution

The architecture is designed to evolve without breaking changes:

1. **Protocol Versions**: Support multiple MCP versions
2. **Transport Plugins**: Add new transports as modules
3. **Handler Extensions**: Enhance without modifying trait
4. **Backward Compatibility**: Maintain API stability

## Design Patterns

### 1. **Type State Pattern**
Used in framework for compile-time guarantees:
```rust
pub struct TypedToolDefinition<T: JsonSchema> {
    // Type parameter ensures schema generation
}
```

### 2. **Builder Pattern**
Fluent API for server configuration:
```rust
McpServerBuilder::new(context)
    .with_tool(...)
    .build()
```

### 3. **Strategy Pattern**
Transport selection based on capabilities:
```rust
impl TransportNegotiation {
    fn negotiate(...) -> TransportType
}
```

### 4. **Factory Pattern**
Session handler creation:
```rust
sessions.entry(key).or_insert_with(|| {
    McpProtocolHandlerImpl::new()
})
```

## Security Considerations

### Input Validation

- JSON schema validation for tool inputs
- Size limits on messages
- Rate limiting (planned)

### Session Isolation

- No shared mutable state between sessions
- Separate handler instances per session
- Cookie security (HttpOnly, SameSite)

### Transport Security

- HTTPS support (deployment dependent)
- WebSocket origin validation
- CORS configuration

## Testing Architecture

### Unit Testing

- Handler trait implementations
- Protocol engine logic
- Transport negotiation

### Integration Testing

- End-to-end protocol flows
- Multi-transport scenarios
- Session management

### Performance Testing

- Concurrent connection handling
- Large message processing
- Memory usage under load

## Conclusion

SolidMCP's architecture provides a solid foundation for building MCP servers with:

- **Flexibility**: Multiple transports and handler types
- **Safety**: Type-safe API with compile-time checks
- **Performance**: Efficient async processing
- **Extensibility**: Plugin-based design

The modular architecture ensures that new features can be added without disrupting existing functionality, making it suitable for both simple tools and complex enterprise integrations.