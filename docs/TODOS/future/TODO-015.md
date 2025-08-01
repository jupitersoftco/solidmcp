# TODO-015: Fix Path Traversal Security Vulnerability

**Priority**: 🔴 CRITICAL  
**Effort**: 2 hours  
**Dependencies**: None  
**Category**: Security

## 📋 Description

Fix the critical path traversal vulnerability in `src/tools.rs:199` where file paths are not validated, allowing attackers to read any file on the system.

## 🎯 Acceptance Criteria

- [ ] Path validation function implemented
- [ ] All file operations use validated paths
- [ ] Cannot access files outside allowed directories
- [ ] Tests verify path traversal attempts are blocked
- [ ] No existing functionality broken

## 📊 Current State

```rust
// VULNERABLE CODE in src/tools.rs:199
fs::read_to_string(file_path) // NO VALIDATION!
```

## 🔧 Implementation

### 1. Create Path Validation Module

Create `src/utils/path_security.rs`:
```rust
use std::path::{Path, PathBuf};
use crate::McpError;

pub fn validate_path(path: &str, allowed_dir: &Path) -> Result<PathBuf, McpError> {
    let requested = Path::new(path);
    let canonical = requested.canonicalize()
        .map_err(|_| McpError::InvalidPath(path.to_string()))?;
    
    let allowed_canonical = allowed_dir.canonicalize()
        .map_err(|_| McpError::Internal("Invalid allowed directory".into()))?;
    
    if !canonical.starts_with(&allowed_canonical) {
        return Err(McpError::InvalidPath(format!(
            "Path traversal attempt: {}", path
        )));
    }
    
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_valid_path() {
        let temp_dir = env::temp_dir();
        let result = validate_path("test.txt", &temp_dir);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_path_traversal_blocked() {
        let temp_dir = env::temp_dir();
        let result = validate_path("../../../etc/passwd", &temp_dir);
        assert!(matches!(result, Err(McpError::InvalidPath(_))));
    }
}
```

### 2. Update McpError Type

In `src/error.rs` (or create if doesn't exist):
```rust
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    // ... other variants
}
```

### 3. Fix Vulnerable Code

Update `src/tools.rs`:
```rust
use crate::utils::path_security::validate_path;

// In the file reading function:
pub async fn read_file(params: Value) -> Result<Value> {
    let file_path = params["path"].as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing path parameter"))?;
    
    // Get allowed directory from configuration or context
    let allowed_dir = Path::new("./allowed_files"); // TODO: Get from config
    
    // Validate the path
    let safe_path = validate_path(file_path, &allowed_dir)?;
    
    // Read using validated path
    let contents = fs::read_to_string(safe_path)
        .map_err(|e| McpError::Internal(format!("Failed to read file: {}", e)))?;
    
    Ok(json!({ "contents": contents }))
}
```

## 🧪 Testing

Create `tests/security_test.rs`:
```rust
#[tokio::test]
async fn test_path_traversal_blocked() {
    let server = create_test_server();
    
    let response = server.call_tool("read_file", json!({
        "path": "../../../etc/passwd"
    })).await;
    
    assert!(response.is_err());
    assert!(response.unwrap_err().to_string().contains("Invalid path"));
}

#[tokio::test]
async fn test_valid_file_read() {
    let server = create_test_server();
    
    // Create test file in allowed directory
    fs::create_dir_all("./allowed_files").unwrap();
    fs::write("./allowed_files/test.txt", "Hello").unwrap();
    
    let response = server.call_tool("read_file", json!({
        "path": "test.txt"
    })).await;
    
    assert!(response.is_ok());
    assert_eq!(response.unwrap()["contents"], "Hello");
}
```

## ✅ Verification

1. Run security tests: `cargo test security_test`
2. Attempt path traversal manually
3. Verify legitimate file access still works
4. Run full test suite to ensure no regressions

## 📝 Notes

- Consider making allowed directories configurable
- May need to handle symbolic links specially
- Consider adding file type restrictions
- Log all file access attempts for audit trail