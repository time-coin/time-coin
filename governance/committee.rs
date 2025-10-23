use std::collections::HashSet;
pub type EpochId = u64;
pub type PublicKey = [u8; 32];

#[derive(Clone, Debug)]
pub enum MasternodeTier { Bronze, Silver, Gold }

#[derive(Clone, Debug)]
pub struct CommitteeMember {
    pub pubkey: PublicKey,
    pub tier: MasternodeTier,
}

#[derive(Clone, Debug)]
pub struct GoldCommitteeSnapshot {
    pub epoch_id: EpochId,
    pub voters: Vec<PublicKey>,
}

#[derive(Clone, Debug)]
pub struct VoterSnapshot {
    pub epoch_id: EpochId,
    pub eligible: Vec<CommitteeMember>,
}

impl VoterSnapshot {
    pub fn contains(&self, pk: &PublicKey) -> bool {
        self.eligible.iter().any(|m| &m.pubkey == pk)
    }
    pub fn gold_set(&self) -> HashSet<PublicKey> {
        self.eligible.iter()
            .filter(|m| matches!(m.tier, MasternodeTier::Gold))
            .map(|m| m.pubkey)
            .collect()
    }
}
