# Consensus Migration - Old BFT to Deterministic

## Problem
The existing blockchain was created with the OLD leader-based BFT consensus system. Those blocks have different hashes and structures than what the NEW deterministic consensus creates.

## Why Blocks Don't Match
1. **Old System (Leader-based):**
   - Leader creates block with their validator_address
   - Block has leader's signature
   - Hash includes leader-specific data

2. **New System (Deterministic):**
   - All nodes create identical block
   - validator_address = `consensus_block_{num}`
   - Hash is deterministic across all nodes

## The Incompatibility
When new node joins and downloads old blocks:
- Old block 1 hash: `8a5817db9bcf7676...` (from leader node)
- New block 2 previous_hash: `9a81c7599d8eed97...` (genesis hash)
- **Chain breaks** because block 2 doesn't point to block 1!

## Solution: Testnet Reset Required

Since this is testnet and we're fundamentally changing consensus:

1. **Stop all nodes**
2. **Clear blockchain data** on all nodes:
   ```bash
   rm -rf /var/lib/time-coin/blockchain/*
   rm -f /var/lib/time-coin/block_height.txt
   ```
3. **Deploy new code** with deterministic consensus
4. **Restart all nodes**
5. **All nodes will recreate blocks 1-47** using deterministic consensus
6. **Blocks will match** because all nodes use same algorithm

## Why This Won't Happen on Mainnet
- Mainnet will launch with deterministic consensus from day 1
- All blocks will be created with same algorithm
- No migration needed

## Test Plan
1. Reset testnet (coordinate with all operators)
2. Start all nodes simultaneously
3. Watch logs for first 10-minute block creation
4. Verify all nodes create identical blocks
5. Confirm consensus without leader election
