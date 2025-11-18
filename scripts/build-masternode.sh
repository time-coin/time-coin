#!/bin/bash
# Build masternode binary for production deployment
# This script builds only what's needed for a masternode

set -e

echo "🔨 Building TIME Coin Masternode (optimized)..."
echo ""

# Build only masternode and its dependencies (using correct package name)
cargo build --release -p time-masternode

echo ""
echo "✅ Masternode built successfully!"
echo ""
echo "📦 Binary location:"
echo "   target/release/time-masternode"
echo ""
echo "📊 Binary size:"
ls -lh target/release/time-masternode | awk '{print "   " $5}'
echo ""
echo "🚀 To run:"
echo "   ./target/release/time-masternode"
echo ""
