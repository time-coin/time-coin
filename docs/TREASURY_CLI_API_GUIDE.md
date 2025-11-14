# Treasury CLI and API Usage Guide

## Table of Contents
1. [CLI Commands](#cli-commands)
2. [API Endpoints](#api-endpoints)
3. [Usage Examples](#usage-examples)
4. [Integration Patterns](#integration-patterns)
5. [Troubleshooting](#troubleshooting)

## CLI Commands

### Current Implementation

The TIME Coin CLI provides RPC commands for treasury operations. These commands connect to a running `timed` node via RPC.

#### Get Treasury Information

**Command:**
```bash
time-cli rpc gettreasury
```

**Description:** Retrieve current treasury statistics including balance, allocations, and pending proposals.

**Example Output:**
```json
{
  "balance": 123456700000000,
  "balance_time": 1234567.0,
  "total_allocated": 234567800000000,
  "total_distributed": 111111100000000,
  "allocation_count": 15678,
  "withdrawal_count": 42,
  "pending_proposals": 3
}
```

**Human-Readable Format:**
```
Treasury Statistics
═══════════════════════════════════════════════
Current Balance:     1,234,567.00 TIME
Total Allocated:     2,345,678.00 TIME (lifetime)
Total Distributed:   1,111,111.00 TIME (lifetime)
───────────────────────────────────────────────
Allocations:         15,678 deposits
Withdrawals:         42 distributions
Pending Proposals:   3 awaiting decision
```

**Use Cases:**
- Monitor treasury health
- Verify fund availability before proposal submission
- Track lifetime treasury activity
- Dashboard integration

#### List Treasury Proposals

**Command:**
```bash
time-cli rpc listproposals
```

**Description:** List all treasury proposals with their current status and voting results.

**Example Output:**
```json
{
  "proposals": [
    {
      "id": "mobile-wallet-2024",
      "title": "iOS and Android Wallet Development",
      "amount": 7500000000000,
      "amount_time": 75000.0,
      "recipient": "time1dev_team_wallet...",
      "submitter": "time1dev_lead...",
      "status": "Active",
      "submission_time": 1699000000,
      "voting_deadline": 1700209600,
      "execution_deadline": 1702801600,
      "votes": {
        "total": 750,
        "yes": 640,
        "no": 110,
        "abstain": 0
      },
      "approval_percentage": 85.3
    },
    {
      "id": "security-audit-q4",
      "title": "Q4 2024 Security Audit",
      "amount": 2500000000000,
      "amount_time": 25000.0,
      "status": "Approved",
      "votes": {
        "total": 500,
        "yes": 350,
        "no": 150,
        "abstain": 0
      },
      "approval_percentage": 70.0
    }
  ]
}
```

**Human-Readable Format:**
```
Treasury Proposals
═══════════════════════════════════════════════

[1] mobile-wallet-2024
    Title: iOS and Android Wallet Development
    Amount: 75,000.00 TIME
    Status: Active (Voting Open)
    ───────────────────────────────────────────
    Voting Deadline: Nov 17, 2023 12:00 UTC
    Execution Deadline: Dec 17, 2023 12:00 UTC
    
    Votes:
      YES:     640 (85.3%)  ████████████████████████
      NO:      110 (14.7%)  ████
      ABSTAIN: 0   (0.0%)   
      
    Total Voting Power: 750
    Approval: 85.3% ✓ (≥67% required)

[2] security-audit-q4
    Title: Q4 2024 Security Audit
    Amount: 25,000.00 TIME
    Status: Approved ✓
    ───────────────────────────────────────────
    Voting Ended: Nov 10, 2023
    Execute By: Dec 10, 2023
    
    Final Results:
      YES: 350 (70.0%) ✓
      NO:  150 (30.0%)
```

**Use Cases:**
- Track active proposals
- Monitor voting progress
- Identify approved proposals ready for execution
- Audit historical proposals

### Future CLI Commands

These commands are planned for future implementation to provide full CLI control:

#### Create Treasury Proposal

**Command (Planned):**
```bash
time-cli treasury propose \
  --id "proposal-id" \
  --title "Proposal Title" \
  --description "Detailed description" \
  --recipient "time1address..." \
  --amount 50000 \
  --voting-days 14
```

**Parameters:**
- `--id`: Unique proposal identifier (required)
- `--title`: Short proposal title (required)
- `--description`: Detailed description (required)
- `--recipient`: TIME address to receive funds (required)
- `--amount`: Amount in TIME (required)
- `--voting-days`: Voting period in days (default: 14)

**Example:**
```bash
time-cli treasury propose \
  --id "website-redesign-2024" \
  --title "TIME Coin Website Redesign" \
  --description "Modernize website with responsive design, updated branding, and improved UX. Deliverables: Figma designs, React frontend, Node.js backend." \
  --recipient "time1designer_team_abc123..." \
  --amount 30000 \
  --voting-days 14
```

**Expected Output:**
```
Proposal Created Successfully
════════════════════════════════════════════════════════════════

ID:                  website-redesign-2024
Title:               TIME Coin Website Redesign
Amount:              30,000.00 TIME
Recipient:           time1designer_team_abc123...
Voting Period:       14 days
Voting Deadline:     Dec 1, 2023 10:30 UTC
Execution Deadline:  Dec 31, 2023 10:30 UTC

Status:              Active (Open for voting)

Next Steps:
  1. Share proposal in community channels
  2. Answer questions from masternode operators
  3. Monitor voting progress: time-cli treasury info website-redesign-2024
  4. Execute if approved: time-cli treasury execute website-redesign-2024
```

#### Vote on Proposal

**Command (Planned):**
```bash
time-cli treasury vote \
  --proposal "proposal-id" \
  --choice yes \
  --masternode "mn-id"
```

**Parameters:**
- `--proposal`: Proposal ID to vote on (required)
- `--choice`: Vote choice: yes, no, abstain (required)
- `--masternode`: Masternode ID (auto-detected if only one)

**Examples:**
```bash
# Vote YES on a proposal
time-cli treasury vote --proposal website-redesign-2024 --choice yes

# Vote NO with specific masternode
time-cli treasury vote \
  --proposal website-redesign-2024 \
  --choice no \
  --masternode mn-gold-production-1

# Abstain from voting
time-cli treasury vote \
  --proposal website-redesign-2024 \
  --choice abstain
```

**Expected Output:**
```
Vote Cast Successfully
════════════════════════════════════════════════════════════════

Proposal:        website-redesign-2024
Your Vote:       YES ✓
Voting Power:    100 (Gold Tier)
Timestamp:       Nov 20, 2023 14:22:15 UTC

Current Results:
  YES:     340 (68.0%) ✓ Passing
  NO:      160 (32.0%)
  ABSTAIN: 50

Total Votes:     550 / 1000 voting power (55% participation)
Approval:        68.0% (≥67% required for approval)

Status:          Active (6 days remaining)
Voting Ends:     Nov 26, 2023 10:30 UTC
```

#### Get Proposal Details

**Command (Planned):**
```bash
time-cli treasury info <proposal-id>
```

**Example:**
```bash
time-cli treasury info website-redesign-2024
```

**Expected Output:**
```
Proposal Details
════════════════════════════════════════════════════════════════

ID:               website-redesign-2024
Title:            TIME Coin Website Redesign
Status:           Active (Voting Open)

FUNDING DETAILS
────────────────────────────────────────────────────────────────
Amount:           30,000.00 TIME
Recipient:        time1designer_team_abc123...
Submitter:        time1community_member_xyz...

TIMELINE
────────────────────────────────────────────────────────────────
Submitted:        Nov 17, 2023 10:30 UTC
Voting Deadline:  Dec 1, 2023 10:30 UTC (6 days remaining)
Execute By:       Dec 31, 2023 10:30 UTC

DESCRIPTION
────────────────────────────────────────────────────────────────
Modernize website with responsive design, updated branding, and
improved UX. Deliverables: Figma designs, React frontend, Node.js
backend.

VOTING RESULTS
────────────────────────────────────────────────────────────────
  YES:     340 (68.0%)  ████████████████████████
  NO:      160 (32.0%)  ███████████
  ABSTAIN: 50  (N/A)    

Total Voting Power: 550 / 1000 (55% participation)
Approval: 68.0% ✓ (≥67% required)

RECENT VOTES
────────────────────────────────────────────────────────────────
[2023-11-20 14:22] mn-gold-production-1    YES  (100 power)
[2023-11-20 13:15] mn-silver-operator-42   YES  (10 power)
[2023-11-20 12:05] mn-bronze-node-7        NO   (1 power)
[2023-11-19 22:30] mn-gold-validator-3     YES  (100 power)
```

#### Execute Approved Proposal

**Command (Planned):**
```bash
time-cli treasury execute <proposal-id>
```

**Example:**
```bash
time-cli treasury execute website-redesign-2024
```

**Expected Output:**
```
Executing Proposal...
════════════════════════════════════════════════════════════════

Proposal:         website-redesign-2024
Status:           Approved ✓
Amount:           30,000.00 TIME
Recipient:        time1designer_team_abc123...

Pre-Execution Checks:
  ✓ Proposal status: Approved
  ✓ Execution deadline: 10 days remaining
  ✓ Treasury balance: 1,234,567 TIME (sufficient)
  ✓ Approval valid: 68.0% (≥67% required)

Distributing Funds...

════════════════════════════════════════════════════════════════
Execution Complete ✓
════════════════════════════════════════════════════════════════

Transaction Details:
  Block:            #12734
  Timestamp:        Nov 25, 2023 16:45:22 UTC
  Amount:           30,000.00 TIME
  Recipient:        time1designer_team_abc123...
  
Treasury Update:
  Previous Balance: 1,234,567.00 TIME
  Distributed:      -30,000.00 TIME
  New Balance:      1,204,567.00 TIME

Proposal Status:  Approved → Executed ✓

View transaction:
  time-cli rpc gettreasurywithdrawals
```

#### List Proposals by Status

**Command (Planned):**
```bash
time-cli treasury list [--status <status>]
```

**Status Options:**
- `active`: Currently accepting votes
- `approved`: Passed voting, awaiting execution
- `rejected`: Failed to reach approval threshold
- `executed`: Successfully executed
- `expired`: Approved but not executed in time
- `all`: All proposals (default)

**Examples:**
```bash
# List all active proposals
time-cli treasury list --status active

# List approved proposals ready for execution
time-cli treasury list --status approved

# List all proposals
time-cli treasury list
```

## API Endpoints

### Current Implementation

The TIME Coin API provides REST endpoints for treasury operations. All endpoints return JSON responses.

#### GET /treasury/stats

**Description:** Get current treasury statistics.

**Request:**
```bash
curl http://localhost:24101/treasury/stats
```

**Response:**
```json
{
  "balance": 123456700000000,
  "balance_time": 1234567.0,
  "total_allocated": 234567800000000,
  "total_distributed": 111111100000000,
  "allocation_count": 15678,
  "withdrawal_count": 42,
  "pending_proposals": 3
}
```

**Response Fields:**
- `balance`: Current treasury balance in smallest unit (1 TIME = 100,000,000 units)
- `balance_time`: Current balance in TIME (human-readable)
- `total_allocated`: Lifetime deposits to treasury
- `total_distributed`: Lifetime withdrawals from treasury
- `allocation_count`: Number of allocation records
- `withdrawal_count`: Number of withdrawal records
- `pending_proposals`: Number of active proposals

**Status Codes:**
- `200 OK`: Success
- `500 Internal Server Error`: Server error

**Example (JavaScript):**
```javascript
async function getTreasuryStats() {
  const response = await fetch('http://localhost:24101/treasury/stats');
  const data = await response.json();
  
  console.log(`Treasury Balance: ${data.balance_time.toLocaleString()} TIME`);
  console.log(`Pending Proposals: ${data.pending_proposals}`);
  
  return data;
}
```

**Example (Python):**
```python
import requests

def get_treasury_stats():
    response = requests.get('http://localhost:24101/treasury/stats')
    data = response.json()
    
    print(f"Treasury Balance: {data['balance_time']:,.2f} TIME")
    print(f"Pending Proposals: {data['pending_proposals']}")
    
    return data
```

#### GET /treasury/allocations

**Description:** Get treasury allocation history (deposits).

**Request:**
```bash
curl http://localhost:24101/treasury/allocations
```

**Response:**
```json
[
  {
    "block_number": 12345,
    "amount": 500000000,
    "source": "BlockReward",
    "timestamp": 1699123456
  },
  {
    "block_number": 12345,
    "amount": 250000000,
    "source": "TransactionFees",
    "timestamp": 1699123456
  },
  {
    "block_number": 12346,
    "amount": 500000000,
    "source": "BlockReward",
    "timestamp": 1699123500
  }
]
```

**Response Fields:**
- `block_number`: Block where allocation occurred
- `amount`: Amount allocated in smallest unit
- `source`: Source of allocation ("BlockReward" or "TransactionFees")
- `timestamp`: Unix timestamp

**Status Codes:**
- `200 OK`: Success
- `500 Internal Server Error`: Server error

**Example (JavaScript with Filtering):**
```javascript
async function getRecentBlockRewardAllocations(days = 7) {
  const response = await fetch('http://localhost:24101/treasury/allocations');
  const allocations = await response.json();
  
  const cutoffTime = Date.now() / 1000 - (days * 86400);
  
  const recentBlockRewards = allocations
    .filter(a => a.source === 'BlockReward')
    .filter(a => a.timestamp >= cutoffTime)
    .map(a => ({
      block: a.block_number,
      amount: a.amount / 100000000,
      date: new Date(a.timestamp * 1000).toLocaleDateString()
    }));
  
  console.log(`Block Reward Allocations (Last ${days} days):`);
  recentBlockRewards.forEach(a => {
    console.log(`  Block ${a.block}: ${a.amount} TIME (${a.date})`);
  });
  
  return recentBlockRewards;
}
```

**Example (Python with Analysis):**
```python
import requests
from datetime import datetime, timedelta

def analyze_allocations(days=30):
    response = requests.get('http://localhost:24101/treasury/allocations')
    allocations = response.json()
    
    cutoff = (datetime.now() - timedelta(days=days)).timestamp()
    recent = [a for a in allocations if a['timestamp'] >= cutoff]
    
    block_rewards = sum(a['amount'] for a in recent if a['source'] == 'BlockReward')
    tx_fees = sum(a['amount'] for a in recent if a['source'] == 'TransactionFees')
    
    print(f"Treasury Allocations (Last {days} days)")
    print(f"  Block Rewards:      {block_rewards / 100000000:,.2f} TIME")
    print(f"  Transaction Fees:   {tx_fees / 100000000:,.2f} TIME")
    print(f"  Total:              {(block_rewards + tx_fees) / 100000000:,.2f} TIME")
    
    return {
        'block_rewards': block_rewards / 100000000,
        'transaction_fees': tx_fees / 100000000,
        'total': (block_rewards + tx_fees) / 100000000
    }
```

#### GET /treasury/withdrawals

**Description:** Get treasury withdrawal history (distributions).

**Request:**
```bash
curl http://localhost:24101/treasury/withdrawals
```

**Response:**
```json
[
  {
    "proposal_id": "mobile-wallet-2024",
    "amount": 7500000000000,
    "recipient": "time1dev_team_wallet...",
    "block_number": 12500,
    "timestamp": 1700000000
  },
  {
    "proposal_id": "security-audit-q4",
    "amount": 2500000000000,
    "recipient": "time1auditor_address...",
    "block_number": 12450,
    "timestamp": 1699900000
  }
]
```

**Response Fields:**
- `proposal_id`: Associated proposal identifier
- `amount`: Amount distributed in smallest unit
- `recipient`: Recipient address
- `block_number`: Block where distribution occurred
- `timestamp`: Unix timestamp

**Status Codes:**
- `200 OK`: Success
- `500 Internal Server Error`: Server error

**Example (JavaScript):**
```javascript
async function getWithdrawalSummary() {
  const response = await fetch('http://localhost:24101/treasury/withdrawals');
  const withdrawals = await response.json();
  
  const totalDistributed = withdrawals.reduce((sum, w) => sum + w.amount, 0) / 100000000;
  const recipients = new Set(withdrawals.map(w => w.recipient)).size;
  
  console.log(`Total Withdrawals: ${withdrawals.length}`);
  console.log(`Total Distributed: ${totalDistributed.toLocaleString()} TIME`);
  console.log(`Unique Recipients: ${recipients}`);
  
  // Group by proposal
  const byProposal = {};
  withdrawals.forEach(w => {
    if (!byProposal[w.proposal_id]) {
      byProposal[w.proposal_id] = {
        amount: 0,
        count: 0
      };
    }
    byProposal[w.proposal_id].amount += w.amount / 100000000;
    byProposal[w.proposal_id].count++;
  });
  
  console.log('\nBy Proposal:');
  Object.entries(byProposal).forEach(([id, data]) => {
    console.log(`  ${id}: ${data.amount.toLocaleString()} TIME (${data.count} withdrawals)`);
  });
  
  return withdrawals;
}
```

#### POST /treasury/approve

**Description:** Approve a proposal for treasury spending (internal governance use).

**Request:**
```bash
curl -X POST http://localhost:24101/treasury/approve \
  -H "Content-Type: application/json" \
  -d '{
    "proposal_id": "mobile-wallet-2024",
    "amount": 7500000000000
  }'
```

**Request Body:**
```json
{
  "proposal_id": "mobile-wallet-2024",
  "amount": 7500000000000
}
```

**Response:**
```json
{
  "status": "success",
  "proposal_id": "mobile-wallet-2024",
  "approved_amount": 7500000000000,
  "message": "Proposal approved for treasury spending"
}
```

**Status Codes:**
- `200 OK`: Proposal approved successfully
- `400 Bad Request`: Invalid request body
- `500 Internal Server Error`: Server error

**Note:** This endpoint is typically called internally by the governance system after a proposal reaches approval threshold. Direct use is for testing or administrative purposes.

#### POST /treasury/distribute

**Description:** Distribute funds for approved proposal.

**Request:**
```bash
curl -X POST http://localhost:24101/treasury/distribute \
  -H "Content-Type: application/json" \
  -d '{
    "proposal_id": "mobile-wallet-2024",
    "recipient": "time1dev_team_wallet...",
    "amount": 7500000000000
  }'
```

**Request Body:**
```json
{
  "proposal_id": "mobile-wallet-2024",
  "recipient": "time1dev_team_wallet...",
  "amount": 7500000000000
}
```

**Response:**
```json
{
  "status": "success",
  "proposal_id": "mobile-wallet-2024",
  "recipient": "time1dev_team_wallet...",
  "amount": 7500000000000,
  "message": "Treasury funds distributed successfully"
}
```

**Status Codes:**
- `200 OK`: Funds distributed successfully
- `400 Bad Request`: Invalid request body or insufficient funds
- `500 Internal Server Error`: Server error

### RPC Endpoints

#### POST /rpc/gettreasury

**Description:** RPC method to get treasury information.

**Request:**
```bash
curl -X POST http://localhost:24101/rpc/gettreasury \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "gettreasury",
    "params": [],
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "balance": 123456700000000,
    "balance_time": 1234567.0,
    "total_allocated": 234567800000000,
    "total_distributed": 111111100000000,
    "allocation_count": 15678,
    "withdrawal_count": 42,
    "pending_proposals": 3
  },
  "id": 1
}
```

#### POST /rpc/listproposals

**Description:** RPC method to list all treasury proposals.

**Request:**
```bash
curl -X POST http://localhost:24101/rpc/listproposals \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "listproposals",
    "params": [],
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "proposals": [
      {
        "id": "mobile-wallet-2024",
        "title": "iOS and Android Wallet Development",
        "amount": 7500000000000,
        "status": "Active",
        "votes": {
          "yes": 640,
          "no": 110,
          "abstain": 0
        },
        "approval_percentage": 85.3
      }
    ]
  },
  "id": 1
}
```

## Usage Examples

### Example 1: Monitor Treasury Health (Dashboard)

**Objective:** Create a real-time dashboard showing treasury status.

**JavaScript Implementation:**
```javascript
class TreasuryDashboard {
  constructor(apiUrl = 'http://localhost:24101') {
    this.apiUrl = apiUrl;
  }
  
  async fetchStats() {
    const response = await fetch(`${this.apiUrl}/treasury/stats`);
    return await response.json();
  }
  
  async fetchAllocations() {
    const response = await fetch(`${this.apiUrl}/treasury/allocations`);
    return await response.json();
  }
  
  async fetchWithdrawals() {
    const response = await fetch(`${this.apiUrl}/treasury/withdrawals`);
    return await response.json();
  }
  
  async render() {
    const stats = await this.fetchStats();
    const allocations = await this.fetchAllocations();
    const withdrawals = await this.fetchWithdrawals();
    
    // Calculate daily growth
    const oneDayAgo = Date.now() / 1000 - 86400;
    const recentAllocations = allocations.filter(a => a.timestamp >= oneDayAgo);
    const dailyGrowth = recentAllocations.reduce((sum, a) => sum + a.amount, 0) / 100000000;
    
    // Calculate utilization
    const utilization = (stats.total_distributed / stats.total_allocated) * 100;
    
    console.log('═══════════════════════════════════════════════');
    console.log('        TIME COIN TREASURY DASHBOARD           ');
    console.log('═══════════════════════════════════════════════');
    console.log();
    console.log(`Current Balance:    ${stats.balance_time.toLocaleString()} TIME`);
    console.log(`Daily Growth:       ${dailyGrowth.toFixed(2)} TIME`);
    console.log(`Pending Proposals:  ${stats.pending_proposals}`);
    console.log();
    console.log('LIFETIME STATISTICS');
    console.log('───────────────────────────────────────────────');
    console.log(`Total Allocated:    ${(stats.total_allocated / 100000000).toLocaleString()} TIME`);
    console.log(`Total Distributed:  ${(stats.total_distributed / 100000000).toLocaleString()} TIME`);
    console.log(`Utilization:        ${utilization.toFixed(2)}%`);
    console.log();
    console.log(`Allocation Count:   ${stats.allocation_count}`);
    console.log(`Withdrawal Count:   ${stats.withdrawal_count}`);
    console.log('═══════════════════════════════════════════════');
    
    return { stats, allocations, withdrawals };
  }
}

// Usage
const dashboard = new TreasuryDashboard();
await dashboard.render();

// Update every 30 seconds
setInterval(() => dashboard.render(), 30000);
```

### Example 2: Proposal Submission Workflow

**Objective:** Submit and monitor a treasury proposal.

**Python Implementation:**
```python
import requests
import time
from datetime import datetime, timedelta

class ProposalManager:
    def __init__(self, api_url='http://localhost:24101'):
        self.api_url = api_url
        
    def check_treasury_balance(self, required_amount):
        """Check if treasury has sufficient funds"""
        response = requests.get(f'{self.api_url}/treasury/stats')
        stats = response.json()
        
        available = stats['balance_time']
        required = required_amount
        
        print(f"Treasury Balance: {available:,.2f} TIME")
        print(f"Required Amount:  {required:,.2f} TIME")
        
        if available >= required:
            print("✓ Sufficient funds available")
            return True
        else:
            print("✗ Insufficient funds")
            return False
    
    def submit_proposal(self, proposal_data):
        """Submit a new proposal (via future CLI/API)"""
        print("\n" + "="*60)
        print("SUBMITTING PROPOSAL")
        print("="*60)
        
        print(f"ID:          {proposal_data['id']}")
        print(f"Title:       {proposal_data['title']}")
        print(f"Amount:      {proposal_data['amount']:,.2f} TIME")
        print(f"Recipient:   {proposal_data['recipient']}")
        print(f"Description: {proposal_data['description'][:50]}...")
        
        # Calculate deadlines
        voting_deadline = datetime.now() + timedelta(days=14)
        execution_deadline = voting_deadline + timedelta(days=30)
        
        print(f"\nVoting Deadline:    {voting_deadline.strftime('%Y-%m-%d %H:%M UTC')}")
        print(f"Execution Deadline: {execution_deadline.strftime('%Y-%m-%d %H:%M UTC')}")
        
        # In real implementation, this would call the API
        print("\n✓ Proposal submitted successfully")
        print(f"  Track progress: time-cli treasury info {proposal_data['id']}")
        
        return {
            'success': True,
            'proposal_id': proposal_data['id'],
            'voting_deadline': voting_deadline.timestamp(),
            'execution_deadline': execution_deadline.timestamp()
        }
    
    def monitor_proposal(self, proposal_id, check_interval=3600):
        """Monitor proposal voting progress"""
        print(f"\nMonitoring proposal: {proposal_id}")
        print(f"Checking every {check_interval} seconds...")
        
        while True:
            # In real implementation, fetch from API
            # For now, simulate checking
            print(f"[{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}] Checking status...")
            
            # Simulate: would fetch from /rpc/listproposals
            # and find matching proposal
            
            # For demo purposes:
            print("  Status: Active")
            print("  Votes: 340 YES, 160 NO (68% approval)")
            print("  Voting Power: 500 / 1000 (50% participation)")
            
            time.sleep(check_interval)

# Usage Example
manager = ProposalManager()

# 1. Check treasury balance
required = 50000  # 50,000 TIME
if manager.check_treasury_balance(required):
    
    # 2. Submit proposal
    proposal = {
        'id': 'marketing-campaign-2024',
        'title': 'Q4 Marketing Campaign',
        'description': 'Comprehensive marketing campaign including social media, influencer partnerships, and conference sponsorships.',
        'amount': required,
        'recipient': 'time1marketing_team_abc123...'
    }
    
    result = manager.submit_proposal(proposal)
    
    if result['success']:
        # 3. Monitor progress (in separate process/thread)
        # manager.monitor_proposal(proposal['id'])
        pass
```

### Example 3: Voting Analytics

**Objective:** Analyze voting patterns and participation.

**JavaScript Implementation:**
```javascript
class VotingAnalytics {
  constructor(apiUrl = 'http://localhost:24101') {
    this.apiUrl = apiUrl;
  }
  
  async getProposals() {
    const response = await fetch(`${this.apiUrl}/rpc/listproposals`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'listproposals',
        params: [],
        id: 1
      })
    });
    
    const data = await response.json();
    return data.result.proposals;
  }
  
  async analyzeParticipation() {
    const proposals = await this.getProposals();
    
    console.log('VOTING PARTICIPATION ANALYSIS');
    console.log('═══════════════════════════════════════════════');
    console.log();
    
    proposals.forEach(proposal => {
      const totalVotes = proposal.votes.yes + proposal.votes.no + proposal.votes.abstain;
      const participationRate = (totalVotes / 1000) * 100; // Assuming 1000 total power
      
      console.log(`Proposal: ${proposal.id}`);
      console.log(`  Status: ${proposal.status}`);
      console.log(`  Total Votes: ${totalVotes}`);
      console.log(`  Participation: ${participationRate.toFixed(1)}%`);
      console.log(`  Approval: ${proposal.approval_percentage.toFixed(1)}%`);
      
      if (proposal.approval_percentage >= 67) {
        console.log(`  Result: ✓ APPROVED`);
      } else {
        console.log(`  Result: ✗ REJECTED`);
      }
      console.log();
    });
  }
  
  async compareApprovalRates() {
    const proposals = await this.getProposals();
    
    const approved = proposals.filter(p => p.status === 'Approved' || p.status === 'Executed');
    const rejected = proposals.filter(p => p.status === 'Rejected');
    
    const avgApprovalApproved = approved.reduce((sum, p) => sum + p.approval_percentage, 0) / approved.length;
    const avgApprovalRejected = rejected.reduce((sum, p) => sum + p.approval_percentage, 0) / rejected.length;
    
    console.log('APPROVAL RATE COMPARISON');
    console.log('═══════════════════════════════════════════════');
    console.log(`Approved Proposals: ${approved.length}`);
    console.log(`  Avg Approval: ${avgApprovalApproved.toFixed(1)}%`);
    console.log();
    console.log(`Rejected Proposals: ${rejected.length}`);
    console.log(`  Avg Approval: ${avgApprovalRejected.toFixed(1)}%`);
    console.log();
    console.log(`Threshold: 67.0%`);
  }
}

// Usage
const analytics = new VotingAnalytics();
await analytics.analyzeParticipation();
await analytics.compareApprovalRates();
```

## Integration Patterns

### Pattern 1: Proposal Lifecycle Management

Complete workflow for managing a proposal from creation to execution:

```javascript
class ProposalLifecycleManager {
  constructor(apiUrl) {
    this.apiUrl = apiUrl;
  }
  
  // Phase 1: Pre-submission
  async validateProposal(proposalData) {
    // Check treasury balance
    const stats = await this.fetchTreasuryStats();
    if (stats.balance_time < proposalData.amount) {
      throw new Error('Insufficient treasury balance');
    }
    
    // Check for duplicate ID
    const proposals = await this.fetchProposals();
    if (proposals.some(p => p.id === proposalData.id)) {
      throw new Error('Proposal ID already exists');
    }
    
    return true;
  }
  
  // Phase 2: Submission
  async submitProposal(proposalData) {
    await this.validateProposal(proposalData);
    
    // Submit via CLI or API (future implementation)
    console.log(`Submitting proposal: ${proposalData.id}`);
    
    return {
      proposalId: proposalData.id,
      votingDeadline: Date.now() + (14 * 86400 * 1000),
      executionDeadline: Date.now() + (44 * 86400 * 1000)
    };
  }
  
  // Phase 3: Monitoring
  async monitorVoting(proposalId, callback) {
    const checkInterval = 3600000; // 1 hour
    
    const intervalId = setInterval(async () => {
      const proposals = await this.fetchProposals();
      const proposal = proposals.find(p => p.id === proposalId);
      
      if (!proposal) {
        clearInterval(intervalId);
        callback({ error: 'Proposal not found' });
        return;
      }
      
      callback({
        status: proposal.status,
        approval: proposal.approval_percentage,
        votes: proposal.votes
      });
      
      // Stop monitoring if proposal is no longer active
      if (proposal.status !== 'Active') {
        clearInterval(intervalId);
      }
    }, checkInterval);
    
    return intervalId;
  }
  
  // Phase 4: Execution
  async executeIfApproved(proposalId) {
    const proposals = await this.fetchProposals();
    const proposal = proposals.find(p => p.id === proposalId);
    
    if (!proposal) {
      throw new Error('Proposal not found');
    }
    
    if (proposal.status !== 'Approved') {
      throw new Error(`Cannot execute: status is ${proposal.status}`);
    }
    
    // Execute via CLI or API (future implementation)
    console.log(`Executing proposal: ${proposalId}`);
    
    return {
      success: true,
      amount: proposal.amount,
      recipient: proposal.recipient
    };
  }
  
  // Helper methods
  async fetchTreasuryStats() {
    const response = await fetch(`${this.apiUrl}/treasury/stats`);
    return await response.json();
  }
  
  async fetchProposals() {
    const response = await fetch(`${this.apiUrl}/rpc/listproposals`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'listproposals',
        params: [],
        id: 1
      })
    });
    const data = await response.json();
    return data.result.proposals;
  }
}

// Usage
const manager = new ProposalLifecycleManager('http://localhost:24101');

// 1. Submit proposal
const result = await manager.submitProposal({
  id: 'my-proposal',
  title: 'My Proposal',
  amount: 50000,
  description: 'Detailed description',
  recipient: 'time1address...'
});

// 2. Monitor voting
const monitorId = await manager.monitorVoting('my-proposal', (status) => {
  console.log('Proposal Status:', status);
  
  if (status.status === 'Approved') {
    // Proposal approved! Execute it
    manager.executeIfApproved('my-proposal')
      .then(() => console.log('Executed successfully'))
      .catch(err => console.error('Execution failed:', err));
  }
});
```

### Pattern 2: Treasury Health Monitoring

Automated monitoring and alerting:

```python
import requests
import time
from datetime import datetime

class TreasuryMonitor:
    def __init__(self, api_url, alert_threshold=0.1):
        self.api_url = api_url
        self.alert_threshold = alert_threshold  # 10% balance change
        self.last_balance = None
        
    def fetch_stats(self):
        response = requests.get(f'{self.api_url}/treasury/stats')
        return response.json()
    
    def check_health(self):
        stats = self.fetch_stats()
        current_balance = stats['balance_time']
        
        issues = []
        
        # Check for rapid depletion
        if self.last_balance is not None:
            change = (self.last_balance - current_balance) / self.last_balance
            if change > self.alert_threshold:
                issues.append({
                    'type': 'rapid_depletion',
                    'severity': 'warning',
                    'message': f'Balance dropped {change*100:.1f}% since last check',
                    'previous': self.last_balance,
                    'current': current_balance
                })
        
        # Check for low balance
        if current_balance < 100000:  # Less than 100k TIME
            issues.append({
                'type': 'low_balance',
                'severity': 'warning',
                'message': f'Treasury balance low: {current_balance:,.0f} TIME'
            })
        
        # Check pending proposals vs balance
        pending = stats['pending_proposals']
        if pending > 5:
            issues.append({
                'type': 'high_pending',
                'severity': 'info',
                'message': f'High number of pending proposals: {pending}'
            })
        
        self.last_balance = current_balance
        
        return {
            'timestamp': datetime.now().isoformat(),
            'balance': current_balance,
            'issues': issues,
            'healthy': len(issues) == 0
        }
    
    def monitor(self, interval=300):
        """Monitor treasury health continuously"""
        print("Starting treasury health monitor...")
        print(f"Check interval: {interval} seconds")
        print(f"Alert threshold: {self.alert_threshold*100}%")
        print()
        
        while True:
            health = self.check_health()
            
            timestamp = health['timestamp']
            balance = health['balance']
            
            if health['healthy']:
                print(f"[{timestamp}] ✓ Treasury healthy - Balance: {balance:,.0f} TIME")
            else:
                print(f"[{timestamp}] ⚠ Issues detected:")
                for issue in health['issues']:
                    print(f"  [{issue['severity'].upper()}] {issue['message']}")
            
            time.sleep(interval)

# Usage
monitor = TreasuryMonitor('http://localhost:24101', alert_threshold=0.1)
monitor.monitor(interval=300)  # Check every 5 minutes
```

## Troubleshooting

### Common Issues and Solutions

#### Issue: "Connection refused" when calling API

**Symptoms:**
```
Error: connect ECONNREFUSED 127.0.0.1:24101
```

**Causes:**
1. `timed` node not running
2. Wrong port number
3. Firewall blocking connection

**Solutions:**
```bash
# Check if node is running
ps aux | grep timed

# Start the node if not running
timed --config config/testnet.toml

# Check API port in config
cat config/testnet.toml | grep api_port

# Test connection
curl http://localhost:24101/health
```

#### Issue: "Proposal ID already exists"

**Symptoms:**
```
Error: Proposal my-proposal already exists
```

**Cause:** Attempting to create proposal with duplicate ID

**Solution:**
```bash
# Use unique, descriptive IDs
# Include date or version number

# Good examples:
website-redesign-2024-q4
security-audit-2024-11
mobile-wallet-v2

# Check existing proposals
time-cli rpc listproposals
```

#### Issue: "Insufficient treasury balance"

**Symptoms:**
```
Error: Insufficient treasury balance for distribution
```

**Causes:**
1. Requested amount exceeds balance
2. Multiple proposals approved simultaneously
3. Timing issue with concurrent executions

**Solutions:**
```bash
# Check current balance
time-cli rpc gettreasury

# Verify proposal amount is reasonable
# Consider reducing amount or waiting for more funds

# Check pending/approved proposals
time-cli rpc listproposals --status approved
```

#### Issue: "Voting period ended"

**Symptoms:**
```
Error: Voting period has ended for this proposal
```

**Cause:** Attempting to vote after voting deadline

**Solution:**
```bash
# Check proposal deadlines
time-cli treasury info <proposal-id>

# Vote must be cast before voting_deadline
# Cannot change or revoke vote after submission
```

#### Issue: "Proposal not approved"

**Symptoms:**
```
Error: Cannot execute: proposal status is Rejected
```

**Cause:** Attempting to execute rejected proposal

**Solution:**
```bash
# Check proposal status
time-cli rpc listproposals

# Rejected proposals cannot be executed
# Must submit new proposal if still needed
```

### Best Practices

1. **Always check treasury balance before proposal submission**
   ```javascript
   const stats = await fetch('/treasury/stats').then(r => r.json());
   if (stats.balance_time < requestedAmount) {
     console.warn('Insufficient funds');
   }
   ```

2. **Use descriptive proposal IDs**
   ```
   ✓ Good: mobile-wallet-ios-2024-q4
   ✗ Bad: proposal1, test, temp
   ```

3. **Monitor proposal progress actively**
   ```bash
   # Check multiple times during voting period
   time-cli treasury info <proposal-id>
   ```

4. **Execute approved proposals promptly**
   ```bash
   # Don't wait until last minute
   # Execute within first week of approval
   time-cli treasury execute <proposal-id>
   ```

5. **Cache API responses appropriately**
   ```javascript
   // Don't hammer the API
   // Cache treasury stats for 1-5 minutes
   const CACHE_TTL = 60000; // 1 minute
   ```

---

**Document Version:** 1.0  
**Last Updated:** November 2024  
**Status:** Active
