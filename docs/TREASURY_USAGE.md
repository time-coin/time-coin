# Treasury Usage Guide

Quick reference for using TIME Coin's protocol-managed treasury system.

## For Developers

### Creating a Proposal Programmatically

```rust
use time_core::treasury_manager::TreasuryManager;

let mut manager = TreasuryManager::new();

// Create a development grant proposal
manager.create_proposal(
    "dev-grant-2024-q4".to_string(),
    "Mobile Wallet Development".to_string(),
    "Develop iOS and Android mobile wallets".to_string(),
    "time1developer123...".to_string(),
    100_000 * 100_000_000,  // 100,000 TIME
    "time1proposer456...".to_string(),
    current_timestamp(),
    14,  // 14-day voting period
)?;
```

### Voting on a Proposal

```rust
// As a masternode operator
manager.vote_on_proposal(
    "dev-grant-2024-q4",
    "my-masternode-id".to_string(),
    VoteChoice::Yes,
    100,  // Voting power (Gold = 100)
    current_timestamp(),
)?;
```

### Checking Proposal Status

```rust
let proposal = manager.get_proposal("dev-grant-2024-q4")?;
println!("Status: {:?}", proposal.status);
println!("Votes: {} votes cast", proposal.votes.len());

let results = proposal.calculate_results();
println!("Approval: {}%", results.approval_percentage());
```

## For Masternode Operators

### View Treasury Status

```bash
# Get current treasury balance and statistics
time-cli rpc gettreasury
```

**Example Output:**
```
Treasury Balance: 1,234.567 TIME
Total Allocated: 2,345.678 TIME (lifetime)
Total Distributed: 1,111.111 TIME (lifetime)
Pending Proposals: 3
```

### List Active Proposals

```bash
# View all proposals and their status
time-cli rpc listproposals
```

**Example Output:**
```
ID: prop-001
Title: Website Redesign
Amount: 50,000 TIME
Recipient: time1abc123...
Status: Active
Voting Ends: 2024-11-30 12:00:00 UTC
Votes: 110 YES, 1 NO (99% approval)

ID: prop-002
Title: Security Audit
Amount: 25,000 TIME
Status: Approved
Votes: 220 YES, 50 NO (81% approval)
```

### Vote on a Proposal

```bash
# Vote YES on a proposal
time-cli treasury vote \
  --proposal-id prop-001 \
  --masternode-id my-masternode-1 \
  --choice yes \
  --voting-power 100

# Vote NO on a proposal
time-cli treasury vote \
  --proposal-id prop-001 \
  --masternode-id my-masternode-1 \
  --choice no \
  --voting-power 100

# Abstain from voting
time-cli treasury vote \
  --proposal-id prop-001 \
  --masternode-id my-masternode-1 \
  --choice abstain \
  --voting-power 100
```

### View Proposal Details

```bash
# Get detailed information about a specific proposal
time-cli treasury details prop-001
```

**Example Output:**
```
📄 Proposal Details
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ID:          prop-001
Title:       Website Redesign
Description: Modernize TIME Coin website with new UI/UX
Recipient:   TIME1abc123...
Amount:      50000.0 TIME
Submitter:   TIME1xyz789...
Status:      Active

Voting:
  Yes votes:   110
  No votes:    1
  Total votes: 111
  Approval:    99.1%
```

### List Proposals with Filtering

```bash
# List all proposals
time-cli treasury list

# List only active proposals
time-cli treasury list --status active

# List approved proposals
time-cli treasury list --status approved
```

## For Proposal Submitters

### Proposal Requirements

Before submitting a proposal, ensure you have:
- ✅ Clear title and description
- ✅ Recipient address
- ✅ Justified funding amount
- ✅ Detailed milestones (recommended)
- ✅ Timeline for completion
- ✅ Community support

### Submission Checklist

1. **Prepare Proposal Details**
   - Title: Concise, descriptive
   - Description: Comprehensive explanation
   - Amount: Justified and reasonable
   - Timeline: Realistic schedule

2. **Create Proposal**
   ```bash
   # Create a new treasury proposal
   time-cli treasury propose \
     --id "website-redesign-2024" \
     --title "Website Redesign" \
     --description "Modernize TIME Coin website with improved UI/UX" \
     --recipient "TIME1abc123..." \
     --amount 50000 \
     --submitter "TIME1xyz789..." \
     --voting-days 14
   ```

3. **Promote Proposal**
   - Share on Discord/Forum
   - Answer community questions
   - Engage with masternode operators
   - Provide additional details if requested

4. **Monitor Voting**
   - Track votes regularly
   - Address concerns
   - Build consensus

5. **Execute When Approved**
   - Wait for approval (67%+ YES votes)
   - Execute within 30-day deadline
   - Provide updates on progress

### Best Practices

**Do:**
- ✅ Be transparent about goals and timeline
- ✅ Provide detailed milestones
- ✅ Engage with the community
- ✅ Respond to feedback
- ✅ Deliver on promises

**Don't:**
- ❌ Request unrealistic amounts
- ❌ Submit duplicate proposals
- ❌ Make false claims
- ❌ Ignore community feedback
- ❌ Miss execution deadlines

## API Integration

### Get Treasury Stats

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

### Get Allocation History

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
  }
]
```

### Get Withdrawal History

```bash
curl http://localhost:24101/treasury/withdrawals
```

**Response:**
```json
[
  {
    "proposal_id": "prop-001",
    "amount": 5000000000000,
    "recipient": "time1abc123...",
    "block_number": 12400,
    "timestamp": 1699200000
  }
]
```

## Voting Power Reference

| Masternode Tier | Collateral | Voting Power | Notes |
|----------------|------------|--------------|-------|
| Bronze 🥉 | 1,000 TIME | 1 vote | Entry tier |
| Silver 🥈 | 10,000 TIME | 10 votes | Standard tier |
| Gold 🥇 | 100,000 TIME | 100 votes | Premium tier |

### Voting Example

**Scenario:** Proposal needs 67% approval
- 3 Gold masternodes (300 votes total)
- Proposal passes if >= 201 votes are YES

**Voting Outcomes:**
```
✅ 2 Gold YES + 1 Gold NO = 200 YES, 100 NO = 66.67% → PASSES (rounds to 67%)
✅ 3 Gold YES = 300 YES, 0 NO = 100% → PASSES
❌ 1 Gold YES + 2 Gold NO = 100 YES, 200 NO = 33% → FAILS
```

## Common Scenarios

### Scenario 1: Successful Proposal

1. Developer submits proposal for 50,000 TIME
2. Community discusses for 1 week
3. Voting opens for 14 days
4. Masternodes vote:
   - 10 Gold (1000 votes) → 8 YES, 2 NO
   - 50 Silver (500 votes) → 40 YES, 10 NO
   - 100 Bronze (100 votes) → 70 YES, 30 NO
5. Results: 1180 YES, 420 NO = 73.8% approval ✅
6. Proposal APPROVED
7. Developer executes within 30 days
8. Funds distributed on-chain
9. Work begins

### Scenario 2: Rejected Proposal

1. Proposer requests unrealistic 1,000,000 TIME
2. Community skeptical
3. Masternodes vote mostly NO
4. Results: 400 YES, 1200 NO = 25% approval ❌
5. Proposal REJECTED
6. Funds remain in treasury

### Scenario 3: Expired Proposal

1. Proposal approved with 70% YES votes
2. Approved on Block 10,000
3. Execution deadline: Block 13,000 (30 days)
4. Proposer never executes
5. Block 13,001 reached
6. Proposal automatically EXPIRED
7. Approval invalidated
8. Funds remain in treasury

## Troubleshooting

### "Insufficient Treasury Balance"
**Problem:** Proposal amount exceeds treasury balance  
**Solution:** Wait for more block rewards or reduce proposal amount

### "Voting Period Ended"
**Problem:** Trying to vote after deadline  
**Solution:** Wait for next proposal or create new one

### "Masternode Already Voted"
**Problem:** Attempting to vote twice  
**Solution:** Each masternode can only vote once per proposal

### "Proposal Not Approved"
**Problem:** Trying to execute rejected proposal  
**Solution:** Only approved proposals can be executed

### "Execution Deadline Passed"
**Problem:** Proposal expired without execution  
**Solution:** Submit new proposal if still needed

## Security Tips

### For Masternode Operators
1. **Verify Proposals** - Research before voting
2. **Check Recipients** - Validate addresses
3. **Review Amounts** - Ensure reasonable funding
4. **Consider Impact** - Think long-term effects
5. **Vote Honestly** - Support ecosystem health

### For Developers
1. **Secure Recipients** - Use secure addresses
2. **Deliver Results** - Honor commitments
3. **Document Progress** - Keep community informed
4. **Be Transparent** - Share updates regularly
5. **Request Fairly** - Ask for reasonable amounts

## Getting Help

### Resources
- **Documentation**: `/docs/TREASURY_ARCHITECTURE.md`
- **CLI Help**: `time-cli --help`
- **API Docs**: `/docs/api.md`

### Community
- **Discord**: https://discord.gg/timecoin
- **Forum**: https://forum.timecoin.org
- **GitHub**: https://github.com/time-coin/time-coin/issues

### Support
For technical issues:
1. Check documentation
2. Search existing GitHub issues
3. Ask in Discord #dev-support
4. Create GitHub issue if needed

---

**Last Updated**: November 2024  
**Version**: 1.0
