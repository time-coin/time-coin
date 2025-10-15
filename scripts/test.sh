#!/bin/bash
set -e

echo "Running TIME Coin tests..."

# Run all tests
cargo test --workspace --all-features

echo "All tests passed!"
