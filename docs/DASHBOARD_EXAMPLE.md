# TIME Coin Dashboard - Example Output

This file shows an example of what the TIME Coin Node Dashboard displays.

## When Node is Running and Connected

```
╔══════════════════════════════════════════════════════════════╗
║         TIME COIN NODE DASHBOARD                            ║
╚══════════════════════════════════════════════════════════════╝
  Current Time: 2025-11-06 16:45:30 UTC
  API Endpoint: http://localhost:24101

┌─ Blockchain Status ──────────────────────────────────────────┐
│ Block Height:        245
│ Best Block Hash:     a1b2c3d4e5f67890...
└──────────────────────────────────────────────────────────────┘

┌─ Network Peers ──────────────────────────────────────────────┐
│ Connected Peers:     5
│  1. 192.168.1.100:24100 (v0.1.0, 120s ago)
│  2. 192.168.1.101:24100 (v0.1.0, 95s ago)
│  3. 10.0.0.50:24100 (v0.1.0, 45s ago)
│  4. 172.16.0.5:24100 (v0.1.0, 30s ago)
│  5. 192.168.2.25:24100 (v0.1.0, 15s ago)
└──────────────────────────────────────────────────────────────┘

┌─ Mempool Status ─────────────────────────────────────────────┐
│ Pending Transactions: 12
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Press Ctrl+C to exit | Auto-refresh every 5 seconds
```

## When Node is Not Reachable

```
╔══════════════════════════════════════════════════════════════╗
║         TIME COIN NODE DASHBOARD                            ║
╚══════════════════════════════════════════════════════════════╝
  Current Time: 2025-11-06 16:45:30 UTC
  API Endpoint: http://localhost:24101

┌─ Error ──────────────────────────────────────────────────────┐
│ Cannot connect to node: Request failed: connection refused
└──────────────────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Press Ctrl+C to exit | Auto-refresh every 5 seconds
```

## Color Coding

In a real terminal, the dashboard uses colors:
- **Cyan/Blue**: Headers and borders for blockchain section
- **Yellow**: Network peers section
- **Magenta**: Mempool section  
- **Red**: Error messages
- **Green**: Block height values
- **Bright Blue**: Hash values
- **White**: Timestamps and general data
- **Gray/Bright Black**: Labels and footer text

## Cross-Platform Screen Clearing

The dashboard uses `crossterm` to properly clear the screen on each refresh:

- **Windows**: Uses Windows Console API for clean clearing without scrolling
- **Linux/macOS**: Uses optimized ANSI sequences
- **All Platforms**: Cursor repositioned to (0,0) for in-place refresh

This ensures the dashboard content updates smoothly without:
- Scrolling the terminal
- Leaving artifacts from previous renders
- Flickering or tearing
- Platform-specific issues

## Performance

- **Refresh Rate**: 5 seconds (configurable in code)
- **API Calls per Refresh**: 4 endpoints (health, blockchain, peers, mempool)
- **Network Impact**: Minimal (~1-2KB per refresh cycle)
- **CPU Usage**: Negligible when idle between refreshes
- **Memory**: ~5-10MB RSS (typical for Rust async applications)
