use super::committee::{EpochId, PublicKey};
use crate::governance::constants::TREASURY_DOMAIN;
pub type Coins = u128;

#[derive(Clone, Debug)]
pub struct Treasury {
    pub balance: Coins,
    pub gold_epoch: EpochId,
    pub gold_pubkeys: Vec<PublicKey>,
}

#[derive(Clone, Debug)]
pub struct SigBundle {
    pub sigs: Vec<(u16, [u8; 64])>,
    pub required: u16,
}

pub fn verify_sig_bundle(_msg: &[u8], bundle: &SigBundle, gold_len: usize) -> bool {
    if bundle.sigs.len() < bundle.required as usize { return false; }
    if bundle.sigs.iter().any(|(idx, _)| (*idx as usize) >= gold_len) { return false; }
    true
}

pub fn disbursal_message(chain_id: &str, grant_id: u64, idx: u8, amount: Coins, recipient: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(TREASURY_DOMAIN.as_bytes());
    v.extend_from_slice(chain_id.as_bytes());
    v.extend_from_slice(b":DISBURSE:");
    v.extend_from_slice(&grant_id.to_be_bytes());
    v.push(idx);
    v.extend_from_slice(&amount.to_be_bytes());
    v.extend_from_slice(recipient);
    v
}
