#!/bin/bash
set -e

echo "Building TIME Coin..."

# Clean
cargo clean

# Build all workspace members
cargo build --workspace --release

echo "Build complete!"
echo "Binaries in: target/release/"
