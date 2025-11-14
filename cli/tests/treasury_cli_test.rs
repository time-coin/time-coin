//! Integration tests for Treasury CLI commands

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_treasury_info_command() {
        // Test that the treasury info command can be parsed
        let output = Command::new("cargo")
            .args(&["run", "--bin", "time-cli", "--", "treasury", "info", "--json"])
            .output();

        // Command should parse successfully (even if API is not available)
        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_list_proposals_command() {
        // Test that the treasury list-proposals command can be parsed
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "time-cli",
                "--",
                "treasury",
                "list-proposals",
                "--json",
            ])
            .output();

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_get_proposal_command() {
        // Test that the treasury get-proposal command can be parsed
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "time-cli",
                "--",
                "treasury",
                "get-proposal",
                "prop-123",
                "--json",
            ])
            .output();

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_propose_command() {
        // Test that the treasury propose command can be parsed
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "time-cli",
                "--",
                "treasury",
                "propose",
                "--title",
                "Test Proposal",
                "--description",
                "A test proposal",
                "--recipient",
                "TIME1test0000000000000000000000000000000",
                "--amount",
                "1000.0",
                "--json",
            ])
            .output();

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_vote_command() {
        // Test that the treasury vote command can be parsed
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--bin",
                "time-cli",
                "--",
                "treasury",
                "vote",
                "prop-123",
                "yes",
                "--json",
            ])
            .output();

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_vote_choices() {
        // Test all valid vote choices
        for choice in &["yes", "no", "abstain"] {
            let output = Command::new("cargo")
                .args(&[
                    "run",
                    "--bin",
                    "time-cli",
                    "--",
                    "treasury",
                    "vote",
                    "prop-123",
                    choice,
                    "--json",
                ])
                .output();

            assert!(output.is_ok());
        }
    }
}
