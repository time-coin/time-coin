# Treasury Developer Integration Guide

## Table of Contents
1. [Quick Start](#quick-start)
2. [Integration Patterns](#integration-patterns)
3. [Code Examples](#code-examples)
4. [Testing Guide](#testing-guide)
5. [Best Practices](#best-practices)
6. [Troubleshooting](#troubleshooting)

## Quick Start

### Prerequisites

```toml
# Add to Cargo.toml
[dependencies]
treasury = { path = "../treasury" }
time-core = { path = "../core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Basic Usage

```rust
use time_core::treasury_manager::{TreasuryManager, CreateProposalParams, VoteChoice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create treasury manager
    let mut manager = TreasuryManager::new();
    
    // 2. Set total voting power (sum of all masternode power)
    manager.set_total_voting_power(1000);
    
    // 3. Create a proposal
    let params = CreateProposalParams {
        id: "my-first-proposal".to_string(),
        title: "Developer Grant".to_string(),
        description: "Funding for new feature development".to_string(),
        recipient: "time1recipient_address".to_string(),
        amount: 50_000 * 100_000_000,  // 50,000 TIME
        submitter: "time1submitter_address".to_string(),
        submission_time: 1699000000,
        voting_period_days: 14,
    };
    
    manager.create_proposal(params)?;
    
    // 4. Cast votes
    manager.vote_on_proposal(
        "my-first-proposal",
        "masternode-gold-1".to_string(),
        VoteChoice::Yes,
        100,  // Gold tier voting power
        1699100000,
    )?;
    
    // 5. Update proposal status after voting deadline
    manager.update_proposals(1699200000)?;
    
    // 6. Execute if approved
    if let Some(proposal) = manager.get_proposal("my-first-proposal") {
        if proposal.status == ProposalStatus::Approved {
            manager.execute_proposal(
                "my-first-proposal",
                12500,  // Block number
                1699300000,  // Execution time
            )?;
        }
    }
    
    Ok(())
}
```

## Integration Patterns

### Pattern 1: Simple Proposal Management

For applications that need to manage treasury proposals:

```rust
use time_core::treasury_manager::{TreasuryManager, CreateProposalParams, VoteChoice};
use std::collections::HashMap;

pub struct ProposalService {
    manager: TreasuryManager,
}

impl ProposalService {
    pub fn new() -> Self {
        Self {
            manager: TreasuryManager::new(),
        }
    }
    
    pub fn initialize(&mut self, total_voting_power: u64) {
        self.manager.set_total_voting_power(total_voting_power);
    }
    
    pub fn submit_proposal(
        &mut self,
        id: String,
        title: String,
        description: String,
        recipient: String,
        amount: u64,
        submitter: String,
    ) -> Result<ProposalInfo, String> {
        let params = CreateProposalParams {
            id: id.clone(),
            title: title.clone(),
            description,
            recipient,
            amount,
            submitter,
            submission_time: current_timestamp(),
            voting_period_days: 14,
        };
        
        self.manager.create_proposal(params)
            .map_err(|e| format!("Failed to create proposal: {:?}", e))?;
        
        Ok(ProposalInfo {
            id,
            title,
            status: "Active".to_string(),
            voting_deadline: current_timestamp() + (14 * 86400),
            execution_deadline: current_timestamp() + (44 * 86400),
        })
    }
    
    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        masternode_id: String,
        vote: VoteChoice,
        voting_power: u64,
    ) -> Result<(), String> {
        self.manager.vote_on_proposal(
            proposal_id,
            masternode_id,
            vote,
            voting_power,
            current_timestamp(),
        ).map_err(|e| format!("Failed to cast vote: {:?}", e))
    }
    
    pub fn get_proposal_status(&self, proposal_id: &str) -> Option<ProposalStatus> {
        self.manager.get_proposal(proposal_id)
            .map(|p| p.status.clone())
    }
    
    pub fn list_active_proposals(&self) -> Vec<ProposalSummary> {
        self.manager.get_active_proposals()
            .iter()
            .map(|p| ProposalSummary {
                id: p.id.clone(),
                title: p.title.clone(),
                amount: p.amount,
                votes_yes: p.count_yes_votes(),
                votes_no: p.count_no_votes(),
                approval_percentage: p.calculate_approval_percentage(),
            })
            .collect()
    }
    
    pub fn execute_approved_proposal(
        &mut self,
        proposal_id: &str,
        block_number: u64,
    ) -> Result<ExecutionResult, String> {
        let timestamp = current_timestamp();
        
        self.manager.execute_proposal(proposal_id, block_number, timestamp)
            .map_err(|e| format!("Failed to execute: {:?}", e))?;
        
        let proposal = self.manager.get_proposal(proposal_id)
            .ok_or("Proposal not found".to_string())?;
        
        Ok(ExecutionResult {
            proposal_id: proposal.id.clone(),
            amount: proposal.amount,
            recipient: proposal.recipient.clone(),
            block_number,
            timestamp,
        })
    }
    
    pub fn update_all_proposals(&mut self) -> Result<UpdateSummary, String> {
        let current_time = current_timestamp();
        
        self.manager.update_proposals(current_time)
            .map_err(|e| format!("Failed to update: {:?}", e))?;
        
        Ok(UpdateSummary {
            active: self.manager.count_proposals_by_status(ProposalStatus::Active),
            approved: self.manager.count_proposals_by_status(ProposalStatus::Approved),
            rejected: self.manager.count_proposals_by_status(ProposalStatus::Rejected),
            expired: self.manager.count_proposals_by_status(ProposalStatus::Expired),
            executed: self.manager.count_proposals_by_status(ProposalStatus::Executed),
        })
    }
}

// Helper types
#[derive(Debug, Clone)]
pub struct ProposalInfo {
    pub id: String,
    pub title: String,
    pub status: String,
    pub voting_deadline: u64,
    pub execution_deadline: u64,
}

#[derive(Debug, Clone)]
pub struct ProposalSummary {
    pub id: String,
    pub title: String,
    pub amount: u64,
    pub votes_yes: u64,
    pub votes_no: u64,
    pub approval_percentage: f64,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub proposal_id: String,
    pub amount: u64,
    pub recipient: String,
    pub block_number: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct UpdateSummary {
    pub active: usize,
    pub approved: usize,
    pub rejected: usize,
    pub expired: usize,
    pub executed: usize,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

### Pattern 2: Event-Driven Treasury Monitoring

For applications that need to react to treasury events:

```rust
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;

pub enum TreasuryEvent {
    ProposalCreated {
        id: String,
        title: String,
        amount: u64,
    },
    VoteCast {
        proposal_id: String,
        masternode_id: String,
        vote: VoteChoice,
    },
    ProposalApproved {
        id: String,
        amount: u64,
        approval_percentage: f64,
    },
    ProposalRejected {
        id: String,
        approval_percentage: f64,
    },
    ProposalExecuted {
        id: String,
        amount: u64,
        recipient: String,
    },
    ProposalExpired {
        id: String,
    },
}

pub struct TreasuryMonitor {
    manager: TreasuryManager,
    event_sender: Sender<TreasuryEvent>,
    last_checked: HashMap<String, ProposalStatus>,
}

impl TreasuryMonitor {
    pub fn new() -> (Self, Receiver<TreasuryEvent>) {
        let (sender, receiver) = channel();
        
        (
            Self {
                manager: TreasuryManager::new(),
                event_sender: sender,
                last_checked: HashMap::new(),
            },
            receiver
        )
    }
    
    pub fn start_monitoring(&mut self) {
        loop {
            self.check_for_changes();
            thread::sleep(Duration::from_secs(60));
        }
    }
    
    fn check_for_changes(&mut self) {
        let current_time = current_timestamp();
        
        // Update all proposals
        if let Err(e) = self.manager.update_proposals(current_time) {
            eprintln!("Error updating proposals: {:?}", e);
            return;
        }
        
        // Check each proposal for status changes
        for proposal in self.manager.get_all_proposals() {
            let last_status = self.last_checked.get(&proposal.id);
            
            if last_status.is_none() {
                // New proposal
                self.emit_event(TreasuryEvent::ProposalCreated {
                    id: proposal.id.clone(),
                    title: proposal.title.clone(),
                    amount: proposal.amount,
                });
            } else if Some(&proposal.status) != last_status {
                // Status changed
                match proposal.status {
                    ProposalStatus::Approved => {
                        self.emit_event(TreasuryEvent::ProposalApproved {
                            id: proposal.id.clone(),
                            amount: proposal.amount,
                            approval_percentage: proposal.calculate_approval_percentage(),
                        });
                    },
                    ProposalStatus::Rejected => {
                        self.emit_event(TreasuryEvent::ProposalRejected {
                            id: proposal.id.clone(),
                            approval_percentage: proposal.calculate_approval_percentage(),
                        });
                    },
                    ProposalStatus::Executed => {
                        self.emit_event(TreasuryEvent::ProposalExecuted {
                            id: proposal.id.clone(),
                            amount: proposal.amount,
                            recipient: proposal.recipient.clone(),
                        });
                    },
                    ProposalStatus::Expired => {
                        self.emit_event(TreasuryEvent::ProposalExpired {
                            id: proposal.id.clone(),
                        });
                    },
                    _ => {}
                }
            }
            
            self.last_checked.insert(proposal.id.clone(), proposal.status.clone());
        }
    }
    
    fn emit_event(&self, event: TreasuryEvent) {
        if let Err(e) = self.event_sender.send(event) {
            eprintln!("Failed to emit event: {:?}", e);
        }
    }
}

// Usage example
fn main() {
    let (mut monitor, receiver) = TreasuryMonitor::new();
    
    // Start monitoring in separate thread
    thread::spawn(move || {
        monitor.start_monitoring();
    });
    
    // Handle events in main thread
    for event in receiver {
        match event {
            TreasuryEvent::ProposalCreated { id, title, amount } => {
                println!("New proposal: {} - {} ({}TIME)", id, title, amount / 100_000_000);
            },
            TreasuryEvent::ProposalApproved { id, amount, approval_percentage } => {
                println!("Proposal {} approved with {:.1}% approval", id, approval_percentage);
                // Trigger execution or notify submitter
            },
            TreasuryEvent::ProposalRejected { id, approval_percentage } => {
                println!("Proposal {} rejected with {:.1}% approval", id, approval_percentage);
            },
            TreasuryEvent::ProposalExecuted { id, amount, recipient } => {
                println!("Proposal {} executed: {} TIME → {}", id, amount / 100_000_000, recipient);
            },
            TreasuryEvent::ProposalExpired { id } => {
                println!("Proposal {} expired without execution", id);
            },
            _ => {}
        }
    }
}
```

### Pattern 3: REST API Integration

For web applications that need to expose treasury functionality:

```rust
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    treasury: Arc<RwLock<ProposalService>>,
}

// API request/response types
#[derive(Deserialize)]
struct CreateProposalRequest {
    id: String,
    title: String,
    description: String,
    recipient: String,
    amount: u64,
    submitter: String,
}

#[derive(Serialize)]
struct ProposalResponse {
    id: String,
    title: String,
    description: String,
    amount: u64,
    amount_time: f64,
    status: String,
    voting_deadline: u64,
    execution_deadline: u64,
}

#[derive(Deserialize)]
struct VoteRequest {
    proposal_id: String,
    masternode_id: String,
    vote_choice: String,  // "yes", "no", "abstain"
    voting_power: u64,
}

#[derive(Serialize)]
struct VoteResponse {
    success: bool,
    message: String,
}

// API handlers
async fn create_proposal(
    State(state): State<AppState>,
    Json(request): Json<CreateProposalRequest>,
) -> Json<Result<ProposalResponse, String>> {
    let mut treasury = state.treasury.write().await;
    
    let result = treasury.submit_proposal(
        request.id.clone(),
        request.title.clone(),
        request.description.clone(),
        request.recipient,
        request.amount,
        request.submitter,
    );
    
    Json(result.map(|info| ProposalResponse {
        id: info.id,
        title: info.title,
        description: request.description,
        amount: request.amount,
        amount_time: request.amount as f64 / 100_000_000.0,
        status: info.status,
        voting_deadline: info.voting_deadline,
        execution_deadline: info.execution_deadline,
    }))
}

async fn cast_vote(
    State(state): State<AppState>,
    Json(request): Json<VoteRequest>,
) -> Json<VoteResponse> {
    let mut treasury = state.treasury.write().await;
    
    let vote_choice = match request.vote_choice.as_str() {
        "yes" => VoteChoice::Yes,
        "no" => VoteChoice::No,
        "abstain" => VoteChoice::Abstain,
        _ => return Json(VoteResponse {
            success: false,
            message: "Invalid vote choice".to_string(),
        }),
    };
    
    match treasury.cast_vote(
        &request.proposal_id,
        request.masternode_id,
        vote_choice,
        request.voting_power,
    ) {
        Ok(_) => Json(VoteResponse {
            success: true,
            message: "Vote cast successfully".to_string(),
        }),
        Err(e) => Json(VoteResponse {
            success: false,
            message: e,
        }),
    }
}

async fn list_proposals(
    State(state): State<AppState>,
) -> Json<Vec<ProposalSummary>> {
    let treasury = state.treasury.read().await;
    Json(treasury.list_active_proposals())
}

async fn get_proposal_details(
    State(state): State<AppState>,
    Json(proposal_id): Json<String>,
) -> Json<Option<ProposalResponse>> {
    let treasury = state.treasury.read().await;
    
    let proposal = treasury.manager.get_proposal(&proposal_id)?;
    
    Json(Some(ProposalResponse {
        id: proposal.id,
        title: proposal.title,
        description: proposal.description,
        amount: proposal.amount,
        amount_time: proposal.amount as f64 / 100_000_000.0,
        status: format!("{:?}", proposal.status),
        voting_deadline: proposal.voting_deadline,
        execution_deadline: proposal.execution_deadline,
    }))
}

// Build router
fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/proposals", get(list_proposals))
        .route("/proposals/create", post(create_proposal))
        .route("/proposals/details", post(get_proposal_details))
        .route("/proposals/vote", post(cast_vote))
        .with_state(state)
}

// Main server
#[tokio::main]
async fn main() {
    let service = ProposalService::new();
    let state = AppState {
        treasury: Arc::new(RwLock::new(service)),
    };
    
    let app = build_router(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    println!("Treasury API listening on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await.unwrap();
}
```

## Code Examples

### Example 1: Complete Proposal Lifecycle

```rust
use time_core::treasury_manager::{TreasuryManager, CreateProposalParams, VoteChoice};

fn example_complete_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = TreasuryManager::new();
    manager.set_total_voting_power(1000);
    
    // Phase 1: Create proposal
    println!("=== Phase 1: Creating Proposal ===");
    let params = CreateProposalParams {
        id: "website-redesign-2024".to_string(),
        title: "TIME Coin Website Redesign".to_string(),
        description: "Modern responsive design with improved UX".to_string(),
        recipient: "time1designer_team".to_string(),
        amount: 30_000 * 100_000_000,  // 30,000 TIME
        submitter: "time1community_member".to_string(),
        submission_time: 1699000000,
        voting_period_days: 14,
    };
    
    manager.create_proposal(params)?;
    println!("✓ Proposal created: website-redesign-2024");
    
    // Phase 2: Cast votes
    println!("\n=== Phase 2: Voting Period ===");
    
    // Gold masternodes vote
    for i in 1..=5 {
        let vote = if i <= 4 { VoteChoice::Yes } else { VoteChoice::No };
        manager.vote_on_proposal(
            "website-redesign-2024",
            format!("masternode-gold-{}", i),
            vote,
            100,
            1699000000 + (i * 10000),
        )?;
        println!("✓ Vote cast: masternode-gold-{} → {:?}", i, vote);
    }
    
    // Silver masternodes vote
    for i in 1..=20 {
        let vote = if i <= 15 { VoteChoice::Yes } else { VoteChoice::No };
        manager.vote_on_proposal(
            "website-redesign-2024",
            format!("masternode-silver-{}", i),
            vote,
            10,
            1699000000 + (50000 + i * 1000),
        )?;
    }
    println!("✓ 20 silver masternode votes cast");
    
    // Bronze masternodes vote
    for i in 1..=50 {
        let vote = if i <= 35 { VoteChoice::Yes } else { VoteChoice::No };
        manager.vote_on_proposal(
            "website-redesign-2024",
            format!("masternode-bronze-{}", i),
            vote,
            1,
            1699000000 + (100000 + i * 500),
        )?;
    }
    println!("✓ 50 bronze masternode votes cast");
    
    // Phase 3: Check voting results
    println!("\n=== Phase 3: Voting Results ===");
    let proposal = manager.get_proposal("website-redesign-2024")
        .expect("Proposal should exist");
    
    let results = proposal.calculate_results();
    println!("YES votes:  {} power", results.yes_power);
    println!("NO votes:   {} power", results.no_power);
    println!("Approval:   {:.1}%", results.approval_percentage());
    
    // Phase 4: Update status after voting deadline
    println!("\n=== Phase 4: Status Update ===");
    let voting_deadline = 1699000000 + (14 * 86400);
    manager.update_proposals(voting_deadline + 1)?;
    
    let proposal = manager.get_proposal("website-redesign-2024")
        .expect("Proposal should exist");
    println!("Status: {:?}", proposal.status);
    
    // Phase 5: Execute if approved
    if proposal.status == ProposalStatus::Approved {
        println!("\n=== Phase 5: Execution ===");
        manager.execute_proposal(
            "website-redesign-2024",
            12500,
            voting_deadline + 86400,
        )?;
        println!("✓ Proposal executed successfully");
        
        let proposal = manager.get_proposal("website-redesign-2024")
            .expect("Proposal should exist");
        println!("Final status: {:?}", proposal.status);
    }
    
    Ok(())
}
```

### Example 2: Batch Voting

```rust
fn example_batch_voting(
    manager: &mut TreasuryManager,
    proposal_id: &str,
    masternodes: Vec<(String, u64)>,  // (id, voting_power)
    vote: VoteChoice,
    timestamp: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Batch voting: {} masternodes", masternodes.len());
    
    let mut successful = 0;
    let mut failed = 0;
    
    for (mn_id, power) in masternodes {
        match manager.vote_on_proposal(
            proposal_id,
            mn_id.clone(),
            vote,
            power,
            timestamp,
        ) {
            Ok(_) => {
                successful += 1;
                println!("  ✓ {}: {} power", mn_id, power);
            },
            Err(e) => {
                failed += 1;
                eprintln!("  ✗ {}: {:?}", mn_id, e);
            }
        }
    }
    
    println!("\nBatch results:");
    println!("  Successful: {}", successful);
    println!("  Failed: {}", failed);
    
    Ok(())
}

// Usage
fn main() {
    let mut manager = TreasuryManager::new();
    manager.set_total_voting_power(1000);
    
    // Create a test proposal
    // ... (omitted for brevity)
    
    // Prepare masternode list
    let gold_nodes: Vec<(String, u64)> = (1..=10)
        .map(|i| (format!("mn-gold-{}", i), 100))
        .collect();
    
    let silver_nodes: Vec<(String, u64)> = (1..=50)
        .map(|i| (format!("mn-silver-{}", i), 10))
        .collect();
    
    // Batch vote
    example_batch_voting(
        &mut manager,
        "test-proposal",
        gold_nodes,
        VoteChoice::Yes,
        1699000000,
    ).unwrap();
    
    example_batch_voting(
        &mut manager,
        "test-proposal",
        silver_nodes,
        VoteChoice::Yes,
        1699010000,
    ).unwrap();
}
```

### Example 3: Proposal Analytics

```rust
use std::collections::HashMap;

struct ProposalAnalytics {
    manager: TreasuryManager,
}

impl ProposalAnalytics {
    fn new(manager: TreasuryManager) -> Self {
        Self { manager }
    }
    
    fn approval_rate_by_amount(&self) -> HashMap<String, f64> {
        let proposals = self.manager.get_all_proposals();
        
        let mut ranges = HashMap::new();
        ranges.insert("0-10k".to_string(), vec![]);
        ranges.insert("10k-50k".to_string(), vec![]);
        ranges.insert("50k-100k".to_string(), vec![]);
        ranges.insert("100k+".to_string(), vec![]);
        
        for proposal in proposals {
            let amount_time = proposal.amount / 100_000_000;
            let results = proposal.calculate_results();
            let approval = results.approval_percentage();
            
            let range = if amount_time < 10_000 {
                "0-10k"
            } else if amount_time < 50_000 {
                "10k-50k"
            } else if amount_time < 100_000 {
                "50k-100k"
            } else {
                "100k+"
            };
            
            if let Some(list) = ranges.get_mut(range) {
                list.push(approval);
            }
        }
        
        // Calculate average approval for each range
        ranges.iter()
            .map(|(range, approvals)| {
                let avg = if approvals.is_empty() {
                    0.0
                } else {
                    approvals.iter().sum::<f64>() / approvals.len() as f64
                };
                (range.clone(), avg)
            })
            .collect()
    }
    
    fn participation_rate(&self) -> f64 {
        let proposals = self.manager.get_all_proposals();
        let total_power = self.manager.get_total_voting_power();
        
        if proposals.is_empty() || total_power == 0 {
            return 0.0;
        }
        
        let total_participation: u64 = proposals.iter()
            .map(|p| p.total_voting_power)
            .sum();
        
        let avg_participation = total_participation / proposals.len() as u64;
        
        (avg_participation as f64 / total_power as f64) * 100.0
    }
    
    fn success_rate(&self) -> f64 {
        let proposals = self.manager.get_all_proposals();
        
        if proposals.is_empty() {
            return 0.0;
        }
        
        let successful = proposals.iter()
            .filter(|p| {
                matches!(p.status, ProposalStatus::Approved | ProposalStatus::Executed)
            })
            .count();
        
        (successful as f64 / proposals.len() as f64) * 100.0
    }
    
    fn print_report(&self) {
        println!("=== Treasury Proposal Analytics ===\n");
        
        println!("Overall Statistics:");
        println!("  Success Rate:       {:.1}%", self.success_rate());
        println!("  Participation Rate: {:.1}%", self.participation_rate());
        println!();
        
        println!("Approval Rate by Amount:");
        let rates = self.approval_rate_by_amount();
        for (range, rate) in rates {
            println!("  {}: {:.1}%", range, rate);
        }
        println!();
        
        println!("Status Breakdown:");
        let statuses = self.manager.count_all_proposal_statuses();
        for (status, count) in statuses {
            println!("  {:?}: {}", status, count);
        }
    }
}

// Usage
fn main() {
    let mut manager = TreasuryManager::new();
    // ... populate with proposals ...
    
    let analytics = ProposalAnalytics::new(manager);
    analytics.print_report();
}
```

## Testing Guide

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proposal_creation() {
        let mut manager = TreasuryManager::new();
        manager.set_total_voting_power(1000);
        
        let params = CreateProposalParams {
            id: "test-proposal".to_string(),
            title: "Test".to_string(),
            description: "Test description".to_string(),
            recipient: "time1test".to_string(),
            amount: 1000 * 100_000_000,
            submitter: "time1submitter".to_string(),
            submission_time: 1000000,
            voting_period_days: 14,
        };
        
        let result = manager.create_proposal(params);
        assert!(result.is_ok());
        
        let proposal = manager.get_proposal("test-proposal");
        assert!(proposal.is_some());
        assert_eq!(proposal.unwrap().status, ProposalStatus::Active);
    }
    
    #[test]
    fn test_voting() {
        let mut manager = TreasuryManager::new();
        manager.set_total_voting_power(1000);
        
        // Create proposal
        let params = CreateProposalParams {
            id: "test-voting".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            recipient: "time1test".to_string(),
            amount: 1000 * 100_000_000,
            submitter: "time1submitter".to_string(),
            submission_time: 1000000,
            voting_period_days: 14,
        };
        manager.create_proposal(params).unwrap();
        
        // Cast vote
        let result = manager.vote_on_proposal(
            "test-voting",
            "mn-1".to_string(),
            VoteChoice::Yes,
            100,
            1000000,
        );
        assert!(result.is_ok());
        
        // Verify vote recorded
        let proposal = manager.get_proposal("test-voting").unwrap();
        assert_eq!(proposal.votes.len(), 1);
        assert!(proposal.votes.contains_key("mn-1"));
    }
    
    #[test]
    fn test_approval_threshold() {
        let mut manager = TreasuryManager::new();
        manager.set_total_voting_power(1000);
        
        // Create and vote on proposal
        let params = CreateProposalParams {
            id: "test-approval".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            recipient: "time1test".to_string(),
            amount: 1000 * 100_000_000,
            submitter: "time1submitter".to_string(),
            submission_time: 1000000,
            voting_period_days: 14,
        };
        manager.create_proposal(params).unwrap();
        
        // 670 YES, 330 NO = 67% exactly
        manager.vote_on_proposal("test-approval", "mn-yes".to_string(), VoteChoice::Yes, 670, 1000000).unwrap();
        manager.vote_on_proposal("test-approval", "mn-no".to_string(), VoteChoice::No, 330, 1000001).unwrap();
        
        // Update status after deadline
        let deadline = 1000000 + (14 * 86400) + 1;
        manager.update_proposals(deadline).unwrap();
        
        // Should be approved at exactly 67%
        let proposal = manager.get_proposal("test-approval").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
    }
    
    #[test]
    fn test_execution_deadline() {
        let mut manager = TreasuryManager::new();
        manager.set_total_voting_power(1000);
        
        // Create and approve proposal
        let params = CreateProposalParams {
            id: "test-expiry".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            recipient: "time1test".to_string(),
            amount: 1000 * 100_000_000,
            submitter: "time1submitter".to_string(),
            submission_time: 1000000,
            voting_period_days: 14,
        };
        manager.create_proposal(params).unwrap();
        
        // Vote and approve
        manager.vote_on_proposal("test-expiry", "mn-yes".to_string(), VoteChoice::Yes, 1000, 1000000).unwrap();
        
        let voting_deadline = 1000000 + (14 * 86400);
        manager.update_proposals(voting_deadline + 1).unwrap();
        
        // Verify approved
        let proposal = manager.get_proposal("test-expiry").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
        
        // Try to execute after execution deadline
        let execution_deadline = voting_deadline + (30 * 86400);
        let result = manager.execute_proposal("test-expiry", 100, execution_deadline + 1);
        
        // Should fail
        assert!(result.is_err());
        
        // Status should be Expired
        let proposal = manager.get_proposal("test-expiry").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Expired);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_full_proposal_lifecycle() {
        let mut manager = TreasuryManager::new();
        manager.set_total_voting_power(1000);
        
        // Initial treasury balance
        manager.allocate_funds(1_000_000 * 100_000_000);
        
        // 1. Create proposal
        let params = CreateProposalParams {
            id: "integration-test".to_string(),
            title: "Integration Test".to_string(),
            description: "Full lifecycle test".to_string(),
            recipient: "time1recipient".to_string(),
            amount: 50_000 * 100_000_000,
            submitter: "time1submitter".to_string(),
            submission_time: 1000000,
            voting_period_days: 14,
        };
        manager.create_proposal(params).unwrap();
        
        // 2. Cast votes (70% approval)
        manager.vote_on_proposal("integration-test", "mn-yes-1".to_string(), VoteChoice::Yes, 700, 1000100).unwrap();
        manager.vote_on_proposal("integration-test", "mn-no-1".to_string(), VoteChoice::No, 300, 1000200).unwrap();
        
        // 3. Update after voting deadline
        let voting_deadline = 1000000 + (14 * 86400);
        manager.update_proposals(voting_deadline + 1).unwrap();
        
        let proposal = manager.get_proposal("integration-test").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Approved);
        
        // 4. Execute
        let initial_balance = manager.get_balance();
        manager.execute_proposal("integration-test", 100, voting_deadline + 86400).unwrap();
        
        // Verify execution
        let proposal = manager.get_proposal("integration-test").unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
        
        let final_balance = manager.get_balance();
        assert_eq!(initial_balance - final_balance, 50_000 * 100_000_000);
    }
}
```

## Best Practices

### 1. Always Validate Input

```rust
fn validate_proposal_params(params: &CreateProposalParams) -> Result<(), String> {
    // Check ID format
    if !params.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid proposal ID format".to_string());
    }
    
    // Check title length
    if params.title.is_empty() || params.title.len() > 100 {
        return Err("Title must be 1-100 characters".to_string());
    }
    
    // Check amount is positive
    if params.amount == 0 {
        return Err("Amount must be positive".to_string());
    }
    
    // Check recipient address format
    if !params.recipient.starts_with("time1") {
        return Err("Invalid recipient address".to_string());
    }
    
    Ok(())
}
```

### 2. Handle Errors Gracefully

```rust
fn safe_vote_cast(
    manager: &mut TreasuryManager,
    proposal_id: &str,
    masternode_id: String,
    vote: VoteChoice,
    power: u64,
) -> Result<String, String> {
    match manager.vote_on_proposal(proposal_id, masternode_id.clone(), vote, power, current_timestamp()) {
        Ok(_) => Ok(format!("Vote cast successfully by {}", masternode_id)),
        Err(e) => {
            // Log error
            eprintln!("Vote failed: {:?}", e);
            
            // Return user-friendly message
            Err(match e {
                StateError::IoError(msg) if msg.contains("already voted") => {
                    "You have already voted on this proposal".to_string()
                },
                StateError::IoError(msg) if msg.contains("Voting period ended") => {
                    "Voting period has ended for this proposal".to_string()
                },
                _ => "Failed to cast vote. Please try again.".to_string(),
            })
        }
    }
}
```

### 3. Use Transactions for Batch Operations

```rust
fn batch_proposal_update(
    manager: &mut TreasuryManager,
    current_time: u64,
) -> Result<BatchUpdateResult, String> {
    // Save state in case of error
    let saved_state = manager.clone();
    
    match manager.update_proposals(current_time) {
        Ok(_) => {
            // Collect results
            Ok(BatchUpdateResult {
                updated: manager.count_total_proposals(),
                errors: vec![],
            })
        },
        Err(e) => {
            // Restore state
            *manager = saved_state;
            Err(format!("Batch update failed: {:?}", e))
        }
    }
}
```

### 4. Monitor Proposal Deadlines

```rust
fn check_upcoming_deadlines(manager: &TreasuryManager) -> Vec<DeadlineAlert> {
    let current_time = current_timestamp();
    let one_day = 86400u64;
    
    manager.get_all_proposals()
        .iter()
        .filter_map(|p| {
            if p.status == ProposalStatus::Active {
                let time_until_voting = p.voting_deadline.saturating_sub(current_time);
                if time_until_voting < one_day {
                    return Some(DeadlineAlert {
                        proposal_id: p.id.clone(),
                        alert_type: AlertType::VotingEnding,
                        hours_remaining: (time_until_voting / 3600) as u32,
                    });
                }
            } else if p.status == ProposalStatus::Approved {
                let time_until_exec = p.execution_deadline.saturating_sub(current_time);
                if time_until_exec < one_day {
                    return Some(DeadlineAlert {
                        proposal_id: p.id.clone(),
                        alert_type: AlertType::ExecutionExpiring,
                        hours_remaining: (time_until_exec / 3600) as u32,
                    });
                }
            }
            None
        })
        .collect()
}
```

## Troubleshooting

### Common Issues

#### Issue: "Proposal already exists"

```rust
// Check before creating
if manager.get_proposal(&proposal_id).is_some() {
    return Err("Proposal with this ID already exists");
}

// Use unique IDs with timestamps
let unique_id = format!("proposal-{}-{}", title_slug, unix_timestamp());
```

#### Issue: "Masternode already voted"

```rust
// Check before voting
let proposal = manager.get_proposal(proposal_id)
    .ok_or("Proposal not found")?;
    
if proposal.votes.contains_key(&masternode_id) {
    return Err("This masternode has already voted");
}
```

#### Issue: "Insufficient treasury balance"

```rust
// Check before execution
let treasury_balance = manager.get_balance();
let proposal_amount = proposal.amount;

if treasury_balance < proposal_amount {
    return Err(format!(
        "Insufficient balance: {} TIME available, {} TIME required",
        treasury_balance / 100_000_000,
        proposal_amount / 100_000_000
    ));
}
```

### Debug Helpers

```rust
fn debug_proposal_state(manager: &TreasuryManager, proposal_id: &str) {
    if let Some(proposal) = manager.get_proposal(proposal_id) {
        println!("Proposal Debug Info:");
        println!("  ID: {}", proposal.id);
        println!("  Status: {:?}", proposal.status);
        println!("  Amount: {} TIME", proposal.amount / 100_000_000);
        println!("  Voting Deadline: {}", proposal.voting_deadline);
        println!("  Execution Deadline: {}", proposal.execution_deadline);
        println!("  Total Votes: {}", proposal.votes.len());
        
        let results = proposal.calculate_results();
        println!("  YES Power: {}", results.yes_power);
        println!("  NO Power: {}", results.no_power);
        println!("  Approval: {:.2}%", results.approval_percentage());
    } else {
        println!("Proposal not found: {}", proposal_id);
    }
}
```

---

**Document Version:** 1.0  
**Last Updated:** November 2024  
**Status:** Active
