#!/bin/bash
set -e

echo "🔧 Setting up development environment..."

# Install required tools
echo "Installing development tools..."
cargo install cargo-watch 2>/dev/null || echo "cargo-watch already installed"
cargo install cargo-audit 2>/dev/null || echo "cargo-audit already installed"
cargo install cargo-tarpaulin 2>/dev/null || echo "cargo-tarpaulin already installed"

# Create local config
mkdir -p config
if [ ! -f config/local.toml ]; then
    cp config/treasury.toml config/local.toml
    echo "Created config/local.toml"
fi

echo "✅ Development environment ready!"
