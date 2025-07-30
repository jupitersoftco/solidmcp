# TODO-009: Code Duplication Consolidation

**Status**: pending
**Priority**: medium
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-20
**Tags**: code-duplication, refactoring, maintainability, dry-principle
**Estimated Effort**: 4-5 days

## Description

The codebase contains significant code duplication in error handling, session logic, and validation patterns. This violates the DRY (Don't Repeat Yourself) principle and makes maintenance difficult as changes need to be applied in multiple places, increasing the risk of inconsistencies and bugs.

## Identified Duplication Patterns

### 1. Error Handling Duplication
- Similar error handling patterns repeated across multiple files
- Repeated error conversion and logging code
- Inconsistent error message formatting
- Duplicate error type definitions

### 2. Session Logic Duplication
- Session validation repeated in HTTP and WebSocket handlers
- Session creation logic duplicated
- Session cleanup patterns repeated
- Similar session state management

### 3. Validation Duplication
- JSON-RPC message validation repeated
- Parameter validation patterns duplicated
- Similar type checking and conversion logic
- Repeated schema validation code

### 4. Response Generation Duplication
- Similar response formatting across different endpoints
- Repeated progress notification handling
- Duplicate success/error response creation
- Similar serialization patterns

## Impact Analysis

### Current State
- **Estimated Duplication**: ~30% of codebase has duplicated patterns
- **Maintenance Burden**: Changes require updates in 3-5 places
- **Bug Risk**: Inconsistencies between similar implementations
- **Development Velocity**: Slower feature development due to repetition

### Files Affected
- `src/http.rs` and `src/websocket.rs` - Transport handling duplication
- `src/protocol_impl.rs` and `src/handler.rs` - Protocol handling duplication
- `src/shared.rs` and `src/framework.rs` - Session management duplication
- Multiple test files with similar setup patterns

## Acceptance Criteria

- [ ] Create centralized error handling module
- [ ] Consolidate session management logic
- [ ] Create reusable validation utilities
- [ ] Implement shared response generation utilities
- [ ] Reduce code duplication by at least 60%
- [ ] Maintain or improve test coverage
- [ ] Ensure all existing functionality is preserved
- [ ] Add comprehensive tests for consolidated utilities

## Technical Implementation Plan

### Phase 1: Error Handling Consolidation
```rust
// src/errors/mod.rs
pub mod protocol_errors;
pub mod transport_errors;
pub mod session_errors;

pub trait ErrorContext {
    fn with_context(self, context: &str) -> Self;
    fn to_jsonrpc_error(&self) -> JsonRpcError;
}

pub struct ErrorHandler {
    logger: Logger,
}

impl ErrorHandler {
    pub fn handle_and_log<E: ErrorContext>(&self, error: E, operation: &str) -> JsonRpcError {
        let context = format!("Operation: {}", operation);
        let contextualized = error.with_context(&context);
        self.logger.error(&contextualized);
        contextualized.to_jsonrpc_error()
    }
}
```

### Phase 2: Session Logic Consolidation
```rust
// src/session/manager.rs
pub struct SessionManager {
    storage: Arc<DashMap<String, SessionHandle>>,
    validator: SessionValidator,
    cleaner: SessionCleaner,
}

pub trait SessionOperations {
    fn create_session(&self, request: &HttpRequest) -> Result<Session, SessionError>;
    fn validate_session(&self, session_id: &str) -> Result<&Session, SessionError>;
    fn cleanup_session(&self, session_id: &str) -> Result<(), SessionError>;
}

// Usage in both HTTP and WebSocket
impl HttpHandler {
    fn handle_request(&self, request: HttpRequest) -> Result<HttpResponse, TransportError> {
        let session = self.session_manager.validate_session(&request.session_id())?;
        // ... rest of handling
    }
}
```

### Phase 3: Validation Utilities Consolidation
```rust
// src/validation/mod.rs
pub struct MessageValidator {
    schema_validator: SchemaValidator,
    size_limits: SizeLimits,
}

pub trait Validate<T> {
    type Error;
    fn validate(&self, input: &T) -> Result<(), Self::Error>;
}

impl MessageValidator {
    pub fn validate_jsonrpc_message(&self, message: &str) -> Result<JsonRpcMessage, ValidationError> {
        self.validate_size(message)?;
        let parsed = self.parse_message(message)?;
        self.validate_schema(&parsed)?;
        Ok(parsed)
    }
}

// Usage across HTTP and WebSocket
pub fn handle_incoming_message(validator: &MessageValidator, raw_message: &str) -> Result<JsonRpcMessage, ValidationError> {
    validator.validate_jsonrpc_message(raw_message)
}
```

### Phase 4: Response Generation Consolidation
```rust
// src/response/generator.rs
pub struct ResponseGenerator {
    serializer: JsonSerializer,
    progress_handler: ProgressHandler,
}

impl ResponseGenerator {
    pub fn success_response<T: Serialize>(&self, id: JsonRpcId, result: T) -> JsonRpcResponse {
        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
    }
    
    pub fn error_response(&self, id: Option<JsonRpcId>, error: impl Into<JsonRpcError>) -> JsonRpcResponse {
        JsonRpcResponse::error(id, error.into())
    }
    
    pub fn progress_response<T: Serialize>(&self, progress_token: ProgressToken, partial_result: T) -> JsonRpcResponse {
        self.progress_handler.create_progress_response(progress_token, partial_result)
    }
}
```

## Consolidation Strategy

### Extract Common Patterns
1. **Identify Duplication**: Use tools like `cargo-duplicate` to find duplicated code
2. **Extract Utilities**: Create reusable utility functions and traits
3. **Create Abstractions**: Build abstractions that capture common patterns
4. **Refactor Gradually**: Replace duplicated code incrementally

### Utility Modules Structure
```
src/
├── common/
│   ├── errors.rs         # Centralized error handling
│   ├── validation.rs     # Common validation utilities
│   ├── responses.rs      # Response generation utilities
│   └── session.rs        # Session management utilities
├── traits/
│   ├── handler.rs        # Common handler traits
│   ├── validator.rs      # Validation traits
│   └── transport.rs      # Transport abstraction traits
```

## Testing Strategy

### Before Refactoring
```bash
# Measure current duplication
cargo clippy -- -W clippy::redundant_clone
tokei --sort lines  # Get baseline metrics
```

### During Refactoring
- Unit tests for each consolidated utility
- Integration tests to ensure existing functionality
- Regression tests for edge cases

### After Refactoring
```bash
# Verify duplication reduction
tokei --sort lines  # Compare with baseline
cargo test --all    # Ensure all tests pass
```

## Expected Benefits

### Code Quality Improvements
- **DRY Compliance**: Eliminate repeated code patterns
- **Consistency**: Uniform behavior across similar operations
- **Maintainability**: Single point of change for common operations
- **Testability**: Focused testing of consolidated utilities

### Developer Experience
- **Faster Development**: Reuse utilities instead of reimplementing
- **Fewer Bugs**: Consistent implementations reduce error-prone variations
- **Easier Code Review**: Reviewers can focus on business logic, not boilerplate
- **Better Onboarding**: Clear patterns make codebase easier to understand

## Risk Assessment
- **Low Risk**: Consolidation typically improves code quality
- **Medium Impact**: Significant improvement in maintainability
- **Medium Complexity**: Requires careful extraction to maintain functionality

## Dependencies
- Enabled by: TODO-005 (God Object Refactoring)
- Related to: TODO-007 (Large Method Extraction)
- Enables: Better code organization for future features

## Progress Tracking

### Phase 1 (Days 1-2): Error Handling
- [ ] Create centralized error module
- [ ] Consolidate error types
- [ ] Replace duplicated error handling

### Phase 2 (Days 2-3): Session Logic
- [ ] Extract session management utilities
- [ ] Consolidate session validation
- [ ] Update HTTP and WebSocket handlers

### Phase 3 (Days 3-4): Validation
- [ ] Create validation utility module
- [ ] Consolidate message validation
- [ ] Update all validation call sites

### Phase 4 (Days 4-5): Response Generation
- [ ] Create response generation utilities
- [ ] Consolidate response formatting
- [ ] Update all response generation sites

## Progress Notes
- 2025-07-30: Duplication analysis completed, consolidation plan created