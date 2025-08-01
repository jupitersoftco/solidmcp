//! Security tests for example tools
//!
//! Run with: cargo run --example security_test

use std::{fs, path::Path};
use tempfile::TempDir;

mod utils;
use utils::path_security::validate_path;

fn test_path_traversal_protection() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let allowed_dir = temp_dir.path();
    
    // Create a test file in the allowed directory
    let test_file = allowed_dir.join("test.txt");
    fs::write(&test_file, "test content")?;
    
    // Create a file outside the allowed directory (in temp)
    let outside_file = std::env::temp_dir().join("outside.txt");
    fs::write(&outside_file, "secret content")?;
    
    // Test 1: Valid path should work
    let result = validate_path("test.txt", allowed_dir);
    assert!(result.is_ok(), "Valid path should be allowed");
    
    // Test 2: Path traversal attempts should fail
    let outside_filename = format!("../{}", outside_file.file_name().unwrap().to_string_lossy());
    let traversal_attempts = vec![
        "../../../etc/passwd",
        "./../outside.txt",
        "subdir/../../outside.txt",
        "/etc/passwd",
        &outside_filename,
    ];
    
    for attempt in traversal_attempts {
        let result = validate_path(attempt, allowed_dir);
        assert!(
            result.is_err(),
            "Path traversal attempt should be blocked: {}",
            attempt
        );
        println!("✅ Blocked path traversal attempt: {}", attempt);
    }
    
    // Test 3: Subdirectories should be allowed
    let subdir = allowed_dir.join("subdir");
    fs::create_dir(&subdir)?;
    let subfile = subdir.join("subfile.txt");
    fs::write(&subfile, "sub content")?;
    
    let result = validate_path("subdir/subfile.txt", allowed_dir);
    assert!(result.is_ok(), "Subdirectory access should be allowed");
    
    println!("✅ All security tests passed!");
    
    // Cleanup
    fs::remove_file(&outside_file)?;
    
    Ok(())
}

fn main() {
    println!("Running security tests...");
    if let Err(e) = test_path_traversal_protection() {
        eprintln!("Test failed: {}", e);
        std::process::exit(1);
    }
}