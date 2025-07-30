# TDD Quick Reference Card

## The Cycle (Each < 10 minutes)
```
RED → GREEN → REFACTOR → REPEAT
```

## FIRST Principles Checklist
- [ ] **F**ast - Milliseconds, not seconds
- [ ] **I**solated - No test dependencies
- [ ] **R**epeatable - Same result every time
- [ ] **S**elf-Validating - Clear pass/fail
- [ ] **T**imely - Written before code

## Test Structure
```rust
#[test]
fn test_feature_when_scenario_then_behavior() {
    // Arrange
    let input = setup_test_data();
    
    // Act
    let result = function_under_test(input);
    
    // Assert
    assert_eq!(result, expected_value);
}
```

## Green Bar Patterns
| Pattern | When to Use | Example |
|---------|-------------|---------|
| Fake It | First test | `return 42;` |
| Triangulate | Need abstraction | Test with 2+ examples |
| Obvious | Simple implementation | Direct implementation |

## Red Flags 🚩
- Test setup > 5 lines
- Multiple assertions per test
- Test needs test to understand
- No test for boundary conditions
- Testing private methods
- Mocking everything

## TODO List Format
```rust
// TODO: Test empty input handling
// TODO: Test maximum size boundary
// TODO: Test concurrent access
```

## Commands
```bash
# Run single test
cargo test test_name

# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Check coverage
cargo tarpaulin
```