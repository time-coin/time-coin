//! Input validation utilities for TIME CLI

/// Validate TIME address format
pub fn validate_address(addr: &str) -> Result<(), String> {
    if addr.is_empty() {
        return Err("Address cannot be empty".to_string());
    }
    if !addr.starts_with("TIME") {
        return Err("Address must start with 'TIME'".to_string());
    }
    if addr.len() < 40 {
        return Err("Address is too short".to_string());
    }
    Ok(())
}

/// Validate that two addresses are different
pub fn validate_addresses_different(addr1: &str, addr2: &str) -> Result<(), String> {
    if addr1 == addr2 {
        return Err("Addresses must be different".to_string());
    }
    Ok(())
}

/// Validate amount is positive
pub fn validate_amount(amount: u64) -> Result<(), String> {
    if amount == 0 {
        return Err("Amount must be greater than zero".to_string());
    }
    Ok(())
}

/// Validate count is positive
pub fn validate_count(count: usize) -> Result<(), String> {
    if count == 0 {
        return Err("Count must be greater than zero".to_string());
    }
    Ok(())
}

/// Validate public key format
pub fn validate_pubkey(pubkey: &str) -> Result<(), String> {
    if pubkey.is_empty() {
        return Err("Public key cannot be empty".to_string());
    }
    if pubkey.len() != 64 && pubkey.len() != 66 {
        return Err("Public key must be 64 or 66 hex characters".to_string());
    }
    if !pubkey.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Public key must contain only hex characters".to_string());
    }
    Ok(())
}

/// Validate transaction ID format
pub fn validate_txid(txid: &str) -> Result<(), String> {
    if txid.is_empty() {
        return Err("Transaction ID cannot be empty".to_string());
    }
    if txid.len() != 64 {
        return Err("Transaction ID must be 64 hex characters".to_string());
    }
    if !txid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Transaction ID must contain only hex characters".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_address() {
        assert!(validate_address("TIME1234567890123456789012345678901234567890").is_ok());
        assert!(validate_address("").is_err());
        assert!(validate_address("BTC123").is_err());
        assert!(validate_address("TIME123").is_err());
    }

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(1).is_ok());
        assert!(validate_amount(0).is_err());
    }

    #[test]
    fn test_validate_txid() {
        assert!(
            validate_txid("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
                .is_ok()
        );
        assert!(validate_txid("").is_err());
        assert!(validate_txid("123").is_err());
        assert!(
            validate_txid("zzz4567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
                .is_err()
        );
    }
}
