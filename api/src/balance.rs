/// Shared utilities for balance calculations
use time_core::state::BlockchainState;
use time_mempool::Mempool;

/// Calculate balance changes from unconfirmed mempool transactions
///
/// This function computes the net pending balance for an address by:
/// - Adding outputs sent to the address in pending transactions
/// - Subtracting inputs that spend the address's UTXOs
///
/// # Arguments
/// * `address` - The address to calculate pending balance for
/// * `blockchain` - Reference to the blockchain state (for UTXO set access)
/// * `mempool` - Reference to the mempool containing pending transactions
///
/// # Returns
/// Net unconfirmed balance (received - spent) in satoshis
pub async fn calculate_mempool_balance(
    address: &str,
    blockchain: &BlockchainState,
    mempool: &Mempool,
) -> u64 {
    let mempool_txs = mempool.get_all_transactions().await;
    let utxo_set = blockchain.utxo_set();

    let mut pending_received = 0u64;
    let mut pending_spent = 0u64;

    for tx in mempool_txs {
        // Add outputs sent to this address
        for output in &tx.outputs {
            if output.address == address {
                pending_received = pending_received.saturating_add(output.amount);
            }
        }

        // Subtract inputs spending this address's UTXOs
        for input in &tx.inputs {
            if let Some(utxo) = utxo_set.get(&input.previous_output) {
                if utxo.address == address {
                    pending_spent = pending_spent.saturating_add(utxo.amount);
                }
            }
        }
    }

    // Net unconfirmed balance = received - spent
    pending_received.saturating_sub(pending_spent)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add unit tests for balance calculation logic
    // test_case: zero pending transactions -> should return 0
    // test_case: pending received only -> should return received amount
    // test_case: pending spent only -> should return 0 (no negative balance)
    // test_case: received > spent -> should return net positive
    // test_case: spent > received -> should return 0 (saturating sub)
}
