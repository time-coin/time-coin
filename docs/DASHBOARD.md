# TIME Coin Node Dashboard

A real-time monitoring dashboard for TIME Coin nodes that displays blockchain status, network peers, and mempool information.

## Features

- **Real-time Updates**: Auto-refreshes every 5 seconds
- **Cross-Platform**: Works correctly on Windows (Command Prompt, PowerShell, Git Bash), Linux, and macOS
- **Blockchain Status**: Shows current block height and best block hash
- **Network Monitoring**: Displays connected peers with connection details
- **Mempool Status**: Shows pending transactions count
- **Colorful Interface**: Easy-to-read colored output using Unicode box-drawing characters

## Installation

The dashboard is built as part of the TIME Coin CLI tools:

```bash
# Build from source
cd time-coin
cargo build --release --bin time-dashboard

# The binary will be available at:
# target/release/time-dashboard
```

## Usage

### Basic Usage

Connect to the default local node (http://localhost:24101):

```bash
time-dashboard
```

### Custom API Endpoint

Connect to a remote or custom node:

```bash
time-dashboard http://192.168.1.100:24101
```

### Running as a Background Monitor

On Linux/macOS:
```bash
# Run in a tmux or screen session
tmux new -s dashboard
time-dashboard
# Detach with Ctrl+B, then D
```

On Windows:
```powershell
# Run in a separate PowerShell window
Start-Process powershell -ArgumentList "time-dashboard"
```

## Dashboard Layout

```
╔══════════════════════════════════════════════════════════════╗
║         TIME COIN NODE DASHBOARD                            ║
╚══════════════════════════════════════════════════════════════╝
  Current Time: 2025-11-06 16:30:45 UTC
  API Endpoint: http://localhost:24101

┌─ Blockchain Status ──────────────────────────────────────────┐
│ Block Height:        123
│ Best Block Hash:     a1b2c3d4e5f6...
└──────────────────────────────────────────────────────────────┘

┌─ Network Peers ──────────────────────────────────────────────┐
│ Connected Peers:     3
│  1. 192.168.1.100:24100 (v0.1.0, 120s ago)
│  2. 192.168.1.101:24100 (v0.1.0, 95s ago)
│  3. 10.0.0.50:24100 (v0.1.0, 45s ago)
└──────────────────────────────────────────────────────────────┘

┌─ Mempool Status ─────────────────────────────────────────────┐
│ Pending Transactions: 5
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Press Ctrl+C to exit | Auto-refresh every 5 seconds
```

## Requirements

- TIME Coin node running with API enabled (default port 24101)
- API must be accessible from the dashboard location

## Technical Details

### Cross-Platform Terminal Clearing

The dashboard uses the `crossterm` crate for terminal manipulation, which provides cross-platform support for:

- **Screen clearing**: Properly clears the terminal without scrolling on all platforms
- **Cursor positioning**: Moves cursor to home position (0,0)
- **Color support**: ANSI color codes work consistently across platforms

#### Why Crossterm?

Previously, manual ANSI escape sequences (`\x1b[2J\x1b[H`) were used for screen clearing. However, this approach has issues:

- **Windows Command Prompt**: Causes scrolling instead of clearing
- **Windows PowerShell**: Inconsistent behavior
- **Git Bash on Windows**: Works but not optimal

The `crossterm` crate handles platform-specific differences automatically:
- On Windows, it uses Windows Console API when appropriate
- On Unix-like systems, it uses ANSI escape sequences
- Falls back gracefully when terminal capabilities are limited

### API Endpoints Used

The dashboard queries the following TIME Coin API endpoints:

- `GET /health` - Check node health status
- `GET /blockchain/info` - Get current blockchain height and best block hash
- `GET /peers` - List connected network peers
- `GET /mempool` - Get mempool transaction count

## Troubleshooting

### Dashboard Shows Connection Error

**Problem**: Cannot connect to node: Request failed: ...

**Solution**:
1. Verify the node is running: `systemctl status timed` (Linux) or check task manager (Windows)
2. Check the API is enabled in your node configuration
3. Verify the correct port is being used (default: 24101)
4. Check firewall settings if connecting to a remote node

### Colors Not Displaying

**Problem**: Special characters or boxes appear instead of colors/borders

**Solution**:
1. Ensure your terminal supports Unicode and ANSI colors
2. On Windows, use Windows Terminal, PowerShell 7+, or Git Bash
3. Avoid using legacy Command Prompt (cmd.exe)

### Refresh Rate Too Fast/Slow

**Problem**: Dashboard updates too frequently or not enough

**Solution**: 
The refresh rate is currently hardcoded to 5 seconds. To change it:
1. Edit `cli/src/bin/time-dashboard.rs`
2. Find the line: `time::sleep(Duration::from_secs(5)).await;`
3. Change `5` to your preferred interval in seconds
4. Rebuild: `cargo build --release --bin time-dashboard`

## Development

### Building from Source

```bash
cd time-coin/cli
cargo build --bin time-dashboard
```

### Running Tests

```bash
cargo test --bin time-dashboard
```

### Code Location

- Source: `cli/src/bin/time-dashboard.rs`
- Dependencies: `cli/Cargo.toml`

## Future Enhancements

Potential improvements for future versions:

- [ ] Configurable refresh rate via CLI argument
- [ ] Save dashboard output to log file
- [ ] Historical charts for block height and peer count
- [ ] Alerts for low peer count or synchronization issues
- [ ] Support for displaying masternode-specific metrics
- [ ] Web-based dashboard alternative
- [ ] JSON output mode for integration with monitoring tools

## See Also

- [API Documentation](API.md) - Full API reference
- [Building Guide](BUILDING.md) - Build instructions
- [Node Setup](../scripts/README.md) - Setting up a TIME Coin node
