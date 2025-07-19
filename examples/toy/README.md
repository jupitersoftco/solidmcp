# Toy Notes Server - MCP Protocol Demonstration

A comprehensive example MCP server that demonstrates all protocol features through a simple note-taking application.

> **Note**: This example currently uses the built-in tools (echo and read_file) while the high-level API is being integrated. The full note-taking functionality with resources and prompts will be available once the integration is complete.

## Features Demonstrated

### 🛠️ Tools
- **create_note** - Create a new note with title, content, and tags
- **list_notes** - List all notes with optional tag filtering
- **read_note** - Read a specific note by ID
- **delete_note** - Delete a note by ID

### 📚 Resources
- Notes are exposed as resources with URIs like `note://uuid`
- Each note can be read as a markdown resource
- Resources include metadata like creation date

### 📝 Prompts
- **meeting_notes** - Template for meeting notes with customizable title and attendees
- **daily_journal** - Template for daily journal entries
- **todo_list** - Template for TODO lists with optional project name

### 🔔 Notifications (TODO)
- File change notifications when notes are created/updated/deleted

## Running the Server

```bash
# From the solidmcp root directory
cd examples/toy
cargo run
```

The server will start on port 3000 by default. You can customize this with the `PORT` environment variable:

```bash
PORT=8080 cargo run
```

Notes are stored in a `notes` directory by default. You can customize this with the `NOTES_DIR` environment variable:

```bash
NOTES_DIR=/path/to/notes cargo run
```

## Testing with Claude Desktop

1. Add the server to your Claude Desktop configuration:

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

### Creating a Note
```json
{
  "tool": "create_note",
  "arguments": {
    "title": "My First Note",
    "content": "This is a test note with **markdown** support!",
    "tags": ["test", "example"]
  }
}
```

### Listing Notes
```json
{
  "tool": "list_notes",
  "arguments": {
    "tag": "test"
  }
}
```

### Reading a Note
```json
{
  "tool": "read_note",
  "arguments": {
    "id": "uuid-from-list"
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
- Read a specific note: `resources/read` with URI `note://uuid`

## Architecture

This example demonstrates how to build an MCP server using the high-level API from solidmcp:

1. **Tools** - Implement the `McpTool` trait for each tool
2. **Resources** - Implement the `McpResourceProvider` trait
3. **Prompts** - Implement the `McpPromptProvider` trait
4. **Server** - Use `McpServerBuilder` to combine everything

The server handles all the protocol details, letting you focus on implementing your domain logic.

## Testing

Integration tests are provided to verify all MCP features work correctly:

```bash
cargo test --test integration
```

## File Format

Notes are stored as markdown files with metadata in the header:

```markdown
# Note Title

Tags: tag1, tag2

Created: 2024-01-01 12:00:00
Updated: 2024-01-01 12:00:00

---

Note content goes here...
```