# 🏗️ SolidMCP Design Patterns & Architecture Guide

**For Future AI Agents & Developers**

This guide explains the key design patterns, architectural decisions, and abstractions in SolidMCP. Understanding these patterns is essential for working with the codebase effectively.

## 📋 Table of Contents

1. [Core Architecture Overview](#core-architecture-overview)
2. [Key Design Patterns](#key-design-patterns)
3. [Error Handling Strategy](#error-handling-strategy)
4. [Session Management Architecture](#session-management-architecture)
5. [Testing Patterns](#testing-patterns)
6. [Performance Optimizations](#performance-optimizations)
7. [Future Agent Guidelines](#future-agent-guidelines)

## 🏛️ Core Architecture Overview

SolidMCP follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│             Framework Layer             │  ← High-level builder API
│         (McpServerBuilder)              │
├─────────────────────────────────────────┤
│            Protocol Layer               │  ← MCP protocol implementation  
│      (McpProtocolEngine)                │
├─────────────────────────────────────────┤
│           Transport Layer               │  ← HTTP/WebSocket handling
│     (HttpMcpHandler, WebSocket)         │
├─────────────────────────────────────────┤
│            Core Types                   │  ← Shared types and utilities
│   (McpError, ResourceLimits, etc.)     │
└─────────────────────────────────────────┘
```

### Key Principles

1. **Dependency Inversion**: Higher layers depend on abstractions, not concrete implementations
2. **Single Responsibility**: Each module has one clear purpose
3. **Type Safety**: Extensive use of Rust's type system to prevent runtime errors
4. **Zero-Copy where possible**: Minimize allocations in hot paths

## 🎯 Key Design Patterns

### 1. Builder Pattern (Framework Layer)

**Location**: `src/framework/builder/mod.rs`

The `McpServerBuilder` uses a fluent builder pattern for type-safe server construction:

```rust
let server = McpServerBuilder::new(context, "my-server", "1.0.0")
    .with_tool("search", "Search database", search_handler)
    .with_resource_provider(Box::new(FileProvider::new()))
    .with_limits(ResourceLimits {
        max_sessions: Some(1000),
        max_message_size: 1024 * 1024,
        ..Default::default()
    })
    .build()
    .await?;
```

**Key Features**:
- **Compile-time type checking** for tool inputs/outputs
- **Fluent chaining** for ergonomic API
- **Automatic schema generation** via `JsonSchema` derive
- **Context sharing** across all tools

**Why This Pattern**:
- Prevents invalid server configurations
- Clear API for users
- Type safety ensures tools match their declared schemas

### 2. Protocol Engine Pattern (Core)

**Location**: `src/shared.rs`

The `McpProtocolEngine` is the central message router:

```rust
pub struct McpProtocolEngine {
    session_handlers: Arc<DashMap<String, Arc<McpProtocolHandlerImpl>>>,
    handler: Option<Arc<dyn McpHandler>>,
    limits: ResourceLimits,
}
```

**Key Features**:
- **Per-session isolation** using DashMap for lock-free concurrency
- **Protocol version negotiation** supporting multiple MCP versions
- **Custom handler delegation** with fallback to built-in implementation
- **Resource limit enforcement** at the engine level

**Message Flow**:
```rust
Client Request → Transport → Engine → Custom Handler OR Built-in Handler → Response
```

**Why This Pattern**:
- **Session isolation**: Multiple clients can't interfere with each other
- **Lock-free performance**: DashMap enables high concurrency
- **Extensibility**: Custom handlers can override any behavior
- **Protocol compliance**: Built-in handler ensures MCP compliance

### 3. Structured Error Handling

**Location**: `src/error.rs`

SolidMCP uses a comprehensive error type instead of `anyhow::Error`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("JSON-RPC parse error: {0}")]
    ParseError(String),
    
    #[error("Invalid method: {0}")]
    InvalidMethod(String),
    
    #[error("Message too large: {0} bytes (limit: {1})")]
    MessageTooLarge(usize, usize),
    
    #[error("Too many sessions: {0} (limit: {1})")]
    TooManySessions(usize),
    
    // ... more specific variants
}
```

**Why Structured Errors**:
- **Precise error handling**: Each error type has specific context
- **Better debugging**: Clear error messages with relevant data
- **API stability**: Changes to error types are breaking changes (good!)
- **JSON-RPC compliance**: Maps cleanly to JSON-RPC error codes

### 4. Modular HTTP Handler Architecture

**Location**: `src/http/` (multiple modules)

The HTTP handler was refactored from a 630-line monolith into focused modules:

```
src/http/
├── mod.rs           # Public API
├── session.rs       # Session management
├── validation.rs    # Request validation  
├── response.rs      # Response building
└── progress.rs      # Progress notifications
```

**Pattern Benefits**:
- **Single Responsibility**: Each module has one clear purpose
- **Testability**: Modules can be tested in isolation
- **Maintainability**: Changes are localized to relevant modules
- **Reusability**: Modules can be composed differently

### 5. Resource Limits Pattern

**Location**: `src/limits.rs`

Resource limits are enforced at multiple layers:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_sessions: Option<usize>,
    pub max_message_size: usize,
    pub max_tools: Option<usize>,
    pub max_resources: Option<usize>,
    pub max_prompts: Option<usize>,
}
```

**Enforcement Points**:
1. **Message Size**: Validated in `McpProtocolEngine::handle_message`
2. **Session Count**: Enforced when creating new sessions
3. **Registration Limits**: Checked during tool/resource registration

**Why This Pattern**:
- **DoS Protection**: Prevents resource exhaustion attacks
- **Operational Safety**: Servers can handle load gracefully
- **Configurable**: Limits can be tuned per deployment

### 6. Zero-Copy JSON Processing

**Location**: `src/protocol/message.rs`

SolidMCP uses `serde_json::RawValue` for zero-copy parsing:

```rust
pub struct RawMessage<'a> {
    pub jsonrpc: &'a str,
    pub id: Option<&'a RawValue>,
    pub method: &'a str,
    pub params: Option<&'a RawValue>,
}
```

**Performance Benefits**:
- **Single Parse Pass**: Message parsed once, not multiple times
- **Zero Allocations**: Parameters stay as borrowed `&RawValue`
- **Lazy Evaluation**: Only parse params when needed
- **25% Performance Gain**: Measured improvement in benchmarks

## 🚨 Error Handling Strategy

### The McpError Hierarchy

```rust
McpError
├── Protocol Errors (JSON-RPC related)
│   ├── ParseError
│   ├── InvalidRequest
│   └── InvalidMethod
├── Resource Errors (limits/constraints)
│   ├── MessageTooLarge
│   ├── TooManySessions
│   └── TooManyTools
├── Handler Errors (user code issues)
│   ├── ToolError
│   ├── ResourceError
│   └── PromptError
└── System Errors (infrastructure)
    ├── Json (serde_json::Error)
    ├── Io (std::io::Error)
    └── Network (transport errors)
```

### Error Propagation Rules

1. **Library Errors**: Always use `McpError` types
2. **User Handler Errors**: Convert to `McpError::ToolError` etc.
3. **System Errors**: Wrap in appropriate `McpError` variant
4. **Error Context**: Always include relevant context (sizes, limits, etc.)

### JSON-RPC Error Mapping

```rust
impl McpError {
    pub fn to_jsonrpc_code(&self) -> i32 {
        match self {
            McpError::ParseError(_) => -32700,
            McpError::InvalidRequest(_) => -32600,  
            McpError::InvalidMethod(_) => -32601,
            McpError::InvalidParams(_) => -32602,
            _ => -32603, // Internal error
        }
    }
}
```

## 🔄 Session Management Architecture

### Session Storage Strategy

SolidMCP uses **DashMap** for lock-free session storage:

```rust
session_handlers: Arc<DashMap<String, Arc<McpProtocolHandlerImpl>>>
```

**Why DashMap**:
- **Lock-free reads**: Multiple clients can read concurrently
- **Minimal write contention**: Only blocks specific hash buckets
- **Memory efficient**: No global locks
- **Arc wrapping**: Handlers can be shared safely

### Session Lifecycle

```
1. Client connects → Transport detects session ID
2. Engine checks session_handlers.contains_key(id)
3. If new session:
   a. Check session limits
   b. Create new McpProtocolHandlerImpl
   c. Insert into DashMap
4. Route message to session handler
5. Handler maintains per-session state
```

### Session Isolation

Each session has independent:
- **Initialization state**: Can be re-initialized
- **Protocol version**: Negotiated per session
- **Client info**: Stored per session
- **Handler state**: No shared mutable state

## 🧪 Testing Patterns

### Test Organization

```
tests/
├── *_test.rs           # Integration tests
├── helpers/mod.rs      # Test utilities
└── mcp_test_helpers.rs # MCP-specific helpers
```

### Key Testing Patterns

1. **Test Server Pattern**:
```rust
let server = TestServer::start().await?;
let client = McpHttpClient::new();
let response = client.initialize(&server.http_url()).await?;
```

2. **Parallel Test Execution**:
```rust
#[tokio::test]
async fn test_concurrent_clients() {
    let tasks: Vec<_> = (0..10).map(|i| {
        tokio::spawn(async move {
            // Each task gets independent client
        })
    }).collect();
}
```

3. **Mock Handler Pattern**:
```rust
struct MockHandler;
impl McpHandler for MockHandler {
    // Override specific methods for testing
}
```

### Test Categories

- **Unit Tests**: In `src/` alongside code (`#[cfg(test)]` modules)
- **Integration Tests**: In `tests/` directory
- **Protocol Compliance**: JSON-RPC 2.0 compliance tests
- **Performance Tests**: Benchmarks in `benches/`

## ⚡ Performance Optimizations

### 1. Zero-Copy JSON Parsing

```rust
// Old way (multiple parsing passes)
let message: Value = serde_json::from_str(&body)?;
let method = message["method"].as_str().unwrap();
let params: MyParams = serde_json::from_value(message["params"])?;

// New way (single pass, zero-copy)  
let raw: RawMessage = serde_json::from_str(&body)?;
let params: MyParams = serde_json::from_str(raw.params.get())?;
```

### 2. Lock-Free Session Management

```rust
// Old way (global mutex)
let mut sessions = SESSIONS.lock().unwrap();
let handler = sessions.get_mut(&session_id);

// New way (lock-free DashMap)
let handler = self.session_handlers.get(&session_id);
```

### 3. Efficient String Handling

```rust
// Prefer &str over String where possible
pub fn handle_method(method: &str) -> McpResult<()>

// Use Cow<str> for conditional ownership
pub fn validate_path(path: Cow<str>) -> McpResult<PathBuf>
```

## 🤖 Future Agent Guidelines

### When Working with SolidMCP

1. **Always Read the Architecture First**:
   - Understand the layer you're working in
   - Check existing patterns before creating new ones
   - Look for similar implementations in the codebase

2. **Error Handling Rules**:
   - Use `McpError` types, never `anyhow::Error`
   - Include context in error messages
   - Map to appropriate JSON-RPC codes
   - Test error scenarios explicitly

3. **Performance Considerations**:
   - Avoid allocations in hot paths
   - Use `&str` over `String` when possible
   - Consider zero-copy patterns for JSON
   - Profile before optimizing

4. **Testing Requirements**:
   - Write tests for new functionality
   - Use existing test helpers
   - Test error cases
   - Include integration tests for protocol changes

5. **API Design Principles**:
   - Type safety over convenience
   - Explicit over implicit
   - Composable abstractions
   - Clear error messages

### Common Tasks & Patterns

#### Adding a New Tool

```rust
// 1. Define input/output types with JsonSchema
#[derive(JsonSchema, Deserialize)]
struct SearchInput {
    query: String,
    limit: Option<u32>,
}

#[derive(JsonSchema, Serialize)]  
struct SearchOutput {
    results: Vec<String>,
}

// 2. Implement handler
async fn search_handler(
    input: SearchInput, 
    ctx: Arc<MyContext>
) -> Result<SearchOutput, McpError> {
    // Implementation
}

// 3. Register with builder
let server = McpServerBuilder::new(context, "server", "1.0.0")
    .with_tool("search", "Search the database", search_handler)
    .build()
    .await?;
```

#### Adding Transport Support

1. Create new module in `src/`
2. Implement transport-specific logic
3. Integrate with `McpProtocolEngine`
4. Add transport detection in `src/transport.rs`
5. Update server routing

#### Extending Error Types

```rust
// Add new variant to McpError
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    // ... existing variants
    
    #[error("Custom error: {reason}")]
    CustomError { reason: String },
}

// Update JSON-RPC mapping
impl McpError {
    pub fn to_jsonrpc_code(&self) -> i32 {
        match self {
            // ... existing mappings
            McpError::CustomError { .. } => -32000, // Custom range
        }
    }
}
```

### Code Review Checklist

- ✅ Uses appropriate error types (`McpError`, not `anyhow`)
- ✅ Follows existing module organization
- ✅ Includes comprehensive tests
- ✅ Documents public APIs with rustdoc
- ✅ Handles edge cases and error scenarios
- ✅ Considers performance implications
- ✅ Maintains backward compatibility
- ✅ Updates relevant documentation

### Debugging Tips

1. **Use structured logging**:
```rust
tracing::debug!(
    session_id = %session_id,
    method = %method,
    "Processing request"
);
```

2. **Enable debug logging**:
```bash
RUST_LOG=solidmcp=debug cargo test
```

3. **Use test helpers for debugging**:
```rust
let server = TestServer::start().await?;
println!("Test server running at: {}", server.http_url());
// Server stays running for manual testing
```

4. **Check session state**:
```rust
let session_count = protocol_engine.session_count();
tracing::info!(session_count, "Current sessions");
```

---

## 📚 Additional Resources

- **API Documentation**: Run `cargo doc --open`
- **Examples**: See `examples/` directory
- **Tests**: Study `tests/` for usage patterns
- **Benchmarks**: Run `cargo bench` for performance testing

---

**🎯 Remember**: SolidMCP prioritizes **correctness**, **performance**, and **maintainability** in that order. When in doubt, choose the approach that makes bugs impossible over the approach that's slightly more convenient.

---

*This guide is living documentation. Update it when you make architectural changes or discover new patterns.*