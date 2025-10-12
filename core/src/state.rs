use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChainState {
    pub balances: HashMap<String, u64>,
    pub current_block: u64,
}

impl ChainState {
    pub fn new() -> Self {
        ChainState {
            balances: HashMap::new(),
            current_block: 0,
        }
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }

    pub fn set_balance(&mut self, address: String, amount: u64) {
        self.balances.insert(address, amount);
    }
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = ChainState::new();
        assert_eq!(state.current_block, 0);
        assert_eq!(state.get_balance("alice"), 0);
    }

    #[test]
    fn test_set_balance() {
        let mut state = ChainState::new();
        state.set_balance("alice".to_string(), 100);
        assert_eq!(state.get_balance("alice"), 100);
    }
}
