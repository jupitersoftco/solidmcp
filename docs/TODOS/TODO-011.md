# TODO-011: Type Safety Enhancement - Replace String-Typed with Strong Types

**Status**: pending
**Priority**: medium
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-22
**Tags**: type-safety, stringly-typed, compile-time-validation, rust-best-practices
**Estimated Effort**: 4-5 days

## Description

The codebase relies heavily on stringly-typed patterns where strings are used to represent structured data that should have stronger type guarantees. This leads to runtime validation instead of compile-time safety, making the code more prone to errors and harder to refactor safely.

## Identified Stringly-Typed Issues

### 1. Method Names as Strings
```rust
// Current: Error-prone string matching
match request.method.as_str() {
    "initialize" => handle_initialize(params),
    "tools/list" => handle_tools_list(),
    "tools/call" => handle_tools_call(params),
    // Easy to mistype, no compile-time validation
}
```

### 2. Session IDs and Identifiers
```rust
// Current: Plain strings everywhere
fn get_session(session_id: &str) -> Option<Session>
fn create_session() -> String  // Could return any string
```

### 3. Tool and Resource URIs
```rust
// Current: String URIs with runtime parsing
fn call_tool(tool_name: &str, uri: &str) -> Result<Value, Error>
// No validation that URI is well-formed until runtime
```

### 4. Configuration Keys
```rust
// Current: Magic string configuration
config.get("transport.http.port").unwrap_or("3000")
config.get("session.timeout").unwrap_or("300")
// Typos in keys cause silent failures
```

### 5. Error Codes and Messages
```rust
// Current: Magic number error codes
JsonRpcError::new(-32600, "Invalid Request")
JsonRpcError::new(-32601, "Method not found")
// No type safety for standard error codes
```

## Impact Analysis

### Current Problems
- **Runtime Errors**: Type mismatches discovered at runtime
- **Refactoring Risk**: Renaming requires string replacements across codebase
- **Documentation Burden**: String formats must be documented separately
- **Testing Overhead**: Need extensive runtime validation tests
- **IDE Support**: No autocomplete or compile-time validation

### Error-Prone Patterns
```rust
// Easy to make typos
session_manager.get("sesion_123");  // Typo: "sesion" vs "session"

// Method names can be mistyped
handle_message("tools_list");  // Should be "tools/list"

// URI formats are unchecked
resource_provider.get("file:///invalid/path");  // Malformed URI
```

## Acceptance Criteria

- [ ] Replace method name strings with strongly-typed enum
- [ ] Create type-safe session ID wrapper
- [ ] Implement URI types with compile-time validation
- [ ] Create configuration key types with validation
- [ ] Implement strongly-typed error codes
- [ ] Add newtypes for all domain-specific string values
- [ ] Maintain backward compatibility during transition
- [ ] Ensure all type conversions are explicit and safe

## Technical Implementation

### Phase 1: Method Name Types
```rust
// src/protocol/method.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpMethod {
    Initialize,
    #[serde(rename = "tools/list")]
    ToolsList,
    #[serde(rename = "tools/call")]
    ToolsCall,
    #[serde(rename = "resources/list")]
    ResourcesList,
    #[serde(rename = "resources/read")]
    ResourcesRead,
    #[serde(rename = "prompts/list")]
    PromptsList,
    #[serde(rename = "prompts/get")]
    PromptsGet,
}

impl McpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            McpMethod::Initialize => "initialize",
            McpMethod::ToolsList => "tools/list",
            McpMethod::ToolsCall => "tools/call",
            McpMethod::ResourcesList => "resources/list",
            McpMethod::ResourcesRead => "resources/read",
            McpMethod::PromptsList => "prompts/list",
            McpMethod::PromptsGet => "prompts/get",
        }
    }
}

impl FromStr for McpMethod {
    type Err = InvalidMethodError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "initialize" => Ok(McpMethod::Initialize),
            "tools/list" => Ok(McpMethod::ToolsList),
            "tools/call" => Ok(McpMethod::ToolsCall),
            // ... rest of mappings
            _ => Err(InvalidMethodError::UnknownMethod(s.to_string())),
        }
    }
}
```

### Phase 2: Session ID Types
```rust
// src/session/id.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new() -> Self {
        use uuid::Uuid;
        SessionId(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Result<Self, InvalidSessionIdError> {
        if s.is_empty() || s.len() > 128 {
            return Err(InvalidSessionIdError::InvalidLength);
        }
        if !s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(InvalidSessionIdError::InvalidCharacters);
        }
        Ok(SessionId(s))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### Phase 3: URI Types
```rust
// src/types/uri.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceUri(url::Url);

impl ResourceUri {
    pub fn new(uri: &str) -> Result<Self, InvalidUriError> {
        let url = url::Url::parse(uri)
            .map_err(InvalidUriError::ParseError)?;
        
        // Validate allowed schemes
        match url.scheme() {
            "file" | "http" | "https" | "mcp" => Ok(ResourceUri(url)),
            scheme => Err(InvalidUriError::UnsupportedScheme(scheme.to_string())),
        }
    }
    
    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }
    
    pub fn path(&self) -> &str {
        self.0.path()
    }
    
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolName(String);

impl ToolName {
    pub fn new(name: &str) -> Result<Self, InvalidToolNameError> {
        if name.is_empty() {
            return Err(InvalidToolNameError::Empty);
        }
        if name.len() > 64 {
            return Err(InvalidToolNameError::TooLong);
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(InvalidToolNameError::InvalidCharacters);
        }
        Ok(ToolName(name.to_string()))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### Phase 4: Configuration Types
```rust
// src/config/keys.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigKey {
    TransportHttpPort,
    TransportHttpAddress,
    TransportWebSocketPort,
    SessionTimeout,
    SessionMaxCount,
    LogLevel,
}

impl ConfigKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigKey::TransportHttpPort => "transport.http.port",
            ConfigKey::TransportHttpAddress => "transport.http.address",
            ConfigKey::TransportWebSocketPort => "transport.websocket.port",
            ConfigKey::SessionTimeout => "session.timeout",
            ConfigKey::SessionMaxCount => "session.max_count",
            ConfigKey::LogLevel => "log.level",
        }
    }
    
    pub fn default_value(&self) -> &'static str {
        match self {
            ConfigKey::TransportHttpPort => "3000",
            ConfigKey::TransportHttpAddress => "127.0.0.1",
            ConfigKey::TransportWebSocketPort => "3001",
            ConfigKey::SessionTimeout => "3600",
            ConfigKey::SessionMaxCount => "1000",
            ConfigKey::LogLevel => "info",
        }
    }
}

pub struct TypedConfig {
    inner: HashMap<ConfigKey, String>,
}

impl TypedConfig {
    pub fn get_port(&self, key: ConfigKey) -> Result<u16, ConfigError> {
        let value = self.inner.get(&key).unwrap_or(&key.default_value().to_string());
        value.parse().map_err(|_| ConfigError::InvalidPort(key, value.clone()))
    }
    
    pub fn get_timeout(&self, key: ConfigKey) -> Result<Duration, ConfigError> {
        let value = self.inner.get(&key).unwrap_or(&key.default_value().to_string());
        let seconds: u64 = value.parse().map_err(|_| ConfigError::InvalidTimeout(key, value.clone()))?;
        Ok(Duration::from_secs(seconds))
    }
}
```

### Phase 5: Error Code Types
```rust
// src/errors/jsonrpc.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpcErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerError(i32), // -32000 to -32099
}

impl JsonRpcErrorCode {
    pub fn code(&self) -> i32 {
        match self {
            JsonRpcErrorCode::ParseError => -32700,
            JsonRpcErrorCode::InvalidRequest => -32600,
            JsonRpcErrorCode::MethodNotFound => -32601,
            JsonRpcErrorCode::InvalidParams => -32602,
            JsonRpcErrorCode::InternalError => -32603,
            JsonRpcErrorCode::ServerError(code) => *code,
        }
    }
    
    pub fn message(&self) -> &'static str {
        match self {
            JsonRpcErrorCode::ParseError => "Parse error",
            JsonRpcErrorCode::InvalidRequest => "Invalid Request",
            JsonRpcErrorCode::MethodNotFound => "Method not found",
            JsonRpcErrorCode::InvalidParams => "Invalid params",
            JsonRpcErrorCode::InternalError => "Internal error",
            JsonRpcErrorCode::ServerError(_) => "Server error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcError {
    pub fn from_code(code: JsonRpcErrorCode) -> Self {
        JsonRpcError {
            code: code.code(),
            message: code.message().to_string(),
            data: None,
        }
    }
    
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}
```

## Migration Strategy

### Gradual Migration Approach
1. **Phase 1**: Add new types alongside existing string usage
2. **Phase 2**: Create conversion functions between old and new types
3. **Phase 3**: Update internal APIs to use strong types
4. **Phase 4**: Update public APIs with backward compatibility
5. **Phase 5**: Remove old string-based APIs after deprecation period

### Backward Compatibility
```rust
// Maintain compatibility during transition
impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        SessionId::from_string(s.to_string()).unwrap_or_else(|_| {
            // Log warning about invalid session ID format
            warn!("Invalid session ID format, generating new ID");
            SessionId::new()
        })
    }
}

impl From<SessionId> for String {
    fn from(id: SessionId) -> Self {
        id.0
    }
}
```

## Testing Strategy

### Type Safety Tests
```rust
#[cfg(test)]
mod type_safety_tests {
    use super::*;
    
    #[test]
    fn test_method_parsing_compile_time_safety() {
        // These should compile
        let method = McpMethod::Initialize;
        assert_eq!(method.as_str(), "initialize");
        
        // This should be a compile error if we try to use a string directly
        // handle_method("invalid_method");  // Compile error: expected McpMethod
        
        // Runtime parsing should be explicit
        let parsed = McpMethod::from_str("tools/list").unwrap();
        assert_eq!(parsed, McpMethod::ToolsList);
    }
    
    #[test]
    fn test_session_id_validation() {
        // Valid session IDs
        assert!(SessionId::from_string("session_123".to_string()).is_ok());
        assert!(SessionId::from_string("abc-def-ghi".to_string()).is_ok());
        
        // Invalid session IDs
        assert!(SessionId::from_string("".to_string()).is_err());
        assert!(SessionId::from_string("session with spaces".to_string()).is_err());
        assert!(SessionId::from_string("a".repeat(129)).is_err());
    }
}
```

### Refactoring Safety Tests
```rust
// Test that refactoring is safe with strong types
#[test]
fn test_refactoring_safety() {
    let method = McpMethod::ToolsList;
    
    // If we rename the enum variant, this will be a compile error
    match method {
        McpMethod::ToolsList => {}, // Rename this and compiler will catch all usages
        _ => panic!("Unexpected method"),
    }
}
```

## Expected Benefits

### Compile-Time Safety
- **Type Errors**: Catch type mismatches at compile time
- **Refactoring Safety**: Compiler ensures all usages are updated
- **API Clarity**: Types document expected format and constraints
- **IDE Support**: Better autocomplete and error highlighting

### Runtime Reliability
- **Validation**: Input validation happens once at construction
- **Consistent Behavior**: Strong types prevent invalid states
- **Error Reduction**: Fewer runtime errors from string typos
- **Performance**: No repeated string parsing and validation

## Risk Assessment
- **Low Risk**: Strong types generally improve safety
- **Medium Impact**: Better type safety and developer experience
- **Medium Complexity**: Requires careful API design and migration

## Dependencies
- Related: TODO-003 (Security Vulnerabilities) - better input validation
- Enables: Better API design and testing
- Independent of architectural changes

## Progress Notes
- 2025-07-30: String type analysis completed, strong type design created