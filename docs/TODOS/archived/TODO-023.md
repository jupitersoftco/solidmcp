# TODO-023: Remove Circular Dependencies

**Status**: ✅ COMPLETED (2025-08-01)  
**Priority**: 🟢 MEDIUM  
**Effort**: 2 hours  
**Dependencies**: TODO-022 (need clean modules first)  
**Category**: Architecture, Code Quality

## 📋 Description

Remove circular dependencies between modules, particularly:
- `handlers.rs` ↔ `tools.rs` 
- Legacy `handlers.rs` that duplicates `protocol_impl.rs`
- Move example tools out of core library

## 🎯 Acceptance Criteria

- [x] No circular dependencies remain ✅
- [x] `handlers.rs` removed (merged into protocol_impl) ✅
- [x] `tools.rs` moved to examples ✅
- [x] All tests still pass ✅ (164 tests passing)
- [x] Examples demonstrate tool implementation ✅

## 📊 Current State

```rust
// handlers.rs imports from tools.rs
use crate::tools::McpTools;

// tools.rs imports from handlers.rs  
use crate::handlers::something;

// handlers.rs duplicates protocol_impl.rs functionality!
```

## 🔧 Implementation

### 1. Analyze Dependencies

First, understand what's where:
```bash
# Find circular imports
grep -r "use crate::" src/ | grep -E "(handlers|tools)"
```

### 2. Remove handlers.rs

Move any unique functionality to `protocol_impl.rs`:
```rust
// If handlers.rs has useful code, move it to protocol_impl.rs
// Most likely it's all duplicate and can be deleted

// In protocol_impl.rs, add any missing functionality
impl<C> McpProtocolHandlerImpl<C> {
    // Consolidated implementation
}
```

### 3. Move tools.rs to Examples

Create `examples/example_tools.rs`:
```rust
use solidmcp::{McpServerBuilder, ToolResponse, Value, json};
use std::fs;

/// Example tool implementations
pub struct FileTools {
    allowed_dir: PathBuf,
}

impl FileTools {
    pub fn new(allowed_dir: PathBuf) -> Self {
        Self { allowed_dir }
    }
    
    pub async fn read_file(&self, params: Value) -> ToolResponse {
        let path = match params.get("path").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => return ToolResponse::error("Missing 'path' parameter"),
        };
        
        // Validate path (using code from TODO-015)
        let safe_path = match self.validate_path(path) {
            Ok(p) => p,
            Err(e) => return ToolResponse::error(format!("Invalid path: {}", e)),
        };
        
        match fs::read_to_string(safe_path) {
            Ok(content) => ToolResponse::success(json!({
                "content": content
            })),
            Err(e) => ToolResponse::error(format!("Failed to read file: {}", e)),
        }
    }
    
    fn validate_path(&self, path: &str) -> Result<PathBuf, String> {
        let requested = Path::new(path);
        let canonical = requested.canonicalize()
            .map_err(|_| "Invalid path")?;
        
        if !canonical.starts_with(&self.allowed_dir) {
            return Err("Path traversal attempt".into());
        }
        
        Ok(canonical)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_tools = FileTools::new(PathBuf::from("./allowed_files"));
    
    let server = McpServerBuilder::new()
        .with_tool(
            "read_file",
            "Read contents of a file",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
            move |params| {
                let tools = file_tools.clone();
                async move { tools.read_file(params).await }
            }
        )
        .build();
    
    println!("Server with example tools starting on :3000");
    server.start("127.0.0.1:3000").await
}
```

### 4. Update Imports

Remove all references to removed modules:
```rust
// Remove from any files:
// use crate::handlers::*;
// use crate::tools::*;

// Update protocol_impl.rs if needed
use crate::handler::McpHandler;  // trait only
use crate::error::{McpError, McpResult};
```

### 5. Update lib.rs

Ensure removed modules aren't exported:
```rust
// lib.rs - REMOVE these lines:
// pub mod handlers;  // DELETED
// pub mod tools;     // MOVED TO EXAMPLES

// Only clean exports remain
```

### 6. Create More Examples

Create `examples/custom_handler.rs` to show handler pattern:
```rust
use solidmcp::{McpHandler, McpContext, Value, McpResult};
use async_trait::async_trait;

struct MyCustomHandler;

#[async_trait]
impl<C> McpHandler<C> for MyCustomHandler {
    async fn list_tools(&self, _context: Arc<C>) -> McpResult<Vec<ToolDefinition>> {
        Ok(vec![
            ToolDefinition {
                name: "my_tool".into(),
                description: "A custom tool".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                }),
            }
        ])
    }
    
    async fn call_tool(
        &self,
        tool_name: &str,
        params: Value,
        _context: Arc<C>,
    ) -> McpResult<Value> {
        match tool_name {
            "my_tool" => {
                let input = params.get("input")
                    .and_then(|v| v.as_str())
                    .ok_or(McpError::InvalidParams("Missing input".into()))?;
                
                Ok(json!({
                    "result": format!("Processed: {}", input)
                }))
            }
            _ => Err(McpError::UnknownTool(tool_name.into()))
        }
    }
}
```

## 🧪 Testing

```rust
#[test]
fn test_no_circular_deps() {
    // This test is really done by successful compilation
    // If there were circular deps, it wouldn't compile
}

#[test]
fn test_examples_compile() {
    // Run: cargo build --examples
    // All examples should build successfully
}

#[test]
fn test_protocol_has_all_functionality() {
    // Ensure protocol_impl has everything needed
    let handler = McpProtocolHandlerImpl::new(context, None);
    // Test all methods work
}
```

## ✅ Verification

1. No more `handlers.rs` file exists
2. No more `tools.rs` in src/
3. Examples demonstrate tool patterns
4. `cargo build` succeeds (no circular deps)
5. All tests pass

## 📝 Notes

- This simplifies the codebase significantly
- Examples serve as documentation for patterns
- Users implement their own tools, not use built-ins
- Cleaner separation of library vs examples