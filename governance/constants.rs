pub const GOVERNANCE_DOMAIN: &str = "TIMECOIN_GRANTS_V1";
pub const TREASURY_DOMAIN:  &str = "TIMECOIN_TREASURY_V1";

pub const WEIGHT_GOLD:   f64 = 1.00;
pub const WEIGHT_SILVER: f64 = 0.20;
pub const WEIGHT_BRONZE: f64 = 0.05;

pub const QUORUM_NETWORK:       f64 = 0.40;
pub const APPROVAL_THRESHOLD:   f64 = 0.55;
pub const GOLD_NO_OBJECTION:    f64 = 0.33;

pub const VOTING_PERIOD_SECS: u64 = 7 * 86_400;
pub const COOLDOWN_SECS:      u64 = 24 * 3_600;

pub const PROPOSAL_FEE: u64 = 1_000;
pub const MIN_MILESTONES: u8 = 1;
pub const MAX_MILESTONES: u8 = 12;
