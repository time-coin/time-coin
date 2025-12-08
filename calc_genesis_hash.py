#!/usr/bin/env python3
from hashlib import sha3_256

# Genesis block header data
block_number = 0
timestamp = "2025-12-01T00:00:00+00:00"
previous_hash = "0" * 64
merkle_root = "coinbase_0"
validator_address = "genesis"

# Calculate hash same way as Rust code
hasher = sha3_256()
hasher.update(block_number.to_bytes(8, byteorder='little'))
hasher.update(timestamp.encode())
hasher.update(previous_hash.encode())
hasher.update(merkle_root.encode())
hasher.update(validator_address.encode())

hash1 = hasher.digest()
hash2 = sha3_256(hash1).hexdigest()

print(f"Genesis block hash: {hash2}")
