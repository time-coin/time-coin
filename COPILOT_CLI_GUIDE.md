# GitHub Copilot CLI Guide for TIME Coin Development

This guide provides practical examples of using GitHub Copilot CLI to improve the TIME Coin API codebase.

---

## Quick Reference

| Task | Command |
|------|---------|
| **Explain code** | `gh copilot explain <file>` |
| **Get suggestions** | `gh copilot suggest "task description"` |
| **Ask for help** | `gh copilot ask "question"` |

---

## Setup Instructions

### 1. Install GitHub CLI
```powershell
# Windows (if not already installed)
winget install --id GitHub.cli

# Verify installation
gh --version
```

### 2. Authenticate
```powershell
gh auth login
```

### 3. Install Copilot CLI Extension
```powershell
gh extension install github/gh-copilot
gh copilot --version
```

### 4. Update (if needed)
```powershell
gh extension upgrade gh-copilot
```

---

## Common Tasks for TIME Coin API

### Code Explanation

**Understand complex functions:**
```powershell
# Explain balance calculation logic
gh copilot explain api\src\balance.rs

# Understand error handling
gh copilot explain api\src\error.rs

# Explain route structure
gh copilot explain api\src\routes.rs
```

**Ask specific questions:**
```powershell
gh copilot ask "How does the mempool balance calculation work in api/src/balance.rs?"
gh copilot ask "What's the difference between ApiError::Internal and ApiError::BadRequest?"
```

---

### Refactoring Assistance

**1. Split Large Files**
```powershell
# Get modularization suggestions
gh copilot suggest "split api/src/routes.rs into modules organized by endpoint prefix like /blockchain, /wallet, /treasury"

# Get specific module design
gh copilot suggest "design a module structure for api/src/handlers/ with clear separation of concerns"
```

**2. Extract Duplicate Code**
```powershell
# Find duplicates
gh copilot suggest "find all duplicate or similar functions in api/src/ and suggest how to consolidate them"

# Get extraction pattern
gh copilot suggest "extract common transaction validation logic from handlers into utility function"
```

**3. Improve Error Handling**
```powershell
# Unify patterns
gh copilot suggest "list all error handling patterns in api/src/ and suggest unified approach"

# Add context
gh copilot suggest "add error context to all map_err calls in api/src/wallet_send_handler.rs"
```

---

### Code Generation

**1. Create Route Handlers**
```powershell
# Generate new endpoint
gh copilot suggest "write axum route handler for GET /treasury/budget following our ApiResult pattern"

# Generate with validation
gh copilot suggest "create POST endpoint for masternode registration with email and public key validation"
```

**2. Generate Tests**
```powershell
# Unit tests
gh copilot suggest "generate unit tests for balance calculation in api/src/balance.rs covering edge cases"

# Integration tests
gh copilot suggest "create integration test for wallet send endpoint testing successful transaction creation"

# Property-based tests
gh copilot suggest "write property-based tests for balance calculations using quickcheck"
```

**3. Generate Documentation**
```powershell
# Module docs
gh copilot suggest "generate rustdoc module documentation for api/src/treasury_handlers.rs"

# Function docs
gh copilot suggest "add rustdoc comments to all public functions in api/src/balance.rs"

# Examples
gh copilot suggest "add usage examples to rustdoc for calculate_mempool_balance function"
```

---

### Performance Analysis

**1. Find Inefficiencies**
```powershell
# Identify clone calls
gh copilot suggest "find all unnecessary clone() calls in api/src/ that could use references"

# Find N+1 queries
gh copilot suggest "identify potential N+1 query patterns in blockchain iteration code"

# Arc/RwLock analysis
gh copilot suggest "analyze Arc<RwLock<T>> usage in api/src/state.rs and suggest optimizations"
```

**2. Optimization Suggestions**
```powershell
# General optimizations
gh copilot suggest "suggest performance optimizations for balance calculation in api/src/balance.rs"

# Async improvements
gh copilot suggest "find opportunities to parallelize async operations in api/src/routes.rs"
```

---

### Code Quality

**1. Find Code Smells**
```powershell
# Long functions
gh copilot suggest "identify functions in api/src/ longer than 50 lines that should be refactored"

# Complex conditionals
gh copilot suggest "find complex if-else chains in api/src/ that could be simplified"

# Magic numbers
gh copilot suggest "find all magic numbers in api/src/ that should be named constants"
```

**2. Improve Consistency**
```powershell
# Naming consistency
gh copilot suggest "check naming consistency across all handler functions in api/src/"

# Pattern consistency
gh copilot suggest "ensure all handlers follow consistent parameter extraction pattern"
```

---

### Debugging Assistance

**1. Understanding Errors**
```powershell
# Explain compilation error
gh copilot ask "I'm getting error E0609 'no field votes' in consensus crate, what does this mean?"

# Fix suggestions
gh copilot suggest "fix 'no field votes on type TxConsensusManager' error in consensus/src/lib.rs"
```

**2. Add Logging**
```powershell
# Replace println
gh copilot suggest "replace all println! calls in api/src/routes.rs with structured tracing"

# Add debug logging
gh copilot suggest "add debug logging to balance calculation flow in api/src/balance.rs"
```

---

### Testing & Validation

**1. Test Coverage**
```powershell
# Identify gaps
gh copilot suggest "identify untested functions in api/src/balance.rs"

# Generate missing tests
gh copilot suggest "create tests for error cases in api/src/error.rs"
```

**2. Input Validation**
```powershell
# Add validation
gh copilot suggest "add validator crate validation to GrantApplication struct in api/src/grant_handlers.rs"

# Security checks
gh copilot suggest "audit input validation in all POST handlers for security issues"
```

---

## Interactive Workflow Examples

### Example 1: Refactoring a Handler

```powershell
# Step 1: Understand current code
gh copilot explain api\src\masternode_handlers.rs

# Step 2: Get refactoring suggestion
gh copilot suggest "extract business logic from activate_masternode handler into service layer"

# Step 3: Ask follow-up
gh copilot ask "Should the service layer return domain types or API response types?"

# Step 4: Generate tests
gh copilot suggest "create unit tests for masternode service layer"
```

### Example 2: Adding New Feature

```powershell
# Step 1: Design the API
gh copilot suggest "design REST API endpoints for masternode monitoring with health checks"

# Step 2: Generate handler
gh copilot suggest "create axum handler for GET /masternode/{id}/health following our patterns"

# Step 3: Add validation
gh copilot suggest "add input validation for masternode ID parameter"

# Step 4: Write tests
gh copilot suggest "generate integration tests for masternode health endpoint"

# Step 5: Document
gh copilot suggest "add rustdoc comments to masternode health check handler"
```

### Example 3: Debugging an Issue

```powershell
# Step 1: Understand the problem
gh copilot explain api\src\balance.rs

# Step 2: Ask about behavior
gh copilot ask "Why would calculate_mempool_balance return 0 when there are pending transactions?"

# Step 3: Add debugging
gh copilot suggest "add debug tracing to calculate_mempool_balance to diagnose zero balance issue"

# Step 4: Fix
gh copilot suggest "fix edge case in balance calculation where pending_spent > pending_received"
```

---

## Best Practices for Prompts

### ✅ Good Prompts

```powershell
# Specific and contextual
gh copilot suggest "convert all println! debug statements in api/src/routes.rs to structured tracing::debug! calls"

# Includes file/module context
gh copilot suggest "refactor activate_masternode in api/src/masternode_handlers.rs to use service layer pattern from api/src/services/"

# Mentions existing patterns
gh copilot suggest "add new /treasury/report endpoint following the same ApiResult pattern as other treasury endpoints"

# Defines success criteria
gh copilot suggest "split routes.rs into modules where each module has < 500 lines and handles one domain area"
```

### ❌ Vague Prompts

```powershell
# Too general
gh copilot suggest "improve the code"

# No context
gh copilot suggest "add error handling"

# Unclear goal
gh copilot suggest "make it better"

# Missing constraints
gh copilot suggest "refactor everything"
```

---

## Batch Operations Script

Save this as `copilot-analysis.ps1`:

```powershell
# TIME Coin API Analysis Script

Write-Host "=== TIME Coin API Code Analysis ===" -ForegroundColor Cyan
Write-Host ""

# 1. Code duplication
Write-Host "1. Checking for code duplication..." -ForegroundColor Yellow
gh copilot suggest "find duplicate or very similar functions in api/src/ and list them"
Start-Sleep -Seconds 2

# 2. Error handling consistency
Write-Host "`n2. Analyzing error handling..." -ForegroundColor Yellow
gh copilot suggest "list all error handling patterns in api/src/ and rate consistency"
Start-Sleep -Seconds 2

# 3. Performance issues
Write-Host "`n3. Looking for performance issues..." -ForegroundColor Yellow
gh copilot suggest "identify performance bottlenecks in api/src/ like unnecessary clones or N+1 queries"
Start-Sleep -Seconds 2

# 4. Test coverage gaps
Write-Host "`n4. Checking test coverage..." -ForegroundColor Yellow
gh copilot suggest "list all api/src/ modules without tests or with incomplete tests"
Start-Sleep -Seconds 2

# 5. Documentation needs
Write-Host "`n5. Assessing documentation..." -ForegroundColor Yellow
gh copilot suggest "identify api/src/ modules missing rustdoc comments"
Start-Sleep -Seconds 2

Write-Host "`n=== Analysis Complete ===" -ForegroundColor Green
```

Run with:
```powershell
powershell -ExecutionPolicy Bypass -File copilot-analysis.ps1
```

---

## Quick Wins Checklist

Use these commands to get immediate improvements:

```powershell
# 🎯 High Impact, Low Effort

# 1. Find all println! that should be tracing
gh copilot suggest "list all println! calls in api/src/ and convert to appropriate tracing level"

# 2. Add missing error context
gh copilot suggest "add .context() or better error messages to all .map_err() in api/src/"

# 3. Identify unused imports
gh copilot suggest "find unused imports across all api/src/ files"

# 4. Standardize naming
gh copilot suggest "check if all handler function names follow consistent convention in api/src/"

# 5. Add type documentation
gh copilot suggest "add doc comments to all public structs in api/src/models.rs"
```

---

## Integration with Your Workflow

### VS Code Integration

1. **Install GitHub Copilot extension**
2. **Use Copilot Chat** for inline suggestions
3. **Keyboard shortcuts:**
   - `Ctrl+I` - Inline chat
   - `Ctrl+Shift+I` - Sidebar chat

### Workflow Integration

```powershell
# Before starting work
gh copilot suggest "what should I focus on when refactoring api/src/routes.rs?"

# During development
# Use Copilot Chat in editor for inline suggestions

# Before committing
gh copilot suggest "review my changes and suggest improvements" # then paste git diff

# For code review
gh copilot suggest "analyze this pull request for potential issues" # paste PR diff
```

---

## Time-Saving Aliases

Add to your PowerShell profile (`$PROFILE`):

```powershell
# Copilot aliases
function ghc { gh copilot $args }
function ghce { gh copilot explain $args }
function ghcs { gh copilot suggest $args }
function ghca { gh copilot ask $args }

# TIME Coin specific
function tc-analyze { gh copilot suggest "analyze api/src/$args for improvements" }
function tc-test { gh copilot suggest "generate tests for api/src/$args" }
function tc-doc { gh copilot suggest "add rustdoc comments to api/src/$args" }
```

Usage:
```powershell
ghcs "refactor this function"
tc-analyze routes.rs
tc-test balance.rs
```

---

## Common Issues & Solutions

### Issue: Copilot not installed
```powershell
gh extension install github/gh-copilot
```

### Issue: Authentication failed
```powershell
gh auth refresh
gh auth login
```

### Issue: Response too generic
**Solution:** Be more specific in prompts, include file paths and existing patterns

### Issue: Suggestions don't match coding style
**Solution:** Reference existing code in prompts: "following the pattern in api/src/balance.rs"

---

## Learning Resources

- **Copilot Docs:** https://docs.github.com/en/copilot/github-copilot-in-the-cli
- **Best Practices:** https://github.blog/2023-06-20-how-to-write-better-prompts-for-github-copilot/
- **Rust with Copilot:** https://github.blog/2023-03-09-prompting-github-copilot-for-rust/

---

## Next Steps

1. ✅ Install and authenticate GitHub Copilot CLI
2. ✅ Try the "Quick Wins Checklist" commands above
3. ✅ Run the batch analysis script
4. ✅ Use Copilot for your next refactoring task
5. ✅ Share successful prompts with the team

---

**Last Updated:** 2025-12-02
**Maintainer:** TIME Coin Development Team
