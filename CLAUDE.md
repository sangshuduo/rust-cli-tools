# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build/Lint/Test Commands
- Build: `cargo build [--release]`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt --all -- --check`
- Test: `cargo test`
- Test single: `cargo test -p <package_name> <test_name>`
- Run tool: `cargo run -p <package_name> -- <arguments>`

## Code Style Guidelines
- **Imports**: Standard library first, external crates alphabetically second, modules last
- **Formatting**: Follow standard Rust formatting (4-space indentation)
- **Types**: Explicitly type function parameters and return types; use PathBuf for file paths
- **Naming**: snake_case for variables/functions, CamelCase for types/structs
- **Error Handling**: Prefer Result<T> over unwrap()/expect(); include context in error messages
- **CLI**: Use clap with derive feature for argument parsing; document with /// comments
- **Concurrency**: Use Rayon for CPU-bound tasks; Tokio for I/O-bound operations
- **Resources**: Ensure proper cleanup of file handles, network connections, progress bars
