# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. $HOME/.cargo/env

# Add required components
rustup component add rustfmt clippy

# Install cargo-audit for security checks
cargo install cargo-audit

# Fetch dependencies
cargo fetch