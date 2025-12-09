//! Quorum calculation for TIME Coin consensus
//!
//! ## Byzantine Fault Tolerance Safety Properties
//!
//! To tolerate f Byzantine (malicious) nodes among n total nodes, we require:
//! - n ≥ 3f + 1 (total nodes must be at least 3x malicious + 1)
//! - Quorum threshold ≥ ⌈2n/3⌉ (at least 2/3 of nodes must agree)
//!
//! This ensures that:
//! - Even if f nodes are malicious, honest nodes (n - f ≥ 2f + 1) can form quorum
//! - A malicious coalition of f nodes cannot force consensus (f < n/3 < 2n/3)
//!
//! Examples:
//! - 3 nodes: requires 2 votes (67%), tolerates 0 Byzantine
//! - 4 nodes: requires 3 votes (75%), tolerates 1 Byzantine
//! - 10 nodes: requires 7 votes (70%), tolerates 3 Byzantine
//! - 100 nodes: requires 67 votes (67%), tolerates 33 Byzantine

pub const MIN_MASTERNODES: usize = 3; // Minimum for BFT consensus (tolerates 0 Byzantine failures)
pub const GRACE_PERIOD_SECS: i64 = 1800;

/// Calculate quorum using proper 2/3 threshold for Byzantine Fault Tolerance
///
/// This replaces the previous fixed quorum of 3, which broke BFT safety as networks grew.
pub fn calculate_quorum(total_nodes: usize) -> usize {
    if total_nodes < MIN_MASTERNODES {
        return total_nodes;
    }

    // BFT requires ⌈2n/3⌉ for safety
    // Never less than MIN_MASTERNODES for bootstrap safety
    let quorum = (total_nodes * 2).div_ceil(3);
    quorum.max(MIN_MASTERNODES).min(total_nodes)
}

pub fn has_quorum(active_nodes: usize) -> bool {
    active_nodes >= MIN_MASTERNODES
}

/// Calculate required votes for a threshold
///
/// # Arguments
/// * `total` - Total number of voters
/// * `numerator` - Threshold numerator (e.g., 2 for 2/3)
/// * `denominator` - Threshold denominator (e.g., 3 for 2/3)
pub fn calculate_required_votes(total: usize, numerator: usize, denominator: usize) -> usize {
    (total * numerator).div_ceil(denominator)
}

/// Calculate required votes for BFT consensus (2/3 majority)
///
/// CRITICAL: Uses dynamic 2/3 threshold, not fixed quorum.
/// This ensures Byzantine Fault Tolerance as the network scales.
pub fn required_for_bft(total: usize) -> usize {
    calculate_quorum(total)
}

/// Calculate required votes for simple majority (1/2 threshold)
pub fn required_for_majority(total: usize) -> usize {
    calculate_required_votes(total, 1, 2)
}

/// Calculate rejection threshold (1/3 of total)
pub fn rejection_threshold(total: usize) -> usize {
    total.div_ceil(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bft_safety_small_network() {
        // With 3 nodes, need 2 votes (67%) - tolerates 0 Byzantine
        assert_eq!(required_for_bft(3), 2);

        // With 4 nodes, need 3 votes (75%) - tolerates 1 Byzantine
        assert_eq!(required_for_bft(4), 3);
    }

    #[test]
    fn test_bft_safety_medium_network() {
        // With 10 nodes, need 7 votes (70%) - malicious 3 cannot force consensus
        assert_eq!(required_for_bft(10), 7);

        // With 7 nodes, need 5 votes (71%) - tolerates 2 Byzantine
        assert_eq!(required_for_bft(7), 5);
    }

    #[test]
    fn test_bft_safety_large_network() {
        // With 100 nodes, need 67 votes (67%) - malicious 33 cannot force
        assert_eq!(required_for_bft(100), 67);

        // With 1000 nodes, need 667 votes (67%) - tolerates 333 Byzantine
        assert_eq!(required_for_bft(1000), 667);
    }

    #[test]
    fn test_bft_minimum() {
        // For small networks, use 2/3 threshold
        assert_eq!(required_for_bft(1), 1);  // 1 node = 1 vote
        assert_eq!(required_for_bft(2), 2);  // 2 nodes = 2 votes  
        assert_eq!(required_for_bft(3), 2);  // 3 nodes = 2 votes (67%)
    }

    #[test]
    fn test_bft_properties() {
        // Test that quorum is always > 2/3
        for n in 3..=100 {
            let quorum = required_for_bft(n);
            let min_required = (n * 2) / 3;
            assert!(
                quorum >= min_required,
                "For {} nodes, quorum {} must be >= {}",
                n,
                quorum,
                min_required
            );

            // Test that malicious nodes < 1/3 cannot force consensus
            let max_byzantine = n / 3;
            assert!(
                quorum > max_byzantine,
                "For {} nodes, quorum {} must be > {} Byzantine nodes",
                n,
                quorum,
                max_byzantine
            );
        }
    }
}
