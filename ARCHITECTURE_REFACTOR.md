# SolidMCP Architecture Refactor Plan

## Current Issues

### 1. Naming Confusion
- `SharedMcpHandler` suggests it's doing MCP handling, but it should be a trait users implement
- The name implies shared state rather than a pluggable interface

### 2. Built-in Tools Problem
- Current architecture has hardcoded built-in tools (`echo`, `read_file`)
- A generic library shouldn't provide specific functionality - only the framework
- Users can't easily disable or replace built-in tools

### 3. Mixed Responsibilities
- Core protocol is doing business logic instead of just protocol handling
- Two competing systems: built-in tools vs custom tools
- Protocol handler contains tool implementations rather than delegating

## Target Architecture

### 1. Pure Trait-Based Design
```rust
// Core trait that users must implement
pub trait McpHandler: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>>;
    async fn call_tool(&self, name: &str, args: Value, context: &ToolContext) -> Result<Value>;
    async fn list_resources(&self) -> Result<Vec<ResourceInfo>>;
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent>;
    async fn list_prompts(&self) -> Result<Vec<PromptInfo>>;
    async fn get_prompt(&self, name: &str, args: Option<Value>) -> Result<PromptContent>;
}

// Simple usage
struct MyNotesHandler { /* user state */ }
impl McpHandler for MyNotesHandler { /* user implementation */ }

let server = McpServer::new(MyNotesHandler::new());
server.start(3000).await?;
```

### 2. No Built-in Tools
- Remove `McpTools` entirely
- Remove hardcoded `echo` and `read_file` tools
- Library provides only protocol framework
- Users implement their own tools as needed

### 3. Clean Separation of Concerns
- **Protocol Layer**: JSON-RPC parsing, MCP message routing, session management
- **Handler Interface**: Trait definition for user implementations
- **User Implementation**: Business logic in user's trait impl

## Implementation Plan

### Phase 1: Create New Trait Interface
1. Define `McpHandler` trait with all MCP methods
2. Create `ToolContext` for accessing notifications, session info, etc.
3. Define all supporting types (`ToolDefinition`, `ResourceInfo`, etc.)

### Phase 2: Remove Built-in Tools
1. Delete `McpTools` module entirely
2. Remove hardcoded tool implementations from protocol handler
3. Update protocol handler to delegate to `McpHandler` trait

### Phase 3: Simplify Core Architecture
1. Rename `SharedMcpHandler` to something like `McpProtocolEngine`
2. Make it accept `Arc<dyn McpHandler>` in constructor
3. Route all MCP requests to the trait implementation

### Phase 4: Update High-Level API
1. `McpServerBuilder` becomes a convenience wrapper
2. Internally creates a struct that implements `McpHandler`
3. Users can choose: implement trait directly OR use builder pattern

### Phase 5: Update Examples and Tests
1. Toy example implements `McpHandler` directly
2. Built-in tests provide test implementations
3. Update all documentation

## Benefits

### 1. True Generic Library
- No opinions about what tools should exist
- Users have complete control over functionality
- Library focuses on protocol correctness only

### 2. Cleaner Architecture
- Single responsibility: protocol vs business logic
- Easier to test and maintain
- No competing systems

### 3. Better Developer Experience
```rust
// Simple case - implement trait directly
impl McpHandler for MyHandler { ... }

// Complex case - use builder for common patterns
McpServerBuilder::new()
    .add_tool(CustomTool::new())
    .add_resource_provider(FileProvider::new())
    .build() // returns something that implements McpHandler
```

### 4. Protocol Compliance
- All MCP features work the same way
- No special cases for built-in vs custom
- Consistent behavior across all implementations

## Migration Strategy

### Backward Compatibility
- Keep `McpServerBuilder` API working
- Internally, make it create a struct that implements `McpHandler`
- Existing toy example continues to work

### Testing Strategy
- Create comprehensive test suite with mock `McpHandler` implementations
- Test all MCP protocol features through trait interface
- Remove tests that depend on built-in tools

## File Changes Required

### New Files
- `src/handler.rs` - `McpHandler` trait definition
- `src/context.rs` - `ToolContext` and related types

### Modified Files
- `src/core.rs` - Accept `Arc<dyn McpHandler>` instead of custom tools
- `src/shared.rs` - Rename and simplify to `McpProtocolEngine`
- `src/protocol_impl.rs` - Remove tool implementations, delegate to trait
- `src/server.rs` - Make builder create a `McpHandler` implementation
- `examples/toy/src/main.rs` - Implement `McpHandler` trait

### Deleted Files
- `src/tools.rs` - Remove built-in tools entirely

## Success Criteria

1. **Zero built-in functionality** - Library provides only protocol framework
2. **Single integration point** - All customization goes through `McpHandler` trait
3. **Toy example works** - Demonstrates real-world usage
4. **All tests pass** - Protocol compliance maintained
5. **Clean API** - Both trait-based and builder patterns work smoothly

This refactor transforms solidmcp from "an MCP server with customization options" into "an MCP protocol framework that requires user implementation" - which is exactly what a generic library should be.