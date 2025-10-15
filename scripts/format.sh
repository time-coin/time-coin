#!/bin/bash
set -e

echo "Formatting TIME Coin code..."

# Format all code
cargo fmt --all

# Check with clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "Code formatting complete!"
