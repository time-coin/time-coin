use time_core::block::Block;

#[test]
fn test_genesis_file_loads() {
    // Load the genesis file
    let genesis_path = "../config/genesis-testnet.json";
    let contents = std::fs::read_to_string(genesis_path).expect("Failed to read genesis file");
    
    // Parse the genesis file
    #[derive(Debug, serde::Deserialize)]
    struct GenesisFile {
        network: String,
        #[allow(dead_code)]
        version: u32,
        #[serde(default)]
        message: String,
        block: Block,
    }
    
    let genesis: GenesisFile = serde_json::from_str(&contents).expect("Failed to parse genesis file");
    
    // Verify the block structure
    assert_eq!(genesis.block.header.block_number, 0);
    assert_eq!(genesis.network, "testnet");
    assert_eq!(genesis.block.transactions.len(), 1);
    assert!(genesis.block.transactions[0].is_coinbase());
    
    // Verify the hash matches the calculated hash
    let calculated_hash = genesis.block.calculate_hash();
    assert_eq!(genesis.block.hash, calculated_hash, "Genesis block hash mismatch");
    
    println!("✓ Genesis block loaded successfully");
    println!("  Network: {}", genesis.network);
    println!("  Hash: {}", genesis.block.hash);
    println!("  Timestamp: {}", genesis.block.header.timestamp);
}

#[test]
fn test_multiple_loads_same_hash() {
    // This test verifies that loading the genesis file multiple times
    // produces the exact same block hash
    
    #[derive(Debug, serde::Deserialize)]
    struct GenesisFile {
        network: String,
        #[allow(dead_code)]
        version: u32,
        #[serde(default)]
        message: String,
        block: Block,
    }
    
    let genesis_path = "../config/genesis-testnet.json";
    
    // Load genesis first time
    let contents1 = std::fs::read_to_string(genesis_path).expect("Failed to read genesis file");
    let genesis1: GenesisFile = serde_json::from_str(&contents1).expect("Failed to parse genesis file");
    let hash1 = genesis1.block.hash.clone();
    
    // Load genesis second time
    let contents2 = std::fs::read_to_string(genesis_path).expect("Failed to read genesis file");
    let genesis2: GenesisFile = serde_json::from_str(&contents2).expect("Failed to parse genesis file");
    let hash2 = genesis2.block.hash.clone();
    
    // Hashes must be identical
    assert_eq!(hash1, hash2, "Genesis hashes must be identical across loads");
    
    // Timestamps must be identical
    assert_eq!(genesis1.block.header.timestamp, genesis2.block.header.timestamp);
    
    println!("✓ Multiple loads produce identical genesis blocks");
    println!("  Hash: {}", hash1);
}
