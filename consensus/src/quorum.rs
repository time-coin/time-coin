//! Quorum calculation for TIME Coin consensus
pub const MIN_MASTERNODES: usize = 3; // Minimum for BFT consensus (tolerates 0 Byzantine failures)
pub const GRACE_PERIOD_SECS: i64 = 1800;

/// Fixed quorum size for instant finality (independent of network size)
/// This provides fast consensus even as the network grows
pub const FIXED_QUORUM_SIZE: usize = 3;

pub fn calculate_quorum(total_nodes: usize) -> usize {
    if total_nodes < MIN_MASTERNODES {
        return total_nodes;
    }
    // Use fixed quorum size for scalability
    FIXED_QUORUM_SIZE.min(total_nodes)
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

/// Calculate required votes for BFT consensus
/// Uses fixed quorum for scalability instead of 2/3 majority
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
