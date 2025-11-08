#!/bin/bash
set -e
echo "🔧 TIME Coin Merkle Root Fix"
echo "====================================="
echo ""
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Run this from the time-coin repository root"
    exit 1
fi
BRANCH_NAME="fix/merkle-root-calculation-$(date +%Y%m%d)"
echo "📦 Creating branch: $BRANCH_NAME"
git checkout -b "$BRANCH_NAME"
cp cli/src/block_producer.rs cli/src/block_producer.rs.bak
echo "✅ Backed up original file"
python3 << 'PYEOF'
import re
with open('cli/src/block_producer.rs', 'r') as f:
    content = f.read()
new_func = '''    fn calc_merkle(&self, transactions: &[time_core::Transaction]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }

        use sha3::{Digest, Sha3_256};

        // Build proper merkle tree (matching Block::calculate_merkle_root in core/src/block.rs)
        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| tx.txid.clone())
            .collect();

        // Build merkle tree iteratively
        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..hashes.len()).step_by(2) {
                let left = &hashes[i];
                let right = if i + 1 < hashes.len() {
                    &hashes[i + 1]
                } else {
                    left // Duplicate if odd number
                };

                let combined = format!("{}{}", left, right);
                let hash = Sha3_256::digest(combined.as_bytes());
                next_level.push(hex::encode(hash));
            }

            hashes = next_level;
        }

        hashes[0].clone()
    }'''
pattern = r'    fn calc_merkle\(&self, transactions: &\[time_core::Transaction\]\) -> String \{[^}]*?format!\("\{:x\}\", hasher\.finalize\(\))\s*\}'
updated = re.sub(pattern, new_func, content, flags=re.DOTALL)
if updated == content:
    print("ERROR: Could not find function")
    exit(1)
with open('cli/src/block_producer.rs', 'w') as f:
    f.write(updated)
print("✅ Updated calc_merkle function")
PYEOF
echo "📝 Staging changes..."
git add cli/src/block_producer.rs
git commit -m "Fix merkle root calculation mismatch between producer and validator

The BlockProducer.calc_merkle() method was using simple SHA256 hashing,
while Block.calculate_merkle_root() uses proper SHA3-256 merkle tree building.
This caused InvalidMerkleRoot errors during block validation.

Changes:
- Updated calc_merkle() to use iterative SHA3-256 merkle tree algorithm
- Now matches Block::calculate_merkle_root() in core/src/block.rs  
- Ensures producer-calculated merkles match validator expectations
- Fixes BlockError(InvalidMerkleRoot) on block finalization"
echo "✅ Fix committed to $BRANCH_NAME"
echo "Next: git push -u origin $BRANCH_NAME"
