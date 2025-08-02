# MCP 2025-06-18 Type Definitions

## Base Protocol Types

### JSON-RPC Core

```typescript
export type JSONRPCMessage = 
  | JSONRPCRequest 
  | JSONRPCNotification 
  | JSONRPCResponse 
  | JSONRPCError;

export const LATEST_PROTOCOL_VERSION = "2025-06-18";
export const JSONRPC_VERSION = "2.0";

export type ProgressToken = string | number;
export type Cursor = string;
export type RequestId = string | number;
```

### Request/Response Base

```typescript
interface Request {
  method: string;
  params?: {
    _meta?: {
      progressToken?: ProgressToken; // Optional progress token for out-of-band notifications
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
}

interface Notification {
  method: string;
  params?: {
    _meta?: { [key: string]: unknown };
    [key: string]: unknown;
  };
}

interface Result {
  _meta?: { [key: string]: unknown };
  [key: string]: unknown;
}
```

### JSON-RPC Messages

```typescript
interface JSONRPCRequest extends Request {
  jsonrpc: typeof JSONRPC_VERSION; // "2.0"
  id: RequestId; // Required for requests expecting responses
}

interface JSONRPCNotification extends Notification {
  jsonrpc: typeof JSONRPC_VERSION; // "2.0"
  // No id field - notifications don't expect responses
}

interface JSONRPCResponse {
  jsonrpc: typeof JSONRPC_VERSION; // "2.0"
  id: RequestId; // Must match request id
  result: Result;
}

interface JSONRPCError {
  jsonrpc: typeof JSONRPC_VERSION; // "2.0"
  id: RequestId;
  error: {
    code: number; // Standard error codes: -32700 to -32603
    message: string; // Concise single sentence
    data?: unknown; // Additional error information
  };
}
```

### Error Codes

```typescript
export const PARSE_ERROR = -32700;
export const INVALID_REQUEST = -32600;
export const METHOD_NOT_FOUND = -32601;
export const INVALID_PARAMS = -32602;
export const INTERNAL_ERROR = -32603;
```

## Base Metadata Types

```typescript
interface BaseMetadata {
  name: string; // Programmatic identifier
  title?: string; // Human-readable display name
}

interface Implementation extends BaseMetadata {
  version: string; // Required version string
}
```

## Content Types

### Content Blocks

```typescript
export type ContentBlock = 
  | TextContent 
  | ImageContent 
  | AudioContent 
  | ResourceLink 
  | EmbeddedResource;
```

### Text Content

```typescript
interface TextContent {
  type: "text";
  text: string; // Required text content
  annotations?: Annotations;
  _meta?: { [key: string]: unknown };
}
```

### Image Content

```typescript
interface ImageContent {
  type: "image";
  data: string; // Base64-encoded image data
  mimeType: string; // Required MIME type
  annotations?: Annotations;
  _meta?: { [key: string]: unknown };
}
```

### Audio Content

```typescript
interface AudioContent {
  type: "audio";
  data: string; // Base64-encoded audio data
  mimeType: string; // Required MIME type
  annotations?: Annotations;
  _meta?: { [key: string]: unknown };
}
```

### Resource Link

```typescript
interface ResourceLink extends Resource {
  type: "resource_link";
  // Inherits all Resource fields
}
```

### Embedded Resource

```typescript
interface EmbeddedResource {
  type: "resource";
  resource: TextResourceContents | BlobResourceContents;
  annotations?: Annotations;
  _meta?: { [key: string]: unknown };
}
```

## Annotations and Metadata

```typescript
interface Annotations {
  audience?: Role[]; // Intended customer: ["user", "assistant"]
  priority?: number; // Importance: 0 (optional) to 1 (required)
  lastModified?: string; // ISO 8601 timestamp (e.g., "2025-01-12T15:00:58Z")
}

export type Role = "user" | "assistant";
```

## Capabilities

### Client Capabilities

```typescript
interface ClientCapabilities {
  experimental?: { [key: string]: object };
  roots?: {
    listChanged?: boolean; // Supports root list change notifications
  };
  sampling?: object; // Supports LLM sampling
  elicitation?: object; // Supports server elicitation (NEW)
}
```

### Server Capabilities

```typescript
interface ServerCapabilities {
  experimental?: { [key: string]: object };
  logging?: object; // Can send log messages
  completions?: object; // Supports argument autocompletion
  prompts?: {
    listChanged?: boolean; // Supports prompt list change notifications
  };
  resources?: {
    subscribe?: boolean; // Supports resource update subscriptions
    listChanged?: boolean; // Supports resource list change notifications
  };
  tools?: {
    listChanged?: boolean; // Supports tool list change notifications
  };
}
```

## Initialization Types

### Initialize Request

```typescript
interface InitializeRequest extends Request {
  method: "initialize";
  params: {
    protocolVersion: string; // Latest supported version
    capabilities: ClientCapabilities;
    clientInfo: Implementation;
  };
}
```

### Initialize Result

```typescript
interface InitializeResult extends Result {
  protocolVersion: string; // Server's chosen version
  capabilities: ServerCapabilities;
  serverInfo: Implementation;
  instructions?: string; // Usage instructions for LLM
}
```

### Initialized Notification

```typescript
interface InitializedNotification extends Notification {
  method: "notifications/initialized";
  // No params required
}
```

## Resource System Types

### Resource Definition

```typescript
interface Resource extends BaseMetadata {
  uri: string; // Required URI
  description?: string; // Human-readable description
  mimeType?: string; // MIME type if known
  annotations?: Annotations;
  size?: number; // Raw content size in bytes
  _meta?: { [key: string]: unknown };
}
```

### Resource Template

```typescript
interface ResourceTemplate extends BaseMetadata {
  uriTemplate: string; // RFC 6570 URI template
  description?: string; // Template description
  mimeType?: string; // MIME type for all matching resources
  annotations?: Annotations;
  _meta?: { [key: string]: unknown };
}
```

### Resource Contents

```typescript
interface ResourceContents {
  uri: string; // Required URI
  mimeType?: string;
  _meta?: { [key: string]: unknown };
}

interface TextResourceContents extends ResourceContents {
  text: string; // Required text content
}

interface BlobResourceContents extends ResourceContents {
  blob: string; // Base64-encoded binary data
}
```

### Resource Requests

```typescript
interface ListResourcesRequest extends PaginatedRequest {
  method: "resources/list";
}

interface ListResourcesResult extends PaginatedResult {
  resources: Resource[];
}

interface ListResourceTemplatesRequest extends PaginatedRequest {
  method: "resources/templates/list";
}

interface ListResourceTemplatesResult extends PaginatedResult {
  resourceTemplates: ResourceTemplate[];
}

interface ReadResourceRequest extends Request {
  method: "resources/read";
  params: {
    uri: string; // Required resource URI
  };
}

interface ReadResourceResult extends Result {
  contents: (TextResourceContents | BlobResourceContents)[];
}
```

### Resource Subscription

```typescript
interface SubscribeRequest extends Request {
  method: "resources/subscribe";
  params: {
    uri: string; // Required resource URI
  };
}

interface UnsubscribeRequest extends Request {
  method: "resources/unsubscribe";
  params: {
    uri: string; // Required resource URI
  };
}
```

### Resource Notifications

```typescript
interface ResourceListChangedNotification extends Notification {
  method: "notifications/resources/list_changed";
}

interface ResourceUpdatedNotification extends Notification {
  method: "notifications/resources/updated";
  params: {
    uri: string; // Updated resource URI
  };
}
```

## Tool System Types

### Tool Definition

```typescript
interface Tool extends BaseMetadata {
  description?: string; // Human-readable description
  inputSchema: {
    type: "object";
    properties?: { [key: string]: object };
    required?: string[];
  };
  outputSchema?: {
    type: "object";
    properties?: { [key: string]: object };
    required?: string[];
  };
  annotations?: ToolAnnotations;
  _meta?: { [key: string]: unknown };
}
```

### Tool Annotations

```typescript
interface ToolAnnotations {
  title?: string; // Human-readable title
  readOnlyHint?: boolean; // Default: false
  destructiveHint?: boolean; // Default: true (when not read-only)
  idempotentHint?: boolean; // Default: false (when not read-only)
  openWorldHint?: boolean; // Default: true
}
```

### Tool Operations

```typescript
interface ListToolsRequest extends PaginatedRequest {
  method: "tools/list";
}

interface ListToolsResult extends PaginatedResult {
  tools: Tool[];
}

interface CallToolRequest extends Request {
  method: "tools/call";
  params: {
    name: string; // Required tool name
    arguments?: { [key: string]: unknown };
  };
}

interface CallToolResult extends Result {
  content: ContentBlock[]; // Required unstructured result
  structuredContent?: { [key: string]: unknown }; // Optional structured result
  isError?: boolean; // Default: false
}
```

### Tool Notifications

```typescript
interface ToolListChangedNotification extends Notification {
  method: "notifications/tools/list_changed";
}
```

## Prompt System Types

### Prompt Definition

```typescript
interface Prompt extends BaseMetadata {
  description?: string; // Prompt description
  arguments?: PromptArgument[];
  _meta?: { [key: string]: unknown };
}

interface PromptArgument extends BaseMetadata {
  description?: string; // Argument description
  required?: boolean; // Default: false
}
```

### Prompt Messages

```typescript
interface PromptMessage {
  role: Role; // "user" | "assistant"
  content: ContentBlock;
}
```

### Prompt Operations

```typescript
interface ListPromptsRequest extends PaginatedRequest {
  method: "prompts/list";
}

interface ListPromptsResult extends PaginatedResult {
  prompts: Prompt[];
}

interface GetPromptRequest extends Request {
  method: "prompts/get";
  params: {
    name: string; // Required prompt name
    arguments?: { [key: string]: string };
  };
}

interface GetPromptResult extends Result {
  description?: string;
  messages: PromptMessage[];
}
```

### Prompt Notifications

```typescript
interface PromptListChangedNotification extends Notification {
  method: "notifications/prompts/list_changed";
}
```

## Sampling Types

### Sampling Request

```typescript
interface CreateMessageRequest extends Request {
  method: "sampling/createMessage";
  params: {
    messages: SamplingMessage[]; // Required conversation history
    modelPreferences?: ModelPreferences;
    systemPrompt?: string;
    includeContext?: "none" | "thisServer" | "allServers";
    temperature?: number;
    maxTokens: number; // Required token limit
    stopSequences?: string[];
    metadata?: object; // Provider-specific metadata
  };
}
```

### Sampling Result

```typescript
interface CreateMessageResult extends Result, SamplingMessage {
  model: string; // Required model name
  stopReason?: "endTurn" | "stopSequence" | "maxTokens" | string;
}
```

### Sampling Message

```typescript
interface SamplingMessage {
  role: Role;
  content: TextContent | ImageContent | AudioContent;
}
```

### Model Preferences

```typescript
interface ModelPreferences {
  hints?: ModelHint[]; // Evaluated in order
  costPriority?: number; // 0-1 scale
  speedPriority?: number; // 0-1 scale
  intelligencePriority?: number; // 0-1 scale
}

interface ModelHint {
  name?: string; // Model name substring
}
```

## Elicitation Types (NEW)

### Elicit Request

```typescript
interface ElicitRequest extends Request {
  method: "elicitation/create";
  params: {
    message: string; // Required user message
    requestedSchema: {
      type: "object";
      properties: {
        [key: string]: PrimitiveSchemaDefinition;
      };
      required?: string[];
    };
  };
}
```

### Primitive Schema Definitions

```typescript
export type PrimitiveSchemaDefinition = 
  | StringSchema 
  | NumberSchema 
  | BooleanSchema 
  | EnumSchema;

interface StringSchema {
  type: "string";
  title?: string;
  description?: string;
  minLength?: number;
  maxLength?: number;
  format?: "email" | "uri" | "date" | "date-time";
}

interface NumberSchema {
  type: "number" | "integer";
  title?: string;
  description?: string;
  minimum?: number;
  maximum?: number;
}

interface BooleanSchema {
  type: "boolean";
  title?: string;
  description?: string;
  default?: boolean;
}

interface EnumSchema {
  type: "string";
  title?: string;
  description?: string;
  enum: string[]; // Required enum values
  enumNames?: string[]; // Optional display names
}
```

### Elicit Result

```typescript
interface ElicitResult extends Result {
  action: "accept" | "decline" | "cancel"; // Required user action
  content?: { [key: string]: string | number | boolean }; // Present when action is "accept"
}
```

## Logging Types

### Logging Levels

```typescript
export type LoggingLevel = 
  | "debug" 
  | "info" 
  | "notice" 
  | "warning" 
  | "error" 
  | "critical" 
  | "alert" 
  | "emergency";
```

### Logging Operations

```typescript
interface SetLevelRequest extends Request {
  method: "logging/setLevel";
  params: {
    level: LoggingLevel; // Required logging level
  };
}

interface LoggingMessageNotification extends Notification {
  method: "notifications/message";
  params: {
    level: LoggingLevel; // Required severity
    logger?: string; // Optional logger name
    data: unknown; // Required log data (any JSON serializable type)
  };
}
```

## Completion Types

### Completion Request

```typescript
interface CompleteRequest extends Request {
  method: "completion/complete";
  params: {
    ref: PromptReference | ResourceTemplateReference; // Required reference
    argument: {
      name: string; // Required argument name
      value: string; // Required current value
    };
    context?: {
      arguments?: { [key: string]: string }; // Previously resolved variables
    };
  };
}
```

### Completion Result

```typescript
interface CompleteResult extends Result {
  completion: {
    values: string[]; // Required completion values (max 100)
    total?: number; // Total available completions
    hasMore?: boolean; // More completions available
  };
}
```

### Reference Types

```typescript
interface ResourceTemplateReference {
  type: "ref/resource";
  uri: string; // Required URI or URI template
}

interface PromptReference extends BaseMetadata {
  type: "ref/prompt";
  // Inherits name and title from BaseMetadata
}
```

## Roots Types

### Root Definition

```typescript
interface Root {
  uri: string; // Required file:// URI
  name?: string; // Optional human-readable name
  _meta?: { [key: string]: unknown };
}
```

### Root Operations

```typescript
interface ListRootsRequest extends Request {
  method: "roots/list";
}

interface ListRootsResult extends Result {
  roots: Root[];
}
```

### Root Notifications

```typescript
interface RootsListChangedNotification extends Notification {
  method: "notifications/roots/list_changed";
}
```

## Pagination Support

```typescript
interface PaginatedRequest extends Request {
  params?: {
    cursor?: Cursor; // Opaque pagination token
  };
}

interface PaginatedResult extends Result {
  nextCursor?: Cursor; // Token for next page
}
```

## Utility Types

### Empty Result

```typescript
export type EmptyResult = Result; // Success with no data
```

### Progress Notification

```typescript
interface ProgressNotification extends Notification {
  method: "notifications/progress";
  params: {
    progressToken: ProgressToken; // Required token from original request
    progress: number; // Required current progress
    total?: number; // Optional total progress needed
    message?: string; // Optional progress description
  };
}
```

### Cancellation

```typescript
interface CancelledNotification extends Notification {
  method: "notifications/cancelled";
  params: {
    requestId: RequestId; // Required ID of request to cancel
    reason?: string; // Optional cancellation reason
  };
}
```

### Ping

```typescript
interface PingRequest extends Request {
  method: "ping";
  // No params required
}
```

## Message Union Types

### Client Messages

```typescript
export type ClientRequest = 
  | PingRequest
  | InitializeRequest
  | CompleteRequest
  | SetLevelRequest
  | GetPromptRequest
  | ListPromptsRequest
  | ListResourcesRequest
  | ListResourceTemplatesRequest
  | ReadResourceRequest
  | SubscribeRequest
  | UnsubscribeRequest
  | CallToolRequest
  | ListToolsRequest;

export type ClientNotification = 
  | CancelledNotification
  | ProgressNotification
  | InitializedNotification
  | RootsListChangedNotification;

export type ClientResult = 
  | EmptyResult
  | CreateMessageResult
  | ListRootsResult
  | ElicitResult;
```

### Server Messages

```typescript
export type ServerRequest = 
  | PingRequest
  | CreateMessageRequest
  | ListRootsRequest
  | ElicitRequest;

export type ServerNotification = 
  | CancelledNotification
  | ProgressNotification
  | LoggingMessageNotification
  | ResourceUpdatedNotification
  | ResourceListChangedNotification
  | ToolListChangedNotification
  | PromptListChangedNotification;

export type ServerResult = 
  | EmptyResult
  | InitializeResult
  | CompleteResult
  | GetPromptResult
  | ListPromptsResult
  | ListResourceTemplatesResult
  | ListResourcesResult
  | ReadResourceResult
  | CallToolResult
  | ListToolsResult;
```