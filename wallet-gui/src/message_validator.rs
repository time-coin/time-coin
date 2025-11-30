/// Message validation module for TIME Coin protocol
/// Validates incoming network messages for security and correctness
use std::net::SocketAddr;

/// Maximum message size (10 MB)
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum array items in a single message
pub const MAX_ARRAY_ITEMS: usize = 10_000;

/// Maximum string length in a message
pub const MAX_STRING_LENGTH: usize = 1024 * 1024; // 1 MB

/// Maximum block size (for individual blocks)
pub const MAX_BLOCK_SIZE: usize = 2 * 1024 * 1024; // 2 MB

/// Message validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            ValidationResult::Invalid(msg) => Some(msg),
            ValidationResult::Valid => None,
        }
    }
}

/// Message validator
pub struct MessageValidator {
    /// Peer address for logging
    peer: SocketAddr,
}

impl MessageValidator {
    pub fn new(peer: SocketAddr) -> Self {
        Self { peer }
    }

    /// Validate message size
    pub fn validate_size(&self, size: usize) -> ValidationResult {
        if size > MAX_MESSAGE_SIZE {
            ValidationResult::Invalid(format!(
                "Message size {} exceeds maximum {}",
                size, MAX_MESSAGE_SIZE
            ))
        } else {
            ValidationResult::Valid
        }
    }

    /// Validate array length
    pub fn validate_array_length(&self, length: usize, field_name: &str) -> ValidationResult {
        if length > MAX_ARRAY_ITEMS {
            ValidationResult::Invalid(format!(
                "Array '{}' length {} exceeds maximum {}",
                field_name, length, MAX_ARRAY_ITEMS
            ))
        } else {
            ValidationResult::Valid
        }
    }

    /// Validate string length
    pub fn validate_string_length(&self, length: usize, field_name: &str) -> ValidationResult {
        if length > MAX_STRING_LENGTH {
            ValidationResult::Invalid(format!(
                "String '{}' length {} exceeds maximum {}",
                field_name, length, MAX_STRING_LENGTH
            ))
        } else {
            ValidationResult::Valid
        }
    }

    /// Validate block size
    pub fn validate_block_size(&self, size: usize) -> ValidationResult {
        if size > MAX_BLOCK_SIZE {
            ValidationResult::Invalid(format!(
                "Block size {} exceeds maximum {}",
                size, MAX_BLOCK_SIZE
            ))
        } else {
            ValidationResult::Valid
        }
    }

    /// Validate blockchain height
    pub fn validate_height(&self, height: u64) -> ValidationResult {
        // Reasonable maximum height (prevents overflow)
        const MAX_HEIGHT: u64 = 100_000_000;

        if height > MAX_HEIGHT {
            ValidationResult::Invalid(format!(
                "Block height {} exceeds maximum {}",
                height, MAX_HEIGHT
            ))
        } else {
            ValidationResult::Valid
        }
    }

    /// Validate height range
    pub fn validate_height_range(&self, start: u64, end: u64) -> ValidationResult {
        if start > end {
            return ValidationResult::Invalid(format!(
                "Invalid height range: start {} > end {}",
                start, end
            ));
        }

        let range_size = end - start + 1;
        if range_size > MAX_ARRAY_ITEMS as u64 {
            return ValidationResult::Invalid(format!(
                "Height range {} exceeds maximum {}",
                range_size, MAX_ARRAY_ITEMS
            ));
        }

        ValidationResult::Valid
    }

    /// Validate hash format (64 hex characters)
    pub fn validate_hash(&self, hash: &str, field_name: &str) -> ValidationResult {
        if hash.is_empty() {
            return ValidationResult::Invalid(format!("Hash '{}' is empty", field_name));
        }

        if hash.len() != 64 {
            return ValidationResult::Invalid(format!(
                "Hash '{}' length {} != 64",
                field_name,
                hash.len()
            ));
        }

        // Check if all characters are hex
        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return ValidationResult::Invalid(format!(
                "Hash '{}' contains non-hex characters",
                field_name
            ));
        }

        ValidationResult::Valid
    }

    /// Validate address format (TIME prefix)
    pub fn validate_address(&self, address: &str, field_name: &str) -> ValidationResult {
        if address.is_empty() {
            return ValidationResult::Invalid(format!("Address '{}' is empty", field_name));
        }

        if !address.starts_with("TIME") {
            return ValidationResult::Invalid(format!(
                "Address '{}' doesn't start with TIME prefix",
                field_name
            ));
        }

        // TIME addresses should be 40-50 characters
        if address.len() < 40 || address.len() > 50 {
            return ValidationResult::Invalid(format!(
                "Address '{}' length {} invalid (expected 40-50)",
                field_name,
                address.len()
            ));
        }

        ValidationResult::Valid
    }

    /// Validate timestamp (not too far in future)
    pub fn validate_timestamp(&self, timestamp: i64) -> ValidationResult {
        use chrono::Utc;

        let now = Utc::now().timestamp();
        const MAX_FUTURE_DRIFT: i64 = 300; // 5 minutes

        if timestamp > now + MAX_FUTURE_DRIFT {
            ValidationResult::Invalid(format!(
                "Timestamp {} is {} seconds in future (max drift: {}s)",
                timestamp,
                timestamp - now,
                MAX_FUTURE_DRIFT
            ))
        } else if timestamp < 0 {
            ValidationResult::Invalid(format!("Timestamp {} is negative", timestamp))
        } else {
            ValidationResult::Valid
        }
    }

    /// Validate transaction amount
    pub fn validate_amount(&self, amount: u64) -> ValidationResult {
        // Maximum supply: 21 million TIME coins in satoshis
        const MAX_SUPPLY: u64 = 21_000_000 * 100_000_000;

        if amount == 0 {
            ValidationResult::Invalid("Amount cannot be zero".to_string())
        } else if amount > MAX_SUPPLY {
            ValidationResult::Invalid(format!(
                "Amount {} exceeds maximum supply {}",
                amount, MAX_SUPPLY
            ))
        } else {
            ValidationResult::Valid
        }
    }

    /// Log validation error
    pub fn log_error(&self, error: &str) {
        println!("⚠️  [{}] Validation error: {}", self.peer, error);
    }
}

/// Violation tracker for malformed messages
pub struct ViolationTracker {
    violations: std::collections::HashMap<SocketAddr, Vec<Violation>>,
}

#[derive(Debug, Clone)]
struct Violation {
    timestamp: std::time::Instant,
    reason: String,
}

impl ViolationTracker {
    pub fn new() -> Self {
        Self {
            violations: std::collections::HashMap::new(),
        }
    }

    /// Record a validation violation
    pub fn record_violation(&mut self, peer: SocketAddr, reason: String) {
        let violation = Violation {
            timestamp: std::time::Instant::now(),
            reason: reason.clone(),
        };

        self.violations.entry(peer).or_default().push(violation);

        println!("📋 Recorded violation from {}: {}", peer, reason);
    }

    /// Get violation count for a peer in last N minutes
    pub fn get_recent_violations(&self, peer: SocketAddr, minutes: u64) -> usize {
        if let Some(violations) = self.violations.get(&peer) {
            let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(minutes * 60);
            violations.iter().filter(|v| v.timestamp > cutoff).count()
        } else {
            0
        }
    }

    /// Check if peer should be disconnected (>10 violations in 10 minutes)
    pub fn should_disconnect(&self, peer: SocketAddr) -> bool {
        self.get_recent_violations(peer, 10) >= 10
    }

    /// Clean up old violations (>1 hour)
    pub fn cleanup_old_violations(&mut self) {
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(3600);

        for violations in self.violations.values_mut() {
            violations.retain(|v| v.timestamp > cutoff);
        }

        // Remove empty entries
        self.violations.retain(|_, v| !v.is_empty());
    }
}

impl Default for ViolationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validator() -> MessageValidator {
        MessageValidator::new("127.0.0.1:8080".parse().unwrap())
    }

    #[test]
    fn test_validate_size() {
        let validator = test_validator();
        assert!(validator.validate_size(1000).is_valid());
        assert!(validator.validate_size(MAX_MESSAGE_SIZE).is_valid());
        assert!(!validator.validate_size(MAX_MESSAGE_SIZE + 1).is_valid());
    }

    #[test]
    fn test_validate_array_length() {
        let validator = test_validator();
        assert!(validator.validate_array_length(100, "test").is_valid());
        assert!(validator
            .validate_array_length(MAX_ARRAY_ITEMS, "test")
            .is_valid());
        assert!(!validator
            .validate_array_length(MAX_ARRAY_ITEMS + 1, "test")
            .is_valid());
    }

    #[test]
    fn test_validate_hash() {
        let validator = test_validator();
        let valid_hash = "a".repeat(64);
        let invalid_hash_short = "a".repeat(63);
        let invalid_hash_long = "a".repeat(65);
        let invalid_hash_chars = "z".repeat(64);

        assert!(validator.validate_hash(&valid_hash, "test").is_valid());
        assert!(!validator
            .validate_hash(&invalid_hash_short, "test")
            .is_valid());
        assert!(!validator
            .validate_hash(&invalid_hash_long, "test")
            .is_valid());
        assert!(!validator
            .validate_hash(&invalid_hash_chars, "test")
            .is_valid());
    }

    #[test]
    fn test_validate_address() {
        let validator = test_validator();
        let valid_address = "TIME".to_string() + &"a".repeat(40);
        let invalid_no_prefix = "a".repeat(44);
        let invalid_too_short = "TIME".to_string() + &"a".repeat(30);

        assert!(validator
            .validate_address(&valid_address, "test")
            .is_valid());
        assert!(!validator
            .validate_address(&invalid_no_prefix, "test")
            .is_valid());
        assert!(!validator
            .validate_address(&invalid_too_short, "test")
            .is_valid());
    }

    #[test]
    fn test_validate_height_range() {
        let validator = test_validator();
        assert!(validator.validate_height_range(0, 100).is_valid());
        assert!(!validator.validate_height_range(100, 0).is_valid());
        assert!(!validator
            .validate_height_range(0, MAX_ARRAY_ITEMS as u64 + 1)
            .is_valid());
    }

    #[test]
    fn test_validate_amount() {
        let validator = test_validator();
        assert!(validator.validate_amount(100).is_valid());
        assert!(!validator.validate_amount(0).is_valid());
        assert!(!validator.validate_amount(u64::MAX).is_valid());
    }

    #[test]
    fn test_violation_tracker() {
        let mut tracker = ViolationTracker::new();
        let peer: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        // Record violations
        for i in 0..5 {
            tracker.record_violation(peer, format!("Violation {}", i));
        }

        assert_eq!(tracker.get_recent_violations(peer, 10), 5);
        assert!(!tracker.should_disconnect(peer));

        // Record more violations to trigger disconnect
        for i in 5..15 {
            tracker.record_violation(peer, format!("Violation {}", i));
        }

        assert!(tracker.should_disconnect(peer));
    }
}
