cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo check --workspace
cargo test --workspace
cargo install cargo-audit && cargo audit