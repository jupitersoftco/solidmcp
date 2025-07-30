# TODO-010: Tight Coupling Reduction - Decouple Framework and Transport Layers

**Status**: pending
**Priority**: medium
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-22
**Tags**: coupling, architecture, modularity, separation-of-concerns
**Estimated Effort**: 5-6 days

## Description

The framework layer is tightly coupled to specific transport implementations (HTTP and WebSocket), making it difficult to add new transports, test components in isolation, and maintain clear separation of concerns. This tight coupling violates the dependency inversion principle and reduces system flexibility.

## Current Coupling Issues

### 1. Framework → Transport Dependencies
- Framework directly imports HTTP and WebSocket modules
- Framework contains transport-specific logic
- Hard-coded transport detection and selection
- Transport configuration embedded in framework configuration

### 2. Transport → Framework Dependencies
- Transports directly call framework methods
- Transport error handling depends on framework error types
- Shared mutable state between layers
- Transport lifecycle tied to framework lifecycle

### 3. Cross-Layer Concerns
- Session management spans both layers
- Error handling mixed between layers
- Configuration scattered across layers
- Testing requires both framework and transport components

## Impact Analysis

### Current Problems
- **Inflexibility**: Adding new transports requires framework changes
- **Testing Difficulty**: Cannot test framework without transport implementations
- **Code Reuse**: Framework logic cannot be reused with different transports
- **Maintenance Burden**: Changes in one layer require changes in others

### Dependency Graph (Current)
```
Framework Layer
    ├── Direct dependency on HTTP transport
    ├── Direct dependency on WebSocket transport
    ├── Shared session management
    └── Coupled error handling

HTTP Transport
    ├── Direct calls to Framework methods
    └── Framework-specific error types

WebSocket Transport
    ├── Direct calls to Framework methods
    └── Framework-specific error types
```

## Proposed Architecture

### Dependency Inversion with Traits
```
Framework Layer (Core Business Logic)
    ├── Transport Trait (Abstract Interface)
    └── Session Management (Abstracted)

Transport Implementations
    ├── HTTP Transport (Implements Transport Trait)
    ├── WebSocket Transport (Implements Transport Trait)
    └── Future Transports (TCP, gRPC, etc.)
```

## Acceptance Criteria

- [ ] Define abstract transport trait interface
- [ ] Decouple framework from specific transport implementations
- [ ] Create transport registry/factory pattern
- [ ] Implement dependency injection for transports
- [ ] Enable framework testing without transport implementations
- [ ] Allow transport testing without full framework
- [ ] Maintain backward compatibility for existing APIs
- [ ] Add comprehensive integration tests for decoupled architecture

## Technical Implementation

### Phase 1: Transport Abstraction
```rust
// src/transport/trait.rs
#[async_trait]
pub trait Transport: Send + Sync {
    type Config: Clone + Send + Sync;
    type Connection: Send + Sync;
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn start(&self, config: Self::Config) -> Result<(), Self::Error>;
    async fn stop(&self) -> Result<(), Self::Error>;
    async fn handle_connection(&self, connection: Self::Connection) -> Result<(), Self::Error>;
    
    fn name(&self) -> &'static str;
    fn supported_features(&self) -> TransportFeatures;
}

pub struct TransportFeatures {
    pub supports_streaming: bool,
    pub supports_progress: bool,
    pub supports_bidirectional: bool,
}
```

### Phase 2: Framework Abstraction
```rust
// src/framework/core.rs
pub struct McpServer<C> {
    transports: TransportRegistry,
    message_handler: Arc<dyn MessageHandler<Context = C>>,
    context: C,
}

impl<C> McpServer<C> {
    pub fn builder() -> McpServerBuilder<C> {
        McpServerBuilder::new()
    }
    
    pub fn register_transport<T: Transport + 'static>(&mut self, transport: T) -> Result<(), ServerError> {
        self.transports.register(Box::new(transport))
    }
    
    pub async fn start(&self) -> Result<(), ServerError> {
        for transport in self.transports.iter() {
            transport.start(transport.config()).await
                .map_err(|e| ServerError::TransportStart(transport.name(), e.into()))?;
        }
        Ok(())
    }
}
```

### Phase 3: Transport Registry
```rust
// src/transport/registry.rs
pub struct TransportRegistry {
    transports: HashMap<String, Box<dyn Transport>>,
    default_transport: Option<String>,
}

impl TransportRegistry {
    pub fn register(&mut self, transport: Box<dyn Transport>) -> Result<(), RegistryError> {
        let name = transport.name().to_string();
        if self.transports.contains_key(&name) {
            return Err(RegistryError::DuplicateTransport(name));
        }
        self.transports.insert(name, transport);
        Ok(())
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Transport> {
        self.transports.get(name).map(|t| t.as_ref())
    }
    
    pub fn detect_transport(&self, request_headers: &HeaderMap) -> Option<&dyn Transport> {
        // Transport detection logic moved here
        for transport in self.transports.values() {
            if transport.can_handle_request(request_headers) {
                return Some(transport.as_ref());
            }
        }
        None
    }
}
```

### Phase 4: HTTP Transport Implementation
```rust
// src/transport/http.rs
pub struct HttpTransport {
    config: HttpConfig,
    message_handler: Option<Arc<dyn MessageHandler>>,
}

#[async_trait]
impl Transport for HttpTransport {
    type Config = HttpConfig;
    type Connection = HttpConnection;
    type Error = HttpTransportError;
    
    async fn start(&self, config: Self::Config) -> Result<(), Self::Error> {
        // HTTP server startup logic
        let app = Router::new()
            .route("/mcp", post(self.handle_mcp_request))
            .with_state(self.clone());
        
        let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
    
    async fn handle_connection(&self, connection: Self::Connection) -> Result<(), Self::Error> {
        // Handle individual HTTP request
        let message = connection.extract_message()?;
        let response = self.message_handler
            .as_ref()
            .ok_or(HttpTransportError::NoHandler)?
            .handle_message(message)
            .await?;
        connection.send_response(response).await?;
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "http"
    }
    
    fn supported_features(&self) -> TransportFeatures {
        TransportFeatures {
            supports_streaming: false,
            supports_progress: true,
            supports_bidirectional: false,
        }
    }
}
```

### Phase 5: WebSocket Transport Implementation
```rust
// src/transport/websocket.rs
pub struct WebSocketTransport {
    config: WebSocketConfig,
    message_handler: Option<Arc<dyn MessageHandler>>,
}

#[async_trait]
impl Transport for WebSocketTransport {
    type Config = WebSocketConfig;
    type Connection = WebSocketConnection;
    type Error = WebSocketTransportError;
    
    async fn start(&self, config: Self::Config) -> Result<(), Self::Error> {
        // WebSocket server startup logic
        let app = Router::new()
            .route("/ws", get(websocket_handler))
            .with_state(self.clone());
        
        let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
    
    async fn handle_connection(&self, mut connection: Self::Connection) -> Result<(), Self::Error> {
        // Handle WebSocket connection lifecycle
        while let Some(message) = connection.receive().await? {
            let response = self.message_handler
                .as_ref()
                .ok_or(WebSocketTransportError::NoHandler)?
                .handle_message(message)
                .await?;
            connection.send(response).await?;
        }
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "websocket"
    }
    
    fn supported_features(&self) -> TransportFeatures {
        TransportFeatures {
            supports_streaming: true,
            supports_progress: true,
            supports_bidirectional: true,
        }
    }
}
```

## Implementation Strategy

### Phase 1 (Days 1-2): Define Abstractions
- Create transport trait and related types
- Define message handler abstraction
- Design transport registry interface

### Phase 2 (Days 2-3): Framework Decoupling
- Modify framework to use transport abstractions
- Remove direct transport dependencies
- Implement dependency injection

### Phase 3 (Days 3-4): Transport Implementations
- Refactor HTTP transport to implement trait
- Refactor WebSocket transport to implement trait
- Create transport registry implementation

### Phase 4 (Days 4-5): Integration and Testing
- Wire up all components with dependency injection
- Create comprehensive integration tests
- Performance testing to ensure no regressions

### Phase 5 (Days 5-6): Documentation and Examples
- Update documentation for new architecture
- Create examples showing how to add new transports
- Migration guide for existing code

## Testing Strategy

### Unit Testing (Decoupled)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockTransport;
    
    #[tokio::test]
    async fn test_framework_without_real_transport() {
        let mut server = McpServer::builder()
            .with_context(TestContext::new())
            .build();
        
        let mock_transport = MockTransport::new();
        server.register_transport(mock_transport).unwrap();
        
        // Test framework logic without real HTTP/WebSocket
        assert!(server.start().await.is_ok());
    }
}
```

### Integration Testing
- Test framework with real transports
- Test transport switching
- Test multiple concurrent transports

## Expected Benefits

### Architectural Improvements
- **Modularity**: Clear separation between framework and transport concerns
- **Extensibility**: Easy to add new transport types
- **Testability**: Can test components in isolation
- **Reusability**: Framework can be used with different transport combinations

### Development Benefits
- **Faster Testing**: Unit tests don't require transport setup
- **Parallel Development**: Framework and transport work can proceed independently
- **Easier Debugging**: Clear boundaries help isolate issues
- **Better Documentation**: Clear interfaces make system easier to understand

## Risk Assessment
- **Medium Risk**: Large architectural change requires careful coordination
- **High Impact**: Significant improvement in system flexibility and maintainability
- **High Complexity**: Requires understanding of dependency inversion patterns

## Dependencies
- Related: TODO-012 (Resource Leak Prevention)
- Enables: Better testing and future transport additions
- Independent of other architectural changes

## Future Transport Examples
With this architecture, adding new transports becomes straightforward:
- **TCP Transport**: Direct TCP socket communication
- **gRPC Transport**: Protocol Buffers over HTTP/2
- **Unix Socket Transport**: Local inter-process communication
- **Message Queue Transport**: Redis/RabbitMQ integration

## Progress Notes
- 2025-07-30: Coupling analysis completed, decoupling architecture designed