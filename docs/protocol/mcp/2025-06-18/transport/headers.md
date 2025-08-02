# HTTP Headers Reference

Critical headers for MCP over HTTP transport.

## Required Headers

### MCP-Protocol-Version
**NEW in 2025-06-18** - Required for version negotiation

```http
MCP-Protocol-Version: 2025-06-18
```

**Rules:**
- Client MUST send on ALL requests after initialization
- Value MUST match negotiated protocol version
- Missing header → server assumes `2025-03-26` (backwards compatibility)
- Invalid version → 400 Bad Request

**Example Flow:**
```mermaid
sequenceDiagram
    participant Client
    participant Server
    
    Client->>Server: POST /mcp (initialize)<br/>No version header needed
    Server-->>Client: 200 OK<br/>protocolVersion: "2025-06-18"
    
    Client->>Server: POST /mcp (any request)<br/>MCP-Protocol-Version: 2025-06-18
    Server-->>Client: 200 OK
    
    Client->>Server: POST /mcp<br/>MCP-Protocol-Version: 2024-11-05
    Server-->>Client: 400 Bad Request<br/>Unsupported version
```

### Content-Type
Standard HTTP content type for requests and responses

**Request (Client → Server):**
```http
Content-Type: application/json
```

**Response Types:**
```http
# JSON response
Content-Type: application/json

# SSE stream response  
Content-Type: text/event-stream
```

### Accept
Client capabilities for response format

```http
Accept: application/json, text/event-stream
```

**Required values:**
- `application/json` - Simple JSON responses
- `text/event-stream` - SSE streaming responses

## Session Headers

### Mcp-Session-Id
Maintains stateful sessions across requests

**Server provides in initialization:**
```http
HTTP/1.1 200 OK
Mcp-Session-Id: 1868a90c-7a7b-4545-ad16-4bec7e8a8b24
```

**Client includes in subsequent requests:**
```http
POST /mcp HTTP/1.1
Mcp-Session-Id: 1868a90c-7a7b-4545-ad16-4bec7e8a8b24
```

**Constraints:**
- Only visible ASCII characters (0x21-0x7E)
- Should be cryptographically secure
- Globally unique per server

## Security Headers

### Origin
CORS origin validation

```http
Origin: https://client.example.com
```

**Server MUST:**
- Validate against allowed origins
- Reject with 403 if invalid
- Prevent DNS rebinding attacks

### Authorization
Standard HTTP authentication

```http
# Bearer token
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...

# Basic auth
Authorization: Basic dXNlcjpwYXNzd29yZA==

# API key (custom scheme)
Authorization: ApiKey sk_live_abc123
```

## SSE Headers

### Last-Event-ID
Resume interrupted SSE streams

```http
GET /mcp HTTP/1.1
Last-Event-ID: 123
```

Server replays events after specified ID.

### Cache-Control
Prevent caching of event streams

```http
# Server response
Cache-Control: no-cache
```

## Standard HTTP Headers

### Host
Required HTTP/1.1 header

```http
Host: server.example.com
```

### Connection
Connection management

```http
# Keep connection alive
Connection: keep-alive

# Close after response
Connection: close

# Upgrade (WebSocket)
Connection: Upgrade
```

### Transfer-Encoding
For streaming responses

```http
Transfer-Encoding: chunked
```

Used when:
- Response size unknown
- Progress tracking enabled
- Streaming multiple messages

## Complete Request Example

```http
POST /mcp HTTP/1.1
Host: server.example.com
Content-Type: application/json
Accept: application/json, text/event-stream
MCP-Protocol-Version: 2025-06-18
Mcp-Session-Id: 1868a90c-7a7b-4545-ad16-4bec7e8a8b24
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
Origin: https://client.example.com
Content-Length: 123

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "calculator",
    "arguments": {"a": 5, "b": 3}
  }
}
```

## Complete Response Examples

### JSON Response
```http
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 89

{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{
      "type": "text",
      "text": "8"
    }]
  }
}
```

### SSE Stream Response
```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

event: message
data: {"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"calc-123","progress":50}}
id: 1

event: message
data: {"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"8"}]}}
id: 2
```

## Header Validation

### Client Requirements
- [ ] Send `MCP-Protocol-Version` on all requests after init
- [ ] Include both JSON and SSE in `Accept`
- [ ] Preserve `Mcp-Session-Id` if provided
- [ ] Set appropriate `Authorization` if required

### Server Requirements  
- [ ] Validate `MCP-Protocol-Version` matches negotiated
- [ ] Check `Origin` against allowed list
- [ ] Generate secure `Mcp-Session-Id` if using sessions
- [ ] Set correct `Content-Type` for response format

## Common Issues

### Missing Protocol Version
```http
HTTP/1.1 400 Bad Request
Content-Type: application/json

{
  "error": "Missing required header: MCP-Protocol-Version"
}
```

### Invalid Session
```http
HTTP/1.1 404 Not Found
Content-Type: application/json

{
  "error": "Session not found or expired"
}
```

### Wrong Accept Types
```http
HTTP/1.1 406 Not Acceptable
Content-Type: application/json

{
  "error": "Client must accept application/json and text/event-stream"
}
```