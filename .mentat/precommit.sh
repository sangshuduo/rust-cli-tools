. "$HOME/.cargo/env"
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo check --workspace
cargo test --workspace