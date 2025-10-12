#!/bin/bash
set -e

echo "🚀 Setting up TIME Coin development environment..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "📦 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
else
    echo "✅ Rust already installed: $(rustc --version)"
fi

# Install development tools
echo "📦 Installing development tools..."
cargo install cargo-watch || true
cargo install cargo-audit || true

# Create directory structure
echo "📁 Creating directory structure..."
mkdir -p {core,masternode,network,purchase,wallet,api,cli,storage,crypto}/{src,tests}
mkdir -p docs/{architecture,masternodes,api,developers,wallet}
mkdir -p config
mkdir -p tests/{integration,e2e}
mkdir -p tools/{calculator,benchmarks}/src

echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Review the README.md"
echo "2. Check docs/ for architecture"
echo "3. Start coding!"
