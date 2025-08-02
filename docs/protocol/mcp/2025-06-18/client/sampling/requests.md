# Sampling Request Format and Handling

**Protocol Revision**: 2025-06-18

Comprehensive reference for MCP sampling request formats, parameters, validation, and response handling.

## Request Structure

### Complete Request Schema
```typescript
interface CreateMessageRequest {
  jsonrpc: "2.0";
  id: RequestId;
  method: "sampling/createMessage";
  params: {
    messages: SamplingMessage[];
    modelPreferences?: ModelPreferences;
    systemPrompt?: string;
    includeContext?: "none" | "thisServer" | "allServers";
    temperature?: number;
    maxTokens: number;
    stopSequences?: string[];
    metadata?: object;
    _meta?: {
      progressToken?: ProgressToken;
      [key: string]: unknown;
    };
  };
}
```

### Required Fields
- `messages`: Array of conversation messages
- `maxTokens`: Maximum tokens to generate

### Optional Fields
- `modelPreferences`: Model selection hints and priorities
- `systemPrompt`: Instructions for the AI model
- `includeContext`: Include MCP server context
- `temperature`: Sampling randomness (0.0-1.0)
- `stopSequences`: Custom stop sequences
- `metadata`: Provider-specific metadata

## Message Formats

### SamplingMessage Structure
```typescript
interface SamplingMessage {
  role: "user" | "assistant";
  content: TextContent | ImageContent | AudioContent;
}
```

### Text Messages
```json
{
  "role": "user",
  "content": {
    "type": "text",
    "text": "Explain quantum computing in simple terms",
    "annotations": {
      "audience": ["user"],
      "priority": 0.8
    }
  }
}
```

### Image Messages
```json
{
  "role": "user", 
  "content": {
    "type": "image",
    "data": "iVBORw0KGgoAAAANSUhEUgAA...",
    "mimeType": "image/png",
    "annotations": {
      "audience": ["assistant"],
      "priority": 1.0
    }
  }
}
```

### Audio Messages
```json
{
  "role": "user",
  "content": {
    "type": "audio",
    "data": "UklGRiQEAABXQVZFZm10IBAA...",
    "mimeType": "audio/wav",
    "annotations": {
      "audience": ["assistant"],
      "priority": 0.9
    }
  }
}
```

### Multi-Turn Conversations
```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "What's the weather like?"
      }
    },
    {
      "role": "assistant", 
      "content": {
        "type": "text",
        "text": "I need more information. What location?"
      }
    },
    {
      "role": "user",
      "content": {
        "type": "text", 
        "text": "San Francisco, CA"
      }
    }
  ]
}
```

## Model Preferences

### Complete ModelPreferences Schema
```typescript
interface ModelPreferences {
  hints?: ModelHint[];
  costPriority?: number;     // 0.0-1.0
  speedPriority?: number;    // 0.0-1.0
  intelligencePriority?: number; // 0.0-1.0
}

interface ModelHint {
  name?: string;
  // Future: provider?, family?, capability?
}
```

### Priority Examples

#### Cost-Optimized Request
```json
{
  "modelPreferences": {
    "costPriority": 0.9,
    "speedPriority": 0.5,
    "intelligencePriority": 0.3,
    "hints": [
      { "name": "claude-3-haiku" },
      { "name": "gpt-3.5" },
      { "name": "gemini-flash" }
    ]
  }
}
```

#### Performance-Optimized Request
```json
{
  "modelPreferences": {
    "costPriority": 0.2,
    "speedPriority": 0.9,
    "intelligencePriority": 0.7,
    "hints": [
      { "name": "gpt-4-turbo" },
      { "name": "claude-3-sonnet" }
    ]
  }
}
```

#### Intelligence-Optimized Request
```json
{
  "modelPreferences": {
    "costPriority": 0.1,
    "speedPriority": 0.3,
    "intelligencePriority": 0.9,
    "hints": [
      { "name": "claude-3-opus" },
      { "name": "gpt-4" },
      { "name": "gemini-ultra" }
    ]
  }
}
```

### Hint Matching Logic
```typescript
function selectModel(preferences: ModelPreferences, availableModels: Model[]): Model {
  // 1. Process hints in order
  for (const hint of preferences.hints || []) {
    const matches = availableModels.filter(model => 
      model.name.includes(hint.name || '')
    );
    
    if (matches.length > 0) {
      // 2. Apply priorities to select best match
      return selectByPriorities(matches, preferences);
    }
  }
  
  // 3. Fallback to priority-only selection
  return selectByPriorities(availableModels, preferences);
}
```

## System Prompts

### Basic System Prompt
```json
{
  "systemPrompt": "You are a helpful coding assistant. Focus on clean, maintainable code."
}
```

### Role-Specific System Prompt
```json
{
  "systemPrompt": "You are a security expert analyzing code for vulnerabilities. Be thorough and specific in your analysis. Provide actionable recommendations."
}
```

### Context-Aware System Prompt
```json
{
  "systemPrompt": "You are working within a React TypeScript project. Consider modern React patterns and TypeScript best practices in your responses."
}
```

## Context Inclusion

### Context Options
- `"none"`: No additional context
- `"thisServer"`: Include context from requesting server only
- `"allServers"`: Include context from all connected MCP servers

### Example with Context
```json
{
  "includeContext": "allServers",
  "systemPrompt": "Use the available project context to provide accurate, contextually-aware responses.",
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "How should I refactor this component?"
      }
    }
  ]
}
```

## Advanced Parameters

### Temperature Control
```json
{
  "temperature": 0.7,  // 0.0 = deterministic, 1.0 = creative
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "Write a creative story about space exploration"
      }
    }
  ]
}
```

### Stop Sequences
```json
{
  "stopSequences": ["END", "---", "\n\n\n"],
  "messages": [
    {
      "role": "user", 
      "content": {
        "type": "text",
        "text": "Generate a code snippet. Stop at END."
      }
    }
  ]
}
```

### Provider Metadata
```json
{
  "metadata": {
    "anthropic": {
      "top_k": 40,
      "top_p": 0.9
    },
    "openai": {
      "frequency_penalty": 0.1,
      "presence_penalty": 0.1
    }
  }
}
```

## Response Format

### Complete Response Schema
```typescript
interface CreateMessageResult {
  role: "assistant";
  content: TextContent | ImageContent | AudioContent;
  model: string;
  stopReason?: "endTurn" | "stopSequence" | "maxTokens" | string;
  _meta?: { [key: string]: unknown };
}
```

### Successful Response Examples

#### Text Response
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "role": "assistant",
    "content": {
      "type": "text",
      "text": "Here's a clean implementation of the requested function:\n\n```python\ndef fibonacci(n):\n    if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)\n```",
      "annotations": {
        "audience": ["user"],
        "priority": 1.0
      }
    },
    "model": "claude-3-sonnet-20240307",
    "stopReason": "endTurn"
  }
}
```

#### Image Response
```json
{
  "jsonrpc": "2.0", 
  "id": 2,
  "result": {
    "role": "assistant",
    "content": {
      "type": "image",
      "data": "iVBORw0KGgoAAAANSUhEUgAA...",
      "mimeType": "image/png"
    },
    "model": "dall-e-3",
    "stopReason": "endTurn"
  }
}
```

### Stop Reasons
- `"endTurn"`: Model naturally ended response
- `"stopSequence"`: Hit custom stop sequence
- `"maxTokens"`: Reached token limit
- `"contentFilter"`: Content policy violation
- `"error"`: Generation error occurred

## Error Responses

### User Rejection
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -1,
    "message": "User rejected sampling request",
    "data": {
      "reason": "User declined AI interaction",
      "rejectionType": "explicit"
    }
  }
}
```

### Invalid Parameters
```json
{
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {
      "field": "maxTokens",
      "value": -1,
      "expected": "positive integer"
    }
  }
}
```

### Model Unavailable
```json
{
  "error": {
    "code": -32603,
    "message": "No suitable model available",
    "data": {
      "requestedHints": ["gpt-5", "claude-4"],
      "availableModels": ["gpt-4", "claude-3-sonnet"],
      "suggestion": "Try 'gpt-4' or 'claude-3-sonnet'"
    }
  }
}
```

### Rate Limiting
```json
{
  "error": {
    "code": -32000,
    "message": "Rate limit exceeded", 
    "data": {
      "retryAfter": 60,
      "remainingQuota": 0,
      "resetTime": "2024-01-15T10:30:00Z"
    }
  }
}
```

## Validation Rules

### Message Validation
```typescript
function validateMessages(messages: SamplingMessage[]): void {
  if (!messages || messages.length === 0) {
    throw new Error("At least one message required");
  }
  
  for (const message of messages) {
    if (!["user", "assistant"].includes(message.role)) {
      throw new Error(`Invalid role: ${message.role}`);
    }
    
    validateContent(message.content);
  }
}

function validateContent(content: ContentBlock): void {
  switch (content.type) {
    case "text":
      if (!content.text || content.text.trim().length === 0) {
        throw new Error("Text content cannot be empty");
      }
      break;
      
    case "image":
      if (!content.data || !content.mimeType) {
        throw new Error("Image content requires data and mimeType");
      }
      if (!content.mimeType.startsWith("image/")) {
        throw new Error("Invalid image MIME type");
      }
      break;
      
    case "audio":
      if (!content.data || !content.mimeType) {
        throw new Error("Audio content requires data and mimeType");
      }
      if (!content.mimeType.startsWith("audio/")) {
        throw new Error("Invalid audio MIME type");
      }
      break;
  }
}
```

### Parameter Validation
```typescript
function validateSamplingParams(params: CreateMessageParams): void {
  // Required fields
  if (!params.maxTokens || params.maxTokens <= 0) {
    throw new Error("maxTokens must be positive integer");
  }
  
  // Optional field validation
  if (params.temperature !== undefined) {
    if (params.temperature < 0 || params.temperature > 1) {
      throw new Error("temperature must be between 0.0 and 1.0");
    }
  }
  
  if (params.modelPreferences) {
    validateModelPreferences(params.modelPreferences);
  }
  
  validateMessages(params.messages);
}
```

## Implementation Patterns

### Client-Side Request Handling
```typescript
class SamplingService {
  async handleSamplingRequest(request: CreateMessageRequest): Promise<CreateMessageResult> {
    // 1. Validate request
    this.validateRequest(request);
    
    // 2. Present to user for approval
    const approval = await this.requestUserApproval(request);
    if (!approval.approved) {
      throw new McpError(-1, "User rejected sampling request");
    }
    
    // 3. Select model based on preferences
    const model = this.selectModel(request.params.modelPreferences);
    
    // 4. Send to LLM
    const response = await this.callLLM(model, approval.modifiedRequest);
    
    // 5. Show response to user
    const responseApproval = await this.reviewResponse(response);
    if (!responseApproval.approved) {
      throw new McpError(-1, "User rejected AI response");
    }
    
    return responseApproval.finalResponse;
  }
  
  private async requestUserApproval(request: CreateMessageRequest): Promise<UserApproval> {
    return this.userInterface.showSamplingDialog({
      messages: request.params.messages,
      systemPrompt: request.params.systemPrompt,
      modelPreferences: request.params.modelPreferences,
      allowEdit: true,
      showRisks: true
    });
  }
}
```

### Server-Side Request Creation
```typescript
class AgenticServer {
  async requestAnalysis(code: string): Promise<string> {
    const request: CreateMessageRequest = {
      jsonrpc: "2.0",
      id: this.generateId(),
      method: "sampling/createMessage",
      params: {
        messages: [{
          role: "user",
          content: {
            type: "text",
            text: `Analyze this code for security issues:\n\n${code}`
          }
        }],
        modelPreferences: {
          hints: [
            { name: "claude-3-sonnet" },
            { name: "gpt-4" }
          ],
          intelligencePriority: 0.9,
          speedPriority: 0.5,
          costPriority: 0.2
        },
        systemPrompt: "You are a security expert. Be thorough and specific.",
        maxTokens: 1000,
        temperature: 0.3
      }
    };
    
    try {
      const result = await this.client.request(request);
      return result.content.text;
    } catch (error) {
      return this.handleSamplingError(error);
    }
  }
}
```

## Best Practices

### Request Construction
1. **Clear Messages**: Write clear, specific prompts
2. **Appropriate Limits**: Set reasonable maxTokens
3. **Model Flexibility**: Provide multiple hint options
4. **Context Awareness**: Use system prompts effectively
5. **Error Handling**: Plan for user rejection

### Response Processing
1. **Validate Responses**: Check response format and content
2. **Handle Stop Reasons**: React appropriately to different stop reasons
3. **Error Recovery**: Implement graceful error handling
4. **User Feedback**: Show responses to users when appropriate

### Security Considerations
1. **Input Sanitization**: Validate all request parameters
2. **Output Filtering**: Apply content policies to responses
3. **Rate Limiting**: Prevent abuse and excessive usage
4. **Audit Logging**: Track sampling requests for security review
5. **User Consent**: Always involve users in decision-making

## Related Documentation

- [Sampling Overview](README.md) - High-level sampling concepts
- [Security Considerations](security.md) - Comprehensive security guidelines
- [Elicitation](../elicitation/) - User input collection
- [Core Protocol](../../core/) - MCP protocol foundations