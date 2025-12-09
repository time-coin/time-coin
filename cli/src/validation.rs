//! Input validation for CLI commands
//!
//! This module provides validation functions for user inputs to prevent
//! invalid or malicious data from being processed.

use std::error::Error;
use std::fmt;

/// Maximum supply of TIME coins (21 million)
const MAX_SUPPLY: f64 = 21_000_000.0;

/// Minimum transaction amount (1 satoshi = 0.00000001 TIME)
const MIN_AMOUNT: f64 = 0.00000001;

/// TIME address prefix
const ADDRESS_PREFIX: &str = "TIME1";

/// Expected length of a TIME address
const ADDRESS_LENGTH: usize = 42; // TIME1 + 38 characters

/// Expected length of a public key (hex)
const PUBKEY_LENGTH: usize = 64;

#[derive(Debug)]
pub enum ValidationError {
    InvalidAddress(String),
    InvalidAmount(String),
    InvalidPublicKey(String),
    InvalidRange(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValidationError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            ValidationError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            ValidationError::InvalidPublicKey(msg) => write!(f, "Invalid public key: {}", msg),
            ValidationError::InvalidRange(msg) => write!(f, "Invalid range: {}", msg),
        }
    }
}

impl Error for ValidationError {}

/// Validate a TIME Coin address
///
/// Rules:
/// - Must start with "TIME1"
/// - Must be exactly 42 characters long
/// - Must contain only alphanumeric characters (base58)
pub fn validate_address(addr: &str) -> Result<(), ValidationError> {
    if !addr.starts_with(ADDRESS_PREFIX) {
        return Err(ValidationError::InvalidAddress(format!(
            "Address must start with '{}'",
            ADDRESS_PREFIX
        )));
    }

    if addr.len() != ADDRESS_LENGTH {
        return Err(ValidationError::InvalidAddress(format!(
            "Address must be exactly {} characters long (got {})",
            ADDRESS_LENGTH,
            addr.len()
        )));
    }

    // Check for invalid characters (should be base58-like)
    if !addr.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ValidationError::InvalidAddress(
            "Address contains invalid characters".to_string(),
        ));
    }

    Ok(())
}

/// Validate transaction amount
///
/// Rules:
/// - Must be positive
/// - Must be at least 1 satoshi (0.00000001 TIME)
/// - Must not exceed total supply (21,000,000 TIME)
/// - Must have at most 8 decimal places
pub fn validate_amount(amount: f64) -> Result<(), ValidationError> {
    if amount <= 0.0 {
        return Err(ValidationError::InvalidAmount(
            "Amount must be positive".to_string(),
        ));
    }

    if amount < MIN_AMOUNT {
        return Err(ValidationError::InvalidAmount(format!(
            "Amount must be at least {} TIME (1 satoshi)",
            MIN_AMOUNT
        )));
    }

    if amount > MAX_SUPPLY {
        return Err(ValidationError::InvalidAmount(format!(
            "Amount cannot exceed total supply ({} TIME)",
            MAX_SUPPLY
        )));
    }

    // Check decimal places (max 8)
    let amount_str = format!("{:.8}", amount);
    let parts: Vec<&str> = amount_str.split('.').collect();
    if parts.len() == 2 {
        let decimals = parts[1].trim_end_matches('0');
        if decimals.len() > 8 {
            return Err(ValidationError::InvalidAmount(
                "Amount cannot have more than 8 decimal places".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate two addresses are different
pub fn validate_addresses_different(from: &str, to: &str) -> Result<(), ValidationError> {
    if from == to {
        return Err(ValidationError::InvalidAddress(
            "Source and destination addresses must be different".to_string(),
        ));
    }
    Ok(())
}

/// Validate public key
///
/// Rules:
/// - Must be exactly 64 characters (hex)
/// - Must contain only hexadecimal characters
pub fn validate_pubkey(pubkey: &str) -> Result<(), ValidationError> {
    if pubkey.len() != PUBKEY_LENGTH {
        return Err(ValidationError::InvalidPublicKey(format!(
            "Public key must be exactly {} characters (got {})",
            PUBKEY_LENGTH,
            pubkey.len()
        )));
    }

    if !pubkey.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValidationError::InvalidPublicKey(
            "Public key must contain only hexadecimal characters".to_string(),
        ));
    }

    Ok(())
}

/// Validate count parameter (for listing operations)
pub fn validate_count(count: usize) -> Result<(), ValidationError> {
    const MAX_COUNT: usize = 1000;

    if count == 0 {
        return Err(ValidationError::InvalidRange(
            "Count must be at least 1".to_string(),
        ));
    }

    if count > MAX_COUNT {
        return Err(ValidationError::InvalidRange(format!(
            "Count cannot exceed {} (got {})",
            MAX_COUNT, count
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_address_valid() {
        assert!(validate_address("TIME1abcdefghijklmnopqrstuvwxyz12345678").is_ok());
    }

    #[test]
    fn test_validate_address_wrong_prefix() {
        assert!(validate_address("BTC1abcdefghijklmnopqrstuvwxyz12345678").is_err());
    }

    #[test]
    fn test_validate_address_wrong_length() {
        assert!(validate_address("TIME1abc").is_err());
    }

    #[test]
    fn test_validate_amount_valid() {
        assert!(validate_amount(1.0).is_ok());
        assert!(validate_amount(0.00000001).is_ok());
        assert!(validate_amount(21_000_000.0).is_ok());
    }

    #[test]
    fn test_validate_amount_zero() {
        assert!(validate_amount(0.0).is_err());
    }

    #[test]
    fn test_validate_amount_negative() {
        assert!(validate_amount(-1.0).is_err());
    }

    #[test]
    fn test_validate_amount_too_large() {
        assert!(validate_amount(22_000_000.0).is_err());
    }

    #[test]
    fn test_validate_pubkey_valid() {
        let valid_pubkey = "0".repeat(64);
        assert!(validate_pubkey(&valid_pubkey).is_ok());
    }

    #[test]
    fn test_validate_pubkey_wrong_length() {
        assert!(validate_pubkey("abc").is_err());
    }

    #[test]
    fn test_validate_pubkey_non_hex() {
        let invalid_pubkey = "g".repeat(64);
        assert!(validate_pubkey(&invalid_pubkey).is_err());
    }

    #[test]
    fn test_validate_addresses_different() {
        let addr1 = "TIME1abcdefghijklmnopqrstuvwxyz12345678";
        let addr2 = "TIME1zyxwvutsrqponmlkjihgfedcba87654321";
        assert!(validate_addresses_different(addr1, addr2).is_ok());
        assert!(validate_addresses_different(addr1, addr1).is_err());
    }

    #[test]
    fn test_validate_count() {
        assert!(validate_count(1).is_ok());
        assert!(validate_count(100).is_ok());
        assert!(validate_count(0).is_err());
        assert!(validate_count(2000).is_err());
    }
}
