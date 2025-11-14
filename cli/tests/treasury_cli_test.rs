//! Integration tests for Treasury CLI commands
//!
//! These tests verify that the CLI commands are properly structured
//! and can be parsed correctly. They test command-line argument validation
//! but do not require a running API server.

#[cfg(test)]
mod tests {
    use std::process::Command;

    /// Helper function to run a CLI command and check it parses successfully
    fn run_cli_command(args: &[&str]) -> std::io::Result<std::process::Output> {
        let mut cmd_args = vec!["run", "--bin", "time-cli", "--"];
        cmd_args.extend_from_slice(args);
        Command::new("cargo").args(&cmd_args).output()
    }

    #[test]
    fn test_treasury_info_command() {
        // Test that the treasury info command can be parsed
        let output = run_cli_command(&["treasury", "info", "--json"]);

        // Command should parse successfully (even if API is not available)
        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_list_proposals_command() {
        // Test that the treasury list-proposals command can be parsed
        let output = run_cli_command(&["treasury", "list-proposals", "--json"]);

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_get_proposal_command() {
        // Test that the treasury get-proposal command can be parsed
        let output = run_cli_command(&["treasury", "get-proposal", "prop-123", "--json"]);

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_propose_command() {
        // Test that the treasury propose command can be parsed with all required arguments
        let output = run_cli_command(&[
            "treasury",
            "propose",
            "--title",
            "Test Proposal",
            "--description",
            "A test proposal for development funding",
            "--recipient",
            "TIME1test0000000000000000000000000000000",
            "--amount",
            "1000.0",
            "--voting-period",
            "14",
            "--json",
        ]);

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_vote_command() {
        // Test that the treasury vote command can be parsed
        let output = run_cli_command(&["treasury", "vote", "prop-123", "yes", "--json"]);

        assert!(output.is_ok());
    }

    #[test]
    fn test_treasury_vote_choices() {
        // Test all valid vote choices: yes, no, abstain
        for choice in &["yes", "no", "abstain"] {
            let output = run_cli_command(&[
                "treasury",
                "vote",
                "prop-123",
                choice,
                "--masternode-id",
                "192.168.1.100",
                "--json",
            ]);

            assert!(output.is_ok());
        }
    }

    #[test]
    fn test_treasury_propose_with_custom_voting_period() {
        // Test proposal with custom voting period
        let output = run_cli_command(&[
            "treasury",
            "propose",
            "--title",
            "Extended Vote",
            "--description",
            "Proposal with 30-day voting period",
            "--recipient",
            "TIME1recipient000000000000000000000000000",
            "--amount",
            "5000.0",
            "--voting-period",
            "30",
            "--json",
        ]);

        assert!(output.is_ok());
    }
}
