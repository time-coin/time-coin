use ed25519_dalek::{Signature, VerifyingKey, signature::Verifier};
use super::committee::{EpochId, PublicKey};
use crate::governance::constants::TREASURY_DOMAIN;

pub type Coins = u128;

/// Treasury state bound to a specific Gold snapshot epoch.
/// `gold_pubkeys` are the Gold committee verifying keys (32-byte compressed Ed25519).
#[derive(Clone, Debug)]
pub struct Treasury {
    pub balance: Coins,
    pub gold_epoch: EpochId,
    pub gold_pubkeys: Vec<PublicKey>, // index by signer_idx for multisig bundles
}

/// Simple K-of-N multisig bundle over a single message.
/// Each (signer_idx, sig) claims that gold_pubkeys[signer_idx] signed the message.
#[derive(Clone, Debug)]
pub struct SigBundle {
    pub sigs: Vec<(u16, [u8; 64])>, // (signer_idx, sig)
    pub required: u16,               // typically ceil(2/3 * N)
}

impl Treasury {
    pub fn can_spend(&self, amount: Coins) -> bool {
        self.balance >= amount
    }
}

/// Verify K-of-N signatures with Ed25519 over a **single** message.
/// - Ensures signer indices are in range
/// - Ensures distinct signer indices (no duplicates)
/// - Verifies signatures; returns true if valid_count >= required
pub fn verify_sig_bundle(message: &[u8], bundle: &SigBundle, gold_pubkeys: &[PublicKey]) -> bool {
    if bundle.required == 0 { return false; }
    if bundle.sigs.is_empty() { return false; }

    use std::collections::HashSet;
    let mut seen: HashSet<u16> = HashSet::new();
    let mut valid: u16 = 0;

    for (idx, sig_bytes) in &bundle.sigs {
        let i = *idx as usize;
        if i >= gold_pubkeys.len() { return false; }             // out of range
        if !seen.insert(*idx) { continue; }                      // duplicate signer -> skip
        // Convert pubkey/signature
        let pk_bytes = &gold_pubkeys[i];
        let vk = match VerifyingKey::from_bytes(pk_bytes) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let sig = match Signature::from_bytes(sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        if vk.verify(message, &sig).is_ok() {
            valid = valid.saturating_add(1);
            if valid >= bundle.required { return true; }
        }
    }
    valid >= bundle.required
}

/// Build the domain-separated disbursal message:
///  TREASURY_DOMAIN || chain_id || ":DISBURSE:" || grant_id || idx || amount || recipient
pub fn disbursal_message(chain_id: &str, grant_id: u64, idx: u8, amount: Coins, recipient: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(64);
    v.extend_from_slice(TREASURY_DOMAIN.as_bytes());
    v.extend_from_slice(chain_id.as_bytes());
    v.extend_from_slice(b":DISBURSE:");
    v.extend_from_slice(&grant_id.to_be_bytes());
    v.push(idx);
    v.extend_from_slice(&amount.to_be_bytes());
    v.extend_from_slice(recipient);
    v
}
