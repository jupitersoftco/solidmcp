# TODO-005: God Object Refactoring - Split McpProtocolEngine

**Status**: pending
**Priority**: high
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-13
**Tags**: god-object, architecture, refactoring, maintainability
**Estimated Effort**: 5-7 days

## Description

The `McpProtocolEngine` in `src/shared.rs` has grown into a "God Object" with too many responsibilities. It handles session management, message routing, protocol implementation, and transport abstraction all in one class. This violates the Single Responsibility Principle and makes the code difficult to maintain, test, and extend.

## Current Responsibilities Analysis

### McpProtocolEngine Current Duties
1. **Session Management** - Creating, storing, and managing protocol handlers
2. **Message Routing** - Routing messages to appropriate handlers
3. **Protocol Coordination** - Managing protocol handshakes and capabilities
4. **Transport Abstraction** - Handling different transport types
5. **Error Handling** - Managing protocol-level errors
6. **State Management** - Maintaining global protocol state

## Proposed Architecture

### New Component Structure
```
McpServer (Coordinator)
├── SessionManager (Session lifecycle)
├── MessageRouter (Message dispatch)
├── ProtocolOrchestrator (Protocol coordination)
├── TransportManager (Transport abstraction)
└── ErrorHandler (Centralized error management)
```

## Acceptance Criteria

- [ ] Extract `SessionManager` for session lifecycle management
- [ ] Create `MessageRouter` for message dispatch logic
- [ ] Implement `ProtocolOrchestrator` for protocol coordination
- [ ] Build `TransportManager` for transport abstraction
- [ ] Create `ErrorHandler` for centralized error management
- [ ] Maintain backward compatibility during transition
- [ ] Ensure all existing tests pass without modification
- [ ] Add comprehensive tests for each new component
- [ ] Update documentation to reflect new architecture

## Technical Implementation Plan

### Phase 1: Extract SessionManager
```rust
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    config: SessionConfig,
    cleanup_task: Option<JoinHandle<()>>,
}

impl SessionManager {
    pub fn create_session(&self, session_id: String) -> Result<(), SessionError>;
    pub fn get_session(&self, session_id: &str) -> Option<Session>;
    pub fn remove_session(&self, session_id: &str) -> bool;
    pub fn cleanup_expired_sessions(&self);
}
```

### Phase 2: Create MessageRouter
```rust
pub struct MessageRouter {
    routes: HashMap<String, Box<dyn MessageHandler>>,
    fallback_handler: Option<Box<dyn MessageHandler>>,
}

impl MessageRouter {
    pub fn route_message(&self, message: JsonRpcMessage, session: &Session) -> Result<JsonRpcResponse, RouterError>;
    pub fn register_handler(&mut self, method: String, handler: Box<dyn MessageHandler>);
}
```

### Phase 3: Build ProtocolOrchestrator
```rust
pub struct ProtocolOrchestrator {
    supported_versions: Vec<String>,
    capabilities: ServerCapabilities,
    initialization_state: HashMap<String, InitializationState>,
}

impl ProtocolOrchestrator {
    pub fn handle_initialize(&mut self, request: InitializeRequest, session_id: &str) -> Result<InitializeResponse, ProtocolError>;
    pub fn get_capabilities(&self) -> &ServerCapabilities;
    pub fn is_initialized(&self, session_id: &str) -> bool;
}
```

### Phase 4: Implement TransportManager
```rust
pub struct TransportManager {
    http_config: HttpConfig,
    websocket_config: WebSocketConfig,
    active_connections: HashMap<String, ConnectionInfo>,
}

impl TransportManager {
    pub fn handle_http_request(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;
    pub fn handle_websocket_message(&self, message: WebSocketMessage) -> Result<(), TransportError>;
    pub fn detect_transport_type(&self, headers: &HeaderMap) -> TransportType;
}
```

### Phase 5: Create Coordinating McpServer
```rust
pub struct McpServer {
    session_manager: SessionManager,
    message_router: MessageRouter,
    protocol_orchestrator: ProtocolOrchestrator,
    transport_manager: TransportManager,
    error_handler: ErrorHandler,
}
```

## Migration Strategy

### Phase-by-Phase Approach
1. **Week 2**: Extract SessionManager while maintaining existing interface
2. **Week 3**: Create MessageRouter and migrate routing logic
3. **Week 4**: Build ProtocolOrchestrator and TransportManager
4. **Week 5**: Create coordinating McpServer and remove old code

### Backward Compatibility
- Maintain existing public API during transition
- Use adapter pattern for legacy code integration
- Gradual migration with feature flags if needed

## Dependencies
- Requires: TODO-001 (Memory Leak Fix) to be completed first
- Enables: TODO-009 (Code Duplication Consolidation)
- Related: TODO-006 (Global Lock Replacement)

## Risk Assessment
- **High Impact**: Fundamental architecture change
- **High Complexity**: Requires careful design and testing
- **Medium Risk**: Large refactoring with many moving parts

## Testing Strategy
- Unit tests for each extracted component
- Integration tests for component interactions
- Regression tests to ensure no functionality breaks
- Performance tests to verify no degradation
- Gradual rollout with monitoring

## Expected Benefits
- **Maintainability**: Each component has clear responsibilities
- **Testability**: Smaller, focused components are easier to test
- **Extensibility**: New features can be added without modifying core logic
- **Performance**: Potential for better optimization and parallelization
- **Code Understanding**: Clearer structure for new developers

## Progress Notes
- 2025-07-30: Architecture design completed, ready for implementation planning