# Contributing to SolidMCP

Thank you for your interest in contributing to SolidMCP! This document provides guidelines for contributing to the project.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/yourusername/solidmcp.git
   cd solidmcp
   ```
3. Install dependencies:
   ```bash
   cargo build
   ```
4. Run tests to ensure everything works:
   ```bash
   cargo test
   ```

## Development Guidelines

### Code Style

- Follow standard Rust formatting with `cargo fmt`
- Run `cargo clippy` to catch common issues
- Use descriptive variable and function names
- Add documentation for public APIs
- Include tests for new functionality

### Testing

- Add unit tests for new functions and modules
- Add integration tests for new MCP features
- Run the full test suite before submitting:
  ```bash
  cargo test
  cargo test --test "*integration*"
  ```
- Test the toy example to ensure it still works:
  ```bash
  cd examples/toy && cargo test
  ```

### Documentation

- Update README.md if adding new features
- Add inline documentation for public APIs
- Update the toy example if relevant
- Include examples in doc comments

## Submitting Changes

1. Create a new branch for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Make your changes and commit them:
   ```bash
   git commit -m "feat: add your feature description"
   ```
3. Push to your fork and submit a pull request

### Pull Request Guidelines

- Include a clear description of the changes
- Reference any related issues
- Ensure all tests pass
- Update documentation as needed
- Keep commits focused and atomic

### Commit Message Format

Use conventional commit format:
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `test:` for test additions/changes
- `refactor:` for code refactoring

## Areas for Contribution

- **Performance Improvements**: Protocol handling, serialization optimizations
- **Documentation**: Examples, tutorials, API documentation
- **Testing**: Additional test coverage, edge cases
- **Features**: New MCP capabilities, transport improvements
- **Examples**: More real-world example servers

## Questions and Support

- Open an issue for bugs or feature requests
- Start a discussion for questions about usage or architecture
- Check existing issues before creating new ones

## License

By contributing, you agree that your contributions will be licensed under the MIT License.