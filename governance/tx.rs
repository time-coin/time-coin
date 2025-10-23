use super::committee::{MasternodeTier, VoterSnapshot, GoldCommitteeSnapshot, PublicKey};
use super::constants::*;
use super::treasury::{Treasury, SigBundle, verify_sig_bundle, disbursal_message};

pub type Address = [u8; 20];
pub type Coins = u128;

#[derive(Clone, Debug)]
pub enum VoteChoice { Yes, No, Abstain }

#[derive(Clone, Debug)]
pub struct Vote {
    pub voter: PublicKey,
    pub tier: MasternodeTier,
    pub choice: VoteChoice,
}

pub fn weight_for_tier(t: &MasternodeTier) -> f64 {
    match t {
        MasternodeTier::Gold => WEIGHT_GOLD,
        MasternodeTier::Silver => WEIGHT_SILVER,
        MasternodeTier::Bronze => WEIGHT_BRONZE,
    }
}

pub struct Tally { pub total: f64, pub part: f64, pub yes: f64, pub gold_part: f64, pub gold_yes: f64 }

pub fn compute_tally(snapshot: &VoterSnapshot, votes: &[Vote]) -> Tally {
    let mut t = Tally { total: 0.0, part: 0.0, yes: 0.0, gold_part: 0.0, gold_yes: 0.0 };
    for m in &snapshot.eligible { t.total += weight_for_tier(&m.tier); }
    for v in votes {
        match v.choice {
            VoteChoice::Yes | VoteChoice::No => {
                let w = weight_for_tier(&v.tier);
                t.part += w;
                if matches!(v.choice, VoteChoice::Yes) { t.yes += w; }
                if matches!(v.tier, MasternodeTier::Gold) {
                    t.gold_part += w;
                    if matches!(v.choice, VoteChoice::Yes) { t.gold_yes += w; }
                }
            }
            VoteChoice::Abstain => {}
        }
    }
    t
}

pub fn passes(t: &Tally) -> bool {
    let quorum_ok = t.part >= QUORUM_NETWORK * t.total;
    let approval_ok = t.part > 0.0 && (t.yes / t.part) > APPROVAL_THRESHOLD;
    let gold_ok = t.gold_part == 0.0 || (t.gold_yes / t.gold_part) >= GOLD_NO_OBJECTION;
    quorum_ok && approval_ok && gold_ok
}
