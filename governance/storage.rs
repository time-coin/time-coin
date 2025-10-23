use std::collections::{HashMap, BTreeMap};
use super::tx::{Address, Coins};
use super::committee::{PublicKey, MasternodeTier};
use super::tx::{VoteChoice};
use crate::governance::tx::Grant;

/// A single cast vote (normalized form) for persistence.
#[derive(Clone, Debug)]
pub struct StoredVote {
    pub voter: PublicKey,
    pub tier: MasternodeTier,
    pub choice: VoteChoice,
    pub ts: u64,
}

/// Abstraction for governance persistence.
/// Swap this out for your chain runtime, sled/rocks, or DB-backed store.
pub trait GovernanceStore: Send + Sync {
    // Grants
    fn put_grant(&mut self, g: Grant);
    fn get_grant(&self, id: u64) -> Option<Grant>;
    fn list_grants(&self, offset: usize, limit: usize) -> Vec<Grant>;
    fn update_grant(&mut self, g: Grant);

    // Votes
    fn add_or_replace_vote(&mut self, grant_id: u64, v: StoredVote);
    fn get_votes(&self, grant_id: u64) -> Vec<StoredVote>;
    fn delete_votes(&mut self, grant_id: u64);

    // Simple counters
    fn next_grant_id(&mut self) -> u64;
}

/// Simple in-memory store (good for testing and initial wiring).
#[derive(Default)]
pub struct InMemoryGovernance {
    next_id: u64,
    grants: BTreeMap<u64, Grant>,
    votes: HashMap<u64, Vec<StoredVote>>,
}

impl InMemoryGovernance {
    pub fn new() -> Self { Self::default() }
}

impl GovernanceStore for InMemoryGovernance {
    fn put_grant(&mut self, g: Grant) {
        self.grants.insert(g.id, g);
    }

    fn get_grant(&self, id: u64) -> Option<Grant> {
        self.grants.get(&id).cloned()
    }

    fn list_grants(&self, offset: usize, limit: usize) -> Vec<Grant> {
        let mut v: Vec<_> = self.grants.values().cloned().collect();
        v.sort_by_key(|g| g.id);
        v.into_iter().skip(offset).take(limit).collect()
    }

    fn update_grant(&mut self, g: Grant) {
        self.grants.insert(g.id, g);
    }

    fn add_or_replace_vote(&mut self, grant_id: u64, v: StoredVote) {
        let entry = self.votes.entry(grant_id).or_default();
        if let Some(existing) = entry.iter_mut().find(|e| e.voter == v.voter) {
            *existing = v;
        } else {
            entry.push(v);
        }
    }

    fn get_votes(&self, grant_id: u64) -> Vec<StoredVote> {
        self.votes.get(&grant_id).cloned().unwrap_or_default()
    }

    fn delete_votes(&mut self, grant_id: u64) {
        self.votes.remove(&grant_id);
    }

    fn next_grant_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Example helper wiring for a "create proposal" flow using the store.
/// (You can move these into your runtime/handlers.)
pub mod helpers {
    use super::*;
    use crate::governance::constants::*;
    use crate::governance::tx::{Milestone, Grant, GrantStatus};

    pub fn create_proposal<S: GovernanceStore>(
        store: &mut S,
        title: String,
        summary: String,
        proposer: Address,
        recipient: Address,
        requested: Coins,
        milestones: Vec<(String, Coins, String)>,
        forum_uri: String,
        tags: Vec<String>,
        now: u64,
        voter_snapshot_epoch: u64,
    ) -> Result<u64, &'static str> {
        if milestones.is_empty() || milestones.len() as u8 > MAX_MILESTONES { return Err("bad_milestones"); }
        let sum: Coins = milestones.iter().map(|m| m.1).sum();
        if sum != requested { return Err("milestone_sum_mismatch"); }

        let id = store.next_grant_id();
        let grant = Grant {
            id,
            title,
            summary,
            proposer,
            recipient,
            requested,
            milestones: milestones.into_iter()
                .map(|(t,a,u)| Milestone { title: t, amount: a, released: false, evidence_uri: u })
                .collect(),
            status: GrantStatus::Voting {
                started_at: now,
                ends_at: now + VOTING_PERIOD_SECS,
                voter_snapshot_epoch,
            },
            created_at: now,
            updated_at: now,
            yes_votes: 0,
            no_votes: 0,
            forum_uri,
            tags,
        };
        store.put_grant(grant);
        Ok(id)
    }

    pub fn record_vote<S: GovernanceStore>(
        store: &mut S,
        grant_id: u64,
        voter: PublicKey,
        tier: MasternodeTier,
        choice: crate::governance::tx::VoteChoice,
        ts: u64,
    ) {
        store.add_or_replace_vote(grant_id, StoredVote { voter, tier, choice, ts });
    }
}
