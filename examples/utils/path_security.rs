use std::path::{Path, PathBuf};
use anyhow::{bail, Result};

/// Validates that a path is within the allowed directory and returns the canonical path
pub fn validate_path(path: &str, allowed_dir: &Path) -> Result<PathBuf> {
    let requested = Path::new(path);
    
    // If the path is relative, join it with the allowed directory
    let full_path = if requested.is_relative() {
        allowed_dir.join(requested)
    } else {
        requested.to_path_buf()
    };
    
    // Get canonical path (resolves .. and symlinks)
    let canonical = full_path.canonicalize()
        .map_err(|e| anyhow::anyhow!("Invalid path '{}': {}", path, e))?;
    
    let allowed_canonical = allowed_dir.canonicalize()
        .map_err(|_| anyhow::anyhow!("Invalid allowed directory"))?;
    
    // Check if the canonical path is within the allowed directory
    if !canonical.starts_with(&allowed_canonical) {
        bail!("Path traversal attempt detected: '{}'", path);
    }
    
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_valid_path() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path();
        
        // Create a test file
        let test_file = allowed_dir.join("test.txt");
        fs::write(&test_file, "test").unwrap();
        
        // Test with relative path
        let result = validate_path("test.txt", allowed_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file);
    }
    
    #[test]
    fn test_path_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path();
        
        // Test various path traversal attempts
        let traversal_attempts = vec![
            "../../../etc/passwd",
            "../../..",
            "../temp",
            "/etc/passwd",
            "subdir/../../..",
        ];
        
        for attempt in traversal_attempts {
            let result = validate_path(attempt, allowed_dir);
            assert!(result.is_err(), "Path traversal should be blocked: {}", attempt);
            assert!(result.unwrap_err().to_string().contains("Path traversal") 
                || result.unwrap_err().to_string().contains("Invalid path"));
        }
    }
    
    #[test]
    fn test_subdirectory_allowed() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_dir = temp_dir.path();
        
        // Create subdirectory and file
        let subdir = allowed_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        let test_file = subdir.join("test.txt");
        fs::write(&test_file, "test").unwrap();
        
        // Access to subdirectory should be allowed
        let result = validate_path("subdir/test.txt", allowed_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file);
    }
}