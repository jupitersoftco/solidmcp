# Toy Notes Server - Complete SolidMCP Example

A comprehensive example demonstrating all MCP features using the SolidMCP framework. This example shows how to build a real-world MCP server with minimal boilerplate while supporting the complete MCP specification and intelligent transport negotiation.

## Features Demonstrated

### 🛠️ Tools (Type-Safe with Auto Schema Generation)

- **add_note** - Create a new note with automatic timestamp
- **list_notes** - List all available notes with count
- **read_note** - Read a specific note by name
- **delete_note** - Delete a note with confirmation
- **send_notification** - Send log messages at different levels

### 📚 Resources (URI-Based Access)

- Notes are exposed as MCP resources with `note://` URIs
- Automatic resource listing with metadata
- Content served as markdown with proper MIME types

### 📝 Prompts (Dynamic Templates)

- **meeting_notes** - Template for structured meeting notes
- **task_note** - Template for task tracking with priority and due dates
- **daily_journal** - Template for daily journal entries with default dates

### 🔔 Notifications (Clean API)

- Simple notification helpers: `notify.info()`, `notify.warn()`, `notify.error()`
- Automatic log level handling
- Real-time updates when notes are modified

### 🌐 Enhanced Transport Support

- **Smart Transport Detection**: Automatically detects WebSocket vs HTTP capabilities
- **CORS Support**: Full cross-origin headers for web-based MCP clients
- **Transport Discovery**: GET endpoint provides capability information
- **Graceful Fallback**: Seamless fallback from WebSocket to HTTP JSON-RPC

## Running the Server

```bash
# From the solidmcp root directory
cd examples/toy
cargo run
```

The server will start on port 3002 by default. You can customize this with the `PORT` environment variable:

```bash
PORT=8080 cargo run
```

Notes are stored in a temporary directory by default. You can customize this with the `NOTES_DIR` environment variable:

```bash
NOTES_DIR=/path/to/notes cargo run
```

## Connection Endpoints

The toy server provides multiple connection options:

- **WebSocket**: `ws://localhost:3002/mcp` - Full-duplex real-time communication
- **HTTP JSON-RPC**: `http://localhost:3002/mcp` - Request/response with session management
- **Transport Discovery**: `GET http://localhost:3002/mcp` - Returns capability information
- **CORS Preflight**: `OPTIONS http://localhost:3002/mcp` - Supports web clients

## Testing with Different Clients

### Claude Desktop

1. Add the server to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "toy-notes": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/path/to/solidmcp/examples/toy/Cargo.toml"
      ],
      "env": {
        "RUST_LOG": "info",
        "NOTES_DIR": "/path/to/your/notes"
      }
    }
  }
}
```

2. Restart Claude Desktop and the toy-notes server should be available.

### curl (HTTP JSON-RPC)

```bash
# Test transport discovery
curl -X GET http://localhost:3002/mcp

# Test tool call via HTTP
curl -X POST http://localhost:3002/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
  }'
```

### WebSocket Clients

Connect to `ws://localhost:3002/mcp` for real-time bidirectional communication.

## Example Usage

### Adding a Note

```json
{
  "tool": "add_note",
  "arguments": {
    "name": "my-first-note",
    "content": "This is a test note with **markdown** support!"
  }
}
```

### Listing Notes

```json
{
  "tool": "list_notes",
  "arguments": {}
}
```

### Reading a Note

```json
{
  "tool": "read_note",
  "arguments": {
    "name": "my-first-note"
  }
}
```

### Using Prompts

```json
{
  "prompt": "meeting_notes",
  "arguments": {
    "meeting_title": "Weekly Standup",
    "attendees": "Alice, Bob, Charlie"
  }
}
```

### Accessing Resources

Resources can be accessed via their URIs:

- List all resources: `resources/list`
- Read a specific note: `resources/read` with URI `note://my-first-note`

## Architecture

This example demonstrates the SolidMCP framework's key concepts:

### 1. **Context-Driven Design**

```rust
// Your application state - can be anything!
#[derive(Debug)]
struct NotesContext {
    notes_dir: PathBuf,
    notes: RwLock<HashMap<String, String>>,
}
```

### 2. **Type-Safe Tools with Auto Schema**

```rust
// Input/output types with automatic JSON schema generation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct AddNote {
    name: String,
    content: String,
}

// Simple closure-based tool definition
.with_tool(
    "add_note",
    "Add a new note",
    |input: AddNote, ctx: Arc<NotesContext>, notify| async move {
        ctx.save_note(&input.name, &input.content).await?;
        notify.info(&format!("Note '{}' added", input.name))?;
        Ok(NoteResult { message: "Success".to_string() })
    },
)
```

### 3. **Resource Providers**

```rust
impl ResourceProvider<NotesContext> for NotesResourceProvider {
    async fn list_resources(&self, context: Arc<NotesContext>) -> Result<Vec<ResourceInfo>> {
        // Expose your data as MCP resources
    }
}
```

### 4. **Prompt Providers**

```rust
impl PromptProvider<NotesContext> for NotesPromptProvider {
    async fn get_prompt(&self, name: &str, arguments: Option<Value>, context: Arc<NotesContext>) -> Result<PromptContent> {
        // Generate dynamic templates
    }
}
```

### 5. **Intelligent Transport Handling**

The framework automatically:

- Detects WebSocket upgrade requests and establishes WebSocket connections
- Handles HTTP JSON-RPC with proper session management
- Provides CORS headers for web-based clients
- Offers transport discovery endpoint for capability negotiation

The framework handles all protocol details, session management, transport layers, and error handling automatically.

## Testing

Comprehensive integration tests verify all MCP features:

```bash
# Test all functionality
cargo test --test integration

# Test with logging
RUST_LOG=debug cargo test --test integration

# Test specific features
cargo test test_tools
cargo test test_resources
cargo test test_prompts

# Test transport capabilities
cargo test test_transport_detection
```

## Performance Features

### Transport Optimizations

- **WebSocket**: Persistent connections for real-time communication
- **HTTP Session Management**: Efficient session cookies and state handling
- **CORS Caching**: Proper cache headers for preflight requests
- **Connection Reuse**: HTTP/1.1 keep-alive support

### Concurrency Support

- **Thread-Safe Context**: `Arc<RwLock<>>` for safe concurrent access
- **Async Operations**: Non-blocking file I/O and network operations
- **Session Isolation**: Each client maintains independent session state

## Latest Dependencies

The toy example uses the latest versions of all dependencies:

- **Tokio 1.43**: Latest async runtime for high performance
- **Warp 0.3.8**: Lightweight web framework with excellent performance
- **Serde 1.0.217**: JSON serialization with latest optimizations
- **tokio-tungstenite 0.27**: Latest WebSocket implementation
- **schemars 0.8.21**: JSON schema generation for type safety

## Key Benefits Demonstrated

1. **Minimal Boilerplate**: ~200 lines for a complete MCP server
2. **Type Safety**: Compile-time validation of tool inputs/outputs
3. **Auto Schema Generation**: No manual JSON schema writing
4. **Clean APIs**: Simple notification and error handling
5. **Full Protocol Support**: Tools, resources, and prompts in one server
6. **Smart Transport**: Automatic capability detection and fallback
7. **Production Ready**: Proper error handling, logging, and session management
8. **CORS Enabled**: Ready for web-based MCP clients
9. **Test Coverage**: Comprehensive integration tests for reliability
