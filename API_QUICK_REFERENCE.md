# TIME Coin API Quick Reference Card

## 🎯 Recent Improvements (Completed)

### 1. Shared Balance Calculation
**Location:** `api/src/balance.rs`

```rust
use crate::balance::calculate_mempool_balance;

let unconfirmed = calculate_mempool_balance(&address, &blockchain, mempool).await;
```

**Benefits:** Single source of truth, -62 lines of duplicate code

---

### 2. Response Macros
**Location:** `api/src/response.rs`

```rust
use crate::ok_json;

// Before: Ok(Json(data))
// After:
ok_json!(MyResponse { field: value })
```

---

### 3. Enhanced Error Handling
**Location:** `api/src/error.rs`

```rust
// Automatic conversion
.map_err(ApiError::from)?

// Or just use ? operator
do_something()?  // Errors auto-convert to ApiError
```

**New From implementations:**
- `From<Box<dyn Error>>`
- `From<String>`
- `From<&str>`

---

## 📋 Code Standards

### Handler Pattern
```rust
use crate::{ok_json, ApiResult, ApiState};
use axum::{extract::State, Json};

pub async fn my_handler(
    State(state): State<ApiState>,
    Json(req): Json<MyRequest>,
) -> ApiResult<Json<MyResponse>> {
    // 1. Validation
    if req.amount == 0 {
        return Err(ApiError::BadRequest("Amount must be positive".into()));
    }
    
    // 2. Business logic
    let result = process(&state, req).await?;
    
    // 3. Response
    ok_json!(MyResponse { result })
}
```

### Error Handling
```rust
// ✅ Good: Use From trait
something().map_err(ApiError::from)?

// ✅ Good: Use ? operator
let data = fallible_operation()?;

// ❌ Avoid: Manual error formatting
.map_err(|e| ApiError::Internal(format!("Failed: {}", e)))?
```

### Logging
```rust
// ✅ Good: Structured logging
tracing::info!(
    address = %address,
    amount = amount,
    "balance_checked"
);

// ❌ Avoid: println! with emoji
println!("✅ Balance checked: {}", address);
```

---

## 🔧 Common Operations

### Get Balance (with mempool)
```rust
let blockchain = state.blockchain.read().await;
let balance = blockchain.get_balance(&address);

let unconfirmed = if let Some(mempool) = &state.mempool {
    calculate_mempool_balance(&address, &blockchain, mempool).await
} else {
    0
};
```

### Create Transaction Response
```rust
ok_json!(TransactionResponse {
    txid: tx.hash(),
    status: "pending",
    confirmations: 0,
})
```

### Validate Input
```rust
// TODO: Use validator crate (see COPILOT_CLI_GUIDE.md)
if address.len() < 10 {
    return Err(ApiError::InvalidAddress("Too short".into()));
}
```

---

## 🚀 GitHub Copilot Quick Commands

```powershell
# Explain code
gh copilot explain api\src\balance.rs

# Get refactoring suggestions
gh copilot suggest "extract common logic from handlers"

# Generate tests
gh copilot suggest "create unit tests for balance calculation"

# Find issues
gh copilot suggest "identify performance bottlenecks in api/src/"
```

---

## 📁 File Organization

```
api/src/
├── balance.rs              # ✅ Shared balance calculations
├── error.rs                # ✅ Enhanced with From impls
├── response.rs             # ✅ Response utilities
├── lib.rs                  # Module declarations
├── state.rs                # API state management
├── routes.rs               # ⚠️  TODO: Split into modules
├── rpc_handlers.rs         # RPC endpoints
├── handlers.rs             # Misc handlers
├── *_handlers.rs           # Domain-specific handlers
└── models.rs               # Request/response types
```

### TODO: Route Organization
```
api/src/routes/
├── mod.rs              # Route registration
├── blockchain.rs       # /blockchain/*
├── wallet.rs           # /wallet/*
├── treasury.rs         # /treasury/*
└── consensus.rs        # /consensus/*
```

---

## 🧪 Testing Guidelines

### Unit Test Template
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calculate_balance_with_pending() {
        // Arrange
        let state = setup_test_state();
        
        // Act
        let balance = calculate_mempool_balance(
            "TIME1test",
            &state.blockchain,
            &state.mempool
        ).await;
        
        // Assert
        assert_eq!(balance, 100_000_000);
    }
}
```

### Integration Test Template
```rust
#[tokio::test]
async fn test_endpoint_success() {
    let state = setup_test_state().await;
    
    let response = my_handler(
        State(state),
        Json(TestRequest::default())
    ).await;
    
    assert!(response.is_ok());
}
```

---

## 🐛 Common Pitfalls

### 1. Duplicate Balance Calculation
```rust
// ❌ DON'T: Reimplement balance logic
let balance = mempool.iter().sum();

// ✅ DO: Use shared function
let balance = calculate_mempool_balance(&addr, &bc, mempool).await;
```

### 2. Inconsistent Error Handling
```rust
// ❌ DON'T: Multiple patterns
.map_err(|e| ApiError::Internal(format!("{}", e)))?

// ✅ DO: Use From trait
.map_err(ApiError::from)?
```

### 3. Missing Validation
```rust
// ❌ DON'T: Assume valid input
let amount = req.amount;

// ✅ DO: Validate first
if req.amount == 0 {
    return Err(ApiError::BadRequest("Invalid amount".into()));
}
```

---

## 📊 Metrics

| Metric | Value |
|--------|-------|
| **Code Duplication** | ✅ -66% (balance functions) |
| **LOC Saved** | ✅ 62 lines removed |
| **Error Patterns** | ✅ Unified with From trait |
| **Test Coverage** | 🔄 25% → Target: 60%+ |
| **Route Organization** | 🔄 Pending modularization |

---

## 🎓 Learning Resources

1. **API Refactoring Summary:** `API_REFACTORING_SUMMARY.md`
   - Complete implementation guide
   - Before/after metrics
   - Priority queue

2. **Copilot CLI Guide:** `COPILOT_CLI_GUIDE.md`
   - Setup instructions
   - Practical examples
   - Best practices

3. **Module Documentation:**
   - `api/src/balance.rs` - Balance calculations
   - `api/src/response.rs` - Response utilities
   - `api/src/error.rs` - Error handling

---

## 🔗 Quick Links

- 📖 [Full Refactoring Guide](API_REFACTORING_SUMMARY.md)
- 🤖 [Copilot CLI Guide](COPILOT_CLI_GUIDE.md)
- 🏗️ [Project Structure](../README.md)

---

## ⚡ Next Steps

1. **Read:** `API_REFACTORING_SUMMARY.md` for full context
2. **Setup:** Follow `COPILOT_CLI_GUIDE.md` to install tools
3. **Refactor:** Start with route organization (Week 1, Task 4)
4. **Test:** Add integration tests for new features

---

**Last Updated:** 2025-12-02
**Version:** 1.0
