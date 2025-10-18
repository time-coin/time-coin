//! Quorum calculation for TIME Coin consensus
//! Byzantine Fault Tolerance: need 2f+1 nodes where f is max faulty nodes

/// Minimum number of masternodes required for network operation
pub const MIN_MASTERNODES: usize = 3;

/// Grace period for masternode updates (30 minutes)
pub const GRACE_PERIOD_SECS: i64 = 1800;

/// Calculate required votes for consensus
/// Returns the number of votes needed (2/3 + 1)
pub fn calculate_quorum(total_nodes: usize) -> usize {
    if total_nodes < MIN_MASTERNODES {
        return total_nodes; // Need all nodes if below minimum
    }
    
    // Byzantine fault tolerance: 2f+1 where f = floor((n-1)/3)
    // This ensures we can tolerate up to f faulty nodes
    (total_nodes * 2 / 3) + 1
}

/// Check if we have enough nodes for consensus
pub fn has_quorum(active_nodes: usize) -> bool {
    active_nodes >= MIN_MASTERNODES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_calculation() {
        assert_eq!(calculate_quorum(3), 3);  // 3 nodes: need all 3
        assert_eq!(calculate_quorum(4), 3);  // 4 nodes: need 3
        assert_eq!(calculate_quorum(5), 4);  // 5 nodes: need 4
        assert_eq!(calculate_quorum(6), 5);  // 6 nodes: need 5
        assert_eq!(calculate_quorum(7), 5);  // 7 nodes: need 5
    }

    #[test]
    fn test_min_quorum() {
        assert!(has_quorum(3));
        assert!(has_quorum(4));
        assert!(!has_quorum(2));
        assert!(!has_quorum(1));
    }
}
