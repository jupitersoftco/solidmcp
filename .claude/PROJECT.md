# SolidMCP Project Configuration

## Development Practices

### Test-Driven Development (TDD)
This project MANDATES strict TDD practices. The `tdd-enforcer` agent MUST BE USED PROACTIVELY for:
- All bug fixes
- All new features
- Any code refactoring
- Performance improvements

#### TDD Protocol (MANDATORY - Each phase < 10 minutes)
1. **RED**: Write failing test following FIRST principles
2. **GREEN**: Minimal code using Fake It/Triangulate/Obvious patterns
3. **REFACTOR**: Improve while maintaining green bar

The `tdd-enforcer` agent enforces:
- FIRST principles (Fast, Isolated, Repeatable, Self-Validating, Timely)
- AAA pattern (Arrange-Act-Assert)
- Single feature per test
- Clean TDD practices and smell detection
- TODO list management in test files

#### Automatic TDD Agent Invocation
The TDD agent should be invoked automatically when:
- Fixing any reported bug
- Implementing new functionality
- Modifying existing behavior
- Addressing test failures

Example usage:
```bash
# The agent will be invoked automatically for these tasks:
"Fix session handling bug" → tdd-enforcer agent activated
"Add new authentication method" → tdd-enforcer agent activated
"Improve performance of data parsing" → tdd-enforcer agent activated
```

## Project-Specific Guidelines

### Testing Infrastructure
- Unit tests: Located alongside source files as `mod tests`
- Integration tests: In `tests/` directory
- Test helpers: `tests/mcp_test_helpers.rs`
- Run all tests: `cargo test`
- Run specific test: `cargo test test_name`

### Code Quality Standards
- All tests must pass before merging
- Maintain or increase code coverage
- Follow Rust idioms and clippy suggestions
- Use `cargo fmt` for consistent formatting

### Common Test Patterns
1. **HTTP Protocol Tests**: See `tests/http_protocol_compliance_test.rs`
2. **Session Tests**: See `tests/session_reinitialization_test.rs`
3. **Mock Helpers**: Use utilities in `mcp_test_helpers.rs`

### Documentation Requirements
- Test names should be descriptive sentences
- Include `#[should_panic]` for expected failures
- Use doc comments for complex test scenarios
- Reference issue numbers in test comments

## Benefits of TDD Approach

1. **Early Bug Detection**: Problems caught before code review
2. **Living Documentation**: Tests demonstrate intended behavior
3. **Refactoring Safety**: Changes verified against comprehensive test suite
4. **Design Improvement**: Test-first approach leads to better APIs
5. **Debugging Efficiency**: Failing tests isolate problems quickly