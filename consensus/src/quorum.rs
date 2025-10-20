//! Quorum calculation for TIME Coin consensus
pub const MIN_MASTERNODES: usize = 4;
pub const GRACE_PERIOD_SECS: i64 = 1800;
pub fn calculate_quorum(total_nodes: usize) -> usize {
    if total_nodes < MIN_MASTERNODES { return total_nodes; }
    (total_nodes * 2 / 3) + 1
}
pub fn has_quorum(active_nodes: usize) -> bool {
    active_nodes >= MIN_MASTERNODES
}
