#!/bin/bash
set -e

echo "Generating TIME Coin documentation..."

# Generate Rust API docs
cargo doc --workspace --no-deps --open

echo "Documentation generated!"
echo "Open: target/doc/treasury/index.html"
