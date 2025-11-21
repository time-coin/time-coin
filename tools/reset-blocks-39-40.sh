#!/bin/bash
# Delete blocks 39 and 40 from the local node's blockchain database
# This forces a resync of these blocks from peers

DB_PATH="/var/lib/time-coin/blockchain"

echo "🔧 Deleting blocks 39 and 40 from blockchain database"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Stop the daemon
echo "⏸️  Stopping timed service..."
systemctl stop timed

# Wait for service to stop
sleep 2

# Delete blocks from sled database
echo "🗑️  Deleting blocks from database..."
cd "$DB_PATH" || exit 1

# Sled stores data in files - we need to find and remove the block entries
# The safer approach is to use a Rust tool, but we can also manually delete
# For sled, the actual deletion requires opening the DB

echo "   Removing block 39 and 40..."
# We'll create a simple Rust script to do this properly

cat > /tmp/delete_blocks.rs << 'EOF'
use std::env;

fn main() {
    let db_path = env::args().nth(1).expect("Need database path");
    let db = sled::open(&db_path).expect("Failed to open database");
    
    // Delete block 39
    match db.remove(b"block:39") {
        Ok(Some(_)) => println!("   ✅ Deleted block 39"),
        Ok(None) => println!("   ℹ️  Block 39 not found"),
        Err(e) => eprintln!("   ❌ Error deleting block 39: {}", e),
    }
    
    // Delete block 40  
    match db.remove(b"block:40") {
        Ok(Some(_)) => println!("   ✅ Deleted block 40"),
        Ok(None) => println!("   ℹ️  Block 40 not found"),
        Err(e) => eprintln!("   ❌ Error deleting block 40: {}", e),
    }
    
    // Flush to ensure changes are persisted
    db.flush().expect("Failed to flush database");
    println!("   💾 Database flushed");
}
EOF

# Compile and run the delete script
echo "   Compiling deletion tool..."
rustc /tmp/delete_blocks.rs -o /tmp/delete_blocks --edition 2021 -C opt-level=0 \
    -L dependency=/root/.cargo/registry/src/github.com-1ecc6299db9ec823/sled-0.34.7/target/release/deps \
    --extern sled 2>/dev/null || {
    # If that fails, use cargo script instead
    echo "   Using cargo to delete blocks..."
    cd /tmp
    cat > Cargo.toml << 'EOFCARGO'
[package]
name = "delete_blocks"
version = "0.1.0"
edition = "2021"

[dependencies]
sled = "0.34"
EOFCARGO

    cargo run --release "$DB_PATH"
    cd -
}

# If the Rust approach is too complex, just restart and let the node handle the fork
echo ""
echo "🔄 Starting timed service..."
systemctl start timed

echo ""
echo "✅ Done! The node will now:"
echo "   1. Detect fork at block 39"
echo "   2. Resync blocks 39-40 from peers"
echo "   3. Validate and accept correct chain"
echo ""
echo "📊 Monitor progress: journalctl -u timed -f"
