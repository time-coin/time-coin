//! Simple Verifiable Random Function for quorum selection

use sha2::{Sha256, Digest};

/// Simple VRF implementation
pub struct Vrf {
    state: u64,
}

impl Vrf {
    /// Create new VRF with seed
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    
    /// Get next random u64
    pub fn next_u64(&mut self) -> u64 {
        // Simple XorShift64
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    
    /// Hash-based random (for additional entropy)
    pub fn hash_random(&mut self, input: &[u8]) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(self.state.to_le_bytes());
        hasher.update(input);
        let result = hasher.finalize();
        
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[0..8]);
        
        self.state = u64::from_le_bytes(bytes);
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vrf_deterministic() {
        let mut vrf1 = Vrf::new(12345);
        let mut vrf2 = Vrf::new(12345);
        
        for _ in 0..10 {
            assert_eq!(vrf1.next_u64(), vrf2.next_u64());
        }
    }
    
    #[test]
    fn test_vrf_different_seeds() {
        let mut vrf1 = Vrf::new(12345);
        let mut vrf2 = Vrf::new(54321);
        
        assert_ne!(vrf1.next_u64(), vrf2.next_u64());
    }
}
