#!/bin/bash
# Quick deployment script for fork resolution fix
# Run this on your build machine, then deploy to nodes

echo "🔧 Building TIME Coin with fork resolution fix..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"
echo ""
echo "📦 Binary location: target/release/timed"
echo ""
echo "To deploy to nodes:"
echo "  1. Copy binary to nodes: scp target/release/timed root@node:/usr/local/bin/"
echo "  2. Verify config has: allow_block_recreation = true"
echo "  3. Restart service: systemctl restart timed"
echo "  4. Monitor logs: journalctl -u timed -f"
echo ""
echo "Expected behavior:"
echo "  - Nodes will detect fork at block 39"
echo "  - Automatic rollback to block 38"
echo "  - Blocks 39 and 40 recreated via consensus"
echo "  - Fork permanently resolved"
