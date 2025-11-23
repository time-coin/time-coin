#!/bin/bash
# Quick build script for TIME Coin main binaries

set -e

MODE="${1:-debug}"

if [ "$MODE" = "release" ]; then
    echo "🔨 Building TIME Coin (Release mode)..."
    cargo build --release --bin timed --bin time-cli
    echo "✅ Binaries built:"
    echo "   📦 timed:    target/release/timed"
    echo "   📦 time-cli: target/release/time-cli"
else
    echo "🔨 Building TIME Coin (Debug mode)..."
    cargo build --bin timed --bin time-cli
    echo "✅ Binaries built:"
    echo "   📦 timed:    target/debug/timed"
    echo "   📦 time-cli: target/debug/time-cli"
fi

echo ""
echo "💡 Usage:"
echo "   Debug:   ./build.sh"
echo "   Release: ./build.sh release"
