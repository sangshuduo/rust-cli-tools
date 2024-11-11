# Format code
cargo fmt --all

# Run clippy with fixes where possible
cargo clippy --workspace --fix --allow-dirty -- -D warnings

# Run basic checks
cargo check --workspace

# Run tests
cargo test --workspace

# Run security audit
cargo audit

# Run typos check (from pre-commit config)
cargo install typos-cli
typos