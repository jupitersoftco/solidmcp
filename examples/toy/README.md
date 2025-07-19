# Toy Notes Server - Complete SolidMCP Example

A comprehensive example demonstrating all MCP features using the SolidMCP framework. This example shows how to build a real-world MCP server with minimal boilerplate while supporting the complete MCP specification.

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

## Testing with Claude Desktop

1. Add the server to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "toy-notes": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "/path/to/solidmcp/examples/toy/Cargo.toml"],
      "env": {
        "RUST_LOG": "info",
        "NOTES_DIR": "/path/to/your/notes"
      }
    }
  }
}
```

2. Restart Claude Desktop and the toy-notes server should be available.

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
```

## Key Benefits Demonstrated

1. **Minimal Boilerplate**: ~200 lines for a complete MCP server
2. **Type Safety**: Compile-time validation of tool inputs/outputs
3. **Auto Schema Generation**: No manual JSON schema writing
4. **Clean APIs**: Simple notification and error handling
5. **Full Protocol Support**: Tools, resources, and prompts in one server
6. **Production Ready**: Proper error handling, logging, and session management