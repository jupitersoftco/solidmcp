# SolidMCP Integration Guides

This document provides detailed integration guides for connecting various MCP clients to your SolidMCP server.

## Table of Contents

1. [Claude Desktop Integration](#claude-desktop-integration)
2. [Cursor Integration](#cursor-integration)
3. [Custom TypeScript/JavaScript Client](#custom-typescriptjavascript-client)
4. [Custom Python Client](#custom-python-client)
5. [Custom Rust Client](#custom-rust-client)
6. [Testing with curl](#testing-with-curl)
7. [Integration Best Practices](#integration-best-practices)

## Claude Desktop Integration

Claude Desktop supports MCP servers for extending its capabilities with custom tools.

### Configuration

1. **Create MCP server configuration:**

   Create or edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

   ```json
   {
     "mcpServers": {
       "my-solidmcp-server": {
         "command": "cargo",
         "args": ["run", "--release"],
         "cwd": "/path/to/your/solidmcp/project"
       }
     }
   }
   ```

2. **Alternative: Use compiled binary:**

   ```json
   {
     "mcpServers": {
       "my-solidmcp-server": {
         "command": "/path/to/your/server/binary",
         "args": ["--port", "3000"]
       }
     }
   }
   ```

3. **With environment variables:**

   ```json
   {
     "mcpServers": {
       "my-solidmcp-server": {
         "command": "cargo",
         "args": ["run"],
         "cwd": "/path/to/project",
         "env": {
           "DATABASE_URL": "postgresql://localhost/mydb",
           "API_KEY": "your-api-key"
         }
       }
     }
   }
   ```

### Troubleshooting Claude Desktop

- Check logs: `~/Library/Logs/Claude/mcp.log`
- Ensure server starts successfully: `cargo run` in terminal first
- Verify tools appear in Claude's interface after restart

## Cursor Integration

Cursor IDE supports MCP for code-aware AI assistance.

### Setup

1. **Install the MCP extension** (if available) or configure manually

2. **Add server configuration:**

   In Cursor settings, add:

   ```json
   {
     "mcp.servers": {
       "solidmcp": {
         "uri": "ws://localhost:3000/mcp",
         "name": "My SolidMCP Server"
       }
     }
   }
   ```

3. **Start your server:**

   ```bash
   cargo run --bin your-server
   ```

### Cursor-Specific Considerations

- Cursor may reconnect frequently - SolidMCP handles re-initialization
- WebSocket transport is preferred for real-time updates
- Session state is maintained per connection

## Custom TypeScript/JavaScript Client

### Using the official MCP SDK

```typescript
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { WebSocketTransport } from "@modelcontextprotocol/sdk/client/websocket.js";

// For WebSocket connection
async function connectWebSocket() {
  const transport = new WebSocketTransport(
    new WebSocket("ws://localhost:3000/mcp")
  );
  
  const client = new Client({
    name: "my-client",
    version: "1.0.0",
  }, {
    capabilities: {}
  });
  
  await client.connect(transport);
  
  // Initialize
  const initResult = await client.initialize();
  console.log("Server capabilities:", initResult.capabilities);
  
  // List tools
  const tools = await client.listTools();
  console.log("Available tools:", tools);
  
  // Call a tool
  const result = await client.callTool("my_tool", {
    param1: "value1"
  });
  console.log("Tool result:", result);
  
  return client;
}

// For HTTP connection (with axios)
import axios from 'axios';

class HttpMcpClient {
  private sessionCookie?: string;
  private requestId = 0;
  
  constructor(private baseUrl: string) {}
  
  async request(method: string, params?: any) {
    const response = await axios.post(
      `${this.baseUrl}/mcp`,
      {
        jsonrpc: "2.0",
        method,
        params: params || {},
        id: ++this.requestId
      },
      {
        headers: {
          'Content-Type': 'application/json',
          'Cookie': this.sessionCookie || ''
        },
        withCredentials: true
      }
    );
    
    // Save session cookie
    const setCookie = response.headers['set-cookie'];
    if (setCookie) {
      this.sessionCookie = setCookie[0];
    }
    
    if (response.data.error) {
      throw new Error(response.data.error.message);
    }
    
    return response.data.result;
  }
  
  async initialize() {
    return this.request('initialize', {
      protocolVersion: '2025-06-18',
      capabilities: {}
    });
  }
  
  async listTools() {
    const result = await this.request('tools/list');
    return result.tools;
  }
  
  async callTool(name: string, arguments: any) {
    return this.request('tools/call', { name, arguments });
  }
}

// Usage
const httpClient = new HttpMcpClient('http://localhost:3000');
await httpClient.initialize();
const tools = await httpClient.listTools();
```

### Handling Notifications

```typescript
// WebSocket client with notification handling
client.on('notification', (notification) => {
  switch (notification.method) {
    case 'log':
      console.log(`[${notification.params.level}] ${notification.params.message}`);
      break;
    case 'progress':
      console.log(`Progress: ${notification.params.progress}/${notification.params.total}`);
      break;
    case 'resources/list_changed':
      console.log('Resources changed, refreshing...');
      break;
  }
});
```

## Custom Python Client

### Using httpx (recommended)

```python
import httpx
import json
from typing import Any, Dict, Optional

class SolidMcpClient:
    def __init__(self, base_url: str = "http://localhost:3000"):
        self.base_url = base_url
        self.client = httpx.Client()
        self.request_id = 0
        
    def _request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Any:
        self.request_id += 1
        
        response = self.client.post(
            f"{self.base_url}/mcp",
            json={
                "jsonrpc": "2.0",
                "method": method,
                "params": params or {},
                "id": self.request_id
            },
            headers={"Content-Type": "application/json"}
        )
        
        data = response.json()
        if "error" in data:
            raise Exception(f"MCP Error: {data['error']['message']}")
            
        return data.get("result")
    
    def initialize(self, protocol_version: str = "2025-06-18") -> Dict[str, Any]:
        """Initialize connection with the MCP server"""
        return self._request("initialize", {
            "protocolVersion": protocol_version,
            "capabilities": {}
        })
    
    def list_tools(self) -> list:
        """List available tools"""
        result = self._request("tools/list")
        return result.get("tools", [])
    
    def call_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool with arguments"""
        return self._request("tools/call", {
            "name": name,
            "arguments": arguments
        })
    
    def list_resources(self) -> list:
        """List available resources"""
        result = self._request("resources/list")
        return result.get("resources", [])
    
    def read_resource(self, uri: str) -> Dict[str, Any]:
        """Read a resource by URI"""
        return self._request("resources/read", {"uri": uri})

# Usage example
async def main():
    client = SolidMcpClient()
    
    # Initialize
    init_result = client.initialize()
    print(f"Server: {init_result['serverInfo']}")
    
    # List and call tools
    tools = client.list_tools()
    for tool in tools:
        print(f"Tool: {tool['name']} - {tool['description']}")
    
    # Call a tool
    result = client.call_tool("search", {
        "query": "test",
        "limit": 10
    })
    print(f"Search results: {result}")

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
```

### WebSocket Client with websockets

```python
import asyncio
import json
import websockets
from typing import Any, Dict, Optional

class SolidMcpWebSocketClient:
    def __init__(self, uri: str = "ws://localhost:3000/mcp"):
        self.uri = uri
        self.websocket = None
        self.request_id = 0
        
    async def connect(self):
        """Connect to the WebSocket server"""
        self.websocket = await websockets.connect(self.uri)
        
    async def _request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Any:
        if not self.websocket:
            raise Exception("Not connected")
            
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": self.request_id
        }
        
        await self.websocket.send(json.dumps(request))
        
        # Wait for response
        while True:
            message = await self.websocket.recv()
            data = json.loads(message)
            
            # Handle notifications
            if "method" in data and not "id" in data:
                await self._handle_notification(data)
                continue
                
            # Check if this is our response
            if data.get("id") == self.request_id:
                if "error" in data:
                    raise Exception(f"MCP Error: {data['error']['message']}")
                return data.get("result")
    
    async def _handle_notification(self, notification: Dict[str, Any]):
        """Handle incoming notifications"""
        method = notification.get("method")
        params = notification.get("params", {})
        
        if method == "log":
            print(f"[{params.get('level')}] {params.get('message')}")
        elif method == "progress":
            print(f"Progress: {params.get('progress')}/{params.get('total')}")
            
    async def initialize(self) -> Dict[str, Any]:
        return await self._request("initialize", {
            "protocolVersion": "2025-06-18",
            "capabilities": {}
        })
        
    async def call_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        return await self._request("tools/call", {
            "name": name,
            "arguments": arguments
        })
        
    async def close(self):
        if self.websocket:
            await self.websocket.close()

# Usage
async def main():
    client = SolidMcpWebSocketClient()
    await client.connect()
    
    try:
        await client.initialize()
        result = await client.call_tool("my_tool", {"param": "value"})
        print(result)
    finally:
        await client.close()
```

## Custom Rust Client

### Using reqwest for HTTP

```rust
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use anyhow::Result;

pub struct SolidMcpHttpClient {
    client: Client,
    base_url: String,
    request_id: std::sync::atomic::AtomicU64,
}

impl SolidMcpHttpClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .build()?;
            
        Ok(Self {
            client,
            base_url: base_url.to_string(),
            request_id: std::sync::atomic::AtomicU64::new(0),
        })
    }
    
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(json!({})),
            "id": id
        });
        
        let response = self.client
            .post(format!("{}/mcp", self.base_url))
            .json(&request)
            .send()
            .await?;
            
        let data: Value = response.json().await?;
        
        if let Some(error) = data.get("error") {
            return Err(anyhow::anyhow!("MCP Error: {}", error["message"]));
        }
        
        Ok(data["result"].clone())
    }
    
    pub async fn initialize(&self) -> Result<Value> {
        self.request("initialize", Some(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {}
        }))).await
    }
    
    pub async fn list_tools(&self) -> Result<Vec<Value>> {
        let result = self.request("tools/list", None).await?;
        Ok(result["tools"].as_array().unwrap_or(&vec![]).clone())
    }
    
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        self.request("tools/call", Some(json!({
            "name": name,
            "arguments": arguments
        }))).await
    }
}

// Usage
#[tokio::main]
async fn main() -> Result<()> {
    let client = SolidMcpHttpClient::new("http://localhost:3000")?;
    
    // Initialize
    let init = client.initialize().await?;
    println!("Server info: {:?}", init["serverInfo"]);
    
    // List tools
    let tools = client.list_tools().await?;
    for tool in tools {
        println!("Tool: {} - {}", tool["name"], tool["description"]);
    }
    
    // Call tool
    let result = client.call_tool("search", json!({
        "query": "test"
    })).await?;
    println!("Result: {:?}", result);
    
    Ok(())
}
```

### Using tokio-tungstenite for WebSocket

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use anyhow::Result;

pub struct SolidMcpWsClient {
    request_id: std::sync::atomic::AtomicU64,
}

impl SolidMcpWsClient {
    pub fn new() -> Self {
        Self {
            request_id: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    pub async fn connect(&self, url: &str) -> Result<()> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Send initialize
        let id = self.request_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let init_request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {}
            },
            "id": id
        });
        
        write.send(Message::Text(init_request.to_string())).await?;
        
        // Handle messages
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    let data: Value = serde_json::from_str(&text)?;
                    
                    // Handle response or notification
                    if data.get("id").is_some() {
                        println!("Response: {}", data);
                    } else if data.get("method").is_some() {
                        println!("Notification: {}", data);
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}
```

## Testing with curl

### Quick Testing Commands

```bash
# Initialize
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18"},"id":1}'

# List tools (save cookies for session)
curl -c cookies.txt -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}'

# Call tool (use cookies)
curl -b cookies.txt -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"my_tool","arguments":{"param":"value"}},"id":3}'

# Test SSE streaming
curl -N -H "Accept: text/event-stream" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"streaming_tool","arguments":{}},"id":4}' \
  http://localhost:3000/mcp
```

### WebSocket Testing with wscat

```bash
# Install wscat
npm install -g wscat

# Connect
wscat -c ws://localhost:3000/mcp

# Send initialize
{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18"},"id":1}

# List tools
{"jsonrpc":"2.0","method":"tools/list","id":2}
```

## Integration Best Practices

### 1. Connection Management

- **Implement reconnection logic** for WebSocket clients
- **Preserve session cookies** for HTTP clients
- **Handle connection errors** gracefully
- **Set appropriate timeouts** for long-running operations

### 2. Error Handling

```typescript
// Comprehensive error handling
try {
  const result = await client.callTool("my_tool", args);
} catch (error) {
  if (error.code === -32601) {
    console.error("Tool not found");
  } else if (error.code === -32602) {
    console.error("Invalid parameters:", error.data);
  } else {
    console.error("Unexpected error:", error);
  }
}
```

### 3. Progress Tracking

```python
# Handle progress notifications
async def handle_notifications(client):
    async for notification in client.notifications():
        if notification["method"] == "progress":
            progress = notification["params"]["progress"]
            total = notification["params"].get("total", 100)
            print(f"Progress: {progress}/{total}")
```

### 4. Resource Caching

```javascript
class CachedMcpClient extends McpClient {
  constructor() {
    super();
    this.resourceCache = new Map();
  }
  
  async readResource(uri) {
    if (this.resourceCache.has(uri)) {
      return this.resourceCache.get(uri);
    }
    
    const content = await super.readResource(uri);
    this.resourceCache.set(uri, content);
    return content;
  }
  
  clearCache() {
    this.resourceCache.clear();
  }
}
```

### 5. Type Safety

For TypeScript/Rust clients, generate types from your tool schemas:

```typescript
// Generated types from JSON Schema
interface SearchInput {
  query: string;
  limit?: number;
}

interface SearchOutput {
  results: string[];
  total: number;
}

// Type-safe client wrapper
class TypedMcpClient {
  async search(input: SearchInput): Promise<SearchOutput> {
    return this.callTool("search", input);
  }
}
```

### 6. Testing Your Integration

1. **Unit test your client:**
   ```python
   def test_initialize():
       client = MockMcpClient()
       result = client.initialize()
       assert result["protocolVersion"] == "2025-06-18"
   ```

2. **Integration test with real server:**
   ```javascript
   describe('MCP Integration', () => {
     let server;
     let client;
     
     beforeAll(async () => {
       server = await startTestServer();
       client = new McpClient('http://localhost:3001');
     });
     
     test('can list tools', async () => {
       await client.initialize();
       const tools = await client.listTools();
       expect(tools).toHaveLength(3);
     });
   });
   ```

3. **Load test for performance:**
   ```bash
   # Using Apache Bench
   ab -n 1000 -c 10 -p request.json -T application/json http://localhost:3000/mcp
   ```

### 7. Security Considerations

- **Use HTTPS in production**
- **Implement authentication** if needed
- **Validate all inputs** on both client and server
- **Set CORS headers** appropriately
- **Rate limit** requests if exposed publicly

Remember to consult the official MCP specification and SolidMCP documentation for the most up-to-date integration details.