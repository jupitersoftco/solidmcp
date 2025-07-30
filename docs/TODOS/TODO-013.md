# TODO-013: Naming Consistency - Standardize Naming Conventions

**Status**: pending
**Priority**: low
**Created**: 2025-07-30
**Updated**: 2025-07-30
**Assignee**: development-team
**Due Date**: 2025-08-27
**Tags**: naming-consistency, code-style, maintainability, refactoring
**Estimated Effort**: 2-3 days

## Description

The codebase has inconsistent naming conventions across different modules and files. This includes inconsistent use of snake_case vs camelCase, abbreviations vs full words, and different naming patterns for similar concepts. Consistent naming improves code readability and makes the codebase easier to navigate and understand.

## Identified Naming Inconsistencies

### 1. Function and Variable Naming
```rust
// Inconsistent: Mix of abbreviations and full words
fn get_conn() -> Connection          // Should be: get_connection()
fn initialize_protocol_handler()     // Good: full words
let sess_id = "123";                // Should be: session_id
let connection_manager = ...;        // Good: full words
```

### 2. Type Naming Patterns
```rust
// Inconsistent: Different patterns for similar concepts
struct HttpConfig { ... }           // Good: descriptive
struct WSConfig { ... }             // Should be: WebSocketConfig
struct McpProtocolEngine { ... }    // Good: descriptive
struct ProtocolImpl { ... }         // Should be: ProtocolImplementation or McpProtocolImpl
```

### 3. Module and File Naming
```rust
// Current inconsistencies:
mod http;                    // Good: clear module name
mod websocket;              // Should be: web_socket (snake_case for modules)
mod protocol_impl;          // Good: descriptive snake_case
mod shared;                 // Should be: protocol_engine or core
```

### 4. Constant and Static Naming
```rust
// Inconsistent patterns:
const MAX_CONNECTIONS: usize = 100;     // Good: SCREAMING_SNAKE_CASE
const defaultTimeout: u64 = 30;         // Should be: DEFAULT_TIMEOUT
static SESSION_STORE: ...;              // Good: descriptive
static SESSIONS: ...;                   // Should be: SESSION_REGISTRY
```

### 5. Error Type Naming
```rust
// Inconsistent error naming:
enum HttpError { ... }               // Good: descriptive
enum WSErr { ... }                   // Should be: WebSocketError
enum ProtocolErr { ... }             // Should be: ProtocolError
enum TransportError { ... }          // Good: descriptive
```

## Current Naming Patterns Analysis

### Abbreviation Usage
- **Inconsistent**: `conn`, `sess`, `proto`, `impl` mixed with full words
- **Preferred**: Use full words for clarity: `connection`, `session`, `protocol`, `implementation`

### Module Naming
- **Current**: Mix of snake_case and single words
- **Rust Standard**: snake_case for module names
- **Preferred**: Descriptive snake_case names

### Type Naming
- **Current**: Some types use abbreviations, others use full names
- **Rust Standard**: PascalCase with descriptive names
- **Preferred**: Clear, unambiguous type names

## Acceptance Criteria

- [ ] Establish comprehensive naming conventions document
- [ ] Standardize all function and variable names to use full words
- [ ] Ensure all module names follow snake_case convention
- [ ] Rename all types to use consistent, descriptive names
- [ ] Standardize constant and static naming to SCREAMING_SNAKE_CASE
- [ ] Update all documentation to reflect new naming
- [ ] Ensure all tests pass after renaming
- [ ] Add linting rules to enforce naming conventions

## Proposed Naming Conventions

### General Principles
1. **Clarity over Brevity**: Use full words instead of abbreviations
2. **Consistency**: Similar concepts should have similar naming patterns
3. **Rust Standards**: Follow official Rust naming conventions
4. **Domain Alignment**: Names should reflect the MCP protocol domain

### Specific Conventions

#### Functions and Variables (snake_case)
```rust
// Before (inconsistent)
fn get_conn() -> Connection
fn init_proto() -> Protocol
let sess_mgr = SessionManager::new();
let ws_config = WebSocketConfig::default();

// After (consistent)
fn get_connection() -> Connection
fn initialize_protocol() -> Protocol
let session_manager = SessionManager::new();
let websocket_config = WebSocketConfig::default();
```

#### Types and Structs (PascalCase)
```rust
// Before (inconsistent)
struct HttpConfig { ... }
struct WSConfig { ... }
struct ProtocolImpl { ... }
struct SessionMgr { ... }

// After (consistent)
struct HttpConfig { ... }
struct WebSocketConfig { ... }
struct ProtocolImplementation { ... }
struct SessionManager { ... }
```

#### Modules (snake_case)
```rust
// Before (inconsistent)
mod http;
mod websocket;
mod protocol_impl;
mod shared;

// After (consistent)
mod http;
mod web_socket;
mod protocol_implementation;
mod protocol_engine;
```

#### Constants (SCREAMING_SNAKE_CASE)
```rust
// Before (inconsistent)
const MAX_CONNECTIONS: usize = 100;
const defaultTimeout: u64 = 30;
const maxSessions: usize = 1000;

// After (consistent)
const MAX_CONNECTIONS: usize = 100;
const DEFAULT_TIMEOUT: u64 = 30;
const MAX_SESSIONS: usize = 1000;
```

#### Error Types (descriptive + Error suffix)
```rust
// Before (inconsistent)
enum HttpError { ... }
enum WSErr { ... }
enum ProtocolErr { ... }

// After (consistent)
enum HttpError { ... }
enum WebSocketError { ... }
enum ProtocolError { ... }
```

## Implementation Plan

### Phase 1: Documentation and Standards (Day 1)
Create comprehensive naming convention documentation:

```rust
// src/NAMING_CONVENTIONS.md
# SolidMCP Naming Conventions

## General Rules
1. Use full words instead of abbreviations
2. Follow Rust naming conventions strictly
3. Be consistent across similar concepts
4. Prioritize clarity and readability

## Specific Patterns
- Functions/Variables: snake_case with full words
- Types/Structs: PascalCase with descriptive names
- Modules: snake_case with clear purpose
- Constants: SCREAMING_SNAKE_CASE
- Enums: PascalCase with Error/Result suffix where appropriate
```

### Phase 2: Automated Detection (Day 1)
Create linting rules to catch naming inconsistencies:

```rust
// In .clippy.toml or Cargo.toml
[lints.clippy]
# Enforce naming conventions
enum_variant_names = "warn"
module_name_repetitions = "warn"
similar_names = "warn"

# Custom lints for abbreviations
# (would need custom implementation)
```

### Phase 3: Systematic Renaming (Days 1-2)

#### Step 1: Create Mapping Table
```rust
// RENAMING_MAP.md
| Current Name | New Name | Type | Location |
|--------------|----------|------|----------|
| get_conn | get_connection | function | src/http.rs:45 |
| WSConfig | WebSocketConfig | struct | src/websocket.rs:12 |
| sess_id | session_id | variable | multiple files |
| ProtocolImpl | ProtocolImplementation | struct | src/protocol_impl.rs:8 |
```

#### Step 2: Rename Using IDE/Tools
```bash
# Use rust-analyzer or sed for systematic renaming
find src -name "*.rs" -exec sed -i 's/get_conn/get_connection/g' {} \;
find src -name "*.rs" -exec sed -i 's/WSConfig/WebSocketConfig/g' {} \;
```

#### Step 3: Update Tests and Documentation
```rust
// Update all test files
find tests -name "*.rs" -exec sed -i 's/old_name/new_name/g' {} \;

// Update documentation
find docs -name "*.md" -exec sed -i 's/old_name/new_name/g' {} \;
```

### Phase 4: Verification and Testing (Day 2-3)
```bash
# Ensure everything compiles
cargo build --all-targets

# Run all tests
cargo test

# Check for remaining inconsistencies
cargo clippy -- -D warnings

# Verify documentation builds
cargo doc --all
```

## Specific Renaming Tasks

### High Priority Renames
1. **Module Renames**:
   - `shared.rs` → `protocol_engine.rs`
   - `protocol_impl.rs` → `protocol_implementation.rs`

2. **Type Renames**:
   - `McpProtocolEngine` → `ProtocolEngine` (if module is named protocol_engine)
   - `McpProtocolHandlerImpl` → `ProtocolHandler`

3. **Function Renames**:
   - All abbreviated function names to full words
   - Consistent verb patterns (get_, set_, create_, handle_)

### Naming Pattern Standardization

#### Transport-Related Names
```rust
// Consistent transport naming
HttpTransport, HttpConfig, HttpError, HttpConnection
WebSocketTransport, WebSocketConfig, WebSocketError, WebSocketConnection
```

#### Session-Related Names
```rust
// Consistent session naming
SessionManager, SessionId, SessionError, SessionConfig
session_id, create_session, get_session, remove_session
```

#### Protocol-Related Names
```rust
// Consistent protocol naming
ProtocolEngine, ProtocolHandler, ProtocolError, ProtocolConfig
handle_message, process_request, generate_response
```

## Testing Strategy

### Rename Verification Tests
```rust
#[cfg(test)]
mod naming_tests {
    // Test that renamed items are accessible
    #[test]
    fn test_renamed_types_accessible() {
        let _config = WebSocketConfig::default(); // Was: WSConfig
        let _manager = SessionManager::new();     // Was: SessionMgr
    }
    
    // Test that old names are not accessible (should not compile)
    #[test]
    #[should_not_compile]
    fn test_old_names_removed() {
        let _config = WSConfig::default(); // Should not compile
    }
}
```

### Documentation Tests
```rust
// Ensure documentation examples use new names
#[doc = r#"
```rust
let session_manager = SessionManager::new(); // Updated example
let websocket_config = WebSocketConfig::default(); // Updated example
```
"#]
```

## Tool Integration

### IDE Configuration
```json
// .vscode/settings.json
{
    "rust-analyzer.diagnostics.disabled": [],
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.extraArgs": ["--", "-D", "warnings"]
}
```

### Pre-commit Hooks
```bash
#!/bin/sh
# .git/hooks/pre-commit
# Check for naming convention violations
cargo clippy -- -D clippy::enum_variant_names -D clippy::module_name_repetitions
```

## Expected Benefits

### Code Quality Improvements
- **Readability**: Consistent naming makes code easier to read
- **Maintainability**: Clear names reduce confusion and errors
- **Professionalism**: Consistent conventions improve code quality perception
- **Onboarding**: New developers can understand code faster

### Development Process Benefits
- **IDE Support**: Better autocomplete and navigation
- **Code Review**: Less time spent on style discussions
- **Documentation**: Clearer API documentation
- **Refactoring**: Easier to find and rename related concepts

## Risk Assessment
- **Very Low Risk**: Naming changes don't affect functionality
- **Low Impact**: Primarily improves developer experience
- **Low Complexity**: Mostly automated search-and-replace operations

## Dependencies
- Should be done after: TODO-014 (Logging Optimization) for minimal conflicts
- Independent of other architectural changes
- Can be done in parallel with other low-priority tasks

## Long-term Maintenance

### Naming Review Process
1. **Code Review**: Check naming consistency in all PRs
2. **Documentation**: Update style guide as conventions evolve
3. **Tooling**: Maintain linting rules to catch violations
4. **Examples**: Keep examples updated with current naming

### Automated Enforcement
```toml
# In Cargo.toml
[lints.clippy]
enum_variant_names = "deny"
module_name_repetitions = "deny"
similar_names = "warn"
```

## Progress Notes
- 2025-07-30: Naming inconsistency analysis completed, standardization plan created